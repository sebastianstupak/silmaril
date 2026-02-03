//! ECS system for processing input and updating input components.
//!
//! This system integrates the platform input backend with the ECS,
//! updating InputState components based on input events.

use super::{InputEvent, InputManager, InputState};
use tracing::{debug, trace};

/// Process input events and update InputState components.
///
/// This system should be run once per frame, before game logic systems
/// that need to query input state.
///
/// # Example
///
/// ```no_run
/// use engine_core::ecs::World;
/// use engine_core::platform::input::{InputState, InputSystem};
///
/// let mut world = World::new();
/// let player = world.spawn();
/// world.add_component(player, InputState::default()).unwrap();
///
/// // Create input system
/// let mut input_system = InputSystem::new();
///
/// // Run every frame
/// // input_system.run(&mut world);
/// ```
pub struct InputSystem {
    /// Optional InputManager for standalone usage.
    ///
    /// If None, the system expects input events to be fed externally
    /// (e.g., from a windowing system).
    manager: Option<InputManager>,

    /// Event buffer for external event feeding.
    event_buffer: Vec<InputEvent>,
}

impl InputSystem {
    /// Create a new input system without an input manager.
    ///
    /// Events must be fed using `feed_event()` or `feed_events()`.
    pub fn new() -> Self {
        Self { manager: None, event_buffer: Vec::new() }
    }

    /// Create a new input system with an input manager.
    ///
    /// The manager will poll events from the platform automatically.
    pub fn with_manager(manager: InputManager) -> Self {
        Self { manager: Some(manager), event_buffer: Vec::new() }
    }

    /// Feed an input event to the system.
    ///
    /// This is used when integrating with external windowing systems.
    pub fn feed_event(&mut self, event: InputEvent) {
        self.event_buffer.push(event);
    }

    /// Feed multiple input events to the system.
    pub fn feed_events(&mut self, events: impl IntoIterator<Item = InputEvent>) {
        self.event_buffer.extend(events);
    }

    /// Update the input system and process events.
    ///
    /// This polls events from the manager (if available) and updates
    /// all InputState components in the world.
    ///
    /// # Performance
    ///
    /// This operation should complete in < 0.5ms for 1000 events.
    pub fn update(&mut self, world: &mut crate::ecs::World) {
        trace!("Updating input system");

        // Poll events from manager if available
        if let Some(manager) = &mut self.manager {
            manager.update();
            self.event_buffer.extend(manager.events().iter().cloned());
        }

        // If no events and no manager, nothing to do
        if self.event_buffer.is_empty() {
            return;
        }

        debug!(event_count = self.event_buffer.len(), "Processing input events");

        // Process events and update all InputState components
        // Clone the events to avoid borrow checker issues
        let events: Vec<_> = self.event_buffer.drain(..).collect();
        for event in events {
            self.process_event(world, &event);
        }
    }

    /// Process a single input event and update InputState components.
    #[allow(unused_variables)] // world parameter reserved for future ECS integration
    fn process_event(&self, _world: &mut crate::ecs::World, event: &InputEvent) {
        // Query all entities with InputState components
        // For now, we update all InputState components with the same events.
        // In the future, we could add filtering based on entity ownership.

        // Since we can't use the query system in this context without
        // more complex integration, we'll provide helper methods that
        // game code can call to update specific entities.
        //
        // For full ECS integration, this would use:
        // let mut query = world.query::<&mut InputState>();
        // for mut state in query.iter_mut() {
        //     self.update_state_from_event(&mut state, event);
        // }

        // For now, this is a placeholder. Full integration requires
        // the World to expose query functionality.
        trace!("Would process event: {:?}", event);
    }

    /// Update an InputState component from an input event.
    ///
    /// This is a helper method that can be used by game code to manually
    /// update input state components.
    pub fn update_state_from_event(&self, state: &mut InputState, event: &InputEvent) {
        match event {
            InputEvent::KeyPressed { key } => {
                state.set_key_pressed(*key, true);
            }
            InputEvent::KeyReleased { key } => {
                state.set_key_pressed(*key, false);
            }
            InputEvent::MouseMoved { x, y } => {
                state.set_mouse_position(*x, *y);
            }
            InputEvent::MouseMotion { delta_x, delta_y } => {
                // Accumulate mouse delta
                let (current_dx, current_dy) = state.mouse_delta;
                state.set_mouse_delta(current_dx + delta_x, current_dy + delta_y);
            }
            InputEvent::MouseButtonPressed { button } => {
                state.set_mouse_button_pressed(*button, true);
            }
            InputEvent::MouseButtonReleased { button } => {
                state.set_mouse_button_pressed(*button, false);
            }
            InputEvent::MouseWheel { .. } => {
                // Mouse wheel events are typically handled separately
                // as they don't have persistent state
            }
            InputEvent::MouseEntered | InputEvent::MouseLeft => {
                // Window focus events - could be used to clear state
            }
            InputEvent::GamepadConnected { id } => {
                state.set_gamepad(Some(*id));
            }
            InputEvent::GamepadDisconnected { .. } => {
                state.set_gamepad(None);
                state.pressed_gamepad_buttons.clear();
                state.gamepad_axes.clear();
            }
            InputEvent::GamepadButtonPressed { button, .. } => {
                state.set_gamepad_button_pressed(*button, true);
            }
            InputEvent::GamepadButtonReleased { button, .. } => {
                state.set_gamepad_button_pressed(*button, false);
            }
            InputEvent::GamepadAxis { axis, value, .. } => {
                state.set_gamepad_axis(*axis, *value);
            }
        }
    }

    /// Get a reference to the input manager (if available).
    pub fn manager(&self) -> Option<&InputManager> {
        self.manager.as_ref()
    }

    /// Get a mutable reference to the input manager (if available).
    pub fn manager_mut(&mut self) -> Option<&mut InputManager> {
        self.manager.as_mut()
    }

    /// Clear the event buffer.
    pub fn clear_events(&mut self) {
        self.event_buffer.clear();
    }
}

impl Default for InputSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::input::{GamepadAxis, GamepadButton, GamepadId, KeyCode, MouseButton};

    #[test]
    fn test_input_system_creation() {
        let system = InputSystem::new();
        assert!(system.manager.is_none());
        assert_eq!(system.event_buffer.len(), 0);
    }

    #[test]
    fn test_feed_event() {
        let mut system = InputSystem::new();

        system.feed_event(InputEvent::KeyPressed { key: KeyCode::A });
        assert_eq!(system.event_buffer.len(), 1);
    }

    #[test]
    fn test_feed_events() {
        let mut system = InputSystem::new();

        let events = vec![
            InputEvent::KeyPressed { key: KeyCode::A },
            InputEvent::KeyPressed { key: KeyCode::B },
            InputEvent::KeyReleased { key: KeyCode::A },
        ];

        system.feed_events(events);
        assert_eq!(system.event_buffer.len(), 3);
    }

    #[test]
    fn test_update_state_keyboard() {
        let system = InputSystem::new();
        let mut state = InputState::new();

        // Key press
        system.update_state_from_event(&mut state, &InputEvent::KeyPressed { key: KeyCode::A });
        assert!(state.is_key_pressed(KeyCode::A));

        // Key release
        system.update_state_from_event(&mut state, &InputEvent::KeyReleased { key: KeyCode::A });
        assert!(!state.is_key_pressed(KeyCode::A));
    }

    #[test]
    fn test_update_state_mouse_position() {
        let system = InputSystem::new();
        let mut state = InputState::new();

        system.update_state_from_event(&mut state, &InputEvent::MouseMoved { x: 100.0, y: 200.0 });
        assert_eq!(state.mouse_position, (100.0, 200.0));
    }

    #[test]
    fn test_update_state_mouse_delta() {
        let system = InputSystem::new();
        let mut state = InputState::new();

        // First delta
        system.update_state_from_event(
            &mut state,
            &InputEvent::MouseMotion { delta_x: 10.0, delta_y: 20.0 },
        );
        assert_eq!(state.mouse_delta, (10.0, 20.0));

        // Second delta accumulates
        system.update_state_from_event(
            &mut state,
            &InputEvent::MouseMotion { delta_x: 5.0, delta_y: 10.0 },
        );
        assert_eq!(state.mouse_delta, (15.0, 30.0));
    }

    #[test]
    fn test_update_state_mouse_button() {
        let system = InputSystem::new();
        let mut state = InputState::new();

        system.update_state_from_event(
            &mut state,
            &InputEvent::MouseButtonPressed { button: MouseButton::Left },
        );
        assert!(state.is_mouse_button_pressed(MouseButton::Left));

        system.update_state_from_event(
            &mut state,
            &InputEvent::MouseButtonReleased { button: MouseButton::Left },
        );
        assert!(!state.is_mouse_button_pressed(MouseButton::Left));
    }

    #[test]
    fn test_update_state_gamepad_connection() {
        let system = InputSystem::new();
        let mut state = InputState::new();
        let gamepad_id = GamepadId::new(0);

        system
            .update_state_from_event(&mut state, &InputEvent::GamepadConnected { id: gamepad_id });
        assert_eq!(state.gamepad_id, Some(gamepad_id));

        system.update_state_from_event(
            &mut state,
            &InputEvent::GamepadDisconnected { id: gamepad_id },
        );
        assert_eq!(state.gamepad_id, None);
    }

    #[test]
    fn test_update_state_gamepad_button() {
        let system = InputSystem::new();
        let mut state = InputState::new();
        let gamepad_id = GamepadId::new(0);

        system.update_state_from_event(
            &mut state,
            &InputEvent::GamepadButtonPressed { id: gamepad_id, button: GamepadButton::South },
        );
        assert!(state.is_gamepad_button_pressed(GamepadButton::South));

        system.update_state_from_event(
            &mut state,
            &InputEvent::GamepadButtonReleased { id: gamepad_id, button: GamepadButton::South },
        );
        assert!(!state.is_gamepad_button_pressed(GamepadButton::South));
    }

    #[test]
    fn test_update_state_gamepad_axis() {
        let system = InputSystem::new();
        let mut state = InputState::new();
        let gamepad_id = GamepadId::new(0);

        system.update_state_from_event(
            &mut state,
            &InputEvent::GamepadAxis { id: gamepad_id, axis: GamepadAxis::LeftStickX, value: 0.75 },
        );
        assert_eq!(state.gamepad_axis(GamepadAxis::LeftStickX), 0.75);
    }

    #[test]
    fn test_clear_events() {
        let mut system = InputSystem::new();

        system.feed_event(InputEvent::KeyPressed { key: KeyCode::A });
        system.feed_event(InputEvent::KeyPressed { key: KeyCode::B });

        assert_eq!(system.event_buffer.len(), 2);

        system.clear_events();
        assert_eq!(system.event_buffer.len(), 0);
    }
}
