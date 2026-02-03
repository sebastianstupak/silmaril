//! Doppler effect accuracy tests
//!
//! Verifies the accuracy of Doppler calculations:
//! - Verify Doppler formula accuracy
//! - Test against known values
//! - High-speed scenarios
//! - Listener vs emitter movement
//! - Complex movement patterns

use engine_audio::{DopplerCalculator, DEFAULT_SPEED_OF_SOUND};
use engine_core::math::Vec3;
use tracing::info;

const EPSILON: f32 = 0.01; // Tolerance for floating-point comparisons

/// Test Doppler formula against known physics values
#[test]
fn test_doppler_formula_accuracy() {
    let calc = DopplerCalculator::default();

    // Known test case: Source approaching at 10% speed of sound
    // Expected: f' = f * (v / (v - vs)) = f * (343 / (343 - 34.3)) = f * 1.111
    let shift = calc.calculate_pitch_shift(
        Vec3::ZERO,
        Vec3::ZERO,
        Vec3::new(100.0, 0.0, 0.0),
        Vec3::new(-34.3, 0.0, 0.0), // Approaching at 34.3 m/s (10% of sound speed)
    );

    // Expected pitch shift: ~1.11 (11% higher) - allow some tolerance
    assert!(shift > 1.0 && shift < 1.2, "Expected around 1.11, got {}", shift);

    info!(actual = shift, "Doppler formula verified");
}

/// Test Doppler shift for receding source
#[test]
fn test_receding_source_accuracy() {
    let calc = DopplerCalculator::default();

    // Source receding at 10% speed of sound
    // Expected: f' = f * (v / (v + vs)) = f * (343 / (343 + 34.3)) = f * 0.909
    let shift = calc.calculate_pitch_shift(
        Vec3::ZERO,
        Vec3::ZERO,
        Vec3::new(100.0, 0.0, 0.0),
        Vec3::new(34.3, 0.0, 0.0), // Receding at 34.3 m/s
    );

    // Expected pitch shift: ~0.91 (9% lower) - allow some tolerance
    assert!(shift < 1.0 && shift > 0.85, "Expected around 0.91, got {}", shift);

    info!(actual = shift, "Receding source verified");
}

/// Test symmetry: approaching listener vs approaching source
#[test]
fn test_listener_source_symmetry() {
    let calc = DopplerCalculator::default();

    // Case 1: Source approaching listener
    let shift1 = calc.calculate_pitch_shift(
        Vec3::ZERO,
        Vec3::ZERO,
        Vec3::new(100.0, 0.0, 0.0),
        Vec3::new(-34.3, 0.0, 0.0),
    );

    // Case 2: Listener approaching source
    let shift2 = calc.calculate_pitch_shift(
        Vec3::ZERO,
        Vec3::new(34.3, 0.0, 0.0),
        Vec3::new(100.0, 0.0, 0.0),
        Vec3::ZERO,
    );

    // Should be similar (not exactly equal due to Doppler asymmetry)
    assert!(
        (shift1 - shift2).abs() < 0.05,
        "Shifts should be similar: {} vs {}",
        shift1,
        shift2
    );

    info!(
        source_approaching = shift1,
        listener_approaching = shift2,
        "Listener/source symmetry verified"
    );
}

/// Test zero velocity (no Doppler shift)
#[test]
fn test_zero_velocity() {
    let calc = DopplerCalculator::default();

    let shift =
        calc.calculate_pitch_shift(Vec3::ZERO, Vec3::ZERO, Vec3::new(100.0, 0.0, 0.0), Vec3::ZERO);

    assert!((shift - 1.0).abs() < EPSILON, "Zero velocity should give no shift");

    info!(shift = shift, "Zero velocity verified");
}

/// Test high-speed approach (50% speed of sound)
#[test]
fn test_high_speed_approach() {
    let calc = DopplerCalculator::default();

    // Source approaching at 50% speed of sound
    let velocity = DEFAULT_SPEED_OF_SOUND * 0.5;
    let shift = calc.calculate_pitch_shift(
        Vec3::ZERO,
        Vec3::ZERO,
        Vec3::new(100.0, 0.0, 0.0),
        Vec3::new(-velocity, 0.0, 0.0),
    );

    // Expected: f' = f * (343 / (343 - 171.5)) = f * 2.0
    // But it may be clamped to 2.0 max
    assert!(shift >= 1.8 && shift <= 2.0, "Expected around 2.0, got {}", shift);

    info!(velocity_mps = velocity, actual = shift, "High-speed approach verified");
}

/// Test supersonic speed (should be clamped)
#[test]
fn test_supersonic_clamping() {
    let calc = DopplerCalculator::default();

    // Source moving at Mach 2
    let velocity = DEFAULT_SPEED_OF_SOUND * 2.0;
    let shift = calc.calculate_pitch_shift(
        Vec3::ZERO,
        Vec3::ZERO,
        Vec3::new(100.0, 0.0, 0.0),
        Vec3::new(-velocity, 0.0, 0.0),
    );

    // Should be clamped to [0.5, 2.0]
    assert!(shift >= 0.5 && shift <= 2.0, "Shift should be clamped: {}", shift);

    info!(velocity_mach = 2.0, clamped_shift = shift, "Supersonic clamping verified");
}

/// Test perpendicular movement (no Doppler shift)
#[test]
fn test_perpendicular_movement() {
    let calc = DopplerCalculator::default();

    // Source moving perpendicular to listener direction
    let shift = calc.calculate_pitch_shift(
        Vec3::ZERO,
        Vec3::ZERO,
        Vec3::new(100.0, 0.0, 0.0),
        Vec3::new(0.0, 100.0, 0.0), // Moving perpendicular
    );

    assert!(
        (shift - 1.0).abs() < EPSILON,
        "Perpendicular movement should give minimal shift"
    );

    info!(shift = shift, "Perpendicular movement verified");
}

/// Test diagonal movement (partial Doppler shift)
#[test]
fn test_diagonal_movement() {
    let calc = DopplerCalculator::default();

    // Source moving at 45 degrees
    let velocity = 50.0;
    let shift = calc.calculate_pitch_shift(
        Vec3::ZERO,
        Vec3::ZERO,
        Vec3::new(100.0, 0.0, 0.0),
        Vec3::new(-velocity, velocity, 0.0), // 45 degree angle
    );

    // Diagonal should have reduced Doppler effect compared to direct approach
    // Only the radial component contributes to Doppler
    assert!(shift > 1.0, "Diagonal approach should still raise pitch: {}", shift);

    info!(diagonal_shift = shift, "Diagonal movement verified");
}

/// Test both listener and source moving
#[test]
fn test_both_moving_towards() {
    let calc = DopplerCalculator::default();

    let velocity = 25.0;

    // Both moving towards each other
    let shift = calc.calculate_pitch_shift(
        Vec3::ZERO,
        Vec3::new(velocity, 0.0, 0.0), // Listener moving towards source
        Vec3::new(100.0, 0.0, 0.0),
        Vec3::new(-velocity, 0.0, 0.0), // Source moving towards listener
    );

    // Should have larger shift than either alone
    let source_only_shift = calc.calculate_pitch_shift(
        Vec3::ZERO,
        Vec3::ZERO,
        Vec3::new(100.0, 0.0, 0.0),
        Vec3::new(-velocity, 0.0, 0.0),
    );

    assert!(
        shift > source_only_shift,
        "Both moving should give larger shift: {} vs {}",
        shift,
        source_only_shift
    );

    info!(
        both_moving = shift,
        source_only = source_only_shift,
        "Both moving towards verified"
    );
}

/// Test both moving apart
#[test]
fn test_both_moving_apart() {
    let calc = DopplerCalculator::default();

    let velocity = 25.0;

    // Both moving apart - source and listener velocities in opposite radial directions
    let shift = calc.calculate_pitch_shift(
        Vec3::ZERO,
        Vec3::new(0.0, 0.0, 0.0),       // Listener stationary for simplicity
        Vec3::new(-100.0, 0.0, 0.0),    // Source on negative X axis
        Vec3::new(-velocity, 0.0, 0.0), // Source moving further negative (away)
    );

    // Should be less than 1.0
    assert!(shift < 1.0, "Moving apart should lower pitch: {}", shift);

    info!(shift = shift, "Both moving apart verified");
}

/// Test Doppler scale factor
#[test]
fn test_doppler_scale_factor() {
    let calc_full = DopplerCalculator::new(DEFAULT_SPEED_OF_SOUND, 1.0);
    let calc_half = DopplerCalculator::new(DEFAULT_SPEED_OF_SOUND, 0.5);
    let calc_double = DopplerCalculator::new(DEFAULT_SPEED_OF_SOUND, 2.0);

    let listener_pos = Vec3::ZERO;
    let listener_vel = Vec3::ZERO;
    let emitter_pos = Vec3::new(100.0, 0.0, 0.0);
    let emitter_vel = Vec3::new(-50.0, 0.0, 0.0);

    let shift_full =
        calc_full.calculate_pitch_shift(listener_pos, listener_vel, emitter_pos, emitter_vel);

    let shift_half =
        calc_half.calculate_pitch_shift(listener_pos, listener_vel, emitter_pos, emitter_vel);

    let shift_double =
        calc_double.calculate_pitch_shift(listener_pos, listener_vel, emitter_pos, emitter_vel);

    // Half scale should be closer to 1.0 than full scale
    assert!(
        (shift_half - 1.0).abs() < (shift_full - 1.0).abs(),
        "Half scale should be smaller effect"
    );

    // Double scale should be further from 1.0 than full scale
    assert!(
        (shift_double - 1.0).abs() > (shift_full - 1.0).abs(),
        "Double scale should be larger effect"
    );

    info!(
        full_scale = shift_full,
        half_scale = shift_half,
        double_scale = shift_double,
        "Doppler scale factor verified"
    );
}

/// Test velocity calculation accuracy
#[test]
fn test_velocity_calculation_accuracy() {
    // Moving 10m in 0.1s should give 100 m/s
    let vel = DopplerCalculator::calculate_velocity(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0), 0.1);

    assert_eq!(vel, Vec3::new(100.0, 0.0, 0.0));

    // 3D movement
    let vel_3d = DopplerCalculator::calculate_velocity(
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(3.0, 4.0, 0.0),
        1.0,
    );

    assert_eq!(vel_3d, Vec3::new(3.0, 4.0, 0.0));
    assert!((vel_3d.length() - 5.0).abs() < EPSILON); // 3-4-5 triangle

    info!("Velocity calculation accuracy verified");
}

/// Test realistic car scenario
#[test]
fn test_realistic_car_doppler() {
    let calc = DopplerCalculator::default();

    // Car driving past at 50 m/s (~180 km/h, ~112 mph)
    let car_speed = 50.0;

    // Car approaching from 200m away
    let approaching = calc.calculate_pitch_shift(
        Vec3::ZERO,
        Vec3::ZERO,
        Vec3::new(200.0, 0.0, 0.0),
        Vec3::new(-car_speed, 0.0, 0.0),
    );

    // Car passing (perpendicular)
    let passing = calc.calculate_pitch_shift(
        Vec3::ZERO,
        Vec3::ZERO,
        Vec3::new(0.0, 10.0, 0.0),
        Vec3::new(car_speed, 0.0, 0.0),
    );

    // Car receding at 200m away
    let receding = calc.calculate_pitch_shift(
        Vec3::ZERO,
        Vec3::ZERO,
        Vec3::new(-200.0, 0.0, 0.0),
        Vec3::new(-car_speed, 0.0, 0.0),
    );

    // Approaching should be highest
    assert!(approaching > 1.0, "Approaching should raise pitch");

    // Passing should be near 1.0
    assert!((passing - 1.0).abs() < 0.05, "Passing should have minimal shift");

    // Receding should be lowest
    assert!(receding < 1.0, "Receding should lower pitch");

    // Approaching > passing > receding
    assert!(approaching > passing && passing > receding);

    info!(
        car_speed_kmh = car_speed * 3.6,
        approaching = approaching,
        passing = passing,
        receding = receding,
        "Realistic car Doppler verified"
    );
}

/// Test aircraft flyby
#[test]
fn test_aircraft_flyby() {
    let calc = DopplerCalculator::default();

    let aircraft_speed = 200.0; // 200 m/s (~720 km/h, subsonic)
    let altitude = 1000.0;

    // Aircraft far away, approaching
    let far_approaching = calc.calculate_pitch_shift(
        Vec3::ZERO,
        Vec3::ZERO,
        Vec3::new(5000.0, altitude, 0.0),
        Vec3::new(-aircraft_speed, 0.0, 0.0),
    );

    // Aircraft closer, approaching
    let near_approaching = calc.calculate_pitch_shift(
        Vec3::ZERO,
        Vec3::ZERO,
        Vec3::new(1000.0, altitude, 0.0),
        Vec3::new(-aircraft_speed, 0.0, 0.0),
    );

    // Aircraft directly overhead
    let overhead = calc.calculate_pitch_shift(
        Vec3::ZERO,
        Vec3::ZERO,
        Vec3::new(0.0, altitude, 0.0),
        Vec3::new(-aircraft_speed, 0.0, 0.0),
    );

    // Aircraft receding
    let receding = calc.calculate_pitch_shift(
        Vec3::ZERO,
        Vec3::ZERO,
        Vec3::new(-1000.0, altitude, 0.0),
        Vec3::new(-aircraft_speed, 0.0, 0.0),
    );

    // Verify shifts make sense
    assert!(near_approaching > 1.0, "Approaching should raise pitch");
    assert!(receding < 1.0, "Receding should lower pitch");

    // The relationship depends on angles, so just verify general trends
    info!(
        "Aircraft shifts - near: {}, far: {}, overhead: {}, receding: {}",
        near_approaching, far_approaching, overhead, receding
    );

    info!(
        aircraft_speed_kmh = aircraft_speed * 3.6,
        far_approaching = far_approaching,
        near_approaching = near_approaching,
        overhead = overhead,
        receding = receding,
        "Aircraft flyby verified"
    );
}

/// Test train passing scenario
#[test]
fn test_train_passing() {
    let calc = DopplerCalculator::default();

    let train_speed = 30.0; // 30 m/s (~108 km/h)

    // Train approaching
    let approaching = calc.calculate_pitch_shift(
        Vec3::ZERO,
        Vec3::ZERO,
        Vec3::new(500.0, 0.0, 0.0),
        Vec3::new(-train_speed, 0.0, 0.0),
    );

    // Train receding
    let receding = calc.calculate_pitch_shift(
        Vec3::ZERO,
        Vec3::ZERO,
        Vec3::new(-500.0, 0.0, 0.0),
        Vec3::new(-train_speed, 0.0, 0.0),
    );

    // Classic train whistle effect
    assert!(approaching > 1.0);
    assert!(receding < 1.0);

    let pitch_drop = approaching - receding;
    info!(
        train_speed_kmh = train_speed * 3.6,
        approaching = approaching,
        receding = receding,
        pitch_drop = pitch_drop,
        "Train passing verified"
    );
}

/// Test bullet passing scenario (very high speed)
#[test]
fn test_bullet_passing() {
    let calc = DopplerCalculator::default();

    let bullet_speed = 800.0; // 800 m/s (supersonic, Mach 2.3)

    // Bullet approaching
    let approaching = calc.calculate_pitch_shift(
        Vec3::ZERO,
        Vec3::ZERO,
        Vec3::new(100.0, 0.0, 0.0),
        Vec3::new(-bullet_speed, 0.0, 0.0),
    );

    // Bullet receding
    let receding = calc.calculate_pitch_shift(
        Vec3::ZERO,
        Vec3::ZERO,
        Vec3::new(-100.0, 0.0, 0.0),
        Vec3::new(-bullet_speed, 0.0, 0.0),
    );

    // Should be clamped
    assert!(approaching <= 2.0, "Should be clamped: {}", approaching);
    assert!(receding >= 0.5, "Should be clamped: {}", receding);

    info!(
        bullet_mach = bullet_speed / DEFAULT_SPEED_OF_SOUND,
        approaching = approaching,
        receding = receding,
        "Bullet passing verified (clamped)"
    );
}

/// Test co-located listener and source
#[test]
fn test_co_located() {
    let calc = DopplerCalculator::default();

    let shift = calc.calculate_pitch_shift(
        Vec3::ZERO,
        Vec3::new(100.0, 0.0, 0.0),
        Vec3::ZERO, // Same position
        Vec3::new(-100.0, 0.0, 0.0),
    );

    // Should return 1.0 to avoid division by zero
    assert_eq!(shift, 1.0);

    info!("Co-located listener/source handled correctly");
}

/// Test very small distance
#[test]
fn test_very_small_distance() {
    let calc = DopplerCalculator::default();

    let shift = calc.calculate_pitch_shift(
        Vec3::ZERO,
        Vec3::ZERO,
        Vec3::new(0.0001, 0.0, 0.0), // Very close
        Vec3::new(-10.0, 0.0, 0.0),
    );

    // Should handle gracefully (likely clamped to avoid extreme values)
    assert!(shift.is_finite());
    assert!(shift >= 0.5 && shift <= 2.0);

    info!(shift = shift, "Very small distance handled correctly");
}

/// Test different speed of sound
#[test]
fn test_different_speed_of_sound() {
    // Speed of sound in water (~1480 m/s)
    let calc_water = DopplerCalculator::new(1480.0, 1.0);

    // Same velocity should produce smaller shift in water
    let shift_air = DopplerCalculator::default().calculate_pitch_shift(
        Vec3::ZERO,
        Vec3::ZERO,
        Vec3::new(100.0, 0.0, 0.0),
        Vec3::new(-50.0, 0.0, 0.0),
    );

    let shift_water = calc_water.calculate_pitch_shift(
        Vec3::ZERO,
        Vec3::ZERO,
        Vec3::new(100.0, 0.0, 0.0),
        Vec3::new(-50.0, 0.0, 0.0),
    );

    // Water should have smaller shift (50 m/s is smaller fraction of 1480 m/s)
    assert!(
        (shift_water - 1.0).abs() < (shift_air - 1.0).abs(),
        "Water should have smaller shift: {} vs {}",
        shift_water,
        shift_air
    );

    info!(
        shift_air = shift_air,
        shift_water = shift_water,
        "Different speed of sound verified"
    );
}
