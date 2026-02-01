# Audio Architecture

> **Audio system for agent-game-engine**
>
> Kira-based spatial audio with 3D positioning and effect processing

---

## Overview

The agent-game-engine uses Kira for audio playback:
- **Spatial audio** - 3D positioned sounds with distance attenuation
- **Music management** - Looping tracks with crossfade support
- **Sound effects** - One-shot and looping sound playback
- **Effect processing** - Reverb, filters, and custom DSP
- **Resource management** - Async loading and streaming

**Status:** ⚪ Not implemented (Phase 3.3)

---

## Architecture

### Audio Components

```rust
use engine_core::Component;

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct AudioSource {
    pub clip: AssetHandle<AudioClip>,
    pub volume: f32,
    pub pitch: f32,
    pub looping: bool,
    pub spatial: bool,
    pub playing: bool,
}

#[derive(Component, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AudioListener {
    pub active: bool,
}

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct AudioEmitter3D {
    pub max_distance: f32,
    pub rolloff_factor: f32,
    pub reference_distance: f32,
}
```

**Implementation:** TBD in `engine/audio/src/components.rs`

---

## Audio Manager

### Initialization

```rust
use kira::manager::{AudioManager, AudioManagerSettings};
use kira::manager::backend::DefaultBackend;

pub struct GameAudioManager {
    manager: AudioManager,
    sounds: HashMap<AssetHandle<AudioClip>, StaticSoundHandle>,
    music: Option<StaticSoundHandle>,
}

impl GameAudioManager {
    pub fn new() -> Result<Self, AudioError> {
        let manager = AudioManager::<DefaultBackend>::new(
            AudioManagerSettings::default()
        )?;

        Ok(Self {
            manager,
            sounds: HashMap::new(),
            music: None,
        })
    }

    pub fn play_sound(&mut self, clip: &AudioClip, settings: PlaybackSettings)
        -> Result<StaticSoundHandle, AudioError>
    {
        let sound_data = StaticSoundData::from_file(&clip.path)?
            .volume(settings.volume)
            .playback_rate(settings.pitch as f64);

        let handle = self.manager.play(sound_data)?;
        Ok(handle)
    }

    pub fn play_music(&mut self, clip: &AudioClip, loop_enabled: bool)
        -> Result<(), AudioError>
    {
        // Stop current music
        if let Some(music) = &mut self.music {
            music.stop(Tween::default())?;
        }

        let sound_data = StaticSoundData::from_file(&clip.path)?
            .loop_region(if loop_enabled {
                Some(kira::sound::Region::from_start())
            } else {
                None
            });

        let handle = self.manager.play(sound_data)?;
        self.music = Some(handle);

        Ok(())
    }
}
```

---

## Audio Systems

### Audio Playback System

Play sounds attached to entities:

```rust
use engine_profiling::profile_scope;

#[profile(category = "Audio")]
pub fn audio_playback_system(
    world: &mut World,
    audio_manager: &mut GameAudioManager,
) {
    profile_scope!("audio_playback");

    for (entity, audio_source) in world.query::<(&Entity, &mut AudioSource)>() {
        if audio_source.playing && !audio_manager.is_playing(entity) {
            let settings = PlaybackSettings {
                volume: audio_source.volume,
                pitch: audio_source.pitch,
                looping: audio_source.looping,
            };

            if let Ok(handle) = audio_manager.play_sound(&audio_source.clip, settings) {
                audio_manager.register_sound(*entity, handle);
            }
        }

        if !audio_source.playing && audio_manager.is_playing(entity) {
            audio_manager.stop_sound(*entity);
        }
    }
}
```

### Spatial Audio System

Update 3D audio positions:

```rust
#[profile(category = "Audio")]
pub fn spatial_audio_system(
    world: &World,
    audio_manager: &mut GameAudioManager,
) {
    profile_scope!("spatial_audio");

    // Find active listener
    let listener_pos = world
        .query::<(&Transform, &AudioListener)>()
        .find(|(_, listener)| listener.active)
        .map(|(transform, _)| transform.position)
        .unwrap_or(Vec3::ZERO);

    // Update spatial audio sources
    for (entity, transform, audio_source, emitter) in world.query::<(
        &Entity,
        &Transform,
        &AudioSource,
        &AudioEmitter3D,
    )>() {
        if !audio_source.spatial {
            continue;
        }

        let distance = (transform.position - listener_pos).length();
        let attenuation = calculate_attenuation(
            distance,
            emitter.reference_distance,
            emitter.max_distance,
            emitter.rolloff_factor,
        );

        audio_manager.set_volume(*entity, audio_source.volume * attenuation);
    }
}

fn calculate_attenuation(
    distance: f32,
    reference: f32,
    max_distance: f32,
    rolloff: f32,
) -> f32 {
    if distance >= max_distance {
        return 0.0;
    }

    if distance <= reference {
        return 1.0;
    }

    // Inverse distance attenuation
    reference / (reference + rolloff * (distance - reference))
}
```

---

## Audio Assets

### Audio Clip

```rust
pub struct AudioClip {
    pub path: PathBuf,
    pub duration: Duration,
    pub channels: u32,
    pub sample_rate: u32,
}

impl Asset for AudioClip {
    fn load(path: &Path) -> Result<Self, AssetError> {
        // Probe audio file metadata
        let metadata = probe_audio_file(path)?;

        Ok(Self {
            path: path.to_path_buf(),
            duration: metadata.duration,
            channels: metadata.channels,
            sample_rate: metadata.sample_rate,
        })
    }
}
```

### Asset Loading

```rust
pub fn load_audio_clip(path: &str) -> Result<AudioClip, AssetError> {
    AudioClip::load(Path::new(path))
}
```

---

## Audio Effects

### Reverb

Add reverb to sounds:

```rust
use kira::effect::reverb::ReverbBuilder;

pub fn add_reverb_to_sound(
    audio_manager: &mut GameAudioManager,
    entity: Entity,
) -> Result<(), AudioError> {
    let reverb = ReverbBuilder::new()
        .room_size(0.8)
        .damping(0.5)
        .wet(0.3)
        .build();

    audio_manager.add_effect_to_sound(entity, reverb)?;
    Ok(())
}
```

### Low-pass Filter

Filter high frequencies:

```rust
use kira::effect::filter::FilterBuilder;

pub fn add_lowpass_filter(
    audio_manager: &mut GameAudioManager,
    entity: Entity,
    cutoff: f32,
) -> Result<(), AudioError> {
    let filter = FilterBuilder::new()
        .cutoff(cutoff)
        .resonance(0.5)
        .build();

    audio_manager.add_effect_to_sound(entity, filter)?;
    Ok(())
}
```

---

## Music System

### Track Management

```rust
pub struct MusicManager {
    current_track: Option<String>,
    next_track: Option<String>,
    crossfade_duration: Duration,
}

impl MusicManager {
    pub fn play_track(&mut self, track: &str, audio_manager: &mut GameAudioManager) {
        if let Some(current) = &self.current_track {
            if current == track {
                return; // Already playing
            }
        }

        // Crossfade to new track
        self.next_track = Some(track.to_string());
        // ... crossfade logic
    }

    pub fn stop(&mut self, audio_manager: &mut GameAudioManager) {
        if let Some(music) = &mut audio_manager.music {
            music.stop(Tween::default());
        }
        self.current_track = None;
    }
}
```

### Crossfade

Smooth transition between tracks:

```rust
pub fn crossfade_music(
    from_handle: &mut StaticSoundHandle,
    to_handle: &StaticSoundHandle,
    duration: Duration,
) {
    // Fade out current track
    from_handle.set_volume(
        0.0,
        Tween {
            duration: duration.as_secs_f64(),
            ..Default::default()
        },
    );

    // Fade in new track
    to_handle.set_volume(
        1.0,
        Tween {
            duration: duration.as_secs_f64(),
            ..Default::default()
        },
    );
}
```

---

## Performance Targets

| Metric | Target | Critical |
|--------|--------|----------|
| Audio latency | < 50ms | < 100ms |
| CPU usage (10 sources) | < 5% | < 10% |
| CPU usage (100 sources) | < 15% | < 30% |
| Memory per clip (1 min) | < 10MB | < 20MB |
| Spatial audio update | < 1ms | < 5ms |

---

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_manager_creation() {
        let audio_manager = GameAudioManager::new().unwrap();
        assert!(audio_manager.sounds.is_empty());
    }

    #[test]
    fn test_attenuation_calculation() {
        let attenuation = calculate_attenuation(10.0, 1.0, 100.0, 1.0);
        assert!(attenuation > 0.0 && attenuation < 1.0);

        let attenuation_max = calculate_attenuation(100.0, 1.0, 100.0, 1.0);
        assert_eq!(attenuation_max, 0.0);

        let attenuation_ref = calculate_attenuation(0.5, 1.0, 100.0, 1.0);
        assert_eq!(attenuation_ref, 1.0);
    }
}
```

---

## Best Practices

### DO

- ✅ Use streaming for long music tracks
- ✅ Preload short sound effects
- ✅ Pool sound handles for reuse
- ✅ Use spatial audio for 3D positioned sounds
- ✅ Limit concurrent sounds (e.g., max 32)

### DON'T

- ❌ Load all audio into memory at startup
- ❌ Play 100s of sounds simultaneously
- ❌ Forget to stop sounds when entities despawn
- ❌ Use high sample rates unnecessarily (44.1kHz is fine)
- ❌ Apply heavy effects to every sound

---

## Advanced Topics

### Audio Streaming

Stream large audio files:

```rust
use kira::sound::streaming::StreamingSoundData;

pub fn stream_music(path: &Path) -> Result<StreamingSoundData<FromFileError>, AudioError> {
    let sound_data = StreamingSoundData::from_file(path)?
        .loop_region(Some(kira::sound::Region::from_start()));

    Ok(sound_data)
}
```

### Custom DSP

Implement custom audio effects:

```rust
use kira::effect::Effect;

pub struct CustomDistortion {
    drive: f32,
}

impl Effect for CustomDistortion {
    fn process(&mut self, input: f32, _dt: f32) -> f32 {
        // Simple soft clipping distortion
        let driven = input * self.drive;
        driven.tanh() / self.drive.tanh()
    }
}
```

### Audio Groups

Group sounds for volume control:

```rust
pub struct AudioGroup {
    pub name: String,
    pub volume: f32,
    pub sounds: Vec<Entity>,
}

pub fn set_group_volume(
    world: &World,
    audio_manager: &mut GameAudioManager,
    group: &AudioGroup,
) {
    for entity in &group.sounds {
        if let Some(source) = world.get::<AudioSource>(*entity) {
            audio_manager.set_volume(*entity, source.volume * group.volume);
        }
    }
}
```

---

## References

- **Implementation:** TBD `engine/audio/src/`
- **Kira Docs:** https://docs.rs/kira/latest/kira/
- **Audio Assets:** TBD `assets/audio/`

**Related Documentation:**
- [ECS](ecs.md)
- [Performance Targets](performance-targets.md)
- [Asset Management](asset-management.md) (TBD)
