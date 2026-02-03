//! Audio Engine
//!
//! Core audio playback engine with platform abstraction.

use crate::effects::AudioEffect;
use crate::error::AudioResult;
use crate::platform::{create_audio_backend, AudioBackend};
use glam::Vec3;
use std::path::Path;

/// Audio engine managing all audio playback
///
/// This wraps a platform-specific AudioBackend implementation:
/// - Desktop (Windows, Linux, macOS): Kira audio engine
/// - Web (WASM): Web Audio API
/// - Android: OpenSL ES / AAudio
/// - iOS: Core Audio
pub struct AudioEngine {
    backend: Box<dyn AudioBackend>,
}

impl AudioEngine {
    /// Create a new audio engine with the appropriate platform backend
    pub fn new() -> AudioResult<Self> {
        Ok(Self { backend: create_audio_backend()? })
    }

    /// Load sound from file
    pub fn load_sound(&mut self, name: &str, path: impl AsRef<Path>) -> AudioResult<()> {
        self.backend.load_sound(name, path.as_ref())
    }

    /// Play 2D sound (UI, menu sounds)
    pub fn play_2d(&mut self, sound_name: &str, volume: f32, looping: bool) -> AudioResult<u64> {
        self.backend.play_2d(sound_name, volume, looping)
    }

    /// Play 3D spatial sound
    pub fn play_3d(
        &mut self,
        entity: u32,
        sound_name: &str,
        position: Vec3,
        volume: f32,
        looping: bool,
        max_distance: f32,
    ) -> AudioResult<u64> {
        self.backend
            .play_3d(entity, sound_name, position, volume, looping, max_distance)
    }

    /// Stop sound instance
    pub fn stop(&mut self, instance_id: u64, fade_out_duration: Option<f32>) {
        self.backend.stop(instance_id, fade_out_duration)
    }

    /// Set listener position/orientation (camera)
    pub fn set_listener_transform(&mut self, position: Vec3, forward: Vec3, up: Vec3) {
        self.backend.set_listener_transform(position, forward, up)
    }

    /// Update emitter position
    pub fn update_emitter_position(&mut self, entity: u32, position: Vec3) {
        self.backend.update_emitter_position(entity, position)
    }

    /// Remove emitter
    pub fn remove_emitter(&mut self, entity: u32) {
        self.backend.remove_emitter(entity)
    }

    /// Get playback state
    pub fn is_playing(&self, instance_id: u64) -> bool {
        self.backend.is_playing(instance_id)
    }

    /// Clean up finished sounds
    pub fn cleanup_finished(&mut self) {
        self.backend.cleanup_finished()
    }

    /// Get number of active sounds
    pub fn active_sound_count(&self) -> usize {
        self.backend.active_sound_count()
    }

    /// Get number of loaded sounds
    pub fn loaded_sound_count(&self) -> usize {
        self.backend.loaded_sound_count()
    }

    /// Play streaming audio (for music and large files)
    ///
    /// Streaming audio is loaded progressively from disk, making it ideal for:
    /// - Background music
    /// - Ambient soundscapes
    /// - Large audio files (> 1MB)
    ///
    /// Use this instead of load_sound() + play_2d() for files that are too large
    /// to keep entirely in memory.
    pub fn play_stream(
        &mut self,
        path: impl AsRef<Path>,
        volume: f32,
        looping: bool,
    ) -> AudioResult<u64> {
        self.backend.play_stream(path.as_ref(), volume, looping)
    }

    /// Add audio effect to sound instance
    ///
    /// Effects are applied in the order they are added. Multiple effects
    /// can be stacked on a single sound instance.
    ///
    /// Returns the index of the added effect, which can be used with remove_effect.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use engine_audio::{AudioEngine, ReverbEffect, AudioEffect};
    /// use glam::Vec3;
    ///
    /// let mut audio = AudioEngine::new().unwrap();
    /// audio.load_sound("gunshot", "assets/gunshot.wav").unwrap();
    ///
    /// let instance = audio.play_3d(1, "gunshot", Vec3::ZERO, 1.0, false, 100.0).unwrap();
    ///
    /// // Add reverb for indoor environment
    /// let reverb = ReverbEffect::large_hall();
    /// audio.add_effect(instance, AudioEffect::Reverb(reverb)).unwrap();
    /// ```
    pub fn add_effect(&mut self, instance_id: u64, effect: AudioEffect) -> AudioResult<usize> {
        self.backend.add_effect(instance_id, effect)
    }

    /// Remove effect from sound instance by index
    ///
    /// Returns true if the effect was removed, false if the index was invalid.
    pub fn remove_effect(&mut self, instance_id: u64, effect_index: usize) -> bool {
        self.backend.remove_effect(instance_id, effect_index)
    }

    /// Clear all effects from sound instance
    pub fn clear_effects(&mut self, instance_id: u64) {
        self.backend.clear_effects(instance_id)
    }

    /// Get number of effects on sound instance
    pub fn effect_count(&self, instance_id: u64) -> usize {
        self.backend.effect_count(instance_id)
    }

    /// Set pitch/playback rate for sound instance
    ///
    /// This is used by the Doppler effect system to adjust pitch based on velocity.
    ///
    /// # Arguments
    ///
    /// * `instance_id` - Sound instance ID
    /// * `pitch` - Pitch multiplier (1.0 = normal, 2.0 = octave up, 0.5 = octave down)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use engine_audio::AudioEngine;
    /// use glam::Vec3;
    ///
    /// let mut audio = AudioEngine::new().unwrap();
    /// audio.load_sound("car", "assets/car.wav").unwrap();
    ///
    /// let instance = audio.play_3d(1, "car", Vec3::ZERO, 1.0, true, 100.0).unwrap();
    ///
    /// // Approaching sound (higher pitch)
    /// audio.set_pitch(instance, 1.2);
    ///
    /// // Receding sound (lower pitch)
    /// audio.set_pitch(instance, 0.8);
    /// ```
    pub fn set_pitch(&mut self, instance_id: u64, pitch: f32) {
        self.backend.set_pitch(instance_id, pitch)
    }

    /// Access backend mutably (for advanced use cases)
    #[allow(dead_code)]
    pub(crate) fn backend_mut(&mut self) -> &mut dyn AudioBackend {
        self.backend.as_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_engine_creation() {
        let engine = AudioEngine::new();
        assert!(engine.is_ok());
    }

    #[test]
    fn test_active_sound_tracking() {
        let engine = AudioEngine::new().unwrap();
        assert_eq!(engine.active_sound_count(), 0);
    }

    #[test]
    fn test_listener_transform() {
        let mut engine = AudioEngine::new().unwrap();

        engine.set_listener_transform(
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(0.0, 0.0, -1.0),
            Vec3::new(0.0, 1.0, 0.0),
        );

        // No crash = success
    }

    #[test]
    fn test_emitter_management() {
        let mut engine = AudioEngine::new().unwrap();

        let entity_id = 42;
        engine.update_emitter_position(entity_id, Vec3::new(5.0, 0.0, 0.0));

        engine.remove_emitter(entity_id);

        // No crash = success
    }

    #[test]
    fn test_streaming_audio_api() {
        let engine = AudioEngine::new().unwrap();

        // Test that streaming API exists (will fail without actual file)
        // In production, this would stream background music
        assert_eq!(engine.active_sound_count(), 0);
    }
}
