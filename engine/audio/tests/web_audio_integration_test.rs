//! Web Audio API integration tests
//!
//! These tests validate the Web Audio backend implementation.
//! Note: Full testing requires a browser environment with Web Audio API support.

#![cfg(target_arch = "wasm32")]

use engine_audio::{AudioBackend, AudioEngine};
use glam::Vec3;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_audio_engine_creation() {
    let engine = AudioEngine::new();
    assert!(engine.is_ok(), "Failed to create audio engine");
}

#[wasm_bindgen_test]
fn test_initial_state() {
    let engine = AudioEngine::new().unwrap();
    assert_eq!(engine.active_sound_count(), 0);
    assert_eq!(engine.loaded_sound_count(), 0);
}

#[wasm_bindgen_test]
fn test_listener_transform() {
    let mut engine = AudioEngine::new().unwrap();

    // Set listener position and orientation
    engine.set_listener_transform(
        Vec3::new(1.0, 2.0, 3.0),
        Vec3::new(0.0, 0.0, -1.0),
        Vec3::new(0.0, 1.0, 0.0),
    );

    // No crash = success (listener state is internal to Web Audio API)
    assert_eq!(engine.active_sound_count(), 0);
}

#[wasm_bindgen_test]
fn test_emitter_management() {
    let mut engine = AudioEngine::new().unwrap();

    let entity_id = 42;
    engine.update_emitter_position(entity_id, Vec3::new(5.0, 0.0, 0.0));
    engine.remove_emitter(entity_id);

    // No crash = success
    assert_eq!(engine.active_sound_count(), 0);
}

#[wasm_bindgen_test]
fn test_multiple_emitters() {
    let mut engine = AudioEngine::new().unwrap();

    // Create multiple emitters
    for i in 0..10 {
        engine.update_emitter_position(i, Vec3::new(i as f32, 0.0, 0.0));
    }

    // Remove them
    for i in 0..10 {
        engine.remove_emitter(i);
    }

    assert_eq!(engine.active_sound_count(), 0);
}

#[wasm_bindgen_test]
fn test_cleanup_finished() {
    let mut engine = AudioEngine::new().unwrap();

    // Cleanup on empty engine should not crash
    engine.cleanup_finished();
    assert_eq!(engine.active_sound_count(), 0);
}

#[wasm_bindgen_test]
fn test_is_playing_nonexistent() {
    let engine = AudioEngine::new().unwrap();

    // Querying non-existent sound should return false, not crash
    assert!(!engine.is_playing(999));
}

// Note: The following tests would require actual audio files to be loaded
// In a real browser test environment, you would:
// 1. Serve test audio files (e.g., test.wav, test.ogg)
// 2. Load them with load_sound()
// 3. Play and verify playback state
//
// Example (requires test assets):
// #[wasm_bindgen_test]
// async fn test_load_and_play_2d() {
//     let mut engine = AudioEngine::new().unwrap();
//     engine.load_sound("test", "/test-assets/beep.wav").unwrap();
//     let instance = engine.play_2d("test", 1.0, false).unwrap();
//     assert!(engine.is_playing(instance));
// }
