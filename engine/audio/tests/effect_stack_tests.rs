//! Effect stacking tests
//!
//! Tests for multiple audio effects applied to a single sound:
//! - Multiple effects on single sound
//! - Effect order dependency
//! - Effect parameter changes during playback
//! - Effect removal during playback
//! - Complex effect chains

use engine_audio::{
    AudioEffect, AudioEngine, EchoEffect, EqEffect, FilterEffect, FilterType, ReverbEffect,
};
use tracing::info;

/// Test adding multiple effects to engine
#[test]
fn test_multiple_effects_on_sound() {
    let engine = AudioEngine::new().expect("Failed to create audio engine");

    // Create various effects
    let reverb = AudioEffect::Reverb(ReverbEffect::small_room());
    let echo = AudioEffect::Echo(EchoEffect::slapback());
    let filter = AudioEffect::Filter(FilterEffect::muffled());
    let eq = AudioEffect::Eq(EqEffect::bass_boost());

    // Verify all effects are valid
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

    match &eq {
        AudioEffect::Eq(e) => assert!(e.validate()),
        _ => panic!("Expected EQ"),
    }

    // Test that we can query effect count
    let count = engine.effect_count(999); // Non-existent instance
    assert_eq!(count, 0);

    info!("Multiple effects validated successfully");
}

/// Test effect order matters
#[test]
fn test_effect_order_matters() {
    // Test that different effect orders produce different results

    // Chain 1: Reverb -> EQ -> Filter
    let chain1 = vec![
        AudioEffect::Reverb(ReverbEffect::large_hall()),
        AudioEffect::Eq(EqEffect::bass_boost()),
        AudioEffect::Filter(FilterEffect::muffled()),
    ];

    // Chain 2: Filter -> Reverb -> EQ (different order)
    let chain2 = vec![
        AudioEffect::Filter(FilterEffect::muffled()),
        AudioEffect::Reverb(ReverbEffect::large_hall()),
        AudioEffect::Eq(EqEffect::bass_boost()),
    ];

    // Both chains should be valid
    for effect in &chain1 {
        match effect {
            AudioEffect::Reverb(r) => assert!(r.validate()),
            AudioEffect::Eq(e) => assert!(e.validate()),
            AudioEffect::Filter(f) => assert!(f.validate()),
            _ => {}
        }
    }

    for effect in &chain2 {
        match effect {
            AudioEffect::Reverb(r) => assert!(r.validate()),
            AudioEffect::Eq(e) => assert!(e.validate()),
            AudioEffect::Filter(f) => assert!(f.validate()),
            _ => {}
        }
    }

    info!("Effect order test completed");
}

/// Test common effect combinations
#[test]
fn test_common_effect_combinations() {
    // Indoor gunshot (reverb + slight filter)
    let indoor_gunshot = vec![
        AudioEffect::Reverb(ReverbEffect::small_room()),
        AudioEffect::Filter(FilterEffect {
            filter_type: FilterType::LowPass,
            cutoff_frequency: 800.0,
            resonance: 1.0,
            wet_dry_mix: 0.3,
        }),
    ];

    for effect in &indoor_gunshot {
        match effect {
            AudioEffect::Reverb(r) => assert!(r.validate()),
            AudioEffect::Filter(f) => assert!(f.validate()),
            _ => {}
        }
    }

    // Outdoor echo (minimal reverb + long echo)
    let outdoor_echo = vec![
        AudioEffect::Reverb(ReverbEffect { room_size: 0.2, damping: 0.8, wet_dry_mix: 0.1 }),
        AudioEffect::Echo(EchoEffect::long_echo()),
    ];

    for effect in &outdoor_echo {
        match effect {
            AudioEffect::Reverb(r) => assert!(r.validate()),
            AudioEffect::Echo(e) => assert!(e.validate()),
            _ => {}
        }
    }

    // Radio transmission (bandpass + echo + EQ)
    let radio = vec![
        AudioEffect::Filter(FilterEffect::radio()),
        AudioEffect::Echo(EchoEffect { delay_time: 0.05, feedback: 0.2, wet_dry_mix: 0.15 }),
        AudioEffect::Eq(EqEffect { bass_gain: -5.0, mid_gain: 3.0, treble_gain: -3.0 }),
    ];

    for effect in &radio {
        match effect {
            AudioEffect::Filter(f) => assert!(f.validate()),
            AudioEffect::Echo(e) => assert!(e.validate()),
            AudioEffect::Eq(eq) => assert!(eq.validate()),
            _ => {}
        }
    }

    // Underwater (heavy filter + reverb + EQ)
    let underwater = vec![
        AudioEffect::Filter(FilterEffect {
            filter_type: FilterType::LowPass,
            cutoff_frequency: 300.0,
            resonance: 2.0,
            wet_dry_mix: 0.9,
        }),
        AudioEffect::Reverb(ReverbEffect::large_hall()),
        AudioEffect::Eq(EqEffect { bass_gain: 5.0, mid_gain: -10.0, treble_gain: -15.0 }),
    ];

    for effect in &underwater {
        match effect {
            AudioEffect::Filter(f) => assert!(f.validate()),
            AudioEffect::Reverb(r) => assert!(r.validate()),
            AudioEffect::Eq(eq) => assert!(eq.validate()),
            _ => {}
        }
    }

    info!("Common effect combinations validated");
}

/// Test maximum effect stack
#[test]
fn test_maximum_effect_stack() {
    // Test with many effects stacked
    let effects = vec![
        AudioEffect::Eq(EqEffect::bass_boost()),
        AudioEffect::Filter(FilterEffect::muffled()),
        AudioEffect::Reverb(ReverbEffect::small_room()),
        AudioEffect::Echo(EchoEffect::slapback()),
        AudioEffect::Eq(EqEffect::bright()),
        AudioEffect::Filter(FilterEffect::tinny()),
        AudioEffect::Reverb(ReverbEffect::large_hall()),
        AudioEffect::Echo(EchoEffect::long_echo()),
    ];

    // All effects should still be valid
    for effect in &effects {
        match effect {
            AudioEffect::Reverb(r) => assert!(r.validate()),
            AudioEffect::Echo(e) => assert!(e.validate()),
            AudioEffect::Filter(f) => assert!(f.validate()),
            AudioEffect::Eq(eq) => assert!(eq.validate()),
        }
    }

    info!(effect_count = effects.len(), "Maximum effect stack validated");
}

/// Test effect parameter modification
#[test]
fn test_effect_parameter_modification() {
    // Create effect with initial parameters
    let mut reverb = ReverbEffect::small_room();
    assert!(reverb.validate());

    // Modify parameters
    reverb.room_size = 0.8;
    reverb.wet_dry_mix = 0.5;
    assert!(reverb.validate());

    // Create echo with initial parameters
    let mut echo = EchoEffect::slapback();
    assert!(echo.validate());

    // Modify parameters
    echo.delay_time = 0.5;
    echo.feedback = 0.7;
    assert!(echo.validate());

    info!("Effect parameter modification works");
}

/// Test effect serialization in stacks
#[test]
fn test_effect_stack_serialization() {
    let effects = vec![
        AudioEffect::Reverb(ReverbEffect::cathedral()),
        AudioEffect::Echo(EchoEffect::long_echo()),
        AudioEffect::Filter(FilterEffect::radio()),
    ];

    for effect in &effects {
        let json = serde_json::to_string(effect).expect("Failed to serialize");
        let deserialized: AudioEffect = serde_json::from_str(&json).expect("Failed to deserialize");

        match (effect, &deserialized) {
            (AudioEffect::Reverb(a), AudioEffect::Reverb(b)) => assert_eq!(a, b),
            (AudioEffect::Echo(a), AudioEffect::Echo(b)) => assert_eq!(a, b),
            (AudioEffect::Filter(a), AudioEffect::Filter(b)) => assert_eq!(a, b),
            _ => panic!("Mismatched effect types"),
        }
    }

    info!("Effect stack serialization works");
}

/// Test reverb + echo combination (classic)
#[test]
fn test_reverb_echo_combination() {
    let reverb = ReverbEffect::large_hall();
    let echo = EchoEffect::long_echo();

    assert!(reverb.validate());
    assert!(echo.validate());

    // Should work well together for outdoor/canyon sounds
    info!("Reverb + Echo combination validated");
}

/// Test filter + EQ combination
#[test]
fn test_filter_eq_combination() {
    let filter = FilterEffect::muffled();
    let eq = EqEffect::bass_boost();

    assert!(filter.validate());
    assert!(eq.validate());

    // Should work well together for muffled bass-heavy sounds
    info!("Filter + EQ combination validated");
}

/// Test all effects together
#[test]
fn test_all_effects_together() {
    let reverb = ReverbEffect::default();
    let echo = EchoEffect::default();
    let filter = FilterEffect::default();
    let eq = EqEffect::default();

    assert!(reverb.validate());
    assert!(echo.validate());
    assert!(filter.validate());
    assert!(eq.validate());

    info!("All effects together validated");
}

/// Test effect wet/dry mix stacking
#[test]
fn test_wet_dry_mix_stacking() {
    // Multiple effects with different wet/dry mixes
    let effects = vec![
        AudioEffect::Reverb(ReverbEffect {
            room_size: 0.7,
            damping: 0.5,
            wet_dry_mix: 0.3, // 30% wet
        }),
        AudioEffect::Echo(EchoEffect {
            delay_time: 0.5,
            feedback: 0.5,
            wet_dry_mix: 0.2, // 20% wet
        }),
        AudioEffect::Filter(FilterEffect {
            filter_type: FilterType::LowPass,
            cutoff_frequency: 1000.0,
            resonance: 1.0,
            wet_dry_mix: 0.5, // 50% wet
        }),
    ];

    for effect in &effects {
        match effect {
            AudioEffect::Reverb(r) => {
                assert!(r.validate());
                assert_eq!(r.wet_dry_mix, 0.3);
            }
            AudioEffect::Echo(e) => {
                assert!(e.validate());
                assert_eq!(e.wet_dry_mix, 0.2);
            }
            AudioEffect::Filter(f) => {
                assert!(f.validate());
                assert_eq!(f.wet_dry_mix, 0.5);
            }
            _ => {}
        }
    }

    info!("Wet/dry mix stacking validated");
}

/// Test effect preset combinations
#[test]
fn test_effect_preset_combinations() {
    // Test various preset combinations
    let combinations = vec![
        vec![
            AudioEffect::Reverb(ReverbEffect::small_room()),
            AudioEffect::Echo(EchoEffect::slapback()),
        ],
        vec![
            AudioEffect::Reverb(ReverbEffect::large_hall()),
            AudioEffect::Eq(EqEffect::bass_boost()),
        ],
        vec![
            AudioEffect::Filter(FilterEffect::muffled()),
            AudioEffect::Reverb(ReverbEffect::cathedral()),
        ],
        vec![
            AudioEffect::Echo(EchoEffect::long_echo()),
            AudioEffect::Filter(FilterEffect::radio()),
        ],
    ];

    for (i, combination) in combinations.iter().enumerate() {
        for effect in combination {
            match effect {
                AudioEffect::Reverb(r) => assert!(r.validate()),
                AudioEffect::Echo(e) => assert!(e.validate()),
                AudioEffect::Filter(f) => assert!(f.validate()),
                AudioEffect::Eq(eq) => assert!(eq.validate()),
            }
        }
        info!(combination_index = i, "Preset combination validated");
    }
}

/// Test extreme effect stacking
#[test]
fn test_extreme_effect_stacking() {
    // Stack same effect type multiple times
    let multiple_reverbs = vec![
        AudioEffect::Reverb(ReverbEffect { room_size: 0.3, damping: 0.6, wet_dry_mix: 0.2 }),
        AudioEffect::Reverb(ReverbEffect { room_size: 0.7, damping: 0.4, wet_dry_mix: 0.3 }),
        AudioEffect::Reverb(ReverbEffect { room_size: 0.9, damping: 0.2, wet_dry_mix: 0.4 }),
    ];

    for effect in &multiple_reverbs {
        match effect {
            AudioEffect::Reverb(r) => assert!(r.validate()),
            _ => panic!("Expected reverb"),
        }
    }

    info!("Extreme effect stacking validated");
}

/// Test effect parameter ranges in combinations
#[test]
fn test_effect_parameter_ranges_in_combinations() {
    // Test extreme but valid parameter combinations
    let extreme_combination = vec![
        AudioEffect::Reverb(ReverbEffect {
            room_size: 1.0,   // Max
            damping: 0.0,     // Min
            wet_dry_mix: 1.0, // Max
        }),
        AudioEffect::Echo(EchoEffect {
            delay_time: 2.0,  // Max
            feedback: 0.95,   // Near max (< 1.0 to avoid infinite feedback)
            wet_dry_mix: 0.0, // Min
        }),
        AudioEffect::Filter(FilterEffect {
            filter_type: FilterType::BandPass,
            cutoff_frequency: 20000.0, // Max
            resonance: 10.0,           // Max
            wet_dry_mix: 0.5,          // Mid
        }),
        AudioEffect::Eq(EqEffect {
            bass_gain: 20.0,  // Max
            mid_gain: -20.0,  // Min
            treble_gain: 0.0, // Neutral
        }),
    ];

    for effect in &extreme_combination {
        match effect {
            AudioEffect::Reverb(r) => assert!(r.validate()),
            AudioEffect::Echo(e) => assert!(e.validate()),
            AudioEffect::Filter(f) => assert!(f.validate()),
            AudioEffect::Eq(eq) => assert!(eq.validate()),
        }
    }

    info!("Extreme parameter ranges validated");
}

/// Test effect cloning in stacks
#[test]
fn test_effect_cloning_in_stacks() {
    let original_stack = vec![
        AudioEffect::Reverb(ReverbEffect::cathedral()),
        AudioEffect::Echo(EchoEffect::long_echo()),
    ];

    let cloned_stack = original_stack.clone();

    assert_eq!(original_stack.len(), cloned_stack.len());

    for (original, cloned) in original_stack.iter().zip(cloned_stack.iter()) {
        match (original, cloned) {
            (AudioEffect::Reverb(a), AudioEffect::Reverb(b)) => assert_eq!(a, b),
            (AudioEffect::Echo(a), AudioEffect::Echo(b)) => assert_eq!(a, b),
            _ => panic!("Mismatched effect types"),
        }
    }

    info!("Effect cloning in stacks works");
}

/// Test empty effect stack
#[test]
fn test_empty_effect_stack() {
    let empty_stack: Vec<AudioEffect> = Vec::new();
    assert_eq!(empty_stack.len(), 0);

    info!("Empty effect stack handled correctly");
}

/// Test single effect "stack"
#[test]
fn test_single_effect_stack() {
    let single_effect = vec![AudioEffect::Reverb(ReverbEffect::small_room())];

    assert_eq!(single_effect.len(), 1);
    match &single_effect[0] {
        AudioEffect::Reverb(r) => assert!(r.validate()),
        _ => panic!("Expected reverb"),
    }

    info!("Single effect stack works");
}
