# Audio Demo Guide

## Overview

This singleplayer example demonstrates the complete audio system integration with the ECS. It showcases 2D audio, 3D spatial audio, Doppler effects, and audio effects (reverb).

## What This Demo Demonstrates

### 1. Audio System Initialization

```rust
let mut audio_system = AudioSystem::new()?;
```

The `AudioSystem` integrates with the ECS to automatically manage audio based on entity components.

### 2. Asset Loading

```rust
audio_system.load_sound("footstep", "assets/audio/footstep.wav")?;
audio_system.load_sound("ambient", "assets/audio/ambient.wav")?;
audio_system.load_sound("explosion", "assets/audio/explosion.wav")?;
audio_system.load_sound("music", "assets/audio/music.wav")?;
```

Sounds are loaded once and can be reused for multiple playback instances.

### 3. Audio Listener (Camera)

```rust
let camera = world.spawn();
world.add(camera, Transform { position: Vec3::new(0.0, 1.8, 0.0), .. });
world.add(camera, AudioListener::new());
```

The `AudioListener` component marks the entity as the "ears" of the player (usually attached to the camera). Only one listener should be active at a time.

### 4. 2D Non-Spatial Audio (Background Music)

```rust
let sound = Sound::new("music")
    .non_spatial()
    .with_volume(0.3)
    .looping()
    .auto_play();
```

Non-spatial audio plays at constant volume regardless of entity position. Perfect for:
- Background music
- UI sounds
- Menu audio

### 5. 3D Spatial Audio (Footsteps)

```rust
let sound = Sound::new("footstep")
    .spatial_3d(50.0)  // Max distance: 50 units
    .with_volume(0.8)
    .looping()
    .auto_play()
    .without_doppler();
```

3D spatial audio:
- Attenuates with distance (silent beyond `max_distance`)
- Pans left/right based on position relative to listener
- Updates automatically as entities move

### 6. Doppler Effect (Moving Sound Source)

```rust
let sound = Sound::new("ambient")
    .spatial_3d(100.0)
    .looping()
    .auto_play()
    .with_doppler(1.0);  // Doppler scale: 1.0 = realistic
```

The Doppler effect causes pitch shifts for moving sound sources:
- Approaching sound: higher pitch
- Receding sound: lower pitch
- `doppler_scale` controls intensity (0.0 = disabled, 1.0 = realistic, higher = exaggerated)

In this demo, the ambient sound moves from left (-20) to right (+20) over 5 seconds, demonstrating the Doppler effect.

### 7. Audio Effects (Reverb)

```rust
let reverb = ReverbEffect::large_hall();
audio_system.engine_mut().add_effect(instance_id, AudioEffect::Reverb(reverb))?;
```

Effects are applied to individual sound instances. The demo applies a large hall reverb to the explosion sound.

## Running the Demo

### Step 1: Generate Audio Assets

```bash
cd examples/singleplayer
cargo run --bin generate-audio-assets
```

This creates test audio files in `assets/audio/`:
- `footstep.wav` - Short 440Hz tone (0.1s)
- `ambient.wav` - Dual-tone hum (2s, looping)
- `explosion.wav` - White noise burst (0.3s)
- `music.wav` - Simple 4-note melody (5s, looping)

### Step 2: Run the Demo

```bash
cargo run --bin singleplayer
```

The demo runs for 5 seconds at 60 fps, demonstrating all audio features.

## What You Should Hear

1. **Background Music (2D)**
   - Starts immediately
   - Constant volume (30%)
   - 4-note melody (C-E-G-C) repeating

2. **Footsteps (3D Spatial)**
   - Positioned to the right (5 units)
   - Repeating 440Hz tone
   - Louder on right speaker
   - Attenuates with distance

3. **Moving Ambient Sound (3D + Doppler)**
   - Starts on the left
   - Moves to the right over 5 seconds
   - Pitch rises as it approaches (Doppler)
   - Pitch falls as it recedes

4. **Explosion (3D + Reverb)**
   - Positioned behind-right (-5, 0, 5)
   - White noise burst at start
   - Reverb creates echo effect (large hall)

## Verifying It's Working

### Console Output

You should see log output similar to:

```
INFO Singleplayer Example - Audio Demo Starting
INFO Audio system initialized
INFO Loading audio assets...
INFO All audio assets loaded successfully
INFO Loaded 4 sounds
INFO Created camera entity with AudioListener: Entity(0)
INFO Created background music entity: Entity(1)
INFO Created footstep emitter at Vec3(5.0, 0.0, 0.0), entity: Entity(2)
INFO Created moving sound source entity: Entity(3)
INFO Created explosion emitter at Vec3(-5.0, 0.0, 5.0), entity: Entity(4)
INFO Created 5 entities
INFO Starting game loop (5s at 60 fps)
INFO Frame 0/300 - Active sounds: 4 - Elapsed: 0.0s
INFO Applied reverb effect to explosion
INFO Frame 60/300 - Active sounds: 4 - Elapsed: 1.0s
INFO Frame 120/300 - Active sounds: 3 - Elapsed: 2.0s
INFO Frame 180/300 - Active sounds: 3 - Elapsed: 3.0s
INFO Frame 240/300 - Active sounds: 3 - Elapsed: 4.0s
INFO Game loop complete
INFO Singleplayer Example - Audio Demo Complete
```

### Active Sound Count

- **Frame 0**: 4 sounds (music, footsteps, ambient, explosion)
- **Frame 60+**: 3 sounds (music, footsteps, ambient) - explosion finished
- The count should decrease as one-shot sounds complete

## Performance Metrics

This demo validates the audio system meets performance targets:

| Metric | Target | Measured |
|--------|--------|----------|
| Frame time | < 16.67ms (60 fps) | Check with profiling |
| Active sounds | 256+ supported | Demo uses 4 |
| Audio latency | < 5ms | Platform-dependent |
| Memory usage | < 50MB for audio | Check system monitor |

## Troubleshooting

### No Sound Output

1. **Check audio device**: Ensure speakers/headphones are connected
2. **Check volume**: System volume should be > 0
3. **Check platform**: Desktop audio (Kira) should work on Windows/Linux/macOS
4. **Check logs**: Look for audio initialization errors

### Assets Not Found

```
Error: Audio assets not found!
Please run: cargo run --bin generate-audio-assets
```

**Solution**: Run the asset generator first (see Step 1 above)

### Audio Initialization Failed

```
Error: Failed to initialize audio system: ...
```

**Possible causes**:
- No audio device available
- Audio driver issues
- Platform not supported (WASM requires different build)

**Solution**: Check platform-specific audio backend documentation

### Crackling or Distortion

**Possible causes**:
- Buffer underruns (frame rate too low)
- CPU overload
- Audio driver issues

**Solution**:
- Reduce number of active sounds
- Increase audio buffer size
- Update audio drivers

### No Doppler Effect

**Verify**:
1. Sound has `doppler_enabled = true`
2. Sound has `spatial = true`
3. Entity is actually moving (check logs for position updates)
4. Movement is fast enough (> 10 units/sec recommended)

## Extending the Demo

### Add More Sounds

```rust
// 1. Generate new asset
// (Add to generate_audio_assets.rs)

// 2. Load in main.rs
audio_system.load_sound("jump", "assets/audio/jump.wav")?;

// 3. Create entity with sound
let entity = world.spawn();
world.add(entity, Transform::default());
world.add(entity, Sound::new("jump").spatial_3d(30.0).auto_play());
```

### Interactive Playback

```rust
// Trigger sound manually
audio_system.play_sound(entity, &mut world)?;

// Stop sound
audio_system.stop_sound(entity, &mut world, Some(0.5)); // 0.5s fade out
```

### Multiple Listeners

```rust
// Deactivate current listener
if let Some(listener) = world.get_mut::<AudioListener>(camera1) {
    listener.active = false;
}

// Activate new listener
if let Some(listener) = world.get_mut::<AudioListener>(camera2) {
    listener.active = true;
}
```

## Next Steps

After exploring this audio demo:

1. **Read the audio architecture docs**: `docs/audio.md`
2. **Explore audio effects**: Try different reverb presets, add echo/EQ
3. **Integrate with gameplay**: Add sounds to player actions, enemy AI, etc.
4. **Profile audio performance**: Use Tracy profiler to measure overhead
5. **Test on different platforms**: WASM, Android, iOS (when available)

## References

- **Audio System API**: `engine/audio/src/system.rs`
- **Audio Components**: `engine/audio/src/components.rs`
- **Doppler Effect**: `engine/audio/src/doppler.rs`
- **Audio Effects**: `engine/audio/src/effects.rs`
- **Platform Backends**: `engine/audio/src/platform/`
