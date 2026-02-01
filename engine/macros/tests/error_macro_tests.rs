//! Integration tests for the define_error! macro.
//!
//! These tests verify that the macro generates correct code and that
//! the generated errors work as expected.

use engine_core::{EngineError, ErrorCode, ErrorSeverity};
use engine_macros::define_error;

// Test basic error definition
define_error! {
    pub enum BasicError {
        Simple { message: String } = ErrorCode::EntityNotFound, ErrorSeverity::Error,
    }
}

// Test error with multiple fields
define_error! {
    pub enum MultiFieldError {
        Complex { id: u32, reason: String, details: String } = ErrorCode::ComponentNotFound, ErrorSeverity::Error,
    }
}

// Test error with multiple variants
define_error! {
    pub enum MultiVariantError {
        NotFound { id: u32 } = ErrorCode::EntityNotFound, ErrorSeverity::Error,
        InvalidData { reason: String } = ErrorCode::InvalidFormat, ErrorSeverity::Error,
        SystemFailure { details: String } = ErrorCode::VulkanInitFailed, ErrorSeverity::Critical,
    }
}

// Test error with different severity levels
define_error! {
    pub enum SeverityError {
        Warning { msg: String } = ErrorCode::EntityNotFound, ErrorSeverity::Warning,
        Error { msg: String } = ErrorCode::ComponentNotFound, ErrorSeverity::Error,
        Critical { msg: String } = ErrorCode::VulkanInitFailed, ErrorSeverity::Critical,
    }
}

#[test]
fn test_basic_error_display() {
    let error = BasicError::Simple { message: "test message".to_string() };
    let display = format!("{}", error);
    assert!(display.contains("Simple"));
    assert!(display.contains("test message"));
}

#[test]
fn test_basic_error_code() {
    let error = BasicError::Simple { message: "test".to_string() };
    assert_eq!(error.code(), ErrorCode::EntityNotFound);
}

#[test]
fn test_basic_error_severity() {
    let error = BasicError::Simple { message: "test".to_string() };
    assert_eq!(error.severity(), ErrorSeverity::Error);
}

#[test]
fn test_multi_field_display() {
    let error = MultiFieldError::Complex {
        id: 42,
        reason: "invalid state".to_string(),
        details: "component missing".to_string(),
    };
    let display = format!("{}", error);
    assert!(display.contains("Complex"));
    assert!(display.contains("42"));
    assert!(display.contains("invalid state"));
    assert!(display.contains("component missing"));
}

#[test]
fn test_multi_variant_codes() {
    let err1 = MultiVariantError::NotFound { id: 1 };
    let err2 = MultiVariantError::InvalidData { reason: "test".to_string() };
    let err3 = MultiVariantError::SystemFailure { details: "test".to_string() };

    assert_eq!(err1.code(), ErrorCode::EntityNotFound);
    assert_eq!(err2.code(), ErrorCode::InvalidFormat);
    assert_eq!(err3.code(), ErrorCode::VulkanInitFailed);
}

#[test]
fn test_multi_variant_severities() {
    let err1 = MultiVariantError::NotFound { id: 1 };
    let err2 = MultiVariantError::InvalidData { reason: "test".to_string() };
    let err3 = MultiVariantError::SystemFailure { details: "test".to_string() };

    assert_eq!(err1.severity(), ErrorSeverity::Error);
    assert_eq!(err2.severity(), ErrorSeverity::Error);
    assert_eq!(err3.severity(), ErrorSeverity::Critical);
}

#[test]
fn test_severity_levels() {
    let warn = SeverityError::Warning { msg: "test".to_string() };
    let err = SeverityError::Error { msg: "test".to_string() };
    let crit = SeverityError::Critical { msg: "test".to_string() };

    assert_eq!(warn.severity(), ErrorSeverity::Warning);
    assert_eq!(err.severity(), ErrorSeverity::Error);
    assert_eq!(crit.severity(), ErrorSeverity::Critical);

    // Test ordering
    assert!(warn.severity() < err.severity());
    assert!(err.severity() < crit.severity());
}

#[test]
fn test_error_trait_implementation() {
    let error = BasicError::Simple { message: "test".to_string() };

    // Should implement std::error::Error
    let _err: &dyn std::error::Error = &error;
}

#[test]
fn test_engine_error_trait() {
    let error = MultiVariantError::NotFound { id: 42 };

    // Should implement EngineError
    let engine_err: &dyn EngineError = &error;
    assert_eq!(engine_err.code(), ErrorCode::EntityNotFound);
    assert_eq!(engine_err.severity(), ErrorSeverity::Error);
}

#[test]
fn test_error_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<BasicError>();
    assert_send_sync::<MultiFieldError>();
    assert_send_sync::<MultiVariantError>();
    assert_send_sync::<SeverityError>();
}

#[test]
fn test_error_debug() {
    let error = BasicError::Simple { message: "test".to_string() };
    let debug = format!("{:?}", error);
    assert!(debug.contains("Simple"));
}

#[test]
fn test_result_usage() {
    fn returns_error() -> Result<(), BasicError> {
        Err(BasicError::Simple { message: "failed".to_string() })
    }

    let result = returns_error();
    assert!(result.is_err());
    if let Err(e) = result {
        assert_eq!(e.code(), ErrorCode::EntityNotFound);
    }
}

#[test]
fn test_error_propagation() {
    fn inner() -> Result<(), MultiVariantError> {
        Err(MultiVariantError::InvalidData { reason: "bad data".to_string() })
    }

    fn outer() -> Result<(), MultiVariantError> {
        inner()?;
        Ok(())
    }

    let result = outer();
    assert!(result.is_err());
}
