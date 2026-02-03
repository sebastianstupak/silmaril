//! Integration test for AI Agent Feedback Metrics
//!
//! Verifies end-to-end functionality of the feedback metrics system.

#[cfg(feature = "metrics")]
use silmaril_profiling::{ProfileCategory, Profiler, ProfilerConfig};

#[cfg(feature = "metrics")]
use std::thread;
#[cfg(feature = "metrics")]
use std::time::Duration;

#[test]
#[cfg(feature = "metrics")]
fn test_full_metrics_pipeline() {
    // Create profiler
    let profiler = Profiler::new(ProfilerConfig::default());

    // Run several frames to build up history
    for _ in 0..20 {
        profiler.begin_frame();

        // Simulate various workloads
        {
            let _guard = profiler.scope("ecs_update", ProfileCategory::ECS);
            thread::sleep(Duration::from_millis(1));
        }

        {
            let _guard = profiler.scope("physics_step", ProfileCategory::Physics);
            thread::sleep(Duration::from_millis(2));
        }

        {
            let _guard = profiler.scope("render_frame", ProfileCategory::Rendering);
            thread::sleep(Duration::from_millis(5));
        }

        profiler.end_frame();
    }

    // Extract agent metrics
    let metrics = profiler.get_agent_metrics();

    // Verify frame timing
    assert!(metrics.frame_time_ms > 0.0, "Frame time should be positive");
    assert!(metrics.frame_time_p95_ms > 0.0, "P95 should be positive");
    assert!(metrics.fps > 0.0, "FPS should be positive");
    assert!(
        metrics.frame_time_p95_ms >= metrics.frame_time_ms * 0.8,
        "P95 should be close to current frame time"
    );

    // Verify system breakdown
    assert!(metrics.time_by_category.contains_key("ECS"), "Should have ECS timing");
    assert!(metrics.time_by_category.contains_key("Physics"), "Should have Physics timing");
    assert!(
        metrics.time_by_category.contains_key("Rendering"),
        "Should have Rendering timing"
    );

    // Verify each category has reasonable time
    for (category, time_ms) in &metrics.time_by_category {
        assert!(*time_ms > 0.0, "Category {} should have positive time", category);
        assert!(
            *time_ms < metrics.frame_time_ms,
            "Category {} time should be less than total frame time",
            category
        );
    }

    // Verify JSON serialization works
    let json = metrics.to_json();
    assert!(json.contains("frame_time_ms"), "JSON should contain frame_time_ms");
    assert!(json.contains("time_by_category"), "JSON should contain time_by_category");
    assert!(json.contains("ECS"), "JSON should contain ECS category");
    assert!(json.contains("Physics"), "JSON should contain Physics category");
    assert!(json.contains("Rendering"), "JSON should contain Rendering category");

    // Verify JSON is valid (at least basic structure)
    assert!(json.starts_with('{'), "JSON should start with {{");
    assert!(json.contains('}'), "JSON should contain closing brace");
}

#[test]
#[cfg(all(feature = "metrics", feature = "serde"))]
fn test_json_deserialization() {
    use silmaril_profiling::AgentFeedbackMetrics;

    // Create profiler and generate metrics
    let profiler = Profiler::new(ProfilerConfig::default());

    profiler.begin_frame();
    {
        let _guard = profiler.scope("test", ProfileCategory::ECS);
        thread::sleep(Duration::from_millis(1));
    }
    profiler.end_frame();

    let original = profiler.get_agent_metrics();

    // Serialize to JSON
    let json = original.to_json();

    // Deserialize back
    let deserialized: AgentFeedbackMetrics =
        serde_json::from_str(&json).expect("JSON should deserialize back to AgentFeedbackMetrics");

    // Verify key fields match
    assert_eq!(deserialized.frame_time_ms, original.frame_time_ms, "Frame time should match");
    assert_eq!(deserialized.frame_time_p95_ms, original.frame_time_p95_ms, "P95 should match");
    assert_eq!(deserialized.fps, original.fps, "FPS should match");
    assert_eq!(
        deserialized.is_frame_budget_met, original.is_frame_budget_met,
        "Budget status should match"
    );
}

#[test]
#[cfg(feature = "metrics")]
fn test_budget_detection() {
    // Create profiler
    let profiler = Profiler::new(ProfilerConfig::default());

    // Test frame under budget (fast frame)
    profiler.begin_frame();
    {
        let _guard = profiler.scope("fast_work", ProfileCategory::ECS);
        thread::sleep(Duration::from_millis(1));
    }
    profiler.end_frame();

    let fast_metrics = profiler.get_agent_metrics();
    assert!(fast_metrics.is_frame_budget_met, "Fast frame should meet budget");

    // Test frame over budget (slow frame)
    profiler.begin_frame();
    {
        let _guard = profiler.scope("slow_work", ProfileCategory::ECS);
        thread::sleep(Duration::from_millis(20));
    }
    profiler.end_frame();

    let slow_metrics = profiler.get_agent_metrics();
    assert!(!slow_metrics.is_frame_budget_met, "Slow frame should exceed budget");
}

#[test]
#[cfg(feature = "metrics")]
fn test_p95_calculation_accuracy() {
    // Create profiler
    let profiler = Profiler::new(ProfilerConfig::default());

    // Generate frames with varied distribution
    // Most frames fast, some slow - p95 should be higher than median
    for _ in 0..90 {
        profiler.begin_frame();
        thread::sleep(Duration::from_millis(2));
        profiler.end_frame();
    }

    for _ in 0..10 {
        profiler.begin_frame();
        thread::sleep(Duration::from_millis(10));
        profiler.end_frame();
    }

    let metrics = profiler.get_agent_metrics();

    // P95 should be significantly higher than the most common frame time
    // Since 90% of frames are ~2ms and 10% are ~10ms, p95 should be close to 10ms
    assert!(
        metrics.frame_time_p95_ms > 7.0,
        "P95 should reflect the slower frames, got {}ms",
        metrics.frame_time_p95_ms
    );

    // Also verify p95 is greater than current (which could be either fast or slow)
    assert!(
        metrics.frame_time_p95_ms >= metrics.frame_time_ms * 0.5,
        "P95 should be reasonable compared to current frame time"
    );
}

#[test]
#[cfg(feature = "metrics")]
fn test_category_aggregation() {
    // Create profiler
    let profiler = Profiler::new(ProfilerConfig::default());

    profiler.begin_frame();

    // Multiple scopes in same category should aggregate
    {
        let _guard = profiler.scope("physics_1", ProfileCategory::Physics);
        thread::sleep(Duration::from_millis(1));
    }

    {
        let _guard = profiler.scope("physics_2", ProfileCategory::Physics);
        thread::sleep(Duration::from_millis(1));
    }

    {
        let _guard = profiler.scope("physics_3", ProfileCategory::Physics);
        thread::sleep(Duration::from_millis(1));
    }

    profiler.end_frame();

    let metrics = profiler.get_agent_metrics();

    // Physics category should have aggregated time from all three scopes
    let physics_time = metrics.time_by_category.get("Physics").expect("Should have Physics");
    assert!(
        *physics_time >= 2.0,
        "Physics should aggregate all scopes, got {}ms",
        physics_time
    );
}
