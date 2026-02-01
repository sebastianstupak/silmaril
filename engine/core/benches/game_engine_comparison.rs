//! Game Engine Comparison Benchmarks
//!
//! Practical benchmarks measuring real-world game scenarios that can be
//! compared against Unity, Unreal, Godot, and Bevy.
//!
//! ## Benchmark Scenarios
//!
//! 1. **Simple Game Loop (60 FPS target)**
//!    - 1000 entities with Position + Velocity
//!    - Physics update system
//!    - Rendering query (gather positions)
//!    - Target: <16.67ms per frame
//!
//! 2. **MMO Simulation**
//!    - 10,000 entities (players + NPCs)
//!    - Components: Position, Health, Inventory, NetworkId
//!    - Systems: Movement, Combat, Replication
//!    - Target: <16ms server tick (60 TPS)
//!
//! 3. **Asset Loading**
//!    - Load 1000 "assets" (simulated with file I/O)
//!    - Path normalization for each
//!    - Measure total loading time
//!
//! 4. **State Serialization**
//!    - Serialize world with 10,000 entities
//!    - Deserialize world state
//!    - Measure both operations
//!
//! 5. **Spatial Queries**
//!    - 10,000 entities in 3D space
//!    - Query entities within radius
//!    - Measure query performance
//!
//! ## Comparison Data
//!
//! Results can be compared against industry benchmarks documented in
//! `benchmarks/industry_comparison.yaml`

#![allow(dead_code)]

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_core::{
    ecs::{Component, Entity, World},
    math::{Quat, Transform, Vec3},
    serialization::{Format, Serializable, WorldState},
    spatial::{Aabb, SpatialGrid, SpatialGridConfig},
    Velocity,
};
use std::time::Duration;

// ============================================================
// Components for Realistic Game Scenarios
// ============================================================

#[derive(Debug, Clone, Copy)]
struct Position {
    x: f32,
    y: f32,
    z: f32,
}
impl Component for Position {}

impl Position {
    fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    fn as_vec3(&self) -> Vec3 {
        Vec3::new(self.x, self.y, self.z)
    }
}

#[derive(Debug, Clone, Copy)]
struct Health {
    current: f32,
    max: f32,
}
impl Component for Health {}

#[derive(Debug, Clone, Copy)]
struct Inventory {
    items: [u32; 8], // Simplified inventory
}
impl Component for Inventory {}

#[derive(Debug, Clone, Copy)]
struct NetworkId {
    id: u64,
}
impl Component for NetworkId {}

#[derive(Debug, Clone, Copy)]
struct MoveSpeed {
    speed: f32,
}
impl Component for MoveSpeed {}

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
struct Target {
    entity: Option<Entity>,
}
impl Component for Target {}

#[derive(Debug, Clone, Copy)]
struct Team {
    id: u32,
}
impl Component for Team {}

// ============================================================
// Scenario 1: Simple Game Loop (60 FPS Target)
// ============================================================

fn setup_simple_game_world(entity_count: usize) -> World {
    let mut world = World::new();
    world.register::<Position>();
    world.register::<Velocity>();

    for i in 0..entity_count {
        let entity = world.spawn();
        world.add(entity, Position::new((i % 32) as f32 * 2.0, 0.0, (i / 32) as f32 * 2.0));
        world.add(
            entity,
            Velocity { x: ((i as f32).sin() * 10.0), y: 0.0, z: ((i as f32).cos() * 10.0) },
        );
    }

    world
}

fn physics_update_system(world: &mut World, dt: f32) {
    // Collect all entities first to avoid borrow checker issues
    let entities: Vec<Entity> = world.entities().collect();

    // Iterate through entities with both components
    for entity in entities {
        // Get velocity first (immutable)
        let vel = match world.get::<Velocity>(entity) {
            Some(v) => *v,
            None => continue,
        };

        // Then mutate position
        if let Some(pos) = world.get_mut::<Position>(entity) {
            pos.x += vel.x * dt;
            pos.y += vel.y * dt;
            pos.z += vel.z * dt;
        }
    }
}

fn rendering_query_system(world: &World) -> usize {
    // Simulate rendering by gathering all positions
    let entities: Vec<Entity> = world.entities().collect();
    let mut count = 0;

    for entity in entities {
        if world.get::<Position>(entity).is_some() {
            count += 1;
        }
    }

    count
}

fn bench_simple_game_loop(c: &mut Criterion) {
    let mut group = c.benchmark_group("scenario_1_simple_game_loop");
    group.measurement_time(Duration::from_secs(10));

    for entity_count in [100, 1000, 10000] {
        group.throughput(Throughput::Elements(entity_count as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(entity_count),
            &entity_count,
            |b, &count| {
                let mut world = setup_simple_game_world(count);
                let dt = 1.0 / 60.0; // 60 FPS target

                b.iter(|| {
                    // Complete frame simulation
                    physics_update_system(&mut world, dt);
                    let rendered = rendering_query_system(&world);
                    black_box(rendered);
                });
            },
        );
    }

    group.finish();
}

// ============================================================
// Scenario 2: MMO Simulation
// ============================================================

fn setup_mmo_world(player_count: usize, npc_count: usize) -> World {
    let mut world = World::new();
    world.register::<Position>();
    world.register::<Velocity>();
    world.register::<Health>();
    world.register::<Inventory>();
    world.register::<NetworkId>();
    world.register::<MoveSpeed>();
    world.register::<Team>();

    // Spawn players
    for i in 0..player_count {
        let entity = world.spawn();
        world.add(entity, Position::new((i % 100) as f32 * 5.0, 0.0, (i / 100) as f32 * 5.0));
        world.add(entity, Velocity { x: 0.0, y: 0.0, z: 0.0 });
        world.add(entity, Health { current: 100.0, max: 100.0 });
        world.add(entity, Inventory { items: [0; 8] });
        world.add(entity, NetworkId { id: i as u64 });
        world.add(entity, MoveSpeed { speed: 5.0 });
        world.add(entity, Team { id: i as u32 % 2 });
    }

    // Spawn NPCs
    for i in 0..npc_count {
        let entity = world.spawn();
        world.add(
            entity,
            Position::new((i % 100) as f32 * 5.0 + 2.5, 0.0, (i / 100) as f32 * 5.0 + 2.5),
        );
        world.add(entity, Health { current: 50.0, max: 50.0 });
        world.add(entity, Armor { value: 10.0 });
    }

    world
}

fn movement_system(world: &mut World, dt: f32) {
    let entities: Vec<Entity> = world.entities().collect();

    for entity in entities {
        // Get velocity first (immutable)
        let vel = match world.get::<Velocity>(entity) {
            Some(v) => *v,
            None => continue,
        };

        // Then mutate position
        if let Some(pos) = world.get_mut::<Position>(entity) {
            pos.x += vel.x * dt;
            pos.y += vel.y * dt;
            pos.z += vel.z * dt;
        }
    }
}

fn combat_system(world: &mut World) {
    // Simplified combat: damage all entities with Health
    let entities: Vec<Entity> = world.entities().collect();

    for entity in entities {
        if let Some(health) = world.get_mut::<Health>(entity) {
            health.current = (health.current - 1.0).max(0.0);
        }
    }
}

fn replication_system(world: &World) -> usize {
    // Simulate network replication by counting entities that need sync
    let entities: Vec<Entity> = world.entities().collect();
    let mut sync_count = 0;

    for entity in entities {
        if world.get::<NetworkId>(entity).is_some() && world.get::<Position>(entity).is_some() {
            sync_count += 1;
        }
    }

    sync_count
}

fn bench_mmo_simulation(c: &mut Criterion) {
    let mut group = c.benchmark_group("scenario_2_mmo_simulation");
    group.measurement_time(Duration::from_secs(15));

    // Test with different scales
    for (players, npcs) in [(100, 900), (1000, 9000), (5000, 5000)] {
        let total = players + npcs;
        group.throughput(Throughput::Elements(total as u64));

        group.bench_with_input(
            BenchmarkId::new("server_tick", format!("{}p_{}npc", players, npcs)),
            &(players, npcs),
            |b, &(p, n)| {
                let mut world = setup_mmo_world(p, n);
                let dt = 1.0 / 60.0; // 60 TPS target

                b.iter(|| {
                    // Complete server tick simulation
                    movement_system(&mut world, dt);
                    combat_system(&mut world);
                    let synced = replication_system(&world);
                    black_box(synced);
                });
            },
        );
    }

    group.finish();
}

// ============================================================
// Scenario 3: Asset Loading Simulation
// ============================================================

#[derive(Debug, Clone)]
struct AssetPath {
    path: String,
    normalized: String,
}

fn normalize_path(path: &str) -> String {
    // Simulate path normalization work
    path.replace('\\', "/").to_lowercase().trim_start_matches("./").to_string()
}

fn simulate_asset_load(path: &str) -> AssetPath {
    // Simulate I/O and parsing overhead
    let normalized = normalize_path(path);

    // Simulate some computation
    let _hash = normalized
        .bytes()
        .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));

    AssetPath { path: path.to_string(), normalized }
}

fn bench_asset_loading(c: &mut Criterion) {
    let mut group = c.benchmark_group("scenario_3_asset_loading");
    group.measurement_time(Duration::from_secs(10));

    for asset_count in [100, 1000, 10000] {
        group.throughput(Throughput::Elements(asset_count as u64));

        // Generate asset paths
        let paths: Vec<String> =
            (0..asset_count).map(|i| format!("assets/models/model_{:04}.mesh", i)).collect();

        group.bench_with_input(BenchmarkId::from_parameter(asset_count), &paths, |b, paths| {
            b.iter(|| {
                let assets: Vec<AssetPath> = paths.iter().map(|p| simulate_asset_load(p)).collect();
                black_box(assets);
            });
        });
    }

    group.finish();
}

// ============================================================
// Scenario 4: State Serialization
// ============================================================

fn setup_serialization_world(entity_count: usize) -> World {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Health>();
    world.register::<Velocity>();

    for i in 0..entity_count {
        let entity = world.spawn();

        let pos = Vec3::new((i % 100) as f32 * 2.0, 0.0, (i / 100) as f32 * 2.0);

        world.add(entity, Transform::new(pos, Quat::IDENTITY, Vec3::ONE));
        world.add(entity, Health { current: 100.0, max: 100.0 });

        if i % 2 == 0 {
            world.add(
                entity,
                Velocity { x: ((i as f32).sin() * 5.0), y: 0.0, z: ((i as f32).cos() * 5.0) },
            );
        }
    }

    world
}

fn bench_state_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("scenario_4_state_serialization");
    group.measurement_time(Duration::from_secs(10));

    for entity_count in [100, 1000, 10000] {
        group.throughput(Throughput::Elements(entity_count as u64));

        let world = setup_serialization_world(entity_count);
        let snapshot = WorldState::snapshot(&world);

        // Benchmark serialization
        group.bench_with_input(
            BenchmarkId::new("serialize_bincode", entity_count),
            &snapshot,
            |b, snapshot| {
                b.iter(|| {
                    let bytes = snapshot.serialize(Format::Bincode).unwrap();
                    black_box(bytes);
                });
            },
        );

        // Benchmark deserialization
        let serialized = snapshot.serialize(Format::Bincode).unwrap();
        group.bench_with_input(
            BenchmarkId::new("deserialize_bincode", entity_count),
            &serialized,
            |b, bytes| {
                b.iter(|| {
                    let state = WorldState::deserialize(bytes, Format::Bincode).unwrap();
                    black_box(state);
                });
            },
        );

        // Benchmark full roundtrip
        group.bench_with_input(
            BenchmarkId::new("roundtrip_bincode", entity_count),
            &world,
            |b, world| {
                b.iter(|| {
                    let snapshot = WorldState::snapshot(world);
                    let bytes = snapshot.serialize(Format::Bincode).unwrap();
                    let restored = WorldState::deserialize(&bytes, Format::Bincode).unwrap();
                    black_box(restored);
                });
            },
        );
    }

    group.finish();
}

// ============================================================
// Scenario 5: Spatial Queries
// ============================================================

fn setup_spatial_world(entity_count: usize) -> World {
    let mut world = World::new();
    world.register::<Aabb>();
    world.register::<Position>();

    // Distribute entities in 3D space
    for i in 0..entity_count {
        let entity = world.spawn();

        let x = (i % 100) as f32 * 2.0;
        let y = ((i / 100) % 100) as f32 * 2.0;
        let z = (i / 10000) as f32 * 2.0;

        let pos = Vec3::new(x, y, z);
        let aabb = Aabb::from_center_half_extents(pos, Vec3::new(0.5, 0.5, 0.5));

        world.add(entity, aabb);
        world.add(entity, Position::new(x, y, z));
    }

    world
}

fn bench_spatial_queries(c: &mut Criterion) {
    let mut group = c.benchmark_group("scenario_5_spatial_queries");
    group.measurement_time(Duration::from_secs(10));

    for entity_count in [100, 1000, 10000] {
        group.throughput(Throughput::Elements(entity_count as u64));

        let world = setup_spatial_world(entity_count);

        // Build spatial grid
        let config = SpatialGridConfig { cell_size: 10.0, entities_per_cell: 16 };
        let grid = SpatialGrid::build(&world, config);

        // Benchmark radius query
        group.bench_with_input(BenchmarkId::new("radius_query", entity_count), &grid, |b, grid| {
            b.iter(|| {
                let center = Vec3::new(50.0, 50.0, 0.0);
                let radius = 10.0;
                let results = grid.query_radius(center, radius);
                black_box(results);
            });
        });

        // Benchmark AABB query
        group.bench_with_input(BenchmarkId::new("aabb_query", entity_count), &grid, |b, grid| {
            b.iter(|| {
                let query_aabb = Aabb::from_center_half_extents(
                    Vec3::new(50.0, 50.0, 0.0),
                    Vec3::new(10.0, 10.0, 10.0),
                );
                let results = grid.query_aabb(&query_aabb);
                black_box(results);
            });
        });

        // Benchmark grid rebuild
        group.bench_with_input(
            BenchmarkId::new("grid_rebuild", entity_count),
            &world,
            |b, world| {
                b.iter(|| {
                    let grid = SpatialGrid::build(world, config);
                    black_box(grid);
                });
            },
        );
    }

    group.finish();
}

// ============================================================
// Comprehensive Comparison Suite
// ============================================================

fn bench_comprehensive_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("comprehensive_comparison");
    group.measurement_time(Duration::from_secs(20));

    // Full frame simulation at 1000 entities
    {
        let mut world = setup_simple_game_world(1000);
        let dt = 1.0 / 60.0;

        group.bench_function("full_frame_1000_entities", |b| {
            b.iter(|| {
                physics_update_system(&mut world, dt);
                let rendered = rendering_query_system(&world);
                black_box(rendered);
            });
        });
    }

    // Server tick simulation at 10K entities
    {
        let mut world = setup_mmo_world(1000, 9000);
        let dt = 1.0 / 60.0;

        group.bench_function("server_tick_10k_entities", |b| {
            b.iter(|| {
                movement_system(&mut world, dt);
                combat_system(&mut world);
                let synced = replication_system(&world);
                black_box(synced);
            });
        });
    }

    // Serialization throughput at 10K entities
    {
        let world = setup_serialization_world(10000);
        let snapshot = WorldState::snapshot(&world);

        group.bench_function("serialize_10k_entities", |b| {
            b.iter(|| {
                let bytes = snapshot.serialize(Format::Bincode).unwrap();
                black_box(bytes);
            });
        });
    }

    group.finish();
}

// ============================================================
// Criterion Configuration
// ============================================================

criterion_group!(
    benches,
    bench_simple_game_loop,
    bench_mmo_simulation,
    bench_asset_loading,
    bench_state_serialization,
    bench_spatial_queries,
    bench_comprehensive_comparison,
);

criterion_main!(benches);
