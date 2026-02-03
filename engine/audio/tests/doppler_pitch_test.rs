//! Test for Doppler effect pitch adjustment integration
//!
//! Verifies that the AudioBackend trait's set_pitch method is correctly
//! integrated with the Doppler effect system.

use engine_audio::{AudioEngine, DopplerCalculator, DEFAULT_SPEED_OF_SOUND};
use glam::Vec3;

#[test]
fn test_set_pitch_api_exists() {
    let mut engine = AudioEngine::new().unwrap();

    // Verify that set_pitch method exists and can be called
    // We can't actually test playback without audio files,
    // but we can verify the API compiles
    let dummy_instance_id = 999;
    engine.set_pitch(dummy_instance_id, 1.0);

    // No crash = success
}

#[test]
fn test_doppler_calculator_pitch_shift() {
    let calc = DopplerCalculator::default();

    // Test approaching source (higher pitch)
    let approaching_shift = calc.calculate_pitch_shift(
        Vec3::ZERO,                 // listener pos
        Vec3::ZERO,                 // listener velocity
        Vec3::new(100.0, 0.0, 0.0), // emitter pos
        Vec3::new(-50.0, 0.0, 0.0), // emitter velocity (approaching)
    );

    assert!(approaching_shift > 1.0, "Approaching source should have pitch > 1.0");
    assert!(approaching_shift <= 2.0, "Pitch should be clamped to max 2.0");

    // Test receding source (lower pitch)
    let receding_shift = calc.calculate_pitch_shift(
        Vec3::ZERO,                 // listener pos
        Vec3::ZERO,                 // listener velocity
        Vec3::new(100.0, 0.0, 0.0), // emitter pos
        Vec3::new(50.0, 0.0, 0.0),  // emitter velocity (receding)
    );

    assert!(receding_shift < 1.0, "Receding source should have pitch < 1.0");
    assert!(receding_shift >= 0.5, "Pitch should be clamped to min 0.5");
}

#[test]
fn test_doppler_speed_of_sound_configuration() {
    // Test custom speed of sound (e.g., for underwater or space scenarios)
    let underwater_calc = DopplerCalculator::new(1500.0, 1.0);
    assert_eq!(underwater_calc.speed_of_sound(), 1500.0);

    let space_calc = DopplerCalculator::new(0.0, 0.0);
    // Should be clamped to minimum
    assert!(space_calc.speed_of_sound() >= 1.0);
}

#[test]
fn test_doppler_scale_factor() {
    let calc_full = DopplerCalculator::new(DEFAULT_SPEED_OF_SOUND, 1.0);
    let calc_half = DopplerCalculator::new(DEFAULT_SPEED_OF_SOUND, 0.5);
    let calc_disabled = DopplerCalculator::new(DEFAULT_SPEED_OF_SOUND, 0.0);

    let listener_pos = Vec3::ZERO;
    let listener_vel = Vec3::ZERO;
    let emitter_pos = Vec3::new(100.0, 0.0, 0.0);
    let emitter_vel = Vec3::new(-30.0, 0.0, 0.0);

    let shift_full =
        calc_full.calculate_pitch_shift(listener_pos, listener_vel, emitter_pos, emitter_vel);
    let shift_half =
        calc_half.calculate_pitch_shift(listener_pos, listener_vel, emitter_pos, emitter_vel);
    let shift_disabled =
        calc_disabled.calculate_pitch_shift(listener_pos, listener_vel, emitter_pos, emitter_vel);

    // Full scale should have largest effect
    assert!(shift_full > shift_half);

    // Half scale should be between full and no effect
    assert!(shift_half > 1.0);
    assert!(shift_half < shift_full);

    // Disabled should be 1.0 (no effect)
    assert_eq!(shift_disabled, 1.0);
}

#[test]
fn test_doppler_velocity_calculation() {
    // Test velocity calculation helper
    let old_pos = Vec3::new(0.0, 0.0, 0.0);
    let new_pos = Vec3::new(10.0, 0.0, 0.0);
    let delta_time = 0.1; // 100ms

    let velocity = DopplerCalculator::calculate_velocity(old_pos, new_pos, delta_time);

    // Should be 100 m/s (10m in 0.1s)
    assert_eq!(velocity.x, 100.0);
    assert_eq!(velocity.y, 0.0);
    assert_eq!(velocity.z, 0.0);
}

#[test]
fn test_doppler_performance_targets() {
    use std::time::Instant;

    let calc = DopplerCalculator::default();

    let listener_pos = Vec3::ZERO;
    let listener_vel = Vec3::new(10.0, 0.0, 0.0);
    let emitter_pos = Vec3::new(100.0, 0.0, 0.0);
    let emitter_vel = Vec3::new(-20.0, 0.0, 0.0);

    // Measure performance of 1000 calculations
    let start = Instant::now();
    for _ in 0..1000 {
        calc.calculate_pitch_shift(listener_pos, listener_vel, emitter_pos, emitter_vel);
    }
    let elapsed = start.elapsed();

    let per_calc = elapsed.as_micros() / 1000;

    // Should be < 50μs per calculation (target from requirements)
    println!("Doppler calculation time: {}μs per call", per_calc);
    assert!(per_calc < 50, "Doppler calculation should be < 50μs, got {}μs", per_calc);
}
