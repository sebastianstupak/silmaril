//! SIMD operation benchmarks
//!
//! Benchmarks Vec3x4 and Vec3x8 SIMD operations and compares them to scalar equivalents.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_math::simd::{vec3_aos_to_soa_4, vec3_aos_to_soa_8, Vec3x4, Vec3x8};
use engine_math::Vec3;

fn bench_vec3_add(c: &mut Criterion) {
    let mut group = c.benchmark_group("vec3_add");

    // Scalar
    let a_scalar = Vec3::new(1.0, 2.0, 3.0);
    let b_scalar = Vec3::new(4.0, 5.0, 6.0);

    group.bench_function("scalar", |bench| {
        bench.iter(|| black_box(black_box(a_scalar) + black_box(b_scalar)));
    });

    // SIMD 4-wide
    let a4 = Vec3x4::splat(Vec3::new(1.0, 2.0, 3.0));
    let b4 = Vec3x4::splat(Vec3::new(4.0, 5.0, 6.0));

    group.bench_function("simd_4wide", |bench| {
        bench.iter(|| black_box(black_box(a4) + black_box(b4)));
    });

    // SIMD 8-wide
    let a8 = Vec3x8::splat(Vec3::new(1.0, 2.0, 3.0));
    let b8 = Vec3x8::splat(Vec3::new(4.0, 5.0, 6.0));

    group.bench_function("simd_8wide", |bench| {
        bench.iter(|| black_box(black_box(a8) + black_box(b8)));
    });

    group.finish();
}

fn bench_vec3_mul_scalar(c: &mut Criterion) {
    let mut group = c.benchmark_group("vec3_mul_scalar");
    let scalar = 2.5;

    // Scalar
    let v_scalar = Vec3::new(1.0, 2.0, 3.0);
    group.bench_function("scalar", |bench| {
        bench.iter(|| black_box(black_box(v_scalar) * black_box(scalar)));
    });

    // SIMD 4-wide
    let v4 = Vec3x4::splat(Vec3::new(1.0, 2.0, 3.0));
    group.bench_function("simd_4wide", |bench| {
        bench.iter(|| black_box(black_box(v4) * black_box(scalar)));
    });

    // SIMD 8-wide
    let v8 = Vec3x8::splat(Vec3::new(1.0, 2.0, 3.0));
    group.bench_function("simd_8wide", |bench| {
        bench.iter(|| black_box(black_box(v8) * black_box(scalar)));
    });

    group.finish();
}

fn bench_vec3_mul_add(c: &mut Criterion) {
    let mut group = c.benchmark_group("vec3_mul_add");
    let dt = 0.016;

    // Scalar
    let pos_scalar = Vec3::new(1.0, 2.0, 3.0);
    let vel_scalar = Vec3::new(0.1, 0.2, 0.3);
    group.bench_function("scalar", |bench| {
        bench.iter(|| black_box(black_box(pos_scalar) + black_box(vel_scalar) * black_box(dt)));
    });

    // SIMD 4-wide
    let pos4 = Vec3x4::splat(Vec3::new(1.0, 2.0, 3.0));
    let vel4 = Vec3x4::splat(Vec3::new(0.1, 0.2, 0.3));
    group.bench_function("simd_4wide", |bench| {
        bench.iter(|| black_box(black_box(pos4).mul_add(black_box(vel4), black_box(dt))));
    });

    // SIMD 8-wide
    let pos8 = Vec3x8::splat(Vec3::new(1.0, 2.0, 3.0));
    let vel8 = Vec3x8::splat(Vec3::new(0.1, 0.2, 0.3));
    group.bench_function("simd_8wide", |bench| {
        bench.iter(|| black_box(black_box(pos8).mul_add(black_box(vel8), black_box(dt))));
    });

    group.finish();
}

fn bench_aos_soa_conversion(c: &mut Criterion) {
    let mut group = c.benchmark_group("aos_soa_conversion");

    // 4-wide conversion
    let aos4 = [
        Vec3::new(1.0, 2.0, 3.0),
        Vec3::new(4.0, 5.0, 6.0),
        Vec3::new(7.0, 8.0, 9.0),
        Vec3::new(10.0, 11.0, 12.0),
    ];

    group.bench_function("aos_to_soa_4", |bench| {
        bench.iter(|| black_box(vec3_aos_to_soa_4(black_box(&aos4))));
    });

    let soa4 = vec3_aos_to_soa_4(&aos4);
    group.bench_function("soa_to_aos_4", |bench| {
        bench.iter(|| black_box(black_box(soa4).to_array()));
    });

    // 8-wide conversion
    let aos8 = [
        Vec3::new(1.0, 2.0, 3.0),
        Vec3::new(4.0, 5.0, 6.0),
        Vec3::new(7.0, 8.0, 9.0),
        Vec3::new(10.0, 11.0, 12.0),
        Vec3::new(13.0, 14.0, 15.0),
        Vec3::new(16.0, 17.0, 18.0),
        Vec3::new(19.0, 20.0, 21.0),
        Vec3::new(22.0, 23.0, 24.0),
    ];

    group.bench_function("aos_to_soa_8", |bench| {
        bench.iter(|| black_box(vec3_aos_to_soa_8(black_box(&aos8))));
    });

    let soa8 = vec3_aos_to_soa_8(&aos8);
    group.bench_function("soa_to_aos_8", |bench| {
        bench.iter(|| black_box(black_box(soa8).to_array()));
    });

    group.finish();
}

fn bench_physics_integration_simd_vs_scalar(c: &mut Criterion) {
    let mut group = c.benchmark_group("physics_integration_comparison");

    for size in [100, 1000, 10000].iter() {
        let actual_size = (size / 8) * 8; // Ensure divisible by 8 for widest SIMD
        group.throughput(Throughput::Elements(actual_size as u64));

        // Scalar version
        group.bench_with_input(BenchmarkId::new("scalar", size), &actual_size, |bench, &size| {
            let mut positions: Vec<Vec3> =
                (0..size).map(|i| Vec3::new(i as f32, i as f32, i as f32)).collect();
            let velocities: Vec<Vec3> = (0..size)
                .map(|i| Vec3::new(i as f32 * 0.1, i as f32 * 0.1, i as f32 * 0.1))
                .collect();
            let dt = 0.016;

            bench.iter(|| {
                for i in 0..size {
                    positions[i] = positions[i] + velocities[i] * dt;
                }
                black_box(&positions);
            });
        });

        // SIMD version (4-wide)
        group.bench_with_input(
            BenchmarkId::new("simd_4wide", size),
            &actual_size,
            |bench, &size| {
                let mut positions: Vec<Vec3> =
                    (0..size).map(|i| Vec3::new(i as f32, i as f32, i as f32)).collect();
                let velocities: Vec<Vec3> = (0..size)
                    .map(|i| Vec3::new(i as f32 * 0.1, i as f32 * 0.1, i as f32 * 0.1))
                    .collect();
                let dt = 0.016;

                bench.iter(|| {
                    for chunk_idx in (0..size).step_by(4) {
                        // Convert AoS to SoA
                        let pos_aos = [
                            positions[chunk_idx],
                            positions[chunk_idx + 1],
                            positions[chunk_idx + 2],
                            positions[chunk_idx + 3],
                        ];
                        let vel_aos = [
                            velocities[chunk_idx],
                            velocities[chunk_idx + 1],
                            velocities[chunk_idx + 2],
                            velocities[chunk_idx + 3],
                        ];

                        let pos_soa = vec3_aos_to_soa_4(&pos_aos);
                        let vel_soa = vec3_aos_to_soa_4(&vel_aos);

                        // SIMD operation
                        let new_pos = pos_soa.mul_add(vel_soa, dt);

                        // Convert back to AoS
                        let result = new_pos.to_array();
                        positions[chunk_idx] = result[0];
                        positions[chunk_idx + 1] = result[1];
                        positions[chunk_idx + 2] = result[2];
                        positions[chunk_idx + 3] = result[3];
                    }
                    black_box(&positions);
                });
            },
        );

        // SIMD version (8-wide)
        group.bench_with_input(
            BenchmarkId::new("simd_8wide", size),
            &actual_size,
            |bench, &size| {
                let mut positions: Vec<Vec3> =
                    (0..size).map(|i| Vec3::new(i as f32, i as f32, i as f32)).collect();
                let velocities: Vec<Vec3> = (0..size)
                    .map(|i| Vec3::new(i as f32 * 0.1, i as f32 * 0.1, i as f32 * 0.1))
                    .collect();
                let dt = 0.016;

                bench.iter(|| {
                    for chunk_idx in (0..size).step_by(8) {
                        // Convert AoS to SoA
                        let pos_aos = [
                            positions[chunk_idx],
                            positions[chunk_idx + 1],
                            positions[chunk_idx + 2],
                            positions[chunk_idx + 3],
                            positions[chunk_idx + 4],
                            positions[chunk_idx + 5],
                            positions[chunk_idx + 6],
                            positions[chunk_idx + 7],
                        ];
                        let vel_aos = [
                            velocities[chunk_idx],
                            velocities[chunk_idx + 1],
                            velocities[chunk_idx + 2],
                            velocities[chunk_idx + 3],
                            velocities[chunk_idx + 4],
                            velocities[chunk_idx + 5],
                            velocities[chunk_idx + 6],
                            velocities[chunk_idx + 7],
                        ];

                        let pos_soa = vec3_aos_to_soa_8(&pos_aos);
                        let vel_soa = vec3_aos_to_soa_8(&vel_aos);

                        // SIMD operation
                        let new_pos = pos_soa.mul_add(vel_soa, dt);

                        // Convert back to AoS
                        let result = new_pos.to_array();
                        positions[chunk_idx] = result[0];
                        positions[chunk_idx + 1] = result[1];
                        positions[chunk_idx + 2] = result[2];
                        positions[chunk_idx + 3] = result[3];
                        positions[chunk_idx + 4] = result[4];
                        positions[chunk_idx + 5] = result[5];
                        positions[chunk_idx + 6] = result[6];
                        positions[chunk_idx + 7] = result[7];
                    }
                    black_box(&positions);
                });
            },
        );

        // SIMD 4-wide version without conversion overhead (best case)
        group.bench_with_input(
            BenchmarkId::new("simd_4wide_no_conversion", size),
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

        // SIMD 8-wide version without conversion overhead (best case)
        group.bench_with_input(
            BenchmarkId::new("simd_8wide_no_conversion", size),
            &actual_size,
            |bench, &size| {
                let num_chunks = size / 8;
                let mut positions: Vec<Vec3x8> = (0..num_chunks)
                    .map(|i| Vec3x8::splat(Vec3::new(i as f32, i as f32, i as f32)))
                    .collect();
                let velocities: Vec<Vec3x8> = (0..num_chunks)
                    .map(|i| {
                        Vec3x8::splat(Vec3::new(i as f32 * 0.1, i as f32 * 0.1, i as f32 * 0.1))
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
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_vec3_add,
    bench_vec3_mul_scalar,
    bench_vec3_mul_add,
    bench_aos_soa_conversion,
    bench_physics_integration_simd_vs_scalar,
);
criterion_main!(benches);
