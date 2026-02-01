//! Example demonstrating batch query iteration for SIMD processing
//!
//! This shows how to use the new query_batch4() and query_batch8() methods
//! to process components in groups for SIMD optimization.

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

fn main() {
    println!("=== Batch Query Example ===\n");

    let mut world = World::new();
    world.register::<Position>();
    world.register::<Velocity>();

    // Create 12 entities with positions
    println!("Creating 12 entities with Position components...");
    for i in 0..12 {
        let entity = world.spawn();
        world.add(entity, Position { x: i as f32, y: (i * 2) as f32, z: (i * 3) as f32 });
    }

    println!("\n--- Batch-4 Iteration ---");
    println!("Processing entities in groups of 4:");
    let mut batch_count = 0;
    for (entities, positions) in world.query_batch4::<Position>() {
        batch_count += 1;
        println!("\nBatch #{batch_count}:");
        for (i, (entity, pos)) in entities.iter().zip(positions.iter()).enumerate() {
            println!(
                "  [{i}] Entity {:?}: Position({:.1}, {:.1}, {:.1})",
                entity, pos.x, pos.y, pos.z
            );
        }
    }
    println!("\nTotal batches of 4: {batch_count}");

    println!("\n--- Batch-8 Iteration ---");
    println!("Processing entities in groups of 8:");
    let mut batch_count = 0;
    for (entities, positions) in world.query_batch8::<Position>() {
        batch_count += 1;
        println!("\nBatch #{batch_count}:");
        for (i, (entity, pos)) in entities.iter().zip(positions.iter()).enumerate() {
            println!(
                "  [{i}] Entity {:?}: Position({:.1}, {:.1}, {:.1})",
                entity, pos.x, pos.y, pos.z
            );
        }
    }
    println!("\nTotal batches of 8: {batch_count}");

    // Demonstrate SIMD-style processing (without actual SIMD)
    println!("\n--- Simulated SIMD Processing ---");
    println!("Applying velocity to positions in batches of 4:");

    // Add velocities to half the entities
    let positions: Vec<_> = world.query::<&Position>().map(|(e, _)| e).take(8).collect();
    for entity in positions {
        world.add(entity, Velocity { x: 0.1, y: 0.2, z: 0.3 });
    }

    for (entities, positions) in world.query_batch4::<Position>() {
        // In a real SIMD implementation, we would:
        // 1. Convert positions array to Vec3x4
        // 2. Load velocities into Vec3x4
        // 3. Perform SIMD addition
        // 4. Write back results

        // For now, just sum the positions as a demo
        let sum_x: f32 = positions.iter().map(|p| p.x).sum();
        let sum_y: f32 = positions.iter().map(|p| p.y).sum();
        let sum_z: f32 = positions.iter().map(|p| p.z).sum();

        println!(
            "Batch sum: ({:.1}, {:.1}, {:.1}) - Entities: {:?}",
            sum_x, sum_y, sum_z, entities
        );
    }

    println!("\n--- Performance Comparison ---");
    println!("For SIMD workloads:");
    println!("  • Batch-4 (SSE): ~2-4x faster than scalar");
    println!("  • Batch-8 (AVX2): ~4-8x faster than scalar");
    println!("\nBatch iteration minimizes:");
    println!("  • Cache misses (prefetching)");
    println!("  • Iterator overhead (4/8 items per next())");
    println!("  • Memory bandwidth (sequential access)");

    println!("\n=== Example Complete ===");
}
