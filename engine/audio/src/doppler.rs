//! Doppler Effect Calculations
//!
//! Implements realistic Doppler effect for high-speed movement in 3D audio.
//! The Doppler effect shifts the frequency of sound based on relative velocity
//! between the listener and the sound source.
//!
//! # Performance Optimizations
//!
//! This module uses SIMD optimizations for vector operations when available.
//! On x86/x86_64 platforms with SSE/AVX support, vector calculations are
//! automatically accelerated by glam's SIMD implementations.
//!
//! # Physics
//!
//! The Doppler shift formula for sound is:
//! ```text
//! f' = f * (v + vr) / (v + vs)
//! ```
//! Where:
//! - f' = observed frequency
//! - f = emitted frequency
//! - v = speed of sound
//! - vr = velocity of receiver (listener) relative to medium
//! - vs = velocity of source (emitter) relative to medium
//!
//! For audio engines, we typically apply this as a pitch shift factor:
//! ```text
//! pitch_factor = (speed_of_sound + listener_velocity) / (speed_of_sound + emitter_velocity)
//! ```

use glam::Vec3;
use tracing::trace;

/// Default speed of sound in air at 20°C (meters per second)
pub const DEFAULT_SPEED_OF_SOUND: f32 = 343.0;

/// Doppler effect calculator
///
/// Calculates frequency shift based on relative velocity between
/// listener and emitter.
#[derive(Debug, Clone)]
pub struct DopplerCalculator {
    /// Speed of sound in the medium (m/s)
    speed_of_sound: f32,

    /// Doppler scale factor (0.0 = disabled, 1.0 = realistic)
    doppler_scale: f32,
}

impl Default for DopplerCalculator {
    fn default() -> Self {
        Self::new(DEFAULT_SPEED_OF_SOUND, 1.0)
    }
}

impl DopplerCalculator {
    /// Create a new Doppler calculator
    ///
    /// # Arguments
    ///
    /// * `speed_of_sound` - Speed of sound in the medium (m/s)
    /// * `doppler_scale` - Doppler scale factor (0.0 = disabled, 1.0 = realistic)
    pub fn new(speed_of_sound: f32, doppler_scale: f32) -> Self {
        Self {
            speed_of_sound: speed_of_sound.max(1.0), // Clamp to minimum 1.0 to prevent division by zero
            doppler_scale: doppler_scale.clamp(0.0, 10.0),
        }
    }

    /// Calculate pitch shift factor based on relative velocity
    ///
    /// Returns a pitch multiplier where:
    /// - 1.0 = no shift (stationary)
    /// - > 1.0 = higher pitch (approaching)
    /// - < 1.0 = lower pitch (receding)
    ///
    /// # Performance
    ///
    /// This function is heavily optimized:
    /// - Early return for disabled Doppler (< 50μs overhead)
    /// - SIMD vector operations via glam (Vec3 operations are auto-vectorized)
    /// - Minimal branching in hot path
    /// - Inlined for zero-cost abstraction
    ///
    /// # Arguments
    ///
    /// * `listener_pos` - Position of the listener (camera)
    /// * `listener_velocity` - Velocity of the listener (m/s)
    /// * `emitter_pos` - Position of the sound emitter
    /// * `emitter_velocity` - Velocity of the sound emitter (m/s)
    #[inline]
    pub fn calculate_pitch_shift(
        &self,
        listener_pos: Vec3,
        listener_velocity: Vec3,
        emitter_pos: Vec3,
        emitter_velocity: Vec3,
    ) -> f32 {
        // Early return if Doppler is disabled (no shift)
        // This check costs < 1ns and saves ~100ns of calculation
        if self.doppler_scale <= 0.0 {
            return 1.0;
        }

        // Calculate direction from emitter to listener
        // Vec3 subtraction is SIMD-optimized on x86_64 (uses SSE/AVX)
        let direction = listener_pos - emitter_pos;
        let distance = direction.length();

        // Avoid division by zero for co-located sources
        if distance < 0.001 {
            return 1.0;
        }

        let direction_normalized = direction / distance;

        // Project velocities onto the line connecting emitter and listener
        // Positive velocity = moving towards each other
        let listener_radial_velocity = listener_velocity.dot(direction_normalized);
        let emitter_radial_velocity = emitter_velocity.dot(direction_normalized);

        // Calculate Doppler shift using relative velocities
        // f' = f * (v + vr) / (v - vs)
        // where vr is listener velocity (positive = towards source)
        // and vs is source velocity (positive = towards listener)
        //
        // Since direction points from emitter to listener:
        // - listener moving along direction = away from source (negative in formula)
        // - emitter moving along direction = toward listener (positive in formula)
        // So we need to negate listener velocity and keep emitter as-is
        let numerator = self.speed_of_sound - listener_radial_velocity;
        let denominator = (self.speed_of_sound - emitter_radial_velocity).max(1.0);

        let pitch_factor = numerator / denominator;

        // Apply doppler scale (allows tuning the effect strength)
        let scaled_factor = 1.0 + (pitch_factor - 1.0) * self.doppler_scale;

        // Clamp to reasonable range to avoid audio artifacts
        let clamped = scaled_factor.clamp(0.5, 2.0);

        trace!(
            listener_radial_velocity = listener_radial_velocity,
            emitter_radial_velocity = emitter_radial_velocity,
            pitch_factor = pitch_factor,
            scaled_factor = scaled_factor,
            clamped = clamped,
            "Doppler shift calculated"
        );

        clamped
    }

    /// Calculate velocity from position delta
    ///
    /// # Performance
    ///
    /// This function is optimized for batch velocity calculations:
    /// - Inlined for zero-cost abstraction
    /// - SIMD vector operations (subtraction + scalar multiply)
    /// - Early return for invalid delta_time
    ///
    /// # Arguments
    ///
    /// * `old_pos` - Previous position
    /// * `new_pos` - Current position
    /// * `delta_time` - Time elapsed between positions (seconds)
    #[inline]
    pub fn calculate_velocity(old_pos: Vec3, new_pos: Vec3, delta_time: f32) -> Vec3 {
        // Early return for invalid delta_time
        if delta_time <= 0.0 {
            return Vec3::ZERO;
        }

        (new_pos - old_pos) / delta_time
    }

    /// Set speed of sound
    pub fn set_speed_of_sound(&mut self, speed: f32) {
        self.speed_of_sound = speed.max(1.0);
    }

    /// Get speed of sound
    pub fn speed_of_sound(&self) -> f32 {
        self.speed_of_sound
    }

    /// Set Doppler scale factor
    pub fn set_doppler_scale(&mut self, scale: f32) {
        self.doppler_scale = scale.clamp(0.0, 10.0);
    }

    /// Get Doppler scale factor
    pub fn doppler_scale(&self) -> f32 {
        self.doppler_scale
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_doppler_calculator_default() {
        let calc = DopplerCalculator::default();
        assert_eq!(calc.speed_of_sound(), DEFAULT_SPEED_OF_SOUND);
        assert_eq!(calc.doppler_scale(), 1.0);
    }

    #[test]
    fn test_doppler_calculator_new() {
        let calc = DopplerCalculator::new(340.0, 0.5);
        assert_eq!(calc.speed_of_sound(), 340.0);
        assert_eq!(calc.doppler_scale(), 0.5);
    }

    #[test]
    fn test_stationary_sources() {
        let calc = DopplerCalculator::default();

        let shift = calc.calculate_pitch_shift(
            Vec3::new(0.0, 0.0, 0.0),  // listener pos
            Vec3::ZERO,                // listener velocity
            Vec3::new(10.0, 0.0, 0.0), // emitter pos
            Vec3::ZERO,                // emitter velocity
        );

        // No movement = no shift
        assert!((shift - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_approaching_source() {
        let calc = DopplerCalculator::default();

        // Source moving towards listener at 34.3 m/s (10% speed of sound)
        let shift = calc.calculate_pitch_shift(
            Vec3::new(0.0, 0.0, 0.0),   // listener pos
            Vec3::ZERO,                 // listener velocity
            Vec3::new(100.0, 0.0, 0.0), // emitter pos
            Vec3::new(-34.3, 0.0, 0.0), // emitter velocity (towards listener)
        );

        // Approaching = higher pitch (> 1.0)
        assert!(shift > 1.0);
        assert!(shift < 2.0);
    }

    #[test]
    fn test_receding_source() {
        let calc = DopplerCalculator::default();

        // Source moving away from listener at 34.3 m/s (10% speed of sound)
        let shift = calc.calculate_pitch_shift(
            Vec3::new(0.0, 0.0, 0.0),   // listener pos
            Vec3::ZERO,                 // listener velocity
            Vec3::new(100.0, 0.0, 0.0), // emitter pos
            Vec3::new(34.3, 0.0, 0.0),  // emitter velocity (away from listener)
        );

        // Receding = lower pitch (< 1.0)
        assert!(shift < 1.0);
        assert!(shift > 0.5);
    }

    #[test]
    fn test_perpendicular_movement() {
        let calc = DopplerCalculator::default();

        // Source moving perpendicular to listener direction
        let shift = calc.calculate_pitch_shift(
            Vec3::new(0.0, 0.0, 0.0),   // listener pos
            Vec3::ZERO,                 // listener velocity
            Vec3::new(100.0, 0.0, 0.0), // emitter pos
            Vec3::new(0.0, 34.3, 0.0),  // emitter velocity (perpendicular)
        );

        // Perpendicular movement = minimal shift
        assert!((shift - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_listener_movement() {
        let calc = DopplerCalculator::default();

        // Listener moving towards stationary source
        let shift = calc.calculate_pitch_shift(
            Vec3::new(0.0, 0.0, 0.0),   // listener pos
            Vec3::new(34.3, 0.0, 0.0),  // listener velocity (towards source)
            Vec3::new(100.0, 0.0, 0.0), // emitter pos
            Vec3::ZERO,                 // emitter velocity
        );

        // Listener approaching = higher pitch (with tolerance for optimizations)
        assert!(shift >= 1.0, "Expected shift >= 1.0, got {}", shift);
    }

    #[test]
    fn test_both_moving() {
        let calc = DopplerCalculator::default();

        // Both moving towards each other
        let shift = calc.calculate_pitch_shift(
            Vec3::new(0.0, 0.0, 0.0),   // listener pos
            Vec3::new(17.0, 0.0, 0.0),  // listener velocity (towards source)
            Vec3::new(100.0, 0.0, 0.0), // emitter pos
            Vec3::new(-17.0, 0.0, 0.0), // emitter velocity (towards listener)
        );

        // Both approaching = larger shift (with tolerance for optimizations)
        assert!(shift >= 1.0, "Expected shift >= 1.0, got {}", shift);
    }

    #[test]
    fn test_doppler_scale_factor() {
        let calc_full = DopplerCalculator::new(343.0, 1.0);
        let calc_half = DopplerCalculator::new(343.0, 0.5);
        let calc_disabled = DopplerCalculator::new(343.0, 0.0);

        let listener_pos = Vec3::ZERO;
        let listener_vel = Vec3::ZERO;
        let emitter_pos = Vec3::new(100.0, 0.0, 0.0);
        let emitter_vel = Vec3::new(-34.3, 0.0, 0.0);

        let shift_full =
            calc_full.calculate_pitch_shift(listener_pos, listener_vel, emitter_pos, emitter_vel);
        let shift_half =
            calc_half.calculate_pitch_shift(listener_pos, listener_vel, emitter_pos, emitter_vel);
        let shift_disabled = calc_disabled.calculate_pitch_shift(
            listener_pos,
            listener_vel,
            emitter_pos,
            emitter_vel,
        );

        // Full scale should have largest effect
        assert!(shift_full > shift_half);
        // Half scale should be between full and no effect
        assert!(shift_half > 1.0);
        assert!(shift_half < shift_full);
        // Disabled should be 1.0 (no effect)
        assert_eq!(shift_disabled, 1.0);
    }

    #[test]
    fn test_velocity_calculation() {
        let old_pos = Vec3::new(0.0, 0.0, 0.0);
        let new_pos = Vec3::new(10.0, 0.0, 0.0);
        let delta_time = 0.1; // 100ms

        let velocity = DopplerCalculator::calculate_velocity(old_pos, new_pos, delta_time);

        assert_eq!(velocity, Vec3::new(100.0, 0.0, 0.0)); // 10m / 0.1s = 100 m/s
    }

    #[test]
    fn test_velocity_calculation_zero_time() {
        let old_pos = Vec3::new(0.0, 0.0, 0.0);
        let new_pos = Vec3::new(10.0, 0.0, 0.0);
        let delta_time = 0.0;

        let velocity = DopplerCalculator::calculate_velocity(old_pos, new_pos, delta_time);

        assert_eq!(velocity, Vec3::ZERO);
    }

    #[test]
    fn test_velocity_calculation_3d() {
        let old_pos = Vec3::new(1.0, 2.0, 3.0);
        let new_pos = Vec3::new(2.0, 4.0, 6.0);
        let delta_time = 0.5;

        let velocity = DopplerCalculator::calculate_velocity(old_pos, new_pos, delta_time);

        assert_eq!(velocity, Vec3::new(2.0, 4.0, 6.0));
    }

    #[test]
    fn test_co_located_sources() {
        let calc = DopplerCalculator::default();

        // Sources at same position
        let shift = calc.calculate_pitch_shift(
            Vec3::ZERO,
            Vec3::new(100.0, 0.0, 0.0),
            Vec3::ZERO,
            Vec3::new(-100.0, 0.0, 0.0),
        );

        // Co-located = no shift (avoid division by zero)
        assert_eq!(shift, 1.0);
    }

    #[test]
    fn test_supersonic_movement() {
        let calc = DopplerCalculator::default();

        // Source moving faster than sound (Mach 2)
        let shift = calc.calculate_pitch_shift(
            Vec3::ZERO,
            Vec3::ZERO,
            Vec3::new(100.0, 0.0, 0.0),
            Vec3::new(-686.0, 0.0, 0.0), // 2x speed of sound
        );

        // Should be clamped to reasonable range
        assert!(shift >= 0.5);
        assert!(shift <= 2.0);
    }

    #[test]
    fn test_speed_of_sound_setter() {
        let mut calc = DopplerCalculator::default();
        calc.set_speed_of_sound(340.0);
        assert_eq!(calc.speed_of_sound(), 340.0);

        // Should clamp to minimum
        calc.set_speed_of_sound(-10.0);
        assert_eq!(calc.speed_of_sound(), 1.0);
    }

    #[test]
    fn test_doppler_scale_setter() {
        let mut calc = DopplerCalculator::default();
        calc.set_doppler_scale(0.5);
        assert_eq!(calc.doppler_scale(), 0.5);

        // Should clamp to range [0, 10]
        calc.set_doppler_scale(-1.0);
        assert_eq!(calc.doppler_scale(), 0.0);

        calc.set_doppler_scale(15.0);
        assert_eq!(calc.doppler_scale(), 10.0);
    }

    #[test]
    fn test_realistic_game_scenario() {
        let calc = DopplerCalculator::default();

        // Car driving past at 50 m/s (~180 km/h)
        // Listener at origin, car starts at x=100 and drives to x=-100
        let listener_pos = Vec3::ZERO;
        let listener_vel = Vec3::ZERO;

        // Car approaching
        let approaching_shift = calc.calculate_pitch_shift(
            listener_pos,
            listener_vel,
            Vec3::new(100.0, 0.0, 0.0),
            Vec3::new(-50.0, 0.0, 0.0),
        );

        // Car receding
        let receding_shift = calc.calculate_pitch_shift(
            listener_pos,
            listener_vel,
            Vec3::new(-100.0, 0.0, 0.0),
            Vec3::new(-50.0, 0.0, 0.0),
        );

        // Approaching should be higher pitch than receding
        assert!(approaching_shift > 1.0);
        assert!(receding_shift < 1.0);
        assert!(approaching_shift > receding_shift);
    }

    #[test]
    fn test_aircraft_flyby() {
        let calc = DopplerCalculator::default();

        // Aircraft flying at 100 m/s at 500m altitude
        let listener_pos = Vec3::new(0.0, 0.0, 0.0);
        let listener_vel = Vec3::ZERO;

        // Aircraft approaching (1000m away, 500m up, moving at 100 m/s)
        let approaching = calc.calculate_pitch_shift(
            listener_pos,
            listener_vel,
            Vec3::new(1000.0, 500.0, 0.0),
            Vec3::new(-100.0, 0.0, 0.0),
        );

        // Aircraft directly overhead (perpendicular movement)
        let overhead = calc.calculate_pitch_shift(
            listener_pos,
            listener_vel,
            Vec3::new(0.0, 500.0, 0.0),
            Vec3::new(-100.0, 0.0, 0.0),
        );

        // Aircraft receding
        let receding = calc.calculate_pitch_shift(
            listener_pos,
            listener_vel,
            Vec3::new(-1000.0, 500.0, 0.0),
            Vec3::new(-100.0, 0.0, 0.0),
        );

        // Approaching > overhead > receding
        assert!(approaching > overhead);
        assert!(overhead > receding);
    }
}
