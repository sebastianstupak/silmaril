//! Error types for the auto-update system.

use engine_core::{EngineError, ErrorCode, ErrorSeverity};
use engine_macros::define_error;

define_error! {
    /// Errors that can occur during auto-update operations.
    pub enum UpdateError {
        /// Failed to check for updates from the server
        CheckFailed { reason: String } = ErrorCode::UpdateCheckFailed, ErrorSeverity::Error,

        /// Failed to download update files
        DownloadFailed { url: String, reason: String } = ErrorCode::UpdateDownloadFailed, ErrorSeverity::Error,

        /// Failed to verify downloaded file integrity
        VerificationFailed { file: String, reason: String } = ErrorCode::UpdateDownloadFailed, ErrorSeverity::Critical,

        /// Failed to apply patch to a file
        PatchFailed { file: String, reason: String } = ErrorCode::UpdateInstallFailed, ErrorSeverity::Critical,

        /// Failed to install update
        InstallFailed { reason: String } = ErrorCode::UpdateInstallFailed, ErrorSeverity::Critical,

        /// Failed to rollback after failed update
        RollbackFailed { reason: String } = ErrorCode::UpdateInstallFailed, ErrorSeverity::Critical,

        /// Invalid manifest format or content
        InvalidManifest { reason: String } = ErrorCode::UpdateCheckFailed, ErrorSeverity::Error,

        /// Version parsing failed
        InvalidVersion { version: String } = ErrorCode::UpdateCheckFailed, ErrorSeverity::Error,

        /// IO operation failed
        IoError { path: String, reason: String } = ErrorCode::UpdateInstallFailed, ErrorSeverity::Error,

        /// Network request failed
        NetworkError { reason: String } = ErrorCode::UpdateDownloadFailed, ErrorSeverity::Error,

        /// Signature verification failed
        SignatureVerificationFailed { file: String } = ErrorCode::UpdateDownloadFailed, ErrorSeverity::Critical,

        /// Insufficient disk space for update
        InsufficientSpace { required: u64, available: u64 } = ErrorCode::UpdateInstallFailed, ErrorSeverity::Error,

        /// Update already in progress
        UpdateInProgress = ErrorCode::UpdateCheckFailed, ErrorSeverity::Warning,

        /// No update available
        NoUpdateAvailable = ErrorCode::UpdateCheckFailed, ErrorSeverity::Warning,

        /// Channel not found
        ChannelNotFound { channel: String } = ErrorCode::UpdateCheckFailed, ErrorSeverity::Error,
    }
}
