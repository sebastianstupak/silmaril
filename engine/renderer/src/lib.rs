//! Engine Renderer
//!
//! Provides Vulkan-based rendering:
//! - Vulkan context and device management
//! - Swapchain management for presentation
//! - Offscreen rendering for headless mode and frame capture
//! - Mesh rendering with PBR materials (future)
//! - Forward+ lighting system (future)
//! - Frame capture capabilities (future)
//!
//! # Phase 1.5 Implementation
//!
//! This crate currently implements Phase 1.5: Vulkan Context with:
//! - Instance creation with validation layers (debug builds only)
//! - Physical device selection with scoring algorithm
//! - Logical device creation with queue management
//! - GPU memory allocation via gpu-allocator
//! - Swapchain creation with optimal configuration
//! - Offscreen render targets for headless rendering
//!
//! # Example
//!
//! ```no_run
//! use engine_renderer::{VulkanContext};
//!
//! // Create headless context (no window)
//! let context = VulkanContext::new("MyApp", None, None)?;
//!
//! // Or with a window surface
//! # use ash::vk;
//! # let surface = vk::SurfaceKHR::null();
//! # let surface_loader = todo!();
//! let context = VulkanContext::new("MyApp", Some(surface), Some(&surface_loader))?;
//! # Ok::<(), engine_renderer::RendererError>(())
//! ```

#![warn(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]
// Disable print statements per CLAUDE.md
#![warn(clippy::print_stdout)]
#![warn(clippy::print_stderr)]

pub mod agentic_debug;
pub mod asset_bridge;
pub mod buffer;
pub mod capture;
pub mod command;
pub mod context;
pub mod debug;
pub mod depth;
pub mod error;
pub mod framebuffer;
pub mod gpu_cache;
pub mod offscreen;
pub mod pipeline;
pub mod render_pass;
pub mod renderer;
pub mod shader;
pub mod surface;
pub mod swapchain;
pub mod sync;
pub mod window;

// Re-export commonly used types
pub use asset_bridge::{AssetBridge, AssetBridgeStats, GpuShader, GpuTexture};
pub use buffer::{GpuBuffer, GpuMesh, IndexBuffer, VertexBuffer};
pub use capture::{
    CaptureConfig, CaptureFormat, CaptureManager, CaptureMetrics, FrameEncoder, FrameReadback,
    MetricsTracker,
};
pub use command::{CommandBuffer, CommandError, CommandPool};
pub use context::{QueueFamilies, VulkanContext};
pub use depth::DepthBuffer;
pub use error::RendererError;
pub use framebuffer::{create_framebuffers, Framebuffer, FramebufferError};
pub use gpu_cache::{GpuCache, GpuCachedMesh, MeshInfo};
pub use offscreen::OffscreenTarget;
pub use pipeline::GraphicsPipeline;
pub use render_pass::{RenderPass, RenderPassConfig, RenderPassError};
pub use renderer::{FrameRecorder, Renderer, ViewportDescriptor};
pub use shader::{stage_from_extension, ShaderModule};
pub use surface::{Surface, SurfaceError};
pub use swapchain::Swapchain;
pub use sync::{create_sync_objects, FrameResources, FrameSync, FrameSyncObjects, SyncError};
pub use window::{Window, WindowConfig, WindowError, WindowEventType};

// Re-export winit types for external use
pub use winit::window::Window as WinitWindow;

// Re-export Rect from engine-render-context for use with ViewportDescriptor
pub use engine_render_context::Rect;

// Re-export from engine-assets for convenience
pub use engine_assets::{MeshData, Vertex};
