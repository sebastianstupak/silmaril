//! Error infrastructure for the agent game engine.
//!
//! This module provides the foundation for structured error handling across the entire engine.
//! All engine errors implement the `EngineError` trait, which provides:
//! - Error codes for programmatic handling
//! - Severity levels for filtering and alerting
//! - Automatic structured logging via tracing
//!
//! # Error Code Ranges
//!
//! Error codes are organized by subsystem:
//! - 1000-1099: Core ECS
//! - 1100-1199: Serialization
//! - 1200-1299: Platform
//! - 1300-1399: Rendering
//! - 1400-1499: Networking
//! - 1500-1599: Physics
//! - 1600-1699: Audio
//! - 1700-1799: LOD
//! - 1800-1899: Interest Management
//! - 1900-1999: Auto-update

use std::fmt;

/// Error codes organized by subsystem.
///
/// Each subsystem has a range of 100 codes. This allows for programmatic
/// error handling and monitoring/alerting based on error types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum ErrorCode {
    // Core ECS (1000-1099)
    EntityNotFound = 1000,
    ComponentNotFound = 1001,
    ArchetypeNotFound = 1002,
    InvalidEntityId = 1003,
    ComponentAlreadyExists = 1004,

    // Serialization (1100-1199)
    SerializationFailed = 1100,
    DeserializationFailed = 1101,
    YamlSerializeFailed = 1102,
    YamlDeserializeFailed = 1103,
    BincodeSerializeFailed = 1104,
    BincodeDeserializeFailed = 1105,
    FlatbuffersSerializeFailed = 1106,
    FlatbuffersDeserializeFailed = 1107,
    InvalidFormat = 1108,
    VersionMismatch = 1109,

    // Platform (1200-1299)
    WindowCreationFailed = 1200,
    SurfaceCreationFailed = 1201,
    InputInitFailed = 1202,
    TimeInitFailed = 1203,
    FileSystemError = 1204,
    ThreadingError = 1205,
    PlatformNotSupported = 1206,

    // Rendering (1300-1399)
    VulkanInitFailed = 1300,
    ShaderCompileFailed = 1301,
    TextureLoadFailed = 1302,
    MeshLoadFailed = 1303,
    SwapchainCreationFailed = 1304,

    // Networking (1400-1499)
    ConnectionFailed = 1400,
    BindFailed = 1401,
    SendFailed = 1402,
    ReceiveFailed = 1403,
    ProtocolError = 1404,

    // Physics (1500-1599)
    PhysicsInitFailed = 1500,
    CollisionDetectionFailed = 1501,

    // Audio (1600-1699)
    AudioInitFailed = 1600,
    SoundLoadFailed = 1601,

    // LOD (1700-1799)
    LodInitFailed = 1700,

    // Interest Management (1800-1899)
    InterestInitFailed = 1800,

    // Auto-update (1900-1999)
    UpdateCheckFailed = 1900,
    UpdateDownloadFailed = 1901,
    UpdateInstallFailed = 1902,
}

impl ErrorCode {
    /// Get the subsystem this error code belongs to.
    pub fn subsystem(&self) -> &'static str {
        let code = *self as u32;
        match code {
            1000..=1099 => "Core ECS",
            1100..=1199 => "Serialization",
            1200..=1299 => "Platform",
            1300..=1399 => "Rendering",
            1400..=1499 => "Networking",
            1500..=1599 => "Physics",
            1600..=1699 => "Audio",
            1700..=1799 => "LOD",
            1800..=1899 => "Interest Management",
            1900..=1999 => "Auto-update",
            _ => "Unknown",
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.subsystem(), *self as u32)
    }
}

/// Error severity levels for filtering and alerting.
///
/// - Warning: Non-fatal issues that don't prevent operation
/// - Error: Failures that prevent a specific operation but don't crash the engine
/// - Critical: Failures that require engine shutdown or restart
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ErrorSeverity {
    Warning,
    Error,
    Critical,
}

impl fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorSeverity::Warning => write!(f, "WARNING"),
            ErrorSeverity::Error => write!(f, "ERROR"),
            ErrorSeverity::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Core trait for all engine errors.
///
/// This trait extends `std::error::Error` with structured error codes,
/// severity levels, and automatic logging via `tracing`.
///
/// # Implementation
///
/// Use the `define_error!` macro from `engine-macros` to automatically
/// implement this trait:
///
/// ```ignore
/// use engine_macros::define_error;
///
/// define_error! {
///     pub enum MyError {
///         NotFound { id: u32 } = ErrorCode::EntityNotFound, ErrorSeverity::Error,
///         InvalidData { reason: String } = ErrorCode::InvalidFormat, ErrorSeverity::Error,
///     }
/// }
/// ```
pub trait EngineError: std::error::Error + Send + Sync {
    /// Get the error code for programmatic handling.
    fn code(&self) -> ErrorCode;

    /// Get the severity level.
    fn severity(&self) -> ErrorSeverity;

    /// Log this error using structured logging.
    ///
    /// This method is automatically called by the error handling infrastructure.
    /// The default implementation uses `tracing` to emit structured log events.
    fn log(&self) {
        use tracing::{error, warn};

        match self.severity() {
            ErrorSeverity::Warning => {
                warn!(
                    error_code = %self.code(),
                    error_message = %self,
                    subsystem = self.code().subsystem(),
                    "Engine warning"
                );
            }
            ErrorSeverity::Error => {
                error!(
                    error_code = %self.code(),
                    error_message = %self,
                    subsystem = self.code().subsystem(),
                    "Engine error"
                );
            }
            ErrorSeverity::Critical => {
                error!(
                    error_code = %self.code(),
                    error_message = %self,
                    subsystem = self.code().subsystem(),
                    "CRITICAL ENGINE ERROR"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_subsystem() {
        assert_eq!(ErrorCode::EntityNotFound.subsystem(), "Core ECS");
        assert_eq!(ErrorCode::SerializationFailed.subsystem(), "Serialization");
        assert_eq!(ErrorCode::WindowCreationFailed.subsystem(), "Platform");
        assert_eq!(ErrorCode::VulkanInitFailed.subsystem(), "Rendering");
        assert_eq!(ErrorCode::ConnectionFailed.subsystem(), "Networking");
    }

    #[test]
    fn test_error_code_display() {
        let code = ErrorCode::EntityNotFound;
        let display = format!("{}", code);
        assert!(display.contains("Core ECS"));
        assert!(display.contains("1000"));
    }

    #[test]
    fn test_severity_ordering() {
        assert!(ErrorSeverity::Warning < ErrorSeverity::Error);
        assert!(ErrorSeverity::Error < ErrorSeverity::Critical);
    }

    #[test]
    fn test_severity_display() {
        assert_eq!(format!("{}", ErrorSeverity::Warning), "WARNING");
        assert_eq!(format!("{}", ErrorSeverity::Error), "ERROR");
        assert_eq!(format!("{}", ErrorSeverity::Critical), "CRITICAL");
    }

    #[test]
    fn test_error_code_ranges() {
        // Verify each subsystem is in correct range
        assert!((ErrorCode::EntityNotFound as u32) >= 1000);
        assert!((ErrorCode::EntityNotFound as u32) < 1100);

        assert!((ErrorCode::SerializationFailed as u32) >= 1100);
        assert!((ErrorCode::SerializationFailed as u32) < 1200);

        assert!((ErrorCode::WindowCreationFailed as u32) >= 1200);
        assert!((ErrorCode::WindowCreationFailed as u32) < 1300);
    }
}
