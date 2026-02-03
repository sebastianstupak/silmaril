//! Comprehensive tests for audio effects
//!
//! Tests cover:
//! - Effect creation and validation
//! - Effect parameter ranges
//! - Effect serialization/deserialization
//! - Effect presets
//! - Effect application to sound instances

use engine_audio::{
    AudioEffect, AudioEngine, EchoEffect, EqEffect, FilterEffect, FilterType, ReverbEffect,
};

#[test]
fn test_reverb_effect_creation() {
    let reverb = ReverbEffect::default();
    assert_eq!(reverb.room_size, 0.5);
    assert_eq!(reverb.damping, 0.5);
    assert_eq!(reverb.wet_dry_mix, 0.2);
}

#[test]
fn test_reverb_effect_validation() {
    // Valid reverb
    let valid = ReverbEffect { room_size: 0.7, damping: 0.4, wet_dry_mix: 0.3 };
    assert!(valid.validate());

    // Invalid room_size (> 1.0)
    let invalid_room = ReverbEffect { room_size: 1.5, damping: 0.5, wet_dry_mix: 0.3 };
    assert!(!invalid_room.validate());

    // Invalid damping (< 0.0)
    let invalid_damping = ReverbEffect { room_size: 0.5, damping: -0.1, wet_dry_mix: 0.3 };
    assert!(!invalid_damping.validate());

    // Invalid wet_dry_mix (> 1.0)
    let invalid_mix = ReverbEffect { room_size: 0.5, damping: 0.5, wet_dry_mix: 1.5 };
    assert!(!invalid_mix.validate());
}

#[test]
fn test_reverb_presets() {
    let small = ReverbEffect::small_room();
    assert!(small.validate());
    assert!(small.room_size < 0.5);
    assert!(small.damping > 0.5);

    let hall = ReverbEffect::large_hall();
    assert!(hall.validate());
    assert!(hall.room_size > 0.5);
    assert!(hall.wet_dry_mix > 0.3);

    let cathedral = ReverbEffect::cathedral();
    assert!(cathedral.validate());
    assert_eq!(cathedral.room_size, 1.0);
    assert!(cathedral.wet_dry_mix >= 0.5);
}

#[test]
fn test_echo_effect_creation() {
    let echo = EchoEffect::default();
    assert_eq!(echo.delay_time, 0.3);
    assert_eq!(echo.feedback, 0.5);
    assert_eq!(echo.wet_dry_mix, 0.3);
}

#[test]
fn test_echo_effect_validation() {
    // Valid echo
    let valid = EchoEffect { delay_time: 0.5, feedback: 0.6, wet_dry_mix: 0.4 };
    assert!(valid.validate());

    // Invalid delay_time (> 2.0)
    let invalid_delay = EchoEffect { delay_time: 3.0, feedback: 0.5, wet_dry_mix: 0.3 };
    assert!(!invalid_delay.validate());

    // Invalid feedback (>= 1.0, would cause infinite feedback)
    let invalid_feedback = EchoEffect { delay_time: 0.5, feedback: 1.0, wet_dry_mix: 0.3 };
    assert!(!invalid_feedback.validate());

    // Invalid wet_dry_mix
    let invalid_mix = EchoEffect { delay_time: 0.5, feedback: 0.5, wet_dry_mix: -0.1 };
    assert!(!invalid_mix.validate());
}

#[test]
fn test_echo_presets() {
    let slapback = EchoEffect::slapback();
    assert!(slapback.validate());
    assert!(slapback.delay_time < 0.15);
    assert!(slapback.feedback < 0.5);

    let long = EchoEffect::long_echo();
    assert!(long.validate());
    assert!(long.delay_time > 0.5);
    assert!(long.feedback > 0.5);
}

#[test]
fn test_filter_effect_creation() {
    let filter = FilterEffect::default();
    assert_eq!(filter.filter_type, FilterType::LowPass);
    assert_eq!(filter.cutoff_frequency, 1000.0);
    assert_eq!(filter.resonance, 1.0);
}

#[test]
fn test_filter_effect_validation() {
    // Valid filter
    let valid = FilterEffect {
        filter_type: FilterType::HighPass,
        cutoff_frequency: 2000.0,
        resonance: 2.0,
        wet_dry_mix: 0.8,
    };
    assert!(valid.validate());

    // Invalid cutoff (< 20 Hz)
    let invalid_cutoff_low = FilterEffect {
        filter_type: FilterType::LowPass,
        cutoff_frequency: 10.0,
        resonance: 1.0,
        wet_dry_mix: 1.0,
    };
    assert!(!invalid_cutoff_low.validate());

    // Invalid cutoff (> 20000 Hz)
    let invalid_cutoff_high = FilterEffect {
        filter_type: FilterType::LowPass,
        cutoff_frequency: 25000.0,
        resonance: 1.0,
        wet_dry_mix: 1.0,
    };
    assert!(!invalid_cutoff_high.validate());

    // Invalid resonance (< 0.5)
    let invalid_resonance = FilterEffect {
        filter_type: FilterType::BandPass,
        cutoff_frequency: 1000.0,
        resonance: 0.3,
        wet_dry_mix: 1.0,
    };
    assert!(!invalid_resonance.validate());
}

#[test]
fn test_filter_presets() {
    let muffled = FilterEffect::muffled();
    assert!(muffled.validate());
    assert_eq!(muffled.filter_type, FilterType::LowPass);
    assert!(muffled.cutoff_frequency < 1000.0);

    let tinny = FilterEffect::tinny();
    assert!(tinny.validate());
    assert_eq!(tinny.filter_type, FilterType::HighPass);

    let radio = FilterEffect::radio();
    assert!(radio.validate());
    assert_eq!(radio.filter_type, FilterType::BandPass);
}

#[test]
fn test_filter_types() {
    assert_eq!(FilterType::LowPass, FilterType::LowPass);
    assert_ne!(FilterType::LowPass, FilterType::HighPass);
    assert_ne!(FilterType::HighPass, FilterType::BandPass);
}

#[test]
fn test_eq_effect_creation() {
    let eq = EqEffect::default();
    assert_eq!(eq.bass_gain, 0.0);
    assert_eq!(eq.mid_gain, 0.0);
    assert_eq!(eq.treble_gain, 0.0);
}

#[test]
fn test_eq_effect_validation() {
    // Valid EQ
    let valid = EqEffect { bass_gain: 5.0, mid_gain: -3.0, treble_gain: 2.0 };
    assert!(valid.validate());

    // Invalid bass_gain (> 20 dB)
    let invalid_bass = EqEffect { bass_gain: 25.0, mid_gain: 0.0, treble_gain: 0.0 };
    assert!(!invalid_bass.validate());

    // Invalid mid_gain (< -20 dB)
    let invalid_mid = EqEffect { bass_gain: 0.0, mid_gain: -25.0, treble_gain: 0.0 };
    assert!(!invalid_mid.validate());

    // Invalid treble_gain
    let invalid_treble = EqEffect { bass_gain: 0.0, mid_gain: 0.0, treble_gain: 30.0 };
    assert!(!invalid_treble.validate());
}

#[test]
fn test_eq_presets() {
    let bass_boost = EqEffect::bass_boost();
    assert!(bass_boost.validate());
    assert!(bass_boost.bass_gain > 0.0);

    let voice = EqEffect::voice_clarity();
    assert!(voice.validate());
    assert!(voice.mid_gain > 0.0);

    let bright = EqEffect::bright();
    assert!(bright.validate());
    assert!(bright.treble_gain > 0.0);
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
fn test_effect_serialization_reverb() {
    let reverb = ReverbEffect { room_size: 0.8, damping: 0.6, wet_dry_mix: 0.4 };

    let json = serde_json::to_string(&reverb).unwrap();
    let deserialized: ReverbEffect = serde_json::from_str(&json).unwrap();

    assert_eq!(reverb, deserialized);
}

#[test]
fn test_effect_serialization_echo() {
    let echo = EchoEffect { delay_time: 0.75, feedback: 0.65, wet_dry_mix: 0.35 };

    let json = serde_json::to_string(&echo).unwrap();
    let deserialized: EchoEffect = serde_json::from_str(&json).unwrap();

    assert_eq!(echo, deserialized);
}

#[test]
fn test_effect_serialization_filter() {
    let filter = FilterEffect {
        filter_type: FilterType::BandPass,
        cutoff_frequency: 1500.0,
        resonance: 2.5,
        wet_dry_mix: 0.9,
    };

    let json = serde_json::to_string(&filter).unwrap();
    let deserialized: FilterEffect = serde_json::from_str(&json).unwrap();

    assert_eq!(filter, deserialized);
}

#[test]
fn test_effect_serialization_eq() {
    let eq = EqEffect { bass_gain: 6.0, mid_gain: -3.0, treble_gain: 4.0 };

    let json = serde_json::to_string(&eq).unwrap();
    let deserialized: EqEffect = serde_json::from_str(&json).unwrap();

    assert_eq!(eq, deserialized);
}

#[test]
fn test_audio_effect_enum_serialization() {
    let effects = vec![
        AudioEffect::Reverb(ReverbEffect::large_hall()),
        AudioEffect::Echo(EchoEffect::slapback()),
        AudioEffect::Filter(FilterEffect::muffled()),
        AudioEffect::Eq(EqEffect::bass_boost()),
    ];

    for effect in effects {
        let json = serde_json::to_string(&effect).unwrap();
        let deserialized: AudioEffect = serde_json::from_str(&json).unwrap();

        match (&effect, &deserialized) {
            (AudioEffect::Reverb(a), AudioEffect::Reverb(b)) => assert_eq!(a, b),
            (AudioEffect::Echo(a), AudioEffect::Echo(b)) => assert_eq!(a, b),
            (AudioEffect::Filter(a), AudioEffect::Filter(b)) => assert_eq!(a, b),
            (AudioEffect::Eq(a), AudioEffect::Eq(b)) => assert_eq!(a, b),
            _ => panic!("Mismatched effect types after deserialization"),
        }
    }
}

#[test]
fn test_engine_effect_api() {
    // Test that the AudioEngine has the effect methods
    let engine = AudioEngine::new();
    assert!(engine.is_ok(), "AudioEngine should initialize");

    let engine = engine.unwrap();

    // Test effect count on non-existent instance
    let count = engine.effect_count(999);
    assert_eq!(count, 0, "Non-existent instance should have 0 effects");
}

#[test]
fn test_multiple_effects_stacking() {
    // Create multiple effects to test stacking
    let reverb = AudioEffect::Reverb(ReverbEffect::small_room());
    let echo = AudioEffect::Echo(EchoEffect::slapback());
    let filter = AudioEffect::Filter(FilterEffect::muffled());

    // Verify each effect is valid
    match &reverb {
        AudioEffect::Reverb(r) => assert!(r.validate()),
        _ => panic!("Expected reverb"),
    }

    match &echo {
        AudioEffect::Echo(e) => assert!(e.validate()),
        _ => panic!("Expected echo"),
    }

    match &filter {
        AudioEffect::Filter(f) => assert!(f.validate()),
        _ => panic!("Expected filter"),
    }
}

#[test]
fn test_effect_parameter_edge_cases() {
    // Test reverb at boundaries
    let min_reverb = ReverbEffect { room_size: 0.0, damping: 0.0, wet_dry_mix: 0.0 };
    assert!(min_reverb.validate());

    let max_reverb = ReverbEffect { room_size: 1.0, damping: 1.0, wet_dry_mix: 1.0 };
    assert!(max_reverb.validate());

    // Test echo at boundaries
    let min_echo = EchoEffect { delay_time: 0.0, feedback: 0.0, wet_dry_mix: 0.0 };
    assert!(min_echo.validate());

    let max_echo = EchoEffect { delay_time: 2.0, feedback: 0.95, wet_dry_mix: 1.0 };
    assert!(max_echo.validate());

    // Test filter at boundaries
    let min_filter = FilterEffect {
        filter_type: FilterType::LowPass,
        cutoff_frequency: 20.0,
        resonance: 0.5,
        wet_dry_mix: 0.0,
    };
    assert!(min_filter.validate());

    let max_filter = FilterEffect {
        filter_type: FilterType::LowPass,
        cutoff_frequency: 20000.0,
        resonance: 10.0,
        wet_dry_mix: 1.0,
    };
    assert!(max_filter.validate());

    // Test EQ at boundaries
    let min_eq = EqEffect { bass_gain: -20.0, mid_gain: -20.0, treble_gain: -20.0 };
    assert!(min_eq.validate());

    let max_eq = EqEffect { bass_gain: 20.0, mid_gain: 20.0, treble_gain: 20.0 };
    assert!(max_eq.validate());
}

#[test]
fn test_effect_clone() {
    let original = ReverbEffect::cathedral();
    let cloned = original;

    assert_eq!(original, cloned);
    assert_eq!(original.room_size, cloned.room_size);
    assert_eq!(original.damping, cloned.damping);
    assert_eq!(original.wet_dry_mix, cloned.wet_dry_mix);
}

#[test]
fn test_effect_debug_output() {
    let reverb = ReverbEffect::default();
    let debug_str = format!("{:?}", reverb);
    assert!(debug_str.contains("ReverbEffect"));
    assert!(debug_str.contains("room_size"));

    let echo = EchoEffect::default();
    let debug_str = format!("{:?}", echo);
    assert!(debug_str.contains("EchoEffect"));
    assert!(debug_str.contains("delay_time"));

    let filter = FilterEffect::default();
    let debug_str = format!("{:?}", filter);
    assert!(debug_str.contains("FilterEffect"));
    assert!(debug_str.contains("cutoff_frequency"));

    let eq = EqEffect::default();
    let debug_str = format!("{:?}", eq);
    assert!(debug_str.contains("EqEffect"));
    assert!(debug_str.contains("bass_gain"));
}

#[test]
fn test_realistic_effect_combinations() {
    // Indoor gunshot (reverb + filter)
    let reverb = ReverbEffect::small_room();
    let filter = FilterEffect::muffled();
    assert!(reverb.validate() && filter.validate());

    // Outdoor echo (long echo + minimal reverb)
    let echo = EchoEffect::long_echo();
    let reverb = ReverbEffect { room_size: 0.2, damping: 0.8, wet_dry_mix: 0.1 };
    assert!(echo.validate() && reverb.validate());

    // Radio transmission (bandpass + echo)
    let filter = FilterEffect::radio();
    let echo = EchoEffect { delay_time: 0.05, feedback: 0.2, wet_dry_mix: 0.15 };
    assert!(filter.validate() && echo.validate());

    // Bass-heavy music (EQ + reverb)
    let eq = EqEffect::bass_boost();
    let reverb = ReverbEffect::large_hall();
    assert!(eq.validate() && reverb.validate());
}
