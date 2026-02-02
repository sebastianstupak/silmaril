//! Session management with in-memory storage (Redis-compatible design).
//!
//! Implements:
//! - Session creation and storage
//! - Idle timeout (30 minutes)
//! - Absolute timeout (24 hours)
//! - Concurrent session limits
//! - Session cleanup

use crate::error::AuthError;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::backtrace::Backtrace;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Default idle timeout (30 minutes).
pub const DEFAULT_IDLE_TIMEOUT_MINUTES: i64 = 30;

/// Default absolute timeout (24 hours).
pub const DEFAULT_ABSOLUTE_TIMEOUT_HOURS: i64 = 24;

/// Default maximum concurrent sessions per user.
pub const DEFAULT_MAX_SESSIONS_PER_USER: usize = 5;

/// Session data stored in memory.
///
/// In production, this should be stored in Redis for:
/// - Distributed session storage
/// - Automatic expiration
/// - High performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Session ID
    pub id: String,
    /// User ID this session belongs to
    pub user_id: String,
    /// Session creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last activity timestamp
    pub last_activity: DateTime<Utc>,
    /// IP address (for security logging)
    pub ip_address: String,
    /// User agent (for security logging)
    pub user_agent: String,
    /// Custom session data (application-specific)
    pub data: HashMap<String, String>,
}

impl Session {
    /// Create a new session.
    pub fn new(user_id: String, ip_address: String, user_agent: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            user_id,
            created_at: now,
            last_activity: now,
            ip_address,
            user_agent,
            data: HashMap::new(),
        }
    }

    /// Check if session has exceeded idle timeout.
    pub fn is_idle_expired(&self, idle_timeout_minutes: i64) -> bool {
        let now = Utc::now();
        let idle_duration = now - self.last_activity;
        idle_duration > Duration::minutes(idle_timeout_minutes)
    }

    /// Check if session has exceeded absolute timeout.
    pub fn is_absolutely_expired(&self, absolute_timeout_hours: i64) -> bool {
        let now = Utc::now();
        let absolute_duration = now - self.created_at;
        absolute_duration > Duration::hours(absolute_timeout_hours)
    }

    /// Update last activity timestamp.
    pub fn touch(&mut self) {
        self.last_activity = Utc::now();
    }
}

/// In-memory session store.
///
/// Thread-safe session storage with automatic cleanup.
/// Design is Redis-compatible for easy migration to distributed storage.
pub struct SessionStore {
    sessions: Arc<RwLock<HashMap<String, Session>>>,
    user_sessions: Arc<RwLock<HashMap<String, Vec<String>>>>,
    idle_timeout_minutes: i64,
    absolute_timeout_hours: i64,
    max_sessions_per_user: usize,
}

impl SessionStore {
    /// Create a new session store with default timeouts.
    pub fn new() -> Self {
        Self::with_config(
            DEFAULT_IDLE_TIMEOUT_MINUTES,
            DEFAULT_ABSOLUTE_TIMEOUT_HOURS,
            DEFAULT_MAX_SESSIONS_PER_USER,
        )
    }

    /// Create a new session store with custom configuration.
    pub fn with_config(
        idle_timeout_minutes: i64,
        absolute_timeout_hours: i64,
        max_sessions_per_user: usize,
    ) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            user_sessions: Arc::new(RwLock::new(HashMap::new())),
            idle_timeout_minutes,
            absolute_timeout_hours,
            max_sessions_per_user,
        }
    }

    /// Create a new session for a user.
    ///
    /// # Errors
    ///
    /// Returns [`AuthError::MaxSessionsReached`] if user has too many concurrent sessions.
    pub fn create_session(
        &self,
        user_id: String,
        ip_address: String,
        user_agent: String,
    ) -> Result<Session, AuthError> {
        // Check concurrent session limit
        {
            let user_sessions = self.user_sessions.read().unwrap();
            if let Some(sessions) = user_sessions.get(&user_id) {
                if sessions.len() >= self.max_sessions_per_user {
                    warn!(
                        user_id = %user_id,
                        active_sessions = sessions.len(),
                        max_sessions = self.max_sessions_per_user,
                        "Maximum concurrent sessions reached"
                    );
                    return Err(AuthError::MaxSessionsReached {
                        user_id,
                        max_sessions: self.max_sessions_per_user,
                        #[cfg(feature = "backtrace")]
                        backtrace: Backtrace::capture(),
                    });
                }
            }
        }

        let session = Session::new(user_id.clone(), ip_address, user_agent);
        let session_id = session.id.clone();

        // Store session
        {
            let mut sessions = self.sessions.write().unwrap();
            sessions.insert(session_id.clone(), session.clone());
        }

        // Track user sessions
        {
            let mut user_sessions = self.user_sessions.write().unwrap();
            user_sessions
                .entry(user_id.clone())
                .or_insert_with(Vec::new)
                .push(session_id.clone());
        }

        info!(
            user_id = %user_id,
            session_id = %session_id,
            "Session created"
        );

        Ok(session)
    }

    /// Get a session by ID.
    ///
    /// Automatically validates timeouts and touches the session.
    ///
    /// # Errors
    ///
    /// Returns [`AuthError::SessionNotFound`] if session doesn't exist.
    /// Returns [`AuthError::SessionExpired`] if session has expired.
    pub fn get_session(&self, session_id: &str) -> Result<Session, AuthError> {
        let mut sessions = self.sessions.write().unwrap();

        let session = sessions.get_mut(session_id).ok_or_else(|| AuthError::SessionNotFound {
            session_id: session_id.to_string(),
            #[cfg(feature = "backtrace")]
            backtrace: Backtrace::capture(),
        })?;

        // Check idle timeout
        if session.is_idle_expired(self.idle_timeout_minutes) {
            let expired_at = session.last_activity + Duration::minutes(self.idle_timeout_minutes);
            self.delete_session_internal(session_id, &mut sessions);
            return Err(AuthError::SessionExpired {
                session_id: session_id.to_string(),
                expired_at,
                #[cfg(feature = "backtrace")]
                backtrace: Backtrace::capture(),
            });
        }

        // Check absolute timeout
        if session.is_absolutely_expired(self.absolute_timeout_hours) {
            let expired_at = session.created_at + Duration::hours(self.absolute_timeout_hours);
            self.delete_session_internal(session_id, &mut sessions);
            return Err(AuthError::SessionExpired {
                session_id: session_id.to_string(),
                expired_at,
                #[cfg(feature = "backtrace")]
                backtrace: Backtrace::capture(),
            });
        }

        // Touch session
        session.touch();
        debug!(session_id = %session_id, "Session accessed");

        Ok(session.clone())
    }

    /// Delete a session.
    pub fn delete_session(&self, session_id: &str) {
        let mut sessions = self.sessions.write().unwrap();
        self.delete_session_internal(session_id, &mut sessions);
    }

    fn delete_session_internal(&self, session_id: &str, sessions: &mut HashMap<String, Session>) {
        if let Some(session) = sessions.remove(session_id) {
            // Remove from user sessions
            let mut user_sessions = self.user_sessions.write().unwrap();
            if let Some(user_session_ids) = user_sessions.get_mut(&session.user_id) {
                user_session_ids.retain(|id| id != session_id);
                if user_session_ids.is_empty() {
                    user_sessions.remove(&session.user_id);
                }
            }

            info!(
                session_id = %session_id,
                user_id = %session.user_id,
                "Session deleted"
            );
        }
    }

    /// Delete all sessions for a user.
    pub fn delete_user_sessions(&self, user_id: &str) {
        let mut user_sessions = self.user_sessions.write().unwrap();
        if let Some(session_ids) = user_sessions.remove(user_id) {
            let mut sessions = self.sessions.write().unwrap();
            for session_id in &session_ids {
                sessions.remove(session_id);
            }
            info!(
                user_id = %user_id,
                session_count = session_ids.len(),
                "All user sessions deleted"
            );
        }
    }

    /// Get all active sessions for a user.
    pub fn get_user_sessions(&self, user_id: &str) -> Vec<Session> {
        let user_sessions = self.user_sessions.read().unwrap();
        let sessions = self.sessions.read().unwrap();

        if let Some(session_ids) = user_sessions.get(user_id) {
            session_ids.iter().filter_map(|id| sessions.get(id).cloned()).collect()
        } else {
            Vec::new()
        }
    }

    /// Clean up expired sessions.
    ///
    /// Should be called periodically (e.g., every 5 minutes).
    pub fn cleanup_expired_sessions(&self) -> usize {
        let mut sessions = self.sessions.write().unwrap();
        let mut expired_sessions = Vec::new();

        for (session_id, session) in sessions.iter() {
            if session.is_idle_expired(self.idle_timeout_minutes)
                || session.is_absolutely_expired(self.absolute_timeout_hours)
            {
                expired_sessions.push(session_id.clone());
            }
        }

        let count = expired_sessions.len();
        for session_id in expired_sessions {
            self.delete_session_internal(&session_id, &mut sessions);
        }

        if count > 0 {
            info!(expired_count = count, "Cleaned up expired sessions");
        }

        count
    }

    /// Get total session count (for monitoring).
    pub fn session_count(&self) -> usize {
        let sessions = self.sessions.read().unwrap();
        sessions.len()
    }
}

impl Default for SessionStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_session() {
        let store = SessionStore::new();
        let session = store
            .create_session(
                "user123".to_string(),
                "127.0.0.1".to_string(),
                "TestAgent/1.0".to_string(),
            )
            .unwrap();

        assert_eq!(session.user_id, "user123");
        assert_eq!(session.ip_address, "127.0.0.1");
        assert_eq!(session.user_agent, "TestAgent/1.0");
    }

    #[test]
    fn test_get_session() {
        let store = SessionStore::new();
        let session = store
            .create_session(
                "user123".to_string(),
                "127.0.0.1".to_string(),
                "TestAgent/1.0".to_string(),
            )
            .unwrap();

        let retrieved = store.get_session(&session.id).unwrap();
        assert_eq!(retrieved.id, session.id);
        assert_eq!(retrieved.user_id, session.user_id);
    }

    #[test]
    fn test_get_nonexistent_session() {
        let store = SessionStore::new();
        let result = store.get_session("nonexistent");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AuthError::SessionNotFound { .. }));
    }

    #[test]
    fn test_delete_session() {
        let store = SessionStore::new();
        let session = store
            .create_session(
                "user123".to_string(),
                "127.0.0.1".to_string(),
                "TestAgent/1.0".to_string(),
            )
            .unwrap();

        store.delete_session(&session.id);

        let result = store.get_session(&session.id);
        assert!(result.is_err());
    }

    #[test]
    fn test_max_sessions_reached() {
        let store = SessionStore::with_config(30, 24, 2); // Max 2 sessions

        let _session1 = store
            .create_session(
                "user123".to_string(),
                "127.0.0.1".to_string(),
                "TestAgent/1.0".to_string(),
            )
            .unwrap();

        let _session2 = store
            .create_session(
                "user123".to_string(),
                "127.0.0.1".to_string(),
                "TestAgent/1.0".to_string(),
            )
            .unwrap();

        // 3rd session should fail
        let result = store.create_session(
            "user123".to_string(),
            "127.0.0.1".to_string(),
            "TestAgent/1.0".to_string(),
        );
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AuthError::MaxSessionsReached { .. }));
    }

    #[test]
    fn test_delete_user_sessions() {
        let store = SessionStore::new();

        let _session1 = store
            .create_session(
                "user123".to_string(),
                "127.0.0.1".to_string(),
                "TestAgent/1.0".to_string(),
            )
            .unwrap();

        let _session2 = store
            .create_session(
                "user123".to_string(),
                "127.0.0.2".to_string(),
                "TestAgent/1.0".to_string(),
            )
            .unwrap();

        assert_eq!(store.get_user_sessions("user123").len(), 2);

        store.delete_user_sessions("user123");

        assert_eq!(store.get_user_sessions("user123").len(), 0);
    }

    #[test]
    fn test_idle_timeout() {
        let store = SessionStore::with_config(0, 24, 5); // 0 minute idle timeout

        let session = store
            .create_session(
                "user123".to_string(),
                "127.0.0.1".to_string(),
                "TestAgent/1.0".to_string(),
            )
            .unwrap();

        // Wait a bit (idle timeout is 0, so any wait should expire it)
        std::thread::sleep(std::time::Duration::from_millis(100));

        let result = store.get_session(&session.id);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AuthError::SessionExpired { .. }));
    }

    #[test]
    fn test_session_touch() {
        let store = SessionStore::with_config(1, 24, 5); // 1 minute idle timeout

        let session = store
            .create_session(
                "user123".to_string(),
                "127.0.0.1".to_string(),
                "TestAgent/1.0".to_string(),
            )
            .unwrap();

        // Keep accessing the session (touching it)
        for _ in 0..3 {
            std::thread::sleep(std::time::Duration::from_millis(100));
            let _ = store.get_session(&session.id).unwrap(); // Should succeed
        }
    }

    #[test]
    fn test_cleanup_expired_sessions() {
        let store = SessionStore::with_config(0, 24, 5); // 0 minute idle timeout

        let _session1 = store
            .create_session(
                "user1".to_string(),
                "127.0.0.1".to_string(),
                "TestAgent/1.0".to_string(),
            )
            .unwrap();

        let _session2 = store
            .create_session(
                "user2".to_string(),
                "127.0.0.2".to_string(),
                "TestAgent/1.0".to_string(),
            )
            .unwrap();

        assert_eq!(store.session_count(), 2);

        std::thread::sleep(std::time::Duration::from_millis(100));

        let cleaned = store.cleanup_expired_sessions();
        assert_eq!(cleaned, 2);
        assert_eq!(store.session_count(), 0);
    }

    #[test]
    fn test_get_user_sessions() {
        let store = SessionStore::new();

        let _session1 = store
            .create_session(
                "user123".to_string(),
                "127.0.0.1".to_string(),
                "TestAgent/1.0".to_string(),
            )
            .unwrap();

        let _session2 = store
            .create_session(
                "user123".to_string(),
                "127.0.0.2".to_string(),
                "TestAgent/1.0".to_string(),
            )
            .unwrap();

        let sessions = store.get_user_sessions("user123");
        assert_eq!(sessions.len(), 2);
    }
}
