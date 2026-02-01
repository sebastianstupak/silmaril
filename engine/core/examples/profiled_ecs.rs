//! Example demonstrating profiled ECS operations
//!
//! This example shows how to use the profiling infrastructure with engine-core.
//! It demonstrates:
//! - Entity spawning and despawning with profiling
//! - Component add/remove operations with profiling
//! - How to extract and analyze profiling metrics
//!
//! Run with profiling enabled:
//! ```bash
//! cargo run --example profiled_ecs --features profiling
//! ```
//!
//! Run without profiling (zero overhead):
//! ```bash
//! cargo run --example profiled_ecs
//! ```

#![allow(dead_code)]

use engine_core::ecs::{Component, World};

#[derive(Debug, Clone, Copy)]
struct Position {
    x: f32,
    y: f32,
    z: f32,
}

impl Component for Position {}

#[derive(Debug, Clone, Copy)]
struct Velocity {
    x: f32,
    y: f32,
    z: f32,
}

impl Component for Velocity {}

#[derive(Debug, Clone, Copy)]
struct Health {
    current: f32,
    max: f32,
}

impl Component for Health {}

fn main() {
    println!("=== Profiled ECS Example ===\n");

    #[cfg(feature = "profiling")]
    {
        println!("✓ Profiling ENABLED");
        println!("  Profiling scopes will be captured\n");

        // Initialize Puffin profiler
        puffin::set_scopes_on(true);
    }

    #[cfg(not(feature = "profiling"))]
    {
        println!("✗ Profiling DISABLED");
        println!("  Zero overhead - profiling macros compile to nothing\n");
    }

    // Create world and register components
    let mut world = World::new();
    world.register::<Position>();
    world.register::<Velocity>();
    world.register::<Health>();

    println!("Spawning 10,000 entities with components...");

    let start = std::time::Instant::now();

    // Spawn entities with multiple components
    let mut entities = Vec::new();
    for i in 0..10_000 {
        let entity = world.spawn();

        world.add(entity, Position { x: i as f32, y: (i * 2) as f32, z: (i * 3) as f32 });

        world.add(entity, Velocity { x: 1.0, y: 0.0, z: -1.0 });

        if i % 2 == 0 {
            world.add(entity, Health { current: 100.0, max: 100.0 });
        }

        entities.push(entity);
    }

    let spawn_time = start.elapsed();
    println!("✓ Spawned in {:?}", spawn_time);

    println!("\nModifying components...");

    let start = std::time::Instant::now();

    // Remove some components
    for (i, &entity) in entities.iter().enumerate() {
        if i % 3 == 0 {
            world.remove::<Velocity>(entity);
        }
    }

    let modify_time = start.elapsed();
    println!("✓ Modified in {:?}", modify_time);

    println!("\nDespawning entities...");

    let start = std::time::Instant::now();

    // Despawn all entities
    for &entity in &entities {
        world.despawn(entity);
    }

    let despawn_time = start.elapsed();
    println!("✓ Despawned in {:?}", despawn_time);

    println!("\n=== Performance Summary ===");
    println!("Total entities: 10,000");
    println!(
        "Spawn time:     {:?} ({:.2} µs/entity)",
        spawn_time,
        spawn_time.as_micros() as f64 / 10_000.0
    );
    println!(
        "Modify time:    {:?} ({:.2} µs/operation)",
        modify_time,
        modify_time.as_micros() as f64 / 3_333.0
    );
    println!(
        "Despawn time:   {:?} ({:.2} µs/entity)",
        despawn_time,
        despawn_time.as_micros() as f64 / 10_000.0
    );

    #[cfg(feature = "profiling")]
    {
        println!("\n=== Profiling Data ===");
        println!("Profiling scopes were captured for:");
        println!("  - entity_spawn (10,000 calls)");
        println!("  - component_add (25,000 calls)");
        println!("  - component_remove (3,333 calls)");
        println!("  - entity_despawn (10,000 calls)");
        println!("\nTo view profiling data:");
        println!("1. Connect with Puffin viewer (cargo install puffin_viewer)");
        println!("2. Or export to Chrome Tracing format");
        println!("3. Or query programmatically with the profiling API");
    }

    #[cfg(feature = "metrics")]
    {
        use agent_game_engine_profiling::{Profiler, ProfilerConfig};

        println!("\n=== Metrics Example ===");

        let profiler = Profiler::new(ProfilerConfig::default());

        // Simulate a game frame
        profiler.begin_frame();

        // Do some work
        let mut world2 = World::new();
        world2.register::<Position>();

        for i in 0..1000 {
            let entity = world2.spawn();
            world2.add(entity, Position { x: i as f32, y: 0.0, z: 0.0 });
        }

        let metrics = profiler.end_frame();

        println!("Frame time: {:.3}ms", metrics.frame_time_ms);
        println!("FPS: {:.1}", metrics.fps);
        println!("Memory: {}MB", metrics.memory_mb);
        println!("Entity count: {}", metrics.entity_count);
    }

    println!("\n✓ Example completed successfully");
}
