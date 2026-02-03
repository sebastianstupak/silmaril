//! Audio Error Types

use thiserror::Error;

/// Audio system errors
#[derive(Debug, Error)]
pub enum AudioError {
    /// Kira manager error
    #[error("Audio manager error: {0}")]
    ManagerError(String),

    /// Sound not found
    #[error("Sound not found: {0}")]
    SoundNotFound(String),

    /// Invalid sound instance
    #[error("Invalid sound instance: {0}")]
    InvalidInstance(u32),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Decode error
    #[error("Audio decode error: {0}")]
    DecodeError(String),

    /// Effect error
    #[error("Audio effect error: {0}")]
    EffectError(String),
}

/// Audio result type
pub type AudioResult<T> = Result<T, AudioError>;
