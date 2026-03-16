//! Error types for engine-dev-tools-hot-reload.

use engine_core::{EngineError, ErrorCode, ErrorSeverity};
use engine_macros::define_error;

define_error! {
    pub enum DevError {
        PortBindFailed { port: u16 } = ErrorCode::DevPortBindFailed, ErrorSeverity::Warning,
        SerializeFailed { reason: String } = ErrorCode::DevSerializeFailed, ErrorSeverity::Error,
        RestoreFailed { reason: String } = ErrorCode::DevRestoreFailed, ErrorSeverity::Warning,
        ReloadFailed { path: String, reason: String } = ErrorCode::DevReloadFailed, ErrorSeverity::Warning,
        TcpSendFailed { reason: String } = ErrorCode::DevTcpSendFailed, ErrorSeverity::Warning,
    }
}
