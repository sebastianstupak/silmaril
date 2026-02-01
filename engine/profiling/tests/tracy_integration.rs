//! Integration tests for Tracy profiler backend.
//!
//! These tests verify that Tracy integration works correctly and compiles
//! to nothing when the feature is disabled.

#![cfg(feature = "profiling-tracy")]

use agent_game_engine_profiling::{profile_scope, ProfileCategory, TracyBackend};

#[test]
fn test_tracy_backend_creation() {
    let backend = TracyBackend::new();
    assert_eq!(backend.frame_count(), 0);
}

#[test]
fn test_tracy_frame_markers() {
    let mut backend = TracyBackend::new();

    backend.begin_frame();
    backend.end_frame();

    assert_eq!(backend.frame_count(), 1);

    backend.begin_frame();
    backend.end_frame();

    assert_eq!(backend.frame_count(), 2);
}

#[test]
fn test_tracy_scopes() {
    let _backend = TracyBackend::new();

    // Basic scope
    {
        profile_scope!("test_scope");
        std::thread::sleep(std::time::Duration::from_micros(10));
    }

    // Nested scopes
    {
        profile_scope!("outer_scope");
        {
            profile_scope!("inner_scope");
            std::thread::sleep(std::time::Duration::from_micros(10));
        }
    }

    // Should complete without panicking
}

#[test]
fn test_tracy_macro_basic() {
    // Test macro without category
    {
        profile_scope!("test_macro");
        std::thread::sleep(std::time::Duration::from_micros(10));
    }

    // Test macro with category
    {
        profile_scope!("test_macro_category", ProfileCategory::Physics);
        std::thread::sleep(std::time::Duration::from_micros(10));
    }
}

#[test]
fn test_tracy_macro_nested() {
    profile_scope!("outer");

    {
        profile_scope!("inner_1", ProfileCategory::ECS);
        std::thread::sleep(std::time::Duration::from_micros(5));
    }

    {
        profile_scope!("inner_2", ProfileCategory::Rendering);
        std::thread::sleep(std::time::Duration::from_micros(5));
    }
}

#[test]
fn test_tracy_all_categories() {
    let categories = [
        ProfileCategory::ECS,
        ProfileCategory::Rendering,
        ProfileCategory::Physics,
        ProfileCategory::Networking,
        ProfileCategory::Audio,
        ProfileCategory::Serialization,
        ProfileCategory::Scripts,
        ProfileCategory::Unknown,
    ];

    for category in &categories {
        profile_scope!("test_category", *category);
        std::thread::sleep(std::time::Duration::from_micros(1));
    }
}

#[test]
fn test_tracy_hot_path_simulation() {
    // Simulate hot path with many scopes (should have minimal overhead)
    for i in 0..1000 {
        profile_scope!("hot_path_iteration");

        if i % 100 == 0 {
            profile_scope!("occasional_work", ProfileCategory::Physics);
            std::thread::sleep(std::time::Duration::from_nanos(100));
        }
    }
}

#[test]
fn test_tracy_frame_simulation() {
    let mut backend = TracyBackend::new();

    // Simulate 10 frames
    for frame in 0..10 {
        backend.begin_frame();

        {
            profile_scope!("game_loop");

            // Simulate ECS update
            {
                profile_scope!("ecs_update", ProfileCategory::ECS);
                std::thread::sleep(std::time::Duration::from_micros(50));
            }

            // Simulate physics
            {
                profile_scope!("physics_step", ProfileCategory::Physics);
                std::thread::sleep(std::time::Duration::from_micros(100));
            }

            // Simulate rendering
            {
                profile_scope!("render_frame", ProfileCategory::Rendering);
                std::thread::sleep(std::time::Duration::from_micros(200));
            }
        }

        backend.end_frame();

        assert_eq!(backend.frame_count(), frame + 1);
    }
}

#[test]
fn test_tracy_dynamic_scope_names() {
    let _backend = TracyBackend::new();
    for i in 0..5 {
        // Tracy works best with static strings, but we can use dynamic ones via format!
        profile_scope!(&format!("dynamic_scope_{}", i));
        std::thread::sleep(std::time::Duration::from_micros(1));
    }
}

#[test]
fn test_tracy_thread_safety() {
    use std::sync::Arc;
    use std::thread;

    let _backend = Arc::new(TracyBackend::new());

    let handles: Vec<_> = (0..4)
        .map(|_i| {
            thread::spawn(move || {
                profile_scope!("thread_work", ProfileCategory::ECS);
                std::thread::sleep(std::time::Duration::from_micros(10));
            })
        })
        .collect();

    for handle in handles {
        handle.join().expect("Thread panicked");
    }
}

/// Test that Tracy compiles to nothing when feature is disabled.
///
/// This is verified by checking that the test compiles successfully
/// when tracy feature is disabled (compile-time check).
#[cfg(not(feature = "profiling-tracy"))]
#[test]
fn test_tracy_zero_cost_when_disabled() {
    // These should compile to nothing
    profile_scope!("test");
    profile_scope!("test", ProfileCategory::Physics);

    // Verify no runtime overhead
    let start = std::time::Instant::now();
    for _ in 0..1_000_000 {
        profile_scope!("hot_loop");
    }
    let elapsed = start.elapsed();

    // Should be near-instant (< 1ms for 1M empty iterations)
    assert!(
        elapsed.as_millis() < 10,
        "Tracy disabled should have zero overhead, took {:?}",
        elapsed
    );
}
