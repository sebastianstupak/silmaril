//! Integration tests for template error handling.
//!
//! Tests verify that all error variants are created correctly with appropriate
//! error codes and severity levels.

#![allow(unexpected_cfgs)]

use engine_core::{EngineError, ErrorCode, ErrorSeverity};
use engine_templating::{TemplateError, TemplateResult};

#[test]
fn test_notfound_error_with_correct_path() {
    let path = "templates/missing.yaml".to_string();
    let err = TemplateError::notfound(path.clone());

    assert_eq!(err.code(), ErrorCode::TemplateNotFound);
    assert_eq!(err.severity(), ErrorSeverity::Error);

    let display = format!("{}", err);
    assert!(
        display.contains("NotFound") || display.contains(&path),
        "Error display should contain error name or path"
    );
}

#[test]
fn test_alreadyexists_error_with_path() {
    let path = "templates/player.yaml".to_string();
    let err = TemplateError::alreadyexists(path.clone());

    assert_eq!(err.code(), ErrorCode::TemplateAlreadyExists);
    assert_eq!(err.severity(), ErrorSeverity::Error);

    let display = format!("{}", err);
    assert!(
        display.contains("AlreadyExists") || display.contains(&path),
        "Error display should contain error name or path"
    );
}

#[test]
fn test_invalidyaml_error_with_reason() {
    let reason = "missing closing bracket at line 42".to_string();
    let err = TemplateError::invalidyaml(reason.clone());

    assert_eq!(err.code(), ErrorCode::TemplateInvalidYaml);
    assert_eq!(err.severity(), ErrorSeverity::Error);

    let display = format!("{}", err);
    assert!(
        display.contains("InvalidYaml") || display.contains(&reason),
        "Error display should contain error name or reason"
    );
}

#[test]
fn test_unknowncomponent_error() {
    let component = "NonExistentComponent".to_string();
    let err = TemplateError::unknowncomponent(component.clone());

    assert_eq!(err.code(), ErrorCode::TemplateUnknownComponent);
    assert_eq!(err.severity(), ErrorSeverity::Error);

    let display = format!("{}", err);
    assert!(
        display.contains("UnknownComponent") || display.contains(&component),
        "Error display should contain error name or component name"
    );
}

#[test]
fn test_circularreference_detection() {
    let path = "a.yaml -> b.yaml -> a.yaml".to_string();
    let err = TemplateError::circularreference(path.clone());

    assert_eq!(err.code(), ErrorCode::TemplateCircularReference);
    assert_eq!(err.severity(), ErrorSeverity::Error);

    let display = format!("{}", err);
    assert!(
        display.contains("CircularReference") || display.contains(&path),
        "Error display should contain error name or circular path"
    );
}

#[test]
fn test_io_error_wrapped() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let err = TemplateError::from_io_error("templates/test.yaml", io_err);

    assert_eq!(err.code(), ErrorCode::TemplateIo);
    assert_eq!(err.severity(), ErrorSeverity::Error);

    let display = format!("{}", err);
    assert!(
        display.contains("Io")
            || display.contains("file not found")
            || display.contains("test.yaml"),
        "Error display should contain error name, path, or IO error message"
    );
}

#[test]
fn test_io_error_direct() {
    let err = TemplateError::io("templates/test.yaml".to_string(), "Permission denied".to_string());

    assert_eq!(err.code(), ErrorCode::TemplateIo);
    assert_eq!(err.severity(), ErrorSeverity::Error);
}

#[test]
fn test_serialization_error() {
    let reason = "Failed to deserialize YAML".to_string();
    let err = TemplateError::serialization(reason.clone());

    assert_eq!(err.code(), ErrorCode::TemplateSerialization);
    assert_eq!(err.severity(), ErrorSeverity::Error);

    let display = format!("{}", err);
    assert!(
        display.contains("Serialization") || display.contains(&reason),
        "Error display should contain error name or reason"
    );
}

#[test]
fn test_result_type_alias() {
    // Test that TemplateResult<T> works correctly
    fn returns_ok() -> TemplateResult<String> {
        Ok("success".to_string())
    }

    fn returns_err() -> TemplateResult<String> {
        Err(TemplateError::notfound("test.yaml".to_string()))
    }

    assert!(returns_ok().is_ok());
    assert!(returns_err().is_err());
}

#[test]
fn test_all_errors_have_correct_severity() {
    // All template errors should have Error severity (not Warning or Critical)
    let errors = vec![
        TemplateError::notfound("test".to_string()),
        TemplateError::alreadyexists("test".to_string()),
        TemplateError::invalidyaml("test".to_string()),
        TemplateError::unknowncomponent("test".to_string()),
        TemplateError::circularreference("test".to_string()),
        TemplateError::io("test.yaml".to_string(), "error".to_string()),
        TemplateError::serialization("test".to_string()),
    ];

    for err in errors {
        assert_eq!(
            err.severity(),
            ErrorSeverity::Error,
            "All template errors should have Error severity"
        );
    }
}

#[test]
fn test_error_codes_are_unique() {
    // Verify that each error variant has a unique error code
    let codes = vec![
        ErrorCode::TemplateNotFound,
        ErrorCode::TemplateAlreadyExists,
        ErrorCode::TemplateInvalidYaml,
        ErrorCode::TemplateUnknownComponent,
        ErrorCode::TemplateCircularReference,
        ErrorCode::TemplateIo,
        ErrorCode::TemplateSerialization,
    ];

    // Convert to numeric values
    let mut numeric_codes: Vec<u32> = codes.iter().map(|c| *c as u32).collect();
    numeric_codes.sort_unstable();

    // Check for duplicates
    for i in 0..numeric_codes.len() - 1 {
        assert_ne!(numeric_codes[i], numeric_codes[i + 1], "Error codes should be unique");
    }
}

#[test]
fn test_error_codes_in_correct_range() {
    // Template error codes should be in the 2000-2099 range
    let codes = vec![
        ErrorCode::TemplateNotFound,
        ErrorCode::TemplateAlreadyExists,
        ErrorCode::TemplateInvalidYaml,
        ErrorCode::TemplateUnknownComponent,
        ErrorCode::TemplateCircularReference,
        ErrorCode::TemplateIo,
        ErrorCode::TemplateSerialization,
    ];

    for code in codes {
        let numeric = code as u32;
        assert!(
            (2000..=2099).contains(&numeric),
            "Template error code {} should be in range 2000-2099",
            numeric
        );
    }
}

#[cfg(feature = "backtrace")]
#[test]
fn test_backtrace_captured() {
    let err = TemplateError::notfound("test.yaml".to_string());
    assert!(
        err.backtrace().is_some(),
        "Backtrace should be captured when feature is enabled"
    );
}

#[cfg(not(feature = "backtrace"))]
#[test]
fn test_backtrace_not_captured() {
    let err = TemplateError::notfound("test.yaml".to_string());
    assert!(
        err.backtrace().is_none(),
        "Backtrace should not be captured when feature is disabled"
    );
}
