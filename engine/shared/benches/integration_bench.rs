//! Benchmarks for physics integration systems.
//!
//! Compares scalar vs SIMD vs parallel implementations across various entity counts.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_core::ecs::World;
use engine_core::math::Transform;
use engine_math::Vec3;
use engine_physics::components::Velocity;
use engine_physics::systems::integration::physics_integration_system;
use engine_physics::systems::integration_simd::physics_integration_system_simd;

/// Create a world with N entities for benchmarking.
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

/// Benchmark scalar integration at various entity counts.
fn bench_scalar_integration(c: &mut Criterion) {
    let mut group = c.benchmark_group("scalar_integration");

    for entity_count in [10, 100, 1_000, 10_000, 50_000].iter() {
        group.throughput(Throughput::Elements(*entity_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(entity_count),
            entity_count,
            |b, &count| {
                let mut world = create_world(count);
                b.iter(|| {
                    physics_integration_system(black_box(&mut world), black_box(0.016));
                });
            },
        );
    }

    group.finish();
}

/// Benchmark SIMD integration at various entity counts.
fn bench_simd_integration(c: &mut Criterion) {
    let mut group = c.benchmark_group("simd_integration");

    for entity_count in [10, 100, 1_000, 10_000, 50_000].iter() {
        group.throughput(Throughput::Elements(*entity_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(entity_count),
            entity_count,
            |b, &count| {
                let mut world = create_world(count);
                b.iter(|| {
                    physics_integration_system_simd(black_box(&mut world), black_box(0.016));
                });
            },
        );
    }

    group.finish();
}

/// Benchmark comparison: scalar vs SIMD at same entity count.
fn bench_scalar_vs_simd(c: &mut Criterion) {
    let mut group = c.benchmark_group("scalar_vs_simd");

    for entity_count in [100, 1_000, 10_000].iter() {
        let count = *entity_count;

        group.bench_function(BenchmarkId::new("scalar", count), |b| {
            let mut world = create_world(count);
            b.iter(|| {
                physics_integration_system(black_box(&mut world), black_box(0.016));
            });
        });

        group.bench_function(BenchmarkId::new("simd", count), |b| {
            let mut world = create_world(count);
            b.iter(|| {
                physics_integration_system_simd(black_box(&mut world), black_box(0.016));
            });
        });
    }

    group.finish();
}

/// Benchmark different batch sizes for SIMD processing.
fn bench_batch_sizes(c: &mut Criterion) {
    use engine_physics::systems::integration_simd::{process_batch_4_simd, process_batch_8_simd};

    let mut group = c.benchmark_group("batch_sizes");

    // Batch of 4 (SSE)
    group.bench_function("batch_4", |b| {
        let mut transforms = vec![Transform::identity(); 4];
        let velocities = vec![Vec3::new(1.0, 2.0, 3.0); 4];
        b.iter(|| {
            process_batch_4_simd(
                black_box(&mut transforms),
                black_box(&velocities),
                black_box(0.016),
            );
        });
    });

    // Batch of 8 (AVX2)
    group.bench_function("batch_8", |b| {
        let mut transforms = vec![Transform::identity(); 8];
        let velocities = vec![Vec3::new(1.0, 2.0, 3.0); 8];
        b.iter(|| {
            process_batch_8_simd(
                black_box(&mut transforms),
                black_box(&velocities),
                black_box(0.016),
            );
        });
    });

    group.finish();
}

/// Benchmark sequential vs parallel processing.
fn bench_sequential_vs_parallel(c: &mut Criterion) {
    use engine_physics::systems::integration_simd::{process_parallel, process_sequential};

    let mut group = c.benchmark_group("sequential_vs_parallel");

    // Test at various counts to see where parallel wins
    for entity_count in [1_000, 5_000, 10_000, 50_000, 100_000].iter() {
        let count = *entity_count;

        group.bench_function(BenchmarkId::new("sequential", count), |b| {
            let mut transforms = vec![Transform::identity(); count];
            let velocities = vec![Vec3::new(1.0, 2.0, 3.0); count];
            b.iter(|| {
                process_sequential(
                    black_box(&mut transforms),
                    black_box(&velocities),
                    black_box(0.016),
                );
            });
        });

        group.bench_function(BenchmarkId::new("parallel", count), |b| {
            let mut transforms = vec![Transform::identity(); count];
            let velocities = vec![Vec3::new(1.0, 2.0, 3.0); count];
            b.iter(|| {
                process_parallel(
                    black_box(&mut transforms),
                    black_box(&velocities),
                    black_box(0.016),
                );
            });
        });
    }

    group.finish();
}

/// Benchmark hybrid processing (8-wide + 4-wide + scalar).
fn bench_hybrid_processing(c: &mut Criterion) {
    use engine_physics::systems::integration_simd::process_sequential;

    let mut group = c.benchmark_group("hybrid_processing");

    // Test various counts that exercise different code paths
    let test_cases = vec![
        ("exact_8", 8),        // Exactly one batch of 8
        ("exact_4", 4),        // Exactly one batch of 4
        ("hybrid_12", 12),     // One batch of 8 + one batch of 4
        ("hybrid_15", 15),     // One batch of 8 + one batch of 4 + 3 scalar
        ("hybrid_100", 100),   // Multiple batches with remainder
        ("hybrid_1000", 1000), // Many batches
    ];

    for (name, count) in test_cases {
        group.bench_function(name, |b| {
            let mut transforms = vec![Transform::identity(); count];
            let velocities = vec![Vec3::new(1.0, 2.0, 3.0); count];
            b.iter(|| {
                process_sequential(
                    black_box(&mut transforms),
                    black_box(&velocities),
                    black_box(0.016),
                );
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_scalar_integration,
    bench_simd_integration,
    bench_scalar_vs_simd,
    bench_batch_sizes,
    bench_sequential_vs_parallel,
    bench_hybrid_processing
);

criterion_main!(benches);
