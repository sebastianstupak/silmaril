//! Authentication error types.
//!
//! All errors follow the engine's custom error handling pattern with error codes
//! and severity levels.

use engine_core::error::{ErrorCode, ErrorSeverity};
use std::backtrace::Backtrace;
use std::fmt;

/// Authentication-specific error codes (2000-2099 range).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum AuthErrorCode {
    // User/Password errors (2000-2019)
    /// Invalid username format
    InvalidUsername = 2000,
    /// Invalid email format
    InvalidEmail = 2001,
    /// Password too weak
    WeakPassword = 2002,
    /// Password hashing failed
    PasswordHashFailed = 2003,
    /// Password verification failed
    PasswordVerifyFailed = 2004,
    /// User not found
    UserNotFound = 2005,
    /// User already exists
    UserAlreadyExists = 2006,
    /// Invalid credentials
    InvalidCredentials = 2007,

    // Token errors (2020-2039)
    /// JWT generation failed
    TokenGenerationFailed = 2020,
    /// JWT validation failed
    TokenValidationFailed = 2021,
    /// Token expired
    TokenExpired = 2022,
    /// Token revoked
    TokenRevoked = 2023,
    /// Invalid token format
    InvalidTokenFormat = 2024,
    /// Token signature verification failed
    TokenSignatureFailed = 2025,

    // Session errors (2040-2059)
    /// Session not found
    SessionNotFound = 2040,
    /// Session expired
    SessionExpired = 2041,
    /// Maximum concurrent sessions reached
    MaxSessionsReached = 2042,
    /// Session creation failed
    SessionCreationFailed = 2043,

    // OAuth errors (2060-2079)
    /// OAuth provider error
    OAuthProviderError = 2060,
    /// OAuth state mismatch
    OAuthStateMismatch = 2061,
    /// OAuth token exchange failed
    OAuthTokenExchangeFailed = 2062,
    /// OAuth account linking failed
    OAuthLinkingFailed = 2063,
    /// OAuth provider not supported
    OAuthProviderNotSupported = 2064,

    // MFA errors (2080-2094)
    /// MFA setup failed
    MfaSetupFailed = 2080,
    /// TOTP verification failed
    TotpVerificationFailed = 2081,
    /// Backup code verification failed
    BackupCodeVerificationFailed = 2082,
    /// MFA required but not provided
    MfaRequired = 2083,
    /// Invalid MFA code
    InvalidMfaCode = 2084,
    /// QR code generation failed
    QrCodeGenerationFailed = 2085,

    // Rate limiting errors (2095-2099)
    /// Rate limit exceeded
    RateLimitExceeded = 2095,
    /// Account locked due to failed attempts
    AccountLocked = 2096,
}

impl AuthErrorCode {
    /// Convert to engine ErrorCode
    pub fn to_engine_code(self) -> ErrorCode {
        // Auth errors are in the networking range (1400-1499)
        // We map our auth codes to a subset
        match self {
            Self::InvalidUsername => ErrorCode::ProtocolError,
            Self::InvalidEmail => ErrorCode::ProtocolError,
            Self::WeakPassword => ErrorCode::ProtocolError,
            Self::PasswordHashFailed => ErrorCode::ProtocolError,
            Self::PasswordVerifyFailed => ErrorCode::ProtocolError,
            Self::UserNotFound => ErrorCode::ProtocolError,
            Self::UserAlreadyExists => ErrorCode::ProtocolError,
            Self::InvalidCredentials => ErrorCode::ProtocolError,
            Self::TokenGenerationFailed => ErrorCode::ProtocolError,
            Self::TokenValidationFailed => ErrorCode::ProtocolError,
            Self::TokenExpired => ErrorCode::ProtocolError,
            Self::TokenRevoked => ErrorCode::ProtocolError,
            Self::InvalidTokenFormat => ErrorCode::ProtocolError,
            Self::TokenSignatureFailed => ErrorCode::ProtocolError,
            Self::SessionNotFound => ErrorCode::ProtocolError,
            Self::SessionExpired => ErrorCode::ProtocolError,
            Self::MaxSessionsReached => ErrorCode::ProtocolError,
            Self::SessionCreationFailed => ErrorCode::ProtocolError,
            Self::OAuthProviderError => ErrorCode::ProtocolError,
            Self::OAuthStateMismatch => ErrorCode::ProtocolError,
            Self::OAuthTokenExchangeFailed => ErrorCode::ProtocolError,
            Self::OAuthLinkingFailed => ErrorCode::ProtocolError,
            Self::OAuthProviderNotSupported => ErrorCode::ProtocolError,
            Self::MfaSetupFailed => ErrorCode::ProtocolError,
            Self::TotpVerificationFailed => ErrorCode::ProtocolError,
            Self::BackupCodeVerificationFailed => ErrorCode::ProtocolError,
            Self::MfaRequired => ErrorCode::ProtocolError,
            Self::InvalidMfaCode => ErrorCode::ProtocolError,
            Self::QrCodeGenerationFailed => ErrorCode::ProtocolError,
            Self::RateLimitExceeded => ErrorCode::ProtocolError,
            Self::AccountLocked => ErrorCode::ProtocolError,
        }
    }
}

/// Authentication error type.
#[derive(Debug)]
pub enum AuthError {
    // User/Password errors
    InvalidUsername {
        username: String,
        reason: String,
        #[cfg(feature = "backtrace")]
        backtrace: Backtrace,
    },
    InvalidEmail {
        email: String,
        reason: String,
        #[cfg(feature = "backtrace")]
        backtrace: Backtrace,
    },
    WeakPassword {
        reason: String,
        #[cfg(feature = "backtrace")]
        backtrace: Backtrace,
    },
    PasswordHashFailed {
        details: String,
        #[cfg(feature = "backtrace")]
        backtrace: Backtrace,
    },
    PasswordVerifyFailed {
        details: String,
        #[cfg(feature = "backtrace")]
        backtrace: Backtrace,
    },
    UserNotFound {
        identifier: String,
        #[cfg(feature = "backtrace")]
        backtrace: Backtrace,
    },
    UserAlreadyExists {
        identifier: String,
        #[cfg(feature = "backtrace")]
        backtrace: Backtrace,
    },
    InvalidCredentials {
        #[cfg(feature = "backtrace")]
        backtrace: Backtrace,
    },

    // Token errors
    TokenGenerationFailed {
        reason: String,
        #[cfg(feature = "backtrace")]
        backtrace: Backtrace,
    },
    TokenValidationFailed {
        reason: String,
        #[cfg(feature = "backtrace")]
        backtrace: Backtrace,
    },
    TokenExpired {
        expired_at: chrono::DateTime<chrono::Utc>,
        #[cfg(feature = "backtrace")]
        backtrace: Backtrace,
    },
    TokenRevoked {
        token_id: String,
        #[cfg(feature = "backtrace")]
        backtrace: Backtrace,
    },

    // Session errors
    SessionNotFound {
        session_id: String,
        #[cfg(feature = "backtrace")]
        backtrace: Backtrace,
    },
    SessionExpired {
        session_id: String,
        expired_at: chrono::DateTime<chrono::Utc>,
        #[cfg(feature = "backtrace")]
        backtrace: Backtrace,
    },
    MaxSessionsReached {
        user_id: String,
        max_sessions: usize,
        #[cfg(feature = "backtrace")]
        backtrace: Backtrace,
    },

    // OAuth errors
    OAuthProviderError {
        provider: String,
        error: String,
        #[cfg(feature = "backtrace")]
        backtrace: Backtrace,
    },
    OAuthStateMismatch {
        #[cfg(feature = "backtrace")]
        backtrace: Backtrace,
    },
    OAuthTokenExchangeFailed {
        provider: String,
        reason: String,
        #[cfg(feature = "backtrace")]
        backtrace: Backtrace,
    },

    // MFA errors
    MfaRequired {
        user_id: String,
        #[cfg(feature = "backtrace")]
        backtrace: Backtrace,
    },
    TotpVerificationFailed {
        #[cfg(feature = "backtrace")]
        backtrace: Backtrace,
    },
    BackupCodeVerificationFailed {
        #[cfg(feature = "backtrace")]
        backtrace: Backtrace,
    },

    // Rate limiting
    RateLimitExceeded {
        retry_after_secs: u64,
        #[cfg(feature = "backtrace")]
        backtrace: Backtrace,
    },
    AccountLocked {
        user_id: String,
        locked_until: chrono::DateTime<chrono::Utc>,
        #[cfg(feature = "backtrace")]
        backtrace: Backtrace,
    },
}

impl AuthError {
    /// Get the error code for this error.
    pub fn code(&self) -> AuthErrorCode {
        match self {
            Self::InvalidUsername { .. } => AuthErrorCode::InvalidUsername,
            Self::InvalidEmail { .. } => AuthErrorCode::InvalidEmail,
            Self::WeakPassword { .. } => AuthErrorCode::WeakPassword,
            Self::PasswordHashFailed { .. } => AuthErrorCode::PasswordHashFailed,
            Self::PasswordVerifyFailed { .. } => AuthErrorCode::PasswordVerifyFailed,
            Self::UserNotFound { .. } => AuthErrorCode::UserNotFound,
            Self::UserAlreadyExists { .. } => AuthErrorCode::UserAlreadyExists,
            Self::InvalidCredentials { .. } => AuthErrorCode::InvalidCredentials,
            Self::TokenGenerationFailed { .. } => AuthErrorCode::TokenGenerationFailed,
            Self::TokenValidationFailed { .. } => AuthErrorCode::TokenValidationFailed,
            Self::TokenExpired { .. } => AuthErrorCode::TokenExpired,
            Self::TokenRevoked { .. } => AuthErrorCode::TokenRevoked,
            Self::SessionNotFound { .. } => AuthErrorCode::SessionNotFound,
            Self::SessionExpired { .. } => AuthErrorCode::SessionExpired,
            Self::MaxSessionsReached { .. } => AuthErrorCode::MaxSessionsReached,
            Self::OAuthProviderError { .. } => AuthErrorCode::OAuthProviderError,
            Self::OAuthStateMismatch { .. } => AuthErrorCode::OAuthStateMismatch,
            Self::OAuthTokenExchangeFailed { .. } => AuthErrorCode::OAuthTokenExchangeFailed,
            Self::MfaRequired { .. } => AuthErrorCode::MfaRequired,
            Self::TotpVerificationFailed { .. } => AuthErrorCode::TotpVerificationFailed,
            Self::BackupCodeVerificationFailed { .. } => {
                AuthErrorCode::BackupCodeVerificationFailed
            }
            Self::RateLimitExceeded { .. } => AuthErrorCode::RateLimitExceeded,
            Self::AccountLocked { .. } => AuthErrorCode::AccountLocked,
        }
    }

    /// Get severity level for this error.
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            Self::WeakPassword { .. }
            | Self::InvalidUsername { .. }
            | Self::InvalidEmail { .. } => ErrorSeverity::Warning,
            Self::RateLimitExceeded { .. } | Self::AccountLocked { .. } => ErrorSeverity::Warning,
            Self::PasswordHashFailed { .. } => ErrorSeverity::Critical,
            _ => ErrorSeverity::Error,
        }
    }

    #[cfg(feature = "backtrace")]
    /// Get the backtrace for this error.
    pub fn backtrace(&self) -> Option<&Backtrace> {
        match self {
            Self::InvalidUsername { backtrace, .. }
            | Self::InvalidEmail { backtrace, .. }
            | Self::WeakPassword { backtrace, .. }
            | Self::PasswordHashFailed { backtrace, .. }
            | Self::PasswordVerifyFailed { backtrace, .. }
            | Self::UserNotFound { backtrace, .. }
            | Self::UserAlreadyExists { backtrace, .. }
            | Self::InvalidCredentials { backtrace, .. }
            | Self::TokenGenerationFailed { backtrace, .. }
            | Self::TokenValidationFailed { backtrace, .. }
            | Self::TokenExpired { backtrace, .. }
            | Self::TokenRevoked { backtrace, .. }
            | Self::SessionNotFound { backtrace, .. }
            | Self::SessionExpired { backtrace, .. }
            | Self::MaxSessionsReached { backtrace, .. }
            | Self::OAuthProviderError { backtrace, .. }
            | Self::OAuthStateMismatch { backtrace, .. }
            | Self::OAuthTokenExchangeFailed { backtrace, .. }
            | Self::MfaRequired { backtrace, .. }
            | Self::TotpVerificationFailed { backtrace, .. }
            | Self::BackupCodeVerificationFailed { backtrace, .. }
            | Self::RateLimitExceeded { backtrace, .. }
            | Self::AccountLocked { backtrace, .. } => Some(backtrace),
        }
    }

    /// Log this error with structured logging.
    pub fn log(&self) {
        use tracing::{error, warn};

        match self.severity() {
            ErrorSeverity::Warning => {
                #[cfg(feature = "backtrace")]
                {
                    if let Some(bt) = self.backtrace() {
                        warn!(
                            error_code = %self.code() as u32,
                            error_message = %self,
                            backtrace = %bt,
                            "Auth warning"
                        );
                        return;
                    }
                }
                warn!(
                    error_code = %self.code() as u32,
                    error_message = %self,
                    "Auth warning"
                );
            }
            ErrorSeverity::Error => {
                #[cfg(feature = "backtrace")]
                {
                    if let Some(bt) = self.backtrace() {
                        error!(
                            error_code = %self.code() as u32,
                            error_message = %self,
                            backtrace = %bt,
                            "Auth error"
                        );
                        return;
                    }
                }
                error!(
                    error_code = %self.code() as u32,
                    error_message = %self,
                    "Auth error"
                );
            }
            ErrorSeverity::Critical => {
                #[cfg(feature = "backtrace")]
                {
                    if let Some(bt) = self.backtrace() {
                        error!(
                            error_code = %self.code() as u32,
                            error_message = %self,
                            backtrace = %bt,
                            "CRITICAL AUTH ERROR"
                        );
                        return;
                    }
                }
                error!(
                    error_code = %self.code() as u32,
                    error_message = %self,
                    "CRITICAL AUTH ERROR"
                );
            }
        }
    }
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUsername { username, reason, .. } => {
                write!(f, "Invalid username '{}': {}", username, reason)
            }
            Self::InvalidEmail { email, reason, .. } => {
                write!(f, "Invalid email '{}': {}", email, reason)
            }
            Self::WeakPassword { reason, .. } => write!(f, "Weak password: {}", reason),
            Self::PasswordHashFailed { details, .. } => {
                write!(f, "Password hashing failed: {}", details)
            }
            Self::PasswordVerifyFailed { details, .. } => {
                write!(f, "Password verification failed: {}", details)
            }
            Self::UserNotFound { identifier, .. } => write!(f, "User not found: {}", identifier),
            Self::UserAlreadyExists { identifier, .. } => {
                write!(f, "User already exists: {}", identifier)
            }
            Self::InvalidCredentials { .. } => write!(f, "Invalid credentials"),
            Self::TokenGenerationFailed { reason, .. } => {
                write!(f, "Token generation failed: {}", reason)
            }
            Self::TokenValidationFailed { reason, .. } => {
                write!(f, "Token validation failed: {}", reason)
            }
            Self::TokenExpired { expired_at, .. } => {
                write!(f, "Token expired at {}", expired_at)
            }
            Self::TokenRevoked { token_id, .. } => write!(f, "Token revoked: {}", token_id),
            Self::SessionNotFound { session_id, .. } => {
                write!(f, "Session not found: {}", session_id)
            }
            Self::SessionExpired { session_id, expired_at, .. } => {
                write!(f, "Session {} expired at {}", session_id, expired_at)
            }
            Self::MaxSessionsReached { user_id, max_sessions, .. } => {
                write!(
                    f,
                    "Maximum concurrent sessions ({}) reached for user {}",
                    max_sessions, user_id
                )
            }
            Self::OAuthProviderError { provider, error, .. } => {
                write!(f, "OAuth provider {} error: {}", provider, error)
            }
            Self::OAuthStateMismatch { .. } => write!(f, "OAuth state mismatch"),
            Self::OAuthTokenExchangeFailed { provider, reason, .. } => {
                write!(f, "OAuth token exchange failed for {}: {}", provider, reason)
            }
            Self::MfaRequired { user_id, .. } => {
                write!(f, "MFA required for user {}", user_id)
            }
            Self::TotpVerificationFailed { .. } => write!(f, "TOTP verification failed"),
            Self::BackupCodeVerificationFailed { .. } => {
                write!(f, "Backup code verification failed")
            }
            Self::RateLimitExceeded { retry_after_secs, .. } => {
                write!(f, "Rate limit exceeded, retry after {} seconds", retry_after_secs)
            }
            Self::AccountLocked { user_id, locked_until, .. } => {
                write!(f, "Account {} locked until {}", user_id, locked_until)
            }
        }
    }
}

impl std::error::Error for AuthError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = AuthError::InvalidUsername {
            username: "test".to_string(),
            reason: "too short".to_string(),
            #[cfg(feature = "backtrace")]
            backtrace: Backtrace::capture(),
        };
        assert!(format!("{}", err).contains("Invalid username"));
        assert!(format!("{}", err).contains("test"));
    }

    #[test]
    fn test_error_codes() {
        let err = AuthError::WeakPassword {
            reason: "too short".to_string(),
            #[cfg(feature = "backtrace")]
            backtrace: Backtrace::capture(),
        };
        assert_eq!(err.code(), AuthErrorCode::WeakPassword);
    }

    #[test]
    fn test_severity_levels() {
        let weak_pw = AuthError::WeakPassword {
            reason: "test".to_string(),
            #[cfg(feature = "backtrace")]
            backtrace: Backtrace::capture(),
        };
        assert_eq!(weak_pw.severity(), ErrorSeverity::Warning);

        let hash_failed = AuthError::PasswordHashFailed {
            details: "test".to_string(),
            #[cfg(feature = "backtrace")]
            backtrace: Backtrace::capture(),
        };
        assert_eq!(hash_failed.severity(), ErrorSeverity::Critical);
    }
}
