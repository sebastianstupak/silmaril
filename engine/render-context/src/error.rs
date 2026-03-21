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
        NotImplemented { feature: String } = ErrorCode::InvalidFormat, ErrorSeverity::Error,
    }
}
