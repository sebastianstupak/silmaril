//! Basic Puffin profiler usage example.
//!
//! This example demonstrates how to use the Puffin backend for profiling.
//!
//! Run with: cargo run --example puffin_basic --features profiling-puffin

#[cfg(feature = "profiling-puffin")]
fn main() {
    use agent_game_engine_profiling::{backends::PuffinBackend, profile_scope, ProfileCategory};
    use std::thread;
    use std::time::Duration;

    println!("Puffin Profiler Example");
    println!("======================\n");

    let mut backend = PuffinBackend::new();

    // Simulate a game loop
    for frame in 0..10 {
        backend.begin_frame();

        // ECS update
        {
            profile_scope!("ecs_update", ProfileCategory::ECS);
            thread::sleep(Duration::from_millis(2));

            // Nested scope
            {
                profile_scope!("entity_spawn");
                thread::sleep(Duration::from_millis(1));
            }
        }

        // Physics simulation
        {
            profile_scope!("physics_step", ProfileCategory::Physics);
            thread::sleep(Duration::from_millis(5));
        }

        // Rendering
        {
            profile_scope!("render_frame", ProfileCategory::Rendering);
            thread::sleep(Duration::from_millis(8));
        }

        backend.end_frame();

        println!("Frame {} complete", frame);
    }

    // Export Chrome Trace
    let trace = backend.export_chrome_trace();
    std::fs::write("profiling_trace.json", trace).expect("Failed to write trace file");

    println!("\n✅ Profiling complete!");
    println!("📊 Chrome Trace exported to: profiling_trace.json");
    println!("📈 Open chrome://tracing and load the file to visualize");
    println!("\nAlternatively, use Puffin viewer:");
    println!("  1. cargo install puffin_viewer");
    println!("  2. puffin_viewer");
    println!("  3. Load profiling_trace.json");
}

#[cfg(not(feature = "profiling-puffin"))]
fn main() {
    eprintln!("This example requires the 'profiling-puffin' feature.");
    eprintln!("Run with: cargo run --example puffin_basic --features profiling-puffin");
    std::process::exit(1);
}
