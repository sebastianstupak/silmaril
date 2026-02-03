//! Platform-specific audio backend abstraction
//!
//! This module provides cross-platform audio support through trait abstraction.
//! Each platform implements the AudioBackend trait with its native audio API.

use crate::effects::AudioEffect;
use crate::error::AudioResult;
use glam::Vec3;
use std::path::Path;

#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android"), not(target_os = "ios")))]
mod kira;

#[cfg(target_arch = "wasm32")]
mod web;

#[cfg(target_os = "android")]
mod android;

#[cfg(target_os = "ios")]
mod ios;

/// Platform-agnostic audio backend trait
///
/// This trait abstracts all audio operations to support multiple platforms:
/// - Desktop (Windows, Linux, macOS): Kira audio engine
/// - Web (WASM): Web Audio API
/// - Android: OpenSL ES / AAudio
/// - iOS: Core Audio
pub trait AudioBackend: Send + Sync {
    /// Create a new audio backend
    fn new() -> AudioResult<Self>
    where
        Self: Sized;

    /// Load sound from file
    fn load_sound(&mut self, name: &str, path: &Path) -> AudioResult<()>;

    /// Play 2D sound (non-spatial)
    fn play_2d(&mut self, sound_name: &str, volume: f32, looping: bool) -> AudioResult<u64>;

    /// Play 3D spatial sound
    fn play_3d(
        &mut self,
        entity: u32,
        sound_name: &str,
        position: Vec3,
        volume: f32,
        looping: bool,
        max_distance: f32,
    ) -> AudioResult<u64>;

    /// Stop sound instance
    fn stop(&mut self, instance_id: u64, fade_out_duration: Option<f32>);

    /// Set listener position and orientation
    fn set_listener_transform(&mut self, position: Vec3, forward: Vec3, up: Vec3);

    /// Update emitter position
    fn update_emitter_position(&mut self, entity: u32, position: Vec3);

    /// Remove emitter
    fn remove_emitter(&mut self, entity: u32);

    /// Check if sound instance is playing
    fn is_playing(&self, instance_id: u64) -> bool;

    /// Clean up finished sounds
    fn cleanup_finished(&mut self);

    /// Get number of active sounds
    fn active_sound_count(&self) -> usize;

    /// Get number of loaded sounds
    fn loaded_sound_count(&self) -> usize;

    /// Play streaming audio (for music and large files)
    fn play_stream(&mut self, path: &Path, volume: f32, looping: bool) -> AudioResult<u64>;

    /// Add effect to sound instance
    ///
    /// Effects are applied in the order they are added. Multiple effects
    /// can be active on a single sound instance.
    fn add_effect(&mut self, instance_id: u64, effect: AudioEffect) -> AudioResult<usize>;

    /// Remove effect from sound instance by index
    ///
    /// Returns true if the effect was removed, false if the index was invalid.
    fn remove_effect(&mut self, instance_id: u64, effect_index: usize) -> bool;

    /// Clear all effects from sound instance
    fn clear_effects(&mut self, instance_id: u64);

    /// Get number of effects on sound instance
    fn effect_count(&self, instance_id: u64) -> usize;

    /// Set pitch/playback rate for sound instance
    ///
    /// # Arguments
    ///
    /// * `instance_id` - Sound instance ID
    /// * `pitch` - Pitch multiplier (1.0 = normal, 2.0 = double speed/octave up, 0.5 = half speed/octave down)
    ///
    /// Typical range: 0.5 - 2.0 to avoid audio artifacts
    fn set_pitch(&mut self, instance_id: u64, pitch: f32);
}

/// Create platform-specific audio backend
pub fn create_audio_backend() -> AudioResult<Box<dyn AudioBackend>> {
    #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android"), not(target_os = "ios")))]
    {
        Ok(Box::new(kira::KiraAudioBackend::new()?))
    }

    #[cfg(target_arch = "wasm32")]
    {
        Ok(Box::new(web::WebAudioBackend::new()?))
    }

    #[cfg(target_os = "android")]
    {
        Ok(Box::new(android::AndroidAudioBackend::new()?))
    }

    #[cfg(target_os = "ios")]
    {
        Ok(Box::new(ios::IOSAudioBackend::new()?))
    }
}
