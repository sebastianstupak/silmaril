//! Platform-specific error types.
//!
//! This module defines all errors that can occur in the platform abstraction layer,
//! including window management, input, time, filesystem, and threading operations.

use crate::{EngineError, ErrorCode, ErrorSeverity};
use engine_macros::define_error;

define_error! {
    pub enum PlatformError {
        WindowCreationFailed { details: String } = ErrorCode::WindowCreationFailed, ErrorSeverity::Critical,
        SurfaceCreationFailed { details: String } = ErrorCode::SurfaceCreationFailed, ErrorSeverity::Critical,
        InputInitFailed { details: String } = ErrorCode::InputInitFailed, ErrorSeverity::Critical,
        TimeInitFailed { details: String } = ErrorCode::TimeInitFailed, ErrorSeverity::Critical,
        FileSystemError { operation: String, path: String, details: String } = ErrorCode::FileSystemError, ErrorSeverity::Error,
        ThreadingError { operation: String, details: String } = ErrorCode::ThreadingError, ErrorSeverity::Error,
        PlatformNotSupported { platform: String, feature: String } = ErrorCode::PlatformNotSupported, ErrorSeverity::Critical,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EngineError;

    #[test]
    fn test_window_creation_error() {
        let error = PlatformError::windowcreationfailed("failed to initialize winit".to_string());
        assert_eq!(error.code(), ErrorCode::WindowCreationFailed);
        assert_eq!(error.severity(), ErrorSeverity::Critical);
    }

    #[test]
    fn test_surface_creation_error() {
        let error =
            PlatformError::surfacecreationfailed("Vulkan surface creation failed".to_string());
        assert_eq!(error.code(), ErrorCode::SurfaceCreationFailed);
        assert_eq!(error.severity(), ErrorSeverity::Critical);
    }

    #[test]
    fn test_filesystem_error() {
        let error = PlatformError::filesystemerror(
            "read".to_string(),
            "/tmp/test.txt".to_string(),
            "permission denied".to_string(),
        );
        assert_eq!(error.code(), ErrorCode::FileSystemError);
        assert_eq!(error.severity(), ErrorSeverity::Error);

        let display = format!("{}", error);
        assert!(display.contains("FileSystemError"));
        assert!(display.contains("read"));
        assert!(display.contains("/tmp/test.txt"));
    }

    #[test]
    fn test_platform_not_supported() {
        let error =
            PlatformError::platformnotsupported("wasm32".to_string(), "threading".to_string());
        assert_eq!(error.code(), ErrorCode::PlatformNotSupported);
        assert_eq!(error.severity(), ErrorSeverity::Critical);
    }

    #[test]
    fn test_threading_error() {
        let error = PlatformError::threadingerror(
            "set_priority".to_string(),
            "insufficient permissions".to_string(),
        );
        assert_eq!(error.code(), ErrorCode::ThreadingError);
        assert_eq!(error.severity(), ErrorSeverity::Error);
    }

    #[test]
    fn test_error_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<PlatformError>();
    }

    #[test]
    fn test_result_usage() {
        fn returns_error() -> Result<(), PlatformError> {
            Err(PlatformError::inputinitfailed("test".to_string()))
        }

        let result = returns_error();
        assert!(result.is_err());
    }
}
