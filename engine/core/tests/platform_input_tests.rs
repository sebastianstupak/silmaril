//! Integration tests for the platform input abstraction layer.
//!
//! These tests verify that the input system works correctly across all platforms,
//! including keyboard, mouse, and gamepad input handling.

use engine_core::platform::{
    create_input_backend, GamepadAxis, GamepadButton, GamepadId, InputBackend, InputEvent,
    InputManager, KeyCode, MouseButton,
};

// ============================================================================
// Input Backend Tests
// ============================================================================

#[test]
fn test_input_backend_creation() {
    // Should create backend for current platform
    let result = create_input_backend();
    assert!(result.is_ok(), "Failed to create input backend");
}

#[test]
fn test_input_backend_initial_state() {
    let backend = create_input_backend().unwrap();

    // Initially, no keys should be pressed
    assert!(!backend.is_key_down(KeyCode::A));
    assert!(!backend.is_key_down(KeyCode::Space));
    assert!(!backend.is_key_down(KeyCode::Enter));

    // Initially, no mouse buttons should be pressed
    assert!(!backend.is_mouse_button_down(MouseButton::Left));
    assert!(!backend.is_mouse_button_down(MouseButton::Right));

    // Initially, mouse position should be (0, 0) or unknown
    let (x, y) = backend.mouse_position();
    assert!(x >= 0.0 && y >= 0.0);

    // Initially, no gamepads should be connected
    assert_eq!(backend.gamepad_count(), 0);
}

// ============================================================================
// Keyboard Input Tests
// ============================================================================

#[test]
fn test_keyboard_input_state() {
    #[cfg(target_os = "windows")]
    {
        use engine_core::platform::input::backend::windows::WindowsInput;

        let mut backend = WindowsInput::new().unwrap();

        // Initially, key A is not pressed
        assert!(!backend.is_key_down(KeyCode::A));

        // Simulate key press
        backend.feed_event(InputEvent::KeyPressed { key: KeyCode::A });
        assert!(backend.is_key_down(KeyCode::A));

        // Simulate key release
        backend.feed_event(InputEvent::KeyReleased { key: KeyCode::A });
        assert!(!backend.is_key_down(KeyCode::A));
    }

    #[cfg(target_os = "linux")]
    {
        use engine_core::platform::input::backend::linux::LinuxInput;

        let mut backend = LinuxInput::new().unwrap();

        assert!(!backend.is_key_down(KeyCode::A));
        backend.feed_event(InputEvent::KeyPressed { key: KeyCode::A });
        assert!(backend.is_key_down(KeyCode::A));
        backend.feed_event(InputEvent::KeyReleased { key: KeyCode::A });
        assert!(!backend.is_key_down(KeyCode::A));
    }

    #[cfg(target_os = "macos")]
    {
        use engine_core::platform::input::backend::macos::MacOSInput;

        let mut backend = MacOSInput::new().unwrap();

        assert!(!backend.is_key_down(KeyCode::A));
        backend.feed_event(InputEvent::KeyPressed { key: KeyCode::A });
        assert!(backend.is_key_down(KeyCode::A));
        backend.feed_event(InputEvent::KeyReleased { key: KeyCode::A });
        assert!(!backend.is_key_down(KeyCode::A));
    }
}

#[test]
fn test_multiple_key_press() {
    #[cfg(target_os = "windows")]
    {
        use engine_core::platform::input::backend::windows::WindowsInput;

        let mut backend = WindowsInput::new().unwrap();

        // Press multiple keys
        backend.feed_event(InputEvent::KeyPressed { key: KeyCode::W });
        backend.feed_event(InputEvent::KeyPressed { key: KeyCode::A });
        backend.feed_event(InputEvent::KeyPressed { key: KeyCode::Space });

        // All keys should be pressed
        assert!(backend.is_key_down(KeyCode::W));
        assert!(backend.is_key_down(KeyCode::A));
        assert!(backend.is_key_down(KeyCode::Space));

        // Other keys should not be pressed
        assert!(!backend.is_key_down(KeyCode::S));
        assert!(!backend.is_key_down(KeyCode::D));

        // Release one key
        backend.feed_event(InputEvent::KeyReleased { key: KeyCode::A });

        // W and Space should still be pressed
        assert!(backend.is_key_down(KeyCode::W));
        assert!(!backend.is_key_down(KeyCode::A));
        assert!(backend.is_key_down(KeyCode::Space));
    }
}

#[test]
fn test_modifier_keys() {
    #[cfg(target_os = "windows")]
    {
        use engine_core::platform::input::backend::windows::WindowsInput;

        let mut backend = WindowsInput::new().unwrap();

        // Test modifier keys
        backend.feed_event(InputEvent::KeyPressed { key: KeyCode::LeftShift });
        backend.feed_event(InputEvent::KeyPressed { key: KeyCode::LeftControl });

        assert!(backend.is_key_down(KeyCode::LeftShift));
        assert!(backend.is_key_down(KeyCode::LeftControl));
        assert!(!backend.is_key_down(KeyCode::LeftAlt));

        // Verify helper methods
        assert!(KeyCode::LeftShift.is_modifier());
        assert!(KeyCode::LeftControl.is_modifier());
        assert!(!KeyCode::A.is_modifier());
    }
}

// ============================================================================
// Mouse Input Tests
// ============================================================================

#[test]
fn test_mouse_button_state() {
    #[cfg(target_os = "windows")]
    {
        use engine_core::platform::input::backend::windows::WindowsInput;

        let mut backend = WindowsInput::new().unwrap();

        // Initially, no buttons are pressed
        assert!(!backend.is_mouse_button_down(MouseButton::Left));

        // Simulate left button press
        backend.feed_event(InputEvent::MouseButtonPressed { button: MouseButton::Left });
        assert!(backend.is_mouse_button_down(MouseButton::Left));

        // Simulate left button release
        backend.feed_event(InputEvent::MouseButtonReleased { button: MouseButton::Left });
        assert!(!backend.is_mouse_button_down(MouseButton::Left));
    }
}

#[test]
fn test_mouse_position() {
    #[cfg(target_os = "windows")]
    {
        use engine_core::platform::input::backend::windows::WindowsInput;

        let mut backend = WindowsInput::new().unwrap();

        // Initially at origin
        assert_eq!(backend.mouse_position(), (0.0, 0.0));

        // Move mouse
        backend.feed_event(InputEvent::MouseMoved { x: 100.0, y: 200.0 });
        assert_eq!(backend.mouse_position(), (100.0, 200.0));

        // Move again
        backend.feed_event(InputEvent::MouseMoved { x: 150.0, y: 250.0 });
        assert_eq!(backend.mouse_position(), (150.0, 250.0));
    }
}

#[test]
fn test_mouse_delta() {
    #[cfg(target_os = "windows")]
    {
        use engine_core::platform::input::backend::windows::WindowsInput;

        let mut backend = WindowsInput::new().unwrap();

        // Simulate mouse motion
        backend.feed_event(InputEvent::MouseMotion { delta_x: 10.0, delta_y: 20.0 });
        assert_eq!(backend.mouse_delta(), (10.0, 20.0));
    }
}

#[test]
fn test_mouse_wheel() {
    #[cfg(target_os = "windows")]
    {
        use engine_core::platform::input::backend::windows::WindowsInput;

        let mut backend = WindowsInput::new().unwrap();

        // Feed mouse wheel event
        backend.feed_event(InputEvent::MouseWheel { delta_x: 0.0, delta_y: 1.0 });

        // Poll events to verify
        let events = backend.poll_events();
        assert_eq!(events.len(), 1);

        if let InputEvent::MouseWheel { delta_x, delta_y } = events[0] {
            assert_eq!(delta_x, 0.0);
            assert_eq!(delta_y, 1.0);
        } else {
            panic!("Expected MouseWheel event");
        }
    }
}

// ============================================================================
// Gamepad Input Tests
// ============================================================================

#[test]
fn test_gamepad_connection() {
    #[cfg(target_os = "windows")]
    {
        use engine_core::platform::input::backend::windows::WindowsInput;

        let mut backend = WindowsInput::new().unwrap();
        let gamepad_id = GamepadId::new(0);

        // Initially, no gamepads are connected
        assert_eq!(backend.gamepad_count(), 0);

        // Connect a gamepad
        backend.feed_event(InputEvent::GamepadConnected { id: gamepad_id });
        assert_eq!(backend.gamepad_count(), 1);

        // Disconnect the gamepad
        backend.feed_event(InputEvent::GamepadDisconnected { id: gamepad_id });
        assert_eq!(backend.gamepad_count(), 0);
    }
}

#[test]
fn test_gamepad_button_state() {
    #[cfg(target_os = "windows")]
    {
        use engine_core::platform::input::backend::windows::WindowsInput;

        let mut backend = WindowsInput::new().unwrap();
        let gamepad_id = GamepadId::new(0);

        // Connect gamepad
        backend.feed_event(InputEvent::GamepadConnected { id: gamepad_id });

        // Initially, no buttons are pressed
        assert!(!backend.is_gamepad_button_down(gamepad_id, GamepadButton::South));

        // Press button
        backend.feed_event(InputEvent::GamepadButtonPressed {
            id: gamepad_id,
            button: GamepadButton::South,
        });
        assert!(backend.is_gamepad_button_down(gamepad_id, GamepadButton::South));

        // Release button
        backend.feed_event(InputEvent::GamepadButtonReleased {
            id: gamepad_id,
            button: GamepadButton::South,
        });
        assert!(!backend.is_gamepad_button_down(gamepad_id, GamepadButton::South));
    }
}

#[test]
fn test_gamepad_axis() {
    #[cfg(target_os = "windows")]
    {
        use engine_core::platform::input::backend::windows::WindowsInput;

        let mut backend = WindowsInput::new().unwrap();
        let gamepad_id = GamepadId::new(0);

        // Connect gamepad
        backend.feed_event(InputEvent::GamepadConnected { id: gamepad_id });

        // Initially, all axes should be at 0.0
        assert_eq!(backend.gamepad_axis(gamepad_id, GamepadAxis::LeftStickX), 0.0);

        // Update axis
        backend.feed_event(InputEvent::GamepadAxis {
            id: gamepad_id,
            axis: GamepadAxis::LeftStickX,
            value: 0.75,
        });
        assert_eq!(backend.gamepad_axis(gamepad_id, GamepadAxis::LeftStickX), 0.75);

        // Update another axis
        backend.feed_event(InputEvent::GamepadAxis {
            id: gamepad_id,
            axis: GamepadAxis::RightStickY,
            value: -0.5,
        });
        assert_eq!(backend.gamepad_axis(gamepad_id, GamepadAxis::RightStickY), -0.5);
    }
}

#[test]
fn test_multiple_gamepads() {
    #[cfg(target_os = "windows")]
    {
        use engine_core::platform::input::backend::windows::WindowsInput;

        let mut backend = WindowsInput::new().unwrap();
        let gamepad1 = GamepadId::new(0);
        let gamepad2 = GamepadId::new(1);

        // Connect two gamepads
        backend.feed_event(InputEvent::GamepadConnected { id: gamepad1 });
        backend.feed_event(InputEvent::GamepadConnected { id: gamepad2 });
        assert_eq!(backend.gamepad_count(), 2);

        // Press button on first gamepad
        backend.feed_event(InputEvent::GamepadButtonPressed {
            id: gamepad1,
            button: GamepadButton::South,
        });

        // Only first gamepad should have button pressed
        assert!(backend.is_gamepad_button_down(gamepad1, GamepadButton::South));
        assert!(!backend.is_gamepad_button_down(gamepad2, GamepadButton::South));

        // Disconnect first gamepad
        backend.feed_event(InputEvent::GamepadDisconnected { id: gamepad1 });
        assert_eq!(backend.gamepad_count(), 1);
    }
}

// ============================================================================
// Input Manager Tests
// ============================================================================

#[test]
fn test_input_manager_creation() {
    let backend = create_input_backend().unwrap();
    let manager = InputManager::new(backend);

    // Verify initial state
    assert_eq!(manager.gamepad_count(), 0);
    assert_eq!(manager.mouse_position(), (0.0, 0.0));
    assert!(!manager.is_key_down(KeyCode::A));
}

#[test]
fn test_input_manager_update() {
    let backend = create_input_backend().unwrap();
    let mut manager = InputManager::new(backend);

    // First update should succeed
    manager.update();

    // Events should be empty initially
    assert_eq!(manager.events().len(), 0);

    // Second update should work
    manager.update();
}

#[test]
fn test_input_manager_key_queries() {
    let backend = create_input_backend().unwrap();
    let manager = InputManager::new(backend);

    // Test all query methods don't panic
    assert!(!manager.is_key_down(KeyCode::A));
    assert!(!manager.is_key_just_pressed(KeyCode::A));
    assert!(!manager.is_key_just_released(KeyCode::A));
}

#[test]
fn test_input_manager_mouse_queries() {
    let backend = create_input_backend().unwrap();
    let manager = InputManager::new(backend);

    // Test all query methods don't panic
    assert!(!manager.is_mouse_button_down(MouseButton::Left));
    assert!(!manager.is_mouse_button_just_pressed(MouseButton::Left));
    assert!(!manager.is_mouse_button_just_released(MouseButton::Left));
    assert_eq!(manager.mouse_position(), (0.0, 0.0));
    assert_eq!(manager.mouse_delta(), (0.0, 0.0));
}

#[test]
fn test_input_manager_gamepad_queries() {
    let backend = create_input_backend().unwrap();
    let manager = InputManager::new(backend);
    let gamepad_id = GamepadId::new(0);

    // Test all query methods don't panic
    assert_eq!(manager.gamepad_count(), 0);
    assert!(!manager.is_gamepad_button_down(gamepad_id, GamepadButton::South));
    assert!(!manager.is_gamepad_button_just_pressed(gamepad_id, GamepadButton::South));
    assert!(!manager.is_gamepad_button_just_released(gamepad_id, GamepadButton::South));
    assert_eq!(manager.gamepad_axis(gamepad_id, GamepadAxis::LeftStickX), 0.0);
}

// ============================================================================
// Event Type Tests
// ============================================================================

#[test]
fn test_input_event_classification() {
    // Keyboard events
    let key_event = InputEvent::KeyPressed { key: KeyCode::A };
    assert!(key_event.is_keyboard());
    assert!(!key_event.is_mouse());
    assert!(!key_event.is_gamepad());

    // Mouse events
    let mouse_event = InputEvent::MouseMoved { x: 100.0, y: 200.0 };
    assert!(!mouse_event.is_keyboard());
    assert!(mouse_event.is_mouse());
    assert!(!mouse_event.is_gamepad());

    // Gamepad events
    let gamepad_event = InputEvent::GamepadConnected { id: GamepadId::new(0) };
    assert!(!gamepad_event.is_keyboard());
    assert!(!gamepad_event.is_mouse());
    assert!(gamepad_event.is_gamepad());
}

#[test]
fn test_key_code_helpers() {
    // Modifier keys
    assert!(KeyCode::LeftShift.is_modifier());
    assert!(KeyCode::RightControl.is_modifier());
    assert!(!KeyCode::A.is_modifier());

    // Function keys
    assert!(KeyCode::F1.is_function_key());
    assert!(KeyCode::F12.is_function_key());
    assert!(!KeyCode::A.is_function_key());

    // Number keys
    assert!(KeyCode::Num0.is_number());
    assert!(KeyCode::Num9.is_number());
    assert!(!KeyCode::Numpad0.is_number());

    // Letter keys
    assert!(KeyCode::A.is_letter());
    assert!(KeyCode::Z.is_letter());
    assert!(!KeyCode::Num0.is_letter());
}

#[test]
fn test_mouse_button_helpers() {
    // Primary buttons
    assert!(MouseButton::Left.is_primary());
    assert!(MouseButton::Right.is_primary());
    assert!(MouseButton::Middle.is_primary());
    assert!(!MouseButton::Button4.is_primary());
}

#[test]
fn test_gamepad_axis_helpers() {
    // Stick axes
    assert!(GamepadAxis::LeftStickX.is_stick());
    assert!(GamepadAxis::RightStickY.is_stick());
    assert!(!GamepadAxis::LeftTriggerPressure.is_stick());

    // Trigger axes
    assert!(GamepadAxis::LeftTriggerPressure.is_trigger());
    assert!(GamepadAxis::RightTriggerPressure.is_trigger());
    assert!(!GamepadAxis::LeftStickX.is_trigger());
}

// ============================================================================
// Cross-Platform Consistency Tests
// ============================================================================

#[test]
fn test_backend_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Box<dyn engine_core::platform::InputBackend>>();
}

#[test]
fn test_input_types_are_copy() {
    fn assert_copy<T: Copy>() {}
    assert_copy::<KeyCode>();
    assert_copy::<MouseButton>();
    assert_copy::<GamepadId>();
    assert_copy::<GamepadButton>();
    assert_copy::<GamepadAxis>();
}

#[test]
fn test_input_types_are_hash() {
    fn assert_hash<T: std::hash::Hash>() {}
    assert_hash::<KeyCode>();
    assert_hash::<MouseButton>();
    assert_hash::<GamepadId>();
    assert_hash::<GamepadButton>();
    assert_hash::<GamepadAxis>();
}

#[test]
fn test_performance_event_polling() {
    use std::time::Instant;

    let backend = create_input_backend().unwrap();
    let mut manager = InputManager::new(backend);

    // Benchmark update performance
    let iterations = 1000;
    let start = Instant::now();

    for _ in 0..iterations {
        manager.update();
    }

    let elapsed = start.elapsed();
    let avg_time_us = elapsed.as_micros() / iterations;

    // Should be very fast (target: < 100us per update)
    assert!(avg_time_us < 100, "Input update took {}us (target: < 100us)", avg_time_us);
}

// ============================================================================
// ECS Integration Tests
// ============================================================================

#[test]
fn test_input_state_as_component() {
    use engine_core::ecs::World;
    use engine_core::platform::InputState;

    let mut world = World::new();
    world.register::<InputState>();

    // Create an entity with input state
    let entity = world.spawn();
    let state = InputState::new();
    world.add(entity, state);

    // Verify component was added
    let retrieved = world.get::<InputState>(entity);
    assert!(retrieved.is_some(), "Failed to retrieve InputState component");
}

#[test]
fn test_input_actions_as_component() {
    use engine_core::ecs::World;
    use engine_core::platform::InputActions;

    let mut world = World::new();
    world.register::<InputActions>();

    // Create an entity with input actions
    let entity = world.spawn();
    let mut actions = InputActions::new();
    actions.bind_key("jump", KeyCode::Space);
    world.add(entity, actions);

    // Verify component was added
    let retrieved = world.get::<InputActions>(entity);
    assert!(retrieved.is_some(), "Failed to retrieve InputActions component");
}

#[test]
fn test_input_system_with_ecs() {
    use engine_core::ecs::World;
    use engine_core::platform::{InputState, InputSystem};

    let mut world = World::new();
    world.register::<InputState>();
    let mut system = InputSystem::new();

    // Create multiple entities with input state
    let player1 = world.spawn();
    world.add(player1, InputState::new());

    let player2 = world.spawn();
    world.add(player2, InputState::new());

    // Update system (should not panic)
    system.update(&mut world);

    // Verify entities still exist
    assert!(world.is_alive(player1));
    assert!(world.is_alive(player2));
}

#[test]
fn test_input_state_update_from_events() {
    use engine_core::platform::{InputState, InputSystem};

    let system = InputSystem::new();
    let mut state = InputState::new();

    // Test keyboard input
    system.update_state_from_event(&mut state, &InputEvent::KeyPressed { key: KeyCode::W });
    assert!(state.is_key_pressed(KeyCode::W));

    system.update_state_from_event(&mut state, &InputEvent::KeyPressed { key: KeyCode::A });
    assert!(state.is_key_pressed(KeyCode::A));

    // Both keys should be pressed simultaneously
    assert!(state.is_key_pressed(KeyCode::W));
    assert!(state.is_key_pressed(KeyCode::A));

    // Release one key
    system.update_state_from_event(&mut state, &InputEvent::KeyReleased { key: KeyCode::W });
    assert!(!state.is_key_pressed(KeyCode::W));
    assert!(state.is_key_pressed(KeyCode::A));
}

#[test]
fn test_input_actions_with_state() {
    use engine_core::platform::{InputActions, InputState};

    let mut actions = InputActions::new();
    let mut state = InputState::new();

    // Bind multiple keys to movement actions
    actions.bind_key("move_forward", KeyCode::W);
    actions.bind_key("move_forward", KeyCode::Up);
    actions.bind_key("move_back", KeyCode::S);
    actions.bind_key("move_left", KeyCode::A);
    actions.bind_key("move_right", KeyCode::D);

    // No actions active initially
    assert!(!actions.is_action_active("move_forward", &state));

    // Press W key
    state.set_key_pressed(KeyCode::W, true);
    assert!(actions.is_action_active("move_forward", &state));

    // Press Up key (alternative binding)
    state.set_key_pressed(KeyCode::W, false);
    state.set_key_pressed(KeyCode::Up, true);
    assert!(actions.is_action_active("move_forward", &state));

    // Multiple actions at once
    state.set_key_pressed(KeyCode::A, true);
    assert!(actions.is_action_active("move_forward", &state));
    assert!(actions.is_action_active("move_left", &state));
}

#[test]
fn test_multiple_entities_with_different_input_bindings() {
    use engine_core::ecs::World;
    use engine_core::platform::{InputActions, InputState};

    let mut world = World::new();
    world.register::<InputActions>();
    world.register::<InputState>();

    // Player 1 uses WASD
    let player1 = world.spawn();
    let mut p1_actions = InputActions::new();
    p1_actions.bind_key("move_forward", KeyCode::W);
    p1_actions.bind_key("move_left", KeyCode::A);
    world.add(player1, p1_actions);
    world.add(player1, InputState::new());

    // Player 2 uses arrow keys
    let player2 = world.spawn();
    let mut p2_actions = InputActions::new();
    p2_actions.bind_key("move_forward", KeyCode::Up);
    p2_actions.bind_key("move_left", KeyCode::Left);
    world.add(player2, p2_actions);
    world.add(player2, InputState::new());

    // Verify both entities have their components
    assert!(world.get::<InputActions>(player1).is_some());
    assert!(world.get::<InputState>(player1).is_some());
    assert!(world.get::<InputActions>(player2).is_some());
    assert!(world.get::<InputState>(player2).is_some());
}

#[test]
fn test_gamepad_analog_input_with_dead_zone() {
    use engine_core::platform::{InputActions, InputState};

    let mut actions = InputActions::with_dead_zone(0.2);
    let mut state = InputState::new();

    actions.bind_axis("move_horizontal", GamepadAxis::LeftStickX);

    // Small input below dead zone
    state.set_gamepad_axis(GamepadAxis::LeftStickX, 0.15);
    assert_eq!(actions.get_action_value("move_horizontal", &state), 0.0);

    // Input above dead zone
    state.set_gamepad_axis(GamepadAxis::LeftStickX, 0.5);
    let value = actions.get_action_value("move_horizontal", &state);
    assert!(value > 0.0);
    assert!(value < 0.5); // Should be remapped

    // Full input
    state.set_gamepad_axis(GamepadAxis::LeftStickX, 1.0);
    assert_eq!(actions.get_action_value("move_horizontal", &state), 1.0);

    // Negative direction
    state.set_gamepad_axis(GamepadAxis::LeftStickX, -0.8);
    let value = actions.get_action_value("move_horizontal", &state);
    assert!(value < 0.0);
}

#[test]
fn test_input_state_clear() {
    use engine_core::platform::{InputState, InputSystem};

    let system = InputSystem::new();
    let mut state = InputState::new();
    let gamepad_id = GamepadId::new(0);

    // Set up various input states
    system.update_state_from_event(&mut state, &InputEvent::KeyPressed { key: KeyCode::W });
    system.update_state_from_event(
        &mut state,
        &InputEvent::MouseButtonPressed { button: MouseButton::Left },
    );
    system.update_state_from_event(&mut state, &InputEvent::MouseMoved { x: 100.0, y: 200.0 });
    system.update_state_from_event(&mut state, &InputEvent::GamepadConnected { id: gamepad_id });
    system.update_state_from_event(
        &mut state,
        &InputEvent::GamepadButtonPressed { id: gamepad_id, button: GamepadButton::South },
    );

    // Verify state is set
    assert!(state.is_key_pressed(KeyCode::W));
    assert!(state.is_mouse_button_pressed(MouseButton::Left));
    assert_eq!(state.mouse_position, (100.0, 200.0));
    assert_eq!(state.gamepad_id, Some(gamepad_id));
    assert!(state.is_gamepad_button_pressed(GamepadButton::South));

    // Clear all state
    state.clear();

    // Verify everything is cleared
    assert!(!state.is_key_pressed(KeyCode::W));
    assert!(!state.is_mouse_button_pressed(MouseButton::Left));
    assert_eq!(state.mouse_position, (0.0, 0.0));
    assert_eq!(state.gamepad_id, None);
    assert!(!state.is_gamepad_button_pressed(GamepadButton::South));
}
