//! Edge case tests for audio system
//!
//! Tests boundary conditions and unusual inputs:
//! - Extreme positions (very far, very close, negative)
//! - Extreme velocities (supersonic, near-zero, negative)
//! - Extreme volumes (0.0, 1.0, beyond limits)
//! - Invalid parameters (NaN, Infinity)
//! - Boundary conditions

use engine_audio::{
    AudioListener, AudioSystem, DopplerCalculator, EchoEffect, EqEffect, FilterEffect, FilterType,
    ReverbEffect, Sound, DEFAULT_SPEED_OF_SOUND,
};
use engine_core::ecs::World;
use engine_core::math::{Quat, Transform, Vec3};
use tracing::info;

/// Test extreme position values
#[test]
fn test_extreme_positions() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    // Listener at origin
    let listener = world.spawn();
    world.add(listener, Transform::default());
    world.add(listener, AudioListener::new());

    // Very close (near zero)
    let entity1 = world.spawn();
    let mut transform1 = Transform::default();
    transform1.position = Vec3::new(0.0001, 0.0001, 0.0001);
    world.add(entity1, transform1);
    world.add(entity1, Sound::new("test.wav").with_doppler(1.0));

    // Very far
    let entity2 = world.spawn();
    let mut transform2 = Transform::default();
    transform2.position = Vec3::new(1000000.0, 1000000.0, 1000000.0);
    world.add(entity2, transform2);
    world.add(entity2, Sound::new("test.wav").with_doppler(1.0));

    // Negative coordinates
    let entity3 = world.spawn();
    let mut transform3 = Transform::default();
    transform3.position = Vec3::new(-5000.0, -5000.0, -5000.0);
    world.add(entity3, transform3);
    world.add(entity3, Sound::new("test.wav").with_doppler(1.0));

    // Should not crash with extreme positions
    system.update(&mut world, 0.016);
    system.update(&mut world, 0.016); // Second update for velocity calculations

    info!("Handled extreme positions without crashing");
}

/// Test extreme velocity values
#[test]
fn test_extreme_velocities() {
    let calc = DopplerCalculator::default();

    // Very slow movement (near zero)
    let shift1 = calc.calculate_pitch_shift(
        Vec3::ZERO,
        Vec3::new(0.001, 0.0, 0.0),
        Vec3::new(100.0, 0.0, 0.0),
        Vec3::new(0.001, 0.0, 0.0),
    );
    assert!((shift1 - 1.0).abs() < 0.01);

    // Supersonic movement (faster than sound)
    let shift2 = calc.calculate_pitch_shift(
        Vec3::ZERO,
        Vec3::ZERO,
        Vec3::new(100.0, 0.0, 0.0),
        Vec3::new(-DEFAULT_SPEED_OF_SOUND * 2.0, 0.0, 0.0), // Mach 2
    );
    // Should be clamped to reasonable range
    assert!(shift2 >= 0.5 && shift2 <= 2.0);

    // Hypersonic movement
    let shift3 = calc.calculate_pitch_shift(
        Vec3::ZERO,
        Vec3::ZERO,
        Vec3::new(100.0, 0.0, 0.0),
        Vec3::new(-DEFAULT_SPEED_OF_SOUND * 10.0, 0.0, 0.0), // Mach 10
    );
    // Should still be clamped
    assert!(shift3 >= 0.5 && shift3 <= 2.0);

    // Moving away (source moving in same direction as distance vector)
    let shift4 = calc.calculate_pitch_shift(
        Vec3::ZERO,
        Vec3::ZERO,
        Vec3::new(-100.0, 0.0, 0.0), // Source on negative X
        Vec3::new(-100.0, 0.0, 0.0), // Moving further away (more negative)
    );
    assert!(shift4 < 1.0);

    info!("Handled extreme velocities correctly");
}

/// Test volume boundary conditions
#[test]
fn test_volume_boundaries() {
    // Test volume clamping
    let sound_min = Sound::new("test.wav").with_volume(-1.0);
    assert_eq!(sound_min.volume, 0.0);

    let sound_zero = Sound::new("test.wav").with_volume(0.0);
    assert_eq!(sound_zero.volume, 0.0);

    let sound_max = Sound::new("test.wav").with_volume(1.0);
    assert_eq!(sound_max.volume, 1.0);

    let sound_over = Sound::new("test.wav").with_volume(2.0);
    assert_eq!(sound_over.volume, 1.0);

    let sound_extreme = Sound::new("test.wav").with_volume(1000.0);
    assert_eq!(sound_extreme.volume, 1.0);

    info!("Volume clamping works correctly");
}

/// Test Doppler scale boundary conditions
#[test]
fn test_doppler_scale_boundaries() {
    // Test Doppler scale clamping
    let sound_negative = Sound::new("test.wav").with_doppler(-1.0);
    assert_eq!(sound_negative.doppler_scale, 0.0);

    let sound_zero = Sound::new("test.wav").with_doppler(0.0);
    assert_eq!(sound_zero.doppler_scale, 0.0);

    let sound_normal = Sound::new("test.wav").with_doppler(1.0);
    assert_eq!(sound_normal.doppler_scale, 1.0);

    let sound_high = Sound::new("test.wav").with_doppler(5.0);
    assert_eq!(sound_high.doppler_scale, 5.0);

    let sound_max = Sound::new("test.wav").with_doppler(10.0);
    assert_eq!(sound_max.doppler_scale, 10.0);

    let sound_over = Sound::new("test.wav").with_doppler(15.0);
    assert_eq!(sound_over.doppler_scale, 10.0);

    info!("Doppler scale clamping works correctly");
}

/// Test NaN and Infinity handling in Doppler calculations
#[test]
fn test_nan_infinity_handling() {
    let calc = DopplerCalculator::default();

    // Test with zero delta time (should return zero velocity)
    let vel = DopplerCalculator::calculate_velocity(
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(10.0, 0.0, 0.0),
        0.0,
    );
    assert_eq!(vel, Vec3::ZERO);

    // Test with negative delta time
    let vel2 = DopplerCalculator::calculate_velocity(
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(10.0, 0.0, 0.0),
        -0.1,
    );
    assert_eq!(vel2, Vec3::ZERO);

    // Test with co-located positions (division by zero protection)
    let shift = calc.calculate_pitch_shift(
        Vec3::ZERO,
        Vec3::new(100.0, 0.0, 0.0),
        Vec3::ZERO, // Same position as listener
        Vec3::new(-100.0, 0.0, 0.0),
    );
    assert_eq!(shift, 1.0); // Should return no shift

    // Test with very small distance
    let shift2 =
        calc.calculate_pitch_shift(Vec3::ZERO, Vec3::ZERO, Vec3::new(0.0001, 0.0, 0.0), Vec3::ZERO);
    assert!(shift2.is_finite());
    assert!(shift2 >= 0.5 && shift2 <= 2.0);

    info!("NaN and Infinity handling works correctly");
}

/// Test effect parameter boundaries
#[test]
fn test_effect_parameter_boundaries() {
    // Reverb boundaries
    let reverb_min = ReverbEffect { room_size: 0.0, damping: 0.0, wet_dry_mix: 0.0 };
    assert!(reverb_min.validate());

    let reverb_max = ReverbEffect { room_size: 1.0, damping: 1.0, wet_dry_mix: 1.0 };
    assert!(reverb_max.validate());

    let reverb_over = ReverbEffect { room_size: 2.0, damping: 2.0, wet_dry_mix: 2.0 };
    assert!(!reverb_over.validate());

    // Echo boundaries
    let echo_min = EchoEffect { delay_time: 0.0, feedback: 0.0, wet_dry_mix: 0.0 };
    assert!(echo_min.validate());

    let echo_max = EchoEffect { delay_time: 2.0, feedback: 0.95, wet_dry_mix: 1.0 };
    assert!(echo_max.validate());

    let echo_invalid_feedback = EchoEffect {
        delay_time: 0.5,
        feedback: 1.0, // Would cause infinite feedback
        wet_dry_mix: 0.5,
    };
    assert!(!echo_invalid_feedback.validate());

    // Filter boundaries
    let filter_min = FilterEffect {
        filter_type: FilterType::LowPass,
        cutoff_frequency: 20.0,
        resonance: 0.5,
        wet_dry_mix: 0.0,
    };
    assert!(filter_min.validate());

    let filter_max = FilterEffect {
        filter_type: FilterType::LowPass,
        cutoff_frequency: 20000.0,
        resonance: 10.0,
        wet_dry_mix: 1.0,
    };
    assert!(filter_max.validate());

    let filter_invalid_low = FilterEffect {
        filter_type: FilterType::LowPass,
        cutoff_frequency: 10.0, // Below 20 Hz
        resonance: 1.0,
        wet_dry_mix: 1.0,
    };
    assert!(!filter_invalid_low.validate());

    // EQ boundaries
    let eq_min = EqEffect { bass_gain: -20.0, mid_gain: -20.0, treble_gain: -20.0 };
    assert!(eq_min.validate());

    let eq_max = EqEffect { bass_gain: 20.0, mid_gain: 20.0, treble_gain: 20.0 };
    assert!(eq_max.validate());

    let eq_invalid = EqEffect { bass_gain: 30.0, mid_gain: 30.0, treble_gain: 30.0 };
    assert!(!eq_invalid.validate());

    info!("Effect parameter boundary validation works correctly");
}

/// Test listener orientation edge cases
#[test]
fn test_listener_orientation_edge_cases() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    // Test with various orientations
    let orientations = vec![
        Quat::IDENTITY,
        Quat::from_rotation_y(std::f32::consts::PI),
        Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
        Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
    ];

    for (i, rotation) in orientations.iter().enumerate() {
        let listener = world.spawn();
        let mut transform = Transform::default();
        transform.rotation = *rotation;
        world.add(listener, transform);
        world.add(listener, AudioListener::new());

        // Should not crash with any rotation
        system.update(&mut world, 0.016);

        // Cleanup for next test
        world.despawn(listener);

        info!(test_case = i, "Handled listener orientation");
    }
}

/// Test max distance boundary conditions
#[test]
fn test_max_distance_boundaries() {
    let sound_zero = Sound::new("test.wav").spatial_3d(0.0);
    assert_eq!(sound_zero.max_distance, 0.0);

    let sound_small = Sound::new("test.wav").spatial_3d(1.0);
    assert_eq!(sound_small.max_distance, 1.0);

    let sound_large = Sound::new("test.wav").spatial_3d(10000.0);
    assert_eq!(sound_large.max_distance, 10000.0);

    let sound_extreme = Sound::new("test.wav").spatial_3d(f32::MAX);
    assert_eq!(sound_extreme.max_distance, f32::MAX);

    info!("Max distance boundaries work correctly");
}

/// Test speed of sound boundary conditions
#[test]
fn test_speed_of_sound_boundaries() {
    let mut calc = DopplerCalculator::default();

    // Normal speed
    calc.set_speed_of_sound(343.0);
    assert_eq!(calc.speed_of_sound(), 343.0);

    // Very slow (should clamp to minimum)
    calc.set_speed_of_sound(0.0);
    assert_eq!(calc.speed_of_sound(), 1.0);

    calc.set_speed_of_sound(-100.0);
    assert_eq!(calc.speed_of_sound(), 1.0);

    // Very fast
    calc.set_speed_of_sound(10000.0);
    assert_eq!(calc.speed_of_sound(), 10000.0);

    info!("Speed of sound boundaries work correctly");
}

/// Test empty world updates
#[test]
fn test_empty_world() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    // Update with completely empty world - should not crash
    for _ in 0..100 {
        system.update(&mut world, 0.016);
    }

    info!("Empty world updates work correctly");
}

/// Test world with only listener (no sounds)
#[test]
fn test_listener_only() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    let listener = world.spawn();
    world.add(listener, Transform::default());
    world.add(listener, AudioListener::new());

    // Update with only listener - should not crash
    for _ in 0..100 {
        system.update(&mut world, 0.016);
    }

    info!("Listener-only world works correctly");
}

/// Test world with only sounds (no listener)
#[test]
fn test_sounds_only() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    // Create sounds without listener
    for i in 0..10 {
        let entity = world.spawn();
        let mut transform = Transform::default();
        transform.position = Vec3::new(i as f32 * 5.0, 0.0, 0.0);
        world.add(entity, transform);
        world.add(entity, Sound::new("test.wav"));
    }

    // Update without listener - should not crash
    for _ in 0..100 {
        system.update(&mut world, 0.016);
    }

    info!("Sound-only world works correctly");
}

/// Test inactive listeners
#[test]
fn test_inactive_listeners() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    // Create inactive listener
    let listener = world.spawn();
    world.add(listener, Transform::default());
    let mut listener_comp = AudioListener::new();
    listener_comp.active = false;
    world.add(listener, listener_comp);

    // Should not use inactive listener
    system.update(&mut world, 0.016);

    info!("Inactive listener handling works correctly");
}

/// Test very small delta time
#[test]
fn test_small_delta_time() {
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

    // Update with very small delta time
    system.update(&mut world, 0.0001);
    system.update(&mut world, 0.00001);
    system.update(&mut world, 0.000001);

    info!("Small delta time handling works correctly");
}

/// Test zero delta time
#[test]
fn test_zero_delta_time() {
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

    // Update with zero delta time - should not crash
    system.update(&mut world, 0.0);
    system.update(&mut world, 0.0);

    info!("Zero delta time handling works correctly");
}

/// Test large delta time
#[test]
fn test_large_delta_time() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    let listener = world.spawn();
    world.add(listener, Transform::default());
    world.add(listener, AudioListener::new());

    let entity = world.spawn();
    let mut transform = Transform::default();
    transform.position = Vec3::new(100.0, 0.0, 0.0);
    world.add(entity, transform);
    world.add(entity, Sound::new("test.wav").with_doppler(1.0));

    // Establish baseline
    system.update(&mut world, 0.016);

    // Update with large delta time (simulation pause/lag)
    system.update(&mut world, 1.0);
    system.update(&mut world, 10.0);

    info!("Large delta time handling works correctly");
}

/// Test sound name edge cases
#[test]
fn test_sound_name_edge_cases() {
    // Empty name
    let sound_empty = Sound::new("");
    assert_eq!(sound_empty.sound_name, "");

    // Very long name
    let long_name = "a".repeat(1000);
    let sound_long = Sound::new(&long_name);
    assert_eq!(sound_long.sound_name.len(), 1000);

    // Special characters
    let sound_special = Sound::new("sound_name.with-special/chars\\and:spaces test.wav");
    assert!(!sound_special.sound_name.is_empty());

    // Unicode
    let sound_unicode = Sound::new("音声ファイル.wav");
    assert!(!sound_unicode.sound_name.is_empty());

    info!("Sound name edge cases handled correctly");
}

/// Test component removal during update
#[test]
fn test_component_removal() {
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
    world.add(entity, Sound::new("test.wav"));

    // Update to establish baseline
    system.update(&mut world, 0.016);

    // Remove sound component
    world.remove::<Sound>(entity);

    // Should handle gracefully
    system.update(&mut world, 0.016);

    info!("Component removal handled correctly");
}

/// Test velocity calculation with same position
#[test]
fn test_velocity_same_position() {
    let vel = DopplerCalculator::calculate_velocity(
        Vec3::new(5.0, 5.0, 5.0),
        Vec3::new(5.0, 5.0, 5.0),
        0.016,
    );
    assert_eq!(vel, Vec3::ZERO);

    info!("Same position velocity calculation works correctly");
}

/// Test perpendicular velocity components
#[test]
fn test_perpendicular_velocity() {
    let calc = DopplerCalculator::default();

    // Emitter moving perpendicular to listener direction (should have minimal Doppler)
    let shift = calc.calculate_pitch_shift(
        Vec3::ZERO,                 // Listener at origin
        Vec3::ZERO,                 // Listener stationary
        Vec3::new(100.0, 0.0, 0.0), // Emitter on X axis
        Vec3::new(0.0, 50.0, 0.0),  // Emitter moving on Y axis (perpendicular)
    );

    // Should have minimal pitch shift (close to 1.0)
    assert!((shift - 1.0).abs() < 0.05);

    info!("Perpendicular velocity Doppler works correctly");
}
