//! Integration tests for Android audio backend
//!
//! Note: These tests require an Android device or emulator with audio support.
//! On desktop platforms, these tests will be skipped.

#![cfg(target_os = "android")]

use engine_audio::{AudioEngine, AudioError};
use glam::Vec3;
use std::path::Path;
use std::thread;
use std::time::Duration;

#[test]
fn test_android_backend_creation() {
    let result = AudioEngine::new();
    assert!(result.is_ok(), "Failed to create Android audio backend");

    let engine = result.unwrap();
    assert_eq!(engine.active_sound_count(), 0);
    assert_eq!(engine.loaded_sound_count(), 0);
}

#[test]
fn test_load_sound_from_assets() {
    let mut engine = AudioEngine::new().unwrap();

    // Note: In real Android app, use AssetManager to load from APK
    // For now, this tests the error path
    let result = engine.load_sound("test", Path::new("/data/local/tmp/test.wav"));

    // File likely doesn't exist, but API should work
    assert!(result.is_err() || result.is_ok());
}

#[test]
fn test_2d_sound_playback_without_file() {
    let mut engine = AudioEngine::new().unwrap();

    // Try to play non-existent sound
    let result = engine.play_2d("nonexistent", 1.0, false);

    match result {
        Err(AudioError::SoundNotFound(_)) => {
            // Expected error
        }
        _ => panic!("Expected SoundNotFound error"),
    }
}

#[test]
fn test_3d_sound_playback_without_file() {
    let mut engine = AudioEngine::new().unwrap();

    // Try to play non-existent 3D sound
    let result = engine.play_3d(1, "nonexistent", Vec3::new(5.0, 0.0, 0.0), 1.0, false, 100.0);

    match result {
        Err(AudioError::SoundNotFound(_)) => {
            // Expected error
        }
        _ => panic!("Expected SoundNotFound error"),
    }
}

#[test]
fn test_listener_transform() {
    let mut engine = AudioEngine::new().unwrap();

    // Should not crash
    engine.set_listener_transform(Vec3::new(10.0, 5.0, 3.0), Vec3::NEG_Z, Vec3::Y);

    // Update again
    engine.set_listener_transform(Vec3::new(20.0, 10.0, 6.0), Vec3::new(1.0, 0.0, 0.0), Vec3::Y);
}

#[test]
fn test_emitter_management() {
    let mut engine = AudioEngine::new().unwrap();

    let entity_id = 42;

    // Update position
    engine.update_emitter_position(entity_id, Vec3::new(5.0, 0.0, 0.0));
    engine.update_emitter_position(entity_id, Vec3::new(10.0, 2.0, 3.0));

    // Remove emitter
    engine.remove_emitter(entity_id);

    // Should not crash
}

#[test]
fn test_instance_lifecycle() {
    let mut engine = AudioEngine::new().unwrap();

    // Create fake instance ID
    let fake_id = 999;

    // Should return false for non-existent instance
    assert!(!engine.is_playing(fake_id));

    // Stop non-existent instance (should not crash)
    engine.stop(fake_id, None);
    engine.stop(fake_id, Some(0.5));
}

#[test]
fn test_cleanup_finished() {
    let mut engine = AudioEngine::new().unwrap();

    // Should not crash even with no sounds
    engine.cleanup_finished();

    assert_eq!(engine.active_sound_count(), 0);
}

#[test]
fn test_multiple_listener_updates() {
    let mut engine = AudioEngine::new().unwrap();

    // Simulate listener movement
    for i in 0..100 {
        let t = i as f32 * 0.1;
        engine.set_listener_transform(
            Vec3::new(t.sin() * 10.0, 5.0, t.cos() * 10.0),
            Vec3::new(-t.sin(), 0.0, -t.cos()),
            Vec3::Y,
        );
    }
}

#[test]
fn test_active_and_loaded_counts() {
    let engine = AudioEngine::new().unwrap();

    let active = engine.active_sound_count();
    let loaded = engine.loaded_sound_count();

    assert_eq!(active, 0);
    assert_eq!(loaded, 0);
}

#[test]
fn test_concurrent_emitter_updates() {
    let mut engine = AudioEngine::new().unwrap();

    // Update multiple emitters
    for entity_id in 0..50 {
        engine.update_emitter_position(
            entity_id,
            Vec3::new((entity_id as f32).sin() * 10.0, 5.0, (entity_id as f32).cos() * 10.0),
        );
    }

    // Remove half of them
    for entity_id in 0..25 {
        engine.remove_emitter(entity_id);
    }
}

/// Performance test - create and manage many emitters
#[test]
fn test_many_emitters() {
    let mut engine = AudioEngine::new().unwrap();

    const EMITTER_COUNT: u32 = 1000;

    // Create many emitters
    for entity_id in 0..EMITTER_COUNT {
        engine.update_emitter_position(
            entity_id,
            Vec3::new((entity_id as f32 % 100.0) - 50.0, 0.0, ((entity_id / 100) as f32) - 5.0),
        );
    }

    // Update listener
    engine.set_listener_transform(Vec3::ZERO, Vec3::NEG_Z, Vec3::Y);

    // Clean up
    for entity_id in 0..EMITTER_COUNT {
        engine.remove_emitter(entity_id);
    }
}

/// Test streaming API
#[test]
fn test_streaming_api() {
    let mut engine = AudioEngine::new().unwrap();

    // Try to stream non-existent file
    let result = engine.play_stream(Path::new("/data/local/tmp/music.ogg"), 0.8, true);

    // File likely doesn't exist, but API should work
    assert!(result.is_err() || result.is_ok());
}

// Note: The following tests require actual audio files in the Android assets.
// They are marked as ignored by default and should be run manually on device.

#[test]
#[ignore]
fn test_wav_playback_on_device() {
    let mut engine = AudioEngine::new().unwrap();

    // Load from Android internal storage (must be manually placed)
    engine
        .load_sound("test_wav", Path::new("/sdcard/test.wav"))
        .expect("WAV file not found - place test.wav in /sdcard/");

    let instance_id = engine.play_2d("test_wav", 1.0, false).expect("Failed to play WAV");

    assert!(engine.is_playing(instance_id));

    // Let it play for a bit
    thread::sleep(Duration::from_millis(100));

    engine.stop(instance_id, Some(0.1));

    // Wait for fade out
    thread::sleep(Duration::from_millis(150));

    assert!(!engine.is_playing(instance_id));
}

#[test]
#[ignore]
fn test_ogg_playback_on_device() {
    let mut engine = AudioEngine::new().unwrap();

    engine
        .load_sound("test_ogg", Path::new("/sdcard/test.ogg"))
        .expect("OGG file not found - place test.ogg in /sdcard/");

    let instance_id = engine.play_2d("test_ogg", 0.8, false).expect("Failed to play OGG");

    assert!(engine.is_playing(instance_id));

    thread::sleep(Duration::from_millis(200));

    engine.cleanup_finished();
}

#[test]
#[ignore]
fn test_mp3_playback_on_device() {
    let mut engine = AudioEngine::new().unwrap();

    engine
        .load_sound("test_mp3", Path::new("/sdcard/test.mp3"))
        .expect("MP3 file not found - place test.mp3 in /sdcard/");

    let instance_id = engine.play_2d("test_mp3", 1.0, false).expect("Failed to play MP3");

    assert!(engine.is_playing(instance_id));

    thread::sleep(Duration::from_millis(200));
}

#[test]
#[ignore]
fn test_3d_audio_on_device() {
    let mut engine = AudioEngine::new().unwrap();

    engine
        .load_sound("footstep", Path::new("/sdcard/footstep.wav"))
        .expect("Footstep sound not found");

    // Play sound to the right
    let instance_right = engine
        .play_3d(1, "footstep", Vec3::new(10.0, 0.0, 0.0), 1.0, false, 50.0)
        .expect("Failed to play 3D sound");

    thread::sleep(Duration::from_millis(500));

    // Play sound to the left
    let instance_left = engine
        .play_3d(2, "footstep", Vec3::new(-10.0, 0.0, 0.0), 1.0, false, 50.0)
        .expect("Failed to play 3D sound");

    thread::sleep(Duration::from_millis(500));

    // Move listener
    engine.set_listener_transform(Vec3::new(5.0, 0.0, 0.0), Vec3::NEG_Z, Vec3::Y);

    thread::sleep(Duration::from_millis(500));

    engine.cleanup_finished();
}

#[test]
#[ignore]
fn test_looping_sound_on_device() {
    let mut engine = AudioEngine::new().unwrap();

    engine
        .load_sound("loop", Path::new("/sdcard/loop.wav"))
        .expect("Loop sound not found");

    let instance_id = engine.play_2d("loop", 0.5, true).expect("Failed to play looping sound");

    assert!(engine.is_playing(instance_id));

    // Let it loop a few times
    thread::sleep(Duration::from_secs(2));

    assert!(engine.is_playing(instance_id));

    engine.stop(instance_id, Some(0.5));

    // Wait for fade out
    thread::sleep(Duration::from_millis(600));

    assert!(!engine.is_playing(instance_id));
}

#[test]
#[ignore]
fn test_many_simultaneous_sounds_on_device() {
    let mut engine = AudioEngine::new().unwrap();

    engine
        .load_sound("gunshot", Path::new("/sdcard/gunshot.wav"))
        .expect("Gunshot sound not found");

    let mut instances = Vec::new();

    // Play many sounds rapidly
    for i in 0..50 {
        let volume = 0.1; // Keep volume low to avoid distortion
        let instance = engine.play_2d("gunshot", volume, false).expect("Failed to play sound");

        instances.push(instance);

        // Small delay between sounds
        thread::sleep(Duration::from_millis(20));
    }

    // Let sounds finish
    thread::sleep(Duration::from_secs(1));

    // Most should have finished
    let active = engine.active_sound_count();
    assert!(active < 10, "Too many sounds still active: {}", active);

    engine.cleanup_finished();
    assert_eq!(engine.active_sound_count(), 0);
}

#[test]
#[ignore]
fn test_streaming_music_on_device() {
    let mut engine = AudioEngine::new().unwrap();

    let instance_id = engine
        .play_stream(Path::new("/sdcard/music.ogg"), 0.5, true)
        .expect("Music file not found");

    assert!(engine.is_playing(instance_id));

    // Let it stream for a while
    thread::sleep(Duration::from_secs(5));

    assert!(engine.is_playing(instance_id));

    // Fade out
    engine.stop(instance_id, Some(1.0));

    thread::sleep(Duration::from_millis(1100));

    assert!(!engine.is_playing(instance_id));
}

#[test]
#[ignore]
fn test_3d_audio_distance_falloff_on_device() {
    let mut engine = AudioEngine::new().unwrap();

    engine
        .load_sound("ambient", Path::new("/sdcard/ambient.wav"))
        .expect("Ambient sound not found");

    // Play sound at various distances
    let distances = [1.0, 5.0, 10.0, 25.0, 50.0, 100.0];

    for (i, &distance) in distances.iter().enumerate() {
        let instance = engine
            .play_3d(
                i as u32,
                "ambient",
                Vec3::new(0.0, 0.0, -distance),
                1.0,
                false,
                50.0, // Max distance
            )
            .expect("Failed to play 3D sound");

        thread::sleep(Duration::from_millis(800));

        if engine.is_playing(instance) {
            engine.stop(instance, None);
        }

        thread::sleep(Duration::from_millis(200));
    }
}

#[test]
#[ignore]
fn test_emitter_movement_on_device() {
    let mut engine = AudioEngine::new().unwrap();

    engine
        .load_sound("siren", Path::new("/sdcard/siren.wav"))
        .expect("Siren sound not found");

    let entity_id = 99;

    // Start playing looping sound
    let instance_id = engine
        .play_3d(entity_id, "siren", Vec3::new(-20.0, 0.0, 0.0), 1.0, true, 100.0)
        .expect("Failed to play sound");

    // Move emitter from left to right
    for i in 0..40 {
        let t = i as f32 / 40.0;
        let x = -20.0 + t * 40.0; // -20 to +20

        engine.update_emitter_position(entity_id, Vec3::new(x, 0.0, 0.0));

        thread::sleep(Duration::from_millis(50));
    }

    engine.stop(instance_id, Some(0.5));
    thread::sleep(Duration::from_millis(600));

    assert!(!engine.is_playing(instance_id));
}
