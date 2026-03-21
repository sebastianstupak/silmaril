//! Viewport module -- native child window for Vulkan rendering.
//!
//! The `native_viewport` submodule manages a platform-native child window
//! for Vulkan surface rendering using the full `engine-renderer` pipeline.

pub mod gizmo_pipeline;
pub mod native_viewport;

pub use gizmo_pipeline::{GizmoAxis, GizmoMode, GizmoVertex};
