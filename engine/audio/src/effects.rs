//! Audio effects for sound processing
//!
//! This module provides audio effects that can be applied to sound instances:
//! - ReverbEffect: Simulates room acoustics
//! - EchoEffect: Delay-based echo effect
//! - FilterEffect: Low-pass, high-pass, and band-pass filtering
//! - EqEffect: 3-band equalizer (bass, mid, treble)
//!
//! # Example
//!
//! ```rust,no_run
//! use engine_audio::{AudioEngine, AudioEffect, ReverbEffect, FilterType};
//! use glam::Vec3;
//!
//! let mut audio = AudioEngine::new().unwrap();
//! audio.load_sound("gunshot", "assets/gunshot.wav").unwrap();
//!
//! let instance = audio.play_3d(
//!     1,
//!     "gunshot",
//!     Vec3::new(10.0, 0.0, 5.0),
//!     1.0,
//!     false,
//!     100.0,
//! ).unwrap();
//!
//! // Add reverb for indoor environment
//! let reverb = ReverbEffect {
//!     room_size: 0.8,
//!     damping: 0.5,
//!     wet_dry_mix: 0.3,
//! };
//! audio.add_effect(instance, AudioEffect::Reverb(reverb)).unwrap();
//! ```

use serde::{Deserialize, Serialize};

/// Audio effect types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AudioEffect {
    /// Reverb effect (room acoustics)
    Reverb(ReverbEffect),
    /// Echo/delay effect
    Echo(EchoEffect),
    /// Filter effect (low-pass, high-pass, band-pass)
    Filter(FilterEffect),
    /// 3-band equalizer
    Eq(EqEffect),
}

/// Reverb effect simulating room acoustics
///
/// Reverb adds depth and space to sounds by simulating sound reflections
/// in an enclosed space.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ReverbEffect {
    /// Room size (0.0 = tiny room, 1.0 = massive cathedral)
    ///
    /// Controls the size of the simulated space. Larger values create
    /// longer, more spacious reverb tails.
    pub room_size: f32,

    /// Damping (0.0 = no damping, 1.0 = maximum damping)
    ///
    /// Controls how quickly high frequencies decay. Higher damping
    /// simulates rooms with more sound-absorbing materials.
    pub damping: f32,

    /// Wet/dry mix (0.0 = all dry, 1.0 = all wet)
    ///
    /// Controls the blend between the original signal (dry) and
    /// the reverberated signal (wet).
    pub wet_dry_mix: f32,
}

impl Default for ReverbEffect {
    fn default() -> Self {
        Self { room_size: 0.5, damping: 0.5, wet_dry_mix: 0.2 }
    }
}

impl ReverbEffect {
    /// Create reverb for a small room
    pub fn small_room() -> Self {
        Self { room_size: 0.3, damping: 0.7, wet_dry_mix: 0.15 }
    }

    /// Create reverb for a large hall
    pub fn large_hall() -> Self {
        Self { room_size: 0.8, damping: 0.3, wet_dry_mix: 0.4 }
    }

    /// Create reverb for a cathedral
    pub fn cathedral() -> Self {
        Self { room_size: 1.0, damping: 0.2, wet_dry_mix: 0.5 }
    }

    /// Validate parameters are in valid range
    pub fn validate(&self) -> bool {
        (0.0..=1.0).contains(&self.room_size)
            && (0.0..=1.0).contains(&self.damping)
            && (0.0..=1.0).contains(&self.wet_dry_mix)
    }
}

/// Echo effect with configurable delay and feedback
///
/// Echo creates discrete repetitions of the sound, useful for
/// creating depth or special effects.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EchoEffect {
    /// Delay time in seconds (0.0 - 2.0)
    ///
    /// Time between each echo repetition. Typical values:
    /// - Short echo: 0.05 - 0.15s
    /// - Medium echo: 0.2 - 0.5s
    /// - Long echo: 0.5 - 2.0s
    pub delay_time: f32,

    /// Feedback amount (0.0 - 0.95)
    ///
    /// Controls how much of the delayed signal feeds back into itself.
    /// Higher values create more repetitions. Values >= 1.0 cause
    /// infinite feedback.
    pub feedback: f32,

    /// Wet/dry mix (0.0 = all dry, 1.0 = all wet)
    ///
    /// Controls the blend between the original signal and the echo.
    pub wet_dry_mix: f32,
}

impl Default for EchoEffect {
    fn default() -> Self {
        Self { delay_time: 0.3, feedback: 0.5, wet_dry_mix: 0.3 }
    }
}

impl EchoEffect {
    /// Create a short slapback echo
    pub fn slapback() -> Self {
        Self { delay_time: 0.08, feedback: 0.3, wet_dry_mix: 0.25 }
    }

    /// Create a long, spacious echo
    pub fn long_echo() -> Self {
        Self { delay_time: 0.75, feedback: 0.6, wet_dry_mix: 0.4 }
    }

    /// Validate parameters are in valid range
    pub fn validate(&self) -> bool {
        (0.0..=2.0).contains(&self.delay_time)
            && (0.0..=0.95).contains(&self.feedback)
            && (0.0..=1.0).contains(&self.wet_dry_mix)
    }
}

/// Filter type for FilterEffect
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilterType {
    /// Low-pass filter (removes high frequencies)
    LowPass,
    /// High-pass filter (removes low frequencies)
    HighPass,
    /// Band-pass filter (removes frequencies outside a range)
    BandPass,
}

/// Filter effect for frequency manipulation
///
/// Filters remove or attenuate specific frequency ranges:
/// - Low-pass: Makes sound muffled/dull (removes highs)
/// - High-pass: Makes sound thin/tinny (removes lows)
/// - Band-pass: Creates telephone/radio effect
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FilterEffect {
    /// Filter type
    pub filter_type: FilterType,

    /// Cutoff frequency in Hz (20.0 - 20000.0)
    ///
    /// For low-pass/high-pass: the frequency where attenuation begins
    /// For band-pass: the center frequency
    pub cutoff_frequency: f32,

    /// Resonance/Q factor (0.5 - 10.0)
    ///
    /// Controls the sharpness of the filter cutoff.
    /// Higher values create more pronounced filtering.
    pub resonance: f32,

    /// Wet/dry mix (0.0 = all dry, 1.0 = all wet)
    pub wet_dry_mix: f32,
}

impl Default for FilterEffect {
    fn default() -> Self {
        Self {
            filter_type: FilterType::LowPass,
            cutoff_frequency: 1000.0,
            resonance: 1.0,
            wet_dry_mix: 1.0,
        }
    }
}

impl FilterEffect {
    /// Create low-pass filter for muffled sound (underwater, through walls)
    pub fn muffled() -> Self {
        Self {
            filter_type: FilterType::LowPass,
            cutoff_frequency: 500.0,
            resonance: 0.7,
            wet_dry_mix: 1.0,
        }
    }

    /// Create high-pass filter for tinny sound (radio, telephone)
    pub fn tinny() -> Self {
        Self {
            filter_type: FilterType::HighPass,
            cutoff_frequency: 800.0,
            resonance: 1.2,
            wet_dry_mix: 1.0,
        }
    }

    /// Create band-pass filter for radio effect
    pub fn radio() -> Self {
        Self {
            filter_type: FilterType::BandPass,
            cutoff_frequency: 1200.0,
            resonance: 2.5,
            wet_dry_mix: 1.0,
        }
    }

    /// Validate parameters are in valid range
    pub fn validate(&self) -> bool {
        (20.0..=20000.0).contains(&self.cutoff_frequency)
            && (0.5..=10.0).contains(&self.resonance)
            && (0.0..=1.0).contains(&self.wet_dry_mix)
    }
}

/// 3-band equalizer effect
///
/// EQ allows independent control of bass, mid, and treble frequencies,
/// useful for:
/// - Enhancing weapon sounds (boost bass for impact)
/// - Making voices clearer (boost mids)
/// - Adding sparkle to UI sounds (boost treble)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EqEffect {
    /// Bass gain in dB (-20.0 to +20.0)
    ///
    /// Controls low frequencies (20 Hz - 250 Hz).
    /// Positive values boost bass, negative values reduce it.
    pub bass_gain: f32,

    /// Mid gain in dB (-20.0 to +20.0)
    ///
    /// Controls middle frequencies (250 Hz - 4000 Hz).
    /// Most important for voice clarity and instrument presence.
    pub mid_gain: f32,

    /// Treble gain in dB (-20.0 to +20.0)
    ///
    /// Controls high frequencies (4000 Hz - 20000 Hz).
    /// Affects clarity, brightness, and air.
    pub treble_gain: f32,
}

impl Default for EqEffect {
    fn default() -> Self {
        Self { bass_gain: 0.0, mid_gain: 0.0, treble_gain: 0.0 }
    }
}

impl EqEffect {
    /// Create EQ preset for enhanced bass (explosions, impacts)
    pub fn bass_boost() -> Self {
        Self { bass_gain: 6.0, mid_gain: 0.0, treble_gain: -2.0 }
    }

    /// Create EQ preset for clear voice
    pub fn voice_clarity() -> Self {
        Self { bass_gain: -3.0, mid_gain: 4.0, treble_gain: 2.0 }
    }

    /// Create EQ preset for bright UI sounds
    pub fn bright() -> Self {
        Self { bass_gain: -4.0, mid_gain: 1.0, treble_gain: 6.0 }
    }

    /// Validate parameters are in valid range
    pub fn validate(&self) -> bool {
        (-20.0..=20.0).contains(&self.bass_gain)
            && (-20.0..=20.0).contains(&self.mid_gain)
            && (-20.0..=20.0).contains(&self.treble_gain)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reverb_default() {
        let reverb = ReverbEffect::default();
        assert_eq!(reverb.room_size, 0.5);
        assert_eq!(reverb.damping, 0.5);
        assert_eq!(reverb.wet_dry_mix, 0.2);
        assert!(reverb.validate());
    }

    #[test]
    fn test_reverb_presets() {
        let small = ReverbEffect::small_room();
        assert!(small.room_size < 0.5);
        assert!(small.validate());

        let hall = ReverbEffect::large_hall();
        assert!(hall.room_size > 0.5);
        assert!(hall.validate());

        let cathedral = ReverbEffect::cathedral();
        assert_eq!(cathedral.room_size, 1.0);
        assert!(cathedral.validate());
    }

    #[test]
    fn test_reverb_validation() {
        let valid = ReverbEffect { room_size: 0.5, damping: 0.5, wet_dry_mix: 0.3 };
        assert!(valid.validate());

        let invalid = ReverbEffect { room_size: 1.5, damping: 0.5, wet_dry_mix: 0.3 };
        assert!(!invalid.validate());
    }

    #[test]
    fn test_echo_default() {
        let echo = EchoEffect::default();
        assert_eq!(echo.delay_time, 0.3);
        assert_eq!(echo.feedback, 0.5);
        assert_eq!(echo.wet_dry_mix, 0.3);
        assert!(echo.validate());
    }

    #[test]
    fn test_echo_presets() {
        let slapback = EchoEffect::slapback();
        assert!(slapback.delay_time < 0.15);
        assert!(slapback.validate());

        let long = EchoEffect::long_echo();
        assert!(long.delay_time > 0.5);
        assert!(long.validate());
    }

    #[test]
    fn test_echo_validation() {
        let valid = EchoEffect { delay_time: 0.5, feedback: 0.6, wet_dry_mix: 0.4 };
        assert!(valid.validate());

        let invalid_delay = EchoEffect { delay_time: 3.0, feedback: 0.5, wet_dry_mix: 0.3 };
        assert!(!invalid_delay.validate());

        let invalid_feedback = EchoEffect { delay_time: 0.5, feedback: 1.0, wet_dry_mix: 0.3 };
        assert!(!invalid_feedback.validate());
    }

    #[test]
    fn test_filter_default() {
        let filter = FilterEffect::default();
        assert_eq!(filter.filter_type, FilterType::LowPass);
        assert_eq!(filter.cutoff_frequency, 1000.0);
        assert!(filter.validate());
    }

    #[test]
    fn test_filter_presets() {
        let muffled = FilterEffect::muffled();
        assert_eq!(muffled.filter_type, FilterType::LowPass);
        assert!(muffled.cutoff_frequency < 1000.0);
        assert!(muffled.validate());

        let tinny = FilterEffect::tinny();
        assert_eq!(tinny.filter_type, FilterType::HighPass);
        assert!(tinny.validate());

        let radio = FilterEffect::radio();
        assert_eq!(radio.filter_type, FilterType::BandPass);
        assert!(radio.validate());
    }

    #[test]
    fn test_filter_validation() {
        let valid = FilterEffect {
            filter_type: FilterType::LowPass,
            cutoff_frequency: 1000.0,
            resonance: 1.5,
            wet_dry_mix: 0.8,
        };
        assert!(valid.validate());

        let invalid = FilterEffect {
            filter_type: FilterType::LowPass,
            cutoff_frequency: 30000.0,
            resonance: 1.0,
            wet_dry_mix: 0.8,
        };
        assert!(!invalid.validate());
    }

    #[test]
    fn test_eq_default() {
        let eq = EqEffect::default();
        assert_eq!(eq.bass_gain, 0.0);
        assert_eq!(eq.mid_gain, 0.0);
        assert_eq!(eq.treble_gain, 0.0);
        assert!(eq.validate());
    }

    #[test]
    fn test_eq_presets() {
        let bass_boost = EqEffect::bass_boost();
        assert!(bass_boost.bass_gain > 0.0);
        assert!(bass_boost.validate());

        let voice = EqEffect::voice_clarity();
        assert!(voice.mid_gain > 0.0);
        assert!(voice.validate());

        let bright = EqEffect::bright();
        assert!(bright.treble_gain > 0.0);
        assert!(bright.validate());
    }

    #[test]
    fn test_eq_validation() {
        let valid = EqEffect { bass_gain: 5.0, mid_gain: -3.0, treble_gain: 2.0 };
        assert!(valid.validate());

        let invalid = EqEffect { bass_gain: 25.0, mid_gain: 0.0, treble_gain: 0.0 };
        assert!(!invalid.validate());
    }

    #[test]
    fn test_audio_effect_enum() {
        let reverb = AudioEffect::Reverb(ReverbEffect::default());
        assert!(matches!(reverb, AudioEffect::Reverb(_)));

        let echo = AudioEffect::Echo(EchoEffect::default());
        assert!(matches!(echo, AudioEffect::Echo(_)));

        let filter = AudioEffect::Filter(FilterEffect::default());
        assert!(matches!(filter, AudioEffect::Filter(_)));

        let eq = AudioEffect::Eq(EqEffect::default());
        assert!(matches!(eq, AudioEffect::Eq(_)));
    }

    #[test]
    fn test_effect_serialization() {
        let reverb = ReverbEffect::default();
        let json = serde_json::to_string(&reverb).unwrap();
        let deserialized: ReverbEffect = serde_json::from_str(&json).unwrap();
        assert_eq!(reverb, deserialized);
    }
}
