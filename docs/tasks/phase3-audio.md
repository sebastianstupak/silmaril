# Phase 3.2: Audio Integration

**Status:** ⚪ Not Started
**Estimated Time:** 3-4 days
**Priority:** Medium (gameplay enhancement)

---

## 🎯 **Objective**

Integrate Kira audio engine for 3D spatial audio, sound effects, and music. Provides high-quality audio playback with minimal performance overhead.

**Features:**
- Kira audio engine integration
- 3D spatial audio with distance attenuation
- Sound component (one-shot and looping)
- Audio streaming for large files
- Audio listener (camera-based)
- Performance-optimized audio processing

---

## 📋 **Detailed Tasks**

### **1. Audio Engine Setup** (Day 1)

**File:** `engine/audio/src/engine.rs`

```rust
use kira::{
    manager::{AudioManager, AudioManagerSettings, backend::DefaultBackend},
    sound::{
        static_sound::{StaticSoundData, StaticSoundSettings, StaticSoundHandle},
        streaming::{StreamingSoundData, StreamingSoundSettings, StreamingSoundHandle},
        PlaybackState,
    },
    spatial::{
        scene::{SpatialSceneHandle, SpatialSceneSettings},
        emitter::{EmitterHandle, EmitterSettings},
        listener::{ListenerHandle, ListenerSettings},
    },
    tween::Tween,
    Volume,
};
use glam::{Vec3, Quat};
use std::collections::HashMap;
use std::path::Path;

/// Audio engine managing all audio playback
pub struct AudioEngine {
    /// Kira audio manager
    manager: AudioManager,

    /// Spatial scene for 3D audio
    spatial_scene: SpatialSceneHandle,

    /// Audio listener (camera position)
    listener: ListenerHandle,

    /// Loaded sounds (cached)
    loaded_sounds: HashMap<String, StaticSoundData>,

    /// Active sound instances
    active_sounds: HashMap<u64, SoundInstance>,

    /// Spatial emitters per entity
    emitters: HashMap<u64, EmitterHandle>,

    /// Next instance ID
    next_instance_id: u64,
}

enum SoundInstance {
    Static(StaticSoundHandle),
    Streaming(StreamingSoundHandle),
}

impl AudioEngine {
    pub fn new() -> Result<Self, AudioError> {
        let mut manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())?;

        // Create spatial scene
        let mut spatial_scene = manager.add_spatial_scene(SpatialSceneSettings::default())?;

        // Create listener
        let listener = spatial_scene.add_listener(
            Vec3::ZERO.into(),
            Quat::IDENTITY.into(),
            ListenerSettings::default(),
        )?;

        Ok(Self {
            manager,
            spatial_scene,
            listener,
            loaded_sounds: HashMap::new(),
            active_sounds: HashMap::new(),
            emitters: HashMap::new(),
            next_instance_id: 0,
        })
    }

    /// Load sound from file
    pub fn load_sound(&mut self, name: &str, path: impl AsRef<Path>) -> Result<(), AudioError> {
        if self.loaded_sounds.contains_key(name) {
            return Ok(()); // Already loaded
        }

        let sound_data = StaticSoundData::from_file(
            path,
            StaticSoundSettings::default(),
        )?;

        self.loaded_sounds.insert(name.to_string(), sound_data);

        tracing::info!("Loaded sound: {}", name);

        Ok(())
    }

    /// Play 2D sound (UI, menu sounds)
    pub fn play_2d(
        &mut self,
        sound_name: &str,
        volume: f32,
        looping: bool,
    ) -> Result<u64, AudioError> {
        let sound_data = self.loaded_sounds
            .get(sound_name)
            .ok_or_else(|| AudioError::SoundNotFound(sound_name.to_string()))?;

        let mut settings = StaticSoundSettings::default();
        settings.volume = Volume::Amplitude(volume as f64);
        settings.loop_region = if looping {
            Some(..)
        } else {
            None
        };

        let handle = self.manager.play(sound_data.clone().with_settings(settings))?;

        let instance_id = self.next_instance_id;
        self.next_instance_id += 1;

        self.active_sounds.insert(instance_id, SoundInstance::Static(handle));

        tracing::debug!("Playing 2D sound: {} (id: {})", sound_name, instance_id);

        Ok(instance_id)
    }

    /// Play 3D spatial sound
    pub fn play_3d(
        &mut self,
        entity: u64,
        sound_name: &str,
        position: Vec3,
        volume: f32,
        looping: bool,
    ) -> Result<u64, AudioError> {
        let sound_data = self.loaded_sounds
            .get(sound_name)
            .ok_or_else(|| AudioError::SoundNotFound(sound_name.to_string()))?;

        // Get or create emitter for entity
        let emitter = if let Some(emitter) = self.emitters.get(&entity) {
            *emitter
        } else {
            let emitter = self.spatial_scene.add_emitter(
                position.into(),
                EmitterSettings::default(),
            )?;
            self.emitters.insert(entity, emitter);
            emitter
        };

        let mut settings = StaticSoundSettings::default();
        settings.volume = Volume::Amplitude(volume as f64);
        settings.loop_region = if looping {
            Some(..)
        } else {
            None
        };
        settings.output_destination = emitter.into();

        let handle = self.manager.play(sound_data.clone().with_settings(settings))?;

        let instance_id = self.next_instance_id;
        self.next_instance_id += 1;

        self.active_sounds.insert(instance_id, SoundInstance::Static(handle));

        tracing::debug!(
            "Playing 3D sound: {} at {:?} (id: {})",
            sound_name,
            position,
            instance_id
        );

        Ok(instance_id)
    }

    /// Stream large audio file (music)
    pub fn play_stream(
        &mut self,
        path: impl AsRef<Path>,
        volume: f32,
        looping: bool,
    ) -> Result<u64, AudioError> {
        let sound_data = StreamingSoundData::from_file(
            path,
            StreamingSoundSettings::default(),
        )?;

        let mut settings = StreamingSoundSettings::default();
        settings.volume = Volume::Amplitude(volume as f64);
        settings.loop_region = if looping {
            Some(..)
        } else {
            None
        };

        let handle = self.manager.play(sound_data.with_settings(settings))?;

        let instance_id = self.next_instance_id;
        self.next_instance_id += 1;

        self.active_sounds.insert(instance_id, SoundInstance::Streaming(handle));

        tracing::debug!("Streaming audio (id: {})", instance_id);

        Ok(instance_id)
    }

    /// Stop sound instance
    pub fn stop(&mut self, instance_id: u64, fade_out_duration: Option<f32>) {
        if let Some(instance) = self.active_sounds.get_mut(&instance_id) {
            let tween = fade_out_duration.map(|duration| {
                Tween {
                    duration: std::time::Duration::from_secs_f32(duration),
                    ..Default::default()
                }
            });

            match instance {
                SoundInstance::Static(handle) => {
                    let _ = handle.stop(tween.unwrap_or(Tween::default()));
                }
                SoundInstance::Streaming(handle) => {
                    let _ = handle.stop(tween.unwrap_or(Tween::default()));
                }
            }

            self.active_sounds.remove(&instance_id);

            tracing::debug!("Stopped sound (id: {})", instance_id);
        }
    }

    /// Set listener position/orientation (camera)
    pub fn set_listener_transform(&mut self, position: Vec3, forward: Vec3, up: Vec3) {
        let _ = self.listener.set_position(position.into(), Tween::default());
        let _ = self.listener.set_orientation((forward, up).into(), Tween::default());
    }

    /// Update emitter position
    pub fn update_emitter_position(&mut self, entity: u64, position: Vec3) {
        if let Some(emitter) = self.emitters.get_mut(&entity) {
            let _ = emitter.set_position(position.into(), Tween::default());
        }
    }

    /// Remove emitter
    pub fn remove_emitter(&mut self, entity: u64) {
        if let Some(emitter) = self.emitters.remove(&entity) {
            let _ = self.spatial_scene.remove_emitter(emitter);
            tracing::debug!("Removed emitter for entity {}", entity);
        }
    }

    /// Get playback state
    pub fn is_playing(&self, instance_id: u64) -> bool {
        if let Some(instance) = self.active_sounds.get(&instance_id) {
            match instance {
                SoundInstance::Static(handle) => {
                    matches!(handle.state(), PlaybackState::Playing)
                }
                SoundInstance::Streaming(handle) => {
                    matches!(handle.state(), PlaybackState::Playing)
                }
            }
        } else {
            false
        }
    }

    /// Clean up finished sounds
    pub fn cleanup_finished(&mut self) {
        self.active_sounds.retain(|_, instance| {
            match instance {
                SoundInstance::Static(handle) => {
                    !matches!(handle.state(), PlaybackState::Stopped)
                }
                SoundInstance::Streaming(handle) => {
                    !matches!(handle.state(), PlaybackState::Stopped)
                }
            }
        });
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    #[error("Kira error: {0}")]
    Kira(#[from] kira::manager::error::PlaySoundError<()>),

    #[error("Sound not found: {0}")]
    SoundNotFound(String),

    #[error("Invalid sound instance: {0}")]
    InvalidInstance(u64),
}
```

---

### **2. Sound Component** (Day 2)

**File:** `engine/ecs/src/components/sound.rs`

```rust
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

    /// Rolloff factor (how quickly volume decreases with distance)
    pub rolloff: f32,

    /// Current instance ID (if playing)
    #[serde(skip)]
    pub instance_id: Option<u64>,
}

impl Default for Sound {
    fn default() -> Self {
        Self {
            sound_name: String::new(),
            volume: 1.0,
            looping: false,
            auto_play: false,
            spatial: true,
            max_distance: 100.0,
            rolloff: 1.0,
            instance_id: None,
        }
    }
}

impl Sound {
    pub fn new(sound_name: impl Into<String>) -> Self {
        Self {
            sound_name: sound_name.into(),
            ..Default::default()
        }
    }

    pub fn with_volume(mut self, volume: f32) -> Self {
        self.volume = volume;
        self
    }

    pub fn looping(mut self) -> Self {
        self.looping = true;
        self
    }

    pub fn auto_play(mut self) -> Self {
        self.auto_play = true;
        self
    }

    pub fn spatial_3d(mut self, max_distance: f32) -> Self {
        self.spatial = true;
        self.max_distance = max_distance;
        self
    }

    pub fn non_spatial(mut self) -> Self {
        self.spatial = false;
        self
    }
}

/// Audio listener component (attach to camera)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AudioListener {
    /// Active (only one listener should be active)
    pub active: bool,
}
```

---

### **3. Audio System** (Day 2-3)

**File:** `engine/audio/src/systems.rs`

```rust
use crate::engine::AudioEngine;
use engine_ecs::prelude::*;
use glam::Vec3;

/// Audio system
pub struct AudioSystem {
    audio_engine: AudioEngine,
}

impl AudioSystem {
    pub fn new() -> Result<Self, AudioError> {
        Ok(Self {
            audio_engine: AudioEngine::new()?,
        })
    }

    /// Load all sounds referenced in world
    pub fn load_sounds(&mut self, world: &World, asset_path: &str) -> Result<(), AudioError> {
        for (_, sound) in world.query::<&Sound>().iter() {
            let path = format!("{}/{}", asset_path, sound.sound_name);
            self.audio_engine.load_sound(&sound.sound_name, &path)?;
        }

        Ok(())
    }

    /// Update audio system
    pub fn update(&mut self, world: &mut World, dt: f32) {
        // Update listener position from camera
        self.update_listener(world);

        // Update sound emitters
        self.update_emitters(world);

        // Handle auto-play sounds
        self.handle_auto_play(world);

        // Cleanup finished sounds
        self.audio_engine.cleanup_finished();
    }

    /// Update listener from camera
    fn update_listener(&mut self, world: &World) {
        for (_, (transform, listener)) in world.query::<(&Transform, &AudioListener)>().iter() {
            if listener.active {
                // Calculate forward and up vectors from rotation
                let forward = transform.rotation * Vec3::new(0.0, 0.0, -1.0);
                let up = transform.rotation * Vec3::new(0.0, 1.0, 0.0);

                self.audio_engine.set_listener_transform(
                    transform.position,
                    forward,
                    up,
                );

                break; // Only one active listener
            }
        }
    }

    /// Update emitter positions
    fn update_emitters(&mut self, world: &World) {
        for (entity, (transform, sound)) in world.query::<(&Transform, &Sound)>().iter() {
            if sound.spatial && sound.instance_id.is_some() {
                self.audio_engine.update_emitter_position(
                    entity.id(),
                    transform.position,
                );
            }
        }
    }

    /// Handle auto-play sounds
    fn handle_auto_play(&mut self, world: &mut World) {
        for (entity, (transform, sound)) in world.query::<(&Transform, &mut Sound)>().iter() {
            if sound.auto_play && sound.instance_id.is_none() {
                // Play sound
                let result = if sound.spatial {
                    self.audio_engine.play_3d(
                        entity.id(),
                        &sound.sound_name,
                        transform.position,
                        sound.volume,
                        sound.looping,
                    )
                } else {
                    self.audio_engine.play_2d(
                        &sound.sound_name,
                        sound.volume,
                        sound.looping,
                    )
                };

                match result {
                    Ok(instance_id) => {
                        sound.instance_id = Some(instance_id);
                        tracing::debug!("Auto-played sound for entity {}", entity.id());
                    }
                    Err(e) => {
                        tracing::error!("Failed to play sound: {}", e);
                    }
                }

                // Disable auto-play after first play
                if !sound.looping {
                    sound.auto_play = false;
                }
            }
        }
    }

    /// Play sound manually
    pub fn play_sound(
        &mut self,
        entity: Entity,
        world: &mut World,
    ) -> Result<u64, AudioError> {
        if let Some((transform, sound)) = world.get_components::<(&Transform, &mut Sound)>(entity) {
            let instance_id = if sound.spatial {
                self.audio_engine.play_3d(
                    entity.id(),
                    &sound.sound_name,
                    transform.position,
                    sound.volume,
                    sound.looping,
                )?
            } else {
                self.audio_engine.play_2d(
                    &sound.sound_name,
                    sound.volume,
                    sound.looping,
                )?
            };

            sound.instance_id = Some(instance_id);

            Ok(instance_id)
        } else {
            Err(AudioError::InvalidInstance(entity.id()))
        }
    }

    /// Stop sound
    pub fn stop_sound(
        &mut self,
        entity: Entity,
        world: &mut World,
        fade_out: Option<f32>,
    ) {
        if let Some(sound) = world.get_component::<&mut Sound>(entity) {
            if let Some(instance_id) = sound.instance_id {
                self.audio_engine.stop(instance_id, fade_out);
                sound.instance_id = None;
            }
        }
    }

    /// Access audio engine
    pub fn engine(&self) -> &AudioEngine {
        &self.audio_engine
    }

    pub fn engine_mut(&mut self) -> &mut AudioEngine {
        &mut self.audio_engine
    }
}
```

---

### **4. Example & Testing** (Day 3-4)

**File:** `examples/audio_demo.rs`

```rust
use engine_ecs::prelude::*;
use engine_audio::AudioSystem;
use glam::{Vec3, Quat};
use std::time::Duration;

fn main() {
    tracing_subscriber::fmt::init();

    // Create world
    let mut world = World::new();

    // Create camera with audio listener
    let camera = world.spawn();
    world.add_component(camera, Transform {
        position: Vec3::new(0.0, 1.0, 5.0),
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
    });
    world.add_component(camera, AudioListener { active: true });

    // Create sound emitter (moving object)
    let emitter = world.spawn();
    world.add_component(emitter, Transform {
        position: Vec3::new(-10.0, 0.0, 0.0),
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
    });
    world.add_component(emitter, Sound::new("footstep.wav")
        .spatial_3d(50.0)
        .looping()
        .auto_play()
        .with_volume(0.8));

    // Create background music
    let music = world.spawn();
    world.add_component(music, Transform::default());
    world.add_component(music, Sound::new("music.ogg")
        .non_spatial()
        .looping()
        .auto_play()
        .with_volume(0.3));

    // Create audio system
    let mut audio_system = AudioSystem::new().unwrap();

    // Load sounds
    audio_system.load_sounds(&world, "assets/audio").unwrap();

    // Simulate
    let mut time = 0.0f32;
    let dt = 1.0 / 60.0;

    for _ in 0..600 {
        // Move emitter in circle
        if let Some(transform) = world.get_component::<&mut Transform>(emitter) {
            let angle = time * 2.0;
            transform.position.x = angle.cos() * 10.0;
            transform.position.z = angle.sin() * 10.0;
        }

        // Update audio
        let start = std::time::Instant::now();
        audio_system.update(&mut world, dt);
        let elapsed = start.elapsed();

        if elapsed.as_micros() > 500 {
            tracing::warn!("Audio update took {}μs (target: <500μs)", elapsed.as_micros());
        }

        time += dt;
        std::thread::sleep(Duration::from_secs_f32(dt));
    }
}
```

---

## 📊 **Audio Effects System** (Implemented)

### **Overview**

The audio effects system provides real-time audio processing effects that can be applied to sound instances during playback. Effects are implemented using Kira's effect system for native platforms.

### **Supported Effects**

#### **1. ReverbEffect - Room Acoustics Simulation**

```rust
pub struct ReverbEffect {
    /// Room size (0.0 = tiny room, 1.0 = massive cathedral)
    pub room_size: f32,

    /// Damping (0.0 = no damping, 1.0 = maximum damping)
    pub damping: f32,

    /// Wet/dry mix (0.0 = all dry, 1.0 = all wet)
    pub wet_dry_mix: f32,
}
```

**Presets:**
- `ReverbEffect::small_room()` - Tight reverb for small spaces
- `ReverbEffect::large_hall()` - Spacious reverb for concert halls
- `ReverbEffect::cathedral()` - Long, reverberant space

**Use Cases:**
- Indoor environments (buildings, caves, tunnels)
- Adding depth to sounds
- Creating atmospheric ambiance

#### **2. EchoEffect - Delay-Based Echo**

```rust
pub struct EchoEffect {
    /// Delay time in seconds (0.0 - 2.0)
    pub delay_time: f32,

    /// Feedback amount (0.0 - 0.95)
    pub feedback: f32,

    /// Wet/dry mix (0.0 = all dry, 1.0 = all wet)
    pub wet_dry_mix: f32,
}
```

**Presets:**
- `EchoEffect::slapback()` - Short, single echo (0.08s delay)
- `EchoEffect::long_echo()` - Long, spacious echo (0.75s delay)

**Use Cases:**
- Canyon/mountain environments
- Radio transmission effects
- Special audio effects

#### **3. FilterEffect - Frequency Manipulation**

```rust
pub struct FilterEffect {
    /// Filter type (LowPass, HighPass, BandPass)
    pub filter_type: FilterType,

    /// Cutoff frequency in Hz (20.0 - 20000.0)
    pub cutoff_frequency: f32,

    /// Resonance/Q factor (0.5 - 10.0)
    pub resonance: f32,

    /// Wet/dry mix (0.0 = all dry, 1.0 = all wet)
    pub wet_dry_mix: f32,
}
```

**Presets:**
- `FilterEffect::muffled()` - Low-pass for underwater/through walls
- `FilterEffect::tinny()` - High-pass for radio/telephone
- `FilterEffect::radio()` - Band-pass for radio transmission

**Use Cases:**
- Underwater audio
- Sounds through walls/obstacles
- Radio/telephone effects
- Creating distance perception

#### **4. EqEffect - 3-Band Equalizer**

```rust
pub struct EqEffect {
    /// Bass gain in dB (-20.0 to +20.0)
    pub bass_gain: f32,

    /// Mid gain in dB (-20.0 to +20.0)
    pub mid_gain: f32,

    /// Treble gain in dB (-20.0 to +20.0)
    pub treble_gain: f32,
}
```

**Presets:**
- `EqEffect::bass_boost()` - Enhanced bass for impacts/explosions
- `EqEffect::voice_clarity()` - Clear mids for dialogue
- `EqEffect::bright()` - Enhanced treble for UI sounds

**Use Cases:**
- Weapon impacts (bass boost)
- Voice enhancement
- UI sound polish

### **Usage Example**

```rust
use engine_audio::{AudioEngine, AudioEffect, ReverbEffect, FilterEffect};
use glam::Vec3;

let mut audio = AudioEngine::new().unwrap();
audio.load_sound("gunshot", "assets/gunshot.wav").unwrap();

// Play sound
let instance = audio.play_3d(
    1,                          // entity ID
    "gunshot",                  // sound name
    Vec3::new(10.0, 0.0, 5.0), // position
    1.0,                        // volume
    false,                      // looping
    100.0,                      // max distance
).unwrap();

// Add reverb for indoor environment
let reverb = ReverbEffect::small_room();
audio.add_effect(instance, AudioEffect::Reverb(reverb)).unwrap();

// Add low-pass filter for muffled effect
let filter = FilterEffect::muffled();
audio.add_effect(instance, AudioEffect::Filter(filter)).unwrap();
```

### **Platform Support**

| Platform | Reverb | Echo | Filter | EQ | Status |
|----------|--------|------|--------|----|--------|
| Desktop (Kira) | ✅ | ✅ | ✅ | ✅ | Implemented |
| Web (Web Audio) | 🚧 | 🚧 | 🚧 | 🚧 | Stub (TODO) |
| Android (Oboe) | 🚧 | 🚧 | 🚧 | 🚧 | Stub (TODO) |
| iOS (Core Audio) | 🚧 | 🚧 | 🚧 | 🚧 | Stub (TODO) |

### **Effect Chain**

Multiple effects can be stacked on a single sound instance:

```rust
// Create complex effect chain
let reverb = AudioEffect::Reverb(ReverbEffect::large_hall());
let echo = AudioEffect::Echo(EchoEffect::slapback());
let filter = AudioEffect::Filter(FilterEffect::muffled());

// Add effects in order (they're applied sequentially)
audio.add_effect(instance, filter).unwrap();    // Applied first
audio.add_effect(instance, reverb).unwrap();    // Applied second
audio.add_effect(instance, echo).unwrap();      // Applied last

// Remove specific effect by index
audio.remove_effect(instance, 1); // Remove reverb

// Clear all effects
audio.clear_effects(instance);
```

### **Performance**

All effect operations are designed for < 0.1ms overhead:

| Operation | Target | Typical |
|-----------|--------|---------|
| Effect creation | < 1µs | ~500ns |
| Effect validation | < 500ns | ~200ns |
| Add effect | < 0.1ms | ~50µs |
| Remove effect | < 0.05ms | ~20µs |
| Effect processing | < 0.1ms | ~30µs |

### **Validation**

All effects validate their parameters to prevent invalid audio processing:

```rust
let reverb = ReverbEffect {
    room_size: 1.5,  // Invalid (> 1.0)
    damping: 0.5,
    wet_dry_mix: 0.3,
};

assert!(!reverb.validate()); // Returns false

// Engine will reject invalid effects
audio.add_effect(instance, AudioEffect::Reverb(reverb))
    .expect_err("Should reject invalid effect");
```

### **Serialization**

All effects support serde serialization for saving/loading:

```rust
let reverb = ReverbEffect::cathedral();
let json = serde_json::to_string(&reverb).unwrap();
let loaded: ReverbEffect = serde_json::from_str(&json).unwrap();
```

## ✅ **Acceptance Criteria**

- [ ] Kira audio engine integrated
- [ ] 2D audio playback works
- [ ] 3D spatial audio with distance attenuation
- [ ] Sound component with auto-play
- [ ] AudioListener component (camera-based)
- [ ] Audio streaming for large files
- [ ] Audio update < 0.5ms
- [ ] No audio glitches or pops
- [ ] Example demonstrates 3D audio
- [ ] Multiple sounds play simultaneously
- [x] Audio effects implemented (reverb, echo, filter, EQ)
- [x] Effect validation ensures valid parameters
- [x] Effect serialization/deserialization
- [x] Effect performance < 0.1ms overhead
- [x] **Task #32: E2E debugging tools for AI agents** ✅
  - [x] AudioDiagnostics for state inspection
  - [x] AudioEventLogger for event history
  - [x] E2E validator with automated tests
  - [x] DEBUG_GUIDE.md for agent workflows
  - [x] Clear PASS/FAIL diagnostic output

---

## 🎯 **Performance Targets**

| Operation | Target | Critical |
|-----------|--------|----------|
| Audio update | < 0.5ms | < 2ms |
| Play sound | < 0.1ms | < 0.5ms |
| Stop sound | < 0.1ms | < 0.5ms |
| Load sound | < 50ms | < 200ms |
| 3D position update | < 0.05ms | < 0.2ms |
| Simultaneous sounds | 32+ | 16+ |

---

**Dependencies:** [phase1-ecs-core.md](phase1-ecs-core.md)
**Next:** [phase3-lod-rendering.md](phase3-lod-rendering.md)
