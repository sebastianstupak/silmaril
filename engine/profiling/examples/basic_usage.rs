// Basic profiling usage example

use agent_game_engine_profiling::{ProfileCategory, Profiler, ProfilerConfig};
use std::thread;
use std::time::Duration;

fn main() {
    println!("=== Basic Profiling Example ===\n");

    // Create profiler with dev configuration
    let profiler = Profiler::new(ProfilerConfig::default_dev());

    println!("Running 5 frames with profiling...\n");

    for frame_num in 0..5 {
        profiler.begin_frame();

        // Simulate physics work
        {
            let _guard = profiler.scope("Physics", ProfileCategory::Physics);
            thread::sleep(Duration::from_millis(3));
        }

        // Simulate rendering work
        {
            let _guard = profiler.scope("Rendering", ProfileCategory::Rendering);
            thread::sleep(Duration::from_millis(8));
        }

        // Simulate networking work
        {
            let _guard = profiler.scope("Networking", ProfileCategory::Networking);
            thread::sleep(Duration::from_millis(2));
        }

        let metrics = profiler.end_frame();

        println!("Frame {}: {:.2}ms ({:.1} FPS)", frame_num, metrics.frame_time_ms, metrics.fps);

        // Show category breakdown
        for (category, time_ms) in &metrics.time_by_category {
            println!("  {:?}: {:.2}ms", category, time_ms);
        }
        println!();
    }

    println!("=== Example Complete ===");
}
