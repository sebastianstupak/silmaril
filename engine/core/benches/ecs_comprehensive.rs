//! Comprehensive ECS benchmarks following Rust best practices
//!
//! Uses Criterion for statistical analysis with:
//! - Proper warmup
//! - Outlier detection
//! - Statistical significance testing
//! - Comparison with baseline
//!
//! Run with:
//! ```bash
//! cargo bench --bench ecs_comprehensive
//! ```

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_core::ecs::{Component, World};
use engine_core::math::{Transform, Vec3};
use engine_core::physics_components::Velocity;

// ============================================================================
// Component Definitions for Benchmarking
// ============================================================================

#[derive(Debug, Clone, Copy)]
struct Health {
    current: f32,
    max: f32,
}
impl Component for Health {}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
struct Damage {
    amount: f32,
}
impl Component for Damage {}

#[derive(Debug, Clone, Copy)]
struct Player {
    #[allow(dead_code)]
    id: u32,
}
impl Component for Player {}

#[derive(Debug, Clone, Copy)]
struct Enemy {
    ai_state: u8,
}
impl Component for Enemy {}

#[derive(Debug, Clone, Copy)]
struct Projectile {
    #[allow(dead_code)]
    damage: f32,
}
impl Component for Projectile {}

// ============================================================================
// Entity Spawning Benchmarks
// ============================================================================

fn bench_entity_spawning(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_spawning");

    // Target: 1M entities/sec (1μs per entity)
    for count in [100, 1_000, 10_000, 100_000].iter() {
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

fn bench_entity_spawning_with_components(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_spawning_with_components");

    for count in [100, 1_000, 10_000].iter() {
        group.throughput(Throughput::Elements(*count as u64));

        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            b.iter_batched(
                || World::new(),
                |mut world| {
                    for i in 0..count {
                        let entity = world.spawn();
                        world.add(entity, Transform::default());
                        world.add(entity, Velocity { x: i as f32, y: 0.0, z: 0.0 });
                        world.add(entity, Health { current: 100.0, max: 100.0 });
                        black_box(entity);
                    }
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

// ============================================================================
// Entity Iteration Benchmarks
// ============================================================================

fn bench_iterate_single_component(c: &mut Criterion) {
    let mut group = c.benchmark_group("iterate_single_component");

    // Target: 10M entities/frame at 60fps = 166M entities/sec
    for count in [1_000, 10_000, 100_000, 1_000_000].iter() {
        group.throughput(Throughput::Elements(*count as u64));

        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            // Setup: Create world with entities
            let mut world = World::new();
            for i in 0..count {
                let entity = world.spawn();
                let mut transform = Transform::default();
                transform.position = Vec3::new(i as f32, 0.0, 0.0);
                world.add(entity, transform);
            }

            b.iter(|| {
                let mut sum = 0.0f32;
                for (_entity, transform) in world.query::<&Transform>() {
                    sum += black_box(transform.position.x);
                }
                black_box(sum);
            });
        });
    }

    group.finish();
}

fn bench_iterate_two_components(c: &mut Criterion) {
    let mut group = c.benchmark_group("iterate_two_components");

    for count in [1_000, 10_000, 100_000].iter() {
        group.throughput(Throughput::Elements(*count as u64));

        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            let mut world = World::new();
            for i in 0..count {
                let entity = world.spawn();
                let mut transform = Transform::default();
                transform.position = Vec3::new(i as f32, 0.0, 0.0);
                world.add(entity, transform);
                world.add(entity, Velocity { x: 1.0, y: 0.0, z: 0.0 });
            }

            b.iter(|| {
                let mut sum = 0.0f32;
                for (_entity, (transform, velocity)) in world.query::<(&Transform, &Velocity)>() {
                    sum += black_box(transform.position.x + velocity.x);
                }
                black_box(sum);
            });
        });
    }

    group.finish();
}

fn bench_iterate_four_components(c: &mut Criterion) {
    let mut group = c.benchmark_group("iterate_four_components");

    for count in [1_000, 10_000, 100_000].iter() {
        group.throughput(Throughput::Elements(*count as u64));

        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            let mut world = World::new();
            for i in 0..count {
                let entity = world.spawn();
                world.add(entity, Transform::default());
                world.add(entity, Velocity { x: 1.0, y: 0.0, z: 0.0 });
                world.add(entity, Health { current: 100.0, max: 100.0 });
                world.add(entity, Player { id: i as u32 });
            }

            b.iter(|| {
                let mut sum = 0.0f32;
                for (_entity, (_t, v, h, _p)) in
                    world.query::<(&Transform, &Velocity, &Health, &Player)>()
                {
                    sum += black_box(v.x + h.current);
                }
                black_box(sum);
            });
        });
    }

    group.finish();
}

// ============================================================================
// Component Operations Benchmarks
// ============================================================================

fn bench_component_add(c: &mut Criterion) {
    let mut group = c.benchmark_group("component_add");

    // Target: <100ns per operation
    group.bench_function("add_single_component", |b| {
        let mut world = World::new();
        let entities: Vec<_> = (0..1000).map(|_| world.spawn()).collect();
        let mut idx = 0;

        b.iter(|| {
            world.add(entities[idx % entities.len()], Health { current: 100.0, max: 100.0 });
            idx += 1;
        });
    });

    group.finish();
}

fn bench_component_remove(c: &mut Criterion) {
    let mut group = c.benchmark_group("component_remove");

    // Target: <100ns per operation
    group.bench_function("remove_single_component", |b| {
        b.iter_batched(
            || {
                let mut world = World::new();
                let entities: Vec<_> = (0..1000)
                    .map(|_| {
                        let e = world.spawn();
                        world.add(e, Health { current: 100.0, max: 100.0 });
                        e
                    })
                    .collect();
                (world, entities)
            },
            |(mut world, entities)| {
                for entity in entities {
                    world.remove::<Health>(black_box(entity));
                }
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

fn bench_component_get(c: &mut Criterion) {
    let mut group = c.benchmark_group("component_get");

    // Target: <20ns per operation (pointer deref + bounds check)
    group.bench_function("get_single_component", |b| {
        let mut world = World::new();
        let entities: Vec<_> = (0..1000)
            .map(|i| {
                let e = world.spawn();
                world.add(e, Health { current: i as f32, max: 100.0 });
                e
            })
            .collect();

        let mut idx = 0;
        b.iter(|| {
            let health = world.get::<Health>(entities[idx % entities.len()]);
            idx += 1;
            black_box(health);
        });
    });

    group.finish();
}

// ============================================================================
// Query Performance Benchmarks
// ============================================================================

fn bench_query_filtering(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_filtering");

    // Test sparse queries (only 10% of entities match)
    for count in [1_000, 10_000, 100_000].iter() {
        group.throughput(Throughput::Elements(*count as u64));

        group.bench_with_input(BenchmarkId::new("sparse_10_percent", count), count, |b, &count| {
            let mut world = World::new();
            for i in 0..count {
                let entity = world.spawn();
                world.add(entity, Transform::default());

                // Only 10% have Player component
                if i % 10 == 0 {
                    world.add(entity, Player { id: i as u32 });
                }
            }

            b.iter(|| {
                let mut found = 0;
                for (_entity, (_transform, _player)) in world.query::<(&Transform, &Player)>() {
                    found += 1;
                }
                black_box(found);
            });
        });
    }

    group.finish();
}

// ============================================================================
// Memory Usage Benchmarks
// ============================================================================

fn bench_memory_per_entity(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_per_entity");

    // Target: ≤24 bytes per entity (Unity DOTS level)
    for count in [1_000, 10_000, 100_000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            b.iter(|| {
                let mut world = World::new();
                for _ in 0..count {
                    let entity = world.spawn();
                    black_box(entity);
                }
                // Memory usage measured by OS tools
                black_box(world);
            });
        });
    }

    group.finish();
}

// ============================================================================
// Realistic Game Scenarios
// ============================================================================

fn bench_game_simulation(c: &mut Criterion) {
    let mut group = c.benchmark_group("game_simulation");

    // Simulate realistic game: 1000 entities with mixed components
    group.bench_function("simulate_1000_entities_frame", |b| {
        let mut world = World::new();

        // 600 enemies
        for _ in 0..600 {
            let entity = world.spawn();
            world.add(entity, Transform::default());
            world.add(entity, Velocity { x: 1.0, y: 0.0, z: 0.0 });
            world.add(entity, Health { current: 50.0, max: 50.0 });
            world.add(entity, Enemy { ai_state: 0 });
        }

        // 300 projectiles
        for _ in 0..300 {
            let entity = world.spawn();
            world.add(entity, Transform::default());
            world.add(entity, Velocity { x: 10.0, y: 0.0, z: 0.0 });
            world.add(entity, Projectile { damage: 10.0 });
        }

        // 100 players
        for i in 0..100 {
            let entity = world.spawn();
            world.add(entity, Transform::default());
            world.add(entity, Velocity { x: 0.0, y: 0.0, z: 0.0 });
            world.add(entity, Health { current: 100.0, max: 100.0 });
            world.add(entity, Player { id: i });
        }

        b.iter(|| {
            // Simulate one frame of game logic
            let dt = 1.0 / 60.0;

            // Update positions
            for (_entity, (transform, velocity)) in world.query_mut::<(&mut Transform, &Velocity)>()
            {
                transform.position.x += velocity.x * dt;
                transform.position.y += velocity.y * dt;
                transform.position.z += velocity.z * dt;
            }

            // AI update
            for (_entity, enemy) in world.query_mut::<&mut Enemy>() {
                enemy.ai_state = (enemy.ai_state + 1) % 4;
            }

            // Health regen
            for (_entity, health) in world.query_mut::<&mut Health>() {
                if health.current < health.max {
                    health.current = (health.current + 1.0).min(health.max);
                }
            }

            black_box(&world);
        });
    });

    group.finish();
}

// ============================================================================
// Criterion Configuration
// ============================================================================

criterion_group!(
    name = entity_spawning;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(std::time::Duration::from_secs(10));
    targets = bench_entity_spawning, bench_entity_spawning_with_components
);

criterion_group!(
    name = entity_iteration;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(std::time::Duration::from_secs(10));
    targets = bench_iterate_single_component, bench_iterate_two_components, bench_iterate_four_components
);

criterion_group!(
    name = component_operations;
    config = Criterion::default()
        .sample_size(1000)
        .measurement_time(std::time::Duration::from_secs(10));
    targets = bench_component_add, bench_component_remove, bench_component_get
);

criterion_group!(
    name = query_performance;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(std::time::Duration::from_secs(10));
    targets = bench_query_filtering
);

criterion_group!(
    name = memory_benchmarks;
    config = Criterion::default()
        .sample_size(50)
        .measurement_time(std::time::Duration::from_secs(5));
    targets = bench_memory_per_entity
);

criterion_group!(
    name = game_scenarios;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(std::time::Duration::from_secs(10));
    targets = bench_game_simulation
);

criterion_main!(
    entity_spawning,
    entity_iteration,
    component_operations,
    query_performance,
    memory_benchmarks,
    game_scenarios
);
