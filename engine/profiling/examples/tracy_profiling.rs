//! Tracy profiler integration example.
//!
//! This example demonstrates how to use Tracy for real-time performance profiling.
//!
//! # Setup
//!
//! 1. Download Tracy profiler from: https://github.com/wolfpld/tracy/releases
//! 2. Build with Tracy enabled:
//!    ```bash
//!    cargo run --example tracy_profiling --features profiling-tracy
//!    ```
//! 3. Launch the Tracy profiler GUI
//! 4. Connect to localhost to see real-time profiling data
//!
//! # What You'll See
//!
//! - Frame markers showing frame boundaries
//! - Categorized scopes (Physics, Rendering, etc.)
//! - Nested scope hierarchies
//! - Real-time timeline with < 10ns overhead per scope
//!
//! # Key Features
//!
//! - **Ultra-low overhead**: < 10ns per scope (5-20x faster than Puffin)
//! - **Real-time visualization**: See performance data as it happens
//! - **Remote profiling**: Profile on one machine, view on another
//! - **GPU profiling**: Future support for GPU timeline (Phase 4+)

#[cfg(feature = "profiling-tracy")]
use agent_game_engine_profiling::{profile_scope, ProfileCategory, TracyBackend};

#[cfg(not(feature = "profiling-tracy"))]
fn main() {
    println!("This example requires the 'profiling-tracy' feature flag.");
    println!("Run with: cargo run --example tracy_profiling --features profiling-tracy");
}

#[cfg(feature = "profiling-tracy")]
fn main() {
    println!("Tracy Profiler Example");
    println!("======================");
    println!();
    println!("1. Launch the Tracy profiler GUI");
    println!("2. Connect to localhost");
    println!("3. You should see real-time profiling data");
    println!();
    println!("Running simulation for 60 frames...");
    println!();

    // Initialize Tracy backend
    let mut backend = TracyBackend::new();

    // Simulate a game loop
    for frame in 0..60 {
        // Mark frame boundary
        backend.begin_frame();

        {
            profile_scope!("game_loop");

            // Simulate game update
            simulate_game_update(frame);

            // Simulate physics
            simulate_physics();

            // Simulate rendering
            simulate_rendering();
        }

        backend.end_frame();

        // Sleep to simulate 60 FPS target
        std::thread::sleep(std::time::Duration::from_millis(16));
    }

    println!("Simulation complete!");
    println!();
    println!("Check the Tracy profiler to see:");
    println!("  - Frame timeline with all scopes");
    println!("  - Categorized scopes (Physics, Rendering, etc.)");
    println!("  - Nested scope hierarchies");
    println!("  - Frame time statistics");
}

#[cfg(feature = "profiling-tracy")]
fn simulate_game_update(frame: u32) {
    profile_scope!("game_update", ProfileCategory::ECS);

    // Simulate entity updates
    {
        profile_scope!("entity_updates", ProfileCategory::ECS);
        std::thread::sleep(std::time::Duration::from_micros(100 + (frame % 50) as u64));
    }

    // Simulate component queries
    {
        profile_scope!("component_queries", ProfileCategory::ECS);
        std::thread::sleep(std::time::Duration::from_micros(50));
    }

    // Simulate AI updates (slower every 10 frames)
    if frame % 10 == 0 {
        profile_scope!("ai_updates", ProfileCategory::Scripts);
        std::thread::sleep(std::time::Duration::from_micros(500));
    }
}

#[cfg(feature = "profiling-tracy")]
fn simulate_physics() {
    profile_scope!("physics_step", ProfileCategory::Physics);

    // Simulate collision detection
    {
        profile_scope!("collision_detection", ProfileCategory::Physics);
        std::thread::sleep(std::time::Duration::from_micros(200));
    }

    // Simulate physics integration
    {
        profile_scope!("integration", ProfileCategory::Physics);
        std::thread::sleep(std::time::Duration::from_micros(150));

        // Simulate SIMD batch processing
        {
            profile_scope!("simd_batch_8", ProfileCategory::Physics);
            std::thread::sleep(std::time::Duration::from_micros(50));
        }
    }

    // Simulate constraint solving
    {
        profile_scope!("constraint_solver", ProfileCategory::Physics);
        std::thread::sleep(std::time::Duration::from_micros(100));
    }
}

#[cfg(feature = "profiling-tracy")]
fn simulate_rendering() {
    profile_scope!("render_frame", ProfileCategory::Rendering);

    // Simulate culling
    {
        profile_scope!("frustum_culling", ProfileCategory::Rendering);
        std::thread::sleep(std::time::Duration::from_micros(50));
    }

    // Simulate draw call preparation
    {
        profile_scope!("prepare_draw_calls", ProfileCategory::Rendering);
        std::thread::sleep(std::time::Duration::from_micros(100));
    }

    // Simulate GPU commands
    {
        profile_scope!("record_commands", ProfileCategory::Rendering);
        std::thread::sleep(std::time::Duration::from_micros(200));

        // Simulate command buffer recording
        for i in 0..5 {
            profile_scope!(&format!("command_buffer_{}", i), ProfileCategory::Rendering);
            std::thread::sleep(std::time::Duration::from_micros(20));
        }
    }

    // Simulate queue submission
    {
        profile_scope!("submit_queue", ProfileCategory::Rendering);
        std::thread::sleep(std::time::Duration::from_micros(50));
    }
}
