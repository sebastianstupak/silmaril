//! High-level input manager.
//!
//! The input manager provides a simplified API for querying input state,
//! handling key bindings, and buffering input for game logic.

use super::{
    GamepadAxis, GamepadButton, GamepadId, InputBackend, InputEvent, KeyCode, MouseButton,
};
use rustc_hash::FxHashMap;
use std::collections::VecDeque;
use tracing::debug;

/// High-level input manager.
///
/// This manager sits on top of the platform-specific input backend and provides:
/// - Simple state queries (is_key_pressed, was_key_just_pressed, etc.)
/// - Input buffering for frame-aligned updates
/// - Key binding support (future)
/// - Input history (future)
pub struct InputManager {
    backend: Box<dyn InputBackend>,

    // Previous frame state for "just pressed/released" detection
    prev_keys: FxHashMap<KeyCode, bool>,
    prev_mouse_buttons: FxHashMap<MouseButton, bool>,
    prev_gamepad_buttons: FxHashMap<(GamepadId, GamepadButton), bool>,

    // Event buffer for this frame
    frame_events: VecDeque<InputEvent>,

    // Mouse delta accumulator
    accumulated_mouse_delta: (f32, f32),
}

impl InputManager {
    /// Create a new input manager with the given backend.
    pub fn new(backend: Box<dyn InputBackend>) -> Self {
        debug!("Creating input manager");

        Self {
            backend,
            prev_keys: FxHashMap::default(),
            prev_mouse_buttons: FxHashMap::default(),
            prev_gamepad_buttons: FxHashMap::default(),
            frame_events: VecDeque::new(),
            accumulated_mouse_delta: (0.0, 0.0),
        }
    }

    /// Update the input manager for a new frame.
    ///
    /// This should be called once at the beginning of each frame to:
    /// 1. Poll new events from the backend
    /// 2. Update previous frame state
    /// 3. Clear per-frame accumulators
    pub fn update(&mut self) {
        // Save current state as previous state
        self.prev_keys.clear();
        self.prev_mouse_buttons.clear();
        self.prev_gamepad_buttons.clear();

        // Poll new events
        let events = self.backend.poll_events();
        self.frame_events.clear();
        self.accumulated_mouse_delta = (0.0, 0.0);

        for event in events {
            // Accumulate mouse delta
            if let InputEvent::MouseMotion { delta_x, delta_y } = event {
                self.accumulated_mouse_delta.0 += delta_x;
                self.accumulated_mouse_delta.1 += delta_y;
            }

            // Store event
            self.frame_events.push_back(event);
        }
    }

    // ========================================================================
    // Keyboard Input
    // ========================================================================

    /// Check if a key is currently pressed.
    pub fn is_key_down(&self, key: KeyCode) -> bool {
        self.backend.is_key_down(key)
    }

    /// Check if a key was just pressed this frame.
    pub fn is_key_just_pressed(&self, key: KeyCode) -> bool {
        let currently_down = self.backend.is_key_down(key);
        let was_down = self.prev_keys.get(&key).copied().unwrap_or(false);
        currently_down && !was_down
    }

    /// Check if a key was just released this frame.
    pub fn is_key_just_released(&self, key: KeyCode) -> bool {
        let currently_down = self.backend.is_key_down(key);
        let was_down = self.prev_keys.get(&key).copied().unwrap_or(false);
        !currently_down && was_down
    }

    // ========================================================================
    // Mouse Input
    // ========================================================================

    /// Check if a mouse button is currently pressed.
    pub fn is_mouse_button_down(&self, button: MouseButton) -> bool {
        self.backend.is_mouse_button_down(button)
    }

    /// Check if a mouse button was just pressed this frame.
    pub fn is_mouse_button_just_pressed(&self, button: MouseButton) -> bool {
        let currently_down = self.backend.is_mouse_button_down(button);
        let was_down = self.prev_mouse_buttons.get(&button).copied().unwrap_or(false);
        currently_down && !was_down
    }

    /// Check if a mouse button was just released this frame.
    pub fn is_mouse_button_just_released(&self, button: MouseButton) -> bool {
        let currently_down = self.backend.is_mouse_button_down(button);
        let was_down = self.prev_mouse_buttons.get(&button).copied().unwrap_or(false);
        !currently_down && was_down
    }

    /// Get the current mouse position.
    pub fn mouse_position(&self) -> (f32, f32) {
        self.backend.mouse_position()
    }

    /// Get the accumulated mouse delta for this frame.
    pub fn mouse_delta(&self) -> (f32, f32) {
        self.accumulated_mouse_delta
    }

    // ========================================================================
    // Gamepad Input
    // ========================================================================

    /// Get the number of connected gamepads.
    pub fn gamepad_count(&self) -> usize {
        self.backend.gamepad_count()
    }

    /// Check if a gamepad button is currently pressed.
    pub fn is_gamepad_button_down(&self, id: GamepadId, button: GamepadButton) -> bool {
        self.backend.is_gamepad_button_down(id, button)
    }

    /// Check if a gamepad button was just pressed this frame.
    pub fn is_gamepad_button_just_pressed(&self, id: GamepadId, button: GamepadButton) -> bool {
        let currently_down = self.backend.is_gamepad_button_down(id, button);
        let was_down = self.prev_gamepad_buttons.get(&(id, button)).copied().unwrap_or(false);
        currently_down && !was_down
    }

    /// Check if a gamepad button was just released this frame.
    pub fn is_gamepad_button_just_released(&self, id: GamepadId, button: GamepadButton) -> bool {
        let currently_down = self.backend.is_gamepad_button_down(id, button);
        let was_down = self.prev_gamepad_buttons.get(&(id, button)).copied().unwrap_or(false);
        !currently_down && was_down
    }

    /// Get the current value of a gamepad axis.
    pub fn gamepad_axis(&self, id: GamepadId, axis: GamepadAxis) -> f32 {
        self.backend.gamepad_axis(id, axis)
    }

    // ========================================================================
    // Event Access
    // ========================================================================

    /// Get all input events for the current frame.
    ///
    /// Useful for UI systems that need to process raw events.
    pub fn events(&self) -> &VecDeque<InputEvent> {
        &self.frame_events
    }

    /// Get a mutable reference to the underlying backend.
    ///
    /// This is useful for integration with windowing systems that need to feed events.
    pub fn backend_mut(&mut self) -> &mut dyn InputBackend {
        &mut *self.backend
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::input::create_input_backend;

    #[test]
    fn test_input_manager_creation() {
        let backend = create_input_backend().unwrap();
        let manager = InputManager::new(backend);

        assert_eq!(manager.gamepad_count(), 0);
        assert_eq!(manager.mouse_position(), (0.0, 0.0));
    }

    #[test]
    fn test_key_state_queries() {
        let backend = create_input_backend().unwrap();
        let manager = InputManager::new(backend);

        // Initially, no keys are pressed
        assert!(!manager.is_key_down(KeyCode::A));
        assert!(!manager.is_key_just_pressed(KeyCode::A));
        assert!(!manager.is_key_just_released(KeyCode::A));
    }

    #[test]
    fn test_mouse_state_queries() {
        let backend = create_input_backend().unwrap();
        let manager = InputManager::new(backend);

        // Initially, no buttons are pressed
        assert!(!manager.is_mouse_button_down(MouseButton::Left));
        assert!(!manager.is_mouse_button_just_pressed(MouseButton::Left));
        assert!(!manager.is_mouse_button_just_released(MouseButton::Left));
    }

    #[test]
    fn test_gamepad_state_queries() {
        let backend = create_input_backend().unwrap();
        let manager = InputManager::new(backend);
        let gamepad_id = GamepadId::new(0);

        // Initially, no buttons are pressed
        assert!(!manager.is_gamepad_button_down(gamepad_id, GamepadButton::South));
        assert!(!manager.is_gamepad_button_just_pressed(gamepad_id, GamepadButton::South));
        assert_eq!(manager.gamepad_axis(gamepad_id, GamepadAxis::LeftStickX), 0.0);
    }

    #[test]
    fn test_update_clears_events() {
        let backend = create_input_backend().unwrap();
        let mut manager = InputManager::new(backend);

        // First update
        manager.update();
        assert_eq!(manager.events().len(), 0);

        // Second update (should clear previous events)
        manager.update();
        assert_eq!(manager.events().len(), 0);
    }
}
