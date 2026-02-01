//! Dedicated benchmark for num_cpus caching optimization.
//!
//! This benchmark measures the performance improvement from caching CPU count
//! at backend creation time.
//!
//! Expected results:
//! - BEFORE optimization: ~1.95µs per call (syscall overhead)
//! - AFTER optimization: ~10-100ns per call (memory read)
//! - Improvement: ~20-200x faster

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use engine_core::platform::create_threading_backend;

/// Benchmark num_cpus with cached value.
/// Target: < 1us (ideal: <100ns)
fn bench_num_cpus_cached(c: &mut Criterion) {
    let backend = create_threading_backend().expect("Failed to create threading backend");

    c.bench_function("threading/num_cpus/cached", |b| {
        b.iter(|| {
            black_box(backend.num_cpus());
        });
    });
}

/// Benchmark num_cpus with uncached (syscall) for comparison.
/// This shows the baseline without our optimization.
fn bench_num_cpus_uncached(c: &mut Criterion) {
    c.bench_function("threading/num_cpus/uncached_baseline", |b| {
        b.iter(|| {
            // This makes a syscall every time (no caching)
            let count = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
            black_box(count);
        });
    });
}

/// Benchmark batch operations to show the cumulative benefit.
fn bench_num_cpus_batch(c: &mut Criterion) {
    let backend = create_threading_backend().expect("Failed to create threading backend");
    let mut group = c.benchmark_group("threading/num_cpus/batch");

    for count in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::new("cached", count), count, |b, &count| {
            b.iter(|| {
                for _ in 0..count {
                    black_box(backend.num_cpus());
                }
            });
        });

        group.bench_with_input(BenchmarkId::new("uncached", count), count, |b, &count| {
            b.iter(|| {
                for _ in 0..count {
                    let n = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
                    black_box(n);
                }
            });
        });
    }

    group.finish();
}

/// Benchmark typical use case: checking CPU count before creating thread pool.
fn bench_typical_usage(c: &mut Criterion) {
    let backend = create_threading_backend().expect("Failed to create threading backend");

    c.bench_function("threading/num_cpus/typical_usage", |b| {
        b.iter(|| {
            // Typical pattern: get CPU count, decide on thread count
            let num_cpus = backend.num_cpus();
            let thread_count = (num_cpus / 2).max(1);
            black_box(thread_count);
        });
    });
}

criterion_group!(
    threading_cache_benches,
    bench_num_cpus_cached,
    bench_num_cpus_uncached,
    bench_num_cpus_batch,
    bench_typical_usage,
);

criterion_main!(threading_cache_benches);
