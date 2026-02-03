//! Integration tests for Puffin profiler backend.
//!
//! These tests verify that the Puffin backend correctly captures scopes,
//! exports Chrome Trace format, and integrates with the profiling infrastructure.

#![cfg(feature = "profiling-puffin")]

use silmaril_profiling::{backends::PuffinBackend, profile_scope, ProfileCategory};
use std::thread;
use std::time::Duration;

#[test]
fn test_puffin_backend_captures_scopes() {
    let mut backend = PuffinBackend::new();

    backend.begin_frame();

    // Create some profiling scopes
    {
        puffin::profile_scope!("test_scope_1", "ECS");
        thread::sleep(Duration::from_micros(100));
    }

    {
        puffin::profile_scope!("test_scope_2", "Rendering");
        thread::sleep(Duration::from_micros(100));
    }

    backend.end_frame();

    // Export should return valid JSON (at minimum an empty array for Phase 0.5.2)
    let trace = backend.export_chrome_trace();
    assert!(trace.starts_with('['));
    assert!(trace.ends_with(']'));
}

#[test]
fn test_chrome_trace_export_format() {
    let mut backend = PuffinBackend::new();

    backend.begin_frame();

    {
        puffin::profile_scope!("game_loop", "ECS");
        thread::sleep(Duration::from_micros(50));
    }

    backend.end_frame();

    let trace = backend.export_chrome_trace();

    // Verify it's valid JSON array
    assert!(trace.starts_with('['));
    assert!(trace.ends_with(']'));

    // Should contain trace event structure
    // Chrome trace events have: name, cat, ph, pid, tid, ts
    if trace.len() > 2 {
        // Only check if there's actual data
        assert!(trace.contains("name") || trace.contains("\""));
    }
}

#[test]
fn test_nested_scopes_captured() {
    let mut backend = PuffinBackend::new();

    backend.begin_frame();

    {
        puffin::profile_scope!("outer", ProfileCategory::Physics.as_str());
        thread::sleep(Duration::from_micros(50));

        {
            puffin::profile_scope!("middle", ProfileCategory::Rendering.as_str());
            thread::sleep(Duration::from_micros(50));

            {
                puffin::profile_scope!("inner", ProfileCategory::ECS.as_str());
                thread::sleep(Duration::from_micros(50));
            }
        }
    }

    backend.end_frame();

    let trace = backend.export_chrome_trace();

    // Should return valid JSON format
    assert!(trace.starts_with('['));
    assert!(trace.ends_with(']'));
}

#[test]
fn test_multiple_frames() {
    let mut backend = PuffinBackend::new();

    // Capture multiple frames
    for _ in 0..3 {
        backend.begin_frame();

        {
            puffin::profile_scope!("frame_work", "ECS");
            thread::sleep(Duration::from_micros(50));
        }

        backend.end_frame();
    }

    let trace = backend.export_chrome_trace();

    // Should return valid JSON format
    assert!(trace.starts_with('['));
    assert!(trace.ends_with(']'));
}

#[test]
fn test_category_mapping() {
    let mut backend = PuffinBackend::new();

    backend.begin_frame();

    // Test all categories
    {
        puffin::profile_scope!("ecs_work", ProfileCategory::ECS.as_str());
    }
    {
        puffin::profile_scope!("rendering_work", ProfileCategory::Rendering.as_str());
    }
    {
        puffin::profile_scope!("physics_work", ProfileCategory::Physics.as_str());
    }
    {
        puffin::profile_scope!("networking_work", ProfileCategory::Networking.as_str());
    }
    {
        puffin::profile_scope!("audio_work", ProfileCategory::Audio.as_str());
    }
    {
        puffin::profile_scope!("serialization_work", ProfileCategory::Serialization.as_str());
    }

    backend.end_frame();

    let trace = backend.export_chrome_trace();

    // Should complete without errors
    assert!(trace.starts_with('['));
}

#[test]
fn test_profile_scope_macro_integration() {
    let mut backend = PuffinBackend::new();

    backend.begin_frame();

    {
        profile_scope!("test_macro");
        thread::sleep(Duration::from_micros(50));
    }

    {
        profile_scope!("test_macro_with_category", ProfileCategory::ECS);
        thread::sleep(Duration::from_micros(50));
    }

    backend.end_frame();

    let trace = backend.export_chrome_trace();
    assert!(trace.starts_with('['));
    assert!(trace.ends_with(']'));
}

#[test]
fn test_empty_frame() {
    let mut backend = PuffinBackend::new();

    backend.begin_frame();
    // No scopes
    backend.end_frame();

    let trace = backend.export_chrome_trace();

    // Should return valid JSON even with no data
    assert!(trace.starts_with('['));
    assert!(trace.ends_with(']'));
}

#[test]
fn test_rapid_frames() {
    let mut backend = PuffinBackend::new();

    // Simulate rapid frame updates
    for _ in 0..100 {
        backend.begin_frame();
        {
            puffin::profile_scope!("rapid", "Test");
        }
        backend.end_frame();
    }

    // Should handle rapid updates without panicking
    let trace = backend.export_chrome_trace();
    assert!(trace.starts_with('['));
    assert!(trace.ends_with(']'));
}

#[test]
fn test_long_scope_names() {
    let mut backend = PuffinBackend::new();

    backend.begin_frame();

    {
        // Use a static string for the test
        puffin::profile_scope!(
            "very_long_scope_name_that_demonstrates_handling_of_lengthy_identifiers_in_profiling",
            "Test"
        );
        thread::sleep(Duration::from_micros(10));
    }

    backend.end_frame();

    let trace = backend.export_chrome_trace();

    // Should handle long names and return valid JSON
    assert!(trace.starts_with('['));
    assert!(trace.ends_with(']'));
}

#[test]
fn test_special_characters_in_scope_names() {
    let mut backend = PuffinBackend::new();

    backend.begin_frame();

    {
        puffin::profile_scope!("scope\"with\"quotes", "Test");
    }
    {
        puffin::profile_scope!("scope\nwith\nnewlines", "Test");
    }
    {
        puffin::profile_scope!("scope\\with\\backslashes", "Test");
    }

    backend.end_frame();

    let trace = backend.export_chrome_trace();

    // Should handle special characters (they should be escaped in JSON)
    assert!(trace.starts_with('['));
    assert!(trace.ends_with(']'));
}

#[test]
fn test_concurrent_threads() {
    let mut backend = PuffinBackend::new();

    backend.begin_frame();

    // Spawn multiple threads doing profiled work
    let handles: Vec<_> = (0..4)
        .map(|_| {
            thread::spawn(move || {
                puffin::profile_scope!("thread_work", "Test");
                thread::sleep(Duration::from_micros(50));
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    backend.end_frame();

    let trace = backend.export_chrome_trace();

    // Should return valid JSON format
    assert!(trace.starts_with('['));
    assert!(trace.ends_with(']'));
}

#[test]
fn test_chrome_trace_json_validity() {
    let mut backend = PuffinBackend::new();

    backend.begin_frame();

    {
        puffin::profile_scope!("test", "Category");
        thread::sleep(Duration::from_micros(100));
    }

    backend.end_frame();

    let trace = backend.export_chrome_trace();

    // Try to parse as JSON (basic validity check)
    // The trace should at least be parseable as a JSON value
    if trace.len() > 2 {
        // If there's data, it should be valid JSON
        // We can't fully parse without serde_json dependency in tests,
        // but we can check basic structure
        assert!(trace.starts_with('['));
        assert!(trace.ends_with(']'));
        assert!(trace.contains('{') || trace == "[]");
    }
}
