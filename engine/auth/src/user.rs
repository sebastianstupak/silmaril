//! User model and validation.
//!
//! Defines the core User struct and validation logic for usernames, emails, and passwords.

use crate::error::{AuthError, AuthErrorCode};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::backtrace::Backtrace;
use uuid::Uuid;

/// User account model.
///
/// This represents a user account in the authentication system. User data is designed
/// to be serializable for storage in PostgreSQL using JSON columns or separate tables.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// Unique user ID
    pub id: Uuid,
    /// Username (3-32 alphanumeric + underscore/dash)
    pub username: String,
    /// Email address
    pub email: String,
    /// Argon2id password hash
    pub password_hash: String,
    /// Account creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last login timestamp
    pub last_login: Option<DateTime<Utc>>,
    /// Account enabled/disabled
    pub enabled: bool,
    /// Email verified
    pub email_verified: bool,
    /// MFA enabled
    pub mfa_enabled: bool,
    /// TOTP secret (Base32-encoded, only if MFA enabled)
    pub totp_secret: Option<String>,
    /// Backup codes (hashed with SHA-256)
    pub backup_codes: Vec<String>,
    /// Failed login attempts counter
    pub failed_login_attempts: u32,
    /// Account locked until (if locked)
    pub locked_until: Option<DateTime<Utc>>,
    /// OAuth provider IDs (provider_name -> provider_user_id)
    pub oauth_providers: std::collections::HashMap<String, String>,
}

impl User {
    /// Create a new user with validated username, email, and password hash.
    ///
    /// # Errors
    ///
    /// Returns [`AuthError::InvalidUsername`] if username is invalid.
    /// Returns [`AuthError::InvalidEmail`] if email is invalid.
    pub fn new(username: String, email: String, password_hash: String) -> Result<Self, AuthError> {
        validate_username(&username)?;
        validate_email(&email)?;

        Ok(Self {
            id: Uuid::new_v4(),
            username,
            email,
            password_hash,
            created_at: Utc::now(),
            last_login: None,
            enabled: true,
            email_verified: false,
            mfa_enabled: false,
            totp_secret: None,
            backup_codes: Vec::new(),
            failed_login_attempts: 0,
            locked_until: None,
            oauth_providers: std::collections::HashMap::new(),
        })
    }

    /// Check if account is locked.
    pub fn is_locked(&self) -> bool {
        if let Some(locked_until) = self.locked_until {
            Utc::now() < locked_until
        } else {
            false
        }
    }

    /// Increment failed login attempts and lock account if threshold exceeded.
    pub fn increment_failed_attempts(&mut self, max_attempts: u32, lockout_duration_minutes: i64) {
        self.failed_login_attempts += 1;

        if self.failed_login_attempts >= max_attempts {
            self.locked_until =
                Some(Utc::now() + chrono::Duration::minutes(lockout_duration_minutes));
            tracing::warn!(
                user_id = %self.id,
                failed_attempts = self.failed_login_attempts,
                locked_until = %self.locked_until.unwrap(),
                "Account locked due to failed login attempts"
            );
        }
    }

    /// Reset failed login attempts (called on successful login).
    pub fn reset_failed_attempts(&mut self) {
        self.failed_login_attempts = 0;
        self.locked_until = None;
    }

    /// Update last login timestamp.
    pub fn update_last_login(&mut self) {
        self.last_login = Some(Utc::now());
    }

    /// Link an OAuth provider account.
    pub fn link_oauth_provider(&mut self, provider: String, provider_user_id: String) {
        self.oauth_providers.insert(provider, provider_user_id);
    }

    /// Check if OAuth provider is linked.
    pub fn has_oauth_provider(&self, provider: &str) -> bool {
        self.oauth_providers.contains_key(provider)
    }
}

/// Validate username format.
///
/// Rules:
/// - 3-32 characters
/// - Alphanumeric, underscore, dash only
/// - Must start with alphanumeric
///
/// # Errors
///
/// Returns [`AuthError::InvalidUsername`] if validation fails.
pub fn validate_username(username: &str) -> Result<(), AuthError> {
    if username.len() < 3 {
        return Err(AuthError::InvalidUsername {
            username: username.to_string(),
            reason: "Username must be at least 3 characters".to_string(),
            #[cfg(feature = "backtrace")]
            backtrace: Backtrace::capture(),
        });
    }

    if username.len() > 32 {
        return Err(AuthError::InvalidUsername {
            username: username.to_string(),
            reason: "Username must be at most 32 characters".to_string(),
            #[cfg(feature = "backtrace")]
            backtrace: Backtrace::capture(),
        });
    }

    // Must start with alphanumeric
    if !username.chars().next().map_or(false, |c| c.is_alphanumeric()) {
        return Err(AuthError::InvalidUsername {
            username: username.to_string(),
            reason: "Username must start with an alphanumeric character".to_string(),
            #[cfg(feature = "backtrace")]
            backtrace: Backtrace::capture(),
        });
    }

    // Only alphanumeric, underscore, dash
    if !username.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
        return Err(AuthError::InvalidUsername {
            username: username.to_string(),
            reason: "Username can only contain alphanumeric characters, underscores, and dashes"
                .to_string(),
            #[cfg(feature = "backtrace")]
            backtrace: Backtrace::capture(),
        });
    }

    Ok(())
}

/// Validate email format.
///
/// Basic email validation (more thorough validation should be done via email verification).
///
/// # Errors
///
/// Returns [`AuthError::InvalidEmail`] if validation fails.
pub fn validate_email(email: &str) -> Result<(), AuthError> {
    // Basic email validation
    if !email.contains('@') {
        return Err(AuthError::InvalidEmail {
            email: email.to_string(),
            reason: "Email must contain @".to_string(),
            #[cfg(feature = "backtrace")]
            backtrace: Backtrace::capture(),
        });
    }

    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 {
        return Err(AuthError::InvalidEmail {
            email: email.to_string(),
            reason: "Email must have exactly one @".to_string(),
            #[cfg(feature = "backtrace")]
            backtrace: Backtrace::capture(),
        });
    }

    if parts[0].is_empty() {
        return Err(AuthError::InvalidEmail {
            email: email.to_string(),
            reason: "Email local part cannot be empty".to_string(),
            #[cfg(feature = "backtrace")]
            backtrace: Backtrace::capture(),
        });
    }

    if !parts[1].contains('.') {
        return Err(AuthError::InvalidEmail {
            email: email.to_string(),
            reason: "Email domain must contain a dot".to_string(),
            #[cfg(feature = "backtrace")]
            backtrace: Backtrace::capture(),
        });
    }

    if email.len() > 254 {
        return Err(AuthError::InvalidEmail {
            email: email.to_string(),
            reason: "Email must be at most 254 characters".to_string(),
            #[cfg(feature = "backtrace")]
            backtrace: Backtrace::capture(),
        });
    }

    Ok(())
}

/// Validate password strength.
///
/// Rules (OWASP 2025 recommendations):
/// - Minimum 8 characters
/// - At least one uppercase letter
/// - At least one lowercase letter
/// - At least one digit
/// - At least one special character
///
/// # Errors
///
/// Returns [`AuthError::WeakPassword`] if validation fails.
pub fn validate_password_strength(password: &str) -> Result<(), AuthError> {
    if password.len() < 8 {
        return Err(AuthError::WeakPassword {
            reason: "Password must be at least 8 characters".to_string(),
            #[cfg(feature = "backtrace")]
            backtrace: Backtrace::capture(),
        });
    }

    if password.len() > 128 {
        return Err(AuthError::WeakPassword {
            reason: "Password must be at most 128 characters".to_string(),
            #[cfg(feature = "backtrace")]
            backtrace: Backtrace::capture(),
        });
    }

    let has_uppercase = password.chars().any(|c| c.is_uppercase());
    let has_lowercase = password.chars().any(|c| c.is_lowercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    let has_special = password.chars().any(|c| !c.is_alphanumeric() && !c.is_whitespace());

    if !has_uppercase {
        return Err(AuthError::WeakPassword {
            reason: "Password must contain at least one uppercase letter".to_string(),
            #[cfg(feature = "backtrace")]
            backtrace: Backtrace::capture(),
        });
    }

    if !has_lowercase {
        return Err(AuthError::WeakPassword {
            reason: "Password must contain at least one lowercase letter".to_string(),
            #[cfg(feature = "backtrace")]
            backtrace: Backtrace::capture(),
        });
    }

    if !has_digit {
        return Err(AuthError::WeakPassword {
            reason: "Password must contain at least one digit".to_string(),
            #[cfg(feature = "backtrace")]
            backtrace: Backtrace::capture(),
        });
    }

    if !has_special {
        return Err(AuthError::WeakPassword {
            reason: "Password must contain at least one special character".to_string(),
            #[cfg(feature = "backtrace")]
            backtrace: Backtrace::capture(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_username() {
        assert!(validate_username("john_doe").is_ok());
        assert!(validate_username("player123").is_ok());
        assert!(validate_username("test-user").is_ok());
    }

    #[test]
    fn test_invalid_username_too_short() {
        assert!(validate_username("ab").is_err());
    }

    #[test]
    fn test_invalid_username_too_long() {
        let long_username = "a".repeat(33);
        assert!(validate_username(&long_username).is_err());
    }

    #[test]
    fn test_invalid_username_special_chars() {
        assert!(validate_username("user@name").is_err());
        assert!(validate_username("user name").is_err());
    }

    #[test]
    fn test_invalid_username_start() {
        assert!(validate_username("_username").is_err());
        assert!(validate_username("-username").is_err());
    }

    #[test]
    fn test_valid_email() {
        assert!(validate_email("user@example.com").is_ok());
        assert!(validate_email("test.user@domain.co.uk").is_ok());
    }

    #[test]
    fn test_invalid_email_no_at() {
        assert!(validate_email("userexample.com").is_err());
    }

    #[test]
    fn test_invalid_email_multiple_at() {
        assert!(validate_email("user@@example.com").is_err());
    }

    #[test]
    fn test_invalid_email_no_domain() {
        assert!(validate_email("user@").is_err());
    }

    #[test]
    fn test_valid_password() {
        assert!(validate_password_strength("StrongP@ss1").is_ok());
        assert!(validate_password_strength("Complex!Pass123").is_ok());
    }

    #[test]
    fn test_invalid_password_too_short() {
        assert!(validate_password_strength("Short1!").is_err());
    }

    #[test]
    fn test_invalid_password_no_uppercase() {
        assert!(validate_password_strength("weakpass123!").is_err());
    }

    #[test]
    fn test_invalid_password_no_lowercase() {
        assert!(validate_password_strength("WEAKPASS123!").is_err());
    }

    #[test]
    fn test_invalid_password_no_digit() {
        assert!(validate_password_strength("WeakPass!").is_err());
    }

    #[test]
    fn test_invalid_password_no_special() {
        assert!(validate_password_strength("WeakPass123").is_err());
    }

    #[test]
    fn test_user_creation() {
        let user = User::new(
            "testuser".to_string(),
            "test@example.com".to_string(),
            "hashed_password".to_string(),
        );
        assert!(user.is_ok());
        let user = user.unwrap();
        assert_eq!(user.username, "testuser");
        assert_eq!(user.email, "test@example.com");
        assert_eq!(user.enabled, true);
        assert_eq!(user.mfa_enabled, false);
    }

    #[test]
    fn test_account_locking() {
        let mut user =
            User::new("testuser".to_string(), "test@example.com".to_string(), "hash".to_string())
                .unwrap();

        assert!(!user.is_locked());

        // Increment attempts
        for _ in 0..4 {
            user.increment_failed_attempts(5, 30);
        }
        assert!(!user.is_locked());

        // 5th attempt should lock
        user.increment_failed_attempts(5, 30);
        assert!(user.is_locked());
    }

    #[test]
    fn test_reset_failed_attempts() {
        let mut user =
            User::new("testuser".to_string(), "test@example.com".to_string(), "hash".to_string())
                .unwrap();

        user.failed_login_attempts = 3;
        user.reset_failed_attempts();
        assert_eq!(user.failed_login_attempts, 0);
    }

    #[test]
    fn test_oauth_provider_linking() {
        let mut user =
            User::new("testuser".to_string(), "test@example.com".to_string(), "hash".to_string())
                .unwrap();

        assert!(!user.has_oauth_provider("steam"));
        user.link_oauth_provider("steam".to_string(), "steam123".to_string());
        assert!(user.has_oauth_provider("steam"));
    }
}
