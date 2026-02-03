# Singleplayer Example - Audio Demo

A demonstration of the audio system integrated with the ECS in a singleplayer context.

## Quick Start

### 1. Generate Audio Assets

```bash
cargo run --release --bin generate-audio-assets
```

This creates test audio files in `assets/audio/`:
- `footstep.wav` - Short 440Hz tone (0.1s)
- `ambient.wav` - Dual-tone hum (2s, looping)
- `explosion.wav` - White noise burst (0.3s)
- `music.wav` - Simple 4-note melody (5s, looping)

### 2. Run the Demo

```bash
cargo run --release --bin singleplayer
```

The demo runs for 5 seconds at 60 fps, demonstrating:
- **2D non-spatial audio** (background music)
- **3D spatial audio** (footsteps at entity positions)
- **Doppler effect** (moving sound source)
- **Audio effects** (reverb on explosion)

### 3. Run Tests

```bash
cargo test --release
```

The E2E test validates:
- Audio system integration
- Sound playback
- Entity position updates
- Performance targets (< 17ms frame time, < 5ms audio update)

## What You Should Hear

1. **Background Music** - Constant volume, 4-note melody (C-E-G-C) repeating
2. **Footsteps** - Positioned to the right, louder on right speaker
3. **Moving Sound** - Starts left, moves right, pitch rises (Doppler)
4. **Explosion** - Short burst with reverb echo

## Documentation

See [AUDIO_GUIDE.md](AUDIO_GUIDE.md) for detailed documentation:
- How the audio system works
- Component API reference
- Performance metrics
- Troubleshooting guide
- Examples of extending the demo

## Architecture

```
examples/singleplayer/
├── src/
│   ├── main.rs                           # Audio demo
│   └── bin/
│       └── generate_audio_assets.rs      # Asset generator
├── tests/
│   └── audio_e2e_test.rs                 # E2E integration test
├── assets/
│   └── audio/                            # Generated audio files
├── AUDIO_GUIDE.md                        # Detailed documentation
├── CLAUDE.md                             # Development guide
└── README.md                             # This file
```

## Performance

The demo validates performance targets:
- **Frame time**: < 16.67ms (60 fps)
- **Audio update**: < 5ms per frame
- **Active sounds**: 4 concurrent sounds
- **Memory**: < 50MB total

## Next Steps

- Read [AUDIO_GUIDE.md](AUDIO_GUIDE.md) for API examples
- Explore [engine/audio/](../../engine/audio/) source code
- Check [docs/audio.md](../../docs/audio.md) for architecture
- Try adding your own sounds and effects
