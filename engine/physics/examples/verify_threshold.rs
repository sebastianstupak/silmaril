//! Simple example to verify the parallel threshold is working correctly.
//!
//! This runs a quick performance comparison to validate that:
//! 1. Sequential is faster below 2,000 entities
//! 2. Parallel is faster above 2,000 entities
//! 3. The threshold of 2,000 is well-chosen

use engine_core::math::Transform;
use engine_math::Vec3;
use engine_physics::systems::integration_simd::{process_parallel, process_sequential};
use std::time::Instant;

fn main() {
    println!("=== Parallel Threshold Verification ===");
    println!("Testing threshold optimization at different entity counts");
    println!();

    // Warm up Rayon's thread pool with substantial work
    println!("Warming up Rayon thread pool...");
    {
        let mut warmup = vec![Transform::identity(); 50_000];
        let warmup_vel = vec![Vec3::new(1.0, 2.0, 3.0); 50_000];
        for _ in 0..20 {
            process_parallel(&mut warmup, &warmup_vel, 0.016);
        }
    }
    println!("Thread pool ready.\n");

    let test_configs = vec![
        ("Below threshold", 1_000),
        ("Below threshold", 1_500),
        ("At threshold", 2_000),
        ("Above threshold", 3_000),
        ("Above threshold", 5_000),
        ("Above threshold", 10_000),
        ("Well above threshold", 20_000),
    ];

    for (category, count) in test_configs {
        println!("Testing {} entities ({}):", count, category);

        let mut transforms_seq = vec![Transform::identity(); count];
        let mut transforms_par = vec![Transform::identity(); count];
        let velocities = vec![Vec3::new(1.0, 2.0, 3.0); count];

        // Warm up (50 iterations each to get stable timings)
        for _ in 0..50 {
            process_sequential(&mut transforms_seq, &velocities, 0.016);
            process_parallel(&mut transforms_par, &velocities, 0.016);
        }

        // Measure sequential (500 iterations for more stable results)
        let iterations = 500;
        let start = Instant::now();
        for _ in 0..iterations {
            process_sequential(&mut transforms_seq, &velocities, 0.016);
        }
        let seq_time = start.elapsed();

        // Measure parallel (500 iterations)
        let start = Instant::now();
        for _ in 0..iterations {
            process_parallel(&mut transforms_par, &velocities, 0.016);
        }
        let par_time = start.elapsed();

        let seq_us = seq_time.as_micros() as f64 / iterations as f64;
        let par_us = par_time.as_micros() as f64 / iterations as f64;
        let speedup = seq_time.as_nanos() as f64 / par_time.as_nanos() as f64;

        let recommendation = if speedup > 1.1 {
            "✓ Use parallel"
        } else if speedup < 0.9 {
            "✓ Use sequential"
        } else {
            "~ Similar performance"
        };

        println!(
            "  Sequential: {:.2}μs  |  Parallel: {:.2}μs  |  Speedup: {:.2}x  |  {}",
            seq_us, par_us, speedup, recommendation
        );

        // Verify correctness
        for (t1, t2) in transforms_seq.iter().zip(transforms_par.iter()) {
            assert!(
                (t1.position.x - t2.position.x).abs() < 1e-6,
                "Sequential and parallel produce different results!"
            );
        }
    }

    println!();
    println!("=== Threshold Validation Complete ===");
    println!();
    println!("Expected behavior:");
    println!("  • Below 2,000: Sequential should be similar or faster");
    println!("  • At/above 2,000: Parallel should show increasing speedup");
    println!();
    println!("Current PARALLEL_THRESHOLD = 2,000");
    println!("This means:");
    println!("  • < 2,000 entities: Uses sequential SIMD processing");
    println!("  • ≥ 2,000 entities: Uses parallel processing with Rayon");
}
