# Android Audio Quick Start Guide

Get started with the Android audio backend in 5 minutes.

## Prerequisites

1. **Android NDK** installed (version 25 or later)
2. **Rust Android targets**:
   ```bash
   rustup target add aarch64-linux-android
   rustup target add armv7-linux-androideabi
   rustup target add i686-linux-android
   rustup target add x86_64-linux-android
   ```

3. **Android device or emulator** with API level 21+ (Android 5.0+)

## Setup

### 1. Configure Cargo

Create or update `.cargo/config.toml`:

```toml
[target.aarch64-linux-android]
ar = "~/Android/Sdk/ndk/25.2.9519653/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar"
linker = "~/Android/Sdk/ndk/25.2.9519653/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android30-clang"

[target.armv7-linux-androideabi]
ar = "~/Android/Sdk/ndk/25.2.9519653/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar"
linker = "~/Android/Sdk/ndk/25.2.9519653/toolchains/llvm/prebuilt/linux-x86_64/bin/armv7a-linux-androideabi30-clang"
```

Adjust paths for your NDK location.

### 2. Add to AndroidManifest.xml

```xml
<uses-permission android:name="android.permission.MODIFY_AUDIO_SETTINGS" />
```

### 3. Build

```bash
cargo build --target aarch64-linux-android
```

## Hello World

```rust
use engine_audio::AudioEngine;
use glam::Vec3;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create audio engine
    let mut audio = AudioEngine::new()?;

    // Load a sound (from internal storage for testing)
    audio.load_sound("beep", "/sdcard/beep.wav")?;

    // Play it
    let instance_id = audio.play_2d("beep", 1.0, false)?;

    // Check if playing
    println!("Playing: {}", audio.is_playing(instance_id));

    // Wait for it to finish
    std::thread::sleep(std::time::Duration::from_secs(1));

    Ok(())
}
```

## 3D Audio Example

```rust
use engine_audio::AudioEngine;
use glam::Vec3;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut audio = AudioEngine::new()?;

    // Load footstep sound
    audio.load_sound("footstep", "/sdcard/footstep.wav")?;

    // Set listener (player camera)
    audio.set_listener_transform(
        Vec3::ZERO,      // Position
        Vec3::NEG_Z,     // Looking forward
        Vec3::Y,         // Up is +Y
    );

    // Play sound to the right of player
    let entity_id = 1;
    audio.play_3d(
        entity_id,
        "footstep",
        Vec3::new(10.0, 0.0, 0.0),  // 10 units to the right
        1.0,                         // Full volume
        false,                       // Don't loop
        50.0,                        // Max distance
    )?;

    // Simulate movement
    for i in 0..100 {
        let x = 10.0 - (i as f32 * 0.2); // Move left
        audio.update_emitter_position(entity_id, Vec3::new(x, 0.0, 0.0));
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    Ok(())
}
```

## Testing on Device

### Push Test Files

```bash
# Connect device
adb devices

# Push test audio files
adb push test_assets/beep.wav /sdcard/
adb push test_assets/footstep.wav /sdcard/
adb push test_assets/music.ogg /sdcard/
```

### Run Tests

```bash
# Build and push test binary
cargo test --target aarch64-linux-android --no-run

# Find the binary
find target/aarch64-linux-android/debug/deps/ -name "android_audio_test-*" -type f

# Push to device
adb push target/aarch64-linux-android/debug/deps/android_audio_test-HASH /data/local/tmp/

# Run on device
adb shell /data/local/tmp/android_audio_test-HASH --ignored
```

### Run Benchmarks

```bash
# Build benchmarks
cargo bench --target aarch64-linux-android --no-run

# Push to device
adb push target/aarch64-linux-android/release/deps/android_audio_benches-HASH /data/local/tmp/

# Run on device
adb shell /data/local/tmp/android_audio_benches-HASH

# With device_benchmarks feature (requires audio files)
cargo bench --target aarch64-linux-android --features device_benchmarks --no-run
adb push target/aarch64-linux-android/release/deps/android_audio_benches-HASH /data/local/tmp/
adb shell /data/local/tmp/android_audio_benches-HASH
```

## Common Issues

### "Permission denied" when playing audio

**Solution**: Add audio permissions to AndroidManifest.xml (see Setup section).

### "Failed to open audio stream"

**Solution**:
1. Check if another app is using audio
2. Restart device
3. Ensure headphones are properly connected

### Sounds not loading

**Solution**:
1. Verify file exists: `adb shell ls /sdcard/`
2. Check file permissions: `adb shell chmod 644 /sdcard/your_file.wav`
3. Ensure file format is supported (WAV, OGG, MP3)

### Crackling audio

**Solution**:
1. Reduce number of simultaneous sounds
2. Lower volume levels
3. Call `cleanup_finished()` regularly

## Loading from APK Assets

For production, load from APK assets instead of `/sdcard/`:

```rust
use ndk::asset::AssetManager;
use std::io::Read;

// Get AssetManager from Android (in your JNI code)
let asset_manager = // ... get from Java

// Open asset
let mut asset = asset_manager.open("sounds/footstep.wav")?;

// Read to buffer
let mut buffer = Vec::new();
asset.read_to_end(&mut buffer)?;

// Write to temporary file for decoding
let temp_path = "/data/local/tmp/footstep.wav";
std::fs::write(&temp_path, &buffer)?;

// Load into audio engine
audio.load_sound("footstep", temp_path)?;
```

## Performance Tips

1. **Preload during loading screens**:
   ```rust
   // Load all sounds before gameplay
   audio.load_sound("footstep", path)?;
   audio.load_sound("gunshot", path)?;
   audio.load_sound("explosion", path)?;
   ```

2. **Stream music, not sound effects**:
   ```rust
   // ✅ Good - stream background music
   audio.play_stream("music.ogg", 0.8, true)?;

   // ❌ Bad - don't stream short sounds
   audio.play_stream("beep.wav", 1.0, false)?;  // Just use play_2d!
   ```

3. **Clean up regularly**:
   ```rust
   // Every second or so
   if frame % 60 == 0 {
       audio.cleanup_finished();
   }
   ```

4. **Limit concurrent sounds**:
   ```rust
   if audio.active_sound_count() < 50 {
       audio.play_2d("gunshot", 1.0, false)?;
   }
   ```

## Next Steps

- Read [Android Audio Backend Documentation](android-audio-backend.md)
- Check [Audio System Overview](../engine/audio/CLAUDE.md)
- Review example games in `examples/`

## Support

For issues, check:
- [Oboe GitHub](https://github.com/google/oboe)
- [Android Audio Guide](https://developer.android.com/guide/topics/media/audio-app/overview)
- Engine documentation in `docs/`
