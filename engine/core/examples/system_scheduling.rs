//! Example demonstrating automatic system scheduling and parallelization
//!
//! This example shows how the scheduling system:
//! - Automatically detects dependencies between systems
//! - Parallelizes independent systems
//! - Ensures correct execution order
//!
//! Run with: cargo run --example system_scheduling

use engine_core::ecs::{Component, Schedule, System, SystemAccess, World};
use std::time::Instant;

// ============================================================================
// Components
// ============================================================================

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
struct Transform {
    x: f32,
    y: f32,
    z: f32,
}

impl Component for Transform {}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
struct Velocity {
    x: f32,
    y: f32,
    z: f32,
}

impl Component for Velocity {}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
struct Health {
    current: f32,
    max: f32,
}

impl Component for Health {}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
struct Damage {
    amount: f32,
}

impl Component for Damage {}

// ============================================================================
// Systems
// ============================================================================

/// Physics system - updates Transform based on Velocity
struct PhysicsSystem;

impl System for PhysicsSystem {
    fn name(&self) -> &str {
        "PhysicsSystem"
    }

    fn run(&mut self, _world: &mut World) {
        println!("  [PhysicsSystem] Running...");

        // In a real implementation, we would query the world:
        // for (transform, velocity) in world.query::<(&mut Transform, &Velocity)>() {
        //     transform.x += velocity.x;
        //     transform.y += velocity.y;
        //     transform.z += velocity.z;
        // }

        std::thread::sleep(std::time::Duration::from_millis(10));
        println!("  [PhysicsSystem] Complete");
    }

    fn access(&self) -> SystemAccess {
        SystemAccess::new()
            .reads::<Velocity>()
            .writes::<Transform>()
    }
}

/// Rendering system - reads Transform to render entities
struct RenderingSystem;

impl System for RenderingSystem {
    fn name(&self) -> &str {
        "RenderingSystem"
    }

    fn run(&mut self, _world: &mut World) {
        println!("  [RenderingSystem] Running...");

        // In a real implementation:
        // for (entity, transform) in world.query::<&Transform>() {
        //     render_entity(entity, transform);
        // }

        std::thread::sleep(std::time::Duration::from_millis(10));
        println!("  [RenderingSystem] Complete");
    }

    fn access(&self) -> SystemAccess {
        SystemAccess::new().reads::<Transform>()
    }
}

/// AI system - updates Velocity based on game logic
struct AISystem;

impl System for AISystem {
    fn name(&self) -> &str {
        "AISystem"
    }

    fn run(&mut self, _world: &mut World) {
        println!("  [AISystem] Running...");

        // In a real implementation:
        // for (entity, velocity) in world.query::<&mut Velocity>() {
        //     // Update velocity based on AI decisions
        // }

        std::thread::sleep(std::time::Duration::from_millis(10));
        println!("  [AISystem] Complete");
    }

    fn access(&self) -> SystemAccess {
        SystemAccess::new().writes::<Velocity>()
    }
}

/// Damage system - applies damage to health
struct DamageSystem;

impl System for DamageSystem {
    fn name(&self) -> &str {
        "DamageSystem"
    }

    fn run(&mut self, _world: &mut World) {
        println!("  [DamageSystem] Running...");

        // In a real implementation:
        // for (health, damage) in world.query::<(&mut Health, &Damage)>() {
        //     health.current -= damage.amount;
        // }

        std::thread::sleep(std::time::Duration::from_millis(10));
        println!("  [DamageSystem] Complete");
    }

    fn access(&self) -> SystemAccess {
        SystemAccess::new()
            .reads::<Damage>()
            .writes::<Health>()
    }
}

/// Death system - removes entities with zero health
struct DeathSystem;

impl System for DeathSystem {
    fn name(&self) -> &str {
        "DeathSystem"
    }

    fn run(&mut self, _world: &mut World) {
        println!("  [DeathSystem] Running...");

        // In a real implementation:
        // let to_remove: Vec<Entity> = world.query::<(Entity, &Health)>()
        //     .filter(|(_, health)| health.current <= 0.0)
        //     .map(|(entity, _)| entity)
        //     .collect();
        //
        // for entity in to_remove {
        //     world.despawn(entity);
        // }

        std::thread::sleep(std::time::Duration::from_millis(10));
        println!("  [DeathSystem] Complete");
    }

    fn access(&self) -> SystemAccess {
        SystemAccess::new().reads::<Health>()
    }
}

// ============================================================================
// Main Example
// ============================================================================

fn main() {
    println!("=== System Scheduling Example ===\n");

    // Create a schedule
    let mut schedule = Schedule::new();

    println!("Adding systems to schedule...");
    schedule.add_system(PhysicsSystem);
    schedule.add_system(RenderingSystem);
    schedule.add_system(AISystem);
    schedule.add_system(DamageSystem);
    schedule.add_system(DeathSystem);

    println!("\nBuilding schedule (analyzing dependencies)...");
    schedule.build();

    println!("\nExecution plan:");
    println!("{}", schedule.debug_info());

    println!("\nExpected parallel execution:");
    println!("  Stage 0: AISystem, DamageSystem (parallel - no conflicts)");
    println!("  Stage 1: PhysicsSystem, DeathSystem (parallel - no conflicts)");
    println!("  Stage 2: RenderingSystem (depends on Transform from Physics)");

    // Create world
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Velocity>();
    world.register::<Health>();
    world.register::<Damage>();

    // Spawn some entities
    for i in 0..5 {
        let entity = world.spawn();
        world.add(
            entity,
            Transform {
                x: i as f32,
                y: 0.0,
                z: 0.0,
            },
        );
        world.add(
            entity,
            Velocity {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
        );
        world.add(
            entity,
            Health {
                current: 100.0,
                max: 100.0,
            },
        );
    }

    println!("\n--- Running schedule ---");
    let start = Instant::now();
    schedule.run(&mut world);
    let elapsed = start.elapsed();

    println!("\n--- Schedule complete ---");
    println!("Total execution time: {:?}", elapsed);
    println!("\nNote: Sequential execution would take ~50ms (5 systems * 10ms)");
    println!("      Parallel execution should take ~30ms (3 stages * 10ms)");
    println!("      Actual time includes scheduling overhead");

    // Demonstrate performance comparison
    println!("\n=== Performance Comparison ===");

    println!("\nSequential execution:");
    let start = Instant::now();
    let mut world_seq = World::new();
    world_seq.register::<Transform>();
    world_seq.register::<Velocity>();
    world_seq.register::<Health>();
    world_seq.register::<Damage>();

    // Run systems sequentially
    let mut ai = AISystem;
    let mut damage = DamageSystem;
    let mut physics = PhysicsSystem;
    let mut death = DeathSystem;
    let mut render = RenderingSystem;

    println!("Stage 0 (sequential):");
    ai.run(&mut world_seq);
    damage.run(&mut world_seq);
    println!("Stage 1 (sequential):");
    physics.run(&mut world_seq);
    death.run(&mut world_seq);
    println!("Stage 2 (sequential):");
    render.run(&mut world_seq);

    let elapsed_seq = start.elapsed();
    println!("Sequential time: {:?}", elapsed_seq);

    println!("\nScheduled execution:");
    let start = Instant::now();
    let mut world_sched = World::new();
    world_sched.register::<Transform>();
    world_sched.register::<Velocity>();
    world_sched.register::<Health>();
    world_sched.register::<Damage>();

    let mut schedule2 = Schedule::new();
    schedule2.add_system(PhysicsSystem);
    schedule2.add_system(RenderingSystem);
    schedule2.add_system(AISystem);
    schedule2.add_system(DamageSystem);
    schedule2.add_system(DeathSystem);
    schedule2.build();
    schedule2.run(&mut world_sched);

    let elapsed_sched = start.elapsed();
    println!("Scheduled time: {:?}", elapsed_sched);

    if elapsed_seq > elapsed_sched {
        let speedup = elapsed_seq.as_secs_f64() / elapsed_sched.as_secs_f64();
        println!("\nSpeedup: {:.2}x faster with scheduling!", speedup);
    } else {
        println!("\nNote: Speedup may not be visible with such short systems.");
        println!("In real games with heavier systems, parallelization provides significant benefits.");
    }

    println!("\n=== Example Complete ===");
}
