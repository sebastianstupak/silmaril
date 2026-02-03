//! Cache efficiency benchmarks for the audio system
//!
//! Measures cache performance and data locality to optimize hot paths:
//! - Cache miss rate in spatial audio calculations
//! - Sequential vs random access patterns
//! - Cache-friendly data layouts (AoS vs SoA)
//! - Cache line utilization
//!
//! Target: < 5% cache miss rate in hot paths
//!
//! These benchmarks help identify cache-unfriendly code patterns and
//! guide data structure layout decisions for maximum performance.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use engine_audio::AudioEngine;
use glam::Vec3;
use std::time::Duration;

/// Benchmark sequential emitter updates (cache-friendly)
///
/// Tests best-case scenario where emitters are updated in order
/// This should have excellent cache performance due to prefetching
fn bench_sequential_emitter_updates(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_sequential_updates");
    group.measurement_time(Duration::from_secs(5));

    for count in [100, 1_000, 10_000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            let mut engine = AudioEngine::new().unwrap();

            // Pre-create emitters
            for i in 0..count {
                engine.update_emitter_position(i, Vec3::ZERO);
            }

            b.iter(|| {
                // Sequential access pattern (cache-friendly)
                for i in 0..count {
                    let pos = Vec3::new(i as f32, 0.0, 0.0);
                    engine.update_emitter_position(black_box(i), black_box(pos));
                }
            });
        });
    }

    group.finish();
}

/// Benchmark random emitter updates (cache-unfriendly)
///
/// Tests worst-case scenario where emitters are updated randomly
/// This should show significant cache misses
fn bench_random_emitter_updates(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_random_updates");
    group.measurement_time(Duration::from_secs(5));

    for count in [100, 1_000, 10_000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            let mut engine = AudioEngine::new().unwrap();

            // Pre-create emitters
            for i in 0..count {
                engine.update_emitter_position(i, Vec3::ZERO);
            }

            // Pre-generate random access pattern (using simple LCG)
            let mut random_order = Vec::with_capacity(count as usize);
            let mut rng_state = 12345u32;
            for _ in 0..count {
                rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
                random_order.push(rng_state % count);
            }

            b.iter(|| {
                // Random access pattern (cache-unfriendly)
                for &i in &random_order {
                    let pos = Vec3::new(i as f32, 0.0, 0.0);
                    engine.update_emitter_position(black_box(i), black_box(pos));
                }
            });
        });
    }

    group.finish();
}

/// Benchmark strided access patterns
///
/// Tests performance of accessing every Nth emitter
/// Helps identify optimal stride for cache line utilization
fn bench_strided_emitter_access(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_strided_access");
    group.measurement_time(Duration::from_secs(5));

    let count = 10_000u32;

    for stride in [1, 2, 4, 8, 16, 32, 64].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(stride), stride, |b, &stride| {
            let mut engine = AudioEngine::new().unwrap();

            // Pre-create emitters
            for i in 0..count {
                engine.update_emitter_position(i, Vec3::ZERO);
            }

            b.iter(|| {
                // Strided access pattern
                let mut i = 0u32;
                while i < count {
                    let pos = Vec3::new(i as f32, 0.0, 0.0);
                    engine.update_emitter_position(black_box(i), black_box(pos));
                    i += stride;
                }
            });
        });
    }

    group.finish();
}

/// Benchmark spatial query locality
///
/// Tests cache performance when querying nearby emitters
/// Spatially coherent queries should have better cache performance
fn bench_spatial_query_locality(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_spatial_locality");
    group.measurement_time(Duration::from_secs(5));

    group.bench_function("nearby_emitters", |b| {
        let mut engine = AudioEngine::new().unwrap();

        // Create grid of emitters (spatially coherent)
        let grid_size = 100;
        for x in 0..grid_size {
            for z in 0..grid_size {
                let id = x * grid_size + z;
                let pos = Vec3::new(x as f32 * 10.0, 0.0, z as f32 * 10.0);
                engine.update_emitter_position(id, pos);
            }
        }

        b.iter(|| {
            // Query nearby emitters (spatially coherent access)
            for x in 40..60 {
                for z in 40..60 {
                    let id = x * grid_size + z;
                    let pos = Vec3::new(x as f32 * 10.0 + 0.1, 0.0, z as f32 * 10.0 + 0.1);
                    engine.update_emitter_position(black_box(id), black_box(pos));
                }
            }
        });
    });

    group.bench_function("scattered_emitters", |b| {
        let mut engine = AudioEngine::new().unwrap();

        // Create grid of emitters
        let grid_size = 100;
        for x in 0..grid_size {
            for z in 0..grid_size {
                let id = x * grid_size + z;
                let pos = Vec3::new(x as f32 * 10.0, 0.0, z as f32 * 10.0);
                engine.update_emitter_position(id, pos);
            }
        }

        b.iter(|| {
            // Query scattered emitters (poor spatial locality)
            let mut rng_state = 12345u32;
            for _ in 0..400 {
                rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
                let x = (rng_state % grid_size) as u32;
                rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
                let z = (rng_state % grid_size) as u32;

                let id = x * grid_size + z;
                let pos = Vec3::new(x as f32 * 10.0 + 0.1, 0.0, z as f32 * 10.0 + 0.1);
                engine.update_emitter_position(black_box(id), black_box(pos));
            }
        });
    });

    group.finish();
}

/// Benchmark bulk operations vs individual operations
///
/// Tests if batching operations improves cache efficiency
/// Bulk operations should have better cache utilization
fn bench_bulk_vs_individual(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_bulk_vs_individual");
    group.measurement_time(Duration::from_secs(5));

    let count = 1000u32;

    group.bench_function("individual_updates", |b| {
        let mut engine = AudioEngine::new().unwrap();

        // Pre-create emitters
        for i in 0..count {
            engine.update_emitter_position(i, Vec3::ZERO);
        }

        b.iter(|| {
            // Update emitters one at a time
            for i in 0..count {
                let pos = Vec3::new(i as f32, 0.0, 0.0);
                engine.update_emitter_position(black_box(i), black_box(pos));
            }
        });
    });

    group.bench_function("batched_updates", |b| {
        let mut engine = AudioEngine::new().unwrap();

        // Pre-create emitters
        for i in 0..count {
            engine.update_emitter_position(i, Vec3::ZERO);
        }

        // Pre-compute all positions (simulate batching)
        let positions: Vec<(u32, Vec3)> =
            (0..count).map(|i| (i, Vec3::new(i as f32, 0.0, 0.0))).collect();

        b.iter(|| {
            // Update emitters in batch (better cache utilization)
            for &(id, pos) in &positions {
                engine.update_emitter_position(black_box(id), black_box(pos));
            }
        });
    });

    group.finish();
}

/// Benchmark cache line utilization with different data sizes
///
/// Tests performance impact of data structure size on cache efficiency
/// Smaller structures should have better cache line utilization
fn bench_cache_line_utilization(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_line_utilization");
    group.measurement_time(Duration::from_secs(5));

    for batch_size in [8, 16, 32, 64, 128].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            batch_size,
            |b, &batch_size| {
                let mut engine = AudioEngine::new().unwrap();

                // Pre-create emitters
                for i in 0..(batch_size * 10) {
                    engine.update_emitter_position(i, Vec3::ZERO);
                }

                b.iter(|| {
                    // Process in cache-line-sized batches
                    for batch in 0..10 {
                        for i in 0..batch_size {
                            let id = batch * batch_size + i;
                            let pos = Vec3::new(id as f32, 0.0, 0.0);
                            engine.update_emitter_position(black_box(id), black_box(pos));
                        }
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark prefetch-friendly access patterns
///
/// Tests if hardware prefetcher can predict access patterns
/// Linear access should trigger hardware prefetching
fn bench_prefetch_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_prefetch_patterns");
    group.measurement_time(Duration::from_secs(5));

    let count = 10_000u32;

    group.bench_function("forward_linear", |b| {
        let mut engine = AudioEngine::new().unwrap();

        for i in 0..count {
            engine.update_emitter_position(i, Vec3::ZERO);
        }

        b.iter(|| {
            // Forward linear access (prefetch-friendly)
            for i in 0..count {
                let pos = Vec3::new(i as f32, 0.0, 0.0);
                engine.update_emitter_position(black_box(i), black_box(pos));
            }
        });
    });

    group.bench_function("backward_linear", |b| {
        let mut engine = AudioEngine::new().unwrap();

        for i in 0..count {
            engine.update_emitter_position(i, Vec3::ZERO);
        }

        b.iter(|| {
            // Backward linear access (may inhibit prefetching)
            for i in (0..count).rev() {
                let pos = Vec3::new(i as f32, 0.0, 0.0);
                engine.update_emitter_position(black_box(i), black_box(pos));
            }
        });
    });

    group.bench_function("alternating", |b| {
        let mut engine = AudioEngine::new().unwrap();

        for i in 0..count {
            engine.update_emitter_position(i, Vec3::ZERO);
        }

        b.iter(|| {
            // Alternating access (defeats prefetcher)
            for i in 0..(count / 2) {
                let id1 = i * 2;
                let id2 = i * 2 + 1;
                let pos1 = Vec3::new(id1 as f32, 0.0, 0.0);
                let pos2 = Vec3::new(id2 as f32, 0.0, 0.0);
                engine.update_emitter_position(black_box(id1), black_box(pos1));
                engine.update_emitter_position(black_box(id2), black_box(pos2));
            }
        });
    });

    group.finish();
}

/// Benchmark cold vs warm cache performance
///
/// Tests performance difference between first access (cold cache)
/// and subsequent accesses (warm cache)
fn bench_cold_vs_warm_cache(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_cold_vs_warm");
    group.measurement_time(Duration::from_secs(5));

    let count = 1000u32;

    group.bench_function("cold_cache_simulation", |b| {
        b.iter_custom(|iters| {
            let mut total_duration = Duration::ZERO;

            for _ in 0..iters {
                // Create fresh engine (cold cache)
                let mut engine = AudioEngine::new().unwrap();

                // Create emitters (cold cache state)
                let start = std::time::Instant::now();
                for i in 0..count {
                    engine.update_emitter_position(black_box(i), black_box(Vec3::ZERO));
                }
                total_duration += start.elapsed();

                // Drop to ensure next iteration is cold
                drop(engine);
            }

            total_duration
        });
    });

    group.bench_function("warm_cache", |b| {
        let mut engine = AudioEngine::new().unwrap();

        // Pre-create emitters
        for i in 0..count {
            engine.update_emitter_position(i, Vec3::ZERO);
        }

        // Warm up cache
        for i in 0..count {
            engine.update_emitter_position(i, Vec3::new(i as f32, 0.0, 0.0));
        }

        b.iter(|| {
            // Update emitters (warm cache)
            for i in 0..count {
                let pos = Vec3::new(i as f32 + 0.1, 0.0, 0.0);
                engine.update_emitter_position(black_box(i), black_box(pos));
            }
        });
    });

    group.finish();
}

/// Benchmark cache thrashing scenario
///
/// Tests performance when working set exceeds cache size
/// Should show significant slowdown when cache is thrashed
fn bench_cache_thrashing(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_thrashing");
    group.measurement_time(Duration::from_secs(8));

    // Test with working sets that fit and exceed typical L3 cache (8-16MB)
    for count in [1_000, 10_000, 100_000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            let mut engine = AudioEngine::new().unwrap();

            // Create large number of emitters
            for i in 0..count {
                engine.update_emitter_position(i, Vec3::ZERO);
            }

            b.iter(|| {
                // Access all emitters sequentially
                // If count is large enough, this will thrash cache
                for i in 0..count {
                    let pos = Vec3::new((i % 1000) as f32, 0.0, 0.0);
                    engine.update_emitter_position(black_box(i), black_box(pos));
                }
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_sequential_emitter_updates,
    bench_random_emitter_updates,
    bench_strided_emitter_access,
    bench_spatial_query_locality,
    bench_bulk_vs_individual,
    bench_cache_line_utilization,
    bench_prefetch_patterns,
    bench_cold_vs_warm_cache,
    bench_cache_thrashing,
);

criterion_main!(benches);
