//! Error types for the auto-update system.

use engine_core::{EngineError, ErrorCode, ErrorSeverity};
use engine_macros::define_error;

define_error! {
    pub enum UpdateError {
        CheckFailed { reason: String } = ErrorCode::UpdateCheckFailed, ErrorSeverity::Error,
        DownloadFailed { url: String, reason: String } = ErrorCode::UpdateDownloadFailed, ErrorSeverity::Error,
        VerificationFailed { file: String, reason: String } = ErrorCode::UpdateDownloadFailed, ErrorSeverity::Critical,
        PatchFailed { file: String, reason: String } = ErrorCode::UpdateInstallFailed, ErrorSeverity::Critical,
        InstallFailed { reason: String } = ErrorCode::UpdateInstallFailed, ErrorSeverity::Critical,
        RollbackFailed { reason: String } = ErrorCode::UpdateInstallFailed, ErrorSeverity::Critical,
        InvalidManifest { reason: String } = ErrorCode::UpdateCheckFailed, ErrorSeverity::Error,
        InvalidVersion { version: String } = ErrorCode::UpdateCheckFailed, ErrorSeverity::Error,
        IoError { path: String, reason: String } = ErrorCode::UpdateInstallFailed, ErrorSeverity::Error,
        NetworkError { reason: String } = ErrorCode::UpdateDownloadFailed, ErrorSeverity::Error,
        SignatureVerificationFailed { file: String } = ErrorCode::UpdateDownloadFailed, ErrorSeverity::Critical,
        InsufficientSpace { required: u64, available: u64 } = ErrorCode::UpdateInstallFailed, ErrorSeverity::Error,
        UpdateInProgress {} = ErrorCode::UpdateCheckFailed, ErrorSeverity::Warning,
        NoUpdateAvailable {} = ErrorCode::UpdateCheckFailed, ErrorSeverity::Warning,
        ChannelNotFound { channel: String } = ErrorCode::UpdateCheckFailed, ErrorSeverity::Error,
    }
}
