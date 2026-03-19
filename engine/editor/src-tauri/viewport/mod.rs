//! Viewport module — native child window for Vulkan rendering.
//!
//! The `native_viewport` submodule manages a platform-native child window
//! for Vulkan surface rendering.

pub mod native_viewport;
#[cfg(windows)]
pub mod vulkan_viewport;
