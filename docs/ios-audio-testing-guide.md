# iOS Audio Backend Testing Guide

## Quick Start

This guide covers testing the iOS audio backend on real devices and simulators.

## Prerequisites

### Development Environment

1. **macOS** (required for iOS development)
2. **Xcode** 14.0 or later
3. **iOS SDK** 15.0 or later
4. **Rust** with iOS targets:

```bash
# Install iOS targets
rustup target add aarch64-apple-ios        # Physical devices
rustup target add aarch64-apple-ios-sim    # M1/M2 simulators
rustup target add x86_64-apple-ios         # Intel simulators

# Install cargo-lipo (for building iOS frameworks)
cargo install cargo-lipo
```

### iOS Simulator Setup

```bash
# List available simulators
xcrun simctl list devices

# Boot a simulator
xcrun simctl boot "iPhone 14"

# Or use Xcode GUI: Xcode → Window → Devices and Simulators
```

### Physical Device Setup

1. Connect iOS device via USB
2. Enable Developer Mode (Settings → Privacy & Security → Developer Mode)
3. Trust your Mac (popup on device)
4. Add device in Xcode (Window → Devices and Simulators)

## Running Tests

### Unit Tests (No Audio Files Required)

```bash
# Run on simulator (M1/M2 Mac)
cargo test --target aarch64-apple-ios-sim --package engine-audio

# Run on simulator (Intel Mac)
cargo test --target x86_64-apple-ios --package engine-audio

# Run on physical device
cargo test --target aarch64-apple-ios --package engine-audio
```

### Integration Tests (Require Test Assets)

First, prepare test audio files:

```bash
# Create test assets directory
mkdir -p engine/audio/test_assets

# Add test files (see "Test Assets" section below)
```

Run integration tests:

```bash
# Run ignored tests (require audio files)
cargo test --target aarch64-apple-ios-sim --package engine-audio -- --ignored

# Run specific test
cargo test --target aarch64-apple-ios-sim --package engine-audio test_load_wav_file -- --ignored
```

### Running Tests in Xcode

For better debugging, run tests through Xcode:

1. Generate Xcode project:
```bash
cargo-xcode --package engine-audio
```

2. Open in Xcode:
```bash
open engine-audio.xcodeproj
```

3. Select target: Product → Scheme → engine-audio-tests
4. Select device/simulator
5. Run: Product → Test (⌘U)

## Running Benchmarks

### Basic Benchmarks (No Audio Files)

```bash
# Run on simulator
cargo bench --target aarch64-apple-ios-sim --package engine-audio --bench ios_backend_benches

# Run on physical device (more accurate)
cargo bench --target aarch64-apple-ios --package engine-audio --bench ios_backend_benches
```

### Full Benchmarks (With Test Assets)

```bash
# Enable asset-dependent benchmarks
cargo bench --target aarch64-apple-ios --package engine-audio --bench ios_backend_benches --features bench_with_assets
```

### Profiling with Instruments

For detailed performance analysis:

1. Build with release profile:
```bash
cargo build --release --target aarch64-apple-ios --package engine-audio
```

2. Create Xcode project (if needed):
```bash
cargo-xcode --package engine-audio
```

3. Open in Xcode and run with Instruments:
   - Product → Profile (⌘I)
   - Choose template: Time Profiler or Allocations
   - Run and analyze

## Test Assets

### Required Files

Create these test audio files in `engine/audio/test_assets/`:

| Filename | Format | Duration | Description |
|----------|--------|----------|-------------|
| `test.wav` | WAV 16-bit | 0.5-1s | Basic test sound |
| `loop.wav` | WAV 16-bit | 2-4s | Seamless loop |
| `short.wav` | WAV 16-bit | 0.1-0.3s | Very short sound |
| `music.wav` | WAV/AAC | 30-60s | Music track |
| `sound1.wav` | WAV 16-bit | 0.5s | Additional sound |
| `sound2.wav` | WAV 16-bit | 0.5s | Additional sound |

### Generating Test Assets

Using `ffmpeg` (recommended):

```bash
cd engine/audio/test_assets

# Generate 1-second sine wave (440 Hz)
ffmpeg -f lavfi -i "sine=frequency=440:duration=1" -ar 44100 -ac 2 -c:a pcm_s16le test.wav

# Generate short beep (0.2 seconds)
ffmpeg -f lavfi -i "sine=frequency=880:duration=0.2" -ar 44100 -ac 2 -c:a pcm_s16le short.wav

# Generate seamless loop (2 seconds)
ffmpeg -f lavfi -i "sine=frequency=440:duration=2" -ar 44100 -ac 2 -c:a pcm_s16le loop.wav

# Copy test.wav to sound1.wav and sound2.wav
cp test.wav sound1.wav
cp test.wav sound2.wav

# Generate 30-second music (for streaming tests)
ffmpeg -f lavfi -i "sine=frequency=440:duration=30" -ar 44100 -ac 2 -c:a aac -b:a 128k music.m4a
```

Or use actual sound effects from a royalty-free library:
- [Freesound.org](https://freesound.org/)
- [OpenGameArt.org](https://opengameart.org/)

### Asset Licensing

Test assets should be:
- Public domain, or
- Licensed under CC0/CC-BY, or
- Created by you

Add attribution in `test_assets/README.md`:

```markdown
# Test Assets Attribution

- `test.wav`: Generated with ffmpeg (public domain)
- `footstep.wav`: From Freesound.org, CC0 license
```

## Testing on Real Devices

### Manual Testing Checklist

#### Basic Functionality
- [ ] Backend initializes without errors
- [ ] Sounds load successfully
- [ ] 2D sounds play audibly
- [ ] 3D sounds play with correct spatialization
- [ ] Volume control works (0.0 to 1.0)
- [ ] Looping sounds loop correctly
- [ ] Sounds stop when requested

#### Spatial Audio
- [ ] Left/right panning works correctly
- [ ] Front/back distinction is audible (use headphones)
- [ ] Distance attenuation works (sound gets quieter with distance)
- [ ] Listener rotation affects sound direction
- [ ] Moving emitters track correctly

#### Interruptions
- [ ] Phone call pauses audio
- [ ] Audio resumes after call ends
- [ ] Siri activation pauses audio
- [ ] Timer/alarm interrupts audio
- [ ] Audio recovers from all interruption types

#### Background Mode
- [ ] Audio continues when app enters background (if enabled)
- [ ] Sound effects stop in background (expected)
- [ ] Background music continues (expected)
- [ ] Audio resumes when returning to foreground

#### Device Compatibility
- [ ] Works on iPhone (all supported models)
- [ ] Works on iPad
- [ ] Works with wired headphones
- [ ] Works with AirPods/AirPods Pro
- [ ] Works with Bluetooth speakers
- [ ] Works with built-in speaker

### Automated Testing

Create an automated test script:

```rust
// engine/audio/tests/device_test.rs
#[cfg(target_os = "ios")]
#[test]
fn automated_device_test() {
    use engine_audio::platform::create_audio_backend;
    use std::path::Path;
    use std::thread;
    use std::time::Duration;

    let mut backend = create_audio_backend().expect("Backend creation failed");

    // Load test sound
    backend.load_sound("test", Path::new("test_assets/test.wav"))
        .expect("Failed to load sound");

    // Play sound
    let id = backend.play_2d("test", 1.0, false).expect("Failed to play");

    // Verify it's playing
    thread::sleep(Duration::from_millis(100));
    assert!(backend.is_playing(id), "Sound should be playing");

    // Wait for completion
    thread::sleep(Duration::from_secs(2));

    // Verify it stopped
    assert!(!backend.is_playing(id), "Sound should have stopped");

    println!("✓ Device test passed");
}
```

Run on device:

```bash
cargo test --target aarch64-apple-ios automated_device_test -- --nocapture
```

## Debugging

### Enable Debug Logging

```bash
# Set log level
export RUST_LOG=engine_audio=debug

# Run tests with logging
cargo test --target aarch64-apple-ios-sim --package engine-audio -- --nocapture
```

### Common Issues

#### "Failed to get AVAudioSession"

**Symptoms:** Backend initialization fails.

**Fix:** This usually happens in simulator. Try on a real device.

#### "No sound output"

**Symptoms:** Tests pass but no audio is heard.

**Fix:**
1. Check device volume (physical buttons)
2. Check silent switch (should be off)
3. Restart audio session:
```rust
backend.drop();
backend = create_audio_backend()?;
```

#### "Sound plays in wrong ear"

**Symptoms:** Left/right channels are swapped.

**Fix:** Check listener orientation:
```rust
// Correct orientation
backend.set_listener_transform(
    position,
    Vec3::new(0.0, 0.0, -1.0),  // Forward = -Z
    Vec3::new(0.0, 1.0, 0.0),   // Up = +Y
);
```

#### "Tests timeout"

**Symptoms:** Tests hang indefinitely.

**Fix:**
1. Run with single thread: `--test-threads=1`
2. Increase timeout: `--test-timeout=60`
3. Check for deadlocks in audio session management

### Xcode Console Logs

View iOS system logs:

1. Window → Devices and Simulators
2. Select device
3. Click "Open Console"
4. Filter by process name: `engine-audio-tests`

Look for Audio Session warnings:
- "Audio session activation failed"
- "Interrupted by another audio session"
- "Route change detected"

## Performance Testing

### Target Metrics

| Operation | Target | Measurement |
|-----------|--------|-------------|
| Backend init | < 50ms | `cargo bench` |
| Load sound | < 20ms/MB | `cargo bench` |
| Play sound | < 1ms | `cargo bench` |
| Update listener | < 100μs | `cargo bench` |
| Update emitter | < 50μs | `cargo bench` |

### Running Performance Tests

```bash
# Run all benchmarks
cargo bench --target aarch64-apple-ios --package engine-audio

# Run specific benchmark
cargo bench --target aarch64-apple-ios --package engine-audio ios_listener_transform_update

# Save baseline
cargo bench --target aarch64-apple-ios --package engine-audio -- --save-baseline main

# Compare to baseline
cargo bench --target aarch64-apple-ios --package engine-audio -- --baseline main
```

### Memory Profiling

Use Xcode Instruments:

1. Build with release profile
2. Profile → Allocations
3. Look for:
   - Audio buffer leaks
   - Growing memory usage
   - Excessive allocations in hot paths

Target: < 50MB total for 100 active sounds

### Battery Impact

Test battery drain:

1. Run game for 30 minutes on device
2. Check battery usage: Settings → Battery
3. Your app should use < 10% battery per hour for audio alone

Optimize by:
- Reducing simultaneous sounds
- Using hardware-decoded formats (AAC)
- Cleaning up finished sounds regularly

## CI/CD Integration

### GitHub Actions (Example)

```yaml
name: iOS Audio Tests

on: [push, pull_request]

jobs:
  test-ios:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install Rust iOS targets
        run: |
          rustup target add aarch64-apple-ios-sim

      - name: Run iOS tests
        run: |
          cargo test --target aarch64-apple-ios-sim --package engine-audio

      - name: Run iOS benchmarks
        run: |
          cargo bench --target aarch64-apple-ios-sim --package engine-audio --bench ios_backend_benches --no-run
```

**Note:** Full benchmark runs require physical devices, which are not available in GitHub Actions.

## Best Practices

### Test Organization

1. **Unit tests** (fast, no files): Test in CI
2. **Integration tests** (require files): Mark with `#[ignore]`, run manually
3. **Device tests** (require hardware): Run on specific test devices
4. **Performance tests**: Run on physical devices, not simulators

### Test Data Management

- Keep test assets small (< 1MB total)
- Use generated audio when possible
- Document asset sources and licenses
- Don't commit large audio files to git

### Test Coverage

Aim for:
- 90%+ code coverage on core logic
- 100% coverage on public API
- Manual testing on 3+ device types
- Testing on iOS 15+ versions

## Troubleshooting CI

### Tests Pass Locally but Fail in CI

**Common causes:**
1. Audio files not in git repository
2. Different iOS SDK version
3. Simulator vs device differences
4. Timing-dependent tests (add delays)

**Fix:**
- Use generated audio in CI
- Pin Xcode version in CI config
- Add retry logic for flaky tests

### Benchmarks Won't Run

**Cause:** Criterion needs more time than CI allows.

**Fix:**
```bash
# Reduce measurement time in CI
cargo bench --package engine-audio -- --measurement-time 1
```

## Resources

- [Apple Audio Documentation](https://developer.apple.com/audio/)
- [AVFoundation Programming Guide](https://developer.apple.com/library/archive/documentation/AudioVideo/Conceptual/AVFoundationPG/)
- [Testing iOS Apps](https://developer.apple.com/documentation/xctest)
- [Rust iOS Development](https://github.com/rust-lang/rust/tree/master/src/tools/rustup#ios)

## Support

For issues with the iOS audio backend:

1. Check this guide first
2. Search existing issues on GitHub
3. Create a new issue with:
   - Device model and iOS version
   - Minimal reproduction case
   - Console logs
   - Expected vs actual behavior
