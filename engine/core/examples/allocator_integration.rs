//! Example demonstrating memory allocator integration with ECS
//!
//! This example shows how to use Arena, Pool, and Frame allocators to optimize
//! memory allocation patterns in game engine code. It demonstrates:
//!
//! - Arena allocator for temporary collections during frame processing
//! - Pool allocator for entity/component object pooling
//! - Frame allocator for per-frame temporary buffers
//! - Integration with ECS for realistic game scenarios
//!
//! Run this example:
//! ```bash
//! cargo run --example allocator_integration --release
//! ```

#![allow(dead_code)]

use engine_core::allocators::{Arena, FrameAllocator, PoolAllocator};
use engine_core::ecs::{Component, World};
use engine_core::Transform;
use std::time::Instant;

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
struct Particle {
    position: [f32; 3],
    velocity: [f32; 3],
    lifetime: f32,
}

/// Demonstrates Arena allocator for temporary collections
fn demo_arena_temporary_collections() {
    println!("=== Arena Allocator: Temporary Collections ===\n");

    let iterations = 1000;

    // Baseline: Using Vec for temporary data
    let start = Instant::now();
    for _frame in 0..iterations {
        let mut temp_transforms = Vec::with_capacity(100);
        for i in 0..100 {
            let mut t = Transform::default();
            t.position = engine_core::Vec3::new(i as f32, 0.0, 0.0);
            temp_transforms.push(t);
        }
        // Process transforms...
        std::hint::black_box(&temp_transforms);
    }
    let vec_time = start.elapsed();

    // Optimized: Using Arena allocator
    let start = Instant::now();
    let mut arena = Arena::with_chunk_size(64 * 1024); // 64KB chunks
    for _frame in 0..iterations {
        let temp_transforms = arena.alloc_slice::<Transform>(100);
        for i in 0..100 {
            temp_transforms[i] = Transform::default();
            temp_transforms[i].position = engine_core::Vec3::new(i as f32, 0.0, 0.0);
        }
        // Process transforms...
        std::hint::black_box(&temp_transforms);

        // Reset for next iteration (frame)
        arena.reset();
    }
    let arena_time = start.elapsed();

    println!("Iterations: {}", iterations);
    println!(
        "Vec allocation:   {:?} ({:.2} µs/iter)",
        vec_time,
        vec_time.as_micros() as f64 / iterations as f64
    );
    println!(
        "Arena allocation: {:?} ({:.2} µs/iter)",
        arena_time,
        arena_time.as_micros() as f64 / iterations as f64
    );
    let speedup = vec_time.as_micros() as f64 / arena_time.as_micros() as f64;
    println!("Speedup: {:.2}x faster\n", speedup);
    println!("Arena stats:");
    println!("  Capacity: {} bytes", arena.capacity());
    println!("  Used: {} bytes", arena.used());
    println!("  Efficiency: {:.1}%\n", arena.efficiency() * 100.0);
}

/// Demonstrates Pool allocator for object reuse
fn demo_pool_object_reuse() {
    println!("=== Pool Allocator: Object Reuse ===\n");

    let iterations = 1000;
    let objects_per_iteration = 50;

    // Baseline: Using Box allocations
    let start = Instant::now();
    for _iteration in 0..iterations {
        let mut objects = Vec::new();
        for i in 0..objects_per_iteration {
            objects.push(Box::new(Particle {
                position: [i as f32, 0.0, 0.0],
                velocity: [1.0, 0.0, 0.0],
                lifetime: 5.0,
            }));
        }
        // Use objects...
        std::hint::black_box(&objects);
        // Drop all objects
    }
    let box_time = start.elapsed();

    // Optimized: Using Pool allocator
    let start = Instant::now();
    let mut pool = PoolAllocator::<Particle>::with_capacity(objects_per_iteration);
    for _iteration in 0..iterations {
        let mut particles = Vec::new();
        for i in 0..objects_per_iteration {
            let particle = pool.alloc(Particle {
                position: [i as f32, 0.0, 0.0],
                velocity: [1.0, 0.0, 0.0],
                lifetime: 5.0,
            });
            particles.push(particle as *mut Particle);
        }
        // Use particles...
        std::hint::black_box(&particles);

        // Return to pool
        for &ptr in &particles {
            unsafe {
                pool.free(&mut *ptr);
            }
        }
    }
    let pool_time = start.elapsed();

    println!("Iterations: {}", iterations);
    println!("Objects per iteration: {}", objects_per_iteration);
    println!(
        "Box allocation:  {:?} ({:.2} µs/iter)",
        box_time,
        box_time.as_micros() as f64 / iterations as f64
    );
    println!(
        "Pool allocation: {:?} ({:.2} µs/iter)",
        pool_time,
        pool_time.as_micros() as f64 / iterations as f64
    );
    let speedup = box_time.as_micros() as f64 / pool_time.as_micros() as f64;
    println!("Speedup: {:.2}x faster\n", speedup);
    println!("Pool stats:");
    println!("  Capacity: {} objects", pool.capacity());
    println!("  Current usage: {} objects", pool.len());
    println!("  Utilization: {:.1}%\n", pool.utilization() * 100.0);
}

/// Demonstrates Frame allocator for per-frame buffers
fn demo_frame_per_frame_buffers() {
    println!("=== Frame Allocator: Per-Frame Buffers ===\n");

    let frame_count = 1000;

    // Baseline: Using Vec for per-frame data
    let start = Instant::now();
    for _frame in 0..frame_count {
        // Simulate various temporary buffers needed during frame
        let buffer1 = vec![0.0f32; 256]; // Render data
        let buffer2 = vec![0u32; 128]; // Index buffer
        let buffer3 = vec![Transform::default(); 64]; // Transform buffer

        std::hint::black_box((&buffer1, &buffer2, &buffer3));
    }
    let vec_time = start.elapsed();

    // Optimized: Using Frame allocator
    let start = Instant::now();
    let mut frame_alloc = FrameAllocator::with_capacity(1024 * 1024); // 1MB
    for _frame in 0..frame_count {
        // Allocate all frame data from single allocator
        {
            let buffer1 = frame_alloc.alloc_slice::<f32>(256);
            std::hint::black_box(buffer1);
        }
        {
            let buffer2 = frame_alloc.alloc_slice::<u32>(128);
            std::hint::black_box(buffer2);
        }
        {
            let buffer3 = frame_alloc.alloc_slice::<Transform>(64);
            std::hint::black_box(buffer3);
        }

        // Reset at end of frame - O(1) operation
        frame_alloc.reset();
    }
    let frame_time = start.elapsed();

    println!("Frame count: {}", frame_count);
    println!(
        "Vec allocation:   {:?} ({:.2} µs/frame)",
        vec_time,
        vec_time.as_micros() as f64 / frame_count as f64
    );
    println!(
        "Frame allocation: {:?} ({:.2} µs/frame)",
        frame_time,
        frame_time.as_micros() as f64 / frame_count as f64
    );
    let speedup = vec_time.as_micros() as f64 / frame_time.as_micros() as f64;
    println!("Speedup: {:.2}x faster\n", speedup);
    println!("Frame allocator stats:");
    println!("  Capacity: {} bytes", frame_alloc.capacity());
    println!("  Peak usage: {} bytes", frame_alloc.peak_used());
    println!("  Peak utilization: {:.1}%\n", frame_alloc.peak_utilization() * 100.0);
}

/// Demonstrates integration with ECS for realistic game scenario
fn demo_ecs_integration() {
    println!("=== ECS Integration: Complete Game Loop ===\n");

    let entity_count = 1000;
    let frame_count = 100;

    println!("Setup: {} entities, {} frames\n", entity_count, frame_count);

    // Baseline: Standard Vec-based approach
    let start = Instant::now();
    {
        let mut world = World::new();
        world.register::<Position>();
        world.register::<Velocity>();

        // Spawn entities
        for i in 0..entity_count {
            let entity = world.spawn();
            world.add(entity, Position { x: i as f32, y: 0.0, z: 0.0 });
            world.add(entity, Velocity { x: 1.0, y: 0.0, z: 0.0 });
        }

        // Simulate frames
        for _frame in 0..frame_count {
            // Collect entities to update (simulate query results)
            let mut temp_data = Vec::with_capacity(entity_count);
            for i in 0..entity_count {
                temp_data.push((i, Position { x: i as f32, y: 0.0, z: 0.0 }));
            }

            // Process
            std::hint::black_box(&temp_data);
        }
    }
    let baseline_time = start.elapsed();

    // Optimized: Using Arena for temporary collections
    let start = Instant::now();
    {
        let mut world = World::new();
        world.register::<Position>();
        world.register::<Velocity>();
        let mut arena = Arena::with_chunk_size(64 * 1024);

        // Spawn entities
        for i in 0..entity_count {
            let entity = world.spawn();
            world.add(entity, Position { x: i as f32, y: 0.0, z: 0.0 });
            world.add(entity, Velocity { x: 1.0, y: 0.0, z: 0.0 });
        }

        // Simulate frames
        for _frame in 0..frame_count {
            // Use arena for temporary query results
            let temp_data = arena.alloc_slice::<(usize, Position)>(entity_count);
            for i in 0..entity_count {
                temp_data[i] = (i, Position { x: i as f32, y: 0.0, z: 0.0 });
            }

            // Process
            std::hint::black_box(temp_data);

            // Reset arena for next frame
            arena.reset();
        }
    }
    let optimized_time = start.elapsed();

    println!(
        "Baseline (Vec):         {:?} ({:.2} ms/frame)",
        baseline_time,
        baseline_time.as_millis() as f64 / frame_count as f64
    );
    println!(
        "Optimized (Arena):      {:?} ({:.2} ms/frame)",
        optimized_time,
        optimized_time.as_millis() as f64 / frame_count as f64
    );
    let speedup = baseline_time.as_micros() as f64 / optimized_time.as_micros() as f64;
    println!("Speedup: {:.2}x faster\n", speedup);
}

/// Demonstrates combined usage of all allocators
fn demo_combined_allocators() {
    println!("=== Combined Allocators: Real-World Scenario ===\n");

    let frame_count = 100;

    let start = Instant::now();
    {
        // Setup allocators for different purposes
        let mut arena = Arena::with_chunk_size(128 * 1024); // For temporary collections
        let mut frame_alloc = FrameAllocator::with_capacity(512 * 1024); // For per-frame buffers
        let mut particle_pool = PoolAllocator::<Particle>::with_capacity(500); // For particles

        // Simulate game loop
        for frame in 0..frame_count {
            // 1. Use frame allocator for immediate temporary data
            {
                let render_data = frame_alloc.alloc_slice::<f32>(256);
                std::hint::black_box(render_data);
            }

            // 2. Use arena for temporary collections that live during system processing
            {
                let query_results = arena.alloc_slice::<Position>(100);
                std::hint::black_box(query_results);
            }

            // 3. Spawn some particles in pool
            if frame % 10 == 0 {
                for i in 0..10 {
                    let _particle = particle_pool.alloc(Particle {
                        position: [i as f32, frame as f32, 0.0],
                        velocity: [1.0, -1.0, 0.0],
                        lifetime: 5.0,
                    });
                }
            }

            // Process frame...
            std::hint::black_box(frame);

            // Cleanup at end of frame
            frame_alloc.reset(); // O(1) - just resets pointer
            arena.reset(); // O(1) - reuses chunks
        }

        // Pool particles would be freed explicitly in real code
        particle_pool.clear();
    }
    let total_time = start.elapsed();

    println!("Frames: {}", frame_count);
    println!(
        "Total time: {:?} ({:.2} ms/frame)",
        total_time,
        total_time.as_millis() as f64 / frame_count as f64
    );
    println!("\nThis demonstrates:");
    println!("  • Arena: Temporary collections during system processing");
    println!("  • Frame: Per-frame immediate buffers (render, audio, etc.)");
    println!("  • Pool: Long-lived objects with frequent allocation/deallocation\n");
}

fn main() {
    println!("╔═══════════════════════════════════════════════════════════╗");
    println!("║    Memory Allocator Integration Example                  ║");
    println!("║    Demonstrating Arena, Pool, and Frame allocators        ║");
    println!("╚═══════════════════════════════════════════════════════════╝\n");

    demo_arena_temporary_collections();
    println!("────────────────────────────────────────────────────────────\n");

    demo_pool_object_reuse();
    println!("────────────────────────────────────────────────────────────\n");

    demo_frame_per_frame_buffers();
    println!("────────────────────────────────────────────────────────────\n");

    demo_ecs_integration();
    println!("────────────────────────────────────────────────────────────\n");

    demo_combined_allocators();
    println!("────────────────────────────────────────────────────────────\n");

    println!("╔═══════════════════════════════════════════════════════════╗");
    println!("║                    Key Takeaways                          ║");
    println!("╠═══════════════════════════════════════════════════════════╣");
    println!("║                                                           ║");
    println!("║  When to use Arena:                                       ║");
    println!("║    • Temporary collections during frame processing        ║");
    println!("║    • Query result caching                                 ║");
    println!("║    • Intermediate computation buffers                     ║");
    println!("║    • String formatting and temporary strings              ║");
    println!("║                                                           ║");
    println!("║  When to use Pool:                                        ║");
    println!("║    • Frequently allocated/deallocated objects             ║");
    println!("║    • Particles, projectiles, effects                      ║");
    println!("║    • Entity allocation (if not using ECS allocator)       ║");
    println!("║    • Component pools for specific types                   ║");
    println!("║                                                           ║");
    println!("║  When to use Frame:                                       ║");
    println!("║    • Data that lives exactly one frame                    ║");
    println!("║    • Render command buffers                               ║");
    println!("║    • Audio mixing buffers                                 ║");
    println!("║    • Debug visualization data                             ║");
    println!("║                                                           ║");
    println!("║  Performance gains: 5-15% in allocation-heavy code        ║");
    println!("║  Fragmentation: Zero for Arena and Frame allocators       ║");
    println!("║  Cache efficiency: Significantly improved (contiguous)    ║");
    println!("║                                                           ║");
    println!("╚═══════════════════════════════════════════════════════════╝\n");

    println!("✓ Example completed successfully");
}
