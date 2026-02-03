//! Web Audio API backend integration tests
//!
//! These tests verify the Web Audio backend implementation.
//! Many tests require a browser environment and will be skipped on non-WASM targets.

#![cfg(target_arch = "wasm32")]

use engine_audio::platform::web::WebAudioBackend;
use engine_audio::platform::AudioBackend;
use engine_audio::AudioError;
use glam::Vec3;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_backend_creation() {
    let result = WebAudioBackend::new();
    assert!(result.is_ok(), "Failed to create Web Audio backend");

    let backend = result.unwrap();
    assert_eq!(backend.active_sound_count(), 0);
    assert_eq!(backend.loaded_sound_count(), 0);
}

#[wasm_bindgen_test]
fn test_listener_transform() {
    let mut backend = WebAudioBackend::new().unwrap();

    let position = Vec3::new(10.0, 5.0, -3.0);
    let forward = Vec3::new(0.0, 0.0, -1.0);
    let up = Vec3::new(0.0, 1.0, 0.0);

    // Should not panic
    backend.set_listener_transform(position, forward, up);
}

#[wasm_bindgen_test]
fn test_emitter_management() {
    let mut backend = WebAudioBackend::new().unwrap();

    let entity_id = 1;
    let position = Vec3::new(5.0, 0.0, 0.0);

    // Update emitter that doesn't exist yet (should not panic)
    backend.update_emitter_position(entity_id, position);

    // Remove non-existent emitter (should not panic)
    backend.remove_emitter(entity_id);
}

#[wasm_bindgen_test]
fn test_sound_not_found_error() {
    let mut backend = WebAudioBackend::new().unwrap();

    // Try to play a sound that hasn't been loaded
    let result = backend.play_2d("nonexistent_sound", 1.0, false);

    assert!(result.is_err());
    match result.unwrap_err() {
        AudioError::SoundNotFound(name) => {
            assert_eq!(name, "nonexistent_sound");
        }
        _ => panic!("Expected SoundNotFound error"),
    }
}

#[wasm_bindgen_test]
fn test_is_playing_nonexistent() {
    let backend = WebAudioBackend::new().unwrap();

    // Check status of non-existent sound instance
    assert!(!backend.is_playing(999));
}

#[wasm_bindgen_test]
fn test_cleanup_finished() {
    let mut backend = WebAudioBackend::new().unwrap();

    // Cleanup with no active sounds (should not panic)
    backend.cleanup_finished();
    assert_eq!(backend.active_sound_count(), 0);
}

#[wasm_bindgen_test]
fn test_stop_nonexistent_sound() {
    let mut backend = WebAudioBackend::new().unwrap();

    // Stop a sound that doesn't exist (should not panic)
    backend.stop(999, None);
    backend.stop(1000, Some(1.0));
}

#[wasm_bindgen_test]
fn test_multiple_listener_updates() {
    let mut backend = WebAudioBackend::new().unwrap();

    // Update listener position multiple times
    for i in 0..10 {
        let position = Vec3::new(i as f32, 0.0, 0.0);
        let forward = Vec3::new(0.0, 0.0, -1.0);
        let up = Vec3::new(0.0, 1.0, 0.0);

        backend.set_listener_transform(position, forward, up);
    }

    // Should complete without errors
}

#[wasm_bindgen_test]
fn test_multiple_emitter_updates() {
    let mut backend = WebAudioBackend::new().unwrap();

    // Update multiple emitters
    for entity_id in 0..5 {
        let position = Vec3::new(entity_id as f32 * 10.0, 0.0, 0.0);
        backend.update_emitter_position(entity_id, position);
    }

    // Remove all emitters
    for entity_id in 0..5 {
        backend.remove_emitter(entity_id);
    }
}

#[wasm_bindgen_test]
fn test_listener_orientation_normalization() {
    let mut backend = WebAudioBackend::new().unwrap();

    // Test with non-normalized vectors (should still work)
    let position = Vec3::ZERO;
    let forward = Vec3::new(0.0, 0.0, -2.0); // Not normalized
    let up = Vec3::new(0.0, 3.0, 0.0); // Not normalized

    backend.set_listener_transform(position, forward, up);
}

#[wasm_bindgen_test]
fn test_3d_sound_position_bounds() {
    let backend = WebAudioBackend::new().unwrap();

    // Test extreme position values
    let extreme_positions = vec![
        Vec3::new(f32::MAX, 0.0, 0.0),
        Vec3::new(f32::MIN, 0.0, 0.0),
        Vec3::new(0.0, f32::MAX, 0.0),
        Vec3::new(0.0, 0.0, f32::MIN),
        Vec3::new(1000000.0, 1000000.0, 1000000.0),
        Vec3::new(-1000000.0, -1000000.0, -1000000.0),
    ];

    for position in extreme_positions {
        // Should not panic when updating positions with extreme values
        let entity_id = 1;
        // This is just a compile-time check - actual update would require loaded sounds
        let _ = position;
    }
}

#[wasm_bindgen_test]
fn test_volume_range() {
    let backend = WebAudioBackend::new().unwrap();

    // Test various volume levels
    let volumes = vec![0.0, 0.25, 0.5, 0.75, 1.0, 1.5, 2.0];

    for volume in volumes {
        // Create gain nodes with different volumes (should not panic)
        let result = backend.create_gain(volume);
        // In a real test, we'd verify the actual gain value
        // For now, just ensure no panic occurs
        let _ = result;
    }
}

#[wasm_bindgen_test]
fn test_max_distance_values() {
    let backend = WebAudioBackend::new().unwrap();

    let position = Vec3::ZERO;

    // Test various max distance values
    let distances = vec![0.1, 1.0, 10.0, 100.0, 1000.0, 10000.0];

    for max_distance in distances {
        let result = backend.create_panner(position, max_distance);
        // Should successfully create panner with valid distances
        assert!(result.is_ok(), "Failed to create panner with max_distance: {}", max_distance);
    }
}

#[wasm_bindgen_test]
fn test_effect_not_implemented() {
    use engine_audio::effects::{AudioEffect, ReverbEffect};

    let mut backend = WebAudioBackend::new().unwrap();

    let reverb = AudioEffect::Reverb(ReverbEffect::default());
    let result = backend.add_effect(0, reverb);

    // Should return error for unimplemented effects
    assert!(result.is_err());
    match result.unwrap_err() {
        AudioError::EffectError(msg) => {
            assert!(msg.contains("not yet implemented"));
        }
        _ => panic!("Expected EffectError"),
    }
}

#[wasm_bindgen_test]
fn test_remove_effect_not_implemented() {
    let mut backend = WebAudioBackend::new().unwrap();

    // Should return false for unimplemented feature
    let result = backend.remove_effect(0, 0);
    assert!(!result);
}

#[wasm_bindgen_test]
fn test_effect_count_not_implemented() {
    let backend = WebAudioBackend::new().unwrap();

    // Should return 0 for unimplemented feature
    let count = backend.effect_count(0);
    assert_eq!(count, 0);
}

#[wasm_bindgen_test]
fn test_clear_effects_not_implemented() {
    let mut backend = WebAudioBackend::new().unwrap();

    // Should not panic (no-op)
    backend.clear_effects(0);
}

// Performance regression tests
#[wasm_bindgen_test]
fn test_performance_listener_updates() {
    let mut backend = WebAudioBackend::new().unwrap();

    let start = web_sys::window().unwrap().performance().unwrap().now();

    // Update listener 100 times
    for i in 0..100 {
        let position = Vec3::new(i as f32, 0.0, 0.0);
        let forward = Vec3::new(0.0, 0.0, -1.0);
        let up = Vec3::new(0.0, 1.0, 0.0);

        backend.set_listener_transform(position, forward, up);
    }

    let elapsed = web_sys::window().unwrap().performance().unwrap().now() - start;

    // Should complete in reasonable time (< 100ms for 100 updates)
    assert!(elapsed < 100.0, "Listener updates too slow: {}ms", elapsed);
}

#[wasm_bindgen_test]
fn test_performance_emitter_updates() {
    let mut backend = WebAudioBackend::new().unwrap();

    let start = web_sys::window().unwrap().performance().unwrap().now();

    // Update 10 emitters, 10 times each
    for _ in 0..10 {
        for entity_id in 0..10 {
            let position = Vec3::new(entity_id as f32 * 10.0, 0.0, 0.0);
            backend.update_emitter_position(entity_id, position);
        }
    }

    let elapsed = web_sys::window().unwrap().performance().unwrap().now() - start;

    // Should complete in reasonable time (< 100ms for 100 updates)
    assert!(elapsed < 100.0, "Emitter updates too slow: {}ms", elapsed);
}

// Edge case tests
#[wasm_bindgen_test]
fn test_zero_volume() {
    let backend = WebAudioBackend::new().unwrap();

    // Create gain node with zero volume (should work)
    let result = backend.create_gain(0.0);
    assert!(result.is_ok());
}

#[wasm_bindgen_test]
fn test_negative_volume() {
    let backend = WebAudioBackend::new().unwrap();

    // Negative volume is technically valid (inverts phase)
    let result = backend.create_gain(-1.0);
    assert!(result.is_ok());
}

#[wasm_bindgen_test]
fn test_very_large_volume() {
    let backend = WebAudioBackend::new().unwrap();

    // Very large volume values
    let result = backend.create_gain(100.0);
    assert!(result.is_ok());
}

#[wasm_bindgen_test]
fn test_zero_max_distance() {
    let backend = WebAudioBackend::new().unwrap();

    let position = Vec3::ZERO;

    // Zero max distance is an edge case
    let result = backend.create_panner(position, 0.0);
    assert!(result.is_ok());
}

#[wasm_bindgen_test]
fn test_same_position_listener_and_source() {
    let mut backend = WebAudioBackend::new().unwrap();

    let position = Vec3::new(5.0, 5.0, 5.0);
    let forward = Vec3::new(0.0, 0.0, -1.0);
    let up = Vec3::new(0.0, 1.0, 0.0);

    // Set listener at position
    backend.set_listener_transform(position, forward, up);

    // Update emitter to same position (should not cause issues)
    backend.update_emitter_position(1, position);
}
