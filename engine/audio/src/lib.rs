//! Engine Audio
//!
//! Provides cross-platform 3D spatial audio:
//! - **Desktop (Windows, Linux, macOS)**: Kira audio engine
//! - **Web (WASM)**: Web Audio API
//! - **Android**: OpenSL ES / AAudio
//! - **iOS**: Core Audio
//!
//! # Features
//!
//! - 2D and 3D audio playback
//! - Spatial audio with distance attenuation
//! - Sound and AudioListener ECS components
//! - Efficient audio streaming for music
//! - Platform-specific optimizations
//!
//! # Example
//!
//! ```rust,no_run
//! use engine_audio::{AudioEngine, Sound, AudioListener};
//! use glam::Vec3;
//!
//! // Create audio engine (platform-specific backend selected automatically)
//! let mut audio = AudioEngine::new().unwrap();
//!
//! // Load sound
//! audio.load_sound("footstep", "assets/footstep.wav").unwrap();
//!
//! // Play 3D sound
//! let instance = audio.play_3d(
//!     1,  // entity ID
//!     "footstep",
//!     Vec3::new(5.0, 0.0, 0.0),
//!     1.0,  // volume
//!     false, // looping
//!     50.0,  // max distance
//! ).unwrap();
//!
//! // Update listener position (camera)
//! audio.set_listener_transform(
//!     Vec3::new(0.0, 1.8, 0.0),  // position
//!     Vec3::new(0.0, 0.0, -1.0), // forward
//!     Vec3::new(0.0, 1.0, 0.0),  // up
//! );
//!
//! // Stream background music
//! let music = audio.play_stream(
//!     "assets/music.ogg",
//!     0.5,  // volume
//!     true, // loop
//! ).unwrap();
//! ```
//!
//! # Platform-Specific Notes
//!
//! ## Web (WASM)
//!
//! The Web Audio backend uses the browser's Web Audio API:
//! - Requires user interaction before first playback (autoplay policy)
//! - Supports OGG, MP3, WAV, AAC formats (browser-dependent)
//! - HRTF-based 3D audio positioning
//! - See [docs/web-audio-backend.md](../../docs/web-audio-backend.md) for details
//!
//! ## Desktop
//!
//! The Kira backend provides high-quality audio on Windows, Linux, and macOS:
//! - Low-latency playback
//! - Advanced spatial audio
//! - Full format support (OGG, MP3, WAV, FLAC)
//!
//! ## Mobile
//!
//! Native audio backends for Android and iOS:
//! - Optimized for battery life
//! - Hardware acceleration where available
//! - Format support varies by platform

#![warn(missing_docs)]

mod components;
mod diagnostics;
mod doppler;
mod effects;
mod engine;
mod error;
mod event_logger;
mod platform;
pub mod simd_batch;
mod system;

pub use components::{AudioListener, Sound};
pub use diagnostics::{AudioDiagnostics, AudioPerformanceMetrics, SoundState};
pub use doppler::{DopplerCalculator, DEFAULT_SPEED_OF_SOUND};
pub use effects::{AudioEffect, EchoEffect, EqEffect, FilterEffect, FilterType, ReverbEffect};
pub use engine::AudioEngine;
pub use error::{AudioError, AudioResult};
pub use event_logger::{AudioEvent, AudioEventLogger, AudioEventType, EventFilter};
pub use system::AudioSystem;

// Re-export platform abstraction for advanced users
pub use platform::{create_audio_backend, AudioBackend};
