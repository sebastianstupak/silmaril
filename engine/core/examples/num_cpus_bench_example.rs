//! Simple example to demonstrate num_cpus caching performance.
//!
//! This example shows the performance difference between cached and uncached
//! CPU count queries.
//!
//! Run with: cargo run --example num_cpus_bench_example --release

use engine_core::platform::create_threading_backend;
use std::time::Instant;

fn main() {
    println!("=== num_cpus Caching Performance Demo ===\n");

    // Create threading backend (caches CPU count)
    let backend = create_threading_backend().expect("Failed to create threading backend");

    let iterations = 10_000;

    // Test 1: Cached version (our optimization)
    println!("Test 1: Cached num_cpus (optimized)");
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = backend.num_cpus();
    }
    let cached_time = start.elapsed();
    let cached_per_call = cached_time.as_nanos() / iterations as u128;

    println!("  Total time: {:?}", cached_time);
    println!("  Per call:   {} ns", cached_per_call);
    println!();

    // Test 2: Uncached version (baseline - syscall every time)
    println!("Test 2: Uncached num_cpus (baseline syscall)");
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    }
    let uncached_time = start.elapsed();
    let uncached_per_call = uncached_time.as_nanos() / iterations as u128;

    println!("  Total time: {:?}", uncached_time);
    println!("  Per call:   {} ns", uncached_per_call);
    println!();

    // Analysis
    println!("=== Performance Analysis ===");
    println!("CPU Count: {}", backend.num_cpus());
    println!();

    let speedup = uncached_per_call as f64 / cached_per_call as f64;
    let time_saved = uncached_time.saturating_sub(cached_time);

    println!("Speedup:     {:.1}x faster", speedup);
    println!("Time saved:  {:?} ({} iterations)", time_saved, iterations);
    println!("Per-call improvement: {} ns → {} ns", uncached_per_call, cached_per_call);
    println!();

    // Target verification
    let target_ns = 1_000; // 1µs target
    let ideal_ns = 100;    // 100ns ideal

    println!("=== Target Verification ===");
    println!("Target:      < {} ns (< 1µs)", target_ns);
    println!("Ideal:       < {} ns", ideal_ns);
    println!("Achieved:    {} ns", cached_per_call);

    if cached_per_call <= ideal_ns {
        println!("Status:      ✅ IDEAL - Beats ideal target!");
    } else if cached_per_call <= target_ns {
        println!("Status:      ✅ GOOD - Meets target");
    } else {
        println!("Status:      ⚠️ NEEDS WORK - Above target");
    }

    println!();
    println!("=== Conclusion ===");
    println!("The cached implementation is {:.1}x faster than syscall baseline.", speedup);
    println!("This optimization eliminates the 1.95µs syscall overhead identified in benchmarks.");
}
