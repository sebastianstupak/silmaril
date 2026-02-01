//! Benchmarks for cache-aligned memory allocations.
//!
//! Compares performance of aligned vs unaligned SIMD operations to demonstrate
//! the benefits of cache-line alignment.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_math::aligned::AlignedVec;
use engine_math::simd::{vec3_aos_to_soa_4, Vec3x4};
use engine_math::Vec3;

fn bench_aligned_vs_unaligned_vec3x4(c: &mut Criterion) {
    let mut group = c.benchmark_group("aligned_vs_unaligned_storage");

    for size in [100, 1000, 10000].iter() {
        let actual_size = (size / 4) * 4; // Ensure divisible by 4
        group.throughput(Throughput::Elements(actual_size as u64));

        // Unaligned storage (standard Vec)
        group.bench_with_input(
            BenchmarkId::new("unaligned_vec", size),
            &actual_size,
            |bench, &size| {
                let num_chunks = size / 4;
                let mut positions: Vec<Vec3x4> = (0..num_chunks)
                    .map(|i| Vec3x4::splat(Vec3::new(i as f32, i as f32, i as f32)))
                    .collect();
                let velocities: Vec<Vec3x4> = (0..num_chunks)
                    .map(|i| {
                        Vec3x4::splat(Vec3::new(i as f32 * 0.1, i as f32 * 0.1, i as f32 * 0.1))
                    })
                    .collect();
                let dt = 0.016;

                bench.iter(|| {
                    for i in 0..num_chunks {
                        positions[i] = positions[i].mul_add(velocities[i], dt);
                    }
                    black_box(&positions);
                });
            },
        );

        // Cache-line aligned storage (AlignedVec<T, 64>)
        group.bench_with_input(
            BenchmarkId::new("aligned_vec_64", size),
            &actual_size,
            |bench, &size| {
                let num_chunks = size / 4;
                let mut positions: AlignedVec<Vec3x4, 64> = AlignedVec::with_capacity(num_chunks);
                let mut velocities: AlignedVec<Vec3x4, 64> = AlignedVec::with_capacity(num_chunks);

                for i in 0..num_chunks {
                    positions.push(Vec3x4::splat(Vec3::new(i as f32, i as f32, i as f32)));
                    velocities.push(Vec3x4::splat(Vec3::new(
                        i as f32 * 0.1,
                        i as f32 * 0.1,
                        i as f32 * 0.1,
                    )));
                }

                let dt = 0.016;

                bench.iter(|| {
                    for i in 0..num_chunks {
                        positions[i] = positions[i].mul_add(velocities[i], dt);
                    }
                    black_box(&positions);
                });
            },
        );

        // 16-byte aligned storage (minimum for SIMD)
        group.bench_with_input(
            BenchmarkId::new("aligned_vec_16", size),
            &actual_size,
            |bench, &size| {
                let num_chunks = size / 4;
                let mut positions: AlignedVec<Vec3x4, 16> = AlignedVec::with_capacity(num_chunks);
                let mut velocities: AlignedVec<Vec3x4, 16> = AlignedVec::with_capacity(num_chunks);

                for i in 0..num_chunks {
                    positions.push(Vec3x4::splat(Vec3::new(i as f32, i as f32, i as f32)));
                    velocities.push(Vec3x4::splat(Vec3::new(
                        i as f32 * 0.1,
                        i as f32 * 0.1,
                        i as f32 * 0.1,
                    )));
                }

                let dt = 0.016;

                bench.iter(|| {
                    for i in 0..num_chunks {
                        positions[i] = positions[i].mul_add(velocities[i], dt);
                    }
                    black_box(&positions);
                });
            },
        );
    }

    group.finish();
}

fn bench_aligned_loads_stores(c: &mut Criterion) {
    let mut group = c.benchmark_group("aligned_loads_stores");

    // Test aligned load/store performance
    let size = 1000;
    group.throughput(Throughput::Elements(size as u64));

    group.bench_function("vec3x4_aligned_store_load", |bench| {
        let mut buffer: AlignedVec<f32, 16> = AlignedVec::with_capacity(size * 12);
        buffer.resize(size * 12, 0.0);

        let test_vec = Vec3x4::splat(Vec3::new(1.0, 2.0, 3.0));

        bench.iter(|| {
            for i in 0..size {
                unsafe {
                    // Store
                    test_vec.store_aligned(buffer.as_mut_ptr().add(i * 12));
                    // Load back
                    let loaded = Vec3x4::load_aligned(buffer.as_ptr().add(i * 12));
                    black_box(loaded);
                }
            }
        });
    });

    group.bench_function("vec3x4_to_array_from_aos", |bench| {
        let positions = [
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(4.0, 5.0, 6.0),
            Vec3::new(7.0, 8.0, 9.0),
            Vec3::new(10.0, 11.0, 12.0),
        ];

        bench.iter(|| {
            let mut sum = Vec3x4::splat(Vec3::new(0.0, 0.0, 0.0));
            for _ in 0..size {
                let simd = vec3_aos_to_soa_4(black_box(&positions));
                sum = sum + simd;
            }
            black_box(sum);
        });
    });

    group.finish();
}

fn bench_cache_line_false_sharing(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_line_false_sharing");

    // Simulate scenario where unaligned data causes false sharing
    // When data is not cache-line aligned, two threads might access
    // different elements that are on the same cache line, causing
    // cache coherency traffic.

    let size = 1000;
    group.throughput(Throughput::Elements(size as u64));

    // Tightly packed (potential false sharing)
    group.bench_function("tightly_packed", |bench| {
        let mut data: Vec<Vec3x4> = (0..size)
            .map(|i| Vec3x4::splat(Vec3::new(i as f32, i as f32, i as f32)))
            .collect();

        bench.iter(|| {
            // Simulate accessing every Nth element (stride pattern)
            for i in (0..size).step_by(8) {
                data[i] = data[i] * 1.1;
            }
            black_box(&data);
        });
    });

    // Cache-line separated (no false sharing)
    group.bench_function("cache_line_separated", |bench| {
        let mut data: AlignedVec<Vec3x4, 64> = AlignedVec::with_capacity(size);
        for i in 0..size {
            data.push(Vec3x4::splat(Vec3::new(i as f32, i as f32, i as f32)));
        }

        bench.iter(|| {
            // Same access pattern, but with cache-line alignment
            for i in (0..size).step_by(8) {
                data[i] = data[i] * 1.1;
            }
            black_box(&data);
        });
    });

    group.finish();
}

fn bench_bulk_physics_integration(c: &mut Criterion) {
    let mut group = c.benchmark_group("bulk_physics_integration");

    for size in [1000, 10000, 100000].iter() {
        let actual_size = (size / 4) * 4;
        group.throughput(Throughput::Elements(actual_size as u64));

        // Standard Vec
        group.bench_with_input(
            BenchmarkId::new("standard_vec", size),
            &actual_size,
            |bench, &size| {
                let num_chunks = size / 4;
                let mut positions: Vec<Vec3x4> = (0..num_chunks)
                    .map(|i| Vec3x4::splat(Vec3::new(i as f32, i as f32, i as f32)))
                    .collect();
                let velocities: Vec<Vec3x4> =
                    (0..num_chunks).map(|_| Vec3x4::splat(Vec3::new(0.1, 0.2, 0.3))).collect();
                let accelerations: Vec<Vec3x4> =
                    (0..num_chunks).map(|_| Vec3x4::splat(Vec3::new(0.0, -9.81, 0.0))).collect();
                let dt = 0.016;

                bench.iter(|| {
                    // Physics integration: pos += vel * dt, vel += acc * dt
                    for i in 0..num_chunks {
                        let mut vel = velocities[i];
                        vel = vel.mul_add(accelerations[i], dt);
                        positions[i] = positions[i].mul_add(vel, dt);
                    }
                    black_box(&positions);
                });
            },
        );

        // AlignedVec<T, 64>
        group.bench_with_input(
            BenchmarkId::new("aligned_vec_64", size),
            &actual_size,
            |bench, &size| {
                let num_chunks = size / 4;
                let mut positions: AlignedVec<Vec3x4, 64> = AlignedVec::with_capacity(num_chunks);
                let mut velocities: AlignedVec<Vec3x4, 64> = AlignedVec::with_capacity(num_chunks);
                let mut accelerations: AlignedVec<Vec3x4, 64> =
                    AlignedVec::with_capacity(num_chunks);

                for i in 0..num_chunks {
                    positions.push(Vec3x4::splat(Vec3::new(i as f32, i as f32, i as f32)));
                    velocities.push(Vec3x4::splat(Vec3::new(0.1, 0.2, 0.3)));
                    accelerations.push(Vec3x4::splat(Vec3::new(0.0, -9.81, 0.0)));
                }

                let dt = 0.016;

                bench.iter(|| {
                    // Physics integration: pos += vel * dt, vel += acc * dt
                    for i in 0..num_chunks {
                        let mut vel = velocities[i];
                        vel = vel.mul_add(accelerations[i], dt);
                        positions[i] = positions[i].mul_add(vel, dt);
                    }
                    black_box(&positions);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_aligned_vs_unaligned_vec3x4,
    bench_aligned_loads_stores,
    bench_cache_line_false_sharing,
    bench_bulk_physics_integration,
);
criterion_main!(benches);
