//! Platform abstraction layer.
//!
//! This module provides cross-platform abstractions for:
//! - Window management
//! - Input handling
//! - Time and timing
//! - Filesystem operations
//! - Threading primitives
//!
//! The platform layer ensures that business logic never contains platform-specific
//! code (`#[cfg(target_os = ...)]`). Instead, all platform-specific implementations
//! are hidden behind traits in this module.

pub mod error;
pub mod filesystem;
mod info;
pub mod input;
pub mod threading;
pub mod time;

// Re-export commonly used types
pub use error::PlatformError;
pub use filesystem::{create_filesystem_backend, FileSystemBackend};
pub use info::PlatformInfo;
pub use input::{
    create_input_backend, GamepadAxis, GamepadButton, GamepadId, InputActions, InputBackend,
    InputEvent, InputManager, InputState, InputSystem, KeyCode, MouseButton,
};
pub use threading::{create_threading_backend, ThreadPriority, ThreadingBackend};
pub use time::{create_time_backend, TimeBackend};
