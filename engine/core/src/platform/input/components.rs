//! ECS components for input state.
//!
//! These components can be attached to entities to make them respond to input.
//! They integrate with the platform input system and ECS for game logic.

use super::{GamepadAxis, GamepadButton, GamepadId, KeyCode, MouseButton};
use rustc_hash::FxHashMap;

/// Component that stores the current input state for an entity.
///
/// This component can be attached to a player entity to track which
/// keys/buttons are currently pressed. It's updated by the input system.
///
/// # Example
///
/// ```
/// use engine_core::ecs::World;
/// use engine_core::platform::input::InputState;
///
/// let mut world = World::new();
/// let player = world.spawn();
/// world.add_component(player, InputState::default()).unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct InputState {
    /// Currently pressed keyboard keys.
    pub pressed_keys: FxHashMap<KeyCode, bool>,

    /// Currently pressed mouse buttons.
    pub pressed_mouse_buttons: FxHashMap<MouseButton, bool>,

    /// Current mouse position in window coordinates.
    pub mouse_position: (f32, f32),

    /// Mouse delta since last frame.
    pub mouse_delta: (f32, f32),

    /// Currently connected gamepad (if any).
    pub gamepad_id: Option<GamepadId>,

    /// Currently pressed gamepad buttons.
    pub pressed_gamepad_buttons: FxHashMap<GamepadButton, bool>,

    /// Current gamepad axis values.
    pub gamepad_axes: FxHashMap<GamepadAxis, f32>,
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            pressed_keys: FxHashMap::default(),
            pressed_mouse_buttons: FxHashMap::default(),
            mouse_position: (0.0, 0.0),
            mouse_delta: (0.0, 0.0),
            gamepad_id: None,
            pressed_gamepad_buttons: FxHashMap::default(),
            gamepad_axes: FxHashMap::default(),
        }
    }
}

impl InputState {
    /// Create a new empty input state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if a key is currently pressed.
    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.pressed_keys.get(&key).copied().unwrap_or(false)
    }

    /// Check if a mouse button is currently pressed.
    pub fn is_mouse_button_pressed(&self, button: MouseButton) -> bool {
        self.pressed_mouse_buttons.get(&button).copied().unwrap_or(false)
    }

    /// Check if a gamepad button is currently pressed.
    pub fn is_gamepad_button_pressed(&self, button: GamepadButton) -> bool {
        self.pressed_gamepad_buttons.get(&button).copied().unwrap_or(false)
    }

    /// Get the current value of a gamepad axis.
    pub fn gamepad_axis(&self, axis: GamepadAxis) -> f32 {
        self.gamepad_axes.get(&axis).copied().unwrap_or(0.0)
    }

    /// Set whether a key is pressed.
    pub fn set_key_pressed(&mut self, key: KeyCode, pressed: bool) {
        self.pressed_keys.insert(key, pressed);
    }

    /// Set whether a mouse button is pressed.
    pub fn set_mouse_button_pressed(&mut self, button: MouseButton, pressed: bool) {
        self.pressed_mouse_buttons.insert(button, pressed);
    }

    /// Set the mouse position.
    pub fn set_mouse_position(&mut self, x: f32, y: f32) {
        self.mouse_position = (x, y);
    }

    /// Set the mouse delta.
    pub fn set_mouse_delta(&mut self, dx: f32, dy: f32) {
        self.mouse_delta = (dx, dy);
    }

    /// Set the connected gamepad ID.
    pub fn set_gamepad(&mut self, gamepad_id: Option<GamepadId>) {
        self.gamepad_id = gamepad_id;
    }

    /// Set whether a gamepad button is pressed.
    pub fn set_gamepad_button_pressed(&mut self, button: GamepadButton, pressed: bool) {
        self.pressed_gamepad_buttons.insert(button, pressed);
    }

    /// Set a gamepad axis value.
    pub fn set_gamepad_axis(&mut self, axis: GamepadAxis, value: f32) {
        self.gamepad_axes.insert(axis, value);
    }

    /// Clear all input state (useful for reset).
    pub fn clear(&mut self) {
        self.pressed_keys.clear();
        self.pressed_mouse_buttons.clear();
        self.mouse_position = (0.0, 0.0);
        self.mouse_delta = (0.0, 0.0);
        self.gamepad_id = None;
        self.pressed_gamepad_buttons.clear();
        self.gamepad_axes.clear();
    }
}

/// Configuration for input action bindings.
///
/// This component maps input events to named actions, allowing for
/// rebindable controls and platform-independent input handling.
///
/// # Example
///
/// ```
/// use engine_core::platform::input::{InputActions, KeyCode};
///
/// let mut actions = InputActions::new();
/// actions.bind_key("jump", KeyCode::Space);
/// actions.bind_key("move_forward", KeyCode::W);
/// ```
#[derive(Debug, Clone)]
pub struct InputActions {
    /// Map from action name to keyboard keys that trigger it.
    pub key_bindings: FxHashMap<String, Vec<KeyCode>>,

    /// Map from action name to mouse buttons that trigger it.
    pub mouse_bindings: FxHashMap<String, Vec<MouseButton>>,

    /// Map from action name to gamepad buttons that trigger it.
    pub gamepad_bindings: FxHashMap<String, Vec<GamepadButton>>,

    /// Map from action name to gamepad axes that control it.
    pub axis_bindings: FxHashMap<String, Vec<GamepadAxis>>,

    /// Dead zone for analog inputs (values below this are treated as 0).
    pub dead_zone: f32,
}

impl Default for InputActions {
    fn default() -> Self {
        Self {
            key_bindings: FxHashMap::default(),
            mouse_bindings: FxHashMap::default(),
            gamepad_bindings: FxHashMap::default(),
            axis_bindings: FxHashMap::default(),
            dead_zone: 0.1, // 10% dead zone by default
        }
    }
}

impl InputActions {
    /// Create a new empty input action map.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new input action map with a custom dead zone.
    pub fn with_dead_zone(dead_zone: f32) -> Self {
        Self { dead_zone, ..Default::default() }
    }

    /// Bind a key to an action.
    pub fn bind_key(&mut self, action: impl Into<String>, key: KeyCode) {
        self.key_bindings.entry(action.into()).or_default().push(key);
    }

    /// Bind a mouse button to an action.
    pub fn bind_mouse_button(&mut self, action: impl Into<String>, button: MouseButton) {
        self.mouse_bindings.entry(action.into()).or_default().push(button);
    }

    /// Bind a gamepad button to an action.
    pub fn bind_gamepad_button(&mut self, action: impl Into<String>, button: GamepadButton) {
        self.gamepad_bindings.entry(action.into()).or_default().push(button);
    }

    /// Bind a gamepad axis to an action.
    pub fn bind_axis(&mut self, action: impl Into<String>, axis: GamepadAxis) {
        self.axis_bindings.entry(action.into()).or_default().push(axis);
    }

    /// Check if an action is currently active based on the input state.
    pub fn is_action_active(&self, action: &str, state: &InputState) -> bool {
        // Check keyboard bindings
        if let Some(keys) = self.key_bindings.get(action) {
            if keys.iter().any(|key| state.is_key_pressed(*key)) {
                return true;
            }
        }

        // Check mouse button bindings
        if let Some(buttons) = self.mouse_bindings.get(action) {
            if buttons.iter().any(|button| state.is_mouse_button_pressed(*button)) {
                return true;
            }
        }

        // Check gamepad button bindings
        if let Some(buttons) = self.gamepad_bindings.get(action) {
            if buttons.iter().any(|button| state.is_gamepad_button_pressed(*button)) {
                return true;
            }
        }

        false
    }

    /// Get the analog value for an action (0.0 to 1.0 or -1.0 to 1.0).
    ///
    /// For digital inputs (keys/buttons), returns 1.0 if pressed, 0.0 otherwise.
    /// For analog inputs (axes), returns the axis value with dead zone applied.
    pub fn get_action_value(&self, action: &str, state: &InputState) -> f32 {
        // Check digital inputs first
        if self.is_action_active(action, state) {
            return 1.0;
        }

        // Check analog inputs (axes)
        if let Some(axes) = self.axis_bindings.get(action) {
            for axis in axes {
                let value = state.gamepad_axis(*axis);
                let abs_value = value.abs();

                // Apply dead zone
                if abs_value > self.dead_zone {
                    // Remap value to account for dead zone
                    let sign = value.signum();
                    let remapped = (abs_value - self.dead_zone) / (1.0 - self.dead_zone);
                    return sign * remapped;
                }
            }
        }

        0.0
    }

    /// Apply dead zone to a raw axis value.
    pub fn apply_dead_zone(&self, value: f32) -> f32 {
        let abs_value = value.abs();

        if abs_value <= self.dead_zone {
            0.0
        } else {
            // Remap value to account for dead zone
            let sign = value.signum();
            let remapped = (abs_value - self.dead_zone) / (1.0 - self.dead_zone);
            sign * remapped
        }
    }

    /// Remove all bindings for an action.
    pub fn unbind_action(&mut self, action: &str) {
        self.key_bindings.remove(action);
        self.mouse_bindings.remove(action);
        self.gamepad_bindings.remove(action);
        self.axis_bindings.remove(action);
    }

    /// Clear all action bindings.
    pub fn clear(&mut self) {
        self.key_bindings.clear();
        self.mouse_bindings.clear();
        self.gamepad_bindings.clear();
        self.axis_bindings.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_state_default() {
        let state = InputState::default();
        assert!(!state.is_key_pressed(KeyCode::A));
        assert!(!state.is_mouse_button_pressed(MouseButton::Left));
        assert_eq!(state.mouse_position, (0.0, 0.0));
        assert_eq!(state.mouse_delta, (0.0, 0.0));
        assert!(state.gamepad_id.is_none());
    }

    #[test]
    fn test_input_state_key_press() {
        let mut state = InputState::new();
        assert!(!state.is_key_pressed(KeyCode::A));

        state.set_key_pressed(KeyCode::A, true);
        assert!(state.is_key_pressed(KeyCode::A));

        state.set_key_pressed(KeyCode::A, false);
        assert!(!state.is_key_pressed(KeyCode::A));
    }

    #[test]
    fn test_input_state_mouse() {
        let mut state = InputState::new();

        state.set_mouse_position(100.0, 200.0);
        assert_eq!(state.mouse_position, (100.0, 200.0));

        state.set_mouse_delta(10.0, 20.0);
        assert_eq!(state.mouse_delta, (10.0, 20.0));

        state.set_mouse_button_pressed(MouseButton::Left, true);
        assert!(state.is_mouse_button_pressed(MouseButton::Left));
    }

    #[test]
    fn test_input_state_gamepad() {
        let mut state = InputState::new();
        let gamepad_id = GamepadId::new(0);

        state.set_gamepad(Some(gamepad_id));
        assert_eq!(state.gamepad_id, Some(gamepad_id));

        state.set_gamepad_button_pressed(GamepadButton::South, true);
        assert!(state.is_gamepad_button_pressed(GamepadButton::South));

        state.set_gamepad_axis(GamepadAxis::LeftStickX, 0.5);
        assert_eq!(state.gamepad_axis(GamepadAxis::LeftStickX), 0.5);
    }

    #[test]
    fn test_input_state_clear() {
        let mut state = InputState::new();
        state.set_key_pressed(KeyCode::A, true);
        state.set_mouse_position(100.0, 200.0);

        state.clear();

        assert!(!state.is_key_pressed(KeyCode::A));
        assert_eq!(state.mouse_position, (0.0, 0.0));
    }

    #[test]
    fn test_input_actions_default() {
        let actions = InputActions::default();
        assert_eq!(actions.dead_zone, 0.1);
    }

    #[test]
    fn test_input_actions_key_binding() {
        let mut actions = InputActions::new();
        let mut state = InputState::new();

        actions.bind_key("jump", KeyCode::Space);

        // Not active initially
        assert!(!actions.is_action_active("jump", &state));

        // Active when key is pressed
        state.set_key_pressed(KeyCode::Space, true);
        assert!(actions.is_action_active("jump", &state));
    }

    #[test]
    fn test_input_actions_multiple_bindings() {
        let mut actions = InputActions::new();
        let mut state = InputState::new();

        // Bind multiple keys to same action
        actions.bind_key("jump", KeyCode::Space);
        actions.bind_key("jump", KeyCode::W);

        // Either key should activate
        state.set_key_pressed(KeyCode::Space, true);
        assert!(actions.is_action_active("jump", &state));

        state.set_key_pressed(KeyCode::Space, false);
        state.set_key_pressed(KeyCode::W, true);
        assert!(actions.is_action_active("jump", &state));
    }

    #[test]
    fn test_input_actions_mouse_binding() {
        let mut actions = InputActions::new();
        let mut state = InputState::new();

        actions.bind_mouse_button("fire", MouseButton::Left);

        state.set_mouse_button_pressed(MouseButton::Left, true);
        assert!(actions.is_action_active("fire", &state));
    }

    #[test]
    fn test_input_actions_gamepad_binding() {
        let mut actions = InputActions::new();
        let mut state = InputState::new();

        actions.bind_gamepad_button("jump", GamepadButton::South);

        state.set_gamepad_button_pressed(GamepadButton::South, true);
        assert!(actions.is_action_active("jump", &state));
    }

    #[test]
    fn test_input_actions_axis_value() {
        let mut actions = InputActions::new();
        let mut state = InputState::new();

        actions.bind_axis("move_horizontal", GamepadAxis::LeftStickX);

        // Below dead zone (0.1)
        state.set_gamepad_axis(GamepadAxis::LeftStickX, 0.05);
        assert_eq!(actions.get_action_value("move_horizontal", &state), 0.0);

        // Above dead zone - should be remapped
        state.set_gamepad_axis(GamepadAxis::LeftStickX, 0.5);
        let value = actions.get_action_value("move_horizontal", &state);
        assert!(value > 0.0);
        assert!(value < 1.0);

        // Full value
        state.set_gamepad_axis(GamepadAxis::LeftStickX, 1.0);
        assert_eq!(actions.get_action_value("move_horizontal", &state), 1.0);
    }

    #[test]
    fn test_dead_zone_application() {
        let actions = InputActions::with_dead_zone(0.2);

        // Below dead zone
        assert_eq!(actions.apply_dead_zone(0.1), 0.0);
        assert_eq!(actions.apply_dead_zone(-0.1), 0.0);

        // At dead zone boundary
        assert_eq!(actions.apply_dead_zone(0.2), 0.0);

        // Above dead zone
        let value = actions.apply_dead_zone(0.6);
        assert!(value > 0.0);
        assert!(value < 1.0);

        // Full value
        assert_eq!(actions.apply_dead_zone(1.0), 1.0);
        assert_eq!(actions.apply_dead_zone(-1.0), -1.0);
    }

    #[test]
    fn test_unbind_action() {
        let mut actions = InputActions::new();

        actions.bind_key("jump", KeyCode::Space);
        actions.bind_mouse_button("jump", MouseButton::Left);

        let mut state = InputState::new();
        state.set_key_pressed(KeyCode::Space, true);
        assert!(actions.is_action_active("jump", &state));

        actions.unbind_action("jump");
        assert!(!actions.is_action_active("jump", &state));
    }

    #[test]
    fn test_clear_actions() {
        let mut actions = InputActions::new();

        actions.bind_key("jump", KeyCode::Space);
        actions.bind_key("fire", KeyCode::F);

        actions.clear();

        let mut state = InputState::new();
        state.set_key_pressed(KeyCode::Space, true);
        state.set_key_pressed(KeyCode::F, true);

        assert!(!actions.is_action_active("jump", &state));
        assert!(!actions.is_action_active("fire", &state));
    }
}
