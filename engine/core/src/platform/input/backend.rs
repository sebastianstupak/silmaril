//! Input backend trait and platform-specific implementations.

use super::{GamepadAxis, GamepadButton, GamepadId, InputEvent, KeyCode, MouseButton};
use crate::platform::PlatformError;

/// Platform-agnostic input backend.
///
/// Implementations handle the platform-specific details of polling input devices
/// and converting native events to our platform-agnostic event types.
pub trait InputBackend: Send + Sync {
    /// Poll for new input events since the last call.
    ///
    /// This should be called once per frame to collect all input events.
    /// Events are returned in the order they occurred.
    fn poll_events(&mut self) -> Vec<InputEvent>;

    /// Check if a key is currently pressed.
    fn is_key_down(&self, key: KeyCode) -> bool;

    /// Check if a mouse button is currently pressed.
    fn is_mouse_button_down(&self, button: MouseButton) -> bool;

    /// Get the current mouse position in window coordinates.
    ///
    /// Returns (0.0, 0.0) if the mouse is outside the window or position is unknown.
    fn mouse_position(&self) -> (f32, f32);

    /// Get the mouse position delta since the last poll.
    ///
    /// Useful for FPS-style mouse controls.
    fn mouse_delta(&self) -> (f32, f32);

    /// Check if a gamepad button is currently pressed.
    fn is_gamepad_button_down(&self, id: GamepadId, button: GamepadButton) -> bool;

    /// Get the current value of a gamepad axis.
    ///
    /// Returns 0.0 if the gamepad is not connected or axis is unknown.
    fn gamepad_axis(&self, id: GamepadId, axis: GamepadAxis) -> f32;

    /// Get the number of connected gamepads.
    fn gamepad_count(&self) -> usize;
}

// Platform-specific implementations
#[cfg(target_os = "windows")]
#[allow(missing_docs)] // Internal implementation details
pub mod windows {
    use super::*;
    use rustc_hash::FxHashMap;
    use std::collections::VecDeque;
    use tracing::debug;

    /// Windows input backend.
    ///
    /// This implementation uses winit for cross-platform input handling.
    /// While winit is cross-platform, we keep platform-specific modules
    /// to allow for future platform-specific optimizations.
    pub struct WindowsInput {
        // Event queue
        event_queue: VecDeque<InputEvent>,

        // Current input state
        keys_down: FxHashMap<KeyCode, bool>,
        mouse_buttons_down: FxHashMap<MouseButton, bool>,
        mouse_position: (f32, f32),
        mouse_delta: (f32, f32),
        last_mouse_position: (f32, f32),

        // Gamepad state
        gamepad_buttons: FxHashMap<(GamepadId, GamepadButton), bool>,
        gamepad_axes: FxHashMap<(GamepadId, GamepadAxis), f32>,
        connected_gamepads: Vec<GamepadId>,
    }

    impl WindowsInput {
        pub fn new() -> Result<Self, PlatformError> {
            debug!("Initializing Windows input backend");

            Ok(Self {
                event_queue: VecDeque::new(),
                keys_down: FxHashMap::default(),
                mouse_buttons_down: FxHashMap::default(),
                mouse_position: (0.0, 0.0),
                mouse_delta: (0.0, 0.0),
                last_mouse_position: (0.0, 0.0),
                gamepad_buttons: FxHashMap::default(),
                gamepad_axes: FxHashMap::default(),
                connected_gamepads: Vec::new(),
            })
        }

        /// Feed an input event to the backend (for integration with windowing system).
        ///
        /// This is used by the windowing system to feed input events to the backend.
        /// In production, this would be called by the winit event loop or similar.
        #[allow(dead_code)] // Used by windowing integration and tests
        pub fn feed_event(&mut self, event: InputEvent) {
            // Update state based on event
            match &event {
                InputEvent::KeyPressed { key } => {
                    self.keys_down.insert(*key, true);
                }
                InputEvent::KeyReleased { key } => {
                    self.keys_down.insert(*key, false);
                }
                InputEvent::MouseMoved { x, y } => {
                    self.mouse_position = (*x, *y);
                }
                InputEvent::MouseMotion { delta_x, delta_y } => {
                    self.mouse_delta = (*delta_x, *delta_y);
                }
                InputEvent::MouseButtonPressed { button } => {
                    self.mouse_buttons_down.insert(*button, true);
                }
                InputEvent::MouseButtonReleased { button } => {
                    self.mouse_buttons_down.insert(*button, false);
                }
                InputEvent::GamepadConnected { id } => {
                    if !self.connected_gamepads.contains(id) {
                        self.connected_gamepads.push(*id);
                    }
                }
                InputEvent::GamepadDisconnected { id } => {
                    self.connected_gamepads.retain(|&gp_id| gp_id != *id);
                }
                InputEvent::GamepadButtonPressed { id, button } => {
                    self.gamepad_buttons.insert((*id, *button), true);
                }
                InputEvent::GamepadButtonReleased { id, button } => {
                    self.gamepad_buttons.insert((*id, *button), false);
                }
                InputEvent::GamepadAxis { id, axis, value } => {
                    self.gamepad_axes.insert((*id, *axis), *value);
                }
                _ => {}
            }

            // Queue event for consumers
            self.event_queue.push_back(event);
        }
    }

    impl InputBackend for WindowsInput {
        fn poll_events(&mut self) -> Vec<InputEvent> {
            // Calculate mouse delta from position changes
            let new_delta_x = self.mouse_position.0 - self.last_mouse_position.0;
            let new_delta_y = self.mouse_position.1 - self.last_mouse_position.1;

            if new_delta_x != 0.0 || new_delta_y != 0.0 {
                self.mouse_delta = (new_delta_x, new_delta_y);
            }

            self.last_mouse_position = self.mouse_position;

            // Drain event queue
            self.event_queue.drain(..).collect()
        }

        fn is_key_down(&self, key: KeyCode) -> bool {
            self.keys_down.get(&key).copied().unwrap_or(false)
        }

        fn is_mouse_button_down(&self, button: MouseButton) -> bool {
            self.mouse_buttons_down.get(&button).copied().unwrap_or(false)
        }

        fn mouse_position(&self) -> (f32, f32) {
            self.mouse_position
        }

        fn mouse_delta(&self) -> (f32, f32) {
            self.mouse_delta
        }

        fn is_gamepad_button_down(&self, id: GamepadId, button: GamepadButton) -> bool {
            self.gamepad_buttons.get(&(id, button)).copied().unwrap_or(false)
        }

        fn gamepad_axis(&self, id: GamepadId, axis: GamepadAxis) -> f32 {
            self.gamepad_axes.get(&(id, axis)).copied().unwrap_or(0.0)
        }

        fn gamepad_count(&self) -> usize {
            self.connected_gamepads.len()
        }
    }
}

#[cfg(target_os = "linux")]
#[allow(missing_docs)] // Internal implementation details
pub mod linux {
    use super::*;

    /// Linux input backend.
    ///
    /// Currently uses the same implementation as Windows (winit-based).
    /// Could be extended with Linux-specific input handling (evdev, etc.).
    pub type LinuxInput = super::windows::WindowsInput;
}

#[cfg(target_os = "macos")]
#[allow(missing_docs)] // Internal implementation details
pub mod macos {
    use super::*;

    /// macOS input backend.
    ///
    /// Currently uses the same implementation as Windows (winit-based).
    /// Could be extended with macOS-specific input handling (IOKit, etc.).
    pub type MacOSInput = super::windows::WindowsInput;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_backend_trait_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Box<dyn InputBackend>>();
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_windows_input_creation() {
        use windows::WindowsInput;

        let input = WindowsInput::new();
        assert!(input.is_ok());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_keyboard_state() {
        use windows::WindowsInput;

        let mut input = WindowsInput::new().unwrap();

        // Initially, no keys are pressed
        assert!(!input.is_key_down(KeyCode::A));

        // Feed a key press event
        input.feed_event(InputEvent::KeyPressed { key: KeyCode::A });
        assert!(input.is_key_down(KeyCode::A));

        // Feed a key release event
        input.feed_event(InputEvent::KeyReleased { key: KeyCode::A });
        assert!(!input.is_key_down(KeyCode::A));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_mouse_state() {
        use windows::WindowsInput;

        let mut input = WindowsInput::new().unwrap();

        // Initially, no buttons are pressed
        assert!(!input.is_mouse_button_down(MouseButton::Left));
        assert_eq!(input.mouse_position(), (0.0, 0.0));

        // Feed mouse move event
        input.feed_event(InputEvent::MouseMoved { x: 100.0, y: 200.0 });
        assert_eq!(input.mouse_position(), (100.0, 200.0));

        // Feed mouse button press event
        input.feed_event(InputEvent::MouseButtonPressed { button: MouseButton::Left });
        assert!(input.is_mouse_button_down(MouseButton::Left));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_gamepad_state() {
        use windows::WindowsInput;

        let mut input = WindowsInput::new().unwrap();
        let gamepad_id = GamepadId::new(0);

        // Initially, no gamepads are connected
        assert_eq!(input.gamepad_count(), 0);

        // Connect a gamepad
        input.feed_event(InputEvent::GamepadConnected { id: gamepad_id });
        assert_eq!(input.gamepad_count(), 1);

        // Press a button
        input.feed_event(InputEvent::GamepadButtonPressed {
            id: gamepad_id,
            button: GamepadButton::South,
        });
        assert!(input.is_gamepad_button_down(gamepad_id, GamepadButton::South));

        // Update an axis
        input.feed_event(InputEvent::GamepadAxis {
            id: gamepad_id,
            axis: GamepadAxis::LeftStickX,
            value: 0.5,
        });
        assert_eq!(input.gamepad_axis(gamepad_id, GamepadAxis::LeftStickX), 0.5);

        // Disconnect gamepad
        input.feed_event(InputEvent::GamepadDisconnected { id: gamepad_id });
        assert_eq!(input.gamepad_count(), 0);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_event_polling() {
        use windows::WindowsInput;

        let mut input = WindowsInput::new().unwrap();

        // Feed multiple events
        input.feed_event(InputEvent::KeyPressed { key: KeyCode::A });
        input.feed_event(InputEvent::MouseMoved { x: 50.0, y: 100.0 });

        // Poll events
        let events = input.poll_events();
        assert_eq!(events.len(), 2);

        // Second poll should return empty
        let events2 = input.poll_events();
        assert_eq!(events2.len(), 0);
    }
}
