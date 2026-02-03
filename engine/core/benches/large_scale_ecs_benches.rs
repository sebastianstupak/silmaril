//! Large-Scale ECS Benchmarks (50K-500K entities)
//!
//! Tests ECS performance at MMO scale to validate scalability.
//! Industry targets:
//! - Unity DOTS: 100K+ entities at 60 FPS
//! - Bevy: 200K+ entities at 60 FPS
//! - Unreal Mass Entity: 100K+ entities at 60 FPS
//!
//! Scenarios:
//! 1. Simple iteration (Position + Velocity)
//! 2. Complex queries (5+ components)
//! 3. Sparse queries (10% of entities match)
//! 4. Entity spawn/despawn at scale
//! 5. Component addition/removal at scale

#![allow(dead_code)] // Benchmark components may not use all fields

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_core::{
    ecs::{Component, Entity, World},
    Velocity,
};
use std::time::Duration;

// ============================================================================
// Components for Large-Scale Testing
// ============================================================================

#[derive(Debug, Clone, Copy)]
struct Position {
    x: f32,
    y: f32,
    z: f32,
}
impl Component for Position {}

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
struct Damage {
    value: f32,
}
impl Component for Damage {}

#[derive(Debug, Clone, Copy)]
struct Team {
    id: u32,
}
impl Component for Team {}

#[derive(Debug, Clone, Copy)]
struct AI {
    state: u32,
    target: u64,
}
impl Component for AI {}

#[derive(Debug, Clone, Copy)]
struct NetworkId {
    id: u64,
}
impl Component for NetworkId {}

// ============================================================================
// Scenario 1: Simple Iteration at Scale
// ============================================================================

fn setup_simple_world(entity_count: usize) -> World {
    let mut world = World::new();
    world.register::<Position>();
    world.register::<Velocity>();

    for i in 0..entity_count {
        let entity = world.spawn();
        world.add(entity, Position { x: (i % 1000) as f32, y: 0.0, z: (i / 1000) as f32 });
        world.add(
            entity,
            Velocity { x: ((i as f32).sin() * 10.0), y: 0.0, z: ((i as f32).cos() * 10.0) },
        );
    }

    world
}

fn physics_update(world: &mut World, dt: f32) {
    let entities: Vec<Entity> = world.entities().collect();

    for entity in entities {
        let vel = match world.get::<Velocity>(entity) {
            Some(v) => *v,
            None => continue,
        };

        if let Some(pos) = world.get_mut::<Position>(entity) {
            pos.x += vel.x * dt;
            pos.y += vel.y * dt;
            pos.z += vel.z * dt;
        }
    }
}

fn bench_large_scale_iteration(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_scale_simple_iteration");
    group.measurement_time(Duration::from_secs(15));

    for entity_count in [50_000, 100_000, 250_000, 500_000] {
        group.throughput(Throughput::Elements(entity_count as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}k", entity_count / 1000)),
            &entity_count,
            |b, &count| {
                let mut world = setup_simple_world(count);
                let dt = 1.0 / 60.0;

                b.iter(|| {
                    physics_update(&mut world, dt);
                    black_box(&world);
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Scenario 2: Complex Queries (5+ components)
// ============================================================================

fn setup_complex_world(entity_count: usize) -> World {
    let mut world = World::new();
    world.register::<Position>();
    world.register::<Velocity>();
    world.register::<Health>();
    world.register::<Armor>();
    world.register::<Team>();
    world.register::<AI>();
    world.register::<NetworkId>();

    for i in 0..entity_count {
        let entity = world.spawn();

        world.add(entity, Position { x: (i % 1000) as f32, y: 0.0, z: (i / 1000) as f32 });
        world.add(
            entity,
            Velocity { x: ((i as f32).sin() * 10.0), y: 0.0, z: ((i as f32).cos() * 10.0) },
        );
        world.add(entity, Health { current: 100.0, max: 100.0 });
        world.add(entity, Armor { value: 10.0 });
        world.add(entity, Team { id: i as u32 % 4 });
        world.add(entity, AI { state: 0, target: 0 });
        world.add(entity, NetworkId { id: i as u64 });
    }

    world
}

fn complex_query_system(world: &mut World) -> usize {
    let entities: Vec<Entity> = world.entities().collect();
    let mut count = 0;

    for entity in entities {
        // Query entities with all 7 components (typical MMO character)
        if world.get::<Position>(entity).is_some()
            && world.get::<Velocity>(entity).is_some()
            && world.get::<Health>(entity).is_some()
            && world.get::<Armor>(entity).is_some()
            && world.get::<Team>(entity).is_some()
            && world.get::<AI>(entity).is_some()
            && world.get::<NetworkId>(entity).is_some()
        {
            count += 1;
        }
    }

    count
}

fn bench_large_scale_complex_queries(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_scale_complex_queries");
    group.measurement_time(Duration::from_secs(15));

    for entity_count in [50_000, 100_000, 250_000] {
        group.throughput(Throughput::Elements(entity_count as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}k_7comp", entity_count / 1000)),
            &entity_count,
            |b, &count| {
                let mut world = setup_complex_world(count);

                b.iter(|| {
                    let matched = complex_query_system(&mut world);
                    black_box(matched);
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Scenario 3: Sparse Queries (10% match rate)
// ============================================================================

fn setup_sparse_world(entity_count: usize) -> World {
    let mut world = World::new();
    world.register::<Position>();
    world.register::<Health>();
    world.register::<Damage>(); // Only 10% have this

    for i in 0..entity_count {
        let entity = world.spawn();

        world.add(entity, Position { x: (i % 1000) as f32, y: 0.0, z: (i / 1000) as f32 });
        world.add(entity, Health { current: 100.0, max: 100.0 });

        // Only 10% of entities have Damage component
        if i % 10 == 0 {
            world.add(entity, Damage { value: 25.0 });
        }
    }

    world
}

fn sparse_query_system(world: &mut World) -> usize {
    let entities: Vec<Entity> = world.entities().collect();
    let mut count = 0;

    for entity in entities {
        // Query for Position + Health + Damage (only 10% match)
        if world.get::<Position>(entity).is_some()
            && world.get::<Health>(entity).is_some()
            && world.get::<Damage>(entity).is_some()
        {
            count += 1;
        }
    }

    count
}

fn bench_large_scale_sparse_queries(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_scale_sparse_queries");
    group.measurement_time(Duration::from_secs(15));

    for entity_count in [50_000, 100_000, 250_000, 500_000] {
        group.throughput(Throughput::Elements(entity_count as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}k_10pct", entity_count / 1000)),
            &entity_count,
            |b, &count| {
                let mut world = setup_sparse_world(count);

                b.iter(|| {
                    let matched = sparse_query_system(&mut world);
                    black_box(matched);
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Scenario 4: Entity Spawn/Despawn at Scale
// ============================================================================

fn bench_large_scale_spawn_despawn(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_scale_spawn_despawn");
    group.measurement_time(Duration::from_secs(15));

    for batch_size in [1000, 5000, 10_000, 50_000] {
        group.throughput(Throughput::Elements(batch_size as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("spawn_{}k", batch_size / 1000)),
            &batch_size,
            |b, &count| {
                let mut world = World::new();
                world.register::<Position>();
                world.register::<Health>();

                b.iter(|| {
                    let mut entities = Vec::with_capacity(count);

                    // Spawn batch
                    for i in 0..count {
                        let entity = world.spawn();
                        world.add(entity, Position { x: i as f32, y: 0.0, z: 0.0 });
                        world.add(entity, Health { current: 100.0, max: 100.0 });
                        entities.push(entity);
                    }

                    black_box(&entities);

                    // Despawn batch
                    for entity in entities {
                        world.despawn(entity);
                    }
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Scenario 5: Component Addition/Removal at Scale
// ============================================================================

fn bench_large_scale_component_ops(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_scale_component_ops");
    group.measurement_time(Duration::from_secs(15));

    for entity_count in [10_000, 50_000, 100_000] {
        group.throughput(Throughput::Elements(entity_count as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("add_remove_{}k", entity_count / 1000)),
            &entity_count,
            |b, &count| {
                let mut world = World::new();
                world.register::<Position>();
                world.register::<Health>();
                world.register::<Armor>();

                // Pre-spawn entities with Position + Health
                let entities: Vec<Entity> = (0..count)
                    .map(|i| {
                        let entity = world.spawn();
                        world.add(entity, Position { x: i as f32, y: 0.0, z: 0.0 });
                        world.add(entity, Health { current: 100.0, max: 100.0 });
                        entity
                    })
                    .collect();

                b.iter(|| {
                    // Add Armor to all entities
                    for &entity in &entities {
                        world.add(entity, Armor { value: 10.0 });
                    }

                    black_box(&world);

                    // Remove Armor from all entities
                    for &entity in &entities {
                        world.remove::<Armor>(entity);
                    }
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Scenario 6: Full MMO Server Tick Simulation
// ============================================================================

fn bench_mmo_server_tick(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_scale_mmo_server_tick");
    group.measurement_time(Duration::from_secs(20));

    for entity_count in [50_000, 100_000, 250_000] {
        group.throughput(Throughput::Elements(entity_count as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}k_entities", entity_count / 1000)),
            &entity_count,
            |b, &count| {
                let mut world = setup_complex_world(count);
                let dt = 1.0 / 60.0;

                b.iter(|| {
                    // Complete MMO server tick:
                    // 1. Movement system
                    let entities: Vec<Entity> = world.entities().collect();
                    for entity in &entities {
                        let vel = match world.get::<Velocity>(*entity) {
                            Some(v) => *v,
                            None => continue,
                        };

                        if let Some(pos) = world.get_mut::<Position>(*entity) {
                            pos.x += vel.x * dt;
                            pos.y += vel.y * dt;
                            pos.z += vel.z * dt;
                        }
                    }

                    // 2. Combat system (damage all entities)
                    for entity in &entities {
                        if let Some(health) = world.get_mut::<Health>(*entity) {
                            health.current = (health.current - 1.0).max(0.0);
                        }
                    }

                    // 3. Network replication query
                    let mut sync_count = 0;
                    for entity in &entities {
                        if world.get::<NetworkId>(*entity).is_some()
                            && world.get::<Position>(*entity).is_some()
                        {
                            sync_count += 1;
                        }
                    }

                    black_box(sync_count);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_large_scale_iteration,
    bench_large_scale_complex_queries,
    bench_large_scale_sparse_queries,
    bench_large_scale_spawn_despawn,
    bench_large_scale_component_ops,
    bench_mmo_server_tick,
);

criterion_main!(benches);
