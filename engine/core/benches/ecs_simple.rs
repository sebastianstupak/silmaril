//! Simplified ECS benchmarks that compile and run
//!
//! Basic benchmarks to establish baseline performance.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_core::ecs::World;
use engine_core::math::Transform;

// Entity spawning benchmarks
fn bench_spawn_entities(c: &mut Criterion) {
    let mut group = c.benchmark_group("spawn_entities");

    for count in [100, 1_000, 10_000].iter() {
        group.throughput(Throughput::Elements(*count as u64));
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            b.iter_batched(
                || World::new(),
                |mut world| {
                    for _ in 0..count {
                        black_box(world.spawn());
                    }
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

// Entity iteration benchmarks
fn bench_iterate_entities(c: &mut Criterion) {
    let mut group = c.benchmark_group("iterate_entities");

    for count in [1_000, 10_000, 100_000].iter() {
        group.throughput(Throughput::Elements(*count as u64));
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            let mut world = World::new();
            for _ in 0..count {
                let entity = world.spawn();
                world.add(entity, Transform::default());
            }

            b.iter(|| {
                let mut sum = 0;
                for (_entity, _transform) in world.query::<&Transform>() {
                    sum += 1;
                }
                black_box(sum);
            });
        });
    }

    group.finish();
}

criterion_group!(
    name = benches;
    config = Criterion::default()
        .sample_size(50)
        .measurement_time(std::time::Duration::from_secs(5));
    targets = bench_spawn_entities, bench_iterate_entities
);

criterion_main!(benches);
