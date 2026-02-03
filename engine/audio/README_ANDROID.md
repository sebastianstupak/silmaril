# Android Audio Backend

Complete implementation of the audio backend for Android using Oboe.

## Features

- ✅ Low-latency audio playback using Oboe (AAudio + OpenSL ES)
- ✅ 3D spatial audio with distance attenuation and stereo panning
- ✅ Multiple audio format support (WAV, OGG/Vorbis, MP3)
- ✅ Streaming for large audio files
- ✅ Up to 256 simultaneous sounds
- ✅ Android lifecycle handling (pause/resume)
- ✅ Comprehensive unit tests
- ✅ Integration tests (requires device)
- ✅ Performance benchmarks

## Implementation Details

### Audio Pipeline

```
File (WAV/OGG/MP3) → Decoder → AudioBuffer (44.1kHz stereo f32) →
  SoundInstance → Audio Callback (mixing + 3D) → Oboe → Hardware
```

### Thread Safety

The implementation uses a shared state approach:
- **Main Thread**: API calls protected by mutex
- **Audio Thread**: High-priority callback for mixing
- **Lock-free reads**: Atomic operations where possible

### 3D Audio Algorithm

**Distance Attenuation**:
```
gain = 1.0 - (distance / max_distance)²
```

**Stereo Panning**:
```
pan = dot(to_source, listener_right)  // -1.0 to 1.0
left_channel *= (1.0 - pan) if pan > 0
right_channel *= (1.0 + pan) if pan < 0
```

## Building for Android

### Prerequisites

```bash
# Install Android NDK (version 25+)
# via Android Studio or command line

# Add Rust targets
rustup target add aarch64-linux-android
rustup target add armv7-linux-androideabi
```

### Build

```bash
# Build for ARM64 (most modern devices)
cargo build --target aarch64-linux-android --release

# Build for ARMv7 (older devices)
cargo build --target armv7-linux-androideabi --release
```

## Testing

### Unit Tests (Cross-Platform)

These tests run on any platform and don't require Android hardware:

```bash
cargo test --test android_audio_unit_test
```

### Integration Tests (Requires Device)

These tests require an actual Android device or emulator:

```bash
# Build tests
cargo test --target aarch64-linux-android --no-run

# Push test files to device
adb push test_assets/beep.wav /sdcard/
adb push test_assets/footstep.wav /sdcard/
adb push test_assets/music.ogg /sdcard/

# Find test binary
TEST_BIN=$(find target/aarch64-linux-android/debug/deps -name "android_audio_test-*" -type f | head -1)

# Push and run
adb push "$TEST_BIN" /data/local/tmp/test
adb shell /data/local/tmp/test

# Run ignored tests (require audio files)
adb shell /data/local/tmp/test --ignored
```

### Benchmarks

```bash
# Basic benchmarks (no audio files required)
cargo bench --target aarch64-linux-android --no-run
BENCH_BIN=$(find target/aarch64-linux-android/release/deps -name "android_audio_benches-*" -type f | head -1)
adb push "$BENCH_BIN" /data/local/tmp/bench
adb shell /data/local/tmp/bench

# Full benchmarks (requires audio files)
cargo bench --target aarch64-linux-android --features device_benchmarks --no-run
adb push test_assets/bench_test.wav /sdcard/
adb push test_assets/bench_music.ogg /sdcard/
adb shell /data/local/tmp/bench
```

## Performance Targets

| Operation | Target | Acceptable |
|-----------|--------|------------|
| Backend creation | <10ms | <50ms |
| Load WAV (1MB) | <50ms | <200ms |
| Load OGG (1MB) | <100ms | <400ms |
| Play 2D sound | <1ms | <5ms |
| Play 3D sound | <2ms | <8ms |
| Listener update | <0.1ms | <0.5ms |
| Emitter update | <0.1ms | <0.5ms |
| Concurrent sounds (50) | <10ms | <30ms |

## Code Organization

```
src/platform/android.rs         - Main implementation (1000+ lines)
tests/android_audio_test.rs     - Integration tests (requires device)
tests/android_audio_unit_test.rs - Unit tests (cross-platform)
benches/android_audio_benches.rs - Performance benchmarks
```

## Dependencies

- **oboe** (0.6): Low-latency audio I/O
- **hound** (3.5): WAV decoding
- **lewton** (0.10): OGG/Vorbis decoding
- **minimp3** (0.5): MP3 decoding
- **jni** (0.21): Java interop for asset loading
- **ndk-context** (0.1): Android NDK context

All dependencies are pure Rust and cross-compile to Android.

## API Examples

### Basic 2D Sound

```rust
use engine_audio::AudioEngine;

let mut audio = AudioEngine::new()?;
audio.load_sound("beep", "/sdcard/beep.wav")?;
let id = audio.play_2d("beep", 1.0, false)?;
```

### 3D Positioned Sound

```rust
use glam::Vec3;

audio.set_listener_transform(Vec3::ZERO, Vec3::NEG_Z, Vec3::Y);
let id = audio.play_3d(
    1,                           // entity ID
    "footstep",
    Vec3::new(10.0, 0.0, 0.0),  // position
    1.0,                         // volume
    false,                       // looping
    50.0,                        // max distance
)?;
```

### Streaming Music

```rust
let id = audio.play_stream(
    "/sdcard/music.ogg",
    0.7,   // volume
    true,  // loop
)?;
```

### Lifecycle Management

```rust
// In your Android activity callbacks:
fn on_pause() {
    audio.pause()?;
}

fn on_resume() {
    audio.resume()?;
}
```

## Troubleshooting

### Audio not playing

1. Check permissions in AndroidManifest.xml
2. Verify file exists: `adb shell ls /sdcard/`
3. Check logs: `adb logcat | grep engine_audio`
4. Ensure stream is started (happens automatically on first play)

### Crackling/distortion

1. Reduce number of concurrent sounds
2. Lower individual volumes
3. Call `cleanup_finished()` regularly
4. Check for audio callback overruns in logs

### High memory usage

1. Use streaming for large files (>10MB)
2. Unload unused sounds between levels
3. Limit preloaded sound count

### High latency

1. Oboe automatically optimizes buffer size
2. Expected latency: 5-20ms depending on device
3. Lower-end devices may have higher latency

## Documentation

- [Android Audio Backend Guide](../../docs/android-audio-backend.md) - Full documentation
- [Quick Start Guide](../../docs/android-audio-quick-start.md) - Get started in 5 minutes
- [Platform Abstraction](../../docs/platform-abstraction.md) - Architecture overview

## Related Files

- `src/platform/mod.rs` - Platform abstraction trait
- `src/engine.rs` - High-level audio engine API
- `src/error.rs` - Error types
- `Cargo.toml` - Android-specific dependencies

## Contributing

When modifying the Android backend:

1. Run unit tests: `cargo test --test android_audio_unit_test`
2. Test on device with various Android versions
3. Run benchmarks to ensure no performance regressions
4. Update documentation if API changes
5. Follow the coding standards in `docs/rules/coding-standards.md`

## License

Part of agent-game-engine, licensed under Apache-2.0.
