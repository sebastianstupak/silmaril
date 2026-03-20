//! Error types for the ops layer — command execution, I/O, and state errors.

#![allow(unexpected_cfgs)]

use engine_core::{EngineError, ErrorCode, ErrorSeverity};
use engine_macros::define_error;

define_error! {
    pub enum OpsError {
        EntityNotFound { id: u64 }
            = ErrorCode::TemplateEntityNotFound, ErrorSeverity::Error,
        ComponentNotFound { entity: u64, type_name: String }
            = ErrorCode::TemplateComponentNotFound, ErrorSeverity::Error,
        ComponentAlreadyExists { entity: u64, type_name: String }
            = ErrorCode::TemplateAlreadyExists, ErrorSeverity::Error,
        IoFailed { path: String, reason: String }
            = ErrorCode::TemplateIo, ErrorSeverity::Error,
        SerializeFailed { reason: String }
            = ErrorCode::TemplateSerialization, ErrorSeverity::Error,
        NoTemplateOpen {}
            = ErrorCode::TemplateNoTemplateOpen, ErrorSeverity::Error,
    }
}
