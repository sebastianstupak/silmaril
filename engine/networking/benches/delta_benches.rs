//! Delta compression benchmarks for state synchronization
//!
//! Measures performance of delta computation, serialization, and application
//! across various change patterns and entity counts.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_core::serialization::{WorldState, WorldStateDelta};
use engine_core::{Entity, Health, Quat, Transform, Vec3, Velocity, World};
use engine_networking::delta::{AdaptiveDeltaStrategy, NetworkDelta};

/// Create a world with N entities, each having Transform, Velocity, and Health
fn create_world(entity_count: usize) -> World {
    let mut world = World::new();

    for i in 0..entity_count {
        let entity = world.spawn();
        let position = Vec3::new(i as f32, i as f32 * 2.0, i as f32 * 3.0);
        world.add(
            entity,
            Transform::new(position, Quat::IDENTITY, Vec3::ONE),
        );
        world.add(entity, Velocity::new(0.1, 0.2, 0.3));
        world.add(entity, Health::new(100.0, 100.0));
    }

    world
}

/// Modify a percentage of entities in the world (position changes only)
fn modify_positions(world: &mut World, change_percent: f32) {
    let entities: Vec<Entity> = world.entities().collect();
    let change_count = (entities.len() as f32 * change_percent) as usize;

    for entity in entities.iter().take(change_count) {
        if let Some(mut transform) = world.get_mut::<Transform>(*entity) {
            transform.position.x += 1.0;
            transform.position.y += 2.0;
            transform.position.z += 3.0;
        }
    }
}

/// Modify entities with mixed changes (position, velocity, health)
fn modify_mixed(world: &mut World, change_percent: f32) {
    let entities: Vec<Entity> = world.entities().collect();
    let change_count = (entities.len() as f32 * change_percent) as usize;

    for (idx, entity) in entities.iter().take(change_count).enumerate() {
        // Vary the changes
        if idx % 3 == 0 {
            if let Some(mut transform) = world.get_mut::<Transform>(*entity) {
                transform.translation[0] += 1.0;
            }
        }
        if idx % 3 == 1 {
            if let Some(mut velocity) = world.get_mut::<Velocity>(*entity) {
                velocity.x += 0.1;
            }
        }
        if idx % 3 == 2 {
            if let Some(mut health) = world.get_mut::<Health>(*entity) {
                health.current = (health.current - 10.0).max(0.0);
            }
        }
    }
}

/// Add and remove entities (simulating spawning/despawning)
fn add_remove_entities(world: &mut World, change_percent: f32) {
    let entities: Vec<Entity> = world.entities().collect();
    let change_count = (entities.len() as f32 * change_percent) as usize;

    // Remove some entities
    for entity in entities.iter().take(change_count / 2) {
        world.despawn(*entity);
    }

    // Add new entities
    for i in 0..(change_count / 2) {
        let entity = world.spawn();
        let position = Vec3::new(i as f32, 0.0, 0.0);
        world.add(entity, Transform::new(position, Quat::IDENTITY, Vec3::ONE));
        world.add(entity, Velocity::new(0.0, 0.0, 0.0));
        world.add(entity, Health::new(100.0, 100.0));
    }
}

/// Benchmark delta diff computation
fn bench_delta_diff(c: &mut Criterion) {
    let mut group = c.benchmark_group("delta_diff");

    for entity_count in [100, 1_000, 10_000] {
        for change_percent in [0.01, 0.05, 0.10, 0.50] {
            group.throughput(Throughput::Elements(entity_count as u64));

            let id = BenchmarkId::from_parameter(format!("{}ent_{}%", entity_count, (change_percent * 100.0) as u32));

            group.bench_with_input(id, &(entity_count, change_percent), |b, &(count, percent)| {
                let mut world1 = create_world(count);
                let state1 = WorldState::snapshot(&world1);

                modify_positions(&mut world1, percent);
                let state2 = WorldState::snapshot(&world1);

                b.iter(|| {
                    let delta = WorldStateDelta::compute(black_box(&state1), black_box(&state2));
                    black_box(delta);
                });
            });
        }
    }

    group.finish();
}

/// Benchmark delta serialization
fn bench_delta_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("delta_serialization");

    for entity_count in [100, 1_000, 10_000] {
        for change_percent in [0.01, 0.05, 0.10, 0.50] {
            let id = BenchmarkId::from_parameter(format!("{}ent_{}%", entity_count, (change_percent * 100.0) as u32));

            group.bench_with_input(id, &(entity_count, change_percent), |b, &(count, percent)| {
                let mut world1 = create_world(count);
                let state1 = WorldState::snapshot(&world1);

                modify_positions(&mut world1, percent);
                let state2 = WorldState::snapshot(&world1);

                let delta = WorldStateDelta::compute(&state1, &state2);

                b.iter(|| {
                    let bytes = bincode::serialize(black_box(&delta)).unwrap();
                    black_box(bytes);
                });
            });
        }
    }

    group.finish();
}

/// Benchmark delta application
fn bench_delta_application(c: &mut Criterion) {
    let mut group = c.benchmark_group("delta_application");

    for entity_count in [100, 1_000, 10_000] {
        for change_percent in [0.01, 0.05, 0.10, 0.50] {
            group.throughput(Throughput::Elements(entity_count as u64));

            let id = BenchmarkId::from_parameter(format!("{}ent_{}%", entity_count, (change_percent * 100.0) as u32));

            group.bench_with_input(id, &(entity_count, change_percent), |b, &(count, percent)| {
                let mut world1 = create_world(count);
                let state1 = WorldState::snapshot(&world1);

                modify_positions(&mut world1, percent);
                let state2 = WorldState::snapshot(&world1);

                let delta = WorldStateDelta::compute(&state1, &state2);

                b.iter(|| {
                    let mut base = state1.clone();
                    delta.apply(black_box(&mut base));
                    black_box(base);
                });
            });
        }
    }

    group.finish();
}

/// Benchmark compression ratio measurement
fn bench_compression_ratio(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression_ratio");

    for entity_count in [100, 1_000, 10_000] {
        for change_percent in [0.01, 0.05, 0.10, 0.50] {
            let id = BenchmarkId::from_parameter(format!("{}ent_{}%", entity_count, (change_percent * 100.0) as u32));

            group.bench_with_input(id, &(entity_count, change_percent), |b, &(count, percent)| {
                let mut world1 = create_world(count);
                let state1 = WorldState::snapshot(&world1);

                modify_positions(&mut world1, percent);
                let state2 = WorldState::snapshot(&world1);

                b.iter(|| {
                    let net_delta = NetworkDelta::from_states(black_box(&state1), black_box(&state2));
                    black_box(net_delta);
                });
            });
        }
    }

    group.finish();
}

/// Benchmark adaptive switching decision
fn bench_adaptive_decision(c: &mut Criterion) {
    let mut group = c.benchmark_group("adaptive_decision");

    for entity_count in [100, 1_000, 10_000] {
        let id = BenchmarkId::from_parameter(format!("{}ent", entity_count));

        group.bench_with_input(id, &entity_count, |b, &count| {
            let mut world1 = create_world(count);
            let state1 = WorldState::snapshot(&world1);

            modify_positions(&mut world1, 0.05);
            let state2 = WorldState::snapshot(&world1);

            let net_delta = NetworkDelta::from_states(&state1, &state2);
            let mut strategy = AdaptiveDeltaStrategy::default();

            // Record some history
            for i in 0..10 {
                strategy.record_delta(0.3 + (i as f32 * 0.05));
            }

            b.iter(|| {
                let should_use = strategy.should_use_delta(black_box(net_delta.compression_ratio));
                black_box(should_use);
            });
        });
    }

    group.finish();
}

/// Benchmark different change patterns
fn bench_change_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("change_patterns");

    let entity_count = 1_000;
    let change_percent = 0.05;

    // Position-only changes
    group.bench_function("position_only", |b| {
        let mut world1 = create_world(entity_count);
        let state1 = WorldState::snapshot(&world1);

        modify_positions(&mut world1, change_percent);
        let state2 = WorldState::snapshot(&world1);

        b.iter(|| {
            let delta = WorldStateDelta::compute(black_box(&state1), black_box(&state2));
            black_box(delta);
        });
    });

    // Mixed changes (position, velocity, health)
    group.bench_function("mixed_changes", |b| {
        let mut world = create_world(entity_count);
        let state1 = WorldState::snapshot(&world);

        modify_mixed(&mut world, change_percent);
        let state2 = WorldState::snapshot(&world);

        b.iter(|| {
            let delta = WorldStateDelta::compute(black_box(&state1), black_box(&state2));
            black_box(delta);
        });
    });

    // Add/remove entities
    group.bench_function("add_remove", |b| {
        let world = create_world(entity_count);
        let state1 = WorldState::snapshot(&world);

        let mut world2 = create_world(entity_count);
        add_remove_entities(&mut world2, change_percent);
        let state2 = WorldState::snapshot(&world2);

        b.iter(|| {
            let delta = WorldStateDelta::compute(black_box(&state1), black_box(&state2));
            black_box(delta);
        });
    });

    group.finish();
}

/// Benchmark full pipeline (diff -> serialize -> deserialize -> apply)
fn bench_full_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_pipeline");

    for entity_count in [100, 1_000, 10_000] {
        let id = BenchmarkId::from_parameter(format!("{}ent", entity_count));

        group.bench_with_input(id, &entity_count, |b, &count| {
            let mut world1 = create_world(count);
            let state1 = WorldState::snapshot(&world1);

            modify_positions(&mut world1, 0.05);
            let state2 = WorldState::snapshot(&world1);

            b.iter(|| {
                // Compute delta
                let delta = WorldStateDelta::compute(black_box(&state1), black_box(&state2));

                // Serialize
                let bytes = bincode::serialize(&delta).unwrap();

                // Deserialize
                let restored_delta: WorldStateDelta = bincode::deserialize(&bytes).unwrap();

                // Apply
                let mut base = state1.clone();
                restored_delta.apply(&mut base);

                black_box(base);
            });
        });
    }

    group.finish();
}

/// Benchmark size comparison (measure compression effectiveness)
fn bench_size_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("size_comparison");
    group.sample_size(20); // Fewer samples since this is more about measurement

    for entity_count in [100, 1_000, 10_000] {
        for change_percent in [0.01, 0.05, 0.10, 0.50] {
            let id = BenchmarkId::from_parameter(format!("{}ent_{}%", entity_count, (change_percent * 100.0) as u32));

            group.bench_with_input(id, &(entity_count, change_percent), |b, &(count, percent)| {
                let mut world1 = create_world(count);
                let state1 = WorldState::snapshot(&world1);

                modify_positions(&mut world1, percent);
                let state2 = WorldState::snapshot(&world1);

                let delta = WorldStateDelta::compute(&state1, &state2);

                b.iter(|| {
                    let delta_size = bincode::serialize(black_box(&delta)).unwrap().len();
                    let full_size = bincode::serialize(black_box(&state2)).unwrap().len();
                    let ratio = delta_size as f32 / full_size as f32;

                    black_box((delta_size, full_size, ratio));
                });
            });
        }
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_delta_diff,
    bench_delta_serialization,
    bench_delta_application,
    bench_compression_ratio,
    bench_adaptive_decision,
    bench_change_patterns,
    bench_full_pipeline,
    bench_size_comparison,
);
criterion_main!(benches);
