//! Demonstration of the Query API for AI agents.
//!
//! This example shows how AI agents can programmatically query profiling data
//! to make informed decisions about performance optimization.
//!
//! Run with: cargo run --example query_api_demo --features metrics

use agent_game_engine_profiling::{ProfileCategory, Profiler, ProfilerConfig};
use std::thread;
use std::time::Duration;

fn main() {
    println!("=== Query API Demo for AI Agents ===\n");

    let profiler = Profiler::new(ProfilerConfig::default());

    // Simulate a game running for 100 frames
    println!("Simulating 100 frames of game execution...\n");

    for frame in 0..100 {
        profiler.begin_frame();

        // Simulate physics work (varies by frame)
        {
            let _guard = profiler.scope("physics_step", ProfileCategory::Physics);
            let delay = if frame < 50 { 100 } else { 150 }; // Physics gets slower after frame 50
            thread::sleep(Duration::from_micros(delay));
        }

        // Simulate rendering work
        {
            let _guard = profiler.scope("render_frame", ProfileCategory::Rendering);
            thread::sleep(Duration::from_micros(200));
        }

        // Simulate ECS updates
        {
            let _guard = profiler.scope("ecs_update", ProfileCategory::ECS);
            thread::sleep(Duration::from_micros(50));
        }

        profiler.end_frame();
    }

    println!("=== AI Agent Queries ===\n");

    // Query 1: Overall physics performance
    println!("Query 1: Overall physics performance");
    let physics_stats = profiler.query().category(ProfileCategory::Physics).aggregate();

    println!("  Total calls: {}", physics_stats.call_count);
    println!("  Average time: {:.2}us", physics_stats.avg_time_us);
    println!("  p50 (median): {}us", physics_stats.p50_us);
    println!("  p95: {}us", physics_stats.p95_us);
    println!("  p99: {}us", physics_stats.p99_us);
    println!("  Min: {}us, Max: {}us\n", physics_stats.min_us, physics_stats.max_us);

    // Query 2: Compare early vs late frames for physics
    println!("Query 2: Physics performance - early vs late frames");

    let early_physics =
        profiler.query().frames(0..50).category(ProfileCategory::Physics).aggregate();

    let late_physics =
        profiler.query().frames(50..100).category(ProfileCategory::Physics).aggregate();

    println!("  Early frames (0-50):");
    println!("    Average: {:.2}us", early_physics.avg_time_us);
    println!("    p95: {}us", early_physics.p95_us);

    println!("  Late frames (50-100):");
    println!("    Average: {:.2}us", late_physics.avg_time_us);
    println!("    p95: {}us", late_physics.p95_us);

    let perf_delta = late_physics.avg_time_us - early_physics.avg_time_us;
    let perf_delta_pct = (perf_delta / early_physics.avg_time_us) * 100.0;

    println!("  Performance delta: {:.2}us ({:.1}%)\n", perf_delta, perf_delta_pct);

    if perf_delta_pct > 10.0 {
        println!("  AI Agent Decision: Physics performance degraded by {:.1}%", perf_delta_pct);
        println!("  Recommendation: Investigate physics optimization\n");
    }

    // Query 3: Compare different systems
    println!("Query 3: System performance comparison");

    let rendering_stats = profiler.query().category(ProfileCategory::Rendering).aggregate();
    let ecs_stats = profiler.query().category(ProfileCategory::ECS).aggregate();

    println!(
        "  Rendering: avg={:.2}us, p95={}us",
        rendering_stats.avg_time_us, rendering_stats.p95_us
    );
    println!(
        "  Physics: avg={:.2}us, p95={}us",
        physics_stats.avg_time_us, physics_stats.p95_us
    );
    println!("  ECS: avg={:.2}us, p95={}us\n", ecs_stats.avg_time_us, ecs_stats.p95_us);

    // Query 4: Specific scope analysis
    println!("Query 4: Specific scope analysis");

    let render_timeline = profiler.query().scope("render_frame").timeline();

    println!("  Render frame timeline events: {}", render_timeline.len());

    if let Some(slowest) = render_timeline.iter().max_by_key(|e| e.duration_us) {
        println!(
            "  Slowest render frame: frame {}, duration: {}us",
            slowest.frame, slowest.duration_us
        );
    }

    println!();

    // Query 5: Export Chrome Trace for visualization
    println!("Query 5: Chrome Trace export");

    let trace = profiler.query().frames(0..10).chrome_trace();

    println!("  Exported first 10 frames to Chrome Trace format");
    println!("  Size: {} bytes", trace.len());
    println!("  Can be loaded in chrome://tracing for visualization\n");

    // Example: AI Agent decision making based on metrics
    println!("=== AI Agent Decision Making ===\n");

    let total_frame_budget_us = 16_667; // 60 FPS = 16.67ms
    let total_system_time_us = physics_stats.p95_us + rendering_stats.p95_us + ecs_stats.p95_us;

    println!("  Frame budget: {}us (60 FPS)", total_frame_budget_us);
    println!("  Total p95 system time: {}us", total_system_time_us);

    let utilization_pct = (total_system_time_us as f32 / total_frame_budget_us as f32) * 100.0;
    println!("  Budget utilization: {:.1}%\n", utilization_pct);

    if total_system_time_us > total_frame_budget_us as u64 {
        println!("  AI Agent Decision: Frame budget exceeded!");
        println!("  Action: Optimization required\n");

        // Identify bottleneck
        let max_time = physics_stats.p95_us.max(rendering_stats.p95_us).max(ecs_stats.p95_us);

        if max_time == physics_stats.p95_us {
            println!("  Bottleneck: Physics ({}us)", physics_stats.p95_us);
        } else if max_time == rendering_stats.p95_us {
            println!("  Bottleneck: Rendering ({}us)", rendering_stats.p95_us);
        } else {
            println!("  Bottleneck: ECS ({}us)", ecs_stats.p95_us);
        }
    } else {
        println!("  AI Agent Decision: Performance within budget");
        println!(
            "  Frame budget headroom: {}us ({:.1}%)",
            total_frame_budget_us - total_system_time_us as u64,
            100.0 - utilization_pct
        );
    }

    println!("\n=== Demo Complete ===");
}
