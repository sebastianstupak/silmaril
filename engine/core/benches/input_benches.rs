//! Comprehensive input system benchmarks.
//!
//! This benchmark suite measures the performance of the input abstraction layer:
//! - Event processing overhead
//! - State update performance
//! - Memory overhead
//! - Query performance
//!
//! # Performance Targets
//!
//! ## Event Processing
//! - Poll empty events: < 100ns per call
//! - Poll 10 events: < 1us total
//! - Poll 100 events: < 10us total
//! - Process keyboard event: < 50ns per event
//! - Process mouse event: < 50ns per event
//! - Process gamepad event: < 100ns per event
//!
//! ## State Queries
//! - is_key_down: < 10ns (hash map lookup)
//! - is_mouse_button_down: < 10ns
//! - is_gamepad_button_down: < 20ns
//! - mouse_position: < 5ns (direct field access)
//! - gamepad_axis: < 20ns (hash map lookup)
//!
//! ## Manager Operations
//! - InputManager::update (empty): < 100ns
//! - InputManager::update (10 events): < 1us
//! - InputManager::update (100 events): < 10us
//! - Just pressed detection: < 50ns

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use engine_core::platform::{
    create_input_backend, GamepadAxis, GamepadButton, GamepadId, InputBackend, InputEvent,
    InputManager, KeyCode, MouseButton,
};

// ============================================================================
// Backend Benchmarks
// ============================================================================

/// Benchmark polling empty events.
fn bench_poll_empty_events(c: &mut Criterion) {
    let mut backend = create_input_backend().expect("Failed to create input backend");

    c.bench_function("input/backend/poll_empty", |b| {
        b.iter(|| {
            let events = backend.poll_events();
            black_box(events);
        });
    });
}

/// Benchmark state queries.
fn bench_state_queries(c: &mut Criterion) {
    let backend = create_input_backend().expect("Failed to create input backend");
    let mut group = c.benchmark_group("input/backend/queries");

    group.bench_function("is_key_down", |b| {
        b.iter(|| {
            let result = backend.is_key_down(KeyCode::A);
            black_box(result);
        });
    });

    group.bench_function("is_mouse_button_down", |b| {
        b.iter(|| {
            let result = backend.is_mouse_button_down(MouseButton::Left);
            black_box(result);
        });
    });

    group.bench_function("mouse_position", |b| {
        b.iter(|| {
            let pos = backend.mouse_position();
            black_box(pos);
        });
    });

    group.bench_function("mouse_delta", |b| {
        b.iter(|| {
            let delta = backend.mouse_delta();
            black_box(delta);
        });
    });

    group.bench_function("is_gamepad_button_down", |b| {
        let gamepad_id = GamepadId::new(0);
        b.iter(|| {
            let result = backend.is_gamepad_button_down(gamepad_id, GamepadButton::South);
            black_box(result);
        });
    });

    group.bench_function("gamepad_axis", |b| {
        let gamepad_id = GamepadId::new(0);
        b.iter(|| {
            let value = backend.gamepad_axis(gamepad_id, GamepadAxis::LeftStickX);
            black_box(value);
        });
    });

    group.bench_function("gamepad_count", |b| {
        b.iter(|| {
            let count = backend.gamepad_count();
            black_box(count);
        });
    });

    group.finish();
}

// ============================================================================
// Event Processing Benchmarks
// ============================================================================

#[cfg(target_os = "windows")]
fn bench_event_processing(c: &mut Criterion) {
    use engine_core::platform::input::backend::windows::WindowsInput;

    let mut group = c.benchmark_group("input/event_processing");

    // Benchmark processing different numbers of events
    for event_count in [1, 10, 50, 100, 500].iter() {
        group.bench_with_input(
            BenchmarkId::new("keyboard_events", event_count),
            event_count,
            |b, &count| {
                let mut backend = WindowsInput::new().unwrap();

                // Pre-generate events
                let events: Vec<_> = (0..count)
                    .map(|i| {
                        let key = if i % 2 == 0 {
                            InputEvent::KeyPressed { key: KeyCode::A }
                        } else {
                            InputEvent::KeyReleased { key: KeyCode::A }
                        };
                        key
                    })
                    .collect();

                b.iter(|| {
                    for event in &events {
                        backend.feed_event(event.clone());
                    }
                    let polled = backend.poll_events();
                    black_box(polled);
                });
            },
        );
    }

    // Benchmark mouse event processing
    group.bench_function("mouse_move_events/100", |b| {
        let mut backend = WindowsInput::new().unwrap();

        let events: Vec<_> =
            (0..100).map(|i| InputEvent::MouseMoved { x: i as f32, y: i as f32 }).collect();

        b.iter(|| {
            for event in &events {
                backend.feed_event(event.clone());
            }
            let polled = backend.poll_events();
            black_box(polled);
        });
    });

    // Benchmark gamepad event processing
    group.bench_function("gamepad_events/100", |b| {
        let mut backend = WindowsInput::new().unwrap();
        let gamepad_id = GamepadId::new(0);

        backend.feed_event(InputEvent::GamepadConnected { id: gamepad_id });

        let events: Vec<_> = (0..100)
            .map(|i| {
                if i % 2 == 0 {
                    InputEvent::GamepadButtonPressed {
                        id: gamepad_id,
                        button: GamepadButton::South,
                    }
                } else {
                    InputEvent::GamepadAxis {
                        id: gamepad_id,
                        axis: GamepadAxis::LeftStickX,
                        value: (i as f32) / 100.0,
                    }
                }
            })
            .collect();

        b.iter(|| {
            for event in &events {
                backend.feed_event(event.clone());
            }
            let polled = backend.poll_events();
            black_box(polled);
        });
    });

    group.finish();
}

#[cfg(not(target_os = "windows"))]
fn bench_event_processing(_c: &mut Criterion) {
    // Skip on non-Windows platforms (same implementation, no need to repeat)
}

// ============================================================================
// Input Manager Benchmarks
// ============================================================================

/// Benchmark InputManager update with no events.
fn bench_manager_update_empty(c: &mut Criterion) {
    let backend = create_input_backend().expect("Failed to create input backend");
    let mut manager = InputManager::new(backend);

    c.bench_function("input/manager/update_empty", |b| {
        b.iter(|| {
            manager.update();
        });
    });
}

/// Benchmark InputManager update with events.
#[cfg(target_os = "windows")]
fn bench_manager_update_with_events(c: &mut Criterion) {
    use engine_core::platform::input::backend::windows::WindowsInput;

    let mut group = c.benchmark_group("input/manager/update_with_events");

    for event_count in [10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(event_count),
            event_count,
            |b, &count| {
                let _backend = WindowsInput::new().unwrap();

                // Pre-generate events
                let events: Vec<_> = (0..count)
                    .map(|i| InputEvent::KeyPressed {
                        key: if i % 2 == 0 { KeyCode::A } else { KeyCode::B },
                    })
                    .collect();

                b.iter(|| {
                    // Create a fresh backend for each iteration
                    let mut fresh_backend = WindowsInput::new().unwrap();

                    // Feed events to backend
                    for event in &events {
                        fresh_backend.feed_event(event.clone());
                    }

                    // Create manager and update
                    let mut manager = InputManager::new(Box::new(fresh_backend));
                    manager.update();
                    black_box(manager);
                });
            },
        );
    }

    group.finish();
}

#[cfg(not(target_os = "windows"))]
fn bench_manager_update_with_events(_c: &mut Criterion) {
    // Skip on non-Windows platforms
}

/// Benchmark InputManager state queries.
fn bench_manager_queries(c: &mut Criterion) {
    let backend = create_input_backend().expect("Failed to create input backend");
    let manager = InputManager::new(backend);

    let mut group = c.benchmark_group("input/manager/queries");

    group.bench_function("is_key_down", |b| {
        b.iter(|| {
            let result = manager.is_key_down(KeyCode::A);
            black_box(result);
        });
    });

    group.bench_function("is_key_just_pressed", |b| {
        b.iter(|| {
            let result = manager.is_key_just_pressed(KeyCode::A);
            black_box(result);
        });
    });

    group.bench_function("is_key_just_released", |b| {
        b.iter(|| {
            let result = manager.is_key_just_released(KeyCode::A);
            black_box(result);
        });
    });

    group.bench_function("is_mouse_button_down", |b| {
        b.iter(|| {
            let result = manager.is_mouse_button_down(MouseButton::Left);
            black_box(result);
        });
    });

    group.bench_function("is_mouse_button_just_pressed", |b| {
        b.iter(|| {
            let result = manager.is_mouse_button_just_pressed(MouseButton::Left);
            black_box(result);
        });
    });

    group.bench_function("mouse_position", |b| {
        b.iter(|| {
            let pos = manager.mouse_position();
            black_box(pos);
        });
    });

    group.bench_function("mouse_delta", |b| {
        b.iter(|| {
            let delta = manager.mouse_delta();
            black_box(delta);
        });
    });

    group.bench_function("gamepad_count", |b| {
        b.iter(|| {
            let count = manager.gamepad_count();
            black_box(count);
        });
    });

    let gamepad_id = GamepadId::new(0);

    group.bench_function("is_gamepad_button_down", |b| {
        b.iter(|| {
            let result = manager.is_gamepad_button_down(gamepad_id, GamepadButton::South);
            black_box(result);
        });
    });

    group.bench_function("gamepad_axis", |b| {
        b.iter(|| {
            let value = manager.gamepad_axis(gamepad_id, GamepadAxis::LeftStickX);
            black_box(value);
        });
    });

    group.finish();
}

// ============================================================================
// Memory Overhead Benchmarks
// ============================================================================

/// Benchmark memory allocation for input state.
#[cfg(target_os = "windows")]
fn bench_memory_overhead(c: &mut Criterion) {
    use engine_core::platform::input::backend::windows::WindowsInput;

    let mut group = c.benchmark_group("input/memory");

    // Benchmark state with different numbers of pressed keys
    group.bench_function("state/10_keys_pressed", |b| {
        b.iter(|| {
            let mut backend = WindowsInput::new().unwrap();

            // Press 10 different keys
            for i in 0..10 {
                let key = match i {
                    0 => KeyCode::A,
                    1 => KeyCode::B,
                    2 => KeyCode::C,
                    3 => KeyCode::D,
                    4 => KeyCode::E,
                    5 => KeyCode::F,
                    6 => KeyCode::G,
                    7 => KeyCode::H,
                    8 => KeyCode::I,
                    _ => KeyCode::J,
                };
                backend.feed_event(InputEvent::KeyPressed { key });
            }

            black_box(backend);
        });
    });

    // Benchmark state with multiple gamepads
    group.bench_function("state/4_gamepads_connected", |b| {
        b.iter(|| {
            let mut backend = WindowsInput::new().unwrap();

            // Connect 4 gamepads
            for i in 0..4 {
                backend.feed_event(InputEvent::GamepadConnected { id: GamepadId::new(i) });
            }

            black_box(backend);
        });
    });

    group.finish();
}

#[cfg(not(target_os = "windows"))]
fn bench_memory_overhead(_c: &mut Criterion) {
    // Skip on non-Windows platforms
}

// ============================================================================
// Realistic Scenario Benchmarks
// ============================================================================

/// Benchmark a realistic game input scenario.
#[cfg(target_os = "windows")]
fn bench_realistic_game_input(c: &mut Criterion) {
    use engine_core::platform::input::backend::windows::WindowsInput;

    c.bench_function("input/realistic/fps_movement", |b| {
        // Simulate FPS game input: WASD movement + mouse look + jump
        b.iter(|| {
            let backend = WindowsInput::new().unwrap();
            let mut manager = InputManager::new(Box::new(backend));

            // Simulate one frame of input
            manager.update();

            // Check movement keys (WASD)
            let w = manager.is_key_down(KeyCode::W);
            let a = manager.is_key_down(KeyCode::A);
            let s = manager.is_key_down(KeyCode::S);
            let d = manager.is_key_down(KeyCode::D);

            // Check jump
            let space = manager.is_key_just_pressed(KeyCode::Space);

            // Get mouse delta for camera
            let mouse_delta = manager.mouse_delta();

            black_box((w, a, s, d, space, mouse_delta));
        });
    });

    c.bench_function("input/realistic/gamepad_control", |b| {
        // Simulate gamepad control: sticks + buttons
        b.iter(|| {
            let backend = create_input_backend().unwrap();
            let mut manager = InputManager::new(backend);
            let gamepad = GamepadId::new(0);

            manager.update();

            // Check left stick (movement)
            let left_x = manager.gamepad_axis(gamepad, GamepadAxis::LeftStickX);
            let left_y = manager.gamepad_axis(gamepad, GamepadAxis::LeftStickY);

            // Check right stick (camera)
            let right_x = manager.gamepad_axis(gamepad, GamepadAxis::RightStickX);
            let right_y = manager.gamepad_axis(gamepad, GamepadAxis::RightStickY);

            // Check buttons
            let jump = manager.is_gamepad_button_just_pressed(gamepad, GamepadButton::South);
            let shoot = manager.is_gamepad_button_down(gamepad, GamepadButton::RightTrigger);

            black_box((left_x, left_y, right_x, right_y, jump, shoot));
        });
    });
}

#[cfg(not(target_os = "windows"))]
fn bench_realistic_game_input(_c: &mut Criterion) {
    // Skip on non-Windows platforms
}

// ============================================================================
// Backend Creation Benchmarks
// ============================================================================

/// Benchmark creating input backend.
fn bench_backend_creation(c: &mut Criterion) {
    c.bench_function("input/backend_creation", |b| {
        b.iter(|| {
            let backend = create_input_backend().unwrap();
            black_box(backend);
        });
    });
}

/// Benchmark creating input manager.
fn bench_manager_creation(c: &mut Criterion) {
    c.bench_function("input/manager_creation", |b| {
        b.iter(|| {
            let backend = create_input_backend().unwrap();
            let manager = InputManager::new(backend);
            black_box(manager);
        });
    });
}

// ============================================================================
// Criterion Configuration
// ============================================================================

criterion_group!(
    backend_benches,
    bench_poll_empty_events,
    bench_state_queries,
    bench_backend_creation,
);

criterion_group!(event_benches, bench_event_processing,);

criterion_group!(
    manager_benches,
    bench_manager_update_empty,
    bench_manager_update_with_events,
    bench_manager_queries,
    bench_manager_creation,
);

criterion_group!(memory_benches, bench_memory_overhead,);

criterion_group!(realistic_benches, bench_realistic_game_input,);

criterion_main!(
    backend_benches,
    event_benches,
    manager_benches,
    memory_benches,
    realistic_benches,
);
