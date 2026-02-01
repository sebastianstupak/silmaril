//! Example demonstrating AI Agent Feedback Metrics
//!
//! This example shows how to collect comprehensive metrics for AI agent consumption.
//! Run with: `cargo run --example agent_feedback --features metrics,config`

use agent_game_engine_profiling::{ProfileCategory, Profiler, ProfilerConfig};
use std::thread;
use std::time::Duration;

fn main() {
    // Create a profiler with default configuration
    let profiler = Profiler::new(ProfilerConfig::default());

    println!("Running sample game frames...\n");

    // Simulate 10 game frames
    for _frame in 0..10 {
        profiler.begin_frame();

        // Simulate ECS work
        {
            let _guard = profiler.scope("ecs_update", ProfileCategory::ECS);
            thread::sleep(Duration::from_millis(2));
        }

        // Simulate physics
        {
            let _guard = profiler.scope("physics_step", ProfileCategory::Physics);
            thread::sleep(Duration::from_millis(3));
        }

        // Simulate rendering
        {
            let _guard = profiler.scope("render_frame", ProfileCategory::Rendering);
            thread::sleep(Duration::from_millis(8));
        }

        let metrics = profiler.end_frame();
        println!(
            "Frame {}: {:.2}ms ({:.1} FPS)",
            metrics.frame_number, metrics.frame_time_ms, metrics.fps
        );
    }

    println!("\n=== AI Agent Feedback Metrics ===\n");

    // Extract comprehensive metrics for AI agent
    let agent_metrics = profiler.get_agent_metrics();

    // Display metrics
    println!("Frame Timing:");
    println!("  Current: {:.2}ms", agent_metrics.frame_time_ms);
    println!("  P95:     {:.2}ms", agent_metrics.frame_time_p95_ms);
    println!("  FPS:     {:.1}", agent_metrics.fps);
    println!(
        "  Budget:  {}",
        if agent_metrics.is_frame_budget_met { "MET" } else { "EXCEEDED" }
    );

    println!("\nTime by Category:");
    for (category, time_ms) in &agent_metrics.time_by_category {
        println!("  {}: {:.2}ms", category, time_ms);
    }

    println!("\nECS Stats:");
    println!("  Entities:    {}", agent_metrics.entity_count);
    println!("  Archetypes:  {}", agent_metrics.archetype_count);

    println!("\nMemory:");
    println!("  Used: {}MB", agent_metrics.memory_used_mb);
    println!("  Peak: {}MB", agent_metrics.memory_peak_mb);

    println!("\n=== JSON Output ===\n");

    // Serialize to JSON for AI agent consumption
    let json = agent_metrics.to_json();
    println!("{}", json);

    println!("\n=== Usage Notes ===\n");
    println!("This JSON can be:");
    println!("- Sent to an AI agent for analysis");
    println!("- Stored as training data");
    println!("- Used for automated performance regression detection");
    println!("- Integrated into CI/CD pipelines");
}
