# Android Audio Backend

This document describes the Android audio backend implementation using Oboe for low-latency, high-performance audio playback.

## Overview

The Android audio backend provides full feature parity with desktop platforms while optimizing for mobile device constraints:

- **Low-latency audio**: Uses Oboe (AAudio on Android 8.1+, OpenSL ES on older devices)
- **3D spatial audio**: HRTF-like positioning with distance attenuation
- **Multiple formats**: WAV, OGG/Vorbis, MP3 support
- **Streaming**: Efficient playback for large audio files
- **Lifecycle handling**: Proper pause/resume on Android lifecycle events

## Architecture

### Audio Pipeline

```
Audio File (WAV/OGG/MP3)
    ↓
Decoder (hound/lewton/minimp3)
    ↓
AudioBuffer (PCM f32, 44.1kHz stereo)
    ↓
SoundInstance (position, volume, 3D params)
    ↓
Audio Callback (mixing, 3D processing)
    ↓
Oboe Output Stream
    ↓
Hardware Audio Output
```

### Threading Model

The Android backend uses a lock-based approach for thread safety:

1. **Main Thread**: API calls (play, stop, load, etc.)
2. **Audio Thread**: High-priority callback mixing all sounds
3. **Shared State**: Protected by `Arc<Mutex<AudioState>>`

The audio callback runs at high priority with minimal latency (~5.8ms at 256 frames/buffer).

## Configuration

### Audio Parameters

All audio is processed at:
- **Sample Rate**: 44,100 Hz (CD quality)
- **Channels**: 2 (stereo)
- **Format**: 32-bit float PCM
- **Buffer Size**: 256 frames (~5.8ms latency)

### Performance Limits

- **Max Simultaneous Sounds**: 256 (configurable via `MAX_ACTIVE_SOUNDS`)
- **Supported Formats**: WAV, OGG/Vorbis, MP3
- **Max File Size**: Limited by available memory (streaming recommended for >10MB files)

## Usage

### Basic Setup

```rust
use engine_audio::AudioEngine;
use glam::Vec3;

// Create audio engine (automatically selects Android backend)
let mut audio = AudioEngine::new()?;

// Load sounds from Android storage
audio.load_sound("footstep", "/sdcard/sounds/footstep.wav")?;
audio.load_sound("music", "/data/data/com.example.game/files/music.ogg")?;
```

### Loading from APK Assets

For production apps, load from Android assets using the AssetManager:

```rust
use ndk::asset::Asset;
use std::io::Read;

// In your JNI code or native activity:
fn load_from_assets(
    audio: &mut AudioEngine,
    asset_manager: &AssetManager,
    filename: &str,
) -> Result<(), AudioError> {
    // Open asset
    let mut asset = asset_manager.open(filename)?;

    // Read to temporary file or decode directly
    let mut buffer = Vec::new();
    asset.read_to_end(&mut buffer)?;

    // Write to temporary file for decoding
    let temp_path = format!("/data/local/tmp/{}", filename);
    std::fs::write(&temp_path, &buffer)?;

    // Load sound
    audio.load_sound("asset_sound", &temp_path)?;

    Ok(())
}
```

### 2D Audio (UI Sounds)

```rust
// Play UI sound (non-spatial)
let instance_id = audio.play_2d("button_click", 1.0, false)?;

// Check if still playing
if audio.is_playing(instance_id) {
    println!("Sound is playing");
}

// Stop with fade out
audio.stop(instance_id, Some(0.3)); // 300ms fade
```

### 3D Spatial Audio

```rust
// Set listener position (usually the camera)
audio.set_listener_transform(
    Vec3::new(0.0, 1.8, 0.0),  // Position (player head height)
    Vec3::NEG_Z,                // Forward direction
    Vec3::Y,                    // Up direction
);

// Play 3D positioned sound
let entity_id = 42;
let instance_id = audio.play_3d(
    entity_id,
    "footstep",
    Vec3::new(5.0, 0.0, 0.0),  // 5 units to the right
    1.0,                        // Full volume
    false,                      // Don't loop
    50.0,                       // Max audible distance
)?;

// Update emitter position as entity moves
audio.update_emitter_position(entity_id, Vec3::new(6.0, 0.0, 1.0));

// Remove emitter when entity is destroyed
audio.remove_emitter(entity_id);
```

### Streaming Music

```rust
// Stream large audio file (background music)
let music_id = audio.play_stream(
    "/sdcard/music/background.ogg",
    0.7,    // 70% volume
    true,   // Loop
)?;

// Later: stop music with fade out
audio.stop(music_id, Some(2.0)); // 2 second fade
```

### Memory Management

```rust
// Clean up finished sounds periodically
audio.cleanup_finished();

// Check resource usage
println!("Active sounds: {}", audio.active_sound_count());
println!("Loaded sounds: {}", audio.loaded_sound_count());
```

## Android Lifecycle Integration

The audio backend must be paused/resumed with the Android activity lifecycle:

```rust
use engine_audio::platform::create_audio_backend;

// In your JNI code or GameActivity:

// On pause (app goes to background)
#[no_mangle]
pub extern "C" fn Java_com_example_game_MainActivity_onPause(
    env: JNIEnv,
    _class: JClass,
) {
    // Access your audio backend
    if let Some(backend) = get_audio_backend() {
        backend.pause().expect("Failed to pause audio");
    }
}

// On resume (app returns to foreground)
#[no_mangle]
pub extern "C" fn Java_com_example_game_MainActivity_onResume(
    env: JNIEnv,
    _class: JClass,
) {
    if let Some(backend) = get_audio_backend() {
        backend.resume().expect("Failed to resume audio");
    }
}
```

## 3D Audio Algorithm

The backend implements a simplified HRTF-like 3D audio system:

### Distance Attenuation

```
distance = |source_position - listener_position|

if distance < 1.0:
    gain = 1.0
else if distance > max_distance:
    gain = 0.0
else:
    gain = 1.0 - (distance / max_distance)²
```

### Stereo Panning

```
listener_right = cross(listener_forward, up)
to_source = normalize(source_position - listener_position)
pan = dot(to_source, listener_right)  // -1.0 (left) to 1.0 (right)

if pan < 0:
    right_channel *= (1.0 + pan)
else:
    left_channel *= (1.0 - pan)
```

This provides convincing spatial audio for most game scenarios without the complexity of full HRTF processing.

## File Format Support

### WAV (Waveform Audio File Format)

- **Decoder**: `hound` crate
- **Supported**: PCM 16-bit and 32-bit float, mono and stereo
- **Resampling**: Automatic conversion to 44.1kHz if needed
- **Use Case**: Short sound effects (footsteps, gunshots, UI sounds)

### OGG/Vorbis

- **Decoder**: `lewton` crate (pure Rust)
- **Supported**: Mono and stereo Vorbis streams
- **Resampling**: Automatic conversion to 44.1kHz if needed
- **Use Case**: Compressed sound effects and music (60-80% size reduction vs WAV)

### MP3 (MPEG Audio Layer 3)

- **Decoder**: `minimp3` crate
- **Supported**: MPEG-1, MPEG-2, MPEG-2.5, mono and stereo
- **Resampling**: Automatic conversion to 44.1kHz if needed
- **Use Case**: Music and ambient sounds (widely supported format)

### Format Recommendations

| Content Type | Recommended Format | Rationale |
|--------------|-------------------|-----------|
| UI Sounds | WAV | Minimal CPU overhead, instant playback |
| Footsteps, Impacts | WAV or OGG | Balance of quality and size |
| Music | OGG or MP3 | Large files benefit from compression |
| Ambient Loops | OGG | Good compression, seamless looping |
| Voice/Dialog | OGG or MP3 | Good speech compression |

## Performance Optimization

### Memory Usage

**Problem**: Loading many sounds consumes RAM.

**Solution**:
- Load only frequently-used sounds
- Use streaming for music and large files
- Unload sounds when changing levels/scenes

```rust
// Load common sounds once
audio.load_sound("footstep", "sounds/footstep.wav")?;
audio.load_sound("jump", "sounds/jump.wav")?;

// Stream music instead of loading
let music_id = audio.play_stream("music/level1.ogg", 0.8, true)?;
```

### CPU Usage

**Problem**: Many simultaneous sounds can cause frame drops.

**Solution**:
- Limit concurrent sounds (already limited to 256)
- Use `cleanup_finished()` regularly
- Reduce sound count in performance-critical sections

```rust
// In your game loop
if frame % 60 == 0 {
    audio.cleanup_finished();
}

// Limit gunshot spam
if audio.active_sound_count() < 50 {
    audio.play_2d("gunshot", 1.0, false)?;
}
```

### Battery Life

**Problem**: Continuous audio processing drains battery.

**Solution**:
- Reduce max_distance for 3D sounds (fewer active sounds)
- Use lower volume for distant sounds
- Pause audio when app is backgrounded (automatic with lifecycle handling)

```rust
// Adaptive max distance based on importance
let max_distance = if is_important {
    100.0  // Critical gameplay sound
} else {
    30.0   // Ambient/non-essential sound
};
```

## Testing on Device

### Setup Test Environment

1. **Build for Android**:
   ```bash
   # Add Android target
   rustup target add aarch64-linux-android

   # Build
   cargo build --target aarch64-linux-android --release
   ```

2. **Push Test Files**:
   ```bash
   adb push test_assets/footstep.wav /sdcard/
   adb push test_assets/music.ogg /sdcard/
   adb push test_assets/gunshot.mp3 /sdcard/
   ```

3. **Run Tests**:
   ```bash
   # Run ignored tests on device
   cargo test --target aarch64-linux-android -- --ignored

   # Run specific test
   cargo test --target aarch64-linux-android test_wav_playback_on_device -- --ignored
   ```

### Performance Benchmarks

```bash
# Run benchmarks on device
cargo bench --target aarch64-linux-android

# Run with actual audio files
cargo bench --target aarch64-linux-android --features device_benchmarks
```

### Expected Performance

| Metric | Target | Acceptable | Poor |
|--------|--------|------------|------|
| Backend Creation | <10ms | <50ms | >100ms |
| Sound Loading (WAV) | <50ms | <200ms | >500ms |
| Sound Loading (OGG) | <100ms | <400ms | >1s |
| Play 2D | <1ms | <5ms | >10ms |
| Play 3D | <2ms | <8ms | >15ms |
| Listener Update | <0.1ms | <0.5ms | >1ms |
| Concurrent Sounds (50) | <10ms | <30ms | >50ms |

## Debugging

### Enable Audio Logging

```bash
# Filter audio logs only
adb logcat | grep "engine_audio"

# All Rust logs
adb logcat | grep "RUST"
```

### Common Issues

#### "Failed to open audio stream"

**Cause**: Audio permissions not granted or device in use.

**Solution**:
- Add `RECORD_AUDIO` permission to AndroidManifest.xml (even for playback on some devices)
- Ensure no other app is using audio exclusively
- Check if headphones are properly connected

#### "Sound not playing"

**Cause**: Stream not started or instance stopped.

**Solution**:
```rust
// Ensure stream is started
let instance_id = audio.play_2d("sound", 1.0, false)?;

// Check if playing
if !audio.is_playing(instance_id) {
    eprintln!("Sound stopped immediately - check audio format");
}
```

#### "Crackling/distortion in audio"

**Cause**: Too many sounds playing simultaneously or clipping.

**Solution**:
- Reduce number of concurrent sounds
- Lower individual sound volumes
- Check for audio callback overruns in logs

#### "High latency"

**Cause**: Large buffer size or device performance.

**Solution**:
- Oboe automatically selects best buffer size
- On low-end devices, expect higher latency
- Consider reducing FRAMES_PER_BUFFER in source (rebuild required)

## Best Practices

### 1. Preload Sounds During Loading Screen

```rust
// During level load
audio.load_sound("footstep", "sounds/footstep.wav")?;
audio.load_sound("gunshot", "sounds/gunshot.wav")?;
audio.load_sound("explosion", "sounds/explosion.wav")?;
```

### 2. Use Object Pooling for Frequent Sounds

```rust
// Instead of playing rapidly
for _ in 0..10 {
    audio.play_2d("gunshot", 1.0, false)?;  // May hit limits
}

// Use rate limiting
let mut last_gunshot = Instant::now();
if last_gunshot.elapsed() > Duration::from_millis(100) {
    audio.play_2d("gunshot", 1.0, false)?;
    last_gunshot = Instant::now();
}
```

### 3. Fade Out Before Stopping

```rust
// Abrupt stop (can cause click/pop)
audio.stop(instance_id, None);

// Smooth fade (better)
audio.stop(instance_id, Some(0.2));  // 200ms fade
```

### 4. Clean Up Regularly

```rust
// In game loop (every second or so)
if frame_count % 60 == 0 {
    audio.cleanup_finished();
}
```

### 5. Handle Background Gracefully

```rust
// Pause all audio when app goes to background
fn on_pause() {
    audio.pause()?;
}

// Resume when app returns
fn on_resume() {
    audio.resume()?;
}
```

## Integration with ECS

See `engine/audio/src/system.rs` for full ECS integration example:

```rust
use engine_audio::{AudioEngine, AudioSystem, Sound, AudioListener};
use engine_core::ecs::World;

// In your game setup
let mut world = World::new();
let mut audio = AudioEngine::new()?;

// Add audio listener (camera)
let camera = world.spawn();
world.add(camera, AudioListener);
world.add(camera, Transform::from_translation(Vec3::new(0.0, 1.8, 0.0)));

// Add sound emitter (footsteps)
let player = world.spawn();
world.add(player, Sound::new("footstep", 1.0, true, 50.0));
world.add(player, Transform::from_translation(Vec3::new(5.0, 0.0, 0.0)));

// Update system
let audio_system = AudioSystem::new(audio);
audio_system.update(&mut world);
```

## Future Improvements

### Potential Enhancements

1. **True Streaming**: Currently streams entire file into memory. Could implement chunked loading for very large files.

2. **Better Resampling**: Uses linear interpolation. Could upgrade to sinc or cubic interpolation for better quality.

3. **Reverb/Effects**: Add DSP effects for indoor/outdoor environments.

4. **Doppler Effect**: Calculate frequency shift based on relative velocity.

5. **Audio Occlusion**: Muffle sounds behind walls using raycasting.

6. **Dynamic Range Compression**: Prevent clipping when many loud sounds play.

7. **Audio Groups**: Volume control for categories (music, SFX, dialog).

8. **Native AAudio API**: Direct AAudio bindings for Android 8.1+ (Oboe already does this).

## Related Documentation

- [Audio System Overview](../engine/audio/CLAUDE.md)
- [Platform Abstraction Guide](platform-abstraction.md)
- [Oboe Documentation](https://github.com/google/oboe)
- [Android Audio Developer Guide](https://developer.android.com/guide/topics/media/audio-app/overview)
