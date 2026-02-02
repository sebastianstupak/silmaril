//! TLS session management
//!
//! Provides session caching, resumption, and lifecycle management for optimal performance.

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info};

/// Session ID type
pub type SessionId = Vec<u8>;

/// TLS session ticket
#[derive(Debug, Clone)]
pub struct SessionTicket {
    /// Session ID
    pub id: SessionId,
    /// Session data (opaque)
    pub data: Vec<u8>,
    /// Creation time
    pub created_at: Instant,
    /// Last used time
    pub last_used: Instant,
    /// Use count
    pub use_count: u32,
}

impl SessionTicket {
    /// Create a new session ticket
    pub fn new(id: SessionId, data: Vec<u8>) -> Self {
        let now = Instant::now();
        Self { id, data, created_at: now, last_used: now, use_count: 0 }
    }

    /// Check if session is expired
    pub fn is_expired(&self, max_age: Duration) -> bool {
        self.created_at.elapsed() > max_age
    }

    /// Mark session as used
    pub fn mark_used(&mut self) {
        self.last_used = Instant::now();
        self.use_count += 1;
    }
}

/// TLS session cache
pub struct SessionCache {
    /// Cached sessions indexed by session ID
    sessions: Arc<RwLock<HashMap<SessionId, SessionTicket>>>,
    /// Maximum session age
    max_age: Duration,
    /// Maximum number of cached sessions
    max_sessions: usize,
}

impl SessionCache {
    /// Create a new session cache
    pub fn new(max_age: Duration, max_sessions: usize) -> Self {
        info!(
            max_age_secs = max_age.as_secs(),
            max_sessions = max_sessions,
            "Creating TLS session cache"
        );

        Self { sessions: Arc::new(RwLock::new(HashMap::new())), max_age, max_sessions }
    }

    /// Get a session ticket by ID
    pub fn get(&self, id: &SessionId) -> Option<SessionTicket> {
        let mut sessions = self.sessions.write();

        if let Some(ticket) = sessions.get_mut(id) {
            // Check if expired
            if ticket.is_expired(self.max_age) {
                debug!(session_id = ?id, "Session expired, removing from cache");
                sessions.remove(id);
                return None;
            }

            // Mark as used
            ticket.mark_used();
            Some(ticket.clone())
        } else {
            None
        }
    }

    /// Store a session ticket
    pub fn put(&self, ticket: SessionTicket) {
        let mut sessions = self.sessions.write();

        // Enforce max sessions limit
        if sessions.len() >= self.max_sessions && !sessions.contains_key(&ticket.id) {
            // Remove oldest session (by last used time)
            if let Some((oldest_id, _)) = sessions
                .iter()
                .min_by_key(|(_, t)| t.last_used)
                .map(|(id, t)| (id.clone(), t.clone()))
            {
                debug!(session_id = ?oldest_id, "Evicting oldest session from cache");
                sessions.remove(&oldest_id);
            }
        }

        debug!(session_id = ?ticket.id, "Storing session in cache");
        sessions.insert(ticket.id.clone(), ticket);
    }

    /// Remove a session from cache
    pub fn remove(&self, id: &SessionId) {
        if self.sessions.write().remove(id).is_some() {
            debug!(session_id = ?id, "Session removed from cache");
        }
    }

    /// Clean up expired sessions
    pub fn cleanup_expired(&self) -> usize {
        let mut sessions = self.sessions.write();
        let before_count = sessions.len();

        sessions.retain(|id, ticket| {
            let keep = !ticket.is_expired(self.max_age);
            if !keep {
                debug!(session_id = ?id, "Removing expired session");
            }
            keep
        });

        let removed = before_count - sessions.len();
        if removed > 0 {
            info!(removed = removed, "Cleaned up expired sessions");
        }
        removed
    }

    /// Get cache statistics
    pub fn stats(&self) -> SessionCacheStats {
        let sessions = self.sessions.read();
        let total = sessions.len();
        let expired = sessions.values().filter(|t| t.is_expired(self.max_age)).count();

        SessionCacheStats {
            total_sessions: total,
            expired_sessions: expired,
            active_sessions: total - expired,
            max_sessions: self.max_sessions,
        }
    }

    /// Clear all sessions
    pub fn clear(&self) {
        let count = self.sessions.write().len();
        self.sessions.write().clear();
        info!(cleared = count, "Session cache cleared");
    }
}

impl Default for SessionCache {
    fn default() -> Self {
        Self::new(
            Duration::from_secs(24 * 60 * 60), // 24 hours
            1000,                              // Max 1000 sessions
        )
    }
}

/// Session cache statistics
#[derive(Debug, Clone, Copy)]
pub struct SessionCacheStats {
    /// Total number of sessions in cache
    pub total_sessions: usize,
    /// Number of expired sessions
    pub expired_sessions: usize,
    /// Number of active (non-expired) sessions
    pub active_sessions: usize,
    /// Maximum allowed sessions
    pub max_sessions: usize,
}

impl SessionCacheStats {
    /// Get cache utilization as percentage
    pub fn utilization(&self) -> f32 {
        if self.max_sessions == 0 {
            0.0
        } else {
            (self.total_sessions as f32 / self.max_sessions as f32) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_ticket_creation() {
        let ticket = SessionTicket::new(vec![1, 2, 3], vec![4, 5, 6]);
        assert_eq!(ticket.id, vec![1, 2, 3]);
        assert_eq!(ticket.data, vec![4, 5, 6]);
        assert_eq!(ticket.use_count, 0);
    }

    #[test]
    fn test_session_ticket_expiration() {
        let ticket = SessionTicket::new(vec![1, 2, 3], vec![4, 5, 6]);
        assert!(!ticket.is_expired(Duration::from_secs(1)));

        // Create expired ticket (simulate old creation time)
        let mut old_ticket = SessionTicket::new(vec![1, 2, 3], vec![4, 5, 6]);
        old_ticket.created_at = Instant::now() - Duration::from_secs(100);
        assert!(old_ticket.is_expired(Duration::from_secs(50)));
    }

    #[test]
    fn test_session_ticket_mark_used() {
        let mut ticket = SessionTicket::new(vec![1, 2, 3], vec![4, 5, 6]);
        assert_eq!(ticket.use_count, 0);

        ticket.mark_used();
        assert_eq!(ticket.use_count, 1);

        ticket.mark_used();
        assert_eq!(ticket.use_count, 2);
    }

    #[test]
    fn test_session_cache_put_get() {
        let cache = SessionCache::new(Duration::from_secs(60), 10);
        let ticket = SessionTicket::new(vec![1, 2, 3], vec![4, 5, 6]);

        cache.put(ticket.clone());

        let retrieved = cache.get(&vec![1, 2, 3]);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, vec![1, 2, 3]);
    }

    #[test]
    fn test_session_cache_expiration() {
        let cache = SessionCache::new(Duration::from_millis(100), 10);
        let mut ticket = SessionTicket::new(vec![1, 2, 3], vec![4, 5, 6]);
        ticket.created_at = Instant::now() - Duration::from_secs(1);

        cache.put(ticket);

        // Should return None because session is expired
        let retrieved = cache.get(&vec![1, 2, 3]);
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_session_cache_max_sessions() {
        let cache = SessionCache::new(Duration::from_secs(60), 3);

        // Add 4 sessions (should evict oldest)
        for i in 0..4 {
            let ticket = SessionTicket::new(vec![i], vec![i]);
            cache.put(ticket);
        }

        let stats = cache.stats();
        assert_eq!(stats.total_sessions, 3); // Max limit enforced
    }

    #[test]
    fn test_session_cache_remove() {
        let cache = SessionCache::new(Duration::from_secs(60), 10);
        let ticket = SessionTicket::new(vec![1, 2, 3], vec![4, 5, 6]);

        cache.put(ticket);
        assert!(cache.get(&vec![1, 2, 3]).is_some());

        cache.remove(&vec![1, 2, 3]);
        assert!(cache.get(&vec![1, 2, 3]).is_none());
    }

    #[test]
    fn test_session_cache_cleanup() {
        let cache = SessionCache::new(Duration::from_millis(100), 10);

        // Add some sessions
        for i in 0..3 {
            let ticket = SessionTicket::new(vec![i], vec![i]);
            cache.put(ticket);
        }

        // Add an expired session
        let mut expired_ticket = SessionTicket::new(vec![99], vec![99]);
        expired_ticket.created_at = Instant::now() - Duration::from_secs(1);
        cache.put(expired_ticket);

        let removed = cache.cleanup_expired();
        assert_eq!(removed, 1);

        let stats = cache.stats();
        assert_eq!(stats.total_sessions, 3);
    }

    #[test]
    fn test_session_cache_stats() {
        let cache = SessionCache::new(Duration::from_secs(60), 10);

        for i in 0..5 {
            let ticket = SessionTicket::new(vec![i], vec![i]);
            cache.put(ticket);
        }

        let stats = cache.stats();
        assert_eq!(stats.total_sessions, 5);
        assert_eq!(stats.max_sessions, 10);
        assert_eq!(stats.utilization(), 50.0);
    }

    #[test]
    fn test_session_cache_clear() {
        let cache = SessionCache::new(Duration::from_secs(60), 10);

        for i in 0..5 {
            let ticket = SessionTicket::new(vec![i], vec![i]);
            cache.put(ticket);
        }

        assert_eq!(cache.stats().total_sessions, 5);

        cache.clear();
        assert_eq!(cache.stats().total_sessions, 0);
    }
}
