# Engine Audio

Cross-platform 3D spatial audio system for the agent-game-engine.

## Features

- **Cross-Platform**: Automatic backend selection for Desktop, Web, Android, and iOS
- **3D Spatial Audio**: HRTF-based positioning with distance attenuation
- **2D Audio**: Non-spatial audio for UI and menu sounds
- **Audio Streaming**: Efficient streaming for background music and large files
- **Audio Effects**: Reverb, echo, filters, and EQ (platform-dependent)
- **ECS Integration**: AudioListener and Sound components

## Platform Backends

| Platform | Backend | Features |
|----------|---------|----------|
| Windows, Linux, macOS | Kira | Full feature set, low latency |
| Web (WASM) | Web Audio API | Browser-native, HRTF 3D audio |
| Android | OpenSL ES / AAudio | Hardware acceleration |
| iOS | Core Audio | Native integration |

## Quick Start

```rust
use engine_audio::AudioEngine;
use glam::Vec3;

// Create audio engine (platform selected automatically)
let mut audio = AudioEngine::new()?;

// Load sound
audio.load_sound("footstep", "assets/footstep.wav")?;

// Play 2D sound
let ui_sound = audio.play_2d("footstep", 1.0, false)?;

// Play 3D spatial sound
let spatial_sound = audio.play_3d(
    entity_id,
    "footstep",
    Vec3::new(5.0, 0.0, 0.0),
    1.0,   // volume
    false, // looping
    50.0,  // max distance
)?;

// Update listener (camera position)
audio.set_listener_transform(
    camera_pos,
    camera_forward,
    camera_up,
);
```

## Building for Different Platforms

### Desktop (Windows, Linux, macOS)

```bash
cargo build --package engine-audio
cargo test --package engine-audio
cargo bench --package engine-audio
```

### Web (WASM)

```bash
# Install wasm-pack
cargo install wasm-pack

# Build for web
wasm-pack build --target web engine/audio

# Run tests in browser
wasm-pack test --headless --firefox engine/audio

# Run benchmarks
wasm-pack test --headless --chrome engine/audio --bench
```

### Android

```bash
# Add Android targets
rustup target add aarch64-linux-android armv7-linux-androideabi

# Build
cargo build --package engine-audio --target aarch64-linux-android
```

### iOS

```bash
# Add iOS targets
rustup target add aarch64-apple-ios x86_64-apple-ios

# Build
cargo build --package engine-audio --target aarch64-apple-ios
```

## Supported Audio Formats

| Format | Desktop | Web | Android | iOS |
|--------|---------|-----|---------|-----|
| WAV | Yes | Yes | Yes | Yes |
| OGG Vorbis | Yes | Yes* | Yes | No |
| MP3 | Yes | Yes | Yes | Yes |
| AAC | Yes | Yes* | Yes | Yes |
| FLAC | Yes | Yes* | Yes | No |

*Browser-dependent

## Testing

### Unit Tests

```bash
# All platforms
cargo test --package engine-audio

# WASM-specific tests
wasm-pack test --headless --firefox engine/audio
```

### Integration Tests

```bash
# Test with actual audio files
cargo test --package engine-audio --test '*' -- --ignored
```

### Benchmarks

```bash
# All benchmarks
cargo bench --package engine-audio

# Specific benchmark suites
cargo bench --package engine-audio --bench audio_benches          # Core audio operations
cargo bench --package engine-audio --bench spatial_audio_benches  # 3D spatial audio
cargo bench --package engine-audio --bench effects_benches        # Audio effects processing
cargo bench --package engine-audio --bench scalability_benches    # Scalability (1-100k sounds)
cargo bench --package engine-audio --bench memory_benches         # Memory allocations & usage
cargo bench --package engine-audio --bench cache_benches          # Cache efficiency
cargo bench --package engine-audio --bench doppler_benches        # Doppler effect calculations
cargo bench --package engine-audio --bench simd_batch_benches     # SIMD batch processing

# WASM benchmarks
wasm-pack test --headless --chrome engine/audio --bench
```

**Benchmark Suites:**

- `audio_benches` - Core audio operations (engine creation, playback, cleanup)
- `spatial_audio_benches` - 3D audio positioning and listener updates
- `effects_benches` - Audio effect processing (reverb, echo, filters)
- `scalability_benches` - Performance at scale (1, 10, 100, 1k, 10k, 100k sounds)
- `memory_benches` - Memory allocation tracking and leak detection
- `cache_benches` - Cache efficiency and data locality optimization
- `doppler_benches` - Doppler effect pitch calculations
- `simd_batch_benches` - SIMD-optimized batch operations

**Performance Validation:**

The benchmark suites validate these targets:
- 10k simultaneous sounds: < 16ms frame time
- 1k simultaneous sounds: < 1ms frame time
- Hot path allocations: < 1KB per frame
- Cache miss rate: < 5%

## Documentation

- [Web Audio Backend Guide](../../docs/web-audio-backend.md) - Detailed Web Audio API documentation
- [Audio System Architecture](../../docs/audio.md) - Overall audio system design
- [Platform Abstraction](../../docs/platform-abstraction.md) - Cross-platform architecture

## Examples

### Streaming Background Music

```rust
// Stream large audio files without loading into memory
let music = audio.play_stream(
    "assets/music/background.ogg",
    0.5,  // volume
    true, // loop
)?;

// Stop with fade-out
audio.stop(music, Some(2.0)); // 2-second fade
```

### Managing Spatial Audio

```rust
// Update emitter positions every frame
for (entity, position) in world.query::<&Position>() {
    audio.update_emitter_position(entity.id(), position.0);
}

// Clean up finished sounds periodically
if frame_count % 60 == 0 {
    audio.cleanup_finished();
}
```

### Audio Effects (Desktop/Mobile only)

```rust
use engine_audio::{AudioEffect, ReverbEffect};

// Add reverb effect
let reverb = AudioEffect::Reverb(ReverbEffect {
    room_size: 0.8,
    dampening: 0.5,
    wet_level: 0.3,
});

let effect_id = audio.add_effect(sound_id, reverb)?;

// Remove effect
audio.remove_effect(sound_id, effect_id);
```

## Performance Targets

| Metric | Target | Notes |
|--------|--------|-------|
| Simultaneous sounds | 256+ | Platform-dependent |
| Playback latency | < 10ms | Desktop/Mobile |
| Playback latency (Web) | < 20ms | Browser-dependent |
| Memory per sound | ~1MB | Decoded PCM |
| CPU overhead | < 1% | Per 100 active sounds |

## Troubleshooting

### Web: "Failed to create AudioContext"

**Cause**: Browser autoplay policy requires user interaction.

**Solution**: Create AudioEngine after user click/tap.

### Web: "Sound not playing"

**Cause**: File not found or format not supported.

**Debug**:
```rust
// Enable debug logging
RUST_LOG=engine_audio=debug cargo run
```

### Desktop: "Failed to load sound"

**Cause**: Missing audio file or unsupported format.

**Solution**: Verify file path and format support.

## Contributing

When adding features to the audio system:

1. Update all platform backends (kira, web, android, ios)
2. Add tests for each platform
3. Update documentation
4. Run benchmarks to ensure no performance regression

## License

Apache-2.0
