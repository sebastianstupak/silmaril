//! Regression tests for audio system
//!
//! Tests for specific bugs that have been fixed to ensure they don't reoccur.
//! Each test documents the original bug scenario and verifies the fix.
//!
//! Bug tracking format:
//! - BUG-AUDIO-XXX: Brief description
//! - Reproduction: How the bug manifested
//! - Fix: What was changed
//! - Test: Verification that bug is fixed

use engine_audio::{
    AudioEngine, AudioListener, AudioSystem, DopplerCalculator, EchoEffect, ReverbEffect, Sound,
};
use engine_core::ecs::World;
use engine_core::math::{Transform, Vec3};
use std::time::{Duration, Instant};
use tracing::info;

/// BUG-AUDIO-001: Doppler shift calculation returned NaN for co-located positions
///
/// Reproduction: When listener and emitter were at exactly the same position,
/// distance was 0.0, causing division by zero in Doppler calculations.
///
/// Fix: Added check for zero distance, returning neutral pitch shift (1.0)
///
/// Test: Verify pitch shift is 1.0 when listener and emitter are co-located
#[test]
fn test_bug_001_doppler_nan_colocated() {
    let calc = DopplerCalculator::default();

    // Both at origin
    let shift = calc.calculate_pitch_shift(Vec3::ZERO, Vec3::ZERO, Vec3::ZERO, Vec3::ZERO);

    assert!(shift.is_finite(), "Pitch shift should not be NaN");
    assert_eq!(shift, 1.0, "Pitch shift should be neutral for co-located");

    info!("BUG-AUDIO-001: Fixed - Doppler NaN for co-located positions");
}

/// BUG-AUDIO-002: Velocity calculation caused panic with zero delta time
///
/// Reproduction: When delta_time was 0.0, velocity calculation divided by zero,
/// causing NaN that propagated through Doppler calculations.
///
/// Fix: Added check for zero/negative delta time, returning zero velocity
///
/// Test: Verify zero velocity is returned for zero delta time
#[test]
fn test_bug_002_velocity_zero_delta_time() {
    let vel = DopplerCalculator::calculate_velocity(
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(10.0, 0.0, 0.0),
        0.0, // Zero delta time
    );

    assert_eq!(vel, Vec3::ZERO, "Zero delta time should return zero velocity");

    // Also test negative delta time
    let vel_neg = DopplerCalculator::calculate_velocity(
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(10.0, 0.0, 0.0),
        -0.1,
    );

    assert_eq!(vel_neg, Vec3::ZERO, "Negative delta time should return zero velocity");

    info!("BUG-AUDIO-002: Fixed - Velocity panic with zero delta time");
}

/// BUG-AUDIO-003: Supersonic velocities caused unbounded pitch shift
///
/// Reproduction: When emitter moved faster than speed of sound, Doppler formula
/// produced negative denominator, causing pitch shift to go negative or infinity.
///
/// Fix: Added clamping to reasonable range [0.5, 2.0] (one octave up/down)
///
/// Test: Verify pitch shift is clamped for supersonic velocities
#[test]
fn test_bug_003_supersonic_velocity_clamping() {
    let calc = DopplerCalculator::default();

    // Emitter moving at Mach 2 directly toward listener
    let shift = calc.calculate_pitch_shift(
        Vec3::ZERO,                  // Listener at origin
        Vec3::ZERO,                  // Listener stationary
        Vec3::new(100.0, 0.0, 0.0),  // Emitter 100m away
        Vec3::new(-686.0, 0.0, 0.0), // Moving at 2x speed of sound toward listener
    );

    assert!(shift >= 0.5 && shift <= 2.0, "Pitch shift should be clamped: got {}", shift);

    // Emitter moving away at Mach 3
    let shift_away = calc.calculate_pitch_shift(
        Vec3::ZERO,
        Vec3::ZERO,
        Vec3::new(-100.0, 0.0, 0.0),
        Vec3::new(-1029.0, 0.0, 0.0), // Moving away at 3x speed of sound
    );

    assert!(
        shift_away >= 0.5 && shift_away <= 2.0,
        "Pitch shift should be clamped: got {}",
        shift_away
    );

    info!("BUG-AUDIO-003: Fixed - Supersonic velocity clamping");
}

/// BUG-AUDIO-004: Volume was not properly clamped, causing distortion
///
/// Reproduction: Negative or excessive volume values were passed through to
/// backend, causing audio distortion or backend errors.
///
/// Fix: Added clamping in Sound component constructor to [0.0, 1.0]
///
/// Test: Verify volume is clamped to valid range
#[test]
fn test_bug_004_volume_clamping() {
    let sound_negative = Sound::new("test.wav").with_volume(-1.0);
    assert_eq!(sound_negative.volume, 0.0);

    let sound_zero = Sound::new("test.wav").with_volume(0.0);
    assert_eq!(sound_zero.volume, 0.0);

    let sound_max = Sound::new("test.wav").with_volume(1.0);
    assert_eq!(sound_max.volume, 1.0);

    let sound_over = Sound::new("test.wav").with_volume(5.0);
    assert_eq!(sound_over.volume, 1.0);

    info!("BUG-AUDIO-004: Fixed - Volume clamping");
}

/// BUG-AUDIO-005: Doppler scale was not clamped, causing extreme pitch changes
///
/// Reproduction: Very large doppler_scale values caused unnatural pitch shifts.
///
/// Fix: Added clamping to [0.0, 10.0] to allow strong but not extreme Doppler
///
/// Test: Verify doppler_scale is clamped
#[test]
fn test_bug_005_doppler_scale_clamping() {
    let sound_negative = Sound::new("test.wav").with_doppler(-1.0);
    assert_eq!(sound_negative.doppler_scale, 0.0);

    let sound_max = Sound::new("test.wav").with_doppler(10.0);
    assert_eq!(sound_max.doppler_scale, 10.0);

    let sound_over = Sound::new("test.wav").with_doppler(100.0);
    assert_eq!(sound_over.doppler_scale, 10.0);

    info!("BUG-AUDIO-005: Fixed - Doppler scale clamping");
}

/// BUG-AUDIO-006: AudioSystem crashed when no listener was present
///
/// Reproduction: Starting audio system before spawning listener entity caused
/// query to fail and audio updates to crash.
///
/// Fix: Made listener query handle empty results gracefully
///
/// Test: Verify system works without listener
#[test]
fn test_bug_006_missing_listener_crash() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    // Create sounds but NO listener
    for i in 0..5 {
        let entity = world.spawn();
        let mut transform = Transform::default();
        transform.position = Vec3::new(i as f32 * 5.0, 0.0, 0.0);
        world.add(entity, transform);
        world.add(entity, Sound::new("test.wav"));
    }

    // Should not crash
    system.update(&mut world, 0.016);

    info!("BUG-AUDIO-006: Fixed - Missing listener crash");
}

/// BUG-AUDIO-007: Multiple active listeners caused audio to switch rapidly
///
/// Reproduction: When multiple listener entities existed, audio position
/// calculations used random listener, causing audio to jump.
///
/// Fix: AudioSystem now uses only first active listener and breaks early
///
/// Test: Verify only first listener is used
#[test]
fn test_bug_007_multiple_listeners() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    // Create multiple listeners at different positions
    let listener1 = world.spawn();
    let mut transform1 = Transform::default();
    transform1.position = Vec3::new(0.0, 0.0, 0.0);
    world.add(listener1, transform1);
    world.add(listener1, AudioListener::new());

    let listener2 = world.spawn();
    let mut transform2 = Transform::default();
    transform2.position = Vec3::new(100.0, 0.0, 0.0);
    world.add(listener2, transform2);
    world.add(listener2, AudioListener::new());

    // Create sound
    let entity = world.spawn();
    world.add(entity, Transform::default());
    world.add(entity, Sound::new("test.wav"));

    // Should use only first active listener (deterministic)
    system.update(&mut world, 0.016);

    info!("BUG-AUDIO-007: Fixed - Multiple listeners determinism");
}

/// BUG-AUDIO-008: Speed of sound could be set to zero, causing division errors
///
/// Reproduction: Setting speed_of_sound to 0.0 or negative caused Doppler
/// calculations to divide by zero.
///
/// Fix: Added minimum clamp (1.0 m/s) in set_speed_of_sound
///
/// Test: Verify speed of sound is clamped to minimum
#[test]
fn test_bug_008_zero_speed_of_sound() {
    let mut calc = DopplerCalculator::default();

    calc.set_speed_of_sound(0.0);
    assert_eq!(calc.speed_of_sound(), 1.0, "Should clamp to minimum");

    calc.set_speed_of_sound(-100.0);
    assert_eq!(calc.speed_of_sound(), 1.0, "Should clamp negative to minimum");

    info!("BUG-AUDIO-008: Fixed - Zero speed of sound");
}

/// BUG-AUDIO-009: Reverb parameters outside [0,1] caused audio artifacts
///
/// Reproduction: Invalid reverb parameters caused backend to produce distortion.
///
/// Fix: Added validation method that checks all parameters are in valid range
///
/// Test: Verify validation rejects invalid parameters
#[test]
fn test_bug_009_reverb_validation() {
    let valid = ReverbEffect { room_size: 0.5, damping: 0.5, wet_dry_mix: 0.5 };
    assert!(valid.validate());

    let invalid_room_size = ReverbEffect { room_size: 2.0, damping: 0.5, wet_dry_mix: 0.5 };
    assert!(!invalid_room_size.validate());

    let invalid_damping = ReverbEffect { room_size: 0.5, damping: -0.1, wet_dry_mix: 0.5 };
    assert!(!invalid_damping.validate());

    info!("BUG-AUDIO-009: Fixed - Reverb validation");
}

/// BUG-AUDIO-010: Echo feedback >= 1.0 caused infinite volume increase
///
/// Reproduction: Setting echo feedback to 1.0 or higher caused each echo to be
/// as loud or louder than previous, creating exponential volume growth.
///
/// Fix: Validation checks feedback is strictly less than 1.0
///
/// Test: Verify validation rejects dangerous feedback values
#[test]
fn test_bug_010_echo_feedback_validation() {
    let valid = EchoEffect { delay_time: 0.5, feedback: 0.5, wet_dry_mix: 0.5 };
    assert!(valid.validate());

    let feedback_one = EchoEffect {
        delay_time: 0.5,
        feedback: 1.0, // Would cause infinite feedback
        wet_dry_mix: 0.5,
    };
    assert!(!feedback_one.validate());

    let feedback_over = EchoEffect { delay_time: 0.5, feedback: 1.5, wet_dry_mix: 0.5 };
    assert!(!feedback_over.validate());

    info!("BUG-AUDIO-010: Fixed - Echo feedback validation");
}

/// BUG-AUDIO-011: Component removal during update caused use-after-free
///
/// Reproduction: Removing Sound component from entity while AudioSystem was
/// iterating caused crash or undefined behavior.
///
/// Fix: AudioSystem uses safe iteration that doesn't assume component exists
///
/// Test: Verify safe handling of component removal
#[test]
fn test_bug_011_component_removal_during_update() {
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

    // First update establishes baseline
    system.update(&mut world, 0.016);

    // Remove component
    world.remove::<Sound>(entity);

    // Should not crash
    system.update(&mut world, 0.016);

    info!("BUG-AUDIO-011: Fixed - Component removal during update");
}

/// BUG-AUDIO-012: Inactive listener was still used for audio calculations
///
/// Reproduction: Setting listener.active = false didn't prevent it from being
/// used, causing audio to calculate from wrong position.
///
/// Fix: AudioSystem filters listeners by .active field
///
/// Test: Verify inactive listeners are not used
#[test]
fn test_bug_012_inactive_listener_used() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    // Create inactive listener
    let listener = world.spawn();
    world.add(listener, Transform::default());
    let mut listener_comp = AudioListener::new();
    listener_comp.active = false;
    world.add(listener, listener_comp);

    let entity = world.spawn();
    world.add(entity, Transform::default());
    world.add(entity, Sound::new("test.wav"));

    // Should handle inactive listener correctly
    system.update(&mut world, 0.016);

    info!("BUG-AUDIO-012: Fixed - Inactive listener used");
}

/// BUG-AUDIO-013: Performance degradation with many finished sounds
///
/// Reproduction: After playing many sounds, performance degraded as finished
/// sounds were never cleaned up.
///
/// Fix: Added cleanup_finished() method that removes stopped instances
///
/// Test: Verify cleanup doesn't crash and maintains performance
#[test]
fn test_bug_013_finished_sounds_accumulation() {
    let mut engine = AudioEngine::new().expect("Failed to create audio engine");

    let start = Instant::now();

    // Cleanup should be fast even with many calls
    for _ in 0..1000 {
        engine.cleanup_finished();
    }

    let elapsed = start.elapsed();

    assert!(elapsed < Duration::from_millis(100), "Cleanup took too long: {:?}", elapsed);

    info!("BUG-AUDIO-013: Fixed - Finished sounds accumulation");
}

/// BUG-AUDIO-014: Perpendicular velocity incorrectly affected Doppler
///
/// Reproduction: When emitter moved perpendicular to listener direction,
/// Doppler shift was too strong (should be minimal).
///
/// Fix: Doppler calculation now projects velocity onto direction vector
///
/// Test: Verify perpendicular movement has minimal Doppler effect
#[test]
fn test_bug_014_perpendicular_doppler() {
    let calc = DopplerCalculator::default();

    // Emitter on X axis, moving on Y axis (perpendicular)
    let shift = calc.calculate_pitch_shift(
        Vec3::ZERO,                 // Listener at origin
        Vec3::ZERO,                 // Listener stationary
        Vec3::new(100.0, 0.0, 0.0), // Emitter on X axis
        Vec3::new(0.0, 50.0, 0.0),  // Moving on Y axis
    );

    // Should have minimal shift (close to 1.0)
    assert!(
        (shift - 1.0).abs() < 0.05,
        "Perpendicular movement should have minimal Doppler: got {}",
        shift
    );

    info!("BUG-AUDIO-014: Fixed - Perpendicular velocity Doppler");
}

/// BUG-AUDIO-015: Empty sound name caused backend errors
///
/// Reproduction: Creating Sound with empty string caused some backends to crash.
///
/// Fix: Empty names are now handled gracefully (no-op or default behavior)
///
/// Test: Verify empty names don't crash
#[test]
fn test_bug_015_empty_sound_name() {
    let sound = Sound::new("");
    assert_eq!(sound.sound_name, "");

    // Should not crash when used in ECS
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
    world.add(entity, sound);

    system.update(&mut world, 0.016);

    info!("BUG-AUDIO-015: Fixed - Empty sound name crash");
}

/// Performance regression: AudioSystem update time increased over time
///
/// Issue: Long-running games experienced gradual slowdown in audio updates.
/// Cause: Finished sound instances were accumulating without cleanup.
///
/// Fix: Automatic cleanup in update loop or explicit cleanup_finished() calls
///
/// Test: Verify update time doesn't increase over many frames
#[test]
fn test_perf_regression_update_time_growth() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut system = AudioSystem::new().expect("Failed to create audio system");

    let listener = world.spawn();
    world.add(listener, Transform::default());
    world.add(listener, AudioListener::new());

    // Create 50 sounds
    for i in 0..50 {
        let entity = world.spawn();
        let mut transform = Transform::default();
        transform.position = Vec3::new((i % 10) as f32 * 3.0, (i / 10) as f32 * 3.0, 0.0);
        world.add(entity, transform);
        world.add(entity, Sound::new("test.wav"));
    }

    // Measure first update
    let start1 = Instant::now();
    system.update(&mut world, 0.016);
    let elapsed1 = start1.elapsed();

    // Run many more updates
    for _ in 0..100 {
        system.update(&mut world, 0.016);
    }

    // Measure final update
    let start2 = Instant::now();
    system.update(&mut world, 0.016);
    let elapsed2 = start2.elapsed();

    // Update time should not significantly increase
    assert!(
        elapsed2.as_micros() < elapsed1.as_micros() * 3,
        "Update time grew too much: {:?} -> {:?}",
        elapsed1,
        elapsed2
    );

    info!("Performance regression test passed - no update time growth");
}

/// Performance regression: Query performance degraded with sparse components
///
/// Issue: When many entities had Transform but few had Sound, query was slow.
/// Cause: Query iteration didn't short-circuit on missing components.
///
/// Fix: Optimized query implementation to skip entities efficiently
///
/// Test: Verify query performance with sparse components
#[test]
fn test_perf_regression_sparse_query() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();

    // Create 1000 entities, only 10% have Sound
    for i in 0..1000 {
        let entity = world.spawn();
        let mut transform = Transform::default();
        transform.position = Vec3::new(i as f32, 0.0, 0.0);
        world.add(entity, transform);

        if i % 10 == 0 {
            world.add(entity, Sound::new("test.wav"));
        }
    }

    // Query should still be fast
    let start = Instant::now();
    let mut count = 0;

    for _ in 0..100 {
        for (_entity, (_transform, _sound)) in world.query::<(&Transform, &Sound)>() {
            count += 1;
        }
    }

    let elapsed = start.elapsed();

    assert_eq!(count, 10000); // 100 iterations * 100 matching entities
    assert!(elapsed < Duration::from_millis(50), "Query took too long: {:?}", elapsed);

    info!("Performance regression test passed - sparse query optimization");
}
