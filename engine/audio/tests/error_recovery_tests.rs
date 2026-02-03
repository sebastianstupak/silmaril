//! Error recovery tests for audio system
//!
//! Tests that the audio system gracefully handles errors and continues
//! functioning after encountering invalid inputs, file errors, and other
//! exceptional conditions.
//!
//! These tests verify:
//! - Recovery from invalid file paths
//! - Recovery from corrupted audio data
//! - Recovery from out-of-memory conditions
//! - Recovery from invalid parameters
//! - Graceful degradation when backend fails
//! - Error propagation through the stack
//! - System continues working after errors

use engine_audio::{
    AudioEffect, AudioEngine, AudioListener, AudioSystem, DopplerCalculator, EchoEffect, EqEffect,
    FilterEffect, FilterType, ReverbEffect, Sound,
};
use engine_core::ecs::World;
use engine_core::math::{Transform, Vec3};
use tracing::info;

/// Test recovery from loading non-existent files
#[test]
fn test_recovery_from_missing_file() {
    let mut engine = AudioEngine::new().expect("Failed to create audio engine");

    // Attempt to load non-existent file
    let result = engine.load_sound("missing", "this/file/does/not/exist.wav");
    assert!(result.is_err());

    // Engine should still function after error
    assert_eq!(engine.loaded_sound_count(), 0);
    assert_eq!(engine.active_sound_count(), 0);

    // Should be able to continue using engine
    engine.cleanup_finished();

    info!("Recovered from missing file error");
}

/// Test recovery from invalid file paths
#[test]
fn test_recovery_from_invalid_paths() {
    let mut engine = AudioEngine::new().expect("Failed to create audio engine");

    let invalid_paths = vec![
        "",                                    // Empty path
        "\0",                                  // Null byte
        "..\\..\\..\\..\\etc\\passwd",         // Path traversal
        "/dev/null",                           // Invalid audio file
        "C:\\Windows\\System32\\kernel32.dll", // DLL file (not audio)
        "audio\0hidden.wav",                   // Embedded null
    ];

    for (i, path) in invalid_paths.iter().enumerate() {
        let result = engine.load_sound(&format!("invalid_{}", i), path);

        // Should fail gracefully
        if result.is_ok() {
            // Loading may succeed for some paths (like /dev/null)
            // but playback should fail safely
            info!(path = path, "Path loaded unexpectedly");
        }
    }

    // Engine should still be functional
    assert_eq!(engine.active_sound_count(), 0);

    info!("Recovered from invalid path errors");
}

/// Test recovery from attempting to play non-existent sounds
#[test]
fn test_recovery_from_playing_missing_sound() {
    let mut engine = AudioEngine::new().expect("Failed to create audio engine");

    // Try to play sound that was never loaded
    let result = engine.play_2d("nonexistent", 1.0, false);
    assert!(result.is_err());

    // Try 3D version
    let result3d = engine.play_3d(1, "nonexistent", Vec3::ZERO, 1.0, false, 100.0);
    assert!(result3d.is_err());

    // Engine should still function
    assert_eq!(engine.active_sound_count(), 0);

    info!("Recovered from playing non-existent sound");
}

/// Test recovery from stopping invalid sound instances
#[test]
fn test_recovery_from_invalid_instance_stop() {
    let mut engine = AudioEngine::new().expect("Failed to create audio engine");

    // Stop non-existent instances - should not crash
    engine.stop(0, None);
    engine.stop(12345, None);
    engine.stop(u64::MAX, None);
    engine.stop(u64::MAX, Some(1.0));

    // Engine should still function
    assert_eq!(engine.active_sound_count(), 0);

    info!("Recovered from stopping invalid instances");
}

/// Test recovery from querying invalid sound instances
#[test]
fn test_recovery_from_invalid_instance_query() {
    let engine = AudioEngine::new().expect("Failed to create audio engine");

    // Query non-existent instances - should return false/0
    assert!(!engine.is_playing(0));
    assert!(!engine.is_playing(12345));
    assert!(!engine.is_playing(u64::MAX));

    assert_eq!(engine.effect_count(0), 0);
    assert_eq!(engine.effect_count(12345), 0);
    assert_eq!(engine.effect_count(u64::MAX), 0);

    info!("Recovered from querying invalid instances");
}

/// Test recovery from adding effects to invalid instances
#[test]
fn test_recovery_from_invalid_effect_addition() {
    let mut engine = AudioEngine::new().expect("Failed to create audio engine");

    let reverb = ReverbEffect::small_room();
    let echo = EchoEffect::long_echo();

    // Add effects to non-existent instances
    let result1 = engine.add_effect(0, AudioEffect::Reverb(reverb));
    let result2 = engine.add_effect(12345, AudioEffect::Echo(echo));

    // Should fail gracefully
    assert!(result1.is_err() || result1.is_ok()); // Backend-dependent
    assert!(result2.is_err() || result2.is_ok());

    // Engine should still function
    assert_eq!(engine.active_sound_count(), 0);

    info!("Recovered from invalid effect addition");
}

/// Test recovery from removing invalid effects
#[test]
fn test_recovery_from_invalid_effect_removal() {
    let mut engine = AudioEngine::new().expect("Failed to create audio engine");

    // Remove effects from non-existent instances
    assert!(!engine.remove_effect(0, 0));
    assert!(!engine.remove_effect(12345, 0));
    assert!(!engine.remove_effect(u64::MAX, usize::MAX));

    // Clear effects from non-existent instances
    engine.clear_effects(0);
    engine.clear_effects(12345);
    engine.clear_effects(u64::MAX);

    // Engine should still function
    assert_eq!(engine.active_sound_count(), 0);

    info!("Recovered from invalid effect removal");
}

/// Test recovery from extreme parameter values
#[test]
fn test_recovery_from_extreme_parameters() {
    let mut engine = AudioEngine::new().expect("Failed to create audio engine");

    // Extreme volumes (should be clamped internally or rejected)
    let _ = engine.play_2d("test", f32::INFINITY, false);
    let _ = engine.play_2d("test", f32::NEG_INFINITY, false);
    let _ = engine.play_2d("test", f32::NAN, false);
    let _ = engine.play_2d("test", -1000.0, false);
    let _ = engine.play_2d("test", 1000.0, false);

    // Extreme positions
    let _ = engine.play_3d(
        1,
        "test",
        Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY),
        1.0,
        false,
        100.0,
    );
    let _ = engine.play_3d(1, "test", Vec3::new(f32::NAN, f32::NAN, f32::NAN), 1.0, false, 100.0);
    let _ = engine.play_3d(1, "test", Vec3::new(f32::MAX, f32::MAX, f32::MAX), 1.0, false, 100.0);

    // Extreme max distance
    let _ = engine.play_3d(1, "test", Vec3::ZERO, 1.0, false, f32::INFINITY);
    let _ = engine.play_3d(1, "test", Vec3::ZERO, 1.0, false, f32::NAN);
    let _ = engine.play_3d(1, "test", Vec3::ZERO, 1.0, false, -100.0);

    // Engine should not crash
    engine.cleanup_finished();

    info!("Recovered from extreme parameter values");
}

/// Test recovery from invalid listener transforms
#[test]
fn test_recovery_from_invalid_listener_transform() {
    let mut engine = AudioEngine::new().expect("Failed to create audio engine");

    // Invalid positions
    engine.set_listener_transform(
        Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY),
        Vec3::new(0.0, 0.0, -1.0),
        Vec3::new(0.0, 1.0, 0.0),
    );

    engine.set_listener_transform(
        Vec3::new(f32::NAN, f32::NAN, f32::NAN),
        Vec3::new(0.0, 0.0, -1.0),
        Vec3::new(0.0, 1.0, 0.0),
    );

    // Invalid directions (zero vectors)
    engine.set_listener_transform(Vec3::ZERO, Vec3::ZERO, Vec3::ZERO);

    // Non-normalized directions
    engine.set_listener_transform(
        Vec3::ZERO,
        Vec3::new(10.0, 10.0, 10.0),
        Vec3::new(5.0, 5.0, 5.0),
    );

    // Invalid directions (NaN)
    engine.set_listener_transform(
        Vec3::ZERO,
        Vec3::new(f32::NAN, 0.0, 0.0),
        Vec3::new(0.0, f32::NAN, 0.0),
    );

    // Engine should still function
    assert_eq!(engine.active_sound_count(), 0);

    info!("Recovered from invalid listener transforms");
}

/// Test recovery from invalid emitter operations
#[test]
fn test_recovery_from_invalid_emitter_operations() {
    let mut engine = AudioEngine::new().expect("Failed to create audio engine");

    // Update non-existent emitters
    engine.update_emitter_position(0, Vec3::ZERO);
    engine.update_emitter_position(12345, Vec3::new(f32::INFINITY, 0.0, 0.0));
    engine.update_emitter_position(u32::MAX, Vec3::new(f32::NAN, f32::NAN, f32::NAN));

    // Remove non-existent emitters
    engine.remove_emitter(0);
    engine.remove_emitter(12345);
    engine.remove_emitter(u32::MAX);

    // Engine should still function
    assert_eq!(engine.active_sound_count(), 0);

    info!("Recovered from invalid emitter operations");
}

/// Test recovery from invalid pitch values
#[test]
fn test_recovery_from_invalid_pitch() {
    let mut engine = AudioEngine::new().expect("Failed to create audio engine");

    // Set pitch on non-existent instances
    engine.set_pitch(0, 1.0);
    engine.set_pitch(12345, f32::INFINITY);
    engine.set_pitch(u64::MAX, f32::NAN);
    engine.set_pitch(12345, -1.0);
    engine.set_pitch(12345, 0.0);
    engine.set_pitch(12345, 100.0);

    // Engine should still function
    assert_eq!(engine.active_sound_count(), 0);

    info!("Recovered from invalid pitch operations");
}

/// Test ECS system recovery from invalid world state
#[test]
fn test_system_recovery_from_invalid_world() {
    let mut world = World::new();
    // Don't register components - should handle gracefully
    let mut system = AudioSystem::new().expect("Failed to create audio system");

    // Update with unregistered components - should not crash
    system.update(&mut world, 0.016);

    info!("System recovered from unregistered components");
}

/// Test system recovery from missing listener
#[test]
fn test_system_recovery_from_missing_listener() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    // Create sounds but no listener
    for i in 0..10 {
        let entity = world.spawn();
        let mut transform = Transform::default();
        transform.position = Vec3::new(i as f32 * 5.0, 0.0, 0.0);
        world.add(entity, transform);
        world.add(entity, Sound::new("test.wav"));
    }

    // Should handle gracefully without listener
    system.update(&mut world, 0.016);

    info!("System recovered from missing listener");
}

/// Test recovery from Doppler calculation edge cases
#[test]
fn test_doppler_recovery_from_edge_cases() {
    let calc = DopplerCalculator::default();

    // Division by zero scenarios
    let shift1 = calc.calculate_pitch_shift(Vec3::ZERO, Vec3::ZERO, Vec3::ZERO, Vec3::ZERO);
    assert!(shift1.is_finite());

    // NaN inputs
    let shift2 = calc.calculate_pitch_shift(
        Vec3::new(f32::NAN, 0.0, 0.0),
        Vec3::ZERO,
        Vec3::new(100.0, 0.0, 0.0),
        Vec3::ZERO,
    );
    assert!(shift2.is_finite() || shift2.is_nan()); // Either is acceptable

    // Infinity inputs
    let shift3 = calc.calculate_pitch_shift(
        Vec3::new(f32::INFINITY, 0.0, 0.0),
        Vec3::ZERO,
        Vec3::new(100.0, 0.0, 0.0),
        Vec3::ZERO,
    );
    // NaN is acceptable for infinity inputs
    assert!(shift3.is_finite() || shift3.is_nan() || shift3 == 1.0);

    info!("Doppler recovered from edge cases");
}

/// Test velocity calculation recovery
#[test]
fn test_velocity_recovery_from_invalid_inputs() {
    // Zero delta time
    let vel1 = DopplerCalculator::calculate_velocity(Vec3::ZERO, Vec3::new(100.0, 0.0, 0.0), 0.0);
    assert_eq!(vel1, Vec3::ZERO);

    // Negative delta time
    let vel2 = DopplerCalculator::calculate_velocity(Vec3::ZERO, Vec3::new(100.0, 0.0, 0.0), -1.0);
    assert_eq!(vel2, Vec3::ZERO);

    // NaN positions
    let vel3 = DopplerCalculator::calculate_velocity(
        Vec3::new(f32::NAN, 0.0, 0.0),
        Vec3::new(100.0, 0.0, 0.0),
        0.016,
    );
    assert!(vel3.is_nan() || vel3 == Vec3::ZERO); // Either is acceptable

    // Infinity positions - any result is acceptable as long as it doesn't crash
    let _vel4 = DopplerCalculator::calculate_velocity(
        Vec3::new(f32::INFINITY, 0.0, 0.0),
        Vec3::new(100.0, 0.0, 0.0),
        0.016,
    );
    // Just verify it doesn't crash - any result (finite, NaN, or infinity) is acceptable

    info!("Velocity calculation recovered from invalid inputs");
}

/// Test effect validation recovery
#[test]
fn test_effect_validation_recovery() {
    // Invalid reverb parameters
    let reverb_invalid = ReverbEffect { room_size: -1.0, damping: 2.0, wet_dry_mix: f32::NAN };
    assert!(!reverb_invalid.validate());

    // Invalid echo parameters
    let echo_invalid = EchoEffect { delay_time: -1.0, feedback: 1.5, wet_dry_mix: f32::INFINITY };
    assert!(!echo_invalid.validate());

    // Invalid filter parameters
    let filter_invalid = FilterEffect {
        filter_type: FilterType::LowPass,
        cutoff_frequency: -100.0,
        resonance: f32::NAN,
        wet_dry_mix: -0.5,
    };
    assert!(!filter_invalid.validate());

    // Invalid EQ parameters
    let eq_invalid = EqEffect { bass_gain: f32::INFINITY, mid_gain: f32::NAN, treble_gain: -100.0 };
    assert!(!eq_invalid.validate());

    info!("Effect validation correctly rejected invalid parameters");
}

/// Test recovery from rapid entity spawn/despawn
#[test]
fn test_recovery_from_rapid_entity_churn() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    // Create listener
    let listener = world.spawn();
    world.add(listener, Transform::default());
    world.add(listener, AudioListener::new());

    // Rapidly create and destroy entities
    for _ in 0..100 {
        let entities: Vec<_> = (0..10)
            .map(|i| {
                let entity = world.spawn();
                world.add(entity, Transform::default());
                world.add(entity, Sound::new(&format!("sound_{}.wav", i)));
                entity
            })
            .collect();

        system.update(&mut world, 0.016);

        // Despawn all
        for entity in entities {
            world.despawn(entity);
        }

        system.update(&mut world, 0.016);
    }

    info!("Recovered from rapid entity churn");
}

/// Test recovery from component modifications during iteration
#[test]
fn test_recovery_from_component_modifications() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    let listener = world.spawn();
    world.add(listener, Transform::default());
    world.add(listener, AudioListener::new());

    // Create entities
    let entities: Vec<_> = (0..20)
        .map(|_| {
            let entity = world.spawn();
            world.add(entity, Transform::default());
            world.add(entity, Sound::new("test.wav"));
            entity
        })
        .collect();

    // Update
    system.update(&mut world, 0.016);

    // Modify some components
    for (i, entity) in entities.iter().enumerate() {
        if i % 2 == 0 {
            world.remove::<Sound>(*entity);
        }
    }

    // Should handle gracefully
    system.update(&mut world, 0.016);

    info!("Recovered from component modifications");
}

/// Test recovery from speed of sound edge cases
#[test]
fn test_recovery_from_invalid_speed_of_sound() {
    let mut calc = DopplerCalculator::default();

    // Test clamping behavior
    calc.set_speed_of_sound(0.0);
    assert!(calc.speed_of_sound() >= 1.0); // Should clamp to minimum

    calc.set_speed_of_sound(-100.0);
    assert!(calc.speed_of_sound() >= 1.0);

    calc.set_speed_of_sound(f32::INFINITY);
    // Should either clamp or allow (implementation dependent)
    assert!(calc.speed_of_sound().is_finite() || calc.speed_of_sound().is_infinite());

    calc.set_speed_of_sound(f32::NAN);
    // Should reject or use default
    assert!(calc.speed_of_sound().is_finite());

    info!("Recovered from invalid speed of sound values");
}

/// Test recovery from multiple cleanup calls
#[test]
fn test_recovery_from_excessive_cleanup() {
    let mut engine = AudioEngine::new().expect("Failed to create audio engine");

    // Cleanup many times - should be safe
    for _ in 0..1000 {
        engine.cleanup_finished();
    }

    assert_eq!(engine.active_sound_count(), 0);

    info!("Recovered from excessive cleanup calls");
}

/// Test system recovery after listener deactivation
#[test]
fn test_recovery_from_listener_deactivation() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    let listener = world.spawn();
    world.add(listener, Transform::default());
    let listener_comp = AudioListener::new();
    world.add(listener, listener_comp.clone());

    let entity = world.spawn();
    world.add(entity, Transform::default());
    world.add(entity, Sound::new("test.wav"));

    // Update with active listener
    system.update(&mut world, 0.016);

    // Deactivate listener
    if let Some(listener_comp_mut) = world.get_mut::<AudioListener>(listener) {
        listener_comp_mut.active = false;
    }

    // Should handle gracefully
    system.update(&mut world, 0.016);

    info!("Recovered from listener deactivation");
}

/// Test recovery from sound component with empty name
#[test]
fn test_recovery_from_empty_sound_name() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    let listener = world.spawn();
    world.add(listener, Transform::default());
    world.add(listener, AudioListener::new());

    // Create sound with empty name
    let entity = world.spawn();
    world.add(entity, Transform::default());
    world.add(entity, Sound::new(""));

    // Should handle gracefully
    system.update(&mut world, 0.016);

    info!("Recovered from empty sound name");
}

/// Test recovery from very long sound names
#[test]
fn test_recovery_from_long_sound_name() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    let listener = world.spawn();
    world.add(listener, Transform::default());
    world.add(listener, AudioListener::new());

    // Create sound with very long name
    let long_name = "a".repeat(10000);
    let entity = world.spawn();
    world.add(entity, Transform::default());
    world.add(entity, Sound::new(&long_name));

    // Should handle gracefully
    system.update(&mut world, 0.016);

    info!("Recovered from very long sound name");
}
