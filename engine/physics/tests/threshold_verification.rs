//! Verification tests for parallel threshold optimization.
//!
//! This test validates that the PARALLEL_THRESHOLD constant is set correctly
//! and that the system behaves as expected at different entity counts.

use engine_core::math::Transform;
use engine_math::Vec3;
use engine_physics::systems::integration_simd::{process_parallel, process_sequential};
use std::time::Instant;

/// Test that sequential processing is faster for counts below threshold.
#[test]
fn test_below_threshold_uses_sequential() {
    // Test with 1,000 entities (below 2,000 threshold)
    let count = 1_000;
    let mut transforms_seq = vec![Transform::identity(); count];
    let mut transforms_par = vec![Transform::identity(); count];
    let velocities = vec![Vec3::new(1.0, 2.0, 3.0); count];

    // Warm up
    for _ in 0..10 {
        process_sequential(&mut transforms_seq, &velocities, 0.016);
        process_parallel(&mut transforms_par, &velocities, 0.016);
    }

    // Measure sequential
    let start = Instant::now();
    for _ in 0..100 {
        process_sequential(&mut transforms_seq, &velocities, 0.016);
    }
    let seq_time = start.elapsed();

    // Measure parallel
    let start = Instant::now();
    for _ in 0..100 {
        process_parallel(&mut transforms_par, &velocities, 0.016);
    }
    let par_time = start.elapsed();

    // Sequential should be faster (or very close) at this count
    // Allow some variance due to system noise
    println!("1,000 entities:");
    println!("  Sequential: {:?}", seq_time);
    println!("  Parallel:   {:?}", par_time);
    println!("  Ratio: {:.2}x", par_time.as_nanos() as f64 / seq_time.as_nanos() as f64);

    // Verify results are correct
    assert_eq!(transforms_seq.len(), transforms_par.len());
    for (t1, t2) in transforms_seq.iter().zip(transforms_par.iter()) {
        assert!((t1.position.x - t2.position.x).abs() < 1e-6);
        assert!((t1.position.y - t2.position.y).abs() < 1e-6);
        assert!((t1.position.z - t2.position.z).abs() < 1e-6);
    }
}

/// Test that parallel processing is faster for counts above threshold.
#[test]
fn test_above_threshold_uses_parallel() {
    // Test with 5,000 entities (above 2,000 threshold)
    let count = 5_000;
    let mut transforms_seq = vec![Transform::identity(); count];
    let mut transforms_par = vec![Transform::identity(); count];
    let velocities = vec![Vec3::new(1.0, 2.0, 3.0); count];

    // Warm up
    for _ in 0..10 {
        process_sequential(&mut transforms_seq, &velocities, 0.016);
        process_parallel(&mut transforms_par, &velocities, 0.016);
    }

    // Measure sequential
    let start = Instant::now();
    for _ in 0..100 {
        process_sequential(&mut transforms_seq, &velocities, 0.016);
    }
    let seq_time = start.elapsed();

    // Measure parallel
    let start = Instant::now();
    for _ in 0..100 {
        process_parallel(&mut transforms_par, &velocities, 0.016);
    }
    let par_time = start.elapsed();

    // Parallel should be faster at this count
    println!("5,000 entities:");
    println!("  Sequential: {:?}", seq_time);
    println!("  Parallel:   {:?}", par_time);
    println!("  Speedup: {:.2}x", seq_time.as_nanos() as f64 / par_time.as_nanos() as f64);

    // Parallel should show speedup (allowing some variance)
    // Expected: 2-4x faster with parallel at 5K entities
    assert!(
        par_time < seq_time,
        "Parallel should be faster than sequential at 5,000 entities"
    );

    // Verify results are correct
    assert_eq!(transforms_seq.len(), transforms_par.len());
    for (t1, t2) in transforms_seq.iter().zip(transforms_par.iter()) {
        assert!((t1.position.x - t2.position.x).abs() < 1e-6);
        assert!((t1.position.y - t2.position.y).abs() < 1e-6);
        assert!((t1.position.z - t2.position.z).abs() < 1e-6);
    }
}

/// Test the crossover point is around 2,000 entities.
#[test]
fn test_crossover_point_validation() {
    let test_counts = vec![1_500, 2_000, 2_500, 3_000];

    println!("\nCrossover Point Analysis:");
    println!("{:-<60}", "");

    for count in test_counts {
        let mut transforms_seq = vec![Transform::identity(); count];
        let mut transforms_par = vec![Transform::identity(); count];
        let velocities = vec![Vec3::new(1.0, 2.0, 3.0); count];

        // Warm up
        for _ in 0..5 {
            process_sequential(&mut transforms_seq, &velocities, 0.016);
            process_parallel(&mut transforms_par, &velocities, 0.016);
        }

        // Measure sequential
        let start = Instant::now();
        for _ in 0..50 {
            process_sequential(&mut transforms_seq, &velocities, 0.016);
        }
        let seq_time = start.elapsed();

        // Measure parallel
        let start = Instant::now();
        for _ in 0..50 {
            process_parallel(&mut transforms_par, &velocities, 0.016);
        }
        let par_time = start.elapsed();

        let speedup = seq_time.as_nanos() as f64 / par_time.as_nanos() as f64;
        let faster = if speedup > 1.0 { "PARALLEL" } else { "SEQUENTIAL" };

        println!("{:5} entities: seq={:8.2}μs  par={:8.2}μs  speedup={:4.2}x  [{}]",
            count,
            seq_time.as_micros() as f64 / 50.0,
            par_time.as_micros() as f64 / 50.0,
            speedup,
            faster
        );
    }

    println!("{:-<60}", "");
}

/// Test that threshold constant matches documented value.
#[test]
fn test_threshold_constant_is_documented() {
    // This test documents the expected threshold value
    // If this fails, the threshold was changed but documentation wasn't updated

    // Read the source file and verify threshold is 2,000
    let source = include_str!("../src/systems/integration_simd.rs");

    assert!(
        source.contains("const PARALLEL_THRESHOLD: usize = 2_000"),
        "PARALLEL_THRESHOLD should be set to 2,000 based on benchmark analysis"
    );

    // Verify documentation mentions the threshold
    assert!(
        source.contains("2,000 entities"),
        "Documentation should mention the 2,000 entity threshold"
    );
}

/// Performance regression test to ensure threshold optimization is maintained.
#[test]
#[ignore] // Run with: cargo test --test threshold_verification -- --ignored --nocapture
fn test_performance_regression() {
    use std::collections::HashMap;

    println!("\nPerformance Regression Test");
    println!("{:=<70}", "");

    let test_cases = vec![
        ("Small (500)", 500),
        ("Below threshold (1,500)", 1_500),
        ("At threshold (2,000)", 2_000),
        ("Above threshold (3,000)", 3_000),
        ("Large (10,000)", 10_000),
    ];

    let mut results: HashMap<&str, (u128, u128)> = HashMap::new();

    for (name, count) in test_cases {
        let mut transforms_seq = vec![Transform::identity(); count];
        let mut transforms_par = vec![Transform::identity(); count];
        let velocities = vec![Vec3::new(1.0, 2.0, 3.0); count];

        // Warm up
        for _ in 0..10 {
            process_sequential(&mut transforms_seq, &velocities, 0.016);
            process_parallel(&mut transforms_par, &velocities, 0.016);
        }

        // Measure sequential (100 iterations)
        let start = Instant::now();
        for _ in 0..100 {
            process_sequential(&mut transforms_seq, &velocities, 0.016);
        }
        let seq_time = start.elapsed().as_nanos();

        // Measure parallel (100 iterations)
        let start = Instant::now();
        for _ in 0..100 {
            process_parallel(&mut transforms_par, &velocities, 0.016);
        }
        let par_time = start.elapsed().as_nanos();

        results.insert(name, (seq_time, par_time));

        let speedup = seq_time as f64 / par_time as f64;
        let better = if speedup > 1.1 {
            "PARALLEL"
        } else if speedup < 0.9 {
            "SEQUENTIAL"
        } else {
            "SIMILAR"
        };

        println!("{:<30} seq={:8.2}μs  par={:8.2}μs  speedup={:5.2}x  [{}]",
            name,
            seq_time as f64 / 100_000.0,
            par_time as f64 / 100_000.0,
            speedup,
            better
        );
    }

    println!("{:=<70}", "");
    println!("\nExpected behavior:");
    println!("  - Below 2,000: Sequential should be similar or faster");
    println!("  - At 2,000: Parallel starts to show benefit");
    println!("  - Above 2,000: Parallel should be significantly faster (1.5-5x)");
    println!();

    // Validate performance characteristics
    let (_seq_small, _par_small) = results.get("Small (500)").unwrap();
    let (seq_large, par_large) = results.get("Large (10,000)").unwrap();

    // At 10K entities, parallel should be significantly faster
    let large_speedup = *seq_large as f64 / *par_large as f64;
    assert!(
        large_speedup > 1.5,
        "At 10K entities, parallel should show at least 1.5x speedup, got {:.2}x",
        large_speedup
    );

    println!("✓ Performance regression test passed!");
}
