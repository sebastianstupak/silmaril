//! Engine Audio
//!
//! Provides 3D spatial audio:
//! - Audio playback (OGG, WAV, MP3)
//! - 3D spatialization with HRTF
//! - Audio mixing and effects
//! - Streaming for music

#![warn(missing_docs)]

pub mod engine;
pub mod sound;
pub mod spatial;
pub mod effects;

// Re-export commonly used types
pub use engine::AudioEngine;
pub use sound::Sound;
