# Audio Architecture

> **Audio system for silmaril**
>
> Kira-based spatial audio with 3D positioning and effect processing

---

## Overview

The silmaril uses Kira for audio playback:
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

Silmaril provides a comprehensive audio effects system with cross-platform support. Effects can be applied to individual sound instances and stacked in any order.

### Available Effects

| Effect | Description | Use Cases |
|--------|-------------|-----------|
| **Reverb** | Simulates room acoustics | Indoor environments, caves, cathedrals |
| **Echo/Delay** | Discrete sound repetitions | Outdoor spaces, special effects |
| **Filter** | Frequency manipulation (low-pass, high-pass, band-pass) | Underwater, radio effects, muffled sounds |
| **Equalizer** | 3-band EQ (bass, mid, treble) | Music mixing, voice clarity, weapon impacts |

### Reverb Effect

Adds depth and space to sounds by simulating room reflections.

```rust
use engine_audio::{AudioEngine, AudioEffect, ReverbEffect};
use glam::Vec3;

let mut audio = AudioEngine::new().unwrap();
audio.load_sound("gunshot", "assets/gunshot.wav").unwrap();

let instance = audio.play_3d(1, "gunshot", Vec3::ZERO, 1.0, false, 100.0).unwrap();

// Indoor environment - small room
let reverb = ReverbEffect::small_room();
audio.add_effect(instance, AudioEffect::Reverb(reverb)).unwrap();

// Large hall - more spacious reverb
let hall = ReverbEffect::large_hall();
audio.add_effect(instance, AudioEffect::Reverb(hall)).unwrap();

// Cathedral - maximum reverb
let cathedral = ReverbEffect::cathedral();
audio.add_effect(instance, AudioEffect::Reverb(cathedral)).unwrap();

// Custom reverb
let custom_reverb = ReverbEffect {
    room_size: 0.7,     // 0.0 = tiny, 1.0 = massive
    damping: 0.4,       // 0.0 = bright, 1.0 = dull
    wet_dry_mix: 0.3,   // 0.0 = all dry, 1.0 = all wet
};
audio.add_effect(instance, AudioEffect::Reverb(custom_reverb)).unwrap();
```

**Parameters:**
- `room_size` (0.0-1.0): Size of simulated space
- `damping` (0.0-1.0): High-frequency absorption (higher = more damping)
- `wet_dry_mix` (0.0-1.0): Balance between original and reverb signal

### Echo/Delay Effect

Creates discrete repetitions of the sound.

```rust
use engine_audio::{AudioEffect, EchoEffect};

// Slapback echo (short delay, common in music production)
let slapback = EchoEffect::slapback();
audio.add_effect(instance, AudioEffect::Echo(slapback)).unwrap();

// Long outdoor echo
let long_echo = EchoEffect::long_echo();
audio.add_effect(instance, AudioEffect::Echo(long_echo)).unwrap();

// Custom echo
let custom_echo = EchoEffect {
    delay_time: 0.5,    // 0.0-2.0 seconds
    feedback: 0.6,      // 0.0-0.95 (higher = more repetitions)
    wet_dry_mix: 0.4,   // 0.0-1.0
};
audio.add_effect(instance, AudioEffect::Echo(custom_echo)).unwrap();
```

**Parameters:**
- `delay_time` (0.0-2.0s): Time between echoes
- `feedback` (0.0-0.95): Amount of signal fed back (controls repetitions)
- `wet_dry_mix` (0.0-1.0): Balance between original and echo signal

**Warning:** Feedback values >= 1.0 cause infinite feedback and are not allowed.

### Filter Effect

Manipulates frequency content to create various tonal effects.

```rust
use engine_audio::{AudioEffect, FilterEffect, FilterType};

// Low-pass filter - muffled/underwater sound
let muffled = FilterEffect::muffled();
audio.add_effect(instance, AudioEffect::Filter(muffled)).unwrap();

// High-pass filter - tinny/telephone sound
let tinny = FilterEffect::tinny();
audio.add_effect(instance, AudioEffect::Filter(tinny)).unwrap();

// Band-pass filter - radio transmission
let radio = FilterEffect::radio();
audio.add_effect(instance, AudioEffect::Filter(radio)).unwrap();

// Custom filter
let custom_filter = FilterEffect {
    filter_type: FilterType::LowPass,
    cutoff_frequency: 1000.0,  // 20.0-20000.0 Hz
    resonance: 2.0,            // 0.5-10.0 (higher = sharper cutoff)
    wet_dry_mix: 1.0,          // 0.0-1.0
};
audio.add_effect(instance, AudioEffect::Filter(custom_filter)).unwrap();
```

**Filter Types:**
- `LowPass`: Removes high frequencies (makes sound muffled/dull)
- `HighPass`: Removes low frequencies (makes sound thin/tinny)
- `BandPass`: Removes frequencies outside a range (telephone/radio effect)

**Parameters:**
- `filter_type`: Type of filter
- `cutoff_frequency` (20-20000 Hz): Frequency where filtering begins
- `resonance` (0.5-10.0): Sharpness of filter cutoff (Q factor)
- `wet_dry_mix` (0.0-1.0): Balance between original and filtered signal

### Equalizer Effect

3-band EQ for independent control of bass, mid, and treble frequencies.

```rust
use engine_audio::{AudioEffect, EqEffect};

// Bass-heavy explosion
let bass_boost = EqEffect::bass_boost();
audio.add_effect(instance, AudioEffect::Eq(bass_boost)).unwrap();

// Clear voice (boost mids)
let voice_clarity = EqEffect::voice_clarity();
audio.add_effect(instance, AudioEffect::Eq(voice_clarity)).unwrap();

// Bright UI sounds (boost treble)
let bright = EqEffect::bright();
audio.add_effect(instance, AudioEffect::Eq(bright)).unwrap();

// Custom EQ
let custom_eq = EqEffect {
    bass_gain: 6.0,      // -20.0 to +20.0 dB
    mid_gain: -3.0,      // -20.0 to +20.0 dB
    treble_gain: 2.0,    // -20.0 to +20.0 dB
};
audio.add_effect(instance, AudioEffect::Eq(custom_eq)).unwrap();
```

**Frequency Bands:**
- **Bass** (20-250 Hz): Low frequencies, rumble, impact
- **Mid** (250-4000 Hz): Most important for voice clarity and presence
- **Treble** (4000-20000 Hz): High frequencies, clarity, air

**Parameters:**
- `bass_gain` (-20 to +20 dB): Boost or cut bass frequencies
- `mid_gain` (-20 to +20 dB): Boost or cut mid frequencies
- `treble_gain` (-20 to +20 dB): Boost or cut treble frequencies

### Stacking Effects

Multiple effects can be applied to a single sound instance and are processed in order:

```rust
// Create realistic indoor gunshot: reverb + filter + eq
let instance = audio.play_3d(1, "gunshot", Vec3::new(10.0, 0.0, 0.0), 1.0, false, 100.0).unwrap();

// 1. Add reverb for room acoustics
audio.add_effect(instance, AudioEffect::Reverb(ReverbEffect::small_room())).unwrap();

// 2. Add slight low-pass filter (walls absorb high frequencies)
audio.add_effect(instance, AudioEffect::Filter(FilterEffect {
    filter_type: FilterType::LowPass,
    cutoff_frequency: 8000.0,
    resonance: 1.0,
    wet_dry_mix: 0.5,
})).unwrap();

// 3. Boost bass for impact
audio.add_effect(instance, AudioEffect::Eq(EqEffect::bass_boost())).unwrap();
```

### Managing Effects

```rust
// Get effect count
let count = audio.effect_count(instance);
println!("Sound has {} effects", count);

// Remove specific effect by index
let effect_index = 1; // Remove second effect
audio.remove_effect(instance, effect_index);

// Clear all effects
audio.clear_effects(instance);
```

### Effect Application Performance

Effects are optimized for real-time performance:
- Effect application: **< 100μs per effect per frame**
- Effect chain (3+ effects): **< 500μs per frame**
- No audible artifacts or clicks

### Platform Support

| Platform | Reverb | Echo | Filter | EQ | Notes |
|----------|--------|------|--------|-----|-------|
| **Desktop (Kira)** | ✅ Full | ✅ Full | ✅ Full | ✅ 3-band | Native Kira effects |
| **Web (WebAudio)** | ✅ Full | ✅ Full | ✅ Full | ✅ 3-band | Web Audio API nodes |
| **Android** | ⚠️ Basic | ⚠️ Basic | ✅ Full | ✅ Full | OpenSL ES effects |
| **iOS** | ⚠️ Basic | ⚠️ Basic | ✅ Full | ✅ Full | Core Audio effects |

**Note:** Android and iOS have simplified reverb/echo implementations. For production apps, consider using platform-specific effect tuning.

### Best Practices

**DO:**
- ✅ Use effect presets for common scenarios (small_room, muffled, bass_boost)
- ✅ Validate effect parameters before applying (`effect.validate()`)
- ✅ Apply effects to sound instances, not globally (allows per-sound control)
- ✅ Stack effects in logical order (reverb last for most natural sound)
- ✅ Use subtle effect settings (less is more)

**DON'T:**
- ❌ Apply too many effects to a single sound (>5 effects can be CPU-intensive)
- ❌ Use extreme parameter values (causes unnatural artifacts)
- ❌ Forget to clear effects when stopping sounds (can leak memory)
- ❌ Apply heavy reverb to all sounds (muddy mix)
- ❌ Use feedback >= 1.0 on echo effects (infinite feedback)

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

## Doppler Effect

The engine implements realistic Doppler shift for high-speed movement, automatically adjusting pitch based on relative velocity between the listener and sound sources.

### Physics

The Doppler shift formula:
```
f' = f * (v + vr) / (v + vs)
```
Where:
- `f'` = observed frequency (pitch)
- `f` = emitted frequency
- `v` = speed of sound (default 343 m/s)
- `vr` = listener velocity relative to medium
- `vs` = source velocity relative to medium

### Usage

Enable Doppler on a per-sound basis:

```rust
use engine_audio::Sound;

// Enable with default scale (1.0 = realistic)
let sound = Sound::new("car_engine.wav")
    .spatial_3d(100.0)
    .with_doppler(1.0);

// Disable Doppler (for ambient sounds)
let ambient = Sound::new("background.wav")
    .spatial_3d(100.0)
    .without_doppler();

// Exaggerated effect (for gameplay feedback)
let jet = Sound::new("jet.wav")
    .spatial_3d(500.0)
    .with_doppler(2.0);
```

### Configuration

Customize global Doppler settings:

```rust
use engine_audio::AudioSystem;

// Custom speed of sound (e.g., for underwater, space, etc.)
let mut audio = AudioSystem::new_with_doppler(1500.0, 1.0).unwrap();

// Adjust scale at runtime
audio.set_doppler_scale(0.5); // Half intensity
audio.set_speed_of_sound(340.0); // Colder air
```

### Performance

Doppler calculations are highly optimized:
- **< 50μs per emitter per frame** on typical hardware
- Minimal overhead (~1-2% CPU for 100 moving sources)
- Automatic velocity tracking with position history
- Clamped to prevent audio artifacts (0.5x - 2.0x pitch range)

### Example Scenarios

**Racing Game:**
```rust
let car = Sound::new("engine.wav")
    .spatial_3d(200.0)
    .with_doppler(1.0)
    .looping();
```

**Aircraft Flyby:**
```rust
let jet = Sound::new("jet_engine.wav")
    .spatial_3d(1000.0)
    .with_doppler(1.5) // Exaggerated for gameplay
    .looping();
```

**Bullet Whizz-By:**
```rust
let bullet = Sound::new("whizz.wav")
    .spatial_3d(50.0)
    .with_doppler(0.8); // Subtle effect
```

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
