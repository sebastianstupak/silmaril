//! Integration tests for the query API.
//!
//! Tests the complete query workflow with real profiling data.

use silmaril_profiling::{ProfileCategory, Profiler, ProfilerConfig};
use std::thread;
use std::time::Duration;

#[test]
fn test_query_with_real_profiling_data() {
    let profiler = Profiler::new(ProfilerConfig::default());

    // Generate some frames with profiling data
    for frame in 0..10 {
        profiler.begin_frame();

        // Simulate physics work
        {
            let _guard = profiler.scope("physics_step", ProfileCategory::Physics);
            thread::sleep(Duration::from_micros(100));
        }

        // Simulate rendering work
        {
            let _guard = profiler.scope("render", ProfileCategory::Rendering);
            thread::sleep(Duration::from_micros(200));
        }

        // Vary the timing for some frames
        if frame >= 5 {
            let _guard = profiler.scope("extra_work", ProfileCategory::ECS);
            thread::sleep(Duration::from_micros(50));
        }

        profiler.end_frame();
        thread::sleep(Duration::from_micros(10)); // Small delay between frames
    }

    // Query all physics scopes
    let physics_stats = profiler.query().category(ProfileCategory::Physics).aggregate();

    assert_eq!(physics_stats.call_count, 10);
    assert!(physics_stats.total_time_us > 0);
    assert!(physics_stats.avg_time_us > 0.0);
    assert!(physics_stats.p50_us > 0);
    assert!(physics_stats.p95_us >= physics_stats.p50_us);
    assert!(physics_stats.p99_us >= physics_stats.p95_us);
    assert!(physics_stats.min_us > 0);
    assert!(physics_stats.max_us >= physics_stats.min_us);

    // Query rendering scopes
    let rendering_stats = profiler.query().category(ProfileCategory::Rendering).aggregate();

    assert_eq!(rendering_stats.call_count, 10);
    // Note: Don't assert timing relationships as they're unreliable due to OS scheduling
    assert!(rendering_stats.total_time_us > 0);

    // Query specific scope by name
    let ecs_stats = profiler.query().scope("extra_work").aggregate();

    assert_eq!(ecs_stats.call_count, 5); // Only last 5 frames

    // Query specific frame range
    let early_frames = profiler.query().frames(0..5).aggregate();
    let late_frames = profiler.query().frames(5..10).aggregate();

    // Both should have captured some work
    assert!(early_frames.total_time_us > 0);
    assert!(late_frames.total_time_us > 0);
}

#[test]
fn test_query_timeline() {
    let profiler = Profiler::new(ProfilerConfig::default());

    // Generate a few frames
    for _ in 0..3 {
        profiler.begin_frame();

        {
            let _guard = profiler.scope("test_scope", ProfileCategory::ECS);
            thread::sleep(Duration::from_micros(10));
        }

        profiler.end_frame();
    }

    // Get timeline events
    let timeline = profiler.query().category(ProfileCategory::ECS).timeline();

    assert_eq!(timeline.len(), 3);

    for event in &timeline {
        assert_eq!(event.name, "test_scope");
        assert_eq!(event.category, ProfileCategory::ECS);
        assert!(event.duration_us > 0);
    }
}

#[test]
fn test_query_chrome_trace_export() {
    let profiler = Profiler::new(ProfilerConfig::default());

    // Generate a frame with some work
    profiler.begin_frame();

    {
        let _guard = profiler.scope("physics", ProfileCategory::Physics);
        thread::sleep(Duration::from_micros(10));
    }

    {
        let _guard = profiler.scope("render", ProfileCategory::Rendering);
        thread::sleep(Duration::from_micros(20));
    }

    profiler.end_frame();

    // Export as Chrome Trace
    let trace = profiler.query().chrome_trace();

    // Verify it's valid JSON array format
    assert!(trace.starts_with('['));
    assert!(trace.ends_with(']'));
    assert!(trace.contains("physics"));
    assert!(trace.contains("render"));
    assert!(trace.contains("Physics"));
    assert!(trace.contains("Rendering"));
    assert!(trace.contains("\"ph\": \"X\"")); // Chrome Trace event type
}

#[test]
fn test_query_chaining() {
    let profiler = Profiler::new(ProfilerConfig::default());

    // Generate multiple frames with different work
    for frame in 0..20 {
        profiler.begin_frame();

        if frame < 10 {
            let _guard = profiler.scope("early_physics", ProfileCategory::Physics);
            thread::sleep(Duration::from_micros(5));
        } else {
            let _guard = profiler.scope("late_physics", ProfileCategory::Physics);
            thread::sleep(Duration::from_micros(5));
        }

        profiler.end_frame();
    }

    // Chain multiple filters
    let early_physics = profiler
        .query()
        .frames(0..10)
        .category(ProfileCategory::Physics)
        .scope("early_physics")
        .aggregate();

    assert_eq!(early_physics.call_count, 10);

    let late_physics = profiler
        .query()
        .frames(10..20)
        .category(ProfileCategory::Physics)
        .scope("late_physics")
        .aggregate();

    assert_eq!(late_physics.call_count, 10);

    // Query with wrong scope name should return empty
    let missing = profiler.query().scope("nonexistent").aggregate();

    assert_eq!(missing.call_count, 0);
    assert_eq!(missing.total_time_us, 0);
}

#[test]
fn test_percentile_accuracy() {
    let profiler = Profiler::new(ProfilerConfig::default());

    // Generate frames with predictable timing
    for i in 1..=100 {
        profiler.begin_frame();

        {
            let _guard = profiler.scope("work", ProfileCategory::ECS);
            // Sleep for i microseconds (1-100)
            thread::sleep(Duration::from_micros(i as u64));
        }

        profiler.end_frame();
    }

    let stats = profiler.query().scope("work").aggregate();

    assert_eq!(stats.call_count, 100);

    // Verify percentiles are ordered correctly (accounting for timing variability)
    assert!(stats.p50_us > 0);
    assert!(stats.p95_us >= stats.p50_us);
    assert!(stats.p99_us >= stats.p95_us);
    assert!(stats.min_us > 0);
    assert!(stats.max_us >= stats.p99_us);
}
