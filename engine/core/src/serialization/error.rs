//! Serialization error types

use std::fmt;

/// Errors that can occur during serialization/deserialization
#[derive(Debug)]
pub enum SerializationError {
    /// YAML serialization failed
    YamlSerialize {
        /// Error details
        details: String
    },
    /// YAML deserialization failed
    YamlDeserialize {
        /// Error details
        details: String
    },
    /// Bincode serialization failed
    BincodeSerialize {
        /// Error details
        details: String
    },
    /// Bincode deserialization failed
    BincodeDeserialize {
        /// Error details
        details: String
    },
    /// FlatBuffers serialization failed
    FlatBuffersSerialize {
        /// Error details
        details: String
    },
    /// FlatBuffers deserialization failed
    FlatBuffersDeserialize {
        /// Error details
        details: String
    },
    /// I/O error during serialization
    IoError {
        /// The underlying I/O error
        source: std::io::Error
    },
    /// UTF-8 conversion error
    Utf8Error {
        /// Error details
        details: String
    },
}

impl fmt::Display for SerializationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::YamlSerialize { details } => write!(f, "YAML serialization failed: {}", details),
            Self::YamlDeserialize { details } => write!(f, "YAML deserialization failed: {}", details),
            Self::BincodeSerialize { details } => write!(f, "Bincode serialization failed: {}", details),
            Self::BincodeDeserialize { details } => write!(f, "Bincode deserialization failed: {}", details),
            Self::FlatBuffersSerialize { details } => write!(f, "FlatBuffers serialization failed: {}", details),
            Self::FlatBuffersDeserialize { details } => write!(f, "FlatBuffers deserialization failed: {}", details),
            Self::IoError { source } => write!(f, "I/O error: {}", source),
            Self::Utf8Error { details } => write!(f, "UTF-8 conversion error: {}", details),
        }
    }
}

impl std::error::Error for SerializationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::IoError { source } => Some(source),
            _ => None,
        }
    }
}

impl From<std::io::Error> for SerializationError {
    fn from(error: std::io::Error) -> Self {
        Self::IoError { source: error }
    }
}
