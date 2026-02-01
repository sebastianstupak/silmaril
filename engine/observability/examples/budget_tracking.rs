//! Example demonstrating the performance budget tracking system.
//!
//! This example shows how to:
//! 1. Set performance budgets for different scopes
//! 2. Profile code execution
//! 3. Get warnings when budgets are exceeded
//! 4. Query violation history

use engine_observability::{Profiler, ProfilerConfig};
use std::thread;
use std::time::Duration;

fn main() {
    // Initialize tracing to see the warnings
    tracing_subscriber::fmt().with_max_level(tracing::Level::WARN).init();

    println!("Performance Budget Tracking Example");
    println!("====================================\n");

    // Create a profiler
    let mut profiler = Profiler::new(ProfilerConfig::default());

    // Set performance budgets for different systems
    profiler.set_budget("physics", Duration::from_millis(5));
    profiler.set_budget("rendering", Duration::from_millis(8));
    profiler.set_budget("audio", Duration::from_millis(2));

    println!("Set budgets:");
    println!("  - physics: 5ms");
    println!("  - rendering: 8ms");
    println!("  - audio: 2ms\n");

    // Simulate a game loop with multiple frames
    for frame in 0..3 {
        println!("Frame {}:", frame);
        profiler.begin_frame();

        // Physics system - within budget
        {
            let _guard = profiler.scope("physics");
            simulate_physics();
            println!("  Physics: ~3ms (within budget)");
        }

        // Rendering system - exceeds budget on frame 1
        {
            let _guard = profiler.scope("rendering");
            if frame == 1 {
                simulate_slow_rendering();
                println!("  Rendering: ~10ms (EXCEEDS BUDGET - warning should be logged)");
            } else {
                simulate_rendering();
                println!("  Rendering: ~6ms (within budget)");
            }
        }

        // Audio system - exceeds budget on frame 2
        {
            let _guard = profiler.scope("audio");
            if frame == 2 {
                simulate_slow_audio();
                println!("  Audio: ~5ms (EXCEEDS BUDGET - warning should be logged)");
            } else {
                simulate_audio();
                println!("  Audio: ~1ms (within budget)");
            }
        }

        profiler.end_frame();
        println!();
    }

    // Query violation history
    let violations = profiler.get_violations();
    println!("\nViolation Summary:");
    println!("==================");
    println!("Total violations: {}\n", violations.len());

    for (i, violation) in violations.iter().enumerate() {
        println!("Violation #{}", i + 1);
        println!("  Scope: {}", violation.scope);
        println!("  Frame: {}", violation.frame);
        println!("  Actual: {:.2}ms", violation.actual.as_secs_f32() * 1000.0);
        println!("  Budget: {:.2}ms", violation.budget.as_secs_f32() * 1000.0);
        println!(
            "  Exceeded by: {:.2}ms",
            (violation.actual.as_secs_f32() - violation.budget.as_secs_f32()) * 1000.0
        );
        println!();
    }

    // Clear violations
    println!("Clearing violations...");
    profiler.clear_violations();
    println!("Violations after clear: {}\n", profiler.get_violations().len());
}

fn simulate_physics() {
    thread::sleep(Duration::from_millis(3));
}

fn simulate_rendering() {
    thread::sleep(Duration::from_millis(6));
}

fn simulate_slow_rendering() {
    thread::sleep(Duration::from_millis(10));
}

fn simulate_audio() {
    thread::sleep(Duration::from_millis(1));
}

fn simulate_slow_audio() {
    thread::sleep(Duration::from_millis(5));
}
