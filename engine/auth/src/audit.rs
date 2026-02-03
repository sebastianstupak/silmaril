//! Audit logging for authentication events.
//!
//! Tracks all security-relevant events for compliance and threat detection.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

/// Authentication event type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthEventType {
    /// User registration
    UserRegistered,
    /// Successful login
    LoginSuccess,
    /// Failed login attempt
    LoginFailed,
    /// Logout
    Logout,
    /// Password changed
    PasswordChanged,
    /// Password reset requested
    PasswordResetRequested,
    /// Password reset completed
    PasswordResetCompleted,
    /// MFA enabled
    MfaEnabled,
    /// MFA disabled
    MfaDisabled,
    /// MFA verification success
    MfaVerifySuccess,
    /// MFA verification failed
    MfaVerifyFailed,
    /// OAuth account linked
    OAuthLinked,
    /// OAuth account unlinked
    OAuthUnlinked,
    /// Token refreshed
    TokenRefreshed,
    /// Token revoked
    TokenRevoked,
    /// Session created
    SessionCreated,
    /// Session expired
    SessionExpired,
    /// Account locked
    AccountLocked,
    /// Account unlocked
    AccountUnlocked,
    /// Email verified
    EmailVerified,
    /// Rate limit exceeded
    RateLimitExceeded,
}

impl AuthEventType {
    /// Check if this event type indicates a security threat.
    #[must_use]
    pub fn is_threat(&self) -> bool {
        matches!(
            self,
            Self::LoginFailed
                | Self::MfaVerifyFailed
                | Self::AccountLocked
                | Self::RateLimitExceeded
        )
    }
}

/// Audit log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Event ID
    pub id: uuid::Uuid,
    /// Event type
    pub event_type: AuthEventType,
    /// User ID (if applicable)
    pub user_id: Option<String>,
    /// Username (if applicable)
    pub username: Option<String>,
    /// IP address
    pub ip_address: String,
    /// User agent
    pub user_agent: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Additional context
    pub details: String,
    /// Success/failure
    pub success: bool,
}

impl AuditEvent {
    /// Create a new audit event.
    #[must_use]
    pub fn new(
        event_type: AuthEventType,
        user_id: Option<String>,
        username: Option<String>,
        ip_address: String,
        user_agent: String,
        details: String,
        success: bool,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            event_type,
            user_id,
            username,
            ip_address,
            user_agent,
            timestamp: Utc::now(),
            details,
            success,
        }
    }

    /// Log this event using structured logging.
    pub fn log(&self) {
        if self.event_type.is_threat() {
            warn!(
                event_id = %self.id,
                event_type = ?self.event_type,
                user_id = ?self.user_id,
                username = ?self.username,
                ip_address = %self.ip_address,
                user_agent = %self.user_agent,
                details = %self.details,
                success = self.success,
                "Security threat event"
            );
        } else {
            info!(
                event_id = %self.id,
                event_type = ?self.event_type,
                user_id = ?self.user_id,
                username = ?self.username,
                ip_address = %self.ip_address,
                details = %self.details,
                success = self.success,
                "Auth event"
            );
        }
    }
}

/// Audit logger for authentication events.
///
/// In production, this should persist events to a database or log aggregation service.
pub struct AuditLogger {
    events: std::sync::Arc<std::sync::RwLock<Vec<AuditEvent>>>,
}

impl AuditLogger {
    /// Create a new audit logger.
    #[must_use]
    pub fn new() -> Self {
        Self { events: std::sync::Arc::new(std::sync::RwLock::new(Vec::new())) }
    }

    /// Log an authentication event.
    pub fn log_event(&self, event: AuditEvent) {
        event.log();

        let mut events = self.events.write().unwrap();
        events.push(event);
    }

    /// Log a user registration event.
    pub fn log_registration(
        &self,
        user_id: String,
        username: String,
        ip_address: String,
        user_agent: String,
    ) {
        let event = AuditEvent::new(
            AuthEventType::UserRegistered,
            Some(user_id),
            Some(username),
            ip_address,
            user_agent,
            "User registered".to_string(),
            true,
        );
        self.log_event(event);
    }

    /// Log a successful login.
    pub fn log_login_success(
        &self,
        user_id: String,
        username: String,
        ip_address: String,
        user_agent: String,
    ) {
        let event = AuditEvent::new(
            AuthEventType::LoginSuccess,
            Some(user_id),
            Some(username),
            ip_address,
            user_agent,
            "Login successful".to_string(),
            true,
        );
        self.log_event(event);
    }

    /// Log a failed login attempt.
    #[allow(clippy::needless_pass_by_value)]
    pub fn log_login_failed(
        &self,
        username: String,
        ip_address: String,
        user_agent: String,
        reason: String,
    ) {
        let event = AuditEvent::new(
            AuthEventType::LoginFailed,
            None,
            Some(username),
            ip_address,
            user_agent,
            format!("Login failed: {reason}"),
            false,
        );
        self.log_event(event);
    }

    /// Log a logout event.
    pub fn log_logout(
        &self,
        user_id: String,
        username: String,
        ip_address: String,
        user_agent: String,
    ) {
        let event = AuditEvent::new(
            AuthEventType::Logout,
            Some(user_id),
            Some(username),
            ip_address,
            user_agent,
            "Logout".to_string(),
            true,
        );
        self.log_event(event);
    }

    /// Log a password change.
    pub fn log_password_changed(&self, user_id: String, ip_address: String, user_agent: String) {
        let event = AuditEvent::new(
            AuthEventType::PasswordChanged,
            Some(user_id),
            None,
            ip_address,
            user_agent,
            "Password changed".to_string(),
            true,
        );
        self.log_event(event);
    }

    /// Log MFA enabled.
    pub fn log_mfa_enabled(&self, user_id: String, ip_address: String, user_agent: String) {
        let event = AuditEvent::new(
            AuthEventType::MfaEnabled,
            Some(user_id),
            None,
            ip_address,
            user_agent,
            "MFA enabled".to_string(),
            true,
        );
        self.log_event(event);
    }

    /// Log account locked.
    #[allow(clippy::needless_pass_by_value)]
    pub fn log_account_locked(
        &self,
        user_id: String,
        username: String,
        ip_address: String,
        user_agent: String,
        reason: String,
    ) {
        let event = AuditEvent::new(
            AuthEventType::AccountLocked,
            Some(user_id),
            Some(username),
            ip_address,
            user_agent,
            format!("Account locked: {reason}"),
            true,
        );
        self.log_event(event);
    }

    /// Log rate limit exceeded.
    pub fn log_rate_limit_exceeded(&self, ip_address: String, user_agent: String) {
        let event = AuditEvent::new(
            AuthEventType::RateLimitExceeded,
            None,
            None,
            ip_address,
            user_agent,
            "Rate limit exceeded".to_string(),
            false,
        );
        self.log_event(event);
    }

    /// Get all events (for testing/monitoring).
    #[must_use]
    pub fn get_events(&self) -> Vec<AuditEvent> {
        let events = self.events.read().unwrap();
        events.clone()
    }

    /// Get events for a specific user.
    #[must_use]
    pub fn get_user_events(&self, user_id: &str) -> Vec<AuditEvent> {
        let events = self.events.read().unwrap();
        events
            .iter()
            .filter(|e| e.user_id.as_deref() == Some(user_id))
            .cloned()
            .collect()
    }

    /// Get events by type.
    #[must_use]
    pub fn get_events_by_type(&self, event_type: AuthEventType) -> Vec<AuditEvent> {
        let events = self.events.read().unwrap();
        events.iter().filter(|e| e.event_type == event_type).cloned().collect()
    }

    /// Get threat events (failed logins, lockouts, etc.).
    #[must_use]
    pub fn get_threat_events(&self) -> Vec<AuditEvent> {
        let events = self.events.read().unwrap();
        events.iter().filter(|e| e.event_type.is_threat()).cloned().collect()
    }

    /// Get events within a time range.
    #[must_use]
    pub fn get_events_since(&self, since: DateTime<Utc>) -> Vec<AuditEvent> {
        let events = self.events.read().unwrap();
        events.iter().filter(|e| e.timestamp >= since).cloned().collect()
    }

    /// Clear all events (for testing).
    pub fn clear(&self) {
        let mut events = self.events.write().unwrap();
        events.clear();
    }

    /// Get total event count.
    #[must_use]
    pub fn event_count(&self) -> usize {
        let events = self.events.read().unwrap();
        events.len()
    }
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_event() {
        let logger = AuditLogger::new();
        logger.log_registration(
            "user123".to_string(),
            "testuser".to_string(),
            "127.0.0.1".to_string(),
            "TestAgent".to_string(),
        );

        assert_eq!(logger.event_count(), 1);
        let events = logger.get_events();
        assert_eq!(events[0].event_type, AuthEventType::UserRegistered);
    }

    #[test]
    fn test_get_user_events() {
        let logger = AuditLogger::new();
        logger.log_registration(
            "user123".to_string(),
            "testuser".to_string(),
            "127.0.0.1".to_string(),
            "TestAgent".to_string(),
        );
        logger.log_login_success(
            "user123".to_string(),
            "testuser".to_string(),
            "127.0.0.1".to_string(),
            "TestAgent".to_string(),
        );
        logger.log_registration(
            "user456".to_string(),
            "otheruser".to_string(),
            "127.0.0.2".to_string(),
            "TestAgent".to_string(),
        );

        let user_events = logger.get_user_events("user123");
        assert_eq!(user_events.len(), 2);
    }

    #[test]
    fn test_get_events_by_type() {
        let logger = AuditLogger::new();
        logger.log_registration(
            "user1".to_string(),
            "user1".to_string(),
            "127.0.0.1".to_string(),
            "TestAgent".to_string(),
        );
        logger.log_registration(
            "user2".to_string(),
            "user2".to_string(),
            "127.0.0.1".to_string(),
            "TestAgent".to_string(),
        );
        logger.log_login_success(
            "user1".to_string(),
            "user1".to_string(),
            "127.0.0.1".to_string(),
            "TestAgent".to_string(),
        );

        let registrations = logger.get_events_by_type(AuthEventType::UserRegistered);
        assert_eq!(registrations.len(), 2);
    }

    #[test]
    fn test_get_threat_events() {
        let logger = AuditLogger::new();
        logger.log_login_success(
            "user1".to_string(),
            "user1".to_string(),
            "127.0.0.1".to_string(),
            "TestAgent".to_string(),
        );
        logger.log_login_failed(
            "user1".to_string(),
            "127.0.0.1".to_string(),
            "TestAgent".to_string(),
            "Invalid password".to_string(),
        );
        logger.log_account_locked(
            "user1".to_string(),
            "user1".to_string(),
            "127.0.0.1".to_string(),
            "TestAgent".to_string(),
            "Too many failed attempts".to_string(),
        );

        let threats = logger.get_threat_events();
        assert_eq!(threats.len(), 2);
    }

    #[test]
    fn test_get_events_since() {
        let logger = AuditLogger::new();
        let past = Utc::now() - chrono::Duration::hours(1);

        logger.log_registration(
            "user1".to_string(),
            "user1".to_string(),
            "127.0.0.1".to_string(),
            "TestAgent".to_string(),
        );

        std::thread::sleep(std::time::Duration::from_millis(10));

        let now = Utc::now();
        logger.log_login_success(
            "user1".to_string(),
            "user1".to_string(),
            "127.0.0.1".to_string(),
            "TestAgent".to_string(),
        );

        let recent_events = logger.get_events_since(now);
        assert_eq!(recent_events.len(), 1);

        let all_events = logger.get_events_since(past);
        assert_eq!(all_events.len(), 2);
    }

    #[test]
    fn test_event_type_is_threat() {
        assert!(AuthEventType::LoginFailed.is_threat());
        assert!(AuthEventType::AccountLocked.is_threat());
        assert!(AuthEventType::RateLimitExceeded.is_threat());
        assert!(!AuthEventType::LoginSuccess.is_threat());
        assert!(!AuthEventType::UserRegistered.is_threat());
    }

    #[test]
    fn test_clear_events() {
        let logger = AuditLogger::new();
        logger.log_registration(
            "user1".to_string(),
            "user1".to_string(),
            "127.0.0.1".to_string(),
            "TestAgent".to_string(),
        );
        assert_eq!(logger.event_count(), 1);

        logger.clear();
        assert_eq!(logger.event_count(), 0);
    }
}
