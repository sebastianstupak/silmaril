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

pub mod context;
pub mod error;
pub mod offscreen;
pub mod render_pass;
pub mod surface;
pub mod swapchain;
pub mod window;

// Re-export commonly used types
pub use context::{QueueFamilies, VulkanContext};
pub use error::RendererError;
pub use offscreen::OffscreenTarget;
pub use render_pass::{RenderPass, RenderPassConfig, RenderPassError};
pub use surface::{Surface, SurfaceError};
pub use swapchain::Swapchain;
pub use window::{Window, WindowConfig, WindowError, WindowEventType};
