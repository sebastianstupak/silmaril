//! Unit tests for audio components (Sound, AudioListener)
//!
//! Tests component behavior in isolation without dependencies.

use engine_audio::{AudioListener, Sound};

#[test]
fn test_sound_default() {
    let sound = Sound::default();

    assert_eq!(sound.sound_name, "");
    assert_eq!(sound.volume, 1.0);
    assert!(!sound.looping);
    assert!(!sound.auto_play);
    assert!(sound.spatial);
    assert_eq!(sound.max_distance, 100.0);
    assert!(sound.instance_id.is_none());
}

#[test]
fn test_sound_builder_pattern() {
    let sound = Sound::new("explosion.wav")
        .with_volume(0.75)
        .looping()
        .auto_play()
        .spatial_3d(150.0);

    assert_eq!(sound.sound_name, "explosion.wav");
    assert_eq!(sound.volume, 0.75);
    assert!(sound.looping);
    assert!(sound.auto_play);
    assert!(sound.spatial);
    assert_eq!(sound.max_distance, 150.0);
}

#[test]
fn test_sound_non_spatial() {
    let sound = Sound::new("ui_click.wav").non_spatial();

    assert!(!sound.spatial);
    assert_eq!(sound.sound_name, "ui_click.wav");
}

#[test]
fn test_sound_volume_clamping() {
    // Test upper bound
    let sound = Sound::new("test.wav").with_volume(2.5);
    assert_eq!(sound.volume, 1.0);

    // Test lower bound
    let sound = Sound::new("test.wav").with_volume(-0.5);
    assert_eq!(sound.volume, 0.0);

    // Test normal range
    let sound = Sound::new("test.wav").with_volume(0.5);
    assert_eq!(sound.volume, 0.5);
}

#[test]
fn test_sound_chaining() {
    // Test that builder methods can be chained in any order
    let sound1 = Sound::new("test.wav").looping().auto_play().with_volume(0.8);
    let sound2 = Sound::new("test.wav").with_volume(0.8).auto_play().looping();

    assert_eq!(sound1.volume, sound2.volume);
    assert_eq!(sound1.looping, sound2.looping);
    assert_eq!(sound1.auto_play, sound2.auto_play);
}

#[test]
fn test_sound_spatial_3d() {
    let sound = Sound::new("ambient.wav").spatial_3d(200.0);

    assert!(sound.spatial);
    assert_eq!(sound.max_distance, 200.0);
}

#[test]
fn test_sound_clone() {
    let sound1 = Sound::new("test.wav").with_volume(0.5).looping();
    let sound2 = sound1.clone();

    assert_eq!(sound1.sound_name, sound2.sound_name);
    assert_eq!(sound1.volume, sound2.volume);
    assert_eq!(sound1.looping, sound2.looping);
}

#[test]
fn test_audio_listener_new() {
    let listener = AudioListener::new();

    assert!(listener.active);
}

#[test]
fn test_audio_listener_default() {
    let listener = AudioListener::default();

    assert!(!listener.active);
}

#[test]
fn test_audio_listener_clone() {
    let listener1 = AudioListener::new();
    let listener2 = listener1.clone();

    assert_eq!(listener1.active, listener2.active);
}

#[cfg(test)]
mod serialization_tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_sound_serialization() {
        let sound = Sound::new("test.wav").with_volume(0.75).looping();

        let json = serde_json::to_string(&sound).unwrap();
        let deserialized: Sound = serde_json::from_str(&json).unwrap();

        assert_eq!(sound.sound_name, deserialized.sound_name);
        assert_eq!(sound.volume, deserialized.volume);
        assert_eq!(sound.looping, deserialized.looping);
    }

    #[test]
    fn test_sound_instance_id_not_serialized() {
        let mut sound = Sound::new("test.wav");
        sound.instance_id = Some(12345);

        let json = serde_json::to_string(&sound).unwrap();
        let deserialized: Sound = serde_json::from_str(&json).unwrap();

        // instance_id should be skipped in serialization
        assert!(deserialized.instance_id.is_none());
    }

    #[test]
    fn test_audio_listener_serialization() {
        let listener = AudioListener::new();

        let json = serde_json::to_string(&listener).unwrap();
        let deserialized: AudioListener = serde_json::from_str(&json).unwrap();

        assert_eq!(listener.active, deserialized.active);
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_volume_always_clamped(volume in -10.0f32..10.0f32) {
            let sound = Sound::new("test.wav").with_volume(volume);
            prop_assert!(sound.volume >= 0.0 && sound.volume <= 1.0);
        }

        #[test]
        fn test_max_distance_always_positive(distance in 0.1f32..10000.0f32) {
            let sound = Sound::new("test.wav").spatial_3d(distance);
            prop_assert!(sound.max_distance > 0.0);
            prop_assert_eq!(sound.max_distance, distance);
        }

        #[test]
        fn test_sound_name_preserved(name in "[a-zA-Z0-9_\\.]{1,50}") {
            let sound = Sound::new(&name);
            prop_assert_eq!(sound.sound_name, name);
        }
    }
}
