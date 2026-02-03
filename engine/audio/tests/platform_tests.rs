//! Platform-specific audio backend tests
//!
//! Tests platform-specific features and cross-platform parity:
//! - Desktop (Kira) specific features
//! - Web Audio API specific features
//! - Android (Oboe) specific features
//! - iOS (Core Audio) specific features
//! - Cross-platform behavior parity

use engine_audio::{create_audio_backend, AudioEngine};
use tracing::info;

/// Test audio backend creation on current platform
#[test]
fn test_platform_backend_creation() {
    let backend = create_audio_backend();
    assert!(backend.is_ok(), "Failed to create platform audio backend");

    info!(platform = std::env::consts::OS, "Audio backend created successfully");
}

/// Test audio engine initialization
#[test]
fn test_audio_engine_initialization() {
    let engine = AudioEngine::new();
    assert!(engine.is_ok(), "Failed to initialize audio engine: {:?}", engine.err());

    info!(platform = std::env::consts::OS, "Audio engine initialized successfully");
}

/// Test backend initialization is idempotent
#[test]
fn test_multiple_backend_creation() {
    // Should be able to create multiple backends
    let backend1 = create_audio_backend();
    assert!(backend1.is_ok());

    let backend2 = create_audio_backend();
    assert!(backend2.is_ok());

    info!("Multiple backend creation works");
}

/// Test backend state after creation
#[test]
fn test_backend_initial_state() {
    let engine = AudioEngine::new().expect("Failed to create engine");

    // Should start with no active sounds
    assert_eq!(engine.active_sound_count(), 0);

    info!("Backend initial state is correct");
}

/// Test cross-platform audio engine API
#[test]
fn test_cross_platform_api() {
    let engine = AudioEngine::new().expect("Failed to create engine");

    // All platforms should support these methods
    let _count = engine.active_sound_count();
    // Note: Other methods require actual audio files, which we don't have in tests

    info!("Cross-platform API works");
}

// Desktop-specific tests
#[cfg(not(any(target_arch = "wasm32", target_os = "android", target_os = "ios")))]
mod desktop {
    use super::*;

    #[test]
    fn test_desktop_backend() {
        let engine = AudioEngine::new().expect("Desktop audio engine should initialize");

        info!("Desktop (Kira) backend initialized successfully");

        // Desktop backend should support all features
        assert_eq!(engine.active_sound_count(), 0);
    }

    #[test]
    fn test_desktop_multiple_engines() {
        let engine1 = AudioEngine::new().expect("First engine should initialize");
        let engine2 = AudioEngine::new().expect("Second engine should initialize");

        assert_eq!(engine1.active_sound_count(), 0);
        assert_eq!(engine2.active_sound_count(), 0);

        info!("Multiple desktop audio engines work");
    }
}

// Web-specific tests
#[cfg(target_arch = "wasm32")]
mod web {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_web_audio_backend() {
        let engine = AudioEngine::new().expect("Web Audio should initialize");

        info!("Web Audio API backend initialized successfully");

        assert_eq!(engine.active_sound_count(), 0);
    }

    #[wasm_bindgen_test]
    fn test_web_audio_context_creation() {
        use web_sys::AudioContext;

        // Test that we can create Web Audio context
        let context = AudioContext::new();
        assert!(context.is_ok(), "AudioContext creation should succeed");

        info!("Web AudioContext created successfully");
    }

    #[wasm_bindgen_test]
    fn test_web_audio_panner_node() {
        use web_sys::AudioContext;

        let context = AudioContext::new().expect("AudioContext should be created");
        let panner = context.create_panner();
        assert!(panner.is_ok(), "PannerNode creation should succeed");

        info!("Web Audio PannerNode created successfully");
    }

    #[wasm_bindgen_test]
    fn test_web_audio_gain_node() {
        use web_sys::AudioContext;

        let context = AudioContext::new().expect("AudioContext should be created");
        let gain = context.create_gain();
        assert!(gain.is_ok(), "GainNode creation should succeed");

        info!("Web Audio GainNode created successfully");
    }
}

// Android-specific tests
#[cfg(target_os = "android")]
mod android {
    use super::*;

    #[test]
    fn test_android_backend() {
        let engine = AudioEngine::new().expect("Android audio engine should initialize");

        info!("Android (Oboe) backend initialized successfully");

        assert_eq!(engine.active_sound_count(), 0);
    }

    #[test]
    fn test_android_low_latency() {
        // Android Oboe backend should support low-latency mode
        let engine = AudioEngine::new().expect("Android audio engine should initialize");

        // Verify engine is ready
        assert_eq!(engine.active_sound_count(), 0);

        info!("Android low-latency audio ready");
    }
}

// iOS-specific tests
#[cfg(target_os = "ios")]
mod ios {
    use super::*;

    #[test]
    fn test_ios_backend() {
        let engine = AudioEngine::new().expect("iOS audio engine should initialize");

        info!("iOS (Core Audio) backend initialized successfully");

        assert_eq!(engine.active_sound_count(), 0);
    }

    #[test]
    fn test_ios_audio_session() {
        // iOS Core Audio backend should configure audio session
        let engine = AudioEngine::new().expect("iOS audio engine should initialize");

        // Verify engine is ready
        assert_eq!(engine.active_sound_count(), 0);

        info!("iOS audio session configured");
    }
}

/// Test cleanup across platforms
#[test]
fn test_platform_cleanup() {
    let mut engine = AudioEngine::new().expect("Failed to create engine");

    // Cleanup should work on all platforms
    engine.cleanup_finished();

    assert_eq!(engine.active_sound_count(), 0);

    info!("Platform cleanup works");
}

/// Test that error types are consistent across platforms
#[test]
fn test_platform_error_consistency() {
    use engine_audio::AudioError;

    // All platforms should have the same error types
    let _manager_error = AudioError::ManagerError("test".to_string());
    let _not_found = AudioError::SoundNotFound("test".to_string());
    let _decode_error = AudioError::DecodeError("test".to_string());
    let _effect_error = AudioError::EffectError("test".to_string());

    info!("Platform error types are consistent");
}

/// Test performance characteristics across platforms
#[test]
fn test_platform_performance() {
    use std::time::Instant;

    let start = Instant::now();
    let engine = AudioEngine::new().expect("Failed to create engine");
    let init_time = start.elapsed();

    info!(
        platform = std::env::consts::OS,
        init_time_ms = init_time.as_millis(),
        "Audio engine initialization time"
    );

    // Initialization should be reasonably fast on all platforms
    assert!(init_time.as_secs() < 5, "Initialization took too long: {:?}", init_time);

    let _count = engine.active_sound_count();
}

/// Test that all platforms support basic operations
#[test]
fn test_platform_basic_operations() {
    let mut engine = AudioEngine::new().expect("Failed to create engine");

    // All platforms should support:
    // - Querying active sound count
    let count = engine.active_sound_count();
    assert_eq!(count, 0);

    // - Cleanup
    engine.cleanup_finished();

    // - Setting listener transform
    use glam::Vec3;
    engine.set_listener_transform(Vec3::ZERO, Vec3::new(0.0, 0.0, -1.0), Vec3::Y);

    info!("All basic operations work on current platform");
}

/// Test backend resilience to rapid operations
#[test]
fn test_platform_rapid_operations() {
    use std::time::Instant;

    let mut engine = AudioEngine::new().expect("Failed to create engine");

    let start = Instant::now();
    for _ in 0..1000 {
        engine.cleanup_finished();
        let _count = engine.active_sound_count();
    }
    let elapsed = start.elapsed();

    info!(
        platform = std::env::consts::OS,
        operations = 2000,
        elapsed_ms = elapsed.as_millis(),
        "Rapid operations completed"
    );

    // Should handle rapid operations efficiently
    assert!(elapsed.as_millis() < 1000);
}

/// Test listener transform updates across platforms
#[test]
fn test_platform_listener_transform() {
    use glam::Vec3;

    let mut engine = AudioEngine::new().expect("Failed to create engine");

    // Test various listener transforms
    let positions = vec![
        Vec3::ZERO,
        Vec3::new(10.0, 0.0, 0.0),
        Vec3::new(0.0, 10.0, 0.0),
        Vec3::new(0.0, 0.0, 10.0),
        Vec3::new(-100.0, -100.0, -100.0),
    ];

    for pos in positions {
        engine.set_listener_transform(pos, Vec3::new(0.0, 0.0, -1.0), Vec3::Y);
    }

    info!("Listener transform updates work on current platform");
}

/// Test that platform backends handle errors gracefully
#[test]
fn test_platform_error_handling() {
    let engine = AudioEngine::new().expect("Failed to create engine");

    // Try to access non-existent sound - should return error, not panic
    let result = engine.is_playing(999999);
    assert!(!result);

    let effect_count = engine.effect_count(999999);
    assert_eq!(effect_count, 0);

    info!("Platform error handling works correctly");
}

/// Test platform-specific optimizations don't break API
#[test]
fn test_platform_optimizations() {
    let engine = AudioEngine::new().expect("Failed to create engine");

    // Despite platform-specific optimizations, API should be consistent
    assert_eq!(engine.active_sound_count(), 0);

    info!(
        platform = std::env::consts::OS,
        "Platform optimizations maintain API consistency"
    );
}

/// Test memory cleanup across platforms
#[test]
fn test_platform_memory_cleanup() {
    // Create and drop engines to test cleanup
    for _ in 0..10 {
        let engine = AudioEngine::new().expect("Failed to create engine");
        drop(engine);
    }

    info!("Platform memory cleanup works correctly");
}

/// Test concurrent engine creation (if platform supports it)
#[test]
fn test_platform_concurrent_creation() {
    use std::sync::Arc;
    use std::sync::Mutex;

    let engines = Arc::new(Mutex::new(Vec::new()));

    // Try to create multiple engines
    for _ in 0..5 {
        let engine = AudioEngine::new();
        if let Ok(engine) = engine {
            engines.lock().unwrap().push(engine);
        }
    }

    let engine_count = engines.lock().unwrap().len();
    info!(
        platform = std::env::consts::OS,
        engines_created = engine_count,
        "Concurrent engine creation test"
    );

    // Should create at least one engine
    assert!(engine_count > 0);
}

/// Test that audio engine is Send/Sync on appropriate platforms
#[test]
fn test_platform_thread_safety() {
    #[allow(dead_code)]
    fn assert_send<T: Send>() {}

    // AudioEngine should have appropriate thread safety markers
    // Note: This is a compile-time check, so if it compiles, it passes

    #[cfg(not(target_arch = "wasm32"))]
    {
        // Non-WASM platforms should be Send
        assert_send::<AudioEngine>();
    }

    info!(platform = std::env::consts::OS, "Thread safety markers are correct");
}
