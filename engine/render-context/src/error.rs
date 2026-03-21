//! Error types for the render context crate.

use engine_core::{EngineError, ErrorCode, ErrorSeverity};
use engine_macros::define_error;

define_error! {
    pub enum RenderContextError {
        VulkanInit { reason: String } = ErrorCode::VulkanInitFailed, ErrorSeverity::Critical,
        SwapchainOutOfDate { } = ErrorCode::SwapchainOutOfDate, ErrorSeverity::Warning,
        DeviceLost { } = ErrorCode::DeviceLost, ErrorSeverity::Critical,
    }
}
