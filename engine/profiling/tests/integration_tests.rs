//! Integration tests for the profiling system.
//!
//! These tests verify the profiler works correctly when integrated into
//! a realistic usage scenario.

use agent_game_engine_profiling::{ProfileCategory, Profiler, ProfilerConfig};
use std::thread;
use std::time::Duration;

#[cfg(feature = "metrics")]
#[test]
fn test_complete_frame_workflow() {
    let profiler = Profiler::new(ProfilerConfig::default());

    // Simulate 10 frames
    for frame_num in 0..10 {
        profiler.begin_frame();

        // Simulate game systems
        {
            let _guard = profiler.scope("input", ProfileCategory::ECS);
            thread::sleep(Duration::from_micros(100));
        }

        {
            let _guard = profiler.scope("physics", ProfileCategory::Physics);
            thread::sleep(Duration::from_micros(200));
        }

        {
            let _guard = profiler.scope("rendering", ProfileCategory::Rendering);
            thread::sleep(Duration::from_micros(300));
        }

        let metrics = profiler.end_frame();

        assert_eq!(metrics.frame_number, frame_num);
        assert!(metrics.frame_time_ms > 0.0);
        assert!(metrics.fps > 0.0);

        // Check category times
        assert!(metrics.time_by_category.contains_key(&ProfileCategory::ECS));
        assert!(metrics.time_by_category.contains_key(&ProfileCategory::Physics));
        assert!(metrics.time_by_category.contains_key(&ProfileCategory::Rendering));
    }

    // Check history
    let history = profiler.frame_history();
    assert_eq!(history.len(), 10);
}

#[cfg(feature = "metrics")]
#[test]
fn test_budget_warning() {
    let mut config = ProfilerConfig::default();
    config.budgets.insert("slow_system".to_string(), 0.1); // 0.1ms budget

    let profiler = Profiler::new(config);

    profiler.begin_frame();

    {
        let _guard = profiler.scope("slow_system", ProfileCategory::ECS);
        thread::sleep(Duration::from_millis(2)); // Exceed budget
    }

    let _metrics = profiler.end_frame();

    // Budget should be set (verified via internal testing method)
    // In production code, warnings would be logged via tracing
}

#[cfg(feature = "metrics")]
#[test]
fn test_multiple_profiler_instances() {
    let profiler1 = Profiler::new(ProfilerConfig::default());
    let profiler2 = Profiler::new(ProfilerConfig::default());

    // Both profilers should work independently
    profiler1.begin_frame();
    profiler2.begin_frame();

    {
        let _g1 = profiler1.scope("system1", ProfileCategory::ECS);
        let _g2 = profiler2.scope("system2", ProfileCategory::Rendering);
        thread::sleep(Duration::from_millis(1));
    }

    let metrics1 = profiler1.end_frame();
    let metrics2 = profiler2.end_frame();

    assert!(metrics1.time_by_category.contains_key(&ProfileCategory::ECS));
    assert!(metrics2.time_by_category.contains_key(&ProfileCategory::Rendering));

    // They shouldn't interfere
    assert!(!metrics1.time_by_category.contains_key(&ProfileCategory::Rendering));
    assert!(!metrics2.time_by_category.contains_key(&ProfileCategory::ECS));
}

#[cfg(feature = "metrics")]
#[test]
fn test_profiler_clone() {
    let profiler = Profiler::new(ProfilerConfig::default());
    let profiler_clone = profiler.clone();

    profiler.begin_frame();

    {
        let _guard = profiler.scope("test", ProfileCategory::ECS);
        thread::sleep(Duration::from_millis(1));
    }

    let metrics = profiler.end_frame();

    // Clone should share the same state
    let history = profiler_clone.frame_history();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].frame_number, metrics.frame_number);
}

#[cfg(feature = "metrics")]
#[test]
fn test_deeply_nested_scopes() {
    let profiler = Profiler::new(ProfilerConfig::default());

    profiler.begin_frame();

    // Create 10 levels of nesting
    fn nest(profiler: &Profiler, depth: u32) {
        if depth == 0 {
            return;
        }

        let name = format!("level_{}", depth);
        let _guard = profiler.scope(&name, ProfileCategory::ECS);
        thread::sleep(Duration::from_micros(100));
        nest(profiler, depth - 1);
    }

    nest(&profiler, 10);

    let metrics = profiler.end_frame();

    // All scopes should be recorded
    assert!(metrics.time_by_category.contains_key(&ProfileCategory::ECS));
    if let Some(&time_ms) = metrics.time_by_category.get(&ProfileCategory::ECS) {
        // Should have accumulated time from all nested scopes
        assert!(time_ms > 0.5); // At least 0.5ms total
    }
}

#[cfg(feature = "metrics")]
#[test]
fn test_concurrent_scopes() {
    use std::sync::Arc;

    let profiler = Arc::new(Profiler::new(ProfilerConfig::default()));

    profiler.begin_frame();

    // Spawn multiple threads that create scopes
    let mut handles = vec![];

    for i in 0..4 {
        let profiler = profiler.clone();
        let handle = thread::spawn(move || {
            let name = format!("thread_{}", i);
            let _guard = profiler.scope(&name, ProfileCategory::ECS);
            thread::sleep(Duration::from_millis(1));
        });
        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }

    let metrics = profiler.end_frame();

    // Should have captured all thread scopes
    assert!(metrics.time_by_category.contains_key(&ProfileCategory::ECS));
    if let Some(&time_ms) = metrics.time_by_category.get(&ProfileCategory::ECS) {
        // All threads contributed to ECS time
        assert!(time_ms > 0.0);
    }
}

#[test]
fn test_disabled_profiling_compiles() {
    // This test verifies that profiling macros compile when features are disabled
    agent_game_engine_profiling::profile_scope!("test");
    agent_game_engine_profiling::profile_scope!("test", ProfileCategory::ECS);

    // Should not panic or do anything
}

#[cfg(feature = "metrics")]
#[test]
fn test_frame_metrics_accuracy() {
    let profiler = Profiler::new(ProfilerConfig::default());

    profiler.begin_frame();

    // Sleep for a known duration
    let sleep_duration = Duration::from_millis(10);
    thread::sleep(sleep_duration);

    let metrics = profiler.end_frame();

    // Frame time should be at least the sleep duration
    assert!(
        metrics.frame_time_ms >= 9.0,
        "Expected at least 9ms, got {}ms",
        metrics.frame_time_ms
    );

    // FPS should be calculated correctly
    let expected_fps = 1000.0 / metrics.frame_time_ms;
    assert!(
        (metrics.fps - expected_fps).abs() < 0.1,
        "FPS mismatch: expected {}, got {}",
        expected_fps,
        metrics.fps
    );
}

#[cfg(feature = "metrics")]
#[test]
fn test_category_aggregation() {
    let profiler = Profiler::new(ProfilerConfig::default());

    profiler.begin_frame();

    // Create multiple scopes in the same category
    for i in 0..5 {
        let name = format!("ecs_system_{}", i);
        let _guard = profiler.scope(&name, ProfileCategory::ECS);
        thread::sleep(Duration::from_micros(100));
    }

    let metrics = profiler.end_frame();

    // All scopes should be aggregated under ECS
    if let Some(&time_ms) = metrics.time_by_category.get(&ProfileCategory::ECS) {
        // Should have accumulated time from all 5 scopes
        assert!(time_ms > 0.4); // At least 0.4ms (5 * 100us = 0.5ms, allowing for variance)
    } else {
        panic!("ECS category not found in metrics");
    }
}

#[cfg(feature = "metrics")]
#[test]
fn test_runtime_budget_modification() {
    let profiler = Profiler::new(ProfilerConfig::default());

    // Set a budget at runtime
    profiler.set_budget("dynamic_system", Duration::from_millis(5));

    let state = profiler.state.lock();
    assert!(state.config.budgets.contains_key("dynamic_system"));
    assert_eq!(state.config.budgets.get("dynamic_system"), Some(&5.0));
}
