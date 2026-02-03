//! Platform-agnostic input handling.
//!
//! This module provides cross-platform input abstractions for:
//! - Keyboard input (key press/release/held state)
//! - Mouse input (position, movement, button presses)
//! - Gamepad input (buttons, axes)
//!
//! The input system is designed to be used with the platform abstraction layer,
//! ensuring that business logic never contains platform-specific input code.

pub mod backend;
mod components;
mod events;
mod gamepad;
mod keyboard;
mod manager;
mod mouse;
mod system;

pub use backend::InputBackend;
pub use components::{InputActions, InputState};
pub use events::InputEvent;
pub use gamepad::{GamepadAxis, GamepadButton, GamepadId};
pub use keyboard::KeyCode;
pub use manager::InputManager;
pub use mouse::MouseButton;
pub use system::InputSystem;

/// Create the platform-specific input backend.
///
/// This factory function selects the appropriate input backend implementation
/// based on the target platform (Windows, Linux, macOS).
pub fn create_input_backend() -> Result<Box<dyn InputBackend>, crate::platform::PlatformError> {
    #[cfg(target_os = "windows")]
    {
        use backend::windows::WindowsInput;
        Ok(Box::new(WindowsInput::new()?))
    }

    #[cfg(target_os = "linux")]
    {
        use backend::linux::LinuxInput;
        Ok(Box::new(LinuxInput::new()?))
    }

    #[cfg(target_os = "macos")]
    {
        use backend::macos::MacOSInput;
        Ok(Box::new(MacOSInput::new()?))
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        Err(crate::platform::PlatformError::platformnotsupported(
            std::env::consts::OS.to_string(),
            "input".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_input_backend() {
        // Should create backend for current platform
        let result = create_input_backend();
        assert!(result.is_ok());
    }

    #[test]
    fn test_input_backend_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Box<dyn InputBackend>>();
    }
}
