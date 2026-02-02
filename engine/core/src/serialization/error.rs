//! Serialization error types.
//!
//! This module defines all errors that can occur during serialization and deserialization
//! of game state, including YAML, Bincode, and FlatBuffers formats.

use crate::{EngineError, ErrorCode, ErrorSeverity};
use engine_macros::define_error;

define_error! {
    pub enum SerializationError {
        YamlSerialize { details: String } = ErrorCode::YamlSerializeFailed, ErrorSeverity::Error,
        YamlDeserialize { details: String } = ErrorCode::YamlDeserializeFailed, ErrorSeverity::Error,
        BincodeSerialize { details: String } = ErrorCode::BincodeSerializeFailed, ErrorSeverity::Error,
        BincodeDeserialize { details: String } = ErrorCode::BincodeDeserializeFailed, ErrorSeverity::Error,
        FlatBuffersSerialize { details: String } = ErrorCode::FlatbuffersSerializeFailed, ErrorSeverity::Error,
        FlatBuffersDeserialize { details: String } = ErrorCode::FlatbuffersDeserializeFailed, ErrorSeverity::Error,
        IoError { details: String } = ErrorCode::FileSystemError, ErrorSeverity::Error,
        Utf8Error { details: String } = ErrorCode::InvalidFormat, ErrorSeverity::Error,
        CompressionError { details: String } = ErrorCode::SerializationFailed, ErrorSeverity::Error,
        DecompressionError { details: String } = ErrorCode::DeserializationFailed, ErrorSeverity::Error,
    }
}

// Implement From<std::io::Error> for convenient error conversion
impl From<std::io::Error> for SerializationError {
    fn from(error: std::io::Error) -> Self {
        Self::ioerror(error.to_string())
    }
}

// Implement From<std::string::FromUtf8Error> for convenient error conversion
impl From<std::string::FromUtf8Error> for SerializationError {
    fn from(error: std::string::FromUtf8Error) -> Self {
        Self::utf8error(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yaml_serialize_error() {
        let error = SerializationError::yamlserialize("invalid YAML".to_string());
        assert_eq!(error.code(), ErrorCode::YamlSerializeFailed);
        assert_eq!(error.severity(), ErrorSeverity::Error);
    }

    #[test]
    fn test_yaml_deserialize_error() {
        let error = SerializationError::yamldeserialize("parse error".to_string());
        assert_eq!(error.code(), ErrorCode::YamlDeserializeFailed);
        assert_eq!(error.severity(), ErrorSeverity::Error);
    }

    #[test]
    fn test_bincode_errors() {
        let ser_err = SerializationError::bincodeserialize("test".to_string());
        let de_err = SerializationError::bincodedeserialize("test".to_string());

        assert_eq!(ser_err.code(), ErrorCode::BincodeSerializeFailed);
        assert_eq!(de_err.code(), ErrorCode::BincodeDeserializeFailed);
    }

    #[test]
    fn test_flatbuffers_errors() {
        let ser_err = SerializationError::flatbuffersserialize("test".to_string());
        let de_err = SerializationError::flatbuffersdeserialize("test".to_string());

        assert_eq!(ser_err.code(), ErrorCode::FlatbuffersSerializeFailed);
        assert_eq!(de_err.code(), ErrorCode::FlatbuffersDeserializeFailed);
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let ser_err: SerializationError = io_err.into();

        assert_eq!(ser_err.code(), ErrorCode::FileSystemError);
        let display = format!("{}", ser_err);
        assert!(display.contains("IoError"));
    }

    #[test]
    fn test_utf8_error_conversion() {
        let bytes = vec![0, 159, 146, 150]; // Invalid UTF-8
        let utf8_err = String::from_utf8(bytes).unwrap_err();
        let ser_err: SerializationError = utf8_err.into();

        assert_eq!(ser_err.code(), ErrorCode::InvalidFormat);
        let display = format!("{}", ser_err);
        assert!(display.contains("Utf8Error"));
    }

    #[test]
    fn test_error_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<SerializationError>();
    }

    #[test]
    fn test_result_usage() {
        fn returns_error() -> Result<(), SerializationError> {
            Err(SerializationError::yamlserialize("failed".to_string()))
        }

        let result = returns_error();
        assert!(result.is_err());
    }

    #[test]
    #[cfg(feature = "backtrace")]
    fn test_backtrace_captured() {
        let error = SerializationError::yamlserialize("test error".to_string());
        let backtrace = error.backtrace();
        assert!(backtrace.is_some(), "Backtrace should be captured when feature is enabled");
    }

    #[test]
    #[cfg(not(feature = "backtrace"))]
    fn test_backtrace_not_captured() {
        let error = SerializationError::yamlserialize("test error".to_string());
        let backtrace = error.backtrace();
        assert!(backtrace.is_none(), "Backtrace should not be captured when feature is disabled");
    }

    #[test]
    fn test_error_log_includes_context() {
        // Just verify that the error can be logged without panicking
        let error = SerializationError::bincodeserialize("test".to_string());
        error.log();
        // If we get here without panicking, the test passes
    }
}
