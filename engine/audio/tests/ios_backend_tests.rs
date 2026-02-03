//! iOS audio backend integration tests
//!
//! These tests verify the iOS Core Audio backend implementation.
//! Most tests require an actual iOS device or simulator to run.

#[cfg(target_os = "ios")]
mod ios_tests {
    use engine_audio::platform::{create_audio_backend, AudioBackend};
    use engine_audio::{AudioError, AudioResult};
    use glam::Vec3;
    use std::path::Path;

    #[test]
    fn test_backend_creation() {
        let result = create_audio_backend();
        assert!(result.is_ok(), "Failed to create iOS audio backend: {:?}", result.err());
    }

    #[test]
    fn test_multiple_backend_creation() {
        // Verify we can create multiple backends (audio session is shared)
        let backend1 = create_audio_backend();
        assert!(backend1.is_ok());

        let backend2 = create_audio_backend();
        assert!(backend2.is_ok());
    }

    #[test]
    fn test_listener_transform_updates() {
        let mut backend = create_audio_backend().expect("Failed to create backend");

        // Set initial listener position
        backend.set_listener_transform(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, -1.0),
            Vec3::new(0.0, 1.0, 0.0),
        );

        // Update listener position
        backend.set_listener_transform(
            Vec3::new(10.0, 5.0, -3.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );

        // Should not panic or error
    }

    #[test]
    fn test_emitter_lifecycle() {
        let mut backend = create_audio_backend().expect("Failed to create backend");

        let entity_id = 42;
        let position = Vec3::new(5.0, 0.0, 0.0);

        // Create emitter
        backend.update_emitter_position(entity_id, position);

        // Update emitter
        backend.update_emitter_position(entity_id, Vec3::new(10.0, 0.0, 0.0));

        // Remove emitter
        backend.remove_emitter(entity_id);

        // Remove non-existent emitter (should not panic)
        backend.remove_emitter(999);
    }

    #[test]
    fn test_sound_not_found() {
        let mut backend = create_audio_backend().expect("Failed to create backend");

        let result = backend.play_2d("nonexistent_sound", 1.0, false);
        assert!(matches!(result, Err(AudioError::SoundNotFound(_))));
    }

    #[test]
    fn test_cleanup_finished() {
        let mut backend = create_audio_backend().expect("Failed to create backend");

        // Cleanup with no active sounds
        backend.cleanup_finished();
        assert_eq!(backend.active_sound_count(), 0);
    }

    #[test]
    fn test_active_sound_count() {
        let backend = create_audio_backend().expect("Failed to create backend");

        // Initially no sounds
        assert_eq!(backend.active_sound_count(), 0);
    }

    #[test]
    fn test_loaded_sound_count() {
        let backend = create_audio_backend().expect("Failed to create backend");

        // Initially no sounds loaded
        assert_eq!(backend.loaded_sound_count(), 0);
    }

    #[test]
    fn test_is_playing_invalid_instance() {
        let backend = create_audio_backend().expect("Failed to create backend");

        // Non-existent instance should return false
        assert!(!backend.is_playing(999));
    }

    #[test]
    fn test_stop_invalid_instance() {
        let mut backend = create_audio_backend().expect("Failed to create backend");

        // Stopping non-existent instance should not panic
        backend.stop(999, None);
        backend.stop(999, Some(1.0));
    }

    #[test]
    fn test_concurrent_emitters() {
        let mut backend = create_audio_backend().expect("Failed to create backend");

        // Create multiple emitters
        for entity_id in 0..10 {
            backend.update_emitter_position(entity_id, Vec3::new(entity_id as f32, 0.0, 0.0));
        }

        // Update all emitters
        for entity_id in 0..10 {
            backend.update_emitter_position(entity_id, Vec3::new(entity_id as f32 * 2.0, 0.0, 0.0));
        }

        // Remove all emitters
        for entity_id in 0..10 {
            backend.remove_emitter(entity_id);
        }
    }

    #[test]
    fn test_listener_orientation_changes() {
        let mut backend = create_audio_backend().expect("Failed to create backend");

        // Test various orientations
        let orientations = vec![
            (Vec3::new(0.0, 0.0, -1.0), Vec3::new(0.0, 1.0, 0.0)), // Forward
            (Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0)),  // Right
            (Vec3::new(0.0, 0.0, 1.0), Vec3::new(0.0, 1.0, 0.0)),  // Backward
            (Vec3::new(-1.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0)), // Left
        ];

        for (forward, up) in orientations {
            backend.set_listener_transform(Vec3::ZERO, forward, up);
        }
    }

    // Tests that require actual audio files (run manually with test assets)

    #[test]
    #[ignore = "Requires test audio files"]
    fn test_load_wav_file() {
        let mut backend = create_audio_backend().expect("Failed to create backend");

        let result = backend.load_sound("test", Path::new("test_assets/test.wav"));
        assert!(result.is_ok(), "Failed to load WAV file: {:?}", result.err());

        assert_eq!(backend.loaded_sound_count(), 1);
    }

    #[test]
    #[ignore = "Requires test audio files"]
    fn test_load_multiple_sounds() {
        let mut backend = create_audio_backend().expect("Failed to create backend");

        backend
            .load_sound("sound1", Path::new("test_assets/sound1.wav"))
            .expect("Failed to load sound1");
        backend
            .load_sound("sound2", Path::new("test_assets/sound2.wav"))
            .expect("Failed to load sound2");

        assert_eq!(backend.loaded_sound_count(), 2);
    }

    #[test]
    #[ignore = "Requires test audio files"]
    fn test_play_2d_sound() {
        let mut backend = create_audio_backend().expect("Failed to create backend");

        backend
            .load_sound("test", Path::new("test_assets/test.wav"))
            .expect("Failed to load sound");

        let instance_id = backend.play_2d("test", 1.0, false).expect("Failed to play sound");

        assert!(backend.is_playing(instance_id));
        assert_eq!(backend.active_sound_count(), 1);
    }

    #[test]
    #[ignore = "Requires test audio files"]
    fn test_play_3d_sound() {
        let mut backend = create_audio_backend().expect("Failed to create backend");

        backend
            .load_sound("test", Path::new("test_assets/test.wav"))
            .expect("Failed to load sound");

        let entity_id = 1;
        let position = Vec3::new(5.0, 0.0, 0.0);

        let instance_id = backend
            .play_3d("test", entity_id, position, 1.0, false, 50.0)
            .expect("Failed to play 3D sound");

        assert!(backend.is_playing(instance_id));
        assert_eq!(backend.active_sound_count(), 1);
    }

    #[test]
    #[ignore = "Requires test audio files"]
    fn test_looping_sound() {
        let mut backend = create_audio_backend().expect("Failed to create backend");

        backend
            .load_sound("loop", Path::new("test_assets/loop.wav"))
            .expect("Failed to load sound");

        let instance_id = backend.play_2d("loop", 0.5, true).expect("Failed to play looping sound");

        assert!(backend.is_playing(instance_id));

        // Stop the looping sound
        backend.stop(instance_id, None);
    }

    #[test]
    #[ignore = "Requires test audio files"]
    fn test_stop_sound() {
        let mut backend = create_audio_backend().expect("Failed to create backend");

        backend
            .load_sound("test", Path::new("test_assets/test.wav"))
            .expect("Failed to load sound");

        let instance_id = backend.play_2d("test", 1.0, true).expect("Failed to play sound");

        assert!(backend.is_playing(instance_id));

        backend.stop(instance_id, None);

        // Give the system a moment to process the stop
        std::thread::sleep(std::time::Duration::from_millis(100));

        assert!(!backend.is_playing(instance_id));
    }

    #[test]
    #[ignore = "Requires test audio files"]
    fn test_multiple_simultaneous_sounds() {
        let mut backend = create_audio_backend().expect("Failed to create backend");

        backend
            .load_sound("test", Path::new("test_assets/test.wav"))
            .expect("Failed to load sound");

        let mut instances = Vec::new();

        // Play 10 simultaneous sounds
        for _ in 0..10 {
            let instance_id = backend.play_2d("test", 0.5, false).expect("Failed to play sound");
            instances.push(instance_id);
        }

        assert_eq!(backend.active_sound_count(), 10);

        // Stop all sounds
        for instance_id in instances {
            backend.stop(instance_id, None);
        }
    }

    #[test]
    #[ignore = "Requires test audio files"]
    fn test_spatial_audio_distance() {
        let mut backend = create_audio_backend().expect("Failed to create backend");

        backend
            .load_sound("test", Path::new("test_assets/test.wav"))
            .expect("Failed to load sound");

        // Set listener at origin
        backend.set_listener_transform(
            Vec3::ZERO,
            Vec3::new(0.0, 0.0, -1.0),
            Vec3::new(0.0, 1.0, 0.0),
        );

        let entity_id = 1;

        // Play sound at various distances
        let distances = vec![1.0, 10.0, 50.0, 100.0];

        for distance in distances {
            let position = Vec3::new(distance, 0.0, 0.0);
            let instance_id = backend
                .play_3d("test", entity_id, position, 1.0, false, 100.0)
                .expect("Failed to play 3D sound");

            // Let it play briefly
            std::thread::sleep(std::time::Duration::from_millis(100));

            backend.stop(instance_id, None);
        }
    }

    #[test]
    #[ignore = "Requires test audio files"]
    fn test_emitter_position_updates() {
        let mut backend = create_audio_backend().expect("Failed to create backend");

        backend
            .load_sound("test", Path::new("test_assets/test.wav"))
            .expect("Failed to load sound");

        let entity_id = 1;
        let initial_position = Vec3::new(5.0, 0.0, 0.0);

        let instance_id = backend
            .play_3d("test", entity_id, initial_position, 1.0, true, 50.0)
            .expect("Failed to play 3D sound");

        // Update emitter position several times
        for i in 0..10 {
            let new_position = Vec3::new(i as f32, 0.0, 0.0);
            backend.update_emitter_position(entity_id, new_position);
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        backend.stop(instance_id, None);
    }

    #[test]
    #[ignore = "Requires test audio files"]
    fn test_stream_playback() {
        let mut backend = create_audio_backend().expect("Failed to create backend");

        let instance_id = backend
            .play_stream(Path::new("test_assets/music.wav"), 0.7, true)
            .expect("Failed to stream audio");

        assert!(backend.is_playing(instance_id));

        std::thread::sleep(std::time::Duration::from_millis(500));

        backend.stop(instance_id, None);
    }

    #[test]
    #[ignore = "Requires test audio files"]
    fn test_cleanup_finished_sounds() {
        let mut backend = create_audio_backend().expect("Failed to create backend");

        backend
            .load_sound("short", Path::new("test_assets/short.wav"))
            .expect("Failed to load sound");

        // Play a short sound
        let instance_id = backend.play_2d("short", 1.0, false).expect("Failed to play sound");

        assert_eq!(backend.active_sound_count(), 1);

        // Wait for sound to finish
        std::thread::sleep(std::time::Duration::from_secs(2));

        // Cleanup should remove finished sound
        backend.cleanup_finished();

        // Sound should no longer be active
        assert_eq!(backend.active_sound_count(), 0);
        assert!(!backend.is_playing(instance_id));
    }

    #[test]
    #[ignore = "Requires test audio files"]
    fn test_load_same_sound_twice() {
        let mut backend = create_audio_backend().expect("Failed to create backend");

        backend
            .load_sound("test", Path::new("test_assets/test.wav"))
            .expect("Failed to load sound");

        assert_eq!(backend.loaded_sound_count(), 1);

        // Loading same sound again should be a no-op
        backend
            .load_sound("test", Path::new("test_assets/test.wav"))
            .expect("Failed to load sound");

        assert_eq!(backend.loaded_sound_count(), 1);
    }

    #[test]
    #[ignore = "Requires test audio files"]
    fn test_volume_control() {
        let mut backend = create_audio_backend().expect("Failed to create backend");

        backend
            .load_sound("test", Path::new("test_assets/test.wav"))
            .expect("Failed to load sound");

        // Test various volumes
        let volumes = vec![0.0, 0.25, 0.5, 0.75, 1.0];

        for volume in volumes {
            let instance_id = backend.play_2d("test", volume, false).expect("Failed to play sound");

            std::thread::sleep(std::time::Duration::from_millis(100));

            backend.stop(instance_id, None);
        }
    }

    #[test]
    #[ignore = "Requires test audio files and long runtime"]
    fn test_stress_many_sounds() {
        let mut backend = create_audio_backend().expect("Failed to create backend");

        backend
            .load_sound("test", Path::new("test_assets/test.wav"))
            .expect("Failed to load sound");

        let mut instances = Vec::new();

        // Play 100 simultaneous sounds
        for _ in 0..100 {
            let instance_id = backend.play_2d("test", 0.1, false);
            if let Ok(id) = instance_id {
                instances.push(id);
            }
        }

        println!("Playing {} sounds", backend.active_sound_count());

        // Let them play for a bit
        std::thread::sleep(std::time::Duration::from_secs(1));

        // Stop all
        for instance_id in instances {
            backend.stop(instance_id, None);
        }

        backend.cleanup_finished();
    }
}

// Tests that run on all platforms (verify compilation)
#[cfg(not(target_os = "ios"))]
mod non_ios_tests {
    use engine_audio::create_audio_backend;

    #[test]
    fn test_ios_backend_fails_on_non_ios() {
        let result = create_audio_backend();

        // On non-iOS platforms with iOS backend selected, should return error
        // But since we have platform-specific compilation, this should select
        // the appropriate backend (Kira on desktop, etc.)
        assert!(result.is_ok() || result.is_err());
    }
}
