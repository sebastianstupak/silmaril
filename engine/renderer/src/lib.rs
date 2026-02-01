//! Engine Renderer
//!
//! Provides Vulkan-based rendering:
//! - Vulkan context and device management
//! - Mesh rendering with PBR materials
//! - Forward+ lighting system
//! - Frame capture capabilities

#![warn(missing_docs)]

pub mod context;
pub mod mesh;
pub mod material;
pub mod lighting;
pub mod capture;

// Re-export commonly used types
pub use context::Renderer;
pub use mesh::Mesh;
pub use material::Material;
