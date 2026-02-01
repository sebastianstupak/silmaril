//! Demonstrates the physics integration system with performance comparison.

use engine_core::ecs::World;
use engine_core::math::Transform;
use engine_physics::components::Velocity;
use engine_physics::systems::{
    integration::physics_integration_system, integration_simd::physics_integration_system_simd,
};
use std::time::Instant;

fn main() {
    println!("Physics Integration System Demo");
    println!("================================\n");

    // Test various entity counts
    for entity_count in [100, 1_000, 10_000, 50_000] {
        println!("Testing with {} entities:", entity_count);

        // Test scalar version
        let mut world_scalar = create_world(entity_count);
        let start = Instant::now();
        for _ in 0..100 {
            physics_integration_system(&mut world_scalar, 0.016);
        }
        let scalar_time = start.elapsed();
        println!("  Scalar:      {:?} (100 iterations)", scalar_time);

        // Test SIMD version
        let mut world_simd = create_world(entity_count);
        let start = Instant::now();
        for _ in 0..100 {
            physics_integration_system_simd(&mut world_simd, 0.016);
        }
        let simd_time = start.elapsed();
        println!("  SIMD:        {:?} (100 iterations)", simd_time);

        // Calculate speedup
        let speedup = scalar_time.as_secs_f64() / simd_time.as_secs_f64();
        println!("  Speedup:     {:.2}x faster\n", speedup);

        if speedup < 1.5 {
            println!("  ⚠️  Warning: Speedup lower than expected (target: 2-4x)");
            println!("      Try compiling with: RUSTFLAGS=\"-C target-cpu=native\"\n");
        } else if speedup >= 3.0 {
            println!("  ✓  Excellent speedup achieved!\n");
        } else {
            println!("  ✓  Good speedup achieved!\n");
        }
    }

    println!("\nDemo complete!");
    println!("\nNotes:");
    println!("  - For best performance, compile with: RUSTFLAGS=\"-C target-cpu=native\"");
    println!("  - Speedup increases with entity count (3-4x for 10k+ entities)");
    println!("  - Parallel processing activates at 10,000+ entities");
}

fn create_world(entity_count: usize) -> World {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Velocity>();

    for i in 0..entity_count {
        let entity = world.spawn();
        world.add(entity, Transform::identity());
        world.add(entity, Velocity::new(i as f32 * 0.1, i as f32 * 0.2, i as f32 * 0.3));
    }

    world
}
