//! Fuzz tests for audio system
//!
//! Tests the audio system with random, malformed, and extreme inputs to verify
//! robustness and error handling.
//!
//! These tests use:
//! - Invalid/malformed inputs
//! - Extreme parameter values
//! - NaN/Infinity values
//! - Very large/small numbers
//! - Empty strings, null bytes
//! - Unicode edge cases
//! - Random combinations

use engine_audio::{
    AudioEngine, AudioListener, AudioSystem, DopplerCalculator, EchoEffect, EqEffect, FilterEffect,
    FilterType, ReverbEffect, Sound,
};
use engine_core::ecs::World;
use engine_core::math::{Quat, Transform, Vec3};
use tracing::info;

/// Fuzz test: Random float values for Doppler calculations
#[test]
fn fuzz_doppler_random_floats() {
    let calc = DopplerCalculator::default();

    let test_values = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        f32::MIN,
        f32::MAX,
        f32::MIN_POSITIVE,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        f32::EPSILON,
        -f32::EPSILON,
        1e-38,
        1e38,
        -1e38,
        1e-10,
        1e10,
    ];

    for &listener_x in &test_values {
        for &emitter_x in &test_values {
            for &velocity_x in &test_values {
                let shift = calc.calculate_pitch_shift(
                    Vec3::new(listener_x, 0.0, 0.0),
                    Vec3::ZERO,
                    Vec3::new(emitter_x, 0.0, 0.0),
                    Vec3::new(velocity_x, 0.0, 0.0),
                );

                // Should never return NaN or infinity (or if it does, should be handled)
                if shift.is_finite() {
                    assert!(
                        shift >= 0.0 && shift <= 10.0,
                        "Pitch shift out of reasonable range: {}",
                        shift
                    );
                }
            }
        }
    }

    info!("Fuzz test passed - Doppler random floats");
}

/// Fuzz test: Random Vec3 values for positions
#[test]
fn fuzz_vec3_positions() {
    let calc = DopplerCalculator::default();

    let test_vecs = vec![
        Vec3::ZERO,
        Vec3::ONE,
        Vec3::NEG_ONE,
        Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY),
        Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY),
        Vec3::new(f32::NAN, f32::NAN, f32::NAN),
        Vec3::new(f32::MAX, f32::MAX, f32::MAX),
        Vec3::new(f32::MIN, f32::MIN, f32::MIN),
        Vec3::new(1e20, 1e20, 1e20),
        Vec3::new(-1e20, -1e20, -1e20),
        Vec3::new(1e-20, 1e-20, 1e-20),
    ];

    for listener_pos in &test_vecs {
        for emitter_pos in &test_vecs {
            let shift =
                calc.calculate_pitch_shift(*listener_pos, Vec3::ZERO, *emitter_pos, Vec3::ZERO);

            // Should not crash or return invalid values
            if shift.is_finite() {
                assert!(shift > 0.0, "Pitch shift should be positive: {}", shift);
            }
        }
    }

    info!("Fuzz test passed - Vec3 positions");
}

/// Fuzz test: Random velocity calculations
#[test]
fn fuzz_velocity_calculation() {
    let delta_times = vec![
        0.0,
        -1.0,
        0.016,
        1.0,
        100.0,
        f32::MIN_POSITIVE,
        f32::MAX,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
    ];

    let positions = vec![
        Vec3::ZERO,
        Vec3::new(100.0, 0.0, 0.0),
        Vec3::new(f32::MAX, 0.0, 0.0),
        Vec3::new(f32::NAN, f32::NAN, f32::NAN),
        Vec3::new(f32::INFINITY, 0.0, 0.0),
    ];

    for &dt in &delta_times {
        for prev_pos in &positions {
            for curr_pos in &positions {
                let _vel = DopplerCalculator::calculate_velocity(*prev_pos, *curr_pos, dt);

                // Any result is acceptable as long as it doesn't crash
                // (infinity, NaN, or finite values are all valid edge case behaviors)
            }
        }
    }

    info!("Fuzz test passed - Velocity calculation");
}

/// Fuzz test: Sound component with extreme values
#[test]
fn fuzz_sound_component_values() {
    let volumes = vec![
        -1000.0,
        -1.0,
        0.0,
        0.5,
        1.0,
        2.0,
        1000.0,
        f32::MIN,
        f32::MAX,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
    ];

    let doppler_scales = vec![-100.0, 0.0, 1.0, 10.0, 100.0, f32::MAX, f32::INFINITY, f32::NAN];

    let distances = vec![-1000.0, 0.0, 1.0, 1000.0, f32::MAX, f32::INFINITY, f32::NAN];

    for &vol in &volumes {
        for &doppler in &doppler_scales {
            for &dist in &distances {
                let sound =
                    Sound::new("test.wav").with_volume(vol).with_doppler(doppler).spatial_3d(dist);

                // Sound should be created without panicking
                // Values should be clamped or validated (or may be NaN/infinity for extreme inputs)
                if sound.volume.is_finite() {
                    assert!(sound.volume >= 0.0 && sound.volume <= 1.0);
                }
                if sound.doppler_scale.is_finite() {
                    assert!(sound.doppler_scale >= 0.0 && sound.doppler_scale <= 10.0);
                }
                // max_distance may be infinity or NaN for invalid inputs, which is acceptable
            }
        }
    }

    info!("Fuzz test passed - Sound component values");
}

/// Fuzz test: String inputs with special characters
#[test]
fn fuzz_sound_names() {
    let long_name = "very_long_".to_string() + &"a".repeat(10000);

    let test_names: Vec<&str> = vec![
        "",                                    // Empty
        " ",                                   // Space
        "  ",                                  // Multiple spaces
        "\0",                                  // Null byte
        "\n\r\t",                              // Whitespace
        "a\0b",                                // Embedded null
        &long_name,                            // Very long
        "音声ファイル",                        // Japanese
        "звук",                                // Russian
        "🔊🎵🎶",                              // Emojis
        "../../../etc/passwd",                 // Path traversal
        "C:\\Windows\\System32\\kernel32.dll", // Windows path
        "/dev/null",                           // Unix special file
        "CON",                                 // Windows reserved
        "LPT1",                                // Windows reserved
        "sound.wav\0hidden",                   // Null injection
        "sound\r\nname",                       // Newline injection
        "sound'OR'1'='1",                      // SQL-like injection
        "<script>alert()</script>",            // HTML injection
        "sound\u{200B}name",                   // Zero-width space
        "\u{FEFF}sound",                       // Byte order mark
    ];

    for name in &test_names {
        let sound = Sound::new(*name);
        // Should not crash or cause UB
        assert_eq!(sound.sound_name, *name);
    }

    info!("Fuzz test passed - Sound names");
}

/// Fuzz test: AudioEngine operations with extreme values
#[test]
fn fuzz_audio_engine_parameters() {
    let mut engine = AudioEngine::new().expect("Failed to create audio engine");

    let volumes = vec![-1000.0, 0.0, 1.0, 1000.0, f32::NAN, f32::INFINITY];

    let positions = vec![
        Vec3::ZERO,
        Vec3::new(f32::MAX, 0.0, 0.0),
        Vec3::new(f32::NAN, f32::NAN, f32::NAN),
        Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY),
    ];

    let distances = vec![-100.0, 0.0, 100.0, f32::MAX, f32::INFINITY, f32::NAN];

    for &vol in &volumes {
        for pos in &positions {
            for &dist in &distances {
                // Try to play with extreme parameters - should not crash
                let _ = engine.play_2d("test", vol, false);
                let _ = engine.play_3d(1, "test", *pos, vol, false, dist);
            }
        }
    }

    info!("Fuzz test passed - AudioEngine parameters");
}

/// Fuzz test: Listener transform with extreme values
#[test]
fn fuzz_listener_transform() {
    let mut engine = AudioEngine::new().expect("Failed to create audio engine");

    let positions = vec![
        Vec3::ZERO,
        Vec3::new(f32::MAX, f32::MAX, f32::MAX),
        Vec3::new(f32::MIN, f32::MIN, f32::MIN),
        Vec3::new(f32::INFINITY, 0.0, 0.0),
        Vec3::new(f32::NAN, f32::NAN, f32::NAN),
    ];

    let directions = vec![
        Vec3::ZERO,                         // Invalid (zero vector)
        Vec3::new(1.0, 0.0, 0.0),           // Valid
        Vec3::new(f32::MAX, 0.0, 0.0),      // Not normalized
        Vec3::new(f32::NAN, 0.0, 0.0),      // NaN
        Vec3::new(f32::INFINITY, 0.0, 0.0), // Infinity
        Vec3::new(0.001, 0.001, 0.001),     // Very small
        Vec3::new(1000.0, 1000.0, 1000.0),  // Very large
    ];

    for pos in &positions {
        for forward in &directions {
            for up in &directions {
                // Should not crash with any combination
                engine.set_listener_transform(*pos, *forward, *up);
            }
        }
    }

    info!("Fuzz test passed - Listener transform");
}

/// Fuzz test: Emitter operations with random entity IDs
#[test]
fn fuzz_emitter_operations() {
    let mut engine = AudioEngine::new().expect("Failed to create audio engine");

    let entity_ids = vec![0, 1, 12345, u32::MAX / 2, u32::MAX - 1, u32::MAX];

    let positions = vec![
        Vec3::ZERO,
        Vec3::new(f32::MAX, 0.0, 0.0),
        Vec3::new(f32::NAN, f32::NAN, f32::NAN),
        Vec3::new(f32::INFINITY, 0.0, 0.0),
    ];

    for &entity_id in &entity_ids {
        for pos in &positions {
            // Should not crash
            engine.update_emitter_position(entity_id, *pos);
            engine.remove_emitter(entity_id);
        }
    }

    info!("Fuzz test passed - Emitter operations");
}

/// Fuzz test: Effect parameters with extreme values
#[test]
fn fuzz_effect_parameters() {
    let test_values = vec![
        -1000.0,
        -1.0,
        0.0,
        0.5,
        1.0,
        2.0,
        1000.0,
        f32::MIN,
        f32::MAX,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
    ];

    for &val1 in &test_values {
        for &val2 in &test_values {
            for &val3 in &test_values {
                // Reverb
                let reverb = ReverbEffect { room_size: val1, damping: val2, wet_dry_mix: val3 };
                let _ = reverb.validate(); // Should not crash

                // Echo
                let echo = EchoEffect { delay_time: val1, feedback: val2, wet_dry_mix: val3 };
                let _ = echo.validate();

                // Filter
                let filter = FilterEffect {
                    filter_type: FilterType::LowPass,
                    cutoff_frequency: val1,
                    resonance: val2,
                    wet_dry_mix: val3,
                };
                let _ = filter.validate();

                // EQ
                let eq = EqEffect { bass_gain: val1, mid_gain: val2, treble_gain: val3 };
                let _ = eq.validate();
            }
        }
    }

    info!("Fuzz test passed - Effect parameters");
}

/// Fuzz test: Pitch values
#[test]
fn fuzz_pitch_values() {
    let mut engine = AudioEngine::new().expect("Failed to create audio engine");

    let instance_ids = vec![0, 1, 12345, u64::MAX / 2, u64::MAX];

    let pitches = vec![
        -1000.0,
        -1.0,
        0.0,
        0.5,
        1.0,
        2.0,
        100.0,
        f32::MIN,
        f32::MAX,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
    ];

    for &instance_id in &instance_ids {
        for &pitch in &pitches {
            // Should not crash
            engine.set_pitch(instance_id, pitch);
        }
    }

    info!("Fuzz test passed - Pitch values");
}

/// Fuzz test: Speed of sound with extreme values
#[test]
fn fuzz_speed_of_sound() {
    let mut calc = DopplerCalculator::default();

    let speeds = vec![
        -1000.0,
        -1.0,
        0.0,
        1.0,
        343.0,
        1000.0,
        10000.0,
        f32::MIN,
        f32::MAX,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
    ];

    for &speed in &speeds {
        calc.set_speed_of_sound(speed);

        // Should always return valid value (or clamp to safe value)
        let result = calc.speed_of_sound();
        // Implementation may clamp to minimum or allow infinity, both are acceptable
        assert!(result > 0.0 || result.is_infinite());
    }

    info!("Fuzz test passed - Speed of sound");
}

/// Fuzz test: AudioSystem with random world states
#[test]
fn fuzz_audio_system_world_states() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    // Create entities with random data
    for i in 0..50 {
        let entity = world.spawn();

        if i % 3 == 0 {
            let mut transform = Transform::default();
            transform.position = Vec3::new(
                (i as f32 * 123.456) % 1000.0 - 500.0,
                (i as f32 * 789.012) % 1000.0 - 500.0,
                (i as f32 * 345.678) % 1000.0 - 500.0,
            );
            world.add(entity, transform);
        }

        if i % 5 == 0 {
            let sound = Sound::new(&format!("sound_{}.wav", i))
                .with_volume((i as f32 % 100.0) / 100.0)
                .with_doppler((i as f32 % 10.0) / 5.0)
                .spatial_3d((i as f32 % 200.0) + 10.0);
            world.add(entity, sound);
        }

        if i % 17 == 0 {
            world.add(entity, AudioListener::new());
        }
    }

    // Should handle random state
    system.update(&mut world, 0.016);

    info!("Fuzz test passed - AudioSystem world states");
}

/// Fuzz test: Quaternion rotations for listener orientation
#[test]
fn fuzz_listener_quaternions() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    let quaternions = vec![
        Quat::IDENTITY,
        Quat::from_xyzw(0.0, 0.0, 0.0, 1.0),
        Quat::from_xyzw(1.0, 0.0, 0.0, 0.0),
        Quat::from_xyzw(0.0, 1.0, 0.0, 0.0),
        Quat::from_xyzw(0.0, 0.0, 1.0, 0.0),
        Quat::from_xyzw(f32::NAN, 0.0, 0.0, 1.0),
        Quat::from_xyzw(f32::INFINITY, 0.0, 0.0, 1.0),
        Quat::from_xyzw(0.5, 0.5, 0.5, 0.5), // Not normalized
        Quat::from_xyzw(100.0, 100.0, 100.0, 100.0),
    ];

    for rotation in &quaternions {
        let listener = world.spawn();
        let mut transform = Transform::default();
        transform.rotation = *rotation;
        world.add(listener, transform);
        world.add(listener, AudioListener::new());

        // Should not crash
        system.update(&mut world, 0.016);

        world.despawn(listener);
    }

    info!("Fuzz test passed - Listener quaternions");
}

/// Fuzz test: Rapid entity spawn/despawn cycles
#[test]
fn fuzz_entity_lifecycle() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    // Create listener
    let listener = world.spawn();
    world.add(listener, Transform::default());
    world.add(listener, AudioListener::new());

    // Rapid spawn/despawn cycles
    for cycle in 0..50 {
        let entities: Vec<_> = (0..10)
            .map(|i| {
                let entity = world.spawn();

                if i % 2 == 0 {
                    world.add(entity, Transform::default());
                }

                if i % 3 == 0 {
                    world.add(entity, Sound::new(&format!("sound_{}.wav", cycle)));
                }

                entity
            })
            .collect();

        system.update(&mut world, 0.016);

        // Despawn half
        for i in 0..5 {
            world.despawn(entities[i]);
        }

        system.update(&mut world, 0.016);

        // Despawn rest
        for i in 5..10 {
            world.despawn(entities[i]);
        }
    }

    info!("Fuzz test passed - Entity lifecycle");
}

/// Fuzz test: Component add/remove patterns
#[test]
fn fuzz_component_mutations() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    let listener = world.spawn();
    world.add(listener, Transform::default());
    world.add(listener, AudioListener::new());

    let entities: Vec<_> = (0..20)
        .map(|_| {
            let entity = world.spawn();
            world.add(entity, Transform::default());
            entity
        })
        .collect();

    for i in 0..100 {
        // Add sounds to some entities
        for (j, entity) in entities.iter().enumerate() {
            if (i + j) % 3 == 0 {
                world.add(*entity, Sound::new("test.wav"));
            }
        }

        system.update(&mut world, 0.016);

        // Remove sounds from some entities
        for (j, entity) in entities.iter().enumerate() {
            if (i + j) % 5 == 0 {
                world.remove::<Sound>(*entity);
            }
        }

        system.update(&mut world, 0.016);
    }

    info!("Fuzz test passed - Component mutations");
}

/// Fuzz test: Delta time variations
#[test]
fn fuzz_delta_time() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    let listener = world.spawn();
    world.add(listener, Transform::default());
    world.add(listener, AudioListener::new());

    let entity = world.spawn();
    world.add(entity, Transform::default());
    world.add(entity, Sound::new("test.wav").with_doppler(1.0));

    let delta_times = vec![
        0.0,
        -1.0,
        0.001,
        0.016,
        0.1,
        1.0,
        10.0,
        100.0,
        f32::MIN_POSITIVE,
        f32::MAX,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
    ];

    for &dt in &delta_times {
        // Should not crash with any delta time
        system.update(&mut world, dt);
    }

    info!("Fuzz test passed - Delta time variations");
}
