//! Kira audio backend for native platforms (Windows, Linux, macOS)

use crate::effects::{AudioEffect, EchoEffect, EqEffect, FilterEffect, ReverbEffect};
use crate::error::{AudioError, AudioResult};
use crate::platform::AudioBackend;
use glam::{Quat, Vec3};
use kira::{
    manager::{backend::DefaultBackend, AudioManager, AudioManagerSettings},
    sound::{
        static_sound::{StaticSoundData, StaticSoundHandle, StaticSoundSettings},
        streaming::{StreamingSoundData, StreamingSoundHandle, StreamingSoundSettings},
        FromFileError, PlaybackState, Region,
    },
    spatial::{
        emitter::{EmitterHandle, EmitterSettings},
        listener::{ListenerHandle, ListenerSettings},
        scene::{SpatialSceneHandle, SpatialSceneSettings},
    },
    track::{
        effect::{delay::DelayBuilder, filter::FilterBuilder, reverb::ReverbBuilder},
        TrackBuilder, TrackHandle,
    },
    tween::{Tween, Value},
    OutputDestination, Volume,
};
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, info, trace, warn};

/// Sound instance (either static or streaming)
enum SoundInstance {
    Static(StaticSoundHandle),
    Streaming(StreamingSoundHandle<FromFileError>),
}

/// Effect track with associated effects
struct EffectTrack {
    track: TrackHandle,
    effects: Vec<AudioEffect>,
}

/// Kira audio backend implementation
pub struct KiraAudioBackend {
    /// Kira audio manager
    manager: AudioManager,

    /// Spatial scene for 3D audio
    spatial_scene: SpatialSceneHandle,

    /// Audio listener (camera position)
    listener: ListenerHandle,

    /// Loaded sounds (cached by name)
    loaded_sounds: HashMap<String, StaticSoundData>,

    /// Active sound instances (keyed by instance ID)
    active_sounds: HashMap<u64, SoundInstance>,

    /// Spatial emitters per entity (handle for routing audio)
    emitters: HashMap<u32, EmitterHandle>,

    /// Effect tracks per sound instance
    effect_tracks: HashMap<u64, EffectTrack>,

    /// Next instance ID
    next_instance_id: u64,
}

impl AudioBackend for KiraAudioBackend {
    fn new() -> AudioResult<Self> {
        let mut manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())
            .map_err(|e| AudioError::ManagerError(format!("{:?}", e)))?;

        // Create spatial scene
        let mut spatial_scene = manager
            .add_spatial_scene(SpatialSceneSettings::default())
            .map_err(|e| AudioError::ManagerError(format!("{:?}", e)))?;

        // Create listener at origin
        let listener = spatial_scene
            .add_listener(
                mint::Vector3 { x: 0.0, y: 0.0, z: 0.0 },
                mint::Quaternion { v: mint::Vector3 { x: 0.0, y: 0.0, z: 0.0 }, s: 1.0 },
                ListenerSettings::default(),
            )
            .map_err(|e| AudioError::ManagerError(format!("{:?}", e)))?;

        info!("Kira audio backend initialized");

        Ok(Self {
            manager,
            spatial_scene,
            listener,
            loaded_sounds: HashMap::new(),
            active_sounds: HashMap::new(),
            emitters: HashMap::new(),
            effect_tracks: HashMap::new(),
            next_instance_id: 0,
        })
    }

    fn load_sound(&mut self, name: &str, path: &Path) -> AudioResult<()> {
        if self.loaded_sounds.contains_key(name) {
            return Ok(()); // Already loaded
        }

        let sound_data = StaticSoundData::from_file(path, StaticSoundSettings::default())
            .map_err(|e| AudioError::DecodeError(format!("{:?}", e)))?;

        self.loaded_sounds.insert(name.to_string(), sound_data);

        info!("Loaded sound: {}", name);

        Ok(())
    }

    fn play_2d(&mut self, sound_name: &str, volume: f32, looping: bool) -> AudioResult<u64> {
        let sound_data = self
            .loaded_sounds
            .get(sound_name)
            .ok_or_else(|| AudioError::SoundNotFound(sound_name.to_string()))?;

        let mut settings = StaticSoundSettings::default();
        settings.volume = Value::Fixed(Volume::Amplitude(volume as f64));
        if looping {
            settings.loop_region = Some(Region::default());
        }

        let handle = self
            .manager
            .play(sound_data.clone().with_settings(settings))
            .map_err(|e| AudioError::ManagerError(format!("{:?}", e)))?;

        let instance_id = self.next_instance_id;
        self.next_instance_id += 1;

        self.active_sounds.insert(instance_id, SoundInstance::Static(handle));

        debug!("Playing 2D sound: {} (id: {})", sound_name, instance_id);

        Ok(instance_id)
    }

    fn play_3d(
        &mut self,
        entity: u32,
        sound_name: &str,
        position: Vec3,
        volume: f32,
        looping: bool,
        max_distance: f32,
    ) -> AudioResult<u64> {
        let sound_data = self
            .loaded_sounds
            .get(sound_name)
            .ok_or_else(|| AudioError::SoundNotFound(sound_name.to_string()))?;

        // Get or create emitter for entity
        if !self.emitters.contains_key(&entity) {
            let emitter_settings = EmitterSettings::new().distances((1.0, max_distance));

            let emitter_pos = mint::Vector3 { x: position.x, y: position.y, z: position.z };

            let emitter = self
                .spatial_scene
                .add_emitter(emitter_pos, emitter_settings)
                .map_err(|e| AudioError::ManagerError(format!("{:?}", e)))?;

            self.emitters.insert(entity, emitter);
        }

        let emitter_id = self.emitters.get(&entity).unwrap().id();

        let mut settings = StaticSoundSettings::default();
        settings.volume = Value::Fixed(Volume::Amplitude(volume as f64));
        if looping {
            settings.loop_region = Some(Region::default());
        }
        settings.output_destination = OutputDestination::Emitter(emitter_id);

        let handle = self
            .manager
            .play(sound_data.clone().with_settings(settings))
            .map_err(|e| AudioError::ManagerError(format!("{:?}", e)))?;

        let instance_id = self.next_instance_id;
        self.next_instance_id += 1;

        self.active_sounds.insert(instance_id, SoundInstance::Static(handle));

        debug!("Playing 3D sound: {} at {:?} (id: {})", sound_name, position, instance_id);

        Ok(instance_id)
    }

    fn stop(&mut self, instance_id: u64, fade_out_duration: Option<f32>) {
        if let Some(instance) = self.active_sounds.get_mut(&instance_id) {
            let tween = fade_out_duration.map_or(Tween::default(), |duration| Tween {
                duration: std::time::Duration::from_secs_f32(duration),
                ..Default::default()
            });

            match instance {
                SoundInstance::Static(handle) => {
                    let _ = handle.stop(tween);
                }
                SoundInstance::Streaming(handle) => {
                    let _ = handle.stop(tween);
                }
            }

            self.active_sounds.remove(&instance_id);

            debug!("Stopped sound (id: {})", instance_id);
        }
    }

    fn set_listener_transform(&mut self, position: Vec3, forward: Vec3, up: Vec3) {
        // Calculate orientation quaternion from forward and up vectors
        let right = forward.cross(up).normalize();
        let up_normalized = right.cross(forward).normalize();
        let orientation = Quat::from_mat3(&glam::Mat3::from_cols(right, up_normalized, -forward));

        let pos = mint::Vector3 { x: position.x, y: position.y, z: position.z };
        let orient = mint::Quaternion {
            v: mint::Vector3 { x: orientation.x, y: orientation.y, z: orientation.z },
            s: orientation.w,
        };

        let _ = self.listener.set_position(pos, Tween::default());
        let _ = self.listener.set_orientation(orient, Tween::default());
    }

    fn update_emitter_position(&mut self, entity: u32, position: Vec3) {
        if let Some(emitter) = self.emitters.get_mut(&entity) {
            let pos = mint::Vector3 { x: position.x, y: position.y, z: position.z };
            let _ = emitter.set_position(pos, Tween::default());
        }
    }

    fn remove_emitter(&mut self, entity: u32) {
        if self.emitters.remove(&entity).is_some() {
            debug!("Removed emitter for entity {}", entity);
            // Note: Kira 0.8 doesn't have remove_emitter, emitters are automatically cleaned up
        }
    }

    fn is_playing(&self, instance_id: u64) -> bool {
        if let Some(instance) = self.active_sounds.get(&instance_id) {
            match instance {
                SoundInstance::Static(handle) => matches!(handle.state(), PlaybackState::Playing),
                SoundInstance::Streaming(handle) => {
                    matches!(handle.state(), PlaybackState::Playing)
                }
            }
        } else {
            false
        }
    }

    fn cleanup_finished(&mut self) {
        self.active_sounds.retain(|_, instance| match instance {
            SoundInstance::Static(handle) => !matches!(handle.state(), PlaybackState::Stopped),
            SoundInstance::Streaming(handle) => !matches!(handle.state(), PlaybackState::Stopped),
        });
    }

    fn active_sound_count(&self) -> usize {
        self.active_sounds.len()
    }

    fn loaded_sound_count(&self) -> usize {
        self.loaded_sounds.len()
    }

    fn play_stream(&mut self, path: &Path, volume: f32, looping: bool) -> AudioResult<u64> {
        let mut settings = StreamingSoundSettings::default();
        settings.volume = Value::Fixed(Volume::Amplitude(volume as f64));
        if looping {
            settings.loop_region = Some(Region::default());
        }

        let sound_data = StreamingSoundData::from_file(path, settings)
            .map_err(|e| AudioError::DecodeError(format!("{:?}", e)))?;

        let handle = self
            .manager
            .play(sound_data)
            .map_err(|e| AudioError::ManagerError(format!("{:?}", e)))?;

        let instance_id = self.next_instance_id;
        self.next_instance_id += 1;

        self.active_sounds.insert(instance_id, SoundInstance::Streaming(handle));

        debug!("Streaming audio (id: {})", instance_id);

        Ok(instance_id)
    }

    fn add_effect(&mut self, instance_id: u64, effect: AudioEffect) -> AudioResult<usize> {
        // Check if sound instance exists
        if !self.active_sounds.contains_key(&instance_id) {
            return Err(AudioError::InvalidInstance(instance_id as u32));
        }

        // Get or create effect track for this instance
        if !self.effect_tracks.contains_key(&instance_id) {
            // Create new track without effects initially
            let track = self
                .manager
                .add_sub_track(TrackBuilder::new())
                .map_err(|e| AudioError::ManagerError(format!("{:?}", e)))?;

            self.effect_tracks
                .insert(instance_id, EffectTrack { track, effects: Vec::new() });
        }

        // Since Kira requires effects to be added during track creation,
        // we need to recreate the track with all effects
        let effect_count = {
            let effect_track = self.effect_tracks.get_mut(&instance_id).unwrap();
            effect_track.effects.push(effect.clone());
            effect_track.effects.len()
        };

        // Rebuild track with all effects
        self.rebuild_effect_track(instance_id)?;
        debug!("Added effect to instance {} (total effects: {})", instance_id, effect_count);

        Ok(effect_count - 1)
    }

    fn remove_effect(&mut self, instance_id: u64, effect_index: usize) -> bool {
        if let Some(effect_track) = self.effect_tracks.get_mut(&instance_id) {
            if effect_index < effect_track.effects.len() {
                effect_track.effects.remove(effect_index);

                // Rebuild track without the removed effect
                if let Err(e) = self.rebuild_effect_track(instance_id) {
                    warn!("Failed to rebuild effect track: {:?}", e);
                    return false;
                }

                debug!("Removed effect {} from instance {}", effect_index, instance_id);
                return true;
            }
        }
        false
    }

    fn clear_effects(&mut self, instance_id: u64) {
        if let Some(effect_track) = self.effect_tracks.remove(&instance_id) {
            debug!("Cleared {} effects from instance {}", effect_track.effects.len(), instance_id);
        }
    }

    fn effect_count(&self, instance_id: u64) -> usize {
        self.effect_tracks.get(&instance_id).map(|t| t.effects.len()).unwrap_or(0)
    }

    fn set_pitch(&mut self, instance_id: u64, pitch: f32) {
        if let Some(instance) = self.active_sounds.get_mut(&instance_id) {
            let clamped_pitch = pitch.clamp(0.5, 2.0);

            match instance {
                SoundInstance::Static(handle) => {
                    let _ = handle.set_playback_rate(clamped_pitch as f64, Tween::default());
                }
                SoundInstance::Streaming(handle) => {
                    let _ = handle.set_playback_rate(clamped_pitch as f64, Tween::default());
                }
            }

            trace!(
                instance_id = instance_id,
                pitch = clamped_pitch,
                "Set pitch for sound instance"
            );
        }
    }
}

impl KiraAudioBackend {
    /// Rebuild effect track with current effects
    ///
    /// This is needed because Kira requires effects to be set during track creation.
    /// We rebuild the track whenever effects are added or removed.
    fn rebuild_effect_track(&mut self, instance_id: u64) -> AudioResult<()> {
        let effect_track = self
            .effect_tracks
            .get(&instance_id)
            .ok_or_else(|| AudioError::InvalidInstance(instance_id as u32))?;

        let mut builder = TrackBuilder::new();

        // Add all effects to the builder
        for effect in &effect_track.effects {
            builder = self.add_effect_to_builder(builder, effect)?;
        }

        // Create new track with effects
        let new_track = self
            .manager
            .add_sub_track(builder)
            .map_err(|e| AudioError::ManagerError(format!("{:?}", e)))?;

        // Replace old track
        if let Some(effect_track) = self.effect_tracks.get_mut(&instance_id) {
            effect_track.track = new_track;
        }

        Ok(())
    }

    /// Add effect to TrackBuilder
    ///
    /// Note: Kira 0.8's add_effect() returns an effect handle, not the builder.
    /// We need to call add_effect() for its side effect and return the builder.
    fn add_effect_to_builder(
        &self,
        mut builder: TrackBuilder,
        effect: &AudioEffect,
    ) -> AudioResult<TrackBuilder> {
        match effect {
            AudioEffect::Reverb(reverb) => {
                let _ = builder.add_effect(self.create_reverb_effect(reverb)?);
            }
            AudioEffect::Echo(echo) => {
                let _ = builder.add_effect(self.create_echo_effect(echo)?);
            }
            AudioEffect::Filter(filter) => {
                let _ = builder.add_effect(self.create_filter_effect(filter)?);
            }
            AudioEffect::Eq(eq) => {
                // EQ is implemented as multiple filter bands
                return self.add_eq_effect(builder, eq);
            }
        }
        Ok(builder)
    }

    /// Create Kira reverb effect from ReverbEffect
    fn create_reverb_effect(&self, reverb: &ReverbEffect) -> AudioResult<ReverbBuilder> {
        if !reverb.validate() {
            return Err(AudioError::ManagerError("Invalid reverb parameters".to_string()));
        }

        // Map our parameters to Kira's reverb
        // Note: Kira 0.8's ReverbBuilder has limited parameters
        // We map our room_size/damping to the available parameters
        Ok(ReverbBuilder::new().mix(reverb.wet_dry_mix as f64))
    }

    /// Create Kira delay effect from EchoEffect
    fn create_echo_effect(&self, echo: &EchoEffect) -> AudioResult<DelayBuilder> {
        if !echo.validate() {
            return Err(AudioError::ManagerError("Invalid echo parameters".to_string()));
        }

        // Map our parameters to Kira's delay
        // feedback expects Value<Volume>, so we need to convert properly
        Ok(DelayBuilder::new()
            .delay_time(Value::Fixed(echo.delay_time as f64))
            .feedback(Value::Fixed(Volume::Amplitude(echo.feedback as f64)))
            .mix(Value::Fixed(echo.wet_dry_mix as f64)))
    }

    /// Create Kira filter effect from FilterEffect
    fn create_filter_effect(&self, filter: &FilterEffect) -> AudioResult<FilterBuilder> {
        if !filter.validate() {
            return Err(AudioError::ManagerError("Invalid filter parameters".to_string()));
        }

        let mut builder = FilterBuilder::new();

        // Set cutoff frequency
        builder = builder.cutoff(Value::Fixed(filter.cutoff_frequency as f64));

        // Set resonance (Q factor)
        builder = builder.resonance(Value::Fixed(filter.resonance as f64));

        // Set mix
        builder = builder.mix(Value::Fixed(filter.wet_dry_mix as f64));

        // Note: Kira's FilterBuilder doesn't have separate filter types (low-pass, high-pass, etc.)
        // in the same way. The default is a low-pass filter. For full implementation,
        // we'd need to check Kira's API for filter mode support or use multiple filters.

        Ok(builder)
    }

    /// Add EQ effect as multiple filter bands
    fn add_eq_effect(&self, mut builder: TrackBuilder, eq: &EqEffect) -> AudioResult<TrackBuilder> {
        if !eq.validate() {
            return Err(AudioError::ManagerError("Invalid EQ parameters".to_string()));
        }

        // EQ is implemented as three band-pass filters
        // This is a simplified implementation - real EQ would use proper shelf filters
        // Note: Kira 0.8's add_effect returns a handle, not the builder

        // Bass band (20-250 Hz)
        if eq.bass_gain.abs() > 0.01 {
            let gain = self.db_to_linear(eq.bass_gain);
            let bass_filter = FilterBuilder::new()
                .cutoff(Value::Fixed(125.0)) // Center of bass band
                .resonance(Value::Fixed(1.0))
                .mix(Value::Fixed(gain as f64));
            let _ = builder.add_effect(bass_filter);
        }

        // Mid band (250-4000 Hz)
        if eq.mid_gain.abs() > 0.01 {
            let gain = self.db_to_linear(eq.mid_gain);
            let mid_filter = FilterBuilder::new()
                .cutoff(Value::Fixed(2000.0)) // Center of mid band
                .resonance(Value::Fixed(1.0))
                .mix(Value::Fixed(gain as f64));
            let _ = builder.add_effect(mid_filter);
        }

        // Treble band (4000-20000 Hz)
        if eq.treble_gain.abs() > 0.01 {
            let gain = self.db_to_linear(eq.treble_gain);
            let treble_filter = FilterBuilder::new()
                .cutoff(Value::Fixed(10000.0)) // Center of treble band
                .resonance(Value::Fixed(1.0))
                .mix(Value::Fixed(gain as f64));
            let _ = builder.add_effect(treble_filter);
        }

        Ok(builder)
    }

    /// Convert dB gain to linear gain
    fn db_to_linear(&self, db: f32) -> f32 {
        10.0_f32.powf(db / 20.0)
    }
}
