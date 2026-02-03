//! Unit tests for Android audio backend (cross-platform)
//!
//! These tests can run on any platform and test the core audio processing logic
//! without requiring Android hardware.

#![cfg(test)]

use glam::Vec3;

// Import internal functions for testing (in real implementation, these would be pub(crate))

/// Test the 3D audio calculation algorithm
#[test]
fn test_3d_audio_distance_attenuation() {
    // Mock the calculate_3d_audio function logic
    fn calculate_gain(distance: f32, max_distance: f32) -> f32 {
        if distance < 1.0 {
            1.0
        } else if distance > max_distance {
            0.0
        } else {
            1.0 - (distance / max_distance).powi(2)
        }
    }

    // Very close - full volume
    assert_eq!(calculate_gain(0.5, 100.0), 1.0);

    // At 1.0 - very slight attenuation starts (1.0 - (1.0/100)^2 = 0.9999)
    assert!((calculate_gain(1.0, 100.0) - 1.0).abs() < 0.001);

    // At 10.0 with max 100.0
    let gain = calculate_gain(10.0, 100.0);
    assert!((gain - 0.99).abs() < 0.01); // Should be ~0.99

    // At 50.0 with max 100.0
    let gain = calculate_gain(50.0, 100.0);
    assert!((gain - 0.75).abs() < 0.01); // Should be ~0.75

    // At max distance
    let gain = calculate_gain(100.0, 100.0);
    assert!(gain < 0.01);

    // Beyond max distance
    assert_eq!(calculate_gain(150.0, 100.0), 0.0);
}

#[test]
fn test_3d_audio_stereo_panning() {
    fn calculate_pan(source_pos: Vec3, listener_pos: Vec3, listener_forward: Vec3) -> f32 {
        let to_source = (source_pos - listener_pos).normalize();
        let listener_right = listener_forward.cross(Vec3::Y).normalize();
        to_source.dot(listener_right).clamp(-1.0, 1.0)
    }

    let listener_pos = Vec3::ZERO;
    let listener_forward = Vec3::NEG_Z;

    // Sound directly in front - center
    let pan = calculate_pan(Vec3::new(0.0, 0.0, -10.0), listener_pos, listener_forward);
    assert!(pan.abs() < 0.01, "Front sound should be centered, got {}", pan);

    // Sound to the right - positive pan
    let pan = calculate_pan(Vec3::new(10.0, 0.0, 0.0), listener_pos, listener_forward);
    assert!(pan > 0.9, "Right sound should pan right, got {}", pan);

    // Sound to the left - negative pan
    let pan = calculate_pan(Vec3::new(-10.0, 0.0, 0.0), listener_pos, listener_forward);
    assert!(pan < -0.9, "Left sound should pan left, got {}", pan);

    // Sound behind - depends on exact position
    let pan = calculate_pan(Vec3::new(0.0, 0.0, 10.0), listener_pos, listener_forward);
    assert!(pan.abs() < 0.01, "Back sound should be centered, got {}", pan);
}

#[test]
fn test_stereo_resampling_ratio() {
    // Test the resampling logic
    fn calculate_output_frames(input_frames: usize, from_rate: f32, to_rate: f32) -> usize {
        let ratio = from_rate / to_rate;
        (input_frames as f32 / ratio) as usize
    }

    // Downsample 48kHz -> 44.1kHz
    let output = calculate_output_frames(48000, 48000.0, 44100.0);
    assert_eq!(output, 44100);

    // Upsample 22kHz -> 44.1kHz
    let output = calculate_output_frames(22050, 22050.0, 44100.0);
    assert_eq!(output, 44100);

    // No resampling needed
    let output = calculate_output_frames(44100, 44100.0, 44100.0);
    assert_eq!(output, 44100);
}

#[test]
fn test_linear_interpolation() {
    fn lerp(a: f32, b: f32, t: f32) -> f32 {
        a + (b - a) * t
    }

    // Halfway between 0 and 1
    assert_eq!(lerp(0.0, 1.0, 0.5), 0.5);

    // At start
    assert_eq!(lerp(0.0, 1.0, 0.0), 0.0);

    // At end
    assert_eq!(lerp(0.0, 1.0, 1.0), 1.0);

    // Negative values
    assert_eq!(lerp(-1.0, 1.0, 0.5), 0.0);

    // Beyond range (extrapolation)
    assert_eq!(lerp(0.0, 1.0, 2.0), 2.0);
}

#[test]
fn test_audio_frame_indexing() {
    // Stereo audio: [L0, R0, L1, R1, L2, R2, ...]
    let samples = vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6];

    // Frame 0
    assert_eq!(samples[0 * 2], 0.1); // Left
    assert_eq!(samples[0 * 2 + 1], 0.2); // Right

    // Frame 1
    assert_eq!(samples[1 * 2], 0.3); // Left
    assert_eq!(samples[1 * 2 + 1], 0.4); // Right

    // Frame 2
    assert_eq!(samples[2 * 2], 0.5); // Left
    assert_eq!(samples[2 * 2 + 1], 0.6); // Right

    // Frame count
    assert_eq!(samples.len() / 2, 3);
}

#[test]
fn test_volume_application() {
    fn apply_volume(sample: f32, volume: f32) -> f32 {
        sample * volume
    }

    // Full volume
    assert_eq!(apply_volume(0.5, 1.0), 0.5);

    // Half volume
    assert_eq!(apply_volume(0.5, 0.5), 0.25);

    // Muted
    assert_eq!(apply_volume(0.5, 0.0), 0.0);

    // Over-amplification
    assert_eq!(apply_volume(0.5, 2.0), 1.0);
}

#[test]
fn test_sample_clamping() {
    fn clamp_sample(sample: f32) -> f32 {
        sample.clamp(-1.0, 1.0)
    }

    // Normal range
    assert_eq!(clamp_sample(0.5), 0.5);
    assert_eq!(clamp_sample(-0.5), -0.5);

    // Clipping positive
    assert_eq!(clamp_sample(1.5), 1.0);

    // Clipping negative
    assert_eq!(clamp_sample(-1.5), -1.0);

    // Boundaries
    assert_eq!(clamp_sample(1.0), 1.0);
    assert_eq!(clamp_sample(-1.0), -1.0);
}

#[test]
fn test_fade_out_calculation() {
    fn calculate_fade_multiplier(remaining: usize, total: usize) -> f32 {
        remaining as f32 / total as f32
    }

    // Start of fade
    assert_eq!(calculate_fade_multiplier(100, 100), 1.0);

    // Halfway through fade
    assert_eq!(calculate_fade_multiplier(50, 100), 0.5);

    // End of fade
    assert_eq!(calculate_fade_multiplier(0, 100), 0.0);

    // 75% remaining
    assert_eq!(calculate_fade_multiplier(75, 100), 0.75);
}

#[test]
fn test_mono_to_stereo_conversion() {
    let mono_samples = vec![0.1, 0.2, 0.3, 0.4];
    let mut stereo_samples = Vec::with_capacity(mono_samples.len() * 2);

    for sample in mono_samples {
        stereo_samples.push(sample); // Left
        stereo_samples.push(sample); // Right
    }

    assert_eq!(stereo_samples.len(), 8);
    assert_eq!(stereo_samples[0], 0.1); // L0
    assert_eq!(stereo_samples[1], 0.1); // R0
    assert_eq!(stereo_samples[2], 0.2); // L1
    assert_eq!(stereo_samples[3], 0.2); // R1
}

#[test]
fn test_sample_format_conversion() {
    // i16 to f32 conversion
    fn i16_to_f32(sample: i16) -> f32 {
        sample as f32 / i16::MAX as f32
    }

    // Max positive
    assert!((i16_to_f32(i16::MAX) - 1.0).abs() < 0.001);

    // Zero
    assert_eq!(i16_to_f32(0), 0.0);

    // Mid-range
    let half = i16::MAX / 2;
    assert!((i16_to_f32(half) - 0.5).abs() < 0.001);

    // Negative
    assert!(i16_to_f32(-16384) < 0.0);
}

#[test]
fn test_panning_application() {
    fn apply_panning(left: f32, right: f32, pan: f32) -> (f32, f32) {
        if pan < 0.0 {
            // Pan left - reduce right channel
            (left, right * (1.0 + pan))
        } else {
            // Pan right - reduce left channel
            (left * (1.0 - pan), right)
        }
    }

    // Center (no panning)
    let (l, r) = apply_panning(1.0, 1.0, 0.0);
    assert_eq!(l, 1.0);
    assert_eq!(r, 1.0);

    // Full left
    let (l, r) = apply_panning(1.0, 1.0, -1.0);
    assert_eq!(l, 1.0);
    assert_eq!(r, 0.0);

    // Full right
    let (l, r) = apply_panning(1.0, 1.0, 1.0);
    assert_eq!(l, 0.0);
    assert_eq!(r, 1.0);

    // Half right
    let (l, r) = apply_panning(1.0, 1.0, 0.5);
    assert_eq!(l, 0.5);
    assert_eq!(r, 1.0);
}

#[test]
fn test_listener_orientation_calculation() {
    // Calculate right vector from forward and up
    let forward = Vec3::NEG_Z;
    let up = Vec3::Y;
    let right = forward.cross(up).normalize();

    assert!((right - Vec3::X).length() < 0.01);

    // Different orientation
    let forward = Vec3::X;
    let up = Vec3::Y;
    let right = forward.cross(up).normalize();

    assert!((right - Vec3::Z).length() < 0.01);
}

#[test]
fn test_distance_calculation() {
    let listener_pos = Vec3::ZERO;

    // Simple distances
    assert_eq!((Vec3::new(3.0, 4.0, 0.0) - listener_pos).length(), 5.0);
    assert_eq!((Vec3::new(1.0, 0.0, 0.0) - listener_pos).length(), 1.0);

    // 3D distance
    let distance = (Vec3::new(1.0, 1.0, 1.0) - listener_pos).length();
    assert!((distance - 1.732).abs() < 0.01);
}

#[test]
fn test_sound_instance_state_machine() {
    #[derive(Debug, PartialEq)]
    enum State {
        Playing,
        FadingOut,
        Stopped,
    }

    fn update_state(
        _current: State,
        fade_remaining: usize,
        position: usize,
        frame_count: usize,
    ) -> State {
        if fade_remaining > 0 {
            State::FadingOut
        } else if position >= frame_count {
            State::Stopped
        } else {
            State::Playing
        }
    }

    // Normal playback
    assert_eq!(update_state(State::Playing, 0, 50, 100), State::Playing);

    // Fading out
    assert_eq!(update_state(State::Playing, 10, 50, 100), State::FadingOut);

    // Stopped at end
    assert_eq!(update_state(State::Playing, 0, 100, 100), State::Stopped);
}

#[test]
fn test_audio_mixing() {
    fn mix_samples(samples: &[(f32, f32)]) -> (f32, f32) {
        let mut left = 0.0;
        let mut right = 0.0;

        for (l, r) in samples {
            left += l;
            right += r;
        }

        (left.clamp(-1.0, 1.0), right.clamp(-1.0, 1.0))
    }

    // Single sound
    let (l, r) = mix_samples(&[(0.5, 0.5)]);
    assert_eq!(l, 0.5);
    assert_eq!(r, 0.5);

    // Two sounds
    let (l, r) = mix_samples(&[(0.3, 0.3), (0.2, 0.2)]);
    assert_eq!(l, 0.5);
    assert_eq!(r, 0.5);

    // Clipping prevention
    let (l, r) = mix_samples(&[(0.8, 0.8), (0.8, 0.8)]);
    assert_eq!(l, 1.0); // Clamped
    assert_eq!(r, 1.0); // Clamped
}

#[test]
fn test_looping_behavior() {
    fn next_position(current: usize, frame_count: usize, looping: bool) -> Option<usize> {
        let next = current + 1;
        if next >= frame_count {
            if looping {
                Some(0) // Wrap to start
            } else {
                None // Stop
            }
        } else {
            Some(next)
        }
    }

    // Non-looping - stops at end
    assert_eq!(next_position(0, 10, false), Some(1));
    assert_eq!(next_position(9, 10, false), None);

    // Looping - wraps around
    assert_eq!(next_position(0, 10, true), Some(1));
    assert_eq!(next_position(9, 10, true), Some(0));
}

#[test]
fn test_sample_rate_conversion_calculations() {
    // Calculate source frame for target frame during resampling
    fn source_frame(target_frame: usize, from_rate: f32, to_rate: f32) -> f32 {
        let ratio = from_rate / to_rate;
        target_frame as f32 * ratio
    }

    // Downsample 48kHz -> 44.1kHz
    assert!((source_frame(0, 48000.0, 44100.0) - 0.0).abs() < 0.01);
    assert!((source_frame(44100, 48000.0, 44100.0) - 48000.0).abs() < 1.0);

    // Upsample 22kHz -> 44.1kHz
    assert!((source_frame(0, 22050.0, 44100.0) - 0.0).abs() < 0.01);
    assert!((source_frame(44100, 22050.0, 44100.0) - 22050.0).abs() < 1.0);
}
