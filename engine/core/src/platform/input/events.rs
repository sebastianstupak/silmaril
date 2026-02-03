//! Input event types.

use super::{GamepadAxis, GamepadButton, GamepadId, KeyCode, MouseButton};

/// Platform-agnostic input event.
///
/// Input events represent discrete input actions that occurred since the last poll.
/// The input manager maintains state based on these events.
#[allow(missing_docs)] // Event field names are self-explanatory
#[derive(Debug, Clone, PartialEq)]
pub enum InputEvent {
    // Keyboard events
    /// A key was pressed down.
    KeyPressed { key: KeyCode },
    /// A key was released.
    KeyReleased { key: KeyCode },

    // Mouse events
    /// Mouse cursor moved to absolute position (window coordinates).
    MouseMoved { x: f32, y: f32 },
    /// Mouse moved by relative delta (useful for FPS controls).
    MouseMotion { delta_x: f32, delta_y: f32 },
    /// Mouse button was pressed.
    MouseButtonPressed { button: MouseButton },
    /// Mouse button was released.
    MouseButtonReleased { button: MouseButton },
    /// Mouse wheel scrolled.
    MouseWheel { delta_x: f32, delta_y: f32 },
    /// Mouse entered the window.
    MouseEntered,
    /// Mouse left the window.
    MouseLeft,

    // Gamepad events
    /// Gamepad was connected.
    GamepadConnected { id: GamepadId },
    /// Gamepad was disconnected.
    GamepadDisconnected { id: GamepadId },
    /// Gamepad button was pressed.
    GamepadButtonPressed { id: GamepadId, button: GamepadButton },
    /// Gamepad button was released.
    GamepadButtonReleased { id: GamepadId, button: GamepadButton },
    /// Gamepad axis changed value.
    GamepadAxis { id: GamepadId, axis: GamepadAxis, value: f32 },
}

impl InputEvent {
    /// Returns true if this is a keyboard event.
    pub fn is_keyboard(&self) -> bool {
        matches!(self, InputEvent::KeyPressed { .. } | InputEvent::KeyReleased { .. })
    }

    /// Returns true if this is a mouse event.
    pub fn is_mouse(&self) -> bool {
        matches!(
            self,
            InputEvent::MouseMoved { .. }
                | InputEvent::MouseMotion { .. }
                | InputEvent::MouseButtonPressed { .. }
                | InputEvent::MouseButtonReleased { .. }
                | InputEvent::MouseWheel { .. }
                | InputEvent::MouseEntered
                | InputEvent::MouseLeft
        )
    }

    /// Returns true if this is a gamepad event.
    pub fn is_gamepad(&self) -> bool {
        matches!(
            self,
            InputEvent::GamepadConnected { .. }
                | InputEvent::GamepadDisconnected { .. }
                | InputEvent::GamepadButtonPressed { .. }
                | InputEvent::GamepadButtonReleased { .. }
                | InputEvent::GamepadAxis { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyboard_events() {
        let event = InputEvent::KeyPressed { key: KeyCode::A };
        assert!(event.is_keyboard());
        assert!(!event.is_mouse());
        assert!(!event.is_gamepad());
    }

    #[test]
    fn test_mouse_events() {
        let event = InputEvent::MouseMoved { x: 100.0, y: 200.0 };
        assert!(!event.is_keyboard());
        assert!(event.is_mouse());
        assert!(!event.is_gamepad());
    }

    #[test]
    fn test_gamepad_events() {
        let event = InputEvent::GamepadConnected { id: GamepadId::new(0) };
        assert!(!event.is_keyboard());
        assert!(!event.is_mouse());
        assert!(event.is_gamepad());
    }
}
