# Engine Audio

## Purpose
The audio crate provides 3D spatial audio:
- **Audio Playback**: Support for OGG, WAV, and MP3 formats
- **3D Spatialization**: HRTF-based 3D audio positioning
- **Audio Mixing**: Real-time mixing with support for hundreds of sources
- **Effects**: Reverb, echo, and other audio effects
- **Streaming**: Efficient streaming for music and ambient sounds

## MUST READ Documentation
Before working on this crate, read these documents in order:

1. **[phase3-audio.md](../../docs/phase3-audio.md)** - Audio system design and 3D spatialization

## Related Crates
- **engine-core**: Queries ECS for audio source positions
- **engine-networking**: Can sync audio events for multiplayer

## Quick Example
```rust
use engine_audio::{AudioEngine, Sound};

fn play_sound(audio: &mut AudioEngine, world: &World) {
    let sound = Sound::load("gunshot.ogg")?;

    // Play 3D positioned sound
    for (pos, audio_source) in world.query::<(&Position, &AudioSource)>() {
        audio.play_3d(sound, pos.into());
    }
}
```

## Key Dependencies
- `kira` - Audio playback and mixing
- `engine-core` - ECS integration

## Performance Targets
- 256+ simultaneous audio sources
- <5ms latency for sound playback
- HRTF-based 3D audio positioning
