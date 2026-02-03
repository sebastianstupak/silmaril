# Web Audio Backend - Quick Start Guide

## What is This?

The Web Audio backend enables the engine-audio crate to run in web browsers using WASM. It provides the same API as the desktop (Kira) backend, allowing your game to run both natively and in the browser without code changes.

## Features

✅ 2D audio playback (UI sounds, menu effects)
✅ 3D spatial audio with HRTF (realistic positioning)
✅ Audio streaming (background music, large files)
✅ Volume control and fade in/out
✅ Distance-based attenuation
✅ Listener orientation (camera-based)
✅ Full cross-platform API compatibility

## Quick Example

```rust
use engine_audio::AudioEngine;
use glam::Vec3;

// Create audio engine (Web Audio on WASM, Kira on desktop)
let mut audio = AudioEngine::new()?;

// Load sound
audio.load_sound("footstep", "assets/footstep.wav")?;

// Play 2D sound (UI, menu)
let ui_sound = audio.play_2d("footstep", 1.0, false)?;

// Play 3D sound (in-game)
let spatial_sound = audio.play_3d(
    entity_id,
    "footstep",
    Vec3::new(5.0, 0.0, 0.0), // Position
    1.0,   // Volume
    false, // Looping
    50.0,  // Max distance
)?;

// Update listener (every frame)
audio.set_listener_transform(
    camera_pos,
    camera_forward,
    camera_up,
);

// Stream music (no pre-loading)
let music = audio.play_stream("assets/music.ogg", 0.5, true)?;

// Stop with 2-second fade
audio.stop(music, Some(2.0));
```

## Building for WASM

### 1. Install wasm-pack

```bash
cargo install wasm-pack
```

### 2. Build for Web

```bash
# Use the provided script
./engine/audio/build-wasm.sh

# Or manually
wasm-pack build --target web engine/audio
```

### 3. Use in HTML

```html
<!DOCTYPE html>
<html>
<head>
    <title>My Game</title>
</head>
<body>
    <script type="module">
        import init, { AudioEngine } from './pkg/engine_audio.js';

        await init();

        // Create audio engine after user interaction
        document.getElementById('start').addEventListener('click', async () => {
            const audio = AudioEngine.new();
            // Use audio...
        });
    </script>

    <button id="start">Start Game</button>
</body>
</html>
```

## Important: Browser Autoplay Policy

Modern browsers require user interaction before playing audio. Always create the AudioContext after a user click/tap:

```rust
// ❌ BAD - May be blocked by browser
let audio = AudioEngine::new()?; // Created on page load
audio.play_2d("sound", 1.0, false)?; // Will fail!

// ✅ GOOD - Created after user interaction
// HTML: <button id="start">Start</button>
// Then in WASM (after button click):
let audio = AudioEngine::new()?; // Now allowed
audio.play_2d("sound", 1.0, false)?; // Works!
```

## Supported Audio Formats

| Format | Chrome | Firefox | Safari | Recommendation |
|--------|--------|---------|--------|----------------|
| WAV | ✅ | ✅ | ✅ | Short sounds |
| OGG | ✅ | ✅ | ❌ | Best compression |
| MP3 | ✅ | ✅ | ✅ | Universal |
| AAC | ✅ | ✅ | ✅ | Safari preferred |

**Best Practice**: Use OGG for Firefox/Chrome, AAC/MP3 for Safari.

## Testing

### Run Unit Tests

```bash
cargo test --package engine-audio --lib web_audio_mock_test
```

### Run Browser Tests

```bash
# Firefox
wasm-pack test --headless --firefox engine/audio

# Chrome
wasm-pack test --headless --chrome engine/audio
```

### Interactive Test Page

```bash
# Build and serve
./engine/audio/build-wasm.sh
python -m http.server 8000

# Open browser
# Navigate to: http://localhost:8000/engine/audio/test-web-audio.html
```

## Common Issues

### "Failed to create AudioContext"

**Cause**: Browser blocked autoplay.

**Solution**: Create AudioEngine after user interaction (click/tap).

### "Sound not playing"

**Causes**:
1. File not found (check URL path)
2. Format not supported (use WAV/MP3)
3. Autoplay blocked (see above)

**Debug**:
```rust
// Enable logging
RUST_LOG=engine_audio=debug

// Check result
let result = audio.play_2d("sound", 1.0, false);
if let Err(e) = result {
    tracing::error!("Failed to play: {:?}", e);
}
```

### "3D audio not working"

**Causes**:
1. Listener not set (call `set_listener_transform()`)
2. Emitter not positioned (call `update_emitter_position()`)
3. Max distance too small (increase value)

**Debug**:
```rust
// Log positions
tracing::debug!("Listener: {:?}", listener_pos);
tracing::debug!("Emitter: {:?}", emitter_pos);
tracing::debug!("Distance: {:?}", listener_pos.distance(emitter_pos));
```

## Performance Tips

1. **Lazy Loading**: Load sounds on-demand, not at startup
2. **Audio Sprites**: Combine small sounds into one file
3. **Streaming**: Use for files > 1MB (music, ambience)
4. **Cleanup**: Call `cleanup_finished()` every 500ms
5. **Limit Sounds**: Keep active sounds < 100 for best performance

## Example: Complete Game Integration

```rust
use engine_audio::AudioEngine;
use engine_core::ecs::World;
use glam::Vec3;

struct Game {
    world: World,
    audio: AudioEngine,
}

impl Game {
    fn new() -> Result<Self, AudioError> {
        let mut audio = AudioEngine::new()?;

        // Load all game sounds
        audio.load_sound("footstep", "assets/footstep.wav")?;
        audio.load_sound("gunshot", "assets/gunshot.wav")?;
        audio.load_sound("explosion", "assets/explosion.wav")?;

        // Start background music
        audio.play_stream("assets/music.ogg", 0.5, true)?;

        Ok(Self {
            world: World::new(),
            audio,
        })
    }

    fn update(&mut self, dt: f32) {
        // Update listener from camera
        if let Some(camera) = self.world.get_camera() {
            self.audio.set_listener_transform(
                camera.position,
                camera.forward,
                camera.up,
            );
        }

        // Update emitter positions
        for (entity, position) in self.world.query::<&Position>() {
            self.audio.update_emitter_position(entity.id(), position.0);
        }

        // Cleanup finished sounds every 30 frames (~500ms at 60 FPS)
        if self.world.frame_count() % 30 == 0 {
            self.audio.cleanup_finished();
        }
    }

    fn on_event(&mut self, event: GameEvent) {
        match event {
            GameEvent::PlayerFootstep { entity, position } => {
                self.audio.play_3d(
                    entity,
                    "footstep",
                    position,
                    0.8,   // Volume
                    false, // Looping
                    20.0,  // Max distance
                ).ok();
            }
            GameEvent::Explosion { position } => {
                // Explosion doesn't need entity tracking
                self.audio.play_3d(
                    u32::MAX, // Temp entity ID
                    "explosion",
                    position,
                    1.0,   // Full volume
                    false,
                    100.0, // Large radius
                ).ok();
            }
        }
    }
}
```

## Next Steps

1. **Read Full Documentation**: `docs/web-audio-backend.md`
2. **Try Examples**: Run the interactive test page
3. **Integrate**: Add audio to your game
4. **Test**: Verify in different browsers
5. **Optimize**: Follow performance tips

## Support

- **Documentation**: `docs/web-audio-backend.md`
- **Examples**: `engine/audio/test-web-audio.html`
- **Tests**: `engine/audio/tests/web_audio_*.rs`
- **Source**: `engine/audio/src/platform/web.rs`

## Browser DevTools

Use Chrome DevTools to debug Web Audio:

1. Open DevTools (F12)
2. Go to "More Tools" → "WebAudio"
3. See real-time audio graph visualization
4. Monitor node connections and parameters

Happy coding! 🎵
