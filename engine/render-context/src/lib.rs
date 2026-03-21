//! Shared Vulkan render context plumbing.
//!
//! This crate provides the core Vulkan infrastructure (instance, device,
//! swapchain, allocator) shared between the renderer and the editor viewport.

#![warn(missing_docs)]

/// Creates a Tracy profiling span for the given name.
///
/// Compiles to a no-op when the `profiling` feature is not enabled.
/// Use this macro in performance-critical paths to instrument them for Tracy.
///
/// # Example
/// ```ignore
/// profile_scope!("render_frame");
/// ```
#[cfg(feature = "profiling")]
#[macro_export]
macro_rules! profile_scope {
    ($name:expr) => {
        let _tracy_span = tracy_client::span!($name);
    };
}

/// Creates a Tracy profiling span for the given name.
///
/// Compiles to a no-op when the `profiling` feature is not enabled.
/// Use this macro in performance-critical paths to instrument them for Tracy.
///
/// # Example
/// ```ignore
/// profile_scope!("render_frame");
/// ```
#[cfg(not(feature = "profiling"))]
#[macro_export]
macro_rules! profile_scope {
    ($name:expr) => {};
}

pub mod buffer;
pub mod command;
pub mod context;
pub mod depth;
pub mod error;
pub mod framebuffer;
pub mod pipeline;
pub mod render_pass;
pub mod shader;
pub mod surface;
pub mod swapchain;
pub mod sync;
pub mod window;

pub use error::{RenderContextError, RendererError};

// Buffer types
pub use buffer::{GpuBuffer, GpuMesh, IndexBuffer, VertexBuffer};

// Command types
pub use command::{CommandBuffer, CommandError, CommandPool};

// Context types
pub use context::{QueueFamilies, VulkanContext};

// Depth types
pub use depth::DepthBuffer;

// Framebuffer types
pub use framebuffer::{create_framebuffers, Framebuffer, FramebufferError};

// Pipeline types
pub use pipeline::GraphicsPipeline;

// Render pass types
pub use render_pass::{RenderPass, RenderPassConfig, RenderPassError};

// Shader types
pub use shader::{stage_from_extension, ShaderModule};

// Surface types
pub use surface::{Surface, SurfaceError};

// Swapchain types
pub use swapchain::Swapchain;

// Sync types
pub use sync::{create_sync_objects, FrameResources, FrameSync, FrameSyncObjects, SyncError};

// Window types
pub use window::{Window, WindowConfig, WindowError, WindowEventType};
