//! Integration tests for the observability crate.
//!
//! Tests the complete budget warning system as specified in Task 0.5.7.

use engine_observability::{Profiler, ProfilerConfig};
use std::thread;
use std::time::Duration;

#[test]
fn test_budget_violation_detection() {
    // Create profiler and set budget
    let mut profiler = Profiler::new(ProfilerConfig::default());
    profiler.set_budget("test_scope", Duration::from_millis(10));

    // Scope that exceeds budget
    {
        let _guard = profiler.scope("test_scope");
        thread::sleep(Duration::from_millis(20));
    }

    // Verify violation was recorded
    let violations = profiler.get_violations();
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].scope, "test_scope");
    assert!(violations[0].actual >= Duration::from_millis(20));
    assert_eq!(violations[0].budget, Duration::from_millis(10));
}

#[test]
fn test_warning_format() {
    // This test verifies the warning is logged in the correct format
    // The actual logging is tested via tracing, but we can verify the violation structure
    let mut profiler = Profiler::new(ProfilerConfig::default());
    profiler.set_budget("physics", Duration::from_millis(5));

    {
        let _guard = profiler.scope("physics");
        thread::sleep(Duration::from_millis(10));
    }

    let violations = profiler.get_violations();
    assert_eq!(violations.len(), 1);

    let violation = &violations[0];
    assert_eq!(violation.scope, "physics");
    assert_eq!(violation.budget, Duration::from_millis(5));

    // Verify the timing information is reasonable
    let actual_ms = violation.actual.as_secs_f32() * 1000.0;
    let budget_ms = violation.budget.as_secs_f32() * 1000.0;
    assert!(actual_ms > budget_ms);
}

#[test]
fn test_violation_history_tracking() {
    let mut profiler = Profiler::new(ProfilerConfig::default());
    profiler.set_budget("test", Duration::from_millis(1));

    // Create multiple violations
    for _ in 0..5 {
        let _guard = profiler.scope("test");
        thread::sleep(Duration::from_millis(5));
    }

    // Verify all violations are tracked
    let violations = profiler.get_violations();
    assert_eq!(violations.len(), 5);

    // Verify they all have the same scope
    for violation in &violations {
        assert_eq!(violation.scope, "test");
    }
}

#[test]
fn test_clearing_violations() {
    let mut profiler = Profiler::new(ProfilerConfig::default());
    profiler.set_budget("test", Duration::from_millis(1));

    // Create violations
    {
        let _guard = profiler.scope("test");
        thread::sleep(Duration::from_millis(5));
    }

    assert_eq!(profiler.get_violations().len(), 1);

    // Clear violations
    profiler.clear_violations();

    // Verify violations are cleared
    assert_eq!(profiler.get_violations().len(), 0);

    // But budget should still be set
    {
        let _guard = profiler.scope("test");
        thread::sleep(Duration::from_millis(5));
    }

    // New violation should be recorded
    assert_eq!(profiler.get_violations().len(), 1);
}

#[test]
fn test_nested_scopes_with_budgets() {
    let mut profiler = Profiler::new(ProfilerConfig::default());
    profiler.set_budget("outer", Duration::from_millis(5));
    profiler.set_budget("inner", Duration::from_millis(3));

    {
        let _outer = profiler.scope("outer");
        thread::sleep(Duration::from_millis(2));

        {
            let _inner = profiler.scope("inner");
            thread::sleep(Duration::from_millis(5));
        }

        thread::sleep(Duration::from_millis(4));
    }

    // Both scopes should have violations
    let violations = profiler.get_violations();
    assert_eq!(violations.len(), 2);

    // Find violations by scope name
    let inner_violation = violations.iter().find(|v| v.scope == "inner").unwrap();
    let outer_violation = violations.iter().find(|v| v.scope == "outer").unwrap();

    assert!(inner_violation.actual >= Duration::from_millis(5));
    assert!(outer_violation.actual >= Duration::from_millis(11));
}

#[test]
fn test_frame_tracking_in_violations() {
    let mut profiler = Profiler::new(ProfilerConfig::default());
    profiler.set_budget("test", Duration::from_millis(1));

    profiler.begin_frame();
    {
        let _guard = profiler.scope("test");
        thread::sleep(Duration::from_millis(5));
    }
    profiler.end_frame();

    profiler.begin_frame();
    {
        let _guard = profiler.scope("test");
        thread::sleep(Duration::from_millis(5));
    }
    profiler.end_frame();

    let violations = profiler.get_violations();
    assert_eq!(violations.len(), 2);

    // First violation should be from frame 0
    assert_eq!(violations[0].frame, 0);

    // Second violation should be from frame 1
    assert_eq!(violations[1].frame, 1);
}

#[test]
fn test_no_violations_without_budget() {
    let mut profiler = Profiler::new(ProfilerConfig::default());

    // No budget set for this scope
    {
        let _guard = profiler.scope("unbounded");
        thread::sleep(Duration::from_millis(100));
    }

    // Should not record any violations
    assert_eq!(profiler.get_violations().len(), 0);
}

#[test]
fn test_multiple_scopes_with_different_budgets() {
    let mut profiler = Profiler::new(ProfilerConfig::default());
    profiler.set_budget("physics", Duration::from_millis(5));
    profiler.set_budget("rendering", Duration::from_millis(8));
    profiler.set_budget("audio", Duration::from_millis(3));

    // Physics within budget
    {
        let _guard = profiler.scope("physics");
        // Very short - should be well within budget
    }

    // Rendering exceeds budget
    {
        let _guard = profiler.scope("rendering");
        thread::sleep(Duration::from_millis(15));
    }

    // Audio within budget
    {
        let _guard = profiler.scope("audio");
        // Very short - should be well within budget
    }

    let violations = profiler.get_violations();
    assert!(
        !violations.is_empty(),
        "Expected at least one violation, got {}",
        violations.len()
    );

    // Find the rendering violation
    let rendering_violation = violations.iter().find(|v| v.scope == "rendering");
    assert!(rendering_violation.is_some(), "Expected rendering violation");
    assert!(rendering_violation.unwrap().actual >= Duration::from_millis(15));
}

#[test]
fn test_disabled_profiler_no_violations() {
    let mut profiler = Profiler::new(ProfilerConfig { enabled: false });
    profiler.set_budget("test", Duration::from_millis(1));

    {
        let _guard = profiler.scope("test");
        thread::sleep(Duration::from_millis(10));
    }

    // Disabled profiler should not record violations
    assert_eq!(profiler.get_violations().len(), 0);
}

#[test]
fn test_violation_timestamp_ordering() {
    let mut profiler = Profiler::new(ProfilerConfig::default());
    profiler.set_budget("test", Duration::from_millis(1));

    {
        let _guard = profiler.scope("test");
        thread::sleep(Duration::from_millis(5));
    }

    thread::sleep(Duration::from_millis(10));

    {
        let _guard = profiler.scope("test");
        thread::sleep(Duration::from_millis(5));
    }

    let violations = profiler.get_violations();
    assert_eq!(violations.len(), 2);

    // Second violation should have a later timestamp
    assert!(violations[0].timestamp < violations[1].timestamp);
}
