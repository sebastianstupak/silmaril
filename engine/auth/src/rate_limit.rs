//! Rate limiting using token bucket algorithm.
//!
//! Implements IP-based rate limiting to prevent:
//! - Brute force attacks
//! - Credential stuffing
//! - DoS attacks
//!
//! Uses the `governor` crate for high-performance rate limiting.

use crate::error::AuthError;
use governor::{
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter as GovernorRateLimiter,
};
use nonzero_ext::nonzero;
use std::backtrace::Backtrace;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, warn};

/// Default rate limit: 5 attempts per 15 minutes per IP.
pub const DEFAULT_MAX_ATTEMPTS: u32 = 5;
pub const DEFAULT_WINDOW_MINUTES: u64 = 15;

/// Rate limiter for authentication attempts.
///
/// Uses token bucket algorithm for smooth rate limiting.
pub struct RateLimiter {
    limiter: Arc<GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock>>,
    max_attempts: u32,
    window_duration: Duration,
}

impl RateLimiter {
    /// Create a new rate limiter with default settings (5 attempts/15min).
    pub fn new() -> Self {
        Self::with_config(DEFAULT_MAX_ATTEMPTS, DEFAULT_WINDOW_MINUTES)
    }

    /// Create a new rate limiter with custom configuration.
    ///
    /// # Arguments
    ///
    /// * `max_attempts` - Maximum number of attempts allowed
    /// * `window_minutes` - Time window in minutes
    pub fn with_config(max_attempts: u32, window_minutes: u64) -> Self {
        let quota = Quota::with_period(Duration::from_secs(window_minutes * 60))
            .expect("Invalid rate limit configuration")
            .allow_burst(NonZeroU32::new(max_attempts).expect("max_attempts must be > 0"));

        Self {
            limiter: Arc::new(GovernorRateLimiter::direct(quota)),
            max_attempts,
            window_duration: Duration::from_secs(window_minutes * 60),
        }
    }

    /// Check if a request should be allowed.
    ///
    /// # Errors
    ///
    /// Returns [`AuthError::RateLimitExceeded`] if rate limit is exceeded.
    pub fn check(&self, identifier: &str) -> Result<(), AuthError> {
        match self.limiter.check() {
            Ok(_) => {
                debug!(identifier = identifier, "Rate limit check passed");
                Ok(())
            }
            Err(_) => {
                warn!(
                    identifier = identifier,
                    max_attempts = self.max_attempts,
                    window_minutes = self.window_duration.as_secs() / 60,
                    "Rate limit exceeded"
                );
                Err(AuthError::RateLimitExceeded {
                    retry_after_secs: self.window_duration.as_secs(),
                    #[cfg(feature = "backtrace")]
                    backtrace: Backtrace::capture(),
                })
            }
        }
    }

    /// Get the maximum number of attempts allowed.
    pub fn max_attempts(&self) -> u32 {
        self.max_attempts
    }

    /// Get the time window duration.
    pub fn window_duration(&self) -> Duration {
        self.window_duration
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

/// IP-based rate limiter that tracks limits per IP address.
///
/// This is more granular than the basic rate limiter and prevents one IP from
/// affecting others.
pub struct IpRateLimiter {
    limiters: Arc<std::sync::RwLock<std::collections::HashMap<String, RateLimiter>>>,
    max_attempts: u32,
    window_minutes: u64,
}

impl IpRateLimiter {
    /// Create a new IP-based rate limiter with default settings.
    pub fn new() -> Self {
        Self::with_config(DEFAULT_MAX_ATTEMPTS, DEFAULT_WINDOW_MINUTES)
    }

    /// Create a new IP-based rate limiter with custom configuration.
    pub fn with_config(max_attempts: u32, window_minutes: u64) -> Self {
        Self {
            limiters: Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())),
            max_attempts,
            window_minutes,
        }
    }

    /// Check if a request from a specific IP should be allowed.
    ///
    /// # Errors
    ///
    /// Returns [`AuthError::RateLimitExceeded`] if rate limit is exceeded.
    pub fn check(&self, ip: &str) -> Result<(), AuthError> {
        // Try to get existing limiter
        {
            let limiters = self.limiters.read().unwrap();
            if let Some(limiter) = limiters.get(ip) {
                return limiter.check(ip);
            }
        }

        // Create new limiter for this IP
        {
            let mut limiters = self.limiters.write().unwrap();
            let limiter = RateLimiter::with_config(self.max_attempts, self.window_minutes);
            limiters.insert(ip.to_string(), limiter);
        }

        // Check again (should always succeed for first attempt)
        let limiters = self.limiters.read().unwrap();
        limiters.get(ip).unwrap().check(ip)
    }

    /// Clear rate limit for a specific IP.
    pub fn reset_ip(&self, ip: &str) {
        let mut limiters = self.limiters.write().unwrap();
        limiters.remove(ip);
        debug!(ip = ip, "Rate limit reset");
    }

    /// Clear all rate limits (for testing or cleanup).
    pub fn clear_all(&self) {
        let mut limiters = self.limiters.write().unwrap();
        limiters.clear();
        debug!("All rate limits cleared");
    }

    /// Get the number of IPs currently being tracked.
    pub fn tracked_ips(&self) -> usize {
        let limiters = self.limiters.read().unwrap();
        limiters.len()
    }
}

impl Default for IpRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_allows_within_limit() {
        let limiter = RateLimiter::with_config(5, 1); // 5 attempts per minute

        // First 5 attempts should succeed
        for _ in 0..5 {
            assert!(limiter.check("test").is_ok());
        }
    }

    #[test]
    fn test_rate_limiter_blocks_over_limit() {
        let limiter = RateLimiter::with_config(3, 1); // 3 attempts per minute

        // First 3 attempts should succeed
        for _ in 0..3 {
            assert!(limiter.check("test").is_ok());
        }

        // 4th attempt should fail
        let result = limiter.check("test");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AuthError::RateLimitExceeded { .. }));
    }

    #[test]
    fn test_rate_limiter_config() {
        let limiter = RateLimiter::with_config(10, 5);
        assert_eq!(limiter.max_attempts(), 10);
        assert_eq!(limiter.window_duration(), Duration::from_secs(5 * 60));
    }

    #[test]
    fn test_ip_rate_limiter_separate_ips() {
        let limiter = IpRateLimiter::with_config(2, 1); // 2 attempts per minute

        // IP1 uses 2 attempts
        assert!(limiter.check("192.168.1.1").is_ok());
        assert!(limiter.check("192.168.1.1").is_ok());

        // IP2 should still have 2 attempts
        assert!(limiter.check("192.168.1.2").is_ok());
        assert!(limiter.check("192.168.1.2").is_ok());

        // Both should be blocked now
        assert!(limiter.check("192.168.1.1").is_err());
        assert!(limiter.check("192.168.1.2").is_err());
    }

    #[test]
    fn test_ip_rate_limiter_reset() {
        let limiter = IpRateLimiter::with_config(2, 1);

        // Use up attempts
        assert!(limiter.check("192.168.1.1").is_ok());
        assert!(limiter.check("192.168.1.1").is_ok());
        assert!(limiter.check("192.168.1.1").is_err());

        // Reset and try again
        limiter.reset_ip("192.168.1.1");
        assert!(limiter.check("192.168.1.1").is_ok());
    }

    #[test]
    fn test_ip_rate_limiter_clear_all() {
        let limiter = IpRateLimiter::with_config(1, 1);

        // Use up attempts for multiple IPs
        assert!(limiter.check("192.168.1.1").is_ok());
        assert!(limiter.check("192.168.1.2").is_ok());
        assert!(limiter.check("192.168.1.1").is_err());
        assert!(limiter.check("192.168.1.2").is_err());

        // Clear all and try again
        limiter.clear_all();
        assert!(limiter.check("192.168.1.1").is_ok());
        assert!(limiter.check("192.168.1.2").is_ok());
    }

    #[test]
    fn test_ip_rate_limiter_tracked_ips() {
        let limiter = IpRateLimiter::new();
        assert_eq!(limiter.tracked_ips(), 0);

        limiter.check("192.168.1.1").unwrap();
        assert_eq!(limiter.tracked_ips(), 1);

        limiter.check("192.168.1.2").unwrap();
        assert_eq!(limiter.tracked_ips(), 2);

        limiter.check("192.168.1.1").unwrap(); // Same IP, no new tracking
        assert_eq!(limiter.tracked_ips(), 2);
    }

    #[test]
    fn test_default_rate_limiter() {
        let limiter = RateLimiter::default();
        assert_eq!(limiter.max_attempts(), DEFAULT_MAX_ATTEMPTS);
        assert_eq!(limiter.window_duration(), Duration::from_secs(DEFAULT_WINDOW_MINUTES * 60));
    }

    #[test]
    fn test_default_ip_rate_limiter() {
        let limiter = IpRateLimiter::default();
        // Should allow default number of attempts
        for _ in 0..DEFAULT_MAX_ATTEMPTS {
            assert!(limiter.check("192.168.1.1").is_ok());
        }
        // Should block on next attempt
        assert!(limiter.check("192.168.1.1").is_err());
    }
}
