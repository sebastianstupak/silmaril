//! ECS Scalability Benchmarks
//!
//! Measures ECS performance scaling from 1K to 1M entities.
//! Tests query iteration, entity spawn/despawn, component operations.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_core::ecs::{Component, World};
use engine_core::math::{Quat, Transform, Vec3};

#[derive(Component, Debug, Clone, Copy)]
struct Position(Vec3);

#[derive(Component, Debug, Clone, Copy)]
struct Velocity(Vec3);

#[derive(Component, Debug, Clone, Copy)]
struct Acceleration(Vec3);

#[derive(Component, Debug, Clone, Copy)]
struct Health {
    current: f32,
    max: f32,
}

/// Benchmark query iteration scaling
fn bench_query_iteration(c: &mut Criterion) {
    let mut group = c.benchmark_group("ecs_query_iteration");

    for size in [100, 1_000, 10_000, 100_000].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let mut world = World::new();
            world.register::<Position>();
            world.register::<Velocity>();

            // Spawn entities
            for i in 0..size {
                let entity = world.spawn();
                world.add(entity, Position(Vec3::new(i as f32, 0.0, 0.0)));
                world.add(entity, Velocity(Vec3::new(1.0, 0.0, 0.0)));
            }

            b.iter(|| {
                for (pos, vel) in world.query::<(&Position, &Velocity)>().iter() {
                    black_box(pos.0 + vel.0);
                }
            });
        });
    }

    group.finish();
}

/// Benchmark mutable query iteration scaling
fn bench_mutable_query(c: &mut Criterion) {
    let mut group = c.benchmark_group("ecs_mutable_query");

    for size in [100, 1_000, 10_000, 100_000].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let mut world = World::new();
            world.register::<Position>();
            world.register::<Velocity>();

            for i in 0..size {
                let entity = world.spawn();
                world.add(entity, Position(Vec3::new(i as f32, 0.0, 0.0)));
                world.add(entity, Velocity(Vec3::new(1.0, 0.0, 0.0)));
            }

            let dt = 1.0 / 60.0;
            b.iter(|| {
                for (mut pos, vel) in world.query::<(&mut Position, &Velocity)>().iter_mut() {
                    pos.0 = black_box(pos.0 + vel.0 * dt);
                }
            });
        });
    }

    group.finish();
}

/// Benchmark entity spawn throughput
fn bench_entity_spawn(c: &mut Criterion) {
    let mut group = c.benchmark_group("ecs_entity_spawn");

    for size in [100, 1_000, 10_000].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_batched(
                || {
                    let mut world = World::new();
                    world.register::<Position>();
                    world.register::<Velocity>();
                    world
                },
                |mut world| {
                    for i in 0..size {
                        let entity = world.spawn();
                        world.add(entity, Position(Vec3::new(i as f32, 0.0, 0.0)));
                        world.add(entity, Velocity(Vec3::new(1.0, 0.0, 0.0)));
                    }
                    black_box(world);
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

/// Benchmark entity despawn throughput
fn bench_entity_despawn(c: &mut Criterion) {
    let mut group = c.benchmark_group("ecs_entity_despawn");

    for size in [100, 1_000, 10_000].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_batched(
                || {
                    let mut world = World::new();
                    world.register::<Position>();
                    world.register::<Velocity>();

                    let mut entities = Vec::with_capacity(size);
                    for i in 0..size {
                        let entity = world.spawn();
                        world.add(entity, Position(Vec3::new(i as f32, 0.0, 0.0)));
                        world.add(entity, Velocity(Vec3::new(1.0, 0.0, 0.0)));
                        entities.push(entity);
                    }
                    (world, entities)
                },
                |(mut world, entities)| {
                    for entity in entities {
                        world.despawn(entity);
                    }
                    black_box(world);
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

/// Benchmark component add throughput
fn bench_component_add(c: &mut Criterion) {
    let mut group = c.benchmark_group("ecs_component_add");

    for size in [100, 1_000, 10_000].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_batched(
                || {
                    let mut world = World::new();
                    world.register::<Position>();
                    world.register::<Velocity>();
                    world.register::<Acceleration>();

                    let mut entities = Vec::with_capacity(size);
                    for i in 0..size {
                        let entity = world.spawn();
                        world.add(entity, Position(Vec3::new(i as f32, 0.0, 0.0)));
                        entities.push(entity);
                    }
                    (world, entities)
                },
                |(mut world, entities)| {
                    for entity in entities {
                        world.add(entity, Velocity(Vec3::ZERO));
                    }
                    black_box(world);
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

/// Benchmark component remove throughput
fn bench_component_remove(c: &mut Criterion) {
    let mut group = c.benchmark_group("ecs_component_remove");

    for size in [100, 1_000, 10_000].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_batched(
                || {
                    let mut world = World::new();
                    world.register::<Position>();
                    world.register::<Velocity>();

                    let mut entities = Vec::with_capacity(size);
                    for i in 0..size {
                        let entity = world.spawn();
                        world.add(entity, Position(Vec3::new(i as f32, 0.0, 0.0)));
                        world.add(entity, Velocity(Vec3::new(1.0, 0.0, 0.0)));
                        entities.push(entity);
                    }
                    (world, entities)
                },
                |(mut world, entities)| {
                    for entity in entities {
                        world.remove::<Velocity>(entity);
                    }
                    black_box(world);
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

/// Benchmark complex query (3+ components)
fn bench_complex_query(c: &mut Criterion) {
    let mut group = c.benchmark_group("ecs_complex_query");

    for size in [100, 1_000, 10_000, 100_000].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let mut world = World::new();
            world.register::<Position>();
            world.register::<Velocity>();
            world.register::<Acceleration>();
            world.register::<Health>();
            world.register::<Transform>();

            for i in 0..size {
                let entity = world.spawn();
                world.add(entity, Position(Vec3::new(i as f32, 0.0, 0.0)));
                world.add(entity, Velocity(Vec3::new(1.0, 0.0, 0.0)));
                world.add(entity, Acceleration(Vec3::new(0.0, -9.8, 0.0)));
                world.add(entity, Health { current: 100.0, max: 100.0 });
                world.add(entity, Transform::default());
            }

            b.iter(|| {
                for (pos, vel, acc, health, transform) in world
                    .query::<(&Position, &Velocity, &Acceleration, &Health, &Transform)>()
                    .iter()
                {
                    black_box((pos, vel, acc, health, transform));
                }
            });
        });
    }

    group.finish();
}

/// Benchmark full physics-like update (position integration)
fn bench_physics_update(c: &mut Criterion) {
    let mut group = c.benchmark_group("ecs_physics_update");

    for size in [1_000, 10_000, 100_000].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let mut world = World::new();
            world.register::<Position>();
            world.register::<Velocity>();
            world.register::<Acceleration>();

            for i in 0..size {
                let entity = world.spawn();
                world.add(entity, Position(Vec3::new(i as f32, 0.0, 0.0)));
                world.add(entity, Velocity(Vec3::new((i % 10) as f32, 0.0, 0.0)));
                world.add(entity, Acceleration(Vec3::new(0.0, -9.8, 0.0)));
            }

            let dt = 1.0 / 60.0;
            b.iter(|| {
                // Update velocities
                for (mut vel, acc) in world.query::<(&mut Velocity, &Acceleration)>().iter_mut() {
                    vel.0 = vel.0 + acc.0 * dt;
                }

                // Update positions
                for (mut pos, vel) in world.query::<(&mut Position, &Velocity)>().iter_mut() {
                    pos.0 = pos.0 + vel.0 * dt;
                }
            });
        });
    }

    group.finish();
}

/// Benchmark memory allocation patterns
fn bench_allocation_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("ecs_allocation_patterns");

    group.bench_function("spawn_despawn_churn", |b| {
        b.iter_batched(
            || {
                let mut world = World::new();
                world.register::<Position>();
                world.register::<Velocity>();
                world
            },
            |mut world| {
                // Simulate churn: spawn and despawn rapidly
                for _ in 0..100 {
                    let mut entities = Vec::new();
                    for i in 0..100 {
                        let entity = world.spawn();
                        world.add(entity, Position(Vec3::new(i as f32, 0.0, 0.0)));
                        world.add(entity, Velocity(Vec3::ZERO));
                        entities.push(entity);
                    }

                    for entity in entities {
                        world.despawn(entity);
                    }
                }
                black_box(world);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_query_iteration,
    bench_mutable_query,
    bench_entity_spawn,
    bench_entity_despawn,
    bench_component_add,
    bench_component_remove,
    bench_complex_query,
    bench_physics_update,
    bench_allocation_patterns,
);
criterion_main!(benches);
