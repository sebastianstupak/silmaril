//! Comprehensive benchmark suite for Profile-Guided Optimization (PGO).
//!
//! This benchmark runs a representative workload covering all hot paths
//! in the engine to generate meaningful profile data for PGO.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_core::ecs::{Component, World};
use engine_core::math::Transform;

// Test components for realistic scenarios
#[derive(Debug, Clone, Copy)]
struct Position {
    x: f32,
    y: f32,
    z: f32,
}

impl Component for Position {}

#[derive(Debug, Clone, Copy)]
struct Velocity {
    x: f32,
    y: f32,
    z: f32,
}

impl Component for Velocity {}

#[derive(Debug, Clone, Copy)]
struct Health {
    current: f32,
    max: f32,
}

impl Component for Health {}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
struct Renderable {
    mesh_id: u32,
    material_id: u32,
}

impl Component for Renderable {}

/// Create a world with various entity counts and component combinations.
fn create_mixed_world(entity_count: usize) -> World {
    let mut world = World::new();
    world.register::<Position>();
    world.register::<Velocity>();
    world.register::<Health>();
    world.register::<Transform>();
    world.register::<Renderable>();

    // 50% have all components (players/NPCs)
    for i in 0..entity_count / 2 {
        let entity = world.spawn();
        world.add(entity, Position { x: i as f32, y: 0.0, z: 0.0 });
        world.add(entity, Velocity { x: 1.0, y: 0.0, z: 0.0 });
        world.add(entity, Health { current: 100.0, max: 100.0 });
        world.add(entity, Transform::identity());
        world.add(entity, Renderable { mesh_id: i as u32 % 10, material_id: i as u32 % 5 });
    }

    // 30% have Position + Renderable only (static objects)
    for i in entity_count / 2..(entity_count * 8) / 10 {
        let entity = world.spawn();
        world.add(entity, Position { x: i as f32, y: 0.0, z: 0.0 });
        world.add(entity, Transform::identity());
        world.add(entity, Renderable { mesh_id: i as u32 % 10, material_id: i as u32 % 5 });
    }

    // 20% have Position + Velocity only (particles)
    for i in (entity_count * 8) / 10..entity_count {
        let entity = world.spawn();
        world.add(entity, Position { x: i as f32, y: 0.0, z: 0.0 });
        world.add(entity, Velocity { x: 1.0, y: 0.0, z: 0.0 });
    }

    world
}

/// Simulate realistic game loop operations.
fn game_loop_simulation(world: &mut World) {
    // 1. Physics update (hot path)
    for (_entity, (pos, vel)) in world.query_mut::<(&mut Position, &Velocity)>() {
        pos.x += vel.x * 0.016;
        pos.y += vel.y * 0.016;
        pos.z += vel.z * 0.016;
    }

    // 2. Health regeneration (conditional logic)
    for (_entity, health) in world.query_mut::<&mut Health>() {
        if health.current < health.max {
            health.current = (health.current + 0.5).min(health.max);
        }
    }

    // 3. Render query (common access pattern)
    let mut count = 0;
    for (_entity, (pos, renderable)) in world.query::<(&Position, &Renderable)>() {
        black_box(pos);
        black_box(renderable);
        count += 1;
    }
    black_box(count);

    // 4. Transform updates
    for (_entity, (pos, transform)) in world.query_mut::<(&Position, &mut Transform)>() {
        transform.position.x = pos.x;
        transform.position.y = pos.y;
        transform.position.z = pos.z;
    }
}

/// Benchmark game loop at various entity counts (1K, 10K, 100K).
fn bench_pgo_game_loop(c: &mut Criterion) {
    let mut group = c.benchmark_group("pgo_game_loop");

    for entity_count in [1_000, 10_000, 100_000].iter() {
        group.throughput(Throughput::Elements(*entity_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(entity_count),
            entity_count,
            |b, &count| {
                let mut world = create_mixed_world(count);
                b.iter(|| {
                    game_loop_simulation(black_box(&mut world));
                });
            },
        );
    }

    group.finish();
}

/// Benchmark entity spawn/despawn patterns.
fn bench_pgo_entity_churn(c: &mut Criterion) {
    let mut group = c.benchmark_group("pgo_entity_churn");

    for batch_size in [10, 100, 1_000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(batch_size), batch_size, |b, &size| {
            b.iter_batched(
                || {
                    let mut world = World::new();
                    world.register::<Position>();
                    world.register::<Velocity>();
                    world.register::<Health>();
                    world
                },
                |mut world| {
                    // Spawn entities
                    let entities: Vec<_> = (0..size)
                        .map(|i| {
                            let entity = world.spawn();
                            world.add(entity, Position { x: i as f32, y: 0.0, z: 0.0 });
                            world.add(entity, Velocity { x: 1.0, y: 0.0, z: 0.0 });
                            world.add(entity, Health { current: 100.0, max: 100.0 });
                            entity
                        })
                        .collect();

                    // Despawn half
                    for entity in entities.iter().step_by(2) {
                        world.despawn(*entity);
                    }

                    black_box(world);
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

/// Benchmark various query patterns.
fn bench_pgo_query_patterns(c: &mut Criterion) {
    let mut world = create_mixed_world(10_000);

    c.bench_function("pgo_query_single_component", |b| {
        b.iter(|| {
            let mut sum: f32 = 0.0;
            for (_entity, pos) in world.query::<&Position>() {
                sum += pos.x + pos.y + pos.z;
            }
            black_box(sum);
        });
    });

    c.bench_function("pgo_query_two_components", |b| {
        b.iter(|| {
            let mut sum: f32 = 0.0;
            for (_entity, (pos, vel)) in world.query::<(&Position, &Velocity)>() {
                sum += pos.x * vel.x + pos.y * vel.y + pos.z * vel.z;
            }
            black_box(sum);
        });
    });

    c.bench_function("pgo_query_all_components", |b| {
        b.iter(|| {
            let mut count = 0;
            for (_entity, (_pos, _vel, _health, _transform, _renderable)) in
                world.query::<(&Position, &Velocity, &Health, &Transform, &Renderable)>()
            {
                count += 1;
            }
            black_box(count);
        });
    });

    c.bench_function("pgo_query_mutable", |b| {
        b.iter(|| {
            for (_entity, (pos, vel)) in world.query_mut::<(&mut Position, &Velocity)>() {
                pos.x += vel.x;
                pos.y += vel.y;
                pos.z += vel.z;
            }
        });
    });
}

/// Benchmark component add/remove patterns.
fn bench_pgo_component_operations(c: &mut Criterion) {
    c.bench_function("pgo_add_remove_component", |b| {
        b.iter_batched(
            || {
                let mut world = World::new();
                world.register::<Position>();
                world.register::<Velocity>();
                let entities: Vec<_> = (0..100).map(|_| world.spawn()).collect();
                (world, entities)
            },
            |(mut world, entities)| {
                // Add components
                for entity in &entities {
                    world.add(*entity, Position { x: 1.0, y: 2.0, z: 3.0 });
                    world.add(*entity, Velocity { x: 1.0, y: 0.0, z: 0.0 });
                }

                // Remove components from half
                for entity in entities.iter().step_by(2) {
                    world.remove::<Velocity>(*entity);
                }

                black_box(world);
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

criterion_group!(
    benches,
    bench_pgo_game_loop,
    bench_pgo_entity_churn,
    bench_pgo_query_patterns,
    bench_pgo_component_operations
);

criterion_main!(benches);
