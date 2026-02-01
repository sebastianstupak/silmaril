//! Comprehensive ECS Query System Benchmarks
//!
//! Exhaustive coverage for optimization validation:
//! - All query types (1-12 components)
//! - Mixed mutability queries
//! - Optional component queries
//! - Query filters (.with(), .without())
//! - Sparse vs dense component distributions
//! - Entity churn scenarios
//! - Large scale operations (100k+ entities)
//! - Real-world game system simulations
//! - Baseline comparisons (Vec, HashMap)

#![allow(dead_code)]

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_core::ecs::{Component, World};
use std::collections::HashMap;

// ============================================================
// Test Components
// ============================================================

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
struct Acceleration {
    x: f32,
    y: f32,
    z: f32,
}
impl Component for Acceleration {}

#[derive(Debug, Clone, Copy)]
struct Mass {
    value: f32,
}
impl Component for Mass {}

#[derive(Debug, Clone, Copy)]
struct Health {
    current: f32,
    max: f32,
}
impl Component for Health {}

#[derive(Debug, Clone, Copy)]
struct Armor {
    value: f32,
}
impl Component for Armor {}

#[derive(Debug, Clone, Copy)]
struct Team {
    id: u32,
}
impl Component for Team {}

#[derive(Debug, Clone, Copy)]
struct Transform {
    position: [f32; 3],
    rotation: [f32; 4],
    scale: [f32; 3],
}
impl Component for Transform {}

#[derive(Debug, Clone, Copy)]
struct Mesh {
    id: u64,
}
impl Component for Mesh {}

#[derive(Debug, Clone, Copy)]
struct Material {
    id: u64,
}
impl Component for Material {}

#[derive(Debug, Clone, Copy)]
struct Visibility {
    visible: bool,
}
impl Component for Visibility {}

#[derive(Debug, Clone, Copy)]
struct Target {
    entity_id: u32,
}
impl Component for Target {}

#[derive(Debug, Clone, Copy)]
struct NavMesh {
    zone_id: u32,
}
impl Component for NavMesh {}

#[derive(Debug, Clone, Copy)]
struct AIState {
    state: u32,
}
impl Component for AIState {}

#[derive(Debug, Clone, Copy)]
struct Lifetime {
    remaining: f32,
}
impl Component for Lifetime {}

#[derive(Debug, Clone, Copy)]
struct Projectile;
impl Component for Projectile {}

#[derive(Debug, Clone, Copy)]
struct Player;
impl Component for Player {}

#[derive(Debug, Clone, Copy)]
struct Enemy;
impl Component for Enemy {}

// ============================================================
// World Setup Utilities
// ============================================================

fn setup_world_n_components(entity_count: usize, component_count: usize) -> World {
    let mut world = World::new();

    // Register all components
    world.register::<Position>();
    world.register::<Velocity>();
    world.register::<Acceleration>();
    world.register::<Mass>();
    world.register::<Health>();
    world.register::<Armor>();
    world.register::<Team>();
    world.register::<Transform>();
    world.register::<Mesh>();
    world.register::<Material>();
    world.register::<Visibility>();
    world.register::<Target>();

    for i in 0..entity_count {
        let e = world.spawn();

        // Add components based on count
        if component_count >= 1 {
            world.add(e, Position { x: i as f32, y: 0.0, z: 0.0 });
        }
        if component_count >= 2 {
            world.add(e, Velocity { x: 1.0, y: 0.0, z: 0.0 });
        }
        if component_count >= 3 {
            world.add(e, Acceleration { x: 0.1, y: 0.0, z: 0.0 });
        }
        if component_count >= 4 {
            world.add(e, Mass { value: 1.0 });
        }
        if component_count >= 5 {
            world.add(e, Health { current: 100.0, max: 100.0 });
        }
        if component_count >= 6 {
            world.add(e, Armor { value: 10.0 });
        }
        if component_count >= 7 {
            world.add(e, Team { id: i as u32 % 4 });
        }
        if component_count >= 8 {
            world.add(
                e,
                Transform {
                    position: [i as f32, 0.0, 0.0],
                    rotation: [0.0, 0.0, 0.0, 1.0],
                    scale: [1.0, 1.0, 1.0],
                },
            );
        }
        if component_count >= 9 {
            world.add(e, Mesh { id: i as u64 % 100 });
        }
        if component_count >= 10 {
            world.add(e, Material { id: i as u64 % 50 });
        }
        if component_count >= 11 {
            world.add(e, Visibility { visible: true });
        }
        if component_count >= 12 {
            world.add(e, Target { entity_id: (i + 1) as u32 });
        }
    }

    world
}

fn setup_world_sparse(entity_count: usize, density: f32) -> World {
    let mut world = World::new();
    world.register::<Position>();
    world.register::<Velocity>();
    world.register::<Health>();

    for i in 0..entity_count {
        let e = world.spawn();
        world.add(e, Position { x: i as f32, y: 0.0, z: 0.0 });

        // Add Velocity based on density
        if (i as f32 / entity_count as f32) < density {
            world.add(e, Velocity { x: 1.0, y: 0.0, z: 0.0 });
        }

        // Add Health to even fewer entities
        if (i as f32 / entity_count as f32) < (density * 0.5) {
            world.add(e, Health { current: 100.0, max: 100.0 });
        }
    }

    world
}

// ============================================================
// 1. Query Type Benchmarks (1-12 components)
// ============================================================

fn bench_query_1_component(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_1_component");

    for size in [1000, 10000, 100000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let world = setup_world_n_components(size, 1);
            b.iter(|| {
                let mut sum = 0.0;
                for (_e, pos) in world.query::<&Position>() {
                    sum += black_box(pos.x);
                }
                black_box(sum);
            });
        });
    }

    group.finish();
}

fn bench_query_2_components(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_2_components");

    for size in [1000, 10000, 100000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let world = setup_world_n_components(size, 2);
            b.iter(|| {
                let mut sum = 0.0;
                for (_e, (pos, vel)) in world.query::<(&Position, &Velocity)>() {
                    sum += black_box(pos.x + vel.x);
                }
                black_box(sum);
            });
        });
    }

    group.finish();
}

fn bench_query_3_components(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_3_components");

    for size in [1000, 10000, 100000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let world = setup_world_n_components(size, 3);
            b.iter(|| {
                let mut sum = 0.0;
                for (_e, (pos, vel, acc)) in world.query::<(&Position, &Velocity, &Acceleration)>()
                {
                    sum += black_box(pos.x + vel.x + acc.x);
                }
                black_box(sum);
            });
        });
    }

    group.finish();
}

fn bench_query_4_components(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_4_components");

    for size in [1000, 10000, 100000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let world = setup_world_n_components(size, 4);
            b.iter(|| {
                let mut sum = 0.0;
                for (_e, (pos, vel, acc, mass)) in
                    world.query::<(&Position, &Velocity, &Acceleration, &Mass)>()
                {
                    sum += black_box(pos.x + vel.x + acc.x + mass.value);
                }
                black_box(sum);
            });
        });
    }

    group.finish();
}

fn bench_query_5_components(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_5_components");

    for size in [1000, 10000, 50000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let world = setup_world_n_components(size, 5);
            b.iter(|| {
                let mut sum = 0.0;
                for (_e, (pos, vel, acc, mass, health)) in
                    world.query::<(&Position, &Velocity, &Acceleration, &Mass, &Health)>()
                {
                    sum += black_box(pos.x + vel.x + acc.x + mass.value + health.current);
                }
                black_box(sum);
            });
        });
    }

    group.finish();
}

fn bench_query_8_components(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_8_components");

    for size in [1000, 10000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let world = setup_world_n_components(size, 8);
            b.iter(|| {
                let mut sum = 0.0;
                for (_e, (pos, vel, acc, mass, health, armor, team, _transform)) in world.query::<(
                    &Position,
                    &Velocity,
                    &Acceleration,
                    &Mass,
                    &Health,
                    &Armor,
                    &Team,
                    &Transform,
                )>(
                ) {
                    sum += black_box(
                        pos.x
                            + vel.x
                            + acc.x
                            + mass.value
                            + health.current
                            + armor.value
                            + team.id as f32,
                    );
                }
                black_box(sum);
            });
        });
    }

    group.finish();
}

fn bench_query_12_components(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_12_components");

    for size in [1000, 5000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let world = setup_world_n_components(size, 12);
            b.iter(|| {
                let mut sum = 0.0;
                for (_e, (pos, vel, acc, mass, health, armor, team, _t, _m, _mat, _vis, _tgt)) in
                    world.query::<(
                        &Position,
                        &Velocity,
                        &Acceleration,
                        &Mass,
                        &Health,
                        &Armor,
                        &Team,
                        &Transform,
                        &Mesh,
                        &Material,
                        &Visibility,
                        &Target,
                    )>()
                {
                    sum += black_box(
                        pos.x
                            + vel.x
                            + acc.x
                            + mass.value
                            + health.current
                            + armor.value
                            + team.id as f32,
                    );
                }
                black_box(sum);
            });
        });
    }

    group.finish();
}

// ============================================================
// 2. Mixed Mutability Benchmarks
// ============================================================

fn bench_query_mixed_mut_immut(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_mixed_mutability");

    for size in [1000, 10000, 50000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::new("mut_immut", size), size, |b, &size| {
            b.iter_batched(
                || setup_world_n_components(size, 3),
                |mut world| {
                    for (_e, (pos, vel, acc)) in
                        world.query_mut::<(&mut Position, &mut Velocity, &mut Acceleration)>()
                    {
                        pos.x += black_box(vel.x + acc.x);
                    }
                },
                criterion::BatchSize::LargeInput,
            );
        });
    }

    for size in [1000, 10000, 50000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::new("all_mut", size), size, |b, &size| {
            b.iter_batched(
                || setup_world_n_components(size, 3),
                |mut world| {
                    for (_e, (pos, vel, acc)) in
                        world.query_mut::<(&mut Position, &mut Velocity, &mut Acceleration)>()
                    {
                        pos.x += vel.x;
                        vel.x += acc.x;
                        acc.x *= 0.99;
                    }
                },
                criterion::BatchSize::LargeInput,
            );
        });
    }

    group.finish();
}

// ============================================================
// 3. Optional Component Benchmarks
// ============================================================

fn bench_query_optional(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_optional");

    for size in [1000, 10000, 50000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let world = setup_world_sparse(size, 0.5);
            b.iter(|| {
                let mut sum = 0.0;
                for (_e, vel_opt) in world.query::<Option<&Velocity>>() {
                    if let Some(vel) = vel_opt {
                        sum += black_box(vel.x);
                    }
                }
                black_box(sum);
            });
        });
    }

    group.finish();
}

// ============================================================
// 4. Query Filter Benchmarks
// ============================================================

fn bench_query_filters(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_filters");

    // .with() filter
    for size in [1000, 10000, 50000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::new("with_filter", size), size, |b, &size| {
            let world = setup_world_sparse(size, 0.5);
            b.iter(|| {
                let mut sum = 0.0;
                for (_e, pos) in world.query::<&Position>().with::<Velocity>() {
                    sum += black_box(pos.x);
                }
                black_box(sum);
            });
        });
    }

    // .without() filter
    for size in [1000, 10000, 50000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::new("without_filter", size), size, |b, &size| {
            let world = setup_world_sparse(size, 0.5);
            b.iter(|| {
                let mut sum = 0.0;
                for (_e, pos) in world.query::<&Position>().without::<Health>() {
                    sum += black_box(pos.x);
                }
                black_box(sum);
            });
        });
    }

    // Nested filters (with + without)
    for size in [1000, 10000, 50000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::new("nested_filters", size), size, |b, &size| {
            let world = setup_world_sparse(size, 0.5);
            b.iter(|| {
                let mut sum = 0.0;
                for (_e, pos) in world.query::<&Position>().with::<Velocity>().without::<Health>() {
                    sum += black_box(pos.x);
                }
                black_box(sum);
            });
        });
    }

    group.finish();
}

// ============================================================
// 5. Sparse vs Dense Distribution
// ============================================================

fn bench_sparse_vs_dense(c: &mut Criterion) {
    let mut group = c.benchmark_group("sparse_vs_dense");

    // Dense (100% have both components)
    group.bench_function("dense_100pct", |b| {
        let world = setup_world_sparse(10000, 1.0);
        b.iter(|| {
            let mut count = 0;
            for (_e, (_pos, _vel)) in world.query::<(&Position, &Velocity)>() {
                count += 1;
            }
            black_box(count);
        });
    });

    // Semi-dense (50%)
    group.bench_function("sparse_50pct", |b| {
        let world = setup_world_sparse(10000, 0.5);
        b.iter(|| {
            let mut count = 0;
            for (_e, (_pos, _vel)) in world.query::<(&Position, &Velocity)>() {
                count += 1;
            }
            black_box(count);
        });
    });

    // Sparse (10%)
    group.bench_function("sparse_10pct", |b| {
        let world = setup_world_sparse(10000, 0.1);
        b.iter(|| {
            let mut count = 0;
            for (_e, (_pos, _vel)) in world.query::<(&Position, &Velocity)>() {
                count += 1;
            }
            black_box(count);
        });
    });

    // Very sparse (1%)
    group.bench_function("sparse_1pct", |b| {
        let world = setup_world_sparse(10000, 0.01);
        b.iter(|| {
            let mut count = 0;
            for (_e, (_pos, _vel)) in world.query::<(&Position, &Velocity)>() {
                count += 1;
            }
            black_box(count);
        });
    });

    group.finish();
}

// ============================================================
// 6. Entity Churn Benchmarks
// ============================================================

fn bench_entity_churn(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_churn");

    group.bench_function("spawn_and_query", |b| {
        b.iter_batched(
            || {
                let mut world = World::new();
                world.register::<Position>();
                world.register::<Velocity>();
                world.register::<Lifetime>();
                world
            },
            |mut world| {
                // Spawn 1000 entities
                for i in 0..1000 {
                    let e = world.spawn();
                    world.add(e, Position { x: i as f32, y: 0.0, z: 0.0 });
                    world.add(e, Velocity { x: 1.0, y: 0.0, z: 0.0 });
                    world.add(e, Lifetime { remaining: 1.0 });
                }

                // Query all entities
                let mut sum = 0.0;
                for (_e, (pos, vel, _lt)) in world.query::<(&Position, &Velocity, &Lifetime)>() {
                    sum += pos.x + vel.x;
                }
                black_box(sum);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.bench_function("spawn_despawn_query", |b| {
        b.iter_batched(
            || {
                let mut world = World::new();
                world.register::<Position>();
                world.register::<Velocity>();
                world.register::<Lifetime>();

                // Pre-populate
                for i in 0..5000 {
                    let e = world.spawn();
                    world.add(e, Position { x: i as f32, y: 0.0, z: 0.0 });
                    world.add(e, Velocity { x: 1.0, y: 0.0, z: 0.0 });
                    world.add(e, Lifetime { remaining: (i % 10) as f32 });
                }
                world
            },
            |mut world| {
                // Collect entities to despawn
                let to_despawn: Vec<_> = world
                    .query::<&Lifetime>()
                    .filter_map(|(e, lt)| if lt.remaining < 1.0 { Some(e) } else { None })
                    .collect();

                // Despawn
                for e in to_despawn {
                    world.despawn(e);
                }

                // Spawn new entities
                for i in 0..500 {
                    let e = world.spawn();
                    world.add(e, Position { x: i as f32, y: 0.0, z: 0.0 });
                    world.add(e, Velocity { x: 1.0, y: 0.0, z: 0.0 });
                    world.add(e, Lifetime { remaining: 10.0 });
                }

                // Query
                let mut sum = 0.0;
                for (_e, (pos, vel)) in world.query::<(&Position, &Velocity)>() {
                    sum += pos.x + vel.x;
                }
                black_box(sum);
            },
            criterion::BatchSize::LargeInput,
        );
    });

    group.finish();
}

// ============================================================
// 7. Real-World Scenario Benchmarks
// ============================================================

fn bench_physics_simulation(c: &mut Criterion) {
    let mut group = c.benchmark_group("real_world_physics");

    for size in [1000, 10000, 50000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_batched(
                || setup_world_n_components(size, 4),
                |mut world| {
                    let dt = 0.016; // 60 FPS

                    // Physics update: acceleration -> velocity -> position
                    // Note: Current implementation doesn't support mixed mutability for 3+ components
                    // So we use all-mutable and read-only access
                    for (_e, (pos, vel, acc, mass)) in world.query_mut::<(
                        &mut Position,
                        &mut Velocity,
                        &mut Acceleration,
                        &mut Mass,
                    )>() {
                        // F = ma, so a = F/m
                        let force_scale = 1.0 / mass.value;
                        vel.x += acc.x * force_scale * dt;
                        vel.y += acc.y * force_scale * dt;
                        vel.z += acc.z * force_scale * dt;

                        pos.x += vel.x * dt;
                        pos.y += vel.y * dt;
                        pos.z += vel.z * dt;
                    }
                },
                criterion::BatchSize::LargeInput,
            );
        });
    }

    group.finish();
}

fn bench_damage_system(c: &mut Criterion) {
    let mut group = c.benchmark_group("real_world_damage");

    for size in [1000, 10000, 50000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_batched(
                || setup_world_n_components(size, 7),
                |mut world| {
                    // Damage system: apply area damage to nearby entities
                    let explosion_pos = Position { x: 50.0, y: 0.0, z: 0.0 };
                    let explosion_radius = 20.0;
                    let base_damage = 50.0;

                    for (_e, (health, armor, pos, _team)) in
                        world.query_mut::<(&mut Health, &mut Armor, &mut Position, &mut Team)>()
                    {
                        let dx = pos.x - explosion_pos.x;
                        let dy = pos.y - explosion_pos.y;
                        let dz = pos.z - explosion_pos.z;
                        let dist_sq = dx * dx + dy * dy + dz * dz;

                        if dist_sq < explosion_radius * explosion_radius {
                            let dist = dist_sq.sqrt();
                            let falloff = 1.0 - (dist / explosion_radius);
                            let damage = base_damage * falloff;
                            let mitigated_damage = damage * (1.0 - armor.value / 100.0);
                            health.current -= mitigated_damage;
                        }
                    }
                },
                criterion::BatchSize::LargeInput,
            );
        });
    }

    group.finish();
}

fn bench_rendering_system(c: &mut Criterion) {
    let mut group = c.benchmark_group("real_world_rendering");

    for size in [1000, 10000, 50000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let world = setup_world_n_components(size, 11);
            b.iter(|| {
                // Rendering: collect visible entities with mesh and material
                let mut render_batch_count = 0;

                for (_e, (_transform, _mesh, _material, visibility)) in
                    world.query::<(&Transform, &Mesh, &Material, &Visibility)>()
                {
                    if visibility.visible {
                        render_batch_count += 1;
                    }
                }
                black_box(render_batch_count);
            });
        });
    }

    group.finish();
}

fn bench_ai_pathfinding(c: &mut Criterion) {
    let mut group = c.benchmark_group("real_world_ai");

    for size in [1000, 5000, 10000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_batched(
                || {
                    let mut world = World::new();
                    world.register::<Position>();
                    world.register::<Target>();
                    world.register::<NavMesh>();
                    world.register::<AIState>();

                    for i in 0..size {
                        let e = world.spawn();
                        world.add(e, Position { x: i as f32, y: 0.0, z: 0.0 });
                        world.add(e, Target { entity_id: (i + 1) as u32 });
                        world.add(e, NavMesh { zone_id: i as u32 % 10 });
                        world.add(e, AIState { state: 0 });
                    }
                    world
                },
                |mut world| {
                    // AI pathfinding: move entities toward their targets
                    for (_e, (pos, target, _navmesh, state)) in
                        world
                            .query_mut::<(&mut Position, &mut Target, &mut NavMesh, &mut AIState)>()
                    {
                        // Simple pathfinding simulation
                        let target_pos_x = target.entity_id as f32 * 10.0;
                        let dx = target_pos_x - pos.x;

                        if dx.abs() > 0.1 {
                            pos.x += dx.signum() * 0.5;
                            state.state = 1; // Moving
                        } else {
                            state.state = 0; // Idle
                        }
                    }
                },
                criterion::BatchSize::LargeInput,
            );
        });
    }

    group.finish();
}

// ============================================================
// 8. Baseline Comparisons
// ============================================================

fn bench_baseline_vec(c: &mut Criterion) {
    let mut group = c.benchmark_group("baseline_vec");

    #[derive(Clone)]
    struct PhysicsData {
        position: Position,
        velocity: Velocity,
        acceleration: Acceleration,
    }

    for size in [1000, 10000, 100000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let data: Vec<PhysicsData> = (0..size)
                .map(|i| PhysicsData {
                    position: Position { x: i as f32, y: 0.0, z: 0.0 },
                    velocity: Velocity { x: 1.0, y: 0.0, z: 0.0 },
                    acceleration: Acceleration { x: 0.1, y: 0.0, z: 0.0 },
                })
                .collect();

            b.iter(|| {
                let mut sum = 0.0;
                for item in &data {
                    sum += black_box(item.position.x + item.velocity.x + item.acceleration.x);
                }
                black_box(sum);
            });
        });
    }

    group.finish();
}

fn bench_baseline_hashmap(c: &mut Criterion) {
    let mut group = c.benchmark_group("baseline_hashmap");

    for size in [1000, 10000, 100000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let mut positions: HashMap<u32, Position> = HashMap::new();
            let mut velocities: HashMap<u32, Velocity> = HashMap::new();
            let mut accelerations: HashMap<u32, Acceleration> = HashMap::new();

            for i in 0..size {
                positions.insert(i as u32, Position { x: i as f32, y: 0.0, z: 0.0 });
                velocities.insert(i as u32, Velocity { x: 1.0, y: 0.0, z: 0.0 });
                accelerations.insert(i as u32, Acceleration { x: 0.1, y: 0.0, z: 0.0 });
            }

            b.iter(|| {
                let mut sum = 0.0;
                for i in 0..size as u32 {
                    if let (Some(pos), Some(vel), Some(acc)) =
                        (positions.get(&i), velocities.get(&i), accelerations.get(&i))
                    {
                        sum += black_box(pos.x + vel.x + acc.x);
                    }
                }
                black_box(sum);
            });
        });
    }

    group.finish();
}

// ============================================================
// Criterion Groups
// ============================================================

criterion_group!(
    query_iteration,
    bench_query_1_component,
    bench_query_2_components,
    bench_query_3_components,
    bench_query_4_components,
    bench_query_5_components,
    bench_query_8_components,
    bench_query_12_components,
);

criterion_group!(query_mutability, bench_query_mixed_mut_immut, bench_query_optional,);

criterion_group!(filter_operations, bench_query_filters, bench_sparse_vs_dense,);

criterion_group!(world_operations, bench_entity_churn,);

criterion_group!(
    real_world_scenarios,
    bench_physics_simulation,
    bench_damage_system,
    bench_rendering_system,
    bench_ai_pathfinding,
);

criterion_group!(baseline_comparisons, bench_baseline_vec, bench_baseline_hashmap,);

criterion_main!(
    query_iteration,
    query_mutability,
    filter_operations,
    world_operations,
    real_world_scenarios,
    baseline_comparisons,
);
