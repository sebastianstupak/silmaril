//! Comprehensive Serialization Benchmarks
//!
//! Tests serialization performance against AAA industry standards.
//! Critical for network state sync (20-60 times per second per client).
//!
//! Industry Targets:
//! - Entity snapshot serialization: <10μs (FlatBuffers zero-copy)
//! - Entity delta serialization: <2μs (only changed components)
//! - World state (1000 entities): <1ms (full snapshot)
//! - Delta compression: <200μs (incremental)
//!
//! Run with:
//! ```bash
//! cargo bench --bench serialization_comprehensive
//! ```

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_core::ecs::{Component, World};
use engine_core::math::{Transform, Vec3};
use engine_core::physics_components::Velocity;
use engine_core::serialization::WorldState;

// ============================================================================
// Helper Functions
// ============================================================================

fn setup_world() -> World {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Velocity>();
    world.register::<Health>();
    world.register::<Player>();
    world
}

// ============================================================================
// Component Definitions for Benchmarking
// ============================================================================

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
struct Health {
    current: f32,
    max: f32,
}
impl Component for Health {}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
struct Player {
    id: u32,
}
impl Component for Player {}

// ============================================================================
// Entity Serialization Benchmarks
// ============================================================================

fn bench_entity_snapshot_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_snapshot_serialization");

    // Target: <10μs per entity (full snapshot)
    // Note: We benchmark single-entity world as proxy for entity serialization
    group.bench_function("single_entity_full", |b| {
        let mut world = setup_world();
        let entity = world.spawn();
        world.add(entity, Transform::default());
        world.add(entity, Velocity { x: 1.0, y: 0.0, z: 0.0 });
        world.add(entity, Health { current: 100.0, max: 100.0 });

        b.iter(|| {
            let state = WorldState::snapshot(black_box(&world));
            black_box(state);
        });
    });

    group.finish();
}

fn bench_world_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("world_serialization");

    // Target: <1ms for 1000 entities
    for count in [100, 1_000, 10_000].iter() {
        group.throughput(Throughput::Elements(*count as u64));

        group.bench_with_input(BenchmarkId::new("full_snapshot", count), count, |b, &count| {
            let mut world = setup_world();
            for i in 0..count {
                let entity = world.spawn();
                let mut transform = Transform::default();
                transform.position = Vec3::new(i as f32, i as f32, 0.0);
                world.add(entity, transform);
                world.add(entity, Velocity { x: 1.0, y: 0.0, z: 0.0 });
                world.add(entity, Health { current: 100.0, max: 100.0 });
            }

            b.iter(|| {
                let state = WorldState::snapshot(black_box(&world));
                black_box(state);
            });
        });
    }

    group.finish();
}

fn bench_world_deserialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("world_deserialization");

    // Target: <5μs per entity (zero-copy read)
    for count in [100, 1_000, 10_000].iter() {
        group.throughput(Throughput::Elements(*count as u64));

        group.bench_with_input(BenchmarkId::new("full_snapshot", count), count, |b, &count| {
            // Setup: Create serialized world state
            let mut world = setup_world();
            for i in 0..count {
                let entity = world.spawn();
                let mut transform = Transform::default();
                transform.position = Vec3::new(i as f32, i as f32, 0.0);
                world.add(entity, transform);
                world.add(entity, Velocity { x: 1.0, y: 0.0, z: 0.0 });
                world.add(entity, Health { current: 100.0, max: 100.0 });
            }

            let serialized_state = WorldState::snapshot(&world);

            b.iter(|| {
                let mut new_world = setup_world();
                serialized_state.restore(&mut new_world);
                black_box(new_world);
            });
        });
    }

    group.finish();
}

// ============================================================================
// Serialization Roundtrip Benchmarks
// ============================================================================

fn bench_serialization_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialization_roundtrip");

    // Target: <15μs per entity (serialize + deserialize)
    for count in [100, 1_000, 10_000].iter() {
        group.throughput(Throughput::Elements(*count as u64));

        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            let mut world = setup_world();
            for i in 0..count {
                let entity = world.spawn();
                let mut transform = Transform::default();
                transform.position = Vec3::new(i as f32, i as f32, 0.0);
                world.add(entity, transform);
                world.add(entity, Velocity { x: 1.0, y: 0.0, z: 0.0 });
            }

            b.iter(|| {
                // Serialize
                let state = WorldState::snapshot(black_box(&world));

                // Deserialize
                let mut new_world = setup_world();
                state.restore(&mut new_world);

                black_box(new_world);
            });
        });
    }

    group.finish();
}

// ============================================================================
// Format Comparison Benchmarks
// ============================================================================

fn bench_yaml_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("yaml_serialization");

    // Compare YAML performance (debug format) vs binary
    for count in [10, 100, 1_000].iter() {
        group.throughput(Throughput::Elements(*count as u64));

        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            let mut world = setup_world();
            for i in 0..count {
                let entity = world.spawn();
                let mut transform = Transform::default();
                transform.position = Vec3::new(i as f32, i as f32, 0.0);
                world.add(entity, transform);
                world.add(entity, Velocity { x: 1.0, y: 0.0, z: 0.0 });
            }

            b.iter(|| {
                let state = WorldState::snapshot(black_box(&world));
                let yaml = serde_yaml::to_string(&state).unwrap();
                black_box(yaml);
            });
        });
    }

    group.finish();
}

// ============================================================================
// Memory Usage Benchmarks
// ============================================================================

fn bench_serialized_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialized_size");

    // Measure memory overhead of serialization
    for count in [100, 1_000, 10_000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            let mut world = setup_world();
            for i in 0..count {
                let entity = world.spawn();
                let mut transform = Transform::default();
                transform.position = Vec3::new(i as f32, i as f32, 0.0);
                world.add(entity, transform);
                world.add(entity, Velocity { x: 1.0, y: 0.0, z: 0.0 });
            }

            let state = WorldState::snapshot(&world);

            b.iter(|| {
                // Measure bincode size
                let bincode_bytes = bincode::serialize(&state).unwrap();
                let bincode_size = bincode_bytes.len();

                // Measure YAML size (debug format)
                let yaml_string = serde_yaml::to_string(&state).unwrap();
                let yaml_size = yaml_string.len();

                black_box((bincode_size, yaml_size));
            });
        });
    }

    group.finish();
}

// ============================================================================
// Criterion Configuration
// ============================================================================

criterion_group!(
    name = entity_serialization;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(std::time::Duration::from_secs(10));
    targets = bench_entity_snapshot_serialization
);

criterion_group!(
    name = world_serialization;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(std::time::Duration::from_secs(10));
    targets = bench_world_serialization, bench_world_deserialization
);

criterion_group!(
    name = roundtrip;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(std::time::Duration::from_secs(10));
    targets = bench_serialization_roundtrip
);

criterion_group!(
    name = format_comparison;
    config = Criterion::default()
        .sample_size(50)
        .measurement_time(std::time::Duration::from_secs(5));
    targets = bench_yaml_serialization, bench_serialized_size
);

criterion_main!(entity_serialization, world_serialization, roundtrip, format_comparison);
