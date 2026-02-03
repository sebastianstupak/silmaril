# Android Audio Backend Testing Guide

Comprehensive testing strategy for the Android audio backend.

## Test Architecture

The Android audio backend has three tiers of testing:

1. **Unit Tests** - Pure logic tests (no Android required)
2. **Integration Tests** - Full backend tests (requires Android device/emulator)
3. **Performance Benchmarks** - Performance validation (requires Android device)

## Unit Tests

### Location
`engine/audio/tests/android_audio_unit_test.rs`

### Running
```bash
# Run on desktop (any platform)
cargo test --test android_audio_unit_test

# Specific test
cargo test --test android_audio_unit_test test_3d_audio_distance_attenuation
```

### Coverage

These tests verify core algorithms without requiring Android:

- ✅ 3D audio distance attenuation calculation
- ✅ Stereo panning algorithm
- ✅ Sample rate conversion math
- ✅ Linear interpolation
- ✅ Audio frame indexing
- ✅ Volume application
- ✅ Sample clamping
- ✅ Fade out calculation
- ✅ Format conversion (i16 → f32)
- ✅ Panning application
- ✅ Distance calculation
- ✅ Audio mixing
- ✅ Looping behavior
- ✅ Mono to stereo conversion

### Example

```bash
$ cargo test --test android_audio_unit_test

running 20 tests
test test_3d_audio_distance_attenuation ... ok
test test_3d_audio_stereo_panning ... ok
test test_stereo_resampling_ratio ... ok
test test_linear_interpolation ... ok
...
test result: ok. 20 passed; 0 failed; 0 ignored
```

## Integration Tests

### Location
`engine/audio/tests/android_audio_test.rs`

### Device Setup

1. **Connect Android device**:
   ```bash
   adb devices
   ```

2. **Push test audio files**:
   ```bash
   # Create test files or use existing ones
   adb push test_assets/test.wav /sdcard/
   adb push test_assets/test.ogg /sdcard/
   adb push test_assets/test.mp3 /sdcard/
   adb push test_assets/music.ogg /sdcard/
   adb push test_assets/footstep.wav /sdcard/
   adb push test_assets/gunshot.wav /sdcard/
   adb push test_assets/ambient.wav /sdcard/
   adb push test_assets/loop.wav /sdcard/
   adb push test_assets/siren.wav /sdcard/
   ```

### Running Tests

#### Basic Tests (No Audio Files)

These tests don't require audio files and can run on any device:

```bash
# Build
cargo test --target aarch64-linux-android --no-run

# Find binary
TEST_BIN=$(find target/aarch64-linux-android/debug/deps -name "android_audio_test-*" -type f | head -1)

# Push to device
adb push "$TEST_BIN" /data/local/tmp/audio_test

# Run
adb shell /data/local/tmp/audio_test

# Example output:
running 13 tests
test test_android_backend_creation ... ok
test test_listener_transform ... ok
test test_emitter_management ... ok
...
test result: ok. 13 passed; 0 failed; 0 ignored
```

#### Full Tests (Requires Audio Files)

These tests require actual audio files on the device:

```bash
# Run with ignored tests
adb shell /data/local/tmp/audio_test --ignored

running 12 tests
test test_wav_playback_on_device ... ok
test test_ogg_playback_on_device ... ok
test test_mp3_playback_on_device ... ok
test test_3d_audio_on_device ... ok
test test_looping_sound_on_device ... ok
test test_many_simultaneous_sounds_on_device ... ok
test test_streaming_music_on_device ... ok
...
test result: ok. 12 passed; 0 failed; 0 ignored
```

### Integration Test Coverage

**Basic Tests** (no files required):
- ✅ Backend creation
- ✅ Listener transform updates
- ✅ Emitter management
- ✅ Error handling (missing files)
- ✅ Instance lifecycle
- ✅ Cleanup operations
- ✅ State queries (active count, etc.)

**Full Tests** (require audio files):
- ✅ WAV playback
- ✅ OGG playback
- ✅ MP3 playback
- ✅ 3D spatial audio
- ✅ Looping sounds
- ✅ Many simultaneous sounds (50+)
- ✅ Streaming music
- ✅ Distance falloff
- ✅ Emitter movement
- ✅ Fade out

### Manual Testing Checklist

Use this for manual validation:

- [ ] Sound plays on device speakers
- [ ] Sound plays through headphones
- [ ] 3D audio pans left/right correctly
- [ ] Distance attenuation works (quieter when far)
- [ ] Looping works without gaps/clicks
- [ ] Fade out is smooth
- [ ] Multiple sounds play simultaneously
- [ ] App survives pause/resume
- [ ] No crashes under load (100+ sounds)
- [ ] No memory leaks (check `adb shell dumpsys meminfo`)

## Performance Benchmarks

### Location
`engine/audio/benches/android_audio_benches.rs`

### Running

#### Basic Benchmarks (No Audio Files)

```bash
# Build
cargo bench --target aarch64-linux-android --no-run

# Find binary
BENCH_BIN=$(find target/aarch64-linux-android/release/deps -name "android_audio_benches-*" -type f | head -1)

# Push and run
adb push "$BENCH_BIN" /data/local/tmp/audio_bench
adb shell /data/local/tmp/audio_bench

# Example output:
android_backend_creation  time: [8.234 ms 8.456 ms 8.678 ms]
android_listener_update   time: [0.089 µs 0.092 µs 0.095 µs]
android_emitter_update    time: [0.134 µs 0.138 µs 0.142 µs]
...
```

#### Full Benchmarks (With Audio Files)

```bash
# Push test files
adb push test_assets/bench_test.wav /sdcard/
adb push test_assets/bench_music.ogg /sdcard/

# Build with device_benchmarks feature
cargo bench --target aarch64-linux-android --features device_benchmarks --no-run

# Push and run
BENCH_BIN=$(find target/aarch64-linux-android/release/deps -name "android_audio_benches-*" -type f | head -1)
adb push "$BENCH_BIN" /data/local/tmp/audio_bench_full
adb shell /data/local/tmp/audio_bench_full

# Example output:
android_load_wav          time: [42.123 ms 43.567 ms 45.012 ms]
android_play_2d           time: [0.678 ms 0.712 ms 0.746 ms]
android_play_3d           time: [1.234 ms 1.289 ms 1.345 ms]
...
```

### Benchmark Coverage

**Basic Benchmarks**:
- Backend creation time
- Listener update latency
- Emitter update latency
- Emitter scaling (10, 50, 100, 500, 1000)
- Is-playing check overhead
- Cleanup operation time
- Active sound count query

**Full Benchmarks** (requires audio files):
- WAV loading time
- OGG loading time
- MP3 loading time
- 2D sound playback latency
- 3D sound playback latency
- Concurrent sounds (10, 50, 100, 200)
- 3D audio calculation overhead
- Streaming start latency

### Performance Baselines

**Low-end device** (e.g., Android 8.0, Snapdragon 450):
- Backend creation: <50ms
- Load WAV (1MB): <200ms
- Play 2D: <5ms
- Play 3D: <10ms
- 50 concurrent sounds: <50ms

**Mid-range device** (e.g., Android 11, Snapdragon 730):
- Backend creation: <20ms
- Load WAV (1MB): <100ms
- Play 2D: <2ms
- Play 3D: <5ms
- 50 concurrent sounds: <20ms

**High-end device** (e.g., Android 13, Snapdragon 8 Gen 2):
- Backend creation: <10ms
- Load WAV (1MB): <50ms
- Play 2D: <1ms
- Play 3D: <2ms
- 50 concurrent sounds: <10ms

## Continuous Integration

### GitHub Actions Setup

```yaml
name: Android Audio Tests

on: [push, pull_request]

jobs:
  test-android:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Setup Android SDK
        uses: android-actions/setup-android@v2

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          target: aarch64-linux-android

      - name: Build tests
        run: cargo test --target aarch64-linux-android --no-run

      # Note: Can't run on device in CI without emulator setup
      # Consider using Android Test Orchestrator or Firebase Test Lab
```

### Local Pre-Commit Hook

Add to `.git/hooks/pre-commit`:

```bash
#!/bin/bash
# Test Android audio unit tests before commit

echo "Running Android audio unit tests..."
cargo test --test android_audio_unit_test

if [ $? -ne 0 ]; then
    echo "Android audio unit tests failed!"
    exit 1
fi

echo "Android audio unit tests passed!"
```

## Debugging

### Enable Verbose Logging

```bash
# Filter audio logs
adb logcat | grep "engine_audio"

# All Rust logs
adb logcat | grep "RustNative"

# Specific log level
adb logcat *:E | grep "engine_audio"  # Errors only
```

### Common Issues and Solutions

#### Issue: "Failed to open audio stream"

**Debug**:
```bash
# Check if audio is available
adb shell dumpsys media.audio_flinger

# Check permissions
adb shell dumpsys package com.your.package | grep "MODIFY_AUDIO"
```

**Solution**:
- Add `MODIFY_AUDIO_SETTINGS` to AndroidManifest.xml
- Ensure no other app is using exclusive audio mode

#### Issue: Sounds not playing

**Debug**:
```bash
# Check if file exists
adb shell ls -l /sdcard/your_sound.wav

# Check file format
adb pull /sdcard/your_sound.wav .
file your_sound.wav

# Test with desktop audio tools
ffmpeg -i your_sound.wav
```

**Solution**:
- Verify file path is correct
- Ensure file format is supported (WAV, OGG, MP3)
- Check file isn't corrupted

#### Issue: Crackling/distortion

**Debug**:
```bash
# Monitor audio callback performance
adb logcat | grep "audio callback"

# Check CPU usage
adb shell top | grep your_package
```

**Solution**:
- Reduce concurrent sound count
- Lower volume levels
- Call `cleanup_finished()` more frequently
- Reduce audio buffer size (requires recompile)

#### Issue: Memory leak

**Debug**:
```bash
# Monitor memory usage over time
while true; do
    adb shell dumpsys meminfo com.your.package | grep "TOTAL"
    sleep 5
done
```

**Solution**:
- Ensure `cleanup_finished()` is called regularly
- Check for retained `Arc<AudioBuffer>` references
- Use streaming for large files instead of loading

## Test Data

### Creating Test Audio Files

```bash
# Generate test WAV (1 second, 440Hz sine wave)
ffmpeg -f lavfi -i "sine=frequency=440:duration=1" -ar 44100 -ac 2 test.wav

# Generate looping ambient (5 seconds)
ffmpeg -f lavfi -i "anoisesrc=d=5:c=pink" -ar 44100 -ac 2 ambient.wav

# Convert to OGG
ffmpeg -i test.wav -c:a libvorbis -q:a 4 test.ogg

# Convert to MP3
ffmpeg -i test.wav -c:a libmp3lame -b:a 192k test.mp3
```

### Recommended Test Files

| File | Purpose | Format | Size | Duration |
|------|---------|--------|------|----------|
| beep.wav | Basic playback | WAV | 10KB | 0.1s |
| footstep.wav | 3D positioning | WAV | 50KB | 0.5s |
| gunshot.wav | Many concurrent | WAV | 30KB | 0.3s |
| ambient.wav | Looping | WAV | 200KB | 2.0s |
| music.ogg | Streaming | OGG | 2MB | 30s |
| loop.wav | Seamless loop | WAV | 100KB | 1.0s |
| siren.wav | Movement test | WAV | 150KB | 1.5s |

## Test Reports

### Generating Test Reports

```bash
# Run tests with JSON output
adb shell /data/local/tmp/audio_test -- --format json > test_results.json

# Generate HTML report
python scripts/generate_test_report.py test_results.json > report.html
```

### Benchmark Reports

```bash
# Generate baseline
adb shell /data/local/tmp/audio_bench --save-baseline baseline

# Compare against baseline
adb shell /data/local/tmp/audio_bench --baseline baseline

# Export to CSV
adb shell /data/local/tmp/audio_bench -- --output-format csv > benchmarks.csv
```

## Best Practices

1. **Always test on multiple devices**:
   - Low-end (Android 8.0)
   - Mid-range (Android 11)
   - High-end (Android 13+)

2. **Test with different audio outputs**:
   - Built-in speaker
   - Wired headphones
   - Bluetooth headphones
   - USB audio

3. **Test under various conditions**:
   - Low battery
   - High CPU load
   - Background apps
   - Phone calls (interruption)

4. **Automated regression testing**:
   - Run benchmarks before/after changes
   - Compare against baseline
   - Flag >10% performance regressions

5. **Memory profiling**:
   - Monitor for leaks (increase over time)
   - Check allocation patterns
   - Validate cleanup

## Related Documentation

- [Android Audio Backend](android-audio-backend.md)
- [Quick Start Guide](android-audio-quick-start.md)
- [Testing Architecture](TESTING_ARCHITECTURE.md)
