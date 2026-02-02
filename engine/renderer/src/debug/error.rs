//! Error types for rendering debug infrastructure

use engine_core::{EngineError, ErrorCode, ErrorSeverity};
use engine_macros::define_error;

define_error! {
    /// Validation errors for rendering debug snapshots
    pub enum ValidationError {
        /// Invalid timestamp in debug snapshot
        InvalidTimestamp { timestamp: f64 } =
            ErrorCode::InvalidTimestamp,
            ErrorSeverity::Error,

        /// Invalid viewport dimensions
        InvalidViewport {} =
            ErrorCode::InvalidViewport,
            ErrorSeverity::Error,

        /// Invalid draw call data
        InvalidDrawCall { index: usize, message: String } =
            ErrorCode::InvalidDrawCall,
            ErrorSeverity::Error,

        /// Invalid transform matrix (contains NaN or Inf)
        InvalidTransform {} =
            ErrorCode::InvalidTransform,
            ErrorSeverity::Error,

        /// Draw call has zero vertices
        ZeroVertices {} =
            ErrorCode::ZeroVertices,
            ErrorSeverity::Error,
    }
}
