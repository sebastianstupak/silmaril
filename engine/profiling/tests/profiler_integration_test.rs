//! Integration test for basic profiler functionality

use silmaril_profiling::{ProfileCategory, Profiler, ProfilerConfig};
use std::thread;
use std::time::Duration;

fn simulate_work(ms: u64) {
    thread::sleep(Duration::from_micros(ms * 100)); // Shorter for tests
}

#[test]
fn test_profiler_basic_usage() {
    let profiler = Profiler::new(ProfilerConfig::default());

    // Set budgets
    profiler.set_budget("game_loop", Duration::from_millis(16));
    profiler.set_budget("physics", Duration::from_millis(5));

    // Run 10 frames
    for _ in 0..10 {
        profiler.begin_frame();

        {
            let _guard = profiler.scope("input", ProfileCategory::ECS);
            simulate_work(10);
        }

        {
            let _guard = profiler.scope("physics", ProfileCategory::Physics);
            simulate_work(30);
        }

        {
            let _guard = profiler.scope("rendering", ProfileCategory::Rendering);
            simulate_work(50);
        }

        let metrics = profiler.end_frame();

        // Verify metrics are reasonable
        assert!(metrics.frame_time_ms > 0.0);
        assert!(metrics.fps > 0.0);
        assert_eq!(metrics.time_by_category.len(), 3); // ECS, Physics, Rendering
    }

    // Verify history
    let history = profiler.frame_history();
    assert_eq!(history.len(), 10, "Should have 10 frames of history");

    // Calculate statistics
    let avg_frame_time: f32 =
        history.iter().map(|m| m.frame_time_ms).sum::<f32>() / history.len() as f32;
    assert!(avg_frame_time > 0.0);

    let min_frame_time = history
        .iter()
        .map(|m| m.frame_time_ms)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let max_frame_time = history
        .iter()
        .map(|m| m.frame_time_ms)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();

    assert!(min_frame_time <= avg_frame_time);
    assert!(max_frame_time >= avg_frame_time);
}

#[test]
fn test_profiler_scope_nesting() {
    let profiler = Profiler::new(ProfilerConfig::default());

    profiler.begin_frame();

    {
        let _outer = profiler.scope("outer", ProfileCategory::ECS);
        simulate_work(10);

        {
            let _inner = profiler.scope("inner", ProfileCategory::Physics);
            simulate_work(5);
        }

        simulate_work(10);
    }

    let metrics = profiler.end_frame();

    // Should have both categories
    assert!(metrics.time_by_category.contains_key(&ProfileCategory::ECS));
    assert!(metrics.time_by_category.contains_key(&ProfileCategory::Physics));
}

#[test]
fn test_profiler_category_aggregation() {
    let profiler = Profiler::new(ProfilerConfig::default());

    profiler.begin_frame();

    // Multiple scopes in same category
    {
        let _guard1 = profiler.scope("task1", ProfileCategory::Physics);
        simulate_work(10);
    }

    {
        let _guard2 = profiler.scope("task2", ProfileCategory::Physics);
        simulate_work(10);
    }

    let metrics = profiler.end_frame();

    // Physics category should aggregate both tasks
    let physics_time = metrics.time_by_category.get(&ProfileCategory::Physics).unwrap();
    assert!(*physics_time > 0.0);
}

#[test]
fn test_frame_history_limit() {
    let mut config = ProfilerConfig::default();
    config.retention.circular_buffer_frames = 100; // Set explicit limit for testing
    let profiler = Profiler::new(config);

    // Run more frames than history should hold
    for _ in 0..150 {
        profiler.begin_frame();
        simulate_work(1);
        let _ = profiler.end_frame();
    }

    let history = profiler.frame_history();

    // Should be limited to configured max history size
    assert!(history.len() <= 100, "History should be limited to 100, got {}", history.len());
    // Should have some frames (not empty)
    assert!(!history.is_empty(), "History should not be empty");
}
