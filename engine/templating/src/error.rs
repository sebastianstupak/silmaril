//! Template system error types using structured error infrastructure.
//!
//! All template errors use custom error types with proper error codes and severity levels.
//! Never use `anyhow` or `Box<dyn Error>` per CLAUDE.md coding standards.

#![allow(unexpected_cfgs)]

use engine_core::{EngineError, ErrorCode, ErrorSeverity};
use engine_macros::define_error;

define_error! {
    pub enum TemplateError {
        NotFound { path: String } = ErrorCode::TemplateNotFound, ErrorSeverity::Error,
        AlreadyExists { path: String } = ErrorCode::TemplateAlreadyExists, ErrorSeverity::Error,
        InvalidYaml { reason: String } = ErrorCode::TemplateInvalidYaml, ErrorSeverity::Error,
        UnknownComponent { component: String } = ErrorCode::TemplateUnknownComponent, ErrorSeverity::Error,
        CircularReference { path: String } = ErrorCode::TemplateCircularReference, ErrorSeverity::Error,
        Io { path: String, error: String } = ErrorCode::TemplateIo, ErrorSeverity::Error,
        Serialization { reason: String } = ErrorCode::TemplateSerialization, ErrorSeverity::Error,
    }
}

/// Type alias for template operation results.
pub type TemplateResult<T> = Result<T, TemplateError>;

impl TemplateError {
    /// Create an Io error from a std::io::Error and a path.
    pub fn from_io_error(path: impl Into<String>, error: std::io::Error) -> Self {
        Self::io(path.into(), error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_error_codes() {
        let err = TemplateError::notfound("test.yaml".to_string());
        assert_eq!(err.code(), ErrorCode::TemplateNotFound);
        assert_eq!(err.severity(), ErrorSeverity::Error);
    }

    #[test]
    fn test_template_error_display() {
        let err = TemplateError::invalidyaml("missing closing bracket".to_string());
        let display = format!("{}", err);
        assert!(display.contains("InvalidYaml") || display.contains("missing closing bracket"));
    }

    #[test]
    fn test_already_exists_error() {
        let err = TemplateError::alreadyexists("templates/player.yaml".to_string());
        assert_eq!(err.code(), ErrorCode::TemplateAlreadyExists);
        assert_eq!(err.severity(), ErrorSeverity::Error);
    }

    #[test]
    fn test_circular_reference_error() {
        let err = TemplateError::circularreference(
            "templates/a.yaml -> templates/b.yaml -> templates/a.yaml".to_string(),
        );
        assert_eq!(err.code(), ErrorCode::TemplateCircularReference);
        assert_eq!(err.severity(), ErrorSeverity::Error);
    }

    #[test]
    fn test_unknown_component_error() {
        let err = TemplateError::unknowncomponent("NonExistentComponent".to_string());
        assert_eq!(err.code(), ErrorCode::TemplateUnknownComponent);
        assert_eq!(err.severity(), ErrorSeverity::Error);
    }

    #[test]
    fn test_serialization_error() {
        let err = TemplateError::serialization("Failed to serialize component data".to_string());
        assert_eq!(err.code(), ErrorCode::TemplateSerialization);
        assert_eq!(err.severity(), ErrorSeverity::Error);
    }
}
