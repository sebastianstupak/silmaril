//! Renderer error types using structured error infrastructure.
//!
//! All renderer errors use custom error types with proper error codes and severity levels.
//! Never use `anyhow` or `Box<dyn Error>` per CLAUDE.md coding standards.

use engine_core::{EngineError, ErrorCode, ErrorSeverity};
use engine_macros::define_error;

define_error! {
    pub enum RendererError {
        InstanceCreationFailed { reason: String } = ErrorCode::InstanceCreationFailed, ErrorSeverity::Critical,
        DeviceEnumerationFailed { reason: String } = ErrorCode::DeviceEnumerationFailed, ErrorSeverity::Critical,
        NoSuitableGpu { available_devices: usize } = ErrorCode::NoSuitableGpu, ErrorSeverity::Critical,
        ExtensionNotSupported { extension_name: String } = ErrorCode::ExtensionNotSupported, ErrorSeverity::Error,
        ValidationLayerNotAvailable { layer_name: String } = ErrorCode::ValidationLayerNotAvailable, ErrorSeverity::Warning,
        DebugMessengerCreationFailed { reason: String } = ErrorCode::DebugMessengerCreationFailed, ErrorSeverity::Warning,
        QueueFamilyNotFound { queue_type: String } = ErrorCode::QueueFamilyNotFound, ErrorSeverity::Critical,
        LogicalDeviceCreationFailed { reason: String } = ErrorCode::LogicalDeviceCreationFailed, ErrorSeverity::Critical,
        SurfaceCreationFailed { reason: String } = ErrorCode::SurfaceCreationFailedRenderer, ErrorSeverity::Critical,
        SurfaceCapabilitiesQueryFailed { reason: String } = ErrorCode::SurfaceCapabilitiesQueryFailed, ErrorSeverity::Error,
        SurfaceFormatQueryFailed { reason: String } = ErrorCode::SurfaceFormatQueryFailed, ErrorSeverity::Error,
        PresentModeQueryFailed { reason: String } = ErrorCode::PresentModeQueryFailed, ErrorSeverity::Error,
        SwapchainCreationFailed { reason: String } = ErrorCode::SwapchainCreationFailed, ErrorSeverity::Error,
        SwapchainImageRetrievalFailed { reason: String } = ErrorCode::SwapchainCreationFailed, ErrorSeverity::Error,
        SwapchainAcquisitionFailed { reason: String } = ErrorCode::SwapchainCreationFailed, ErrorSeverity::Error,
        MemoryAllocationFailed { size: u64, reason: String } = ErrorCode::MemoryAllocationFailed, ErrorSeverity::Error,
        BufferCreationFailed { size: u64, reason: String } = ErrorCode::BufferCreationFailed, ErrorSeverity::Error,
        ImageCreationFailed { width: u32, height: u32, reason: String } = ErrorCode::ImageCreationFailed, ErrorSeverity::Error,
        ImageViewCreationFailed { reason: String } = ErrorCode::ImageViewCreationFailed, ErrorSeverity::Error,
        MemoryMappingFailed { reason: String } = ErrorCode::MemoryMappingFailed, ErrorSeverity::Error,
        CommandPoolCreationFailed { reason: String } = ErrorCode::CommandPoolCreationFailed, ErrorSeverity::Error,
        RenderPassCreationFailed { reason: String } = ErrorCode::RenderPassCreationFailed, ErrorSeverity::Error,
        FramebufferCreationFailed { reason: String } = ErrorCode::FramebufferCreationFailed, ErrorSeverity::Error,
        SyncObjectCreationFailed { object_type: String, reason: String } = ErrorCode::SyncObjectCreationFailed, ErrorSeverity::Error,
        CommandBufferAllocationFailed { count: u32, reason: String } = ErrorCode::CommandBufferAllocationFailed, ErrorSeverity::Error,
        PipelineCreationFailed { reason: String } = ErrorCode::PipelineCreationFailed, ErrorSeverity::Error,
        ShaderCompileFailed { path: String, reason: String } = ErrorCode::ShaderCompileFailed, ErrorSeverity::Error,
        ShaderCompilationFailed { reason: String } = ErrorCode::ShaderCompileFailed, ErrorSeverity::Error,
        ShaderNotFound { path: String, reason: String } = ErrorCode::ShaderCompileFailed, ErrorSeverity::Error,
        InvalidShaderFormat { reason: String } = ErrorCode::ShaderCompileFailed, ErrorSeverity::Error,
        ShaderModuleCreationFailed { reason: String } = ErrorCode::ShaderModuleCreationFailed, ErrorSeverity::Error,
        QueueSubmissionFailed { reason: String } = ErrorCode::QueueSubmissionFailed, ErrorSeverity::Error,
        PresentFailed { reason: String } = ErrorCode::PresentFailed, ErrorSeverity::Warning,
        SwapchainOutOfDate {} = ErrorCode::SwapchainOutOfDate, ErrorSeverity::Warning,
        SwapchainSuboptimal {} = ErrorCode::SwapchainSuboptimal, ErrorSeverity::Warning,
        DeviceLost { reason: String } = ErrorCode::DeviceLost, ErrorSeverity::Critical,
        InvalidMeshData { reason: String } = ErrorCode::InvalidMeshData, ErrorSeverity::Error,
        AssetNotFound { asset_id: String } = ErrorCode::AssetLoadFailed, ErrorSeverity::Error,
        NotImplemented { feature: String } = ErrorCode::InvalidOperation, ErrorSeverity::Error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_renderer_error_codes() {
        let err = RendererError::nosuitablegpu(0);
        assert_eq!(err.code(), ErrorCode::NoSuitableGpu);
        assert_eq!(err.severity(), ErrorSeverity::Critical);
    }

    #[test]
    fn test_renderer_error_display() {
        let err = RendererError::instancecreationfailed("test reason".to_string());
        let display = format!("{}", err);
        assert!(display.contains("InstanceCreationFailed") || display.contains("test reason"));
    }

    #[test]
    fn test_warning_severity() {
        let err = RendererError::swapchainsuboptimal();
        assert_eq!(err.severity(), ErrorSeverity::Warning);
    }

    #[test]
    fn test_critical_severity() {
        let err = RendererError::devicelost("GPU driver crash".to_string());
        assert_eq!(err.severity(), ErrorSeverity::Critical);
    }
}
