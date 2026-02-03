//! Rate Limiting and DDoS Protection
//!
//! Provides connection throttling, bandwidth limiting, and DDoS protection
//! for production-grade server hosting at scale.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, warn};

/// Rate limit configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum connections per IP per window
    pub max_connections_per_ip: usize,
    /// Maximum packets per second per connection
    pub max_packets_per_second: u32,
    /// Maximum bandwidth per connection (bytes/sec)
    pub max_bandwidth_per_connection: usize,
    /// Time window for connection limits
    pub connection_window: Duration,
    /// Burst allowance for packet limits
    pub burst_allowance: u32,
    /// IP ban duration for violators
    pub ban_duration: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_connections_per_ip: 10,
            max_packets_per_second: 60,
            max_bandwidth_per_connection: 1024 * 1024, // 1 MB/sec
            connection_window: Duration::from_secs(60),
            burst_allowance: 10,
            ban_duration: Duration::from_secs(3600), // 1 hour
        }
    }
}

/// Token bucket for rate limiting
#[derive(Debug, Clone)]
struct TokenBucket {
    tokens: f64,
    capacity: f64,
    refill_rate: f64,
    last_refill: Instant,
}

impl TokenBucket {
    fn new(capacity: f64, refill_rate: f64) -> Self {
        Self { tokens: capacity, capacity, refill_rate, last_refill: Instant::now() }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.capacity);
        self.last_refill = now;
    }

    fn try_consume(&mut self, tokens: f64) -> bool {
        self.refill();
        if self.tokens >= tokens {
            self.tokens -= tokens;
            true
        } else {
            false
        }
    }

    fn available(&mut self) -> f64 {
        self.refill();
        self.tokens
    }
}

/// Connection tracking per IP
#[derive(Debug)]
struct IpConnectionTracker {
    connection_count: usize,
    last_connection: Instant,
    total_violations: u32,
}

impl IpConnectionTracker {
    fn new() -> Self {
        Self { connection_count: 0, last_connection: Instant::now(), total_violations: 0 }
    }
}

/// Per-connection rate limiter
#[derive(Debug)]
struct ConnectionRateLimiter {
    packet_bucket: TokenBucket,
    bandwidth_bucket: TokenBucket,
    total_packets: u64,
    total_bytes: u64,
    violations: u32,
}

impl ConnectionRateLimiter {
    fn new(config: &RateLimitConfig) -> Self {
        Self {
            packet_bucket: TokenBucket::new(
                config.max_packets_per_second as f64 + config.burst_allowance as f64,
                config.max_packets_per_second as f64,
            ),
            bandwidth_bucket: TokenBucket::new(
                config.max_bandwidth_per_connection as f64,
                config.max_bandwidth_per_connection as f64,
            ),
            total_packets: 0,
            total_bytes: 0,
            violations: 0,
        }
    }
}

/// Banned IP entry
#[derive(Debug)]
struct BannedIp {
    ban_time: Instant,
    reason: String,
    violations: u32,
}

/// Rate limiter for DDoS protection
pub struct RateLimiter {
    config: RateLimitConfig,
    ip_trackers: Arc<RwLock<HashMap<IpAddr, IpConnectionTracker>>>,
    connection_limiters: Arc<RwLock<HashMap<u64, ConnectionRateLimiter>>>,
    banned_ips: Arc<RwLock<HashMap<IpAddr, BannedIp>>>,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(config: RateLimitConfig) -> Self {
        debug!(
            max_connections_per_ip = config.max_connections_per_ip,
            max_packets_per_second = config.max_packets_per_second,
            max_bandwidth_mbps = config.max_bandwidth_per_connection / (1024 * 1024),
            "Creating rate limiter"
        );

        Self {
            config,
            ip_trackers: Arc::new(RwLock::new(HashMap::new())),
            connection_limiters: Arc::new(RwLock::new(HashMap::new())),
            banned_ips: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if a new connection from this IP is allowed
    pub async fn check_connection(&self, ip: IpAddr) -> Result<(), String> {
        // Check if IP is banned
        let banned = self.banned_ips.read().await;
        if let Some(ban) = banned.get(&ip) {
            let elapsed = ban.ban_time.elapsed();
            if elapsed < self.config.ban_duration {
                let remaining = self.config.ban_duration.as_secs() - elapsed.as_secs();
                return Err(format!("IP banned for {} more seconds: {}", remaining, ban.reason));
            }
        }
        drop(banned);

        // Check connection rate limit
        let mut trackers = self.ip_trackers.write().await;
        let now = Instant::now();

        let tracker = trackers.entry(ip).or_insert_with(IpConnectionTracker::new);

        // Reset count if window has passed
        if now.duration_since(tracker.last_connection) > self.config.connection_window {
            tracker.connection_count = 0;
            tracker.last_connection = now;
        }

        // Check if over limit
        if tracker.connection_count >= self.config.max_connections_per_ip {
            tracker.total_violations += 1;

            // Ban after 3 violations
            if tracker.total_violations >= 3 {
                let mut banned = self.banned_ips.write().await;
                banned.insert(
                    ip,
                    BannedIp {
                        ban_time: now,
                        reason: "Excessive connection attempts".to_string(),
                        violations: tracker.total_violations,
                    },
                );
                warn!(%ip, violations = tracker.total_violations, "IP banned for excessive connections");
            }

            return Err(format!(
                "Connection rate limit exceeded: {}/{} in {:?}",
                tracker.connection_count,
                self.config.max_connections_per_ip,
                self.config.connection_window
            ));
        }

        tracker.connection_count += 1;
        tracker.last_connection = now;

        Ok(())
    }

    /// Register a new connection for rate limiting
    pub async fn register_connection(&self, client_id: u64) {
        let mut limiters = self.connection_limiters.write().await;
        limiters.insert(client_id, ConnectionRateLimiter::new(&self.config));
    }

    /// Remove connection from rate limiting
    pub async fn unregister_connection(&self, client_id: u64) {
        let mut limiters = self.connection_limiters.write().await;
        limiters.remove(&client_id);
    }

    /// Check if a packet from this connection is allowed
    pub async fn check_packet(&self, client_id: u64, packet_size: usize) -> Result<(), String> {
        let mut limiters = self.connection_limiters.write().await;

        let limiter = limiters
            .get_mut(&client_id)
            .ok_or_else(|| "Connection not registered for rate limiting".to_string())?;

        // Check packet rate limit
        if !limiter.packet_bucket.try_consume(1.0) {
            limiter.violations += 1;
            warn!(
                client_id,
                violations = limiter.violations,
                available_tokens = limiter.packet_bucket.available(),
                "Packet rate limit exceeded"
            );

            // Disconnect after too many violations
            if limiter.violations >= 10 {
                return Err("Too many rate limit violations - disconnecting".to_string());
            }

            return Err("Packet rate limit exceeded".to_string());
        }

        // Check bandwidth limit
        if !limiter.bandwidth_bucket.try_consume(packet_size as f64) {
            limiter.violations += 1;
            warn!(
                client_id,
                packet_size,
                violations = limiter.violations,
                "Bandwidth limit exceeded"
            );

            if limiter.violations >= 10 {
                return Err("Too many rate limit violations - disconnecting".to_string());
            }

            return Err("Bandwidth limit exceeded".to_string());
        }

        // Update stats
        limiter.total_packets += 1;
        limiter.total_bytes += packet_size as u64;

        Ok(())
    }

    /// Get connection statistics
    pub async fn get_connection_stats(&self, client_id: u64) -> Option<(u64, u64, u32)> {
        let limiters = self.connection_limiters.read().await;
        limiters.get(&client_id).map(|l| (l.total_packets, l.total_bytes, l.violations))
    }

    /// Manually ban an IP address
    pub async fn ban_ip(&self, ip: IpAddr, reason: String) {
        let mut banned = self.banned_ips.write().await;
        banned.insert(
            ip,
            BannedIp { ban_time: Instant::now(), reason: reason.clone(), violations: 0 },
        );
        warn!(%ip, %reason, "IP manually banned");
    }

    /// Unban an IP address
    pub async fn unban_ip(&self, ip: IpAddr) -> bool {
        let mut banned = self.banned_ips.write().await;
        banned.remove(&ip).is_some()
    }

    /// Get list of banned IPs with details (IP, reason, elapsed time, violations)
    pub async fn get_banned_ips(&self) -> Vec<(IpAddr, String, Duration, u32)> {
        let banned = self.banned_ips.read().await;
        banned
            .iter()
            .map(|(ip, ban)| (*ip, ban.reason.clone(), ban.ban_time.elapsed(), ban.violations))
            .collect()
    }

    /// Cleanup expired bans and old trackers
    pub async fn cleanup(&self) -> (usize, usize) {
        let mut banned_removed = 0;
        let mut trackers_removed = 0;

        // Remove expired bans
        let mut banned = self.banned_ips.write().await;
        banned.retain(|ip, ban| {
            let expired = ban.ban_time.elapsed() >= self.config.ban_duration;
            if expired {
                debug!(%ip, "Ban expired, removing");
                banned_removed += 1;
                false
            } else {
                true
            }
        });
        drop(banned);

        // Remove old IP trackers (1 hour old)
        let mut trackers = self.ip_trackers.write().await;
        trackers.retain(|_, tracker| {
            let old = tracker.last_connection.elapsed() > Duration::from_secs(3600);
            if old {
                trackers_removed += 1;
                false
            } else {
                true
            }
        });

        (banned_removed, trackers_removed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connection_rate_limit() {
        let config = RateLimitConfig {
            max_connections_per_ip: 3,
            connection_window: Duration::from_secs(60),
            ..Default::default()
        };
        let limiter = RateLimiter::new(config);
        let ip: IpAddr = "127.0.0.1".parse().unwrap();

        // First 3 connections should succeed
        assert!(limiter.check_connection(ip).await.is_ok());
        assert!(limiter.check_connection(ip).await.is_ok());
        assert!(limiter.check_connection(ip).await.is_ok());

        // 4th connection should fail
        assert!(limiter.check_connection(ip).await.is_err());
    }

    #[tokio::test]
    async fn test_packet_rate_limit() {
        let config = RateLimitConfig {
            max_packets_per_second: 10,
            burst_allowance: 5,
            ..Default::default()
        };
        let limiter = RateLimiter::new(config);
        let client_id = 1;

        limiter.register_connection(client_id).await;

        // Should allow burst (10 + 5 = 15 packets)
        for _ in 0..15 {
            assert!(limiter.check_packet(client_id, 100).await.is_ok());
        }

        // 16th packet should fail
        assert!(limiter.check_packet(client_id, 100).await.is_err());
    }

    #[tokio::test]
    async fn test_bandwidth_limit() {
        let config = RateLimitConfig {
            max_bandwidth_per_connection: 1000, // 1KB/sec
            max_packets_per_second: 1000,       // Allow many packets
            ..Default::default()
        };
        let limiter = RateLimiter::new(config);
        let client_id = 1;

        limiter.register_connection(client_id).await;

        // Should allow 1KB
        assert!(limiter.check_packet(client_id, 1000).await.is_ok());

        // Should reject next packet (bandwidth exceeded)
        assert!(limiter.check_packet(client_id, 100).await.is_err());
    }

    #[tokio::test]
    async fn test_ip_ban() {
        let config = RateLimitConfig {
            max_connections_per_ip: 2,
            ban_duration: Duration::from_secs(10),
            ..Default::default()
        };
        let limiter = RateLimiter::new(config);
        let ip: IpAddr = "192.168.1.1".parse().unwrap();

        // Exceed limit multiple times to trigger ban
        for _ in 0..3 {
            let _ = limiter.check_connection(ip).await;
            let _ = limiter.check_connection(ip).await;
            let _ = limiter.check_connection(ip).await; // This should trigger violation
        }

        // IP should now be banned
        let result = limiter.check_connection(ip).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("banned"));
    }

    #[tokio::test]
    async fn test_manual_ban_unban() {
        let limiter = RateLimiter::new(RateLimitConfig::default());
        let ip: IpAddr = "10.0.0.1".parse().unwrap();

        // Ban IP
        limiter.ban_ip(ip, "Testing".to_string()).await;

        // Should be blocked
        assert!(limiter.check_connection(ip).await.is_err());

        // Unban IP
        assert!(limiter.unban_ip(ip).await);

        // Should be allowed now
        assert!(limiter.check_connection(ip).await.is_ok());
    }

    #[tokio::test]
    async fn test_connection_stats() {
        let limiter = RateLimiter::new(RateLimitConfig::default());
        let client_id = 42;

        limiter.register_connection(client_id).await;

        // Send some packets
        for _ in 0..5 {
            let _ = limiter.check_packet(client_id, 100).await;
        }

        // Check stats
        let stats = limiter.get_connection_stats(client_id).await;
        assert!(stats.is_some());
        let (packets, bytes, _) = stats.unwrap();
        assert_eq!(packets, 5);
        assert_eq!(bytes, 500);
    }

    #[tokio::test]
    async fn test_cleanup() {
        let config =
            RateLimitConfig { ban_duration: Duration::from_millis(10), ..Default::default() };
        let limiter = RateLimiter::new(config);
        let ip: IpAddr = "1.2.3.4".parse().unwrap();

        // Ban IP
        limiter.ban_ip(ip, "Test".to_string()).await;
        assert_eq!(limiter.get_banned_ips().await.len(), 1);

        // Wait for ban to expire
        tokio::time::sleep(Duration::from_millis(20)).await;

        // Cleanup should remove expired ban
        let (banned_removed, _) = limiter.cleanup().await;
        assert_eq!(banned_removed, 1);
        assert_eq!(limiter.get_banned_ips().await.len(), 0);
    }
}
