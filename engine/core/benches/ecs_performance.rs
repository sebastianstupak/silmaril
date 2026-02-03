//! Comprehensive ECS Performance Benchmarks
//!
//! Compares silmaril ECS performance against industry standards:
//! - Bevy ECS (Rust, archetype-based)
//! - hecs (Rust, minimalist)
//! - EnTT (C++, reference: 0.8ns/entity iteration, 4.9ns/entity creation)
//! - Flecs (C/C++, query-focused)
//!
//! Based on research in PLATFORM_BENCHMARK_COMPARISON.md
//!
//! # Performance Targets
//!
//! ## Entity Creation
//! - **Target**: <1µs per entity
//! - **Industry**: EnTT 4.9ns/entity, Bevy/hecs similar range
//!
//! ## Component Iteration
//! - **Target**: 10M+ entities/sec (100ns/entity max)
//! - **Industry**: EnTT 0.8ns/entity (1 component), 4.2ns/entity (2 components)
//!
//! ## Component Operations
//! - **Add/Remove**: <1µs per operation
//! - **Get/GetMut**: <20ns per operation
//!
//! # Running Benchmarks
//!
//! ```bash
//! # Run all ECS performance benchmarks
//! cargo bench --bench ecs_performance
//!
//! # Run specific benchmark group
//! cargo bench --bench ecs_performance -- entity_creation
//! cargo bench --bench ecs_performance -- component_iteration
//! cargo bench --bench ecs_performance -- component_operations
//! cargo bench --bench ecs_performance -- query_performance
//! cargo bench --bench ecs_performance -- archetype_changes
//!
//! # Generate comparison report
//! cargo bench --bench ecs_performance -- --save-baseline main
//! ```
//!
//! # Interpreting Results
//!
//! Compare results against industry baselines in the summary table.
//! Green: Within target, Yellow: Above target but acceptable, Red: Needs optimization.

use criterion::{
    black_box, criterion_group, criterion_main, BenchmarkId, Criterion, PlotConfiguration,
    Throughput,
};
use engine_core::ecs::{Component, World};
use engine_core::math::{Transform, Vec3};
use engine_core::physics_components::Velocity;

// ============================================================================
// Component Definitions
// ============================================================================

#[derive(Debug, Clone, Copy)]
struct Health {
    current: f32,
    max: f32,
}
impl Component for Health {}

#[derive(Debug, Clone, Copy)]
struct Damage {
    #[allow(dead_code)]
    amount: f32,
    #[allow(dead_code)]
    damage_type: u8,
}
impl Component for Damage {}

#[derive(Debug, Clone, Copy)]
struct Player {
    #[allow(dead_code)]
    id: u32,
    #[allow(dead_code)]
    level: u16,
}
impl Component for Player {}

#[derive(Debug, Clone, Copy)]
struct Enemy {
    ai_state: u8,
    #[allow(dead_code)]
    aggro_range: f32,
}
impl Component for Enemy {}

#[derive(Debug, Clone, Copy)]
struct Projectile {
    #[allow(dead_code)]
    damage: f32,
    #[allow(dead_code)]
    speed: f32,
}
impl Component for Projectile {}

#[derive(Debug, Clone, Copy)]
struct Name {
    #[allow(dead_code)]
    id: u64,
}
impl Component for Name {}

#[derive(Debug, Clone, Copy)]
struct Sprite {
    #[allow(dead_code)]
    texture_id: u32,
}
impl Component for Sprite {}

#[derive(Debug, Clone, Copy)]
struct Collider {
    #[allow(dead_code)]
    radius: f32,
}
impl Component for Collider {}

// ============================================================================
// Helper Functions
// ============================================================================

fn setup_world() -> World {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Velocity>();
    world.register::<Health>();
    world.register::<Damage>();
    world.register::<Player>();
    world.register::<Enemy>();
    world.register::<Projectile>();
    world.register::<Name>();
    world.register::<Sprite>();
    world.register::<Collider>();
    world
}

// ============================================================================
// 1. Entity Creation Benchmarks
// ============================================================================
//
// Target: <1µs per entity
// Industry: EnTT 4.9ns/entity, Bevy/hecs similar
//
// Tests entity allocation performance at various scales.

fn bench_entity_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("1_entity_creation");
    group
        .plot_config(PlotConfiguration::default().summary_scale(criterion::AxisScale::Logarithmic));

    // Spawn 1M entities (measure time)
    for &count in &[1_000, 10_000, 100_000, 1_000_000] {
        group.throughput(Throughput::Elements(count));

        group.bench_with_input(BenchmarkId::new("spawn_entities", count), &count, |b, &count| {
            b.iter_batched(
                || setup_world(),
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

fn bench_entity_creation_with_components(c: &mut Criterion) {
    let mut group = c.benchmark_group("1_entity_creation_with_components");
    group
        .plot_config(PlotConfiguration::default().summary_scale(criterion::AxisScale::Logarithmic));

    // Spawn entities with 1, 2, 3 components
    for &count in &[1_000, 10_000, 100_000] {
        group.throughput(Throughput::Elements(count));

        // 1 component
        group.bench_with_input(
            BenchmarkId::new("spawn_1_component", count),
            &count,
            |b, &count| {
                b.iter_batched(
                    || setup_world(),
                    |mut world| {
                        for _i in 0..count {
                            let entity = world.spawn();
                            world.add(entity, Transform::default());
                            black_box(entity);
                        }
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );

        // 2 components
        group.bench_with_input(
            BenchmarkId::new("spawn_2_components", count),
            &count,
            |b, &count| {
                b.iter_batched(
                    || setup_world(),
                    |mut world| {
                        for i in 0..count {
                            let entity = world.spawn();
                            world.add(entity, Transform::default());
                            world.add(entity, Velocity { x: i as f32, y: 0.0, z: 0.0 });
                            black_box(entity);
                        }
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );

        // 3 components
        group.bench_with_input(
            BenchmarkId::new("spawn_3_components", count),
            &count,
            |b, &count| {
                b.iter_batched(
                    || setup_world(),
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
            },
        );
    }

    group.finish();
}

// ============================================================================
// 2. Component Iteration Benchmarks
// ============================================================================
//
// Target: 10M+ entities/sec (100ns/entity max)
// Industry: EnTT 0.8ns/entity (1 component), 4.2ns/entity (2 components)
//
// Tests iteration performance with different component counts.

fn bench_component_iteration(c: &mut Criterion) {
    let mut group = c.benchmark_group("2_component_iteration");
    group
        .plot_config(PlotConfiguration::default().summary_scale(criterion::AxisScale::Logarithmic));

    for &count in &[1_000, 10_000, 100_000, 1_000_000] {
        group.throughput(Throughput::Elements(count));

        // Iterate 1 component
        group.bench_with_input(
            BenchmarkId::new("iterate_1_component", count),
            &count,
            |b, &count| {
                let mut world = setup_world();
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
            },
        );

        // Iterate 2 components
        group.bench_with_input(
            BenchmarkId::new("iterate_2_components", count),
            &count,
            |b, &count| {
                let mut world = setup_world();
                for i in 0..count {
                    let entity = world.spawn();
                    let mut transform = Transform::default();
                    transform.position = Vec3::new(i as f32, 0.0, 0.0);
                    world.add(entity, transform);
                    world.add(entity, Velocity { x: 1.0, y: 0.0, z: 0.0 });
                }

                b.iter(|| {
                    let mut sum = 0.0f32;
                    for (_entity, (transform, velocity)) in world.query::<(&Transform, &Velocity)>()
                    {
                        sum += black_box(transform.position.x + velocity.x);
                    }
                    black_box(sum);
                });
            },
        );

        // Iterate 3 components
        group.bench_with_input(
            BenchmarkId::new("iterate_3_components", count),
            &count,
            |b, &count| {
                let mut world = setup_world();
                for i in 0..count {
                    let entity = world.spawn();
                    let mut transform = Transform::default();
                    transform.position = Vec3::new(i as f32, 0.0, 0.0);
                    world.add(entity, transform);
                    world.add(entity, Velocity { x: 1.0, y: 0.0, z: 0.0 });
                    world.add(entity, Health { current: 100.0, max: 100.0 });
                }

                b.iter(|| {
                    let mut sum = 0.0f32;
                    for (_entity, (transform, velocity, health)) in
                        world.query::<(&Transform, &Velocity, &Health)>()
                    {
                        sum += black_box(transform.position.x + velocity.x + health.current);
                    }
                    black_box(sum);
                });
            },
        );
    }

    group.finish();
}

fn bench_component_iteration_mutable(c: &mut Criterion) {
    let mut group = c.benchmark_group("2_component_iteration_mutable");
    group
        .plot_config(PlotConfiguration::default().summary_scale(criterion::AxisScale::Logarithmic));

    for &count in &[1_000, 10_000, 100_000, 1_000_000] {
        group.throughput(Throughput::Elements(count));

        // Mutable iteration with 2 components
        group.bench_with_input(
            BenchmarkId::new("iterate_mut_2_components", count),
            &count,
            |b, &count| {
                let mut world = setup_world();
                for i in 0..count {
                    let entity = world.spawn();
                    let mut transform = Transform::default();
                    transform.position = Vec3::new(i as f32, 0.0, 0.0);
                    world.add(entity, transform);
                    world.add(entity, Velocity { x: 1.0, y: 0.0, z: 0.0 });
                }

                b.iter(|| {
                    for (_entity, (transform, velocity)) in
                        world.query_mut::<(&mut Transform, &Velocity)>()
                    {
                        transform.position.x += velocity.x;
                        transform.position.y += velocity.y;
                        transform.position.z += velocity.z;
                    }
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// 3. Component Addition/Removal Benchmarks
// ============================================================================
//
// Target: <1µs per operation
// Industry: EnTT similar range
//
// Tests component add/remove performance.

fn bench_component_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("3_component_operations");

    // Component add
    group.bench_function("component_add_single", |b| {
        let mut world = setup_world();
        let entities: Vec<_> = (0..10_000).map(|_| world.spawn()).collect();
        let mut idx = 0;

        b.iter(|| {
            world.add(entities[idx % entities.len()], Health { current: 100.0, max: 100.0 });
            idx += 1;
        });
    });

    // Component remove
    group.bench_function("component_remove_single", |b| {
        b.iter_batched(
            || {
                let mut world = setup_world();
                let entities: Vec<_> = (0..1_000)
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
                    black_box(world.remove::<Health>(entity));
                }
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // Component get (immutable)
    group.bench_function("component_get", |b| {
        let mut world = setup_world();
        let entities: Vec<_> = (0..10_000)
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

    // Component get_mut
    group.bench_function("component_get_mut", |b| {
        let mut world = setup_world();
        let entities: Vec<_> = (0..10_000)
            .map(|i| {
                let e = world.spawn();
                world.add(e, Health { current: i as f32, max: 100.0 });
                e
            })
            .collect();

        let mut idx = 0;
        b.iter(|| {
            if let Some(health) = world.get_mut::<Health>(entities[idx % entities.len()]) {
                health.current += 1.0;
            }
            idx += 1;
        });
    });

    // Batch add (add multiple components to same entity)
    group.bench_function("component_add_batch_3", |b| {
        let mut world = setup_world();
        let entities: Vec<_> = (0..1_000).map(|_| world.spawn()).collect();
        let mut idx = 0;

        b.iter(|| {
            let entity = entities[idx % entities.len()];
            world.add(entity, Transform::default());
            world.add(entity, Velocity { x: 1.0, y: 0.0, z: 0.0 });
            world.add(entity, Health { current: 100.0, max: 100.0 });
            idx += 1;
        });
    });

    group.finish();
}

// ============================================================================
// 4. Query Performance Benchmarks
// ============================================================================
//
// Tests query performance with different component combinations and filters.

fn bench_query_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("4_query_performance");
    group
        .plot_config(PlotConfiguration::default().summary_scale(criterion::AxisScale::Logarithmic));

    // Simple query (all entities have components)
    for &count in &[1_000, 10_000, 100_000] {
        group.throughput(Throughput::Elements(count));

        group.bench_with_input(
            BenchmarkId::new("query_simple_100_percent", count),
            &count,
            |b, &count| {
                let mut world = setup_world();
                for i in 0..count {
                    let entity = world.spawn();
                    world.add(entity, Transform::default());
                    world.add(entity, Velocity { x: i as f32, y: 0.0, z: 0.0 });
                }

                b.iter(|| {
                    let mut sum = 0.0f32;
                    for (_entity, (transform, velocity)) in world.query::<(&Transform, &Velocity)>()
                    {
                        sum += black_box(transform.position.x + velocity.x);
                    }
                    black_box(sum);
                });
            },
        );
    }

    // Sparse query (only 10% of entities match)
    for &count in &[1_000, 10_000, 100_000] {
        group.throughput(Throughput::Elements(count / 10)); // Only 10% match

        group.bench_with_input(
            BenchmarkId::new("query_sparse_10_percent", count),
            &count,
            |b, &count| {
                let mut world = setup_world();
                for i in 0..count {
                    let entity = world.spawn();
                    world.add(entity, Transform::default());

                    // Only 10% have Player component
                    if i % 10 == 0 {
                        world.add(entity, Player { id: i as u32, level: 1 });
                    }
                }

                b.iter(|| {
                    let mut found = 0;
                    for (_entity, (_transform, _player)) in world.query::<(&Transform, &Player)>() {
                        found += 1;
                    }
                    black_box(found);
                });
            },
        );
    }

    // Complex query (4 components)
    for &count in &[1_000, 10_000, 100_000] {
        group.throughput(Throughput::Elements(count));

        group.bench_with_input(
            BenchmarkId::new("query_complex_4_components", count),
            &count,
            |b, &count| {
                let mut world = setup_world();
                for i in 0..count {
                    let entity = world.spawn();
                    world.add(entity, Transform::default());
                    world.add(entity, Velocity { x: 1.0, y: 0.0, z: 0.0 });
                    world.add(entity, Health { current: 100.0, max: 100.0 });
                    world.add(entity, Player { id: i as u32, level: 1 });
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
            },
        );
    }

    group.finish();
}

// ============================================================================
// 5. Archetype Changes Benchmarks
// ============================================================================
//
// Measures the cost of archetype migration when adding/removing components.
// This is critical for archetype-based ECS systems.

fn bench_archetype_changes(c: &mut Criterion) {
    let mut group = c.benchmark_group("5_archetype_changes");

    // Add component (causes archetype migration)
    group.bench_function("archetype_add_component", |b| {
        b.iter_batched(
            || {
                let mut world = setup_world();
                let entities: Vec<_> = (0..1_000)
                    .map(|_| {
                        let e = world.spawn();
                        world.add(e, Transform::default());
                        world.add(e, Velocity { x: 1.0, y: 0.0, z: 0.0 });
                        e
                    })
                    .collect();
                (world, entities)
            },
            |(mut world, entities)| {
                // Adding Health causes archetype migration
                for entity in entities {
                    world.add(black_box(entity), Health { current: 100.0, max: 100.0 });
                }
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // Remove component (causes archetype migration)
    group.bench_function("archetype_remove_component", |b| {
        b.iter_batched(
            || {
                let mut world = setup_world();
                let entities: Vec<_> = (0..1_000)
                    .map(|_| {
                        let e = world.spawn();
                        world.add(e, Transform::default());
                        world.add(e, Velocity { x: 1.0, y: 0.0, z: 0.0 });
                        world.add(e, Health { current: 100.0, max: 100.0 });
                        e
                    })
                    .collect();
                (world, entities)
            },
            |(mut world, entities)| {
                // Removing Health causes archetype migration
                for entity in entities {
                    black_box(world.remove::<Health>(black_box(entity)));
                }
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // Multiple archetype changes (add then remove)
    group.bench_function("archetype_add_remove_cycle", |b| {
        b.iter_batched(
            || {
                let mut world = setup_world();
                let entities: Vec<_> = (0..1_000)
                    .map(|_| {
                        let e = world.spawn();
                        world.add(e, Transform::default());
                        e
                    })
                    .collect();
                (world, entities)
            },
            |(mut world, entities)| {
                // Add Velocity (archetype migration)
                for entity in &entities {
                    world.add(*entity, Velocity { x: 1.0, y: 0.0, z: 0.0 });
                }

                // Remove Velocity (archetype migration back)
                for entity in &entities {
                    black_box(world.remove::<Velocity>(*entity));
                }
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

// ============================================================================
// 6. Realistic Game Scenarios
// ============================================================================
//
// Simulates realistic game workloads to validate overall ECS performance.

fn bench_game_scenarios(c: &mut Criterion) {
    let mut group = c.benchmark_group("6_game_scenarios");

    // MMORPG scenario: 10,000 entities with mixed components
    group.bench_function("mmorpg_10k_entities_frame", |b| {
        let mut world = setup_world();

        // 6000 NPCs (enemies)
        for _ in 0..6_000 {
            let entity = world.spawn();
            world.add(entity, Transform::default());
            world.add(entity, Velocity { x: 1.0, y: 0.0, z: 0.0 });
            world.add(entity, Health { current: 50.0, max: 50.0 });
            world.add(entity, Enemy { ai_state: 0, aggro_range: 10.0 });
        }

        // 3000 projectiles
        for _ in 0..3_000 {
            let entity = world.spawn();
            world.add(entity, Transform::default());
            world.add(entity, Velocity { x: 10.0, y: 0.0, z: 0.0 });
            world.add(entity, Projectile { damage: 10.0, speed: 20.0 });
        }

        // 1000 players
        for i in 0..1_000 {
            let entity = world.spawn();
            world.add(entity, Transform::default());
            world.add(entity, Velocity { x: 0.0, y: 0.0, z: 0.0 });
            world.add(entity, Health { current: 100.0, max: 100.0 });
            world.add(entity, Player { id: i, level: 1 });
        }

        b.iter(|| {
            let dt = 1.0 / 60.0; // 60 FPS

            // Movement system
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

            // Health regeneration
            for (_entity, health) in world.query_mut::<&mut Health>() {
                if health.current < health.max {
                    health.current = (health.current + 0.1).min(health.max);
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
    name = entity_creation;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(std::time::Duration::from_secs(10));
    targets = bench_entity_creation, bench_entity_creation_with_components
);

criterion_group!(
    name = component_iteration;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(std::time::Duration::from_secs(10));
    targets = bench_component_iteration, bench_component_iteration_mutable
);

criterion_group!(
    name = component_operations;
    config = Criterion::default()
        .sample_size(1000)
        .measurement_time(std::time::Duration::from_secs(10));
    targets = bench_component_operations
);

criterion_group!(
    name = query_performance;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(std::time::Duration::from_secs(10));
    targets = bench_query_performance
);

criterion_group!(
    name = archetype_changes;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(std::time::Duration::from_secs(10));
    targets = bench_archetype_changes
);

criterion_group!(
    name = game_scenarios;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(std::time::Duration::from_secs(10));
    targets = bench_game_scenarios
);

criterion_main!(
    entity_creation,
    component_iteration,
    component_operations,
    query_performance,
    archetype_changes,
    game_scenarios
);
