//! Audio ECS Components

use engine_core::ecs::Component;
use serde::{Deserialize, Serialize};

/// Sound component for entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sound {
    /// Sound asset name
    pub sound_name: String,

    /// Volume (0.0 - 1.0)
    pub volume: f32,

    /// Looping
    pub looping: bool,

    /// Auto-play on spawn
    pub auto_play: bool,

    /// 3D spatial audio
    pub spatial: bool,

    /// Max distance for 3D audio (beyond this, sound is silent)
    pub max_distance: f32,

    /// Enable Doppler effect for high-speed movement
    pub doppler_enabled: bool,

    /// Doppler scale factor (0.0 = disabled, 1.0 = realistic, higher = exaggerated)
    pub doppler_scale: f32,

    /// Current instance ID (if playing)
    #[serde(skip)]
    pub instance_id: Option<u64>,
}

impl Component for Sound {}

impl Default for Sound {
    fn default() -> Self {
        Self {
            sound_name: String::new(),
            volume: 1.0,
            looping: false,
            auto_play: false,
            spatial: true,
            max_distance: 100.0,
            doppler_enabled: true,
            doppler_scale: 1.0,
            instance_id: None,
        }
    }
}

impl Sound {
    /// Create a new sound component
    pub fn new(sound_name: impl Into<String>) -> Self {
        Self { sound_name: sound_name.into(), ..Default::default() }
    }

    /// Set volume
    pub fn with_volume(mut self, volume: f32) -> Self {
        self.volume = volume.clamp(0.0, 1.0);
        self
    }

    /// Enable looping
    pub fn looping(mut self) -> Self {
        self.looping = true;
        self
    }

    /// Enable auto-play
    pub fn auto_play(mut self) -> Self {
        self.auto_play = true;
        self
    }

    /// Configure as 3D spatial sound
    pub fn spatial_3d(mut self, max_distance: f32) -> Self {
        self.spatial = true;
        self.max_distance = max_distance;
        self
    }

    /// Configure as non-spatial (2D) sound
    pub fn non_spatial(mut self) -> Self {
        self.spatial = false;
        self
    }

    /// Enable Doppler effect with custom scale
    pub fn with_doppler(mut self, scale: f32) -> Self {
        self.doppler_enabled = true;
        self.doppler_scale = scale.clamp(0.0, 10.0);
        self
    }

    /// Disable Doppler effect
    pub fn without_doppler(mut self) -> Self {
        self.doppler_enabled = false;
        self
    }
}

/// Audio listener component (attach to camera)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AudioListener {
    /// Active (only one listener should be active)
    pub active: bool,
}

impl Component for AudioListener {}

impl AudioListener {
    /// Create an active audio listener
    pub fn new() -> Self {
        Self { active: true }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sound_builder() {
        let sound = Sound::new("test.wav").with_volume(0.8).looping().auto_play().spatial_3d(50.0);

        assert_eq!(sound.sound_name, "test.wav");
        assert_eq!(sound.volume, 0.8);
        assert!(sound.looping);
        assert!(sound.auto_play);
        assert!(sound.spatial);
        assert_eq!(sound.max_distance, 50.0);
    }

    #[test]
    fn test_sound_non_spatial() {
        let sound = Sound::new("ui.wav").non_spatial();

        assert!(!sound.spatial);
    }

    #[test]
    fn test_volume_clamping() {
        let sound = Sound::new("test.wav").with_volume(1.5);
        assert_eq!(sound.volume, 1.0);

        let sound = Sound::new("test.wav").with_volume(-0.5);
        assert_eq!(sound.volume, 0.0);
    }

    #[test]
    fn test_audio_listener() {
        let listener = AudioListener::new();
        assert!(listener.active);

        let listener = AudioListener::default();
        assert!(!listener.active);
    }

    #[test]
    fn test_doppler_enabled_by_default() {
        let sound = Sound::new("test.wav");
        assert!(sound.doppler_enabled);
        assert_eq!(sound.doppler_scale, 1.0);
    }

    #[test]
    fn test_with_doppler() {
        let sound = Sound::new("car.wav").with_doppler(0.5);
        assert!(sound.doppler_enabled);
        assert_eq!(sound.doppler_scale, 0.5);
    }

    #[test]
    fn test_without_doppler() {
        let sound = Sound::new("ambient.wav").without_doppler();
        assert!(!sound.doppler_enabled);
    }

    #[test]
    fn test_doppler_scale_clamping() {
        let sound = Sound::new("test.wav").with_doppler(15.0);
        assert_eq!(sound.doppler_scale, 10.0);

        let sound = Sound::new("test.wav").with_doppler(-1.0);
        assert_eq!(sound.doppler_scale, 0.0);
    }
}
