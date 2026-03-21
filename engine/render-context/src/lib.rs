//! Shared Vulkan render context plumbing.
//!
//! This crate provides the core Vulkan infrastructure (instance, device,
//! swapchain, allocator) shared between the renderer and the editor viewport.

#![warn(missing_docs)]

pub mod error;
pub use error::RenderContextError;
