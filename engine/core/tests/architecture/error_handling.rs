//! Runtime tests for error handling infrastructure.
//!
//! These tests verify:
//! - All errors implement EngineError trait
//! - Error codes are unique and in correct ranges
//! - Severity levels work correctly
//! - Structured logging integration works

use engine_core::{
    serialization::SerializationError, EngineError, ErrorCode, ErrorSeverity, PlatformError,
};
use std::collections::HashSet;

#[test]
fn test_platform_error_implements_engine_error() {
    let error = PlatformError::WindowCreationFailed { details: "test".to_string() };

    // Verify it implements EngineError trait
    let engine_error: &dyn EngineError = &error;

    assert_eq!(engine_error.code(), ErrorCode::WindowCreationFailed);
    assert_eq!(engine_error.severity(), ErrorSeverity::Critical);
}

#[test]
fn test_serialization_error_implements_engine_error() {
    let error = SerializationError::YamlSerialize { details: "test".to_string() };

    // Verify it implements EngineError trait
    let engine_error: &dyn EngineError = &error;

    assert_eq!(engine_error.code(), ErrorCode::YamlSerializeFailed);
    assert_eq!(engine_error.severity(), ErrorSeverity::Error);
}

#[test]
fn test_all_platform_errors_have_correct_codes() {
    let errors = vec![
        PlatformError::WindowCreationFailed { details: "test".to_string() },
        PlatformError::SurfaceCreationFailed { details: "test".to_string() },
        PlatformError::InputInitFailed { details: "test".to_string() },
        PlatformError::TimeInitFailed { details: "test".to_string() },
        PlatformError::FileSystemError {
            operation: "test".to_string(),
            path: "test".to_string(),
            details: "test".to_string(),
        },
        PlatformError::ThreadingError {
            operation: "test".to_string(),
            details: "test".to_string(),
        },
        PlatformError::PlatformNotSupported {
            platform: "test".to_string(),
            feature: "test".to_string(),
        },
    ];

    // All platform errors should have codes in the 1200-1299 range
    for error in errors {
        let code = error.code() as u32;
        assert!(code >= 1200 && code < 1300, "Platform error code {} out of range", code);
    }
}

#[test]
fn test_all_serialization_errors_have_correct_codes() {
    let errors = vec![
        SerializationError::YamlSerialize { details: "test".to_string() },
        SerializationError::YamlDeserialize { details: "test".to_string() },
        SerializationError::BincodeSerialize { details: "test".to_string() },
        SerializationError::BincodeDeserialize { details: "test".to_string() },
        SerializationError::FlatBuffersSerialize { details: "test".to_string() },
        SerializationError::FlatBuffersDeserialize { details: "test".to_string() },
        SerializationError::IoError { details: "test".to_string() },
        SerializationError::Utf8Error { details: "test".to_string() },
    ];

    // All serialization errors should have codes in the 1100-1199 range
    // (except IoError and Utf8Error which map to other codes)
    for error in &errors {
        let code = error.code() as u32;
        // IoError maps to FileSystemError (1204), Utf8Error to InvalidFormat (1108)
        assert!((code >= 1100 && code < 1300), "Serialization error code {} out of range", code);
    }
}

#[test]
fn test_error_codes_are_unique() {
    // Collect all error codes
    let all_codes = vec![
        ErrorCode::EntityNotFound,
        ErrorCode::ComponentNotFound,
        ErrorCode::ArchetypeNotFound,
        ErrorCode::InvalidEntityId,
        ErrorCode::ComponentAlreadyExists,
        ErrorCode::SerializationFailed,
        ErrorCode::DeserializationFailed,
        ErrorCode::YamlSerializeFailed,
        ErrorCode::YamlDeserializeFailed,
        ErrorCode::BincodeSerializeFailed,
        ErrorCode::BincodeDeserializeFailed,
        ErrorCode::FlatbuffersSerializeFailed,
        ErrorCode::FlatbuffersDeserializeFailed,
        ErrorCode::InvalidFormat,
        ErrorCode::VersionMismatch,
        ErrorCode::WindowCreationFailed,
        ErrorCode::SurfaceCreationFailed,
        ErrorCode::InputInitFailed,
        ErrorCode::TimeInitFailed,
        ErrorCode::FileSystemError,
        ErrorCode::ThreadingError,
        ErrorCode::PlatformNotSupported,
        ErrorCode::VulkanInitFailed,
        ErrorCode::ShaderCompileFailed,
        ErrorCode::TextureLoadFailed,
        ErrorCode::MeshLoadFailed,
        ErrorCode::SwapchainCreationFailed,
        ErrorCode::ConnectionFailed,
        ErrorCode::BindFailed,
        ErrorCode::SendFailed,
        ErrorCode::ReceiveFailed,
        ErrorCode::ProtocolError,
        ErrorCode::PhysicsInitFailed,
        ErrorCode::CollisionDetectionFailed,
        ErrorCode::AudioInitFailed,
        ErrorCode::SoundLoadFailed,
        ErrorCode::LodInitFailed,
        ErrorCode::InterestInitFailed,
        ErrorCode::UpdateCheckFailed,
        ErrorCode::UpdateDownloadFailed,
        ErrorCode::UpdateInstallFailed,
    ];

    // Convert to u32 and check for uniqueness
    let mut code_set = HashSet::new();

    for code in all_codes {
        let code_num = code as u32;
        assert!(code_set.insert(code_num), "Duplicate error code: {}", code_num);
    }
}

#[test]
fn test_error_codes_in_correct_ranges() {
    // Verify each error code is in its designated range
    let test_cases = vec![
        (ErrorCode::EntityNotFound, 1000, 1099, "Core ECS"),
        (ErrorCode::SerializationFailed, 1100, 1199, "Serialization"),
        (ErrorCode::WindowCreationFailed, 1200, 1299, "Platform"),
        (ErrorCode::VulkanInitFailed, 1300, 1399, "Rendering"),
        (ErrorCode::ConnectionFailed, 1400, 1499, "Networking"),
        (ErrorCode::PhysicsInitFailed, 1500, 1599, "Physics"),
        (ErrorCode::AudioInitFailed, 1600, 1699, "Audio"),
        (ErrorCode::LodInitFailed, 1700, 1799, "LOD"),
        (ErrorCode::InterestInitFailed, 1800, 1899, "Interest Management"),
        (ErrorCode::UpdateCheckFailed, 1900, 1999, "Auto-update"),
    ];

    for (code, min, max, subsystem) in test_cases {
        let code_num = code as u32;
        assert!(
            code_num >= min && code_num < max,
            "{} error code {} not in range {}-{}",
            subsystem,
            code_num,
            min,
            max
        );
        assert_eq!(code.subsystem(), subsystem);
    }
}

#[test]
fn test_error_severity_ordering() {
    // Verify severity levels are ordered correctly
    assert!(ErrorSeverity::Warning < ErrorSeverity::Error);
    assert!(ErrorSeverity::Error < ErrorSeverity::Critical);

    // Verify ordering is transitive
    assert!(ErrorSeverity::Warning < ErrorSeverity::Critical);
}

#[test]
fn test_error_severity_display() {
    assert_eq!(format!("{}", ErrorSeverity::Warning), "WARNING");
    assert_eq!(format!("{}", ErrorSeverity::Error), "ERROR");
    assert_eq!(format!("{}", ErrorSeverity::Critical), "CRITICAL");
}

#[test]
fn test_error_code_display() {
    let code = ErrorCode::EntityNotFound;
    let display = format!("{}", code);

    // Display should include both subsystem name and numeric code
    assert!(display.contains("Core ECS"));
    assert!(display.contains("1000"));
}

#[test]
fn test_error_code_subsystem_mapping() {
    // Verify subsystem() method returns correct strings
    assert_eq!(ErrorCode::EntityNotFound.subsystem(), "Core ECS");
    assert_eq!(ErrorCode::SerializationFailed.subsystem(), "Serialization");
    assert_eq!(ErrorCode::WindowCreationFailed.subsystem(), "Platform");
    assert_eq!(ErrorCode::VulkanInitFailed.subsystem(), "Rendering");
    assert_eq!(ErrorCode::ConnectionFailed.subsystem(), "Networking");
    assert_eq!(ErrorCode::PhysicsInitFailed.subsystem(), "Physics");
    assert_eq!(ErrorCode::AudioInitFailed.subsystem(), "Audio");
    assert_eq!(ErrorCode::LodInitFailed.subsystem(), "LOD");
    assert_eq!(ErrorCode::InterestInitFailed.subsystem(), "Interest Management");
    assert_eq!(ErrorCode::UpdateCheckFailed.subsystem(), "Auto-update");
}

#[test]
fn test_platform_error_severities() {
    // Critical errors - system cannot continue
    let critical_errors = vec![
        PlatformError::WindowCreationFailed { details: "test".to_string() },
        PlatformError::SurfaceCreationFailed { details: "test".to_string() },
        PlatformError::InputInitFailed { details: "test".to_string() },
        PlatformError::TimeInitFailed { details: "test".to_string() },
        PlatformError::PlatformNotSupported {
            platform: "test".to_string(),
            feature: "test".to_string(),
        },
    ];

    for error in critical_errors {
        assert_eq!(error.severity(), ErrorSeverity::Critical);
    }

    // Error severity - operation failed but system can continue
    let error_errors = vec![
        PlatformError::FileSystemError {
            operation: "test".to_string(),
            path: "test".to_string(),
            details: "test".to_string(),
        },
        PlatformError::ThreadingError {
            operation: "test".to_string(),
            details: "test".to_string(),
        },
    ];

    for error in error_errors {
        assert_eq!(error.severity(), ErrorSeverity::Error);
    }
}

#[test]
fn test_serialization_error_severities() {
    let errors = vec![
        SerializationError::YamlSerialize { details: "test".to_string() },
        SerializationError::BincodeDeserialize { details: "test".to_string() },
        SerializationError::FlatBuffersSerialize { details: "test".to_string() },
        SerializationError::IoError { details: "test".to_string() },
    ];

    // All serialization errors should be Error severity
    for error in &errors {
        assert_eq!(error.severity(), ErrorSeverity::Error);
    }
}

#[test]
fn test_error_display_formatting() {
    let error = PlatformError::FileSystemError {
        operation: "read".to_string(),
        path: "/tmp/test.txt".to_string(),
        details: "permission denied".to_string(),
    };

    let display = format!("{}", error);

    // Display should contain all error information
    assert!(display.contains("FileSystemError"));
    assert!(display.contains("read"));
    assert!(display.contains("/tmp/test.txt"));
    assert!(display.contains("permission denied"));
}

#[test]
fn test_error_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}

    assert_send_sync::<PlatformError>();
    assert_send_sync::<SerializationError>();
    assert_send_sync::<Box<dyn EngineError>>();
}

#[test]
fn test_error_can_be_used_in_result() {
    fn returns_platform_error() -> Result<(), PlatformError> {
        Err(PlatformError::FileSystemError {
            operation: "test".to_string(),
            path: "test".to_string(),
            details: "test".to_string(),
        })
    }

    fn returns_serialization_error() -> Result<(), SerializationError> {
        Err(SerializationError::YamlSerialize { details: "test".to_string() })
    }

    assert!(returns_platform_error().is_err());
    assert!(returns_serialization_error().is_err());
}

#[test]
fn test_error_conversions() {
    // Test From<std::io::Error> for SerializationError
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let ser_err: SerializationError = io_err.into();

    assert_eq!(ser_err.code(), ErrorCode::FileSystemError);

    // Test From<FromUtf8Error> for SerializationError
    let bytes = vec![0, 159, 146, 150]; // Invalid UTF-8
    let utf8_err = String::from_utf8(bytes).unwrap_err();
    let ser_err: SerializationError = utf8_err.into();

    assert_eq!(ser_err.code(), ErrorCode::InvalidFormat);
}

#[test]
fn test_error_downcast() {
    let error: Box<dyn EngineError> =
        Box::new(PlatformError::WindowCreationFailed { details: "test".to_string() });

    // Verify we can check error properties through trait
    assert_eq!(error.code(), ErrorCode::WindowCreationFailed);
    assert_eq!(error.severity(), ErrorSeverity::Critical);
}

#[test]
fn test_error_logging_doesnt_panic() {
    // Test that calling log() doesn't panic
    let errors: Vec<Box<dyn EngineError>> = vec![
        Box::new(PlatformError::WindowCreationFailed { details: "test".to_string() }),
        Box::new(SerializationError::YamlSerialize { details: "test".to_string() }),
    ];

    for error in errors {
        // This should not panic
        error.log();
    }
}

#[test]
fn test_all_error_codes_have_valid_subsystems() {
    let all_codes = vec![
        ErrorCode::EntityNotFound,
        ErrorCode::ComponentNotFound,
        ErrorCode::SerializationFailed,
        ErrorCode::WindowCreationFailed,
        ErrorCode::VulkanInitFailed,
        ErrorCode::ConnectionFailed,
        ErrorCode::PhysicsInitFailed,
        ErrorCode::AudioInitFailed,
        ErrorCode::LodInitFailed,
        ErrorCode::InterestInitFailed,
        ErrorCode::UpdateCheckFailed,
    ];

    for code in all_codes {
        let subsystem = code.subsystem();
        assert!(!subsystem.is_empty());
        assert_ne!(subsystem, "Unknown");
    }
}

#[test]
fn test_error_source_trait() {
    // Verify errors implement std::error::Error
    use std::error::Error;

    let platform_error = PlatformError::FileSystemError {
        operation: "test".to_string(),
        path: "test".to_string(),
        details: "test".to_string(),
    };

    let _error: &dyn Error = &platform_error;

    let serialization_error = SerializationError::YamlSerialize { details: "test".to_string() };

    let _error: &dyn Error = &serialization_error;
}

#[test]
fn test_error_debug_format() {
    let error = PlatformError::ThreadingError {
        operation: "set_priority".to_string(),
        details: "insufficient permissions".to_string(),
    };

    let debug = format!("{:?}", error);

    // Debug format should contain variant name and fields
    assert!(debug.contains("ThreadingError"));
}

#[test]
fn test_multiple_errors_with_same_code() {
    // Different error variants can map to the same error code
    // Verify both SerializationError and PlatformError can use FileSystemError code

    let ser_error = SerializationError::IoError { details: "test".to_string() };
    let platform_error = PlatformError::FileSystemError {
        operation: "test".to_string(),
        path: "test".to_string(),
        details: "test".to_string(),
    };

    // Both map to FileSystemError
    assert_eq!(ser_error.code(), ErrorCode::FileSystemError);
    assert_eq!(platform_error.code(), ErrorCode::FileSystemError);
}

#[test]
fn test_error_can_be_boxed() {
    let errors: Vec<Box<dyn std::error::Error>> = vec![
        Box::new(PlatformError::TimeInitFailed { details: "test".to_string() }),
        Box::new(SerializationError::BincodeSerialize { details: "test".to_string() }),
    ];

    // Verify all errors can be used as trait objects
    assert_eq!(errors.len(), 2);
}
