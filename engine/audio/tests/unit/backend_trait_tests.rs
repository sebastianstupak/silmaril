//! Unit tests for AudioBackend trait using mock implementations
//!
//! Tests the trait contract without platform-specific code.

use engine_audio::AudioBackend;
use glam::Vec3;
use std::collections::HashMap;
use std::path::Path;

/// Mock audio backend for testing
struct MockAudioBackend {
    loaded_sounds: HashMap<String, ()>,
    active_sounds: HashMap<u64, bool>,
    emitters: HashMap<u32, Vec3>,
    next_id: u64,
    listener_position: Vec3,
    listener_forward: Vec3,
    listener_up: Vec3,
}

impl AudioBackend for MockAudioBackend {
    fn new() -> engine_audio::AudioResult<Self> {
        Ok(Self {
            loaded_sounds: HashMap::new(),
            active_sounds: HashMap::new(),
            emitters: HashMap::new(),
            next_id: 0,
            listener_position: Vec3::ZERO,
            listener_forward: Vec3::new(0.0, 0.0, -1.0),
            listener_up: Vec3::new(0.0, 1.0, 0.0),
        })
    }

    fn load_sound(&mut self, name: &str, _path: &Path) -> engine_audio::AudioResult<()> {
        self.loaded_sounds.insert(name.to_string(), ());
        Ok(())
    }

    fn play_2d(
        &mut self,
        _sound_name: &str,
        _volume: f32,
        _looping: bool,
    ) -> engine_audio::AudioResult<u64> {
        let id = self.next_id;
        self.next_id += 1;
        self.active_sounds.insert(id, true);
        Ok(id)
    }

    fn play_3d(
        &mut self,
        entity: u32,
        _sound_name: &str,
        position: Vec3,
        _volume: f32,
        _looping: bool,
        _max_distance: f32,
    ) -> engine_audio::AudioResult<u64> {
        let id = self.next_id;
        self.next_id += 1;
        self.active_sounds.insert(id, true);
        self.emitters.insert(entity, position);
        Ok(id)
    }

    fn stop(&mut self, instance_id: u64, _fade_out_duration: Option<f32>) {
        self.active_sounds.remove(&instance_id);
    }

    fn set_listener_transform(&mut self, position: Vec3, forward: Vec3, up: Vec3) {
        self.listener_position = position;
        self.listener_forward = forward;
        self.listener_up = up;
    }

    fn update_emitter_position(&mut self, entity: u32, position: Vec3) {
        self.emitters.insert(entity, position);
    }

    fn remove_emitter(&mut self, entity: u32) {
        self.emitters.remove(&entity);
    }

    fn is_playing(&self, instance_id: u64) -> bool {
        self.active_sounds.get(&instance_id).copied().unwrap_or(false)
    }

    fn cleanup_finished(&mut self) {
        // Mock: remove all stopped sounds
        self.active_sounds.retain(|_, playing| *playing);
    }

    fn active_sound_count(&self) -> usize {
        self.active_sounds.len()
    }

    fn loaded_sound_count(&self) -> usize {
        self.loaded_sounds.len()
    }

    fn play_stream(
        &mut self,
        _path: &Path,
        _volume: f32,
        _looping: bool,
    ) -> engine_audio::AudioResult<u64> {
        let id = self.next_id;
        self.next_id += 1;
        self.active_sounds.insert(id, true);
        Ok(id)
    }

    fn add_effect(
        &mut self,
        _instance_id: u64,
        _effect: engine_audio::AudioEffect,
    ) -> engine_audio::AudioResult<usize> {
        // Mock implementation - just return index 0
        Ok(0)
    }

    fn remove_effect(&mut self, _instance_id: u64, _effect_index: usize) -> bool {
        // Mock implementation - always succeed
        true
    }

    fn clear_effects(&mut self, _instance_id: u64) {
        // Mock implementation - no-op
    }

    fn effect_count(&self, _instance_id: u64) -> usize {
        // Mock implementation - always 0
        0
    }

    fn set_pitch(&mut self, _instance_id: u64, _pitch: f32) {
        // Mock implementation - no-op
    }
}

#[test]
fn test_backend_creation() {
    let backend = MockAudioBackend::new();
    assert!(backend.is_ok());

    let backend = backend.unwrap();
    assert_eq!(backend.active_sound_count(), 0);
    assert_eq!(backend.loaded_sound_count(), 0);
}

#[test]
fn test_backend_load_sound() {
    let mut backend = MockAudioBackend::new().unwrap();

    let result = backend.load_sound("test", Path::new("test.wav"));
    assert!(result.is_ok());
    assert_eq!(backend.loaded_sound_count(), 1);

    // Load same sound again
    let result = backend.load_sound("test", Path::new("test.wav"));
    assert!(result.is_ok());
}

#[test]
fn test_backend_play_2d() {
    let mut backend = MockAudioBackend::new().unwrap();

    let instance_id = backend.play_2d("test", 1.0, false).unwrap();
    assert_eq!(backend.active_sound_count(), 1);
    assert!(backend.is_playing(instance_id));
}

#[test]
fn test_backend_play_3d() {
    let mut backend = MockAudioBackend::new().unwrap();

    let entity = 42;
    let position = Vec3::new(5.0, 0.0, 0.0);
    let instance_id = backend.play_3d(entity, "test", position, 1.0, false, 100.0).unwrap();

    assert_eq!(backend.active_sound_count(), 1);
    assert!(backend.is_playing(instance_id));
    assert!(backend.emitters.contains_key(&entity));
}

#[test]
fn test_backend_stop_sound() {
    let mut backend = MockAudioBackend::new().unwrap();

    let instance_id = backend.play_2d("test", 1.0, false).unwrap();
    assert!(backend.is_playing(instance_id));

    backend.stop(instance_id, None);
    assert!(!backend.is_playing(instance_id));
    assert_eq!(backend.active_sound_count(), 0);
}

#[test]
fn test_backend_listener_transform() {
    let mut backend = MockAudioBackend::new().unwrap();

    let position = Vec3::new(10.0, 5.0, 3.0);
    let forward = Vec3::new(0.0, 0.0, -1.0);
    let up = Vec3::new(0.0, 1.0, 0.0);

    backend.set_listener_transform(position, forward, up);

    assert_eq!(backend.listener_position, position);
    assert_eq!(backend.listener_forward, forward);
    assert_eq!(backend.listener_up, up);
}

#[test]
fn test_backend_update_emitter() {
    let mut backend = MockAudioBackend::new().unwrap();

    let entity = 42;
    let position1 = Vec3::new(1.0, 0.0, 0.0);
    let position2 = Vec3::new(5.0, 0.0, 0.0);

    backend.update_emitter_position(entity, position1);
    assert_eq!(backend.emitters.get(&entity), Some(&position1));

    backend.update_emitter_position(entity, position2);
    assert_eq!(backend.emitters.get(&entity), Some(&position2));
}

#[test]
fn test_backend_remove_emitter() {
    let mut backend = MockAudioBackend::new().unwrap();

    let entity = 42;
    backend.update_emitter_position(entity, Vec3::ZERO);
    assert!(backend.emitters.contains_key(&entity));

    backend.remove_emitter(entity);
    assert!(!backend.emitters.contains_key(&entity));
}

#[test]
fn test_backend_multiple_sounds() {
    let mut backend = MockAudioBackend::new().unwrap();

    let id1 = backend.play_2d("test1", 1.0, false).unwrap();
    let id2 = backend.play_2d("test2", 1.0, false).unwrap();
    let id3 = backend.play_2d("test3", 1.0, false).unwrap();

    assert_eq!(backend.active_sound_count(), 3);
    assert!(backend.is_playing(id1));
    assert!(backend.is_playing(id2));
    assert!(backend.is_playing(id3));

    backend.stop(id2, None);
    assert_eq!(backend.active_sound_count(), 2);
    assert!(!backend.is_playing(id2));
}

#[test]
fn test_backend_cleanup_finished() {
    let mut backend = MockAudioBackend::new().unwrap();

    let id1 = backend.play_2d("test1", 1.0, false).unwrap();
    let id2 = backend.play_2d("test2", 1.0, false).unwrap();

    backend.stop(id1, None);

    backend.cleanup_finished();

    // After cleanup, only playing sound should remain
    assert!(backend.is_playing(id2));
}

#[test]
fn test_backend_instance_id_uniqueness() {
    let mut backend = MockAudioBackend::new().unwrap();

    let mut ids = Vec::new();
    for _ in 0..100 {
        ids.push(backend.play_2d("test", 1.0, false).unwrap());
    }

    // All IDs should be unique
    let mut unique_ids = ids.clone();
    unique_ids.sort();
    unique_ids.dedup();
    assert_eq!(ids.len(), unique_ids.len());
}

#[test]
fn test_backend_emitter_persistence() {
    let mut backend = MockAudioBackend::new().unwrap();

    let entity1 = 1;
    let entity2 = 2;

    backend
        .play_3d(entity1, "test", Vec3::new(1.0, 0.0, 0.0), 1.0, false, 100.0)
        .unwrap();
    backend
        .play_3d(entity2, "test", Vec3::new(2.0, 0.0, 0.0), 1.0, false, 100.0)
        .unwrap();

    assert!(backend.emitters.contains_key(&entity1));
    assert!(backend.emitters.contains_key(&entity2));

    backend.remove_emitter(entity1);
    assert!(!backend.emitters.contains_key(&entity1));
    assert!(backend.emitters.contains_key(&entity2));
}
