//! Interest Management Benchmarks
//!
//! Validates performance targets from phase2-interest-basic.md:
//! - Visibility computation: <1ms for 1K entities per client
//! - 100 clients: <100ms total
//! - Spatial queries: radius 100 <100µs, radius 1000 <500µs
//! - Nearest N entities: <200µs
//! - Bandwidth reduction: 80-95%

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_core::{Aabb, Entity, Quat, Transform, Vec3, World};
use engine_interest::{AreaOfInterest, InterestManager};
use engine_networking::InterestFilter;
use std::time::Duration;

/// Helper to create a world with entities in a grid pattern
fn create_world_with_entities(count: usize) -> (World, Vec<Entity>) {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Aabb>();

    let mut entities = Vec::with_capacity(count);

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

/// Benchmark: Single client visibility computation (1K entities)
/// Target: <1ms
fn bench_visibility_single_client_1k(c: &mut Criterion) {
    let mut group = c.benchmark_group("interest_visibility_single_client");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(100);

    let (world, _entities) = create_world_with_entities(1000);

    let mut manager = InterestManager::new(50.0);
    manager.update_from_world(&world);

    let client_id = 1;
    let aoi = AreaOfInterest::new(Vec3::new(500.0, 0.0, 500.0), 100.0);
    manager.set_client_interest(client_id, aoi);

    group.bench_function("1k_entities", |b| {
        b.iter(|| {
            let visible = black_box(manager.calculate_visibility(client_id));
            black_box(visible);
        });
    });

    group.finish();
}

/// Benchmark: Multiple clients visibility (100 clients)
/// Target: <100ms total
fn bench_visibility_100_clients(c: &mut Criterion) {
    let mut group = c.benchmark_group("interest_visibility_multi_client");
    group.measurement_time(Duration::from_secs(15));
    group.sample_size(50);

    let (world, _entities) = create_world_with_entities(1000);

    let mut manager = InterestManager::new(50.0);
    manager.update_from_world(&world);

    // Register 100 clients spread across the world
    for i in 0..100 {
        let pos = Vec3::new(((i % 10) as f32) * 100.0, 0.0, ((i / 10) as f32) * 100.0);
        let aoi = AreaOfInterest::new(pos, 100.0);
        manager.set_client_interest(i, aoi);
    }

    group.bench_function("100_clients_1k_entities", |b| {
        b.iter(|| {
            for i in 0..100 {
                let visible = black_box(manager.calculate_visibility(i));
                black_box(visible);
            }
        });
    });

    group.finish();
}

/// Benchmark: Spatial query performance with different radii
/// Targets: radius 100 <100µs, radius 1000 <500µs
fn bench_spatial_queries(c: &mut Criterion) {
    let mut group = c.benchmark_group("interest_spatial_queries");
    group.measurement_time(Duration::from_secs(10));

    let (world, _entities) = create_world_with_entities(10_000);

    let mut manager = InterestManager::new(50.0);
    manager.update_from_world(&world);

    let center = Vec3::new(500.0, 0.0, 500.0);

    for radius in [100.0, 500.0, 1000.0] {
        group.bench_with_input(BenchmarkId::new("radius", radius as u32), &radius, |b, &r| {
            b.iter(|| {
                let aoi = AreaOfInterest::new(center, r);
                manager.set_client_interest(999, aoi);
                let visible = black_box(manager.calculate_visibility(999));
                black_box(visible);
            });
        });
    }

    group.finish();
}

/// Benchmark: Nearest N entities query
/// Target: <200µs
fn bench_nearest_entities(c: &mut Criterion) {
    let mut group = c.benchmark_group("interest_nearest_entities");
    group.measurement_time(Duration::from_secs(10));

    let (world, _entities) = create_world_with_entities(5_000);

    let mut manager = InterestManager::new(50.0);
    manager.update_from_world(&world);

    let center = Vec3::new(500.0, 0.0, 500.0);

    // Simulate "nearest N" by using small radius AOIs
    for n in [10, 50, 100] {
        // Radius to capture approximately N entities (rough approximation)
        let radius = ((n as f32) * 10.0).sqrt() * 10.0;

        group.bench_with_input(BenchmarkId::new("nearest", n), &radius, |b, &r| {
            b.iter(|| {
                let aoi = AreaOfInterest::new(center, r);
                manager.set_client_interest(999, aoi);
                let visible = black_box(manager.calculate_visibility(999));
                black_box(visible);
            });
        });
    }

    group.finish();
}

/// Benchmark: Interest filtering with different visibility percentages
/// Tests: 10%, 50%, 90% visible
fn bench_interest_filtering(c: &mut Criterion) {
    let mut group = c.benchmark_group("interest_filtering");
    group.measurement_time(Duration::from_secs(10));

    let (world, entities) = create_world_with_entities(1000);

    // Test different visibility percentages by adjusting radius
    for (percentage, radius) in [(10, 50.0), (50, 150.0), (90, 500.0)] {
        let mut filter = InterestFilter::new(50.0);
        filter.update_from_world(&world);
        filter.register_client(1, Vec3::new(500.0, 0.0, 500.0), radius);

        group.bench_with_input(
            BenchmarkId::new("visible_percent", percentage),
            &entities,
            |b, ents| {
                b.iter(|| {
                    let visible = black_box(filter.filter_updates(1, ents));
                    black_box(visible);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Bandwidth reduction validation
/// Target: 80-95% reduction
fn bench_bandwidth_reduction(c: &mut Criterion) {
    let mut group = c.benchmark_group("interest_bandwidth_reduction");
    group.measurement_time(Duration::from_secs(10));

    let (world, _entities) = create_world_with_entities(1000);

    let mut manager = InterestManager::new(50.0);
    manager.update_from_world(&world);

    // Register 100 clients with 100 unit AOI
    for i in 0..100 {
        let pos = Vec3::new(((i % 10) as f32) * 100.0, 0.0, ((i / 10) as f32) * 100.0);
        manager.set_client_interest(i, AreaOfInterest::new(pos, 100.0));

        // Initialize visibility cache
        manager.get_visibility_changes(i);
    }

    group.bench_function("compute_reduction_100_clients_1k_entities", |b| {
        b.iter(|| {
            let (without, with, reduction) = black_box(manager.compute_bandwidth_reduction());
            black_box((without, with, reduction));
        });
    });

    group.finish();
}

/// Benchmark: Update from world performance
fn bench_update_from_world(c: &mut Criterion) {
    let mut group = c.benchmark_group("interest_update_from_world");
    group.measurement_time(Duration::from_secs(10));

    for size in [100, 1000, 5000] {
        let (world, _) = create_world_with_entities(size);

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::new("entities", size), &world, |b, w| {
            let mut manager = InterestManager::new(50.0);
            b.iter(|| {
                black_box(manager.update_from_world(w));
            });
        });
    }

    group.finish();
}

/// Benchmark: Visibility change detection
fn bench_visibility_changes(c: &mut Criterion) {
    let mut group = c.benchmark_group("interest_visibility_changes");
    group.measurement_time(Duration::from_secs(10));

    let (world, _) = create_world_with_entities(1000);

    let mut manager = InterestManager::new(50.0);
    manager.update_from_world(&world);

    let client_id = 1;
    let aoi = AreaOfInterest::new(Vec3::new(500.0, 0.0, 500.0), 100.0);
    manager.set_client_interest(client_id, aoi);

    // Prime the cache
    manager.get_visibility_changes(client_id);

    group.bench_function("detect_changes", |b| {
        b.iter(|| {
            let changes = black_box(manager.get_visibility_changes(client_id));
            black_box(changes);
        });
    });

    group.finish();
}

/// Scalability test: Large world with many clients
fn bench_scalability(c: &mut Criterion) {
    let mut group = c.benchmark_group("interest_scalability");
    group.measurement_time(Duration::from_secs(20));
    group.sample_size(20);

    for (entities, clients) in [(1000, 100), (5000, 100), (10000, 100), (10000, 500)] {
        let (world, _) = create_world_with_entities(entities);

        let mut manager = InterestManager::new(100.0);
        manager.update_from_world(&world);

        // Register clients
        for i in 0..clients {
            let grid_size = (clients as f32).sqrt() as usize;
            let x = ((i % grid_size) as f32) * 200.0;
            let z = ((i / grid_size) as f32) * 200.0;
            let pos = Vec3::new(x, 0.0, z);
            manager.set_client_interest(i as u64, AreaOfInterest::new(pos, 100.0));
        }

        let label = format!("{}e_{}c", entities, clients);
        group.bench_with_input(BenchmarkId::new("full_cycle", &label), &label, |b, _| {
            b.iter(|| {
                // Full cycle: update + calculate all visibility
                for i in 0..clients {
                    let visible = black_box(manager.calculate_visibility(i as u64));
                    black_box(visible);
                }
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_visibility_single_client_1k,
    bench_visibility_100_clients,
    bench_spatial_queries,
    bench_nearest_entities,
    bench_interest_filtering,
    bench_bandwidth_reduction,
    bench_update_from_world,
    bench_visibility_changes,
    bench_scalability,
);
criterion_main!(benches);
