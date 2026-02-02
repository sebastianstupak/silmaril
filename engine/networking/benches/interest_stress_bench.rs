//! Interest Management Stress Benchmarks
//!
//! Extreme performance testing:
//! - Worst-case scenarios
//! - Pathological entity distributions
//! - Extreme scale (10K+ entities, 1000+ clients)
//! - Memory pressure tests
//! - Sustained load tests

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_core::{Aabb, Entity, Quat, Transform, Vec3, World};
use engine_interest::{AreaOfInterest, InterestManager};
use std::time::Duration;

// ============================================================================
// Helper Functions
// ============================================================================

fn create_world_grid(count: usize) -> (World, Vec<Entity>) {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Aabb>();

    let mut entities = Vec::new();
    let grid_size = (count as f32).sqrt() as usize;

    for i in 0..count {
        let entity = world.spawn();
        let x = ((i % grid_size) as f32) * 10.0;
        let z = ((i / grid_size) as f32) * 10.0;
        let pos = Vec3::new(x, 0.0, z);
        world.add(entity, Transform::new(pos, Quat::IDENTITY, Vec3::ONE));
        world.add(entity, Aabb::from_center_half_extents(pos, Vec3::ONE));
        entities.push(entity);
    }

    (world, entities)
}

fn create_clustered_world(count: usize, cluster_radius: f32) -> (World, Vec<Entity>) {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Aabb>();

    let mut entities = Vec::new();

    for i in 0..count {
        let entity = world.spawn();
        let angle = (i as f32) * 2.0 * std::f32::consts::PI / (count as f32);
        let radius = (i % 10) as f32 * cluster_radius / 10.0;
        let pos = Vec3::new(radius * angle.cos(), 0.0, radius * angle.sin());
        world.add(entity, Transform::new(pos, Quat::IDENTITY, Vec3::ONE));
        world.add(entity, Aabb::from_center_half_extents(pos, Vec3::ONE));
        entities.push(entity);
    }

    (world, entities)
}

// ============================================================================
// Extreme Scale Benchmarks
// ============================================================================

fn bench_extreme_entity_count(c: &mut Criterion) {
    let mut group = c.benchmark_group("interest_extreme_scale");
    group.measurement_time(Duration::from_secs(15));
    group.sample_size(20);

    for entity_count in [5_000, 10_000, 20_000] {
        let (world, _entities) = create_world_grid(entity_count);
        let mut manager = InterestManager::new(100.0);
        manager.update_from_world(&world);

        // Single client visibility
        group.throughput(Throughput::Elements(entity_count as u64));
        group.bench_with_input(
            BenchmarkId::new("single_client", entity_count),
            &entity_count,
            |b, _| {
                manager.set_client_interest(
                    1,
                    AreaOfInterest::new(
                        Vec3::new(
                            (entity_count as f32).sqrt() * 5.0,
                            0.0,
                            (entity_count as f32).sqrt() * 5.0,
                        ),
                        200.0,
                    ),
                );
                b.iter(|| {
                    let visible = black_box(manager.calculate_visibility(1));
                    black_box(visible);
                });
            },
        );
    }

    group.finish();
}

fn bench_massive_client_count(c: &mut Criterion) {
    let mut group = c.benchmark_group("interest_massive_clients");
    group.measurement_time(Duration::from_secs(20));
    group.sample_size(15);

    let (world, _entities) = create_world_grid(5_000);

    for client_count in [100, 500, 1000] {
        let mut manager = InterestManager::new(100.0);
        manager.update_from_world(&world);

        // Register all clients
        for i in 0..client_count {
            let x = ((i % 32) as f32) * 100.0;
            let z = ((i / 32) as f32) * 100.0;
            manager.set_client_interest(i as u64, AreaOfInterest::new(Vec3::new(x, 0.0, z), 150.0));
        }

        group.throughput(Throughput::Elements(client_count as u64));
        group.bench_with_input(
            BenchmarkId::new("all_clients", client_count),
            &client_count,
            |b, &count| {
                b.iter(|| {
                    for i in 0..count {
                        let visible = black_box(manager.calculate_visibility(i as u64));
                        black_box(visible);
                    }
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Worst-Case Scenario Benchmarks
// ============================================================================

fn bench_all_entities_one_cell(c: &mut Criterion) {
    let mut group = c.benchmark_group("interest_worst_case");
    group.measurement_time(Duration::from_secs(10));

    // Worst case: all entities in tiny cluster
    let (world, _entities) = create_clustered_world(1000, 10.0);
    let mut manager = InterestManager::new(100.0);
    manager.update_from_world(&world);

    manager.set_client_interest(1, AreaOfInterest::new(Vec3::ZERO, 50.0));

    group.bench_function("clustered_1k_entities", |b| {
        b.iter(|| {
            let visible = black_box(manager.calculate_visibility(1));
            black_box(visible);
        });
    });

    // Worst case: all clients in same location
    for i in 0..100 {
        manager.set_client_interest(i, AreaOfInterest::new(Vec3::ZERO, 50.0));
    }

    group.bench_function("clustered_100_clients", |b| {
        b.iter(|| {
            for i in 0..100 {
                let visible = black_box(manager.calculate_visibility(i));
                black_box(visible);
            }
        });
    });

    group.finish();
}

fn bench_linear_arrangement(c: &mut Criterion) {
    let mut group = c.benchmark_group("interest_pathological");
    group.measurement_time(Duration::from_secs(10));

    // Pathological case: perfect line of entities
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Aabb>();

    for i in 0..5000 {
        let entity = world.spawn();
        let pos = Vec3::new(i as f32 * 5.0, 0.0, 0.0);
        world.add(entity, Transform::new(pos, Quat::IDENTITY, Vec3::ONE));
        world.add(entity, Aabb::from_center_half_extents(pos, Vec3::ONE));
    }

    let mut manager = InterestManager::new(50.0);
    manager.update_from_world(&world);

    group.bench_function("linear_5k_entities", |b| {
        manager.set_client_interest(1, AreaOfInterest::new(Vec3::new(12500.0, 0.0, 0.0), 200.0));
        b.iter(|| {
            let visible = black_box(manager.calculate_visibility(1));
            black_box(visible);
        });
    });

    group.finish();
}

// ============================================================================
// Sustained Load Benchmarks
// ============================================================================

fn bench_continuous_updates(c: &mut Criterion) {
    let mut group = c.benchmark_group("interest_sustained_load");
    group.measurement_time(Duration::from_secs(15));
    group.sample_size(10);

    let (mut world, entities) = create_world_grid(1000);
    let mut manager = InterestManager::new(50.0);

    group.bench_function("continuous_entity_movement", |b| {
        let mut frame = 0;
        b.iter(|| {
            // Simulate entity movement every frame
            for (i, &entity) in entities.iter().enumerate() {
                if i % 10 == frame % 10 {
                    let transform = world.get_mut::<Transform>(entity).unwrap();
                    transform.position.x += 1.0;
                }
            }

            manager.update_from_world(&world);

            // Client tracking
            manager.set_client_interest(
                1,
                AreaOfInterest::new(Vec3::new(frame as f32 * 2.0, 0.0, 0.0), 100.0),
            );

            let visible = black_box(manager.calculate_visibility(1));
            black_box(visible);

            frame += 1;
        });
    });

    group.finish();
}

fn bench_entity_churn(c: &mut Criterion) {
    let mut group = c.benchmark_group("interest_entity_churn");
    group.measurement_time(Duration::from_secs(15));
    group.sample_size(10);

    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Aabb>();

    let mut manager = InterestManager::new(50.0);
    manager.set_client_interest(1, AreaOfInterest::new(Vec3::new(500.0, 0.0, 500.0), 100.0));

    group.bench_function("spawn_despawn_cycle", |b| {
        let mut entities_pool = Vec::new();
        b.iter(|| {
            // Spawn 50 entities
            for i in 0..50 {
                let entity = world.spawn();
                let pos =
                    Vec3::new(500.0 + (i % 10) as f32 * 10.0, 0.0, 500.0 + (i / 10) as f32 * 10.0);
                world.add(entity, Transform::new(pos, Quat::IDENTITY, Vec3::ONE));
                world.add(entity, Aabb::from_center_half_extents(pos, Vec3::ONE));
                entities_pool.push(entity);
            }

            manager.update_from_world(&world);
            let visible = black_box(manager.calculate_visibility(1));
            black_box(visible);

            // Despawn 25 entities
            for _ in 0..25 {
                if let Some(entity) = entities_pool.pop() {
                    world.despawn(entity);
                }
            }

            manager.update_from_world(&world);
        });
    });

    group.finish();
}

// ============================================================================
// Memory Pressure Benchmarks
// ============================================================================

fn bench_high_aoi_count(c: &mut Criterion) {
    let mut group = c.benchmark_group("interest_memory_pressure");
    group.measurement_time(Duration::from_secs(15));
    group.sample_size(10);

    let (world, _entities) = create_world_grid(2000);
    let mut manager = InterestManager::new(50.0);
    manager.update_from_world(&world);

    group.bench_function("1000_active_aois", |b| {
        // Register 1000 clients
        for i in 0..1000 {
            let x = ((i % 32) as f32) * 50.0;
            let z = ((i / 32) as f32) * 50.0;
            manager.set_client_interest(i as u64, AreaOfInterest::new(Vec3::new(x, 0.0, z), 100.0));
        }

        b.iter(|| {
            // Calculate visibility for subset of clients
            for i in (0..1000).step_by(10) {
                let visible = black_box(manager.calculate_visibility(i as u64));
                black_box(visible);
            }
        });
    });

    group.finish();
}

fn bench_large_visibility_sets(c: &mut Criterion) {
    let mut group = c.benchmark_group("interest_large_visibility");
    group.measurement_time(Duration::from_secs(10));

    // Dense cluster - many entities visible
    let (world, _entities) = create_clustered_world(5000, 100.0);
    let mut manager = InterestManager::new(200.0);
    manager.update_from_world(&world);

    group.bench_function("visibility_3k_entities", |b| {
        manager.set_client_interest(1, AreaOfInterest::new(Vec3::ZERO, 150.0));
        b.iter(|| {
            let visible = black_box(manager.calculate_visibility(1));
            black_box(visible);
        });
    });

    group.finish();
}

// ============================================================================
// Benchmark Registration
// ============================================================================

criterion_group!(
    stress_benches,
    bench_extreme_entity_count,
    bench_massive_client_count,
    bench_all_entities_one_cell,
    bench_linear_arrangement,
    bench_continuous_updates,
    bench_entity_churn,
    bench_high_aoi_count,
    bench_large_visibility_sets,
);

criterion_main!(stress_benches);
