//! Authentication system for Agent Game Engine.
//!
//! Production-grade authentication with AAA game studio quality standards.
//!
//! # Features
//!
//! - **User Registration/Login**: Username/email + password with Argon2id hashing
//! - **JWT Tokens**: Access tokens (1hr) + refresh tokens (30 days) with RS256 signing
//! - **Session Management**: In-memory storage with idle/absolute timeouts
//! - **Rate Limiting**: IP-based rate limiting (5 attempts/15min)
//! - **MFA**: TOTP (Google Authenticator compatible) + backup codes
//! - **OAuth**: Steam, Discord social login
//! - **Audit Logging**: All authentication events logged
//! - **Security**: Account lockout, password strength validation, CSRF protection
//!
//! # Quick Start
//!
//! ```no_run
//! use engine_auth::*;
//!
//! # async fn example() -> Result<(), AuthError> {
//! // Create user
//! let password = "StrongP@ss123";
//! let password_hash = password::hash_password(password).await?;
//! let user = User::new(
//!     "player1".to_string(),
//!     "player1@example.com".to_string(),
//!     password_hash,
//! )?;
//!
//! // Generate JWT tokens
//! let jwt_manager = create_test_jwt_manager()?;
//! let tokens = jwt_manager.generate_token_pair(
//!     &user.id.to_string(),
//!     &user.username,
//!     &user.email,
//! )?;
//!
//! // Create session
//! let session_store = SessionStore::new();
//! let session = session_store.create_session(
//!     user.id.to_string(),
//!     "127.0.0.1".to_string(),
//!     "GameClient/1.0".to_string(),
//! )?;
//!
//! // Setup MFA
//! let totp = TotpManager::new("My Game".to_string());
//! let totp_setup = totp.generate_secret(&user.email)?;
//! println!("Scan this QR code:\n{}", totp_setup.qr_code_ascii);
//! # Ok(())
//! # }
//! ```
//!
//! # Security Best Practices
//!
//! 1. **Never log passwords** - Use structured logging for audit events
//! 2. **Rate limit all endpoints** - Prevent brute force attacks
//! 3. **Validate all inputs** - Use provided validation functions
//! 4. **Use HTTPS** - Always encrypt network traffic
//! 5. **Rotate JWT keys** - Implement key rotation for production
//! 6. **Monitor audit logs** - Watch for suspicious activity
//! 7. **Enable MFA** - Encourage users to enable two-factor auth
//!
//! # Architecture
//!
//! The auth system is designed for easy integration with:
//! - PostgreSQL (user storage)
//! - Redis (session storage, token revocation)
//! - Logging infrastructure (tracing)
//! - Game server architecture

#![warn(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

pub mod audit;
pub mod error;
pub mod jwt;
pub mod mfa;
pub mod oauth;
pub mod password;
pub mod rate_limit;
pub mod session;
pub mod user;

// Re-export commonly used types
pub use audit::{AuditEvent, AuditEventType, AuditLogger};
pub use error::{AuthError, AuthErrorCode};
pub use jwt::{AccessTokenClaims, JwtManager, RefreshTokenClaims, TokenPair};
pub use mfa::{BackupCode, BackupCodeManager, TotpManager, TotpSetup};
pub use oauth::{OAuthProfile, OAuthProvider, OAuthState};
pub use password::{hash_password, verify_password};
pub use rate_limit::{IpRateLimiter, RateLimiter};
pub use session::{Session, SessionStore};
pub use user::{validate_email, validate_password_strength, validate_username, User};

/// Create a test JWT manager with generated RSA keys.
///
/// **WARNING:** Only use for testing! In production, load proper RSA keys from files.
///
/// # Errors
///
/// Returns [`AuthError::TokenGenerationFailed`] if key generation fails.
#[cfg(any(test, feature = "test-utils"))]
pub fn create_test_jwt_manager() -> Result<JwtManager, AuthError> {
    let (private_pem, public_pem) = jwt::generate_test_rsa_keys();
    JwtManager::new(&private_pem, &public_pem, "test-engine".to_string())
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_full_registration_flow() {
        // Validate and create user
        let username = "testuser";
        let email = "test@example.com";
        let password = "SecureP@ss123";

        assert!(validate_username(username).is_ok());
        assert!(validate_email(email).is_ok());
        assert!(validate_password_strength(password).is_ok());

        // Hash password
        let password_hash = hash_password(password).await.unwrap();

        // Create user
        let user = User::new(username.to_string(), email.to_string(), password_hash).unwrap();

        assert_eq!(user.username, username);
        assert_eq!(user.email, email);
        assert!(user.enabled);
    }

    #[tokio::test]
    async fn test_full_login_flow() {
        // Setup
        let password = "SecureP@ss123";
        let password_hash = hash_password(password).await.unwrap();
        let user = User::new(
            "testuser".to_string(),
            "test@example.com".to_string(),
            password_hash.clone(),
        )
        .unwrap();

        // Verify password
        let is_valid = verify_password(password, &password_hash).await.unwrap();
        assert!(is_valid);

        // Generate tokens
        let jwt_manager = create_test_jwt_manager().unwrap();
        let tokens = jwt_manager
            .generate_token_pair(&user.id.to_string(), &user.username, &user.email)
            .unwrap();

        // Validate access token
        let claims = jwt_manager.validate_access_token(&tokens.access_token).unwrap();
        assert_eq!(claims.sub, user.id.to_string());

        // Create session
        let session_store = SessionStore::new();
        let session = session_store
            .create_session(
                user.id.to_string(),
                "127.0.0.1".to_string(),
                "TestClient/1.0".to_string(),
            )
            .unwrap();

        assert_eq!(session.user_id, user.id.to_string());
    }

    #[tokio::test]
    async fn test_mfa_flow() {
        // Setup TOTP
        let totp = TotpManager::new("TestApp".to_string());
        let setup = totp.generate_secret("testuser@example.com").unwrap();

        // Generate and verify code
        let code = totp.generate_current_code(&setup.secret, "testuser@example.com").unwrap();
        let is_valid = totp.verify_code(&setup.secret, &code, "testuser@example.com").unwrap();
        assert!(is_valid);

        // Generate backup codes
        let (plaintext, mut hashed) = BackupCodeManager::generate_codes();
        assert_eq!(plaintext.len(), 10);

        // Verify backup code
        BackupCodeManager::verify_and_use(&mut hashed, &plaintext[0]).unwrap();

        // Try to reuse - should fail
        assert!(BackupCodeManager::verify_and_use(&mut hashed, &plaintext[0]).is_err());
    }

    #[tokio::test]
    async fn test_rate_limiting() {
        let limiter = RateLimiter::with_config(3, 1); // 3 attempts per minute

        // First 3 should succeed
        assert!(limiter.check("test").is_ok());
        assert!(limiter.check("test").is_ok());
        assert!(limiter.check("test").is_ok());

        // 4th should fail
        assert!(limiter.check("test").is_err());
    }

    #[tokio::test]
    async fn test_audit_logging() {
        let logger = AuditLogger::new();

        logger.log_registration(
            "user123".to_string(),
            "testuser".to_string(),
            "127.0.0.1".to_string(),
            "TestClient".to_string(),
        );

        logger.log_login_success(
            "user123".to_string(),
            "testuser".to_string(),
            "127.0.0.1".to_string(),
            "TestClient".to_string(),
        );

        let events = logger.get_user_events("user123");
        assert_eq!(events.len(), 2);
    }

    #[tokio::test]
    async fn test_account_lockout() {
        let mut user =
            User::new("testuser".to_string(), "test@example.com".to_string(), "hash".to_string())
                .unwrap();

        assert!(!user.is_locked());

        // Trigger lockout
        for _ in 0..5 {
            user.increment_failed_attempts(5, 30);
        }

        assert!(user.is_locked());
    }

    #[tokio::test]
    async fn test_session_expiration() {
        let store = SessionStore::with_config(0, 24, 5); // 0 minute idle timeout

        let session = store
            .create_session(
                "user123".to_string(),
                "127.0.0.1".to_string(),
                "TestClient".to_string(),
            )
            .unwrap();

        // Wait for expiration
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Should be expired
        let result = store.get_session(&session.id);
        assert!(result.is_err());
    }
}
