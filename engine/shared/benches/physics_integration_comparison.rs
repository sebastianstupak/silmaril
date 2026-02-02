//! Benchmark comparing scalar vs SIMD physics integration performance

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_core::ecs::World;
use engine_core::math::Transform;
use engine_physics::components::Velocity;
use engine_physics::systems::integration::physics_integration_system;

use engine_physics::systems::integration_simd::physics_integration_system_simd;

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

fn bench_scalar_integration(c: &mut Criterion) {
    let mut group = c.benchmark_group("physics_integration_scalar");

    for entity_count in [100, 1_000, 10_000, 50_000].iter() {
        group.throughput(Throughput::Elements(*entity_count as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(entity_count),
            entity_count,
            |bench, &count| {
                let mut world = create_world(count);
                bench.iter(|| {
                    physics_integration_system(&mut world, black_box(0.016));
                });
            },
        );
    }
    group.finish();
}

fn bench_simd_integration(c: &mut Criterion) {
    let mut group = c.benchmark_group("physics_integration_simd");

    for entity_count in [100, 1_000, 10_000, 50_000].iter() {
        group.throughput(Throughput::Elements(*entity_count as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(entity_count),
            entity_count,
            |bench, &count| {
                let mut world = create_world(count);
                bench.iter(|| {
                    physics_integration_system_simd(&mut world, black_box(0.016));
                });
            },
        );
    }
    group.finish();
}

fn bench_scalar_vs_simd(c: &mut Criterion) {
    let mut group = c.benchmark_group("scalar_vs_simd_comparison");

    let entity_count = 10_000;
    group.throughput(Throughput::Elements(entity_count as u64));

    group.bench_function("scalar", |bench| {
        let mut world = create_world(entity_count);
        bench.iter(|| {
            physics_integration_system(&mut world, black_box(0.016));
        });
    });

    group.bench_function("simd", |bench| {
        let mut world = create_world(entity_count);
        bench.iter(|| {
            physics_integration_system_simd(&mut world, black_box(0.016));
        });
    });

    group.finish();
}

criterion_group!(benches, bench_scalar_integration, bench_simd_integration, bench_scalar_vs_simd);
criterion_main!(benches);
