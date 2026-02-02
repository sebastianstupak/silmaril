//! TLS/DTLS error types
//!
//! Provides structured error types for TLS operations following engine error handling standards.

use engine_core::{EngineError, ErrorCode, ErrorSeverity};
use std::fmt;

/// Result type for TLS operations
pub type TlsResult<T> = Result<T, TlsError>;

/// TLS/DTLS errors
#[derive(Debug)]
pub enum TlsError {
    /// Handshake failed
    HandshakeFailed {
        /// Reason for failure
        reason: String,
        /// Backtrace if enabled
        #[cfg(feature = "backtrace")]
        backtrace: std::backtrace::Backtrace,
    },

    /// Certificate validation error
    CertificateValidation {
        /// Reason for validation failure
        reason: String,
        /// Backtrace if enabled
        #[cfg(feature = "backtrace")]
        backtrace: std::backtrace::Backtrace,
    },

    /// Certificate expired
    CertificateExpired {
        /// Certificate subject
        subject: String,
        /// Expiration date
        expired_at: String,
        /// Backtrace if enabled
        #[cfg(feature = "backtrace")]
        backtrace: std::backtrace::Backtrace,
    },

    /// Certificate not yet valid
    CertificateNotYetValid {
        /// Certificate subject
        subject: String,
        /// Valid from date
        valid_from: String,
        /// Backtrace if enabled
        #[cfg(feature = "backtrace")]
        backtrace: std::backtrace::Backtrace,
    },

    /// Invalid certificate
    InvalidCertificate {
        /// Reason
        reason: String,
        /// Backtrace if enabled
        #[cfg(feature = "backtrace")]
        backtrace: std::backtrace::Backtrace,
    },

    /// Certificate chain error
    CertificateChain {
        /// Error description
        reason: String,
        /// Backtrace if enabled
        #[cfg(feature = "backtrace")]
        backtrace: std::backtrace::Backtrace,
    },

    /// Configuration error
    ConfigError {
        /// Error description
        reason: String,
        /// Backtrace if enabled
        #[cfg(feature = "backtrace")]
        backtrace: std::backtrace::Backtrace,
    },

    /// Connection error
    ConnectionError {
        /// Error description
        reason: String,
        /// Backtrace if enabled
        #[cfg(feature = "backtrace")]
        backtrace: std::backtrace::Backtrace,
    },

    /// Encryption error
    EncryptionError {
        /// Error description
        reason: String,
        /// Backtrace if enabled
        #[cfg(feature = "backtrace")]
        backtrace: std::backtrace::Backtrace,
    },

    /// Decryption error
    DecryptionError {
        /// Error description
        reason: String,
        /// Backtrace if enabled
        #[cfg(feature = "backtrace")]
        backtrace: std::backtrace::Backtrace,
    },

    /// ACME protocol error
    Acme {
        /// Error description
        reason: String,
        /// Backtrace if enabled
        #[cfg(feature = "backtrace")]
        backtrace: std::backtrace::Backtrace,
    },

    /// Certificate renewal failed
    RenewalFailed {
        /// Error description
        reason: String,
        /// Backtrace if enabled
        #[cfg(feature = "backtrace")]
        backtrace: std::backtrace::Backtrace,
    },

    /// DTLS handshake failed
    DtlsHandshakeFailed {
        /// Reason for failure
        reason: String,
        /// Backtrace if enabled
        #[cfg(feature = "backtrace")]
        backtrace: std::backtrace::Backtrace,
    },

    /// DTLS packet error
    DtlsPacketError {
        /// Error description
        reason: String,
        /// Backtrace if enabled
        #[cfg(feature = "backtrace")]
        backtrace: std::backtrace::Backtrace,
    },

    /// IO error
    Io {
        /// IO error
        source: std::io::Error,
        /// Backtrace if enabled
        #[cfg(feature = "backtrace")]
        backtrace: std::backtrace::Backtrace,
    },
}

impl fmt::Display for TlsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HandshakeFailed { reason, .. } => write!(f, "TLS handshake failed: {}", reason),
            Self::CertificateValidation { reason, .. } => {
                write!(f, "Certificate validation failed: {}", reason)
            }
            Self::CertificateExpired { subject, expired_at, .. } => {
                write!(f, "Certificate expired for '{}' at {}", subject, expired_at)
            }
            Self::CertificateNotYetValid { subject, valid_from, .. } => {
                write!(f, "Certificate for '{}' not valid until {}", subject, valid_from)
            }
            Self::InvalidCertificate { reason, .. } => write!(f, "Invalid certificate: {}", reason),
            Self::CertificateChain { reason, .. } => {
                write!(f, "Certificate chain error: {}", reason)
            }
            Self::ConfigError { reason, .. } => write!(f, "TLS configuration error: {}", reason),
            Self::ConnectionError { reason, .. } => write!(f, "TLS connection error: {}", reason),
            Self::EncryptionError { reason, .. } => write!(f, "Encryption error: {}", reason),
            Self::DecryptionError { reason, .. } => write!(f, "Decryption error: {}", reason),
            Self::Acme { reason, .. } => write!(f, "ACME protocol error: {}", reason),
            Self::RenewalFailed { reason, .. } => {
                write!(f, "Certificate renewal failed: {}", reason)
            }
            Self::DtlsHandshakeFailed { reason, .. } => {
                write!(f, "DTLS handshake failed: {}", reason)
            }
            Self::DtlsPacketError { reason, .. } => write!(f, "DTLS packet error: {}", reason),
            Self::Io { source, .. } => write!(f, "IO error: {}", source),
        }
    }
}

impl std::error::Error for TlsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl EngineError for TlsError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::HandshakeFailed { .. } => ErrorCode::TlsHandshakeFailed,
            Self::CertificateValidation { .. } => ErrorCode::CertificateValidationFailed,
            Self::CertificateExpired { .. } => ErrorCode::CertificateExpired,
            Self::CertificateNotYetValid { .. } => ErrorCode::CertificateNotYetValid,
            Self::InvalidCertificate { .. } => ErrorCode::InvalidCertificate,
            Self::CertificateChain { .. } => ErrorCode::CertificateChainError,
            Self::ConfigError { .. } => ErrorCode::TlsConfigError,
            Self::ConnectionError { .. } => ErrorCode::TlsConnectionError,
            Self::EncryptionError { .. } => ErrorCode::TlsEncryptionError,
            Self::DecryptionError { .. } => ErrorCode::TlsDecryptionError,
            Self::Acme { .. } => ErrorCode::AcmeError,
            Self::RenewalFailed { .. } => ErrorCode::CertificateRenewalFailed,
            Self::DtlsHandshakeFailed { .. } => ErrorCode::DtlsHandshakeFailed,
            Self::DtlsPacketError { .. } => ErrorCode::DtlsPacketError,
            Self::Io { .. } => ErrorCode::ConnectionFailed,
        }
    }

    fn severity(&self) -> ErrorSeverity {
        match self {
            Self::CertificateExpired { .. } | Self::CertificateNotYetValid { .. } => {
                ErrorSeverity::Critical
            }
            Self::HandshakeFailed { .. }
            | Self::CertificateValidation { .. }
            | Self::InvalidCertificate { .. }
            | Self::CertificateChain { .. } => ErrorSeverity::Error,
            Self::ConfigError { .. } => ErrorSeverity::Critical,
            Self::ConnectionError { .. } | Self::Io { .. } => ErrorSeverity::Error,
            Self::EncryptionError { .. } | Self::DecryptionError { .. } => ErrorSeverity::Critical,
            Self::Acme { .. } | Self::RenewalFailed { .. } => ErrorSeverity::Warning,
            Self::DtlsHandshakeFailed { .. } | Self::DtlsPacketError { .. } => ErrorSeverity::Error,
        }
    }

    #[cfg(feature = "backtrace")]
    fn backtrace(&self) -> Option<&std::backtrace::Backtrace> {
        match self {
            Self::HandshakeFailed { backtrace, .. }
            | Self::CertificateValidation { backtrace, .. }
            | Self::CertificateExpired { backtrace, .. }
            | Self::CertificateNotYetValid { backtrace, .. }
            | Self::InvalidCertificate { backtrace, .. }
            | Self::CertificateChain { backtrace, .. }
            | Self::ConfigError { backtrace, .. }
            | Self::ConnectionError { backtrace, .. }
            | Self::EncryptionError { backtrace, .. }
            | Self::DecryptionError { backtrace, .. }
            | Self::Acme { backtrace, .. }
            | Self::RenewalFailed { backtrace, .. }
            | Self::DtlsHandshakeFailed { backtrace, .. }
            | Self::DtlsPacketError { backtrace, .. }
            | Self::Io { backtrace, .. } => Some(backtrace),
        }
    }
}

impl From<std::io::Error> for TlsError {
    fn from(source: std::io::Error) -> Self {
        Self::Io {
            source,
            #[cfg(feature = "backtrace")]
            backtrace: std::backtrace::Backtrace::capture(),
        }
    }
}

impl From<rustls::Error> for TlsError {
    fn from(err: rustls::Error) -> Self {
        Self::ConnectionError {
            reason: format!("Rustls error: {}", err),
            #[cfg(feature = "backtrace")]
            backtrace: std::backtrace::Backtrace::capture(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = TlsError::HandshakeFailed {
            reason: "Connection refused".to_string(),
            #[cfg(feature = "backtrace")]
            backtrace: std::backtrace::Backtrace::capture(),
        };
        assert!(format!("{}", err).contains("TLS handshake failed"));
    }

    #[test]
    fn test_error_code_mapping() {
        let err = TlsError::CertificateExpired {
            subject: "test.com".to_string(),
            expired_at: "2024-01-01".to_string(),
            #[cfg(feature = "backtrace")]
            backtrace: std::backtrace::Backtrace::capture(),
        };
        assert_eq!(err.code(), ErrorCode::CertificateExpired);
    }

    #[test]
    fn test_error_severity() {
        let err = TlsError::ConfigError {
            reason: "Invalid config".to_string(),
            #[cfg(feature = "backtrace")]
            backtrace: std::backtrace::Backtrace::capture(),
        };
        assert_eq!(err.severity(), ErrorSeverity::Critical);
    }
}
