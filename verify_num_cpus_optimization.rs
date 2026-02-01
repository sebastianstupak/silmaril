//! Standalone verification script for num_cpus caching optimization.
//!
//! This script demonstrates the performance improvement without requiring
//! the full engine to compile.
//!
//! Run with: rustc verify_num_cpus_optimization.rs && ./verify_num_cpus_optimization

use std::time::Instant;

/// Simulates the BEFORE state - uncached, syscall every time
fn uncached_num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

/// Simulates the AFTER state - cached value
struct CachedBackend {
    num_cpus: usize,
}

impl CachedBackend {
    fn new() -> Self {
        // Cache once at initialization
        let num_cpus = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);
        Self { num_cpus }
    }

    fn num_cpus(&self) -> usize {
        // Return cached value (just a memory read)
        self.num_cpus
    }
}

fn main() {
    println!("=== num_cpus Caching Optimization Verification ===\n");

    let iterations = 10_000;

    // Test 1: Uncached (BEFORE optimization)
    println!("BEFORE (uncached - syscall every time):");
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = uncached_num_cpus();
    }
    let uncached_time = start.elapsed();
    let uncached_ns = uncached_time.as_nanos() / iterations as u128;
    println!("  Total: {:?}", uncached_time);
    println!("  Per call: {} ns", uncached_ns);
    println!();

    // Test 2: Cached (AFTER optimization)
    println!("AFTER (cached - memory read):");
    let backend = CachedBackend::new();
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = backend.num_cpus();
    }
    let cached_time = start.elapsed();
    let cached_ns = cached_time.as_nanos() / iterations as u128;
    println!("  Total: {:?}", cached_time);
    println!("  Per call: {} ns", cached_ns);
    println!();

    // Analysis
    println!("=== Results ===");
    println!("CPU count: {}", backend.num_cpus());
    println!();

    let speedup = uncached_ns as f64 / cached_ns as f64;
    let time_saved = uncached_time.saturating_sub(cached_time);

    println!("Speedup: {:.1}x faster", speedup);
    println!("Time saved: {:?} over {} iterations", time_saved, iterations);
    println!("Improvement: {} ns → {} ns per call", uncached_ns, cached_ns);
    println!();

    // Target verification
    let target_ns = 1_000; // 1µs = 1000ns
    let ideal_ns = 100;

    println!("=== Target Verification ===");
    println!("Original performance: {} ns (~{:.2} µs)", uncached_ns, uncached_ns as f64 / 1000.0);
    println!("Target: < {} ns (< 1 µs)", target_ns);
    println!("Ideal: < {} ns", ideal_ns);
    println!("Achieved: {} ns", cached_ns);
    println!();

    if cached_ns <= ideal_ns {
        println!("✅ EXCELLENT - Beats ideal target by {:.1}x", ideal_ns as f64 / cached_ns as f64);
    } else if cached_ns <= target_ns {
        println!("✅ GOOD - Meets target ({}% of target)", (cached_ns * 100) / target_ns);
    } else {
        println!("⚠️ NEEDS WORK - {}% over target", ((cached_ns * 100) / target_ns) - 100);
    }

    println!();
    println!("=== Conclusion ===");
    println!("The cached implementation is {:.1}x faster.", speedup);
    println!("This eliminates the ~1.95µs syscall overhead identified in benchmarks.");
    println!();
    println!("Implementation: Simply cache the value in the backend struct:");
    println!("  struct WindowsThreading {{ num_cpus: usize }}");
    println!("  fn new() -> Self {{ Self {{ num_cpus: available_parallelism() }} }}");
    println!("  fn num_cpus(&self) -> usize {{ self.num_cpus }}  // <-- just memory read!");
}
