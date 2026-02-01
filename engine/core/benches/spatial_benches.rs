//! Benchmarks for spatial data structures.
//!
//! Tests performance of BVH, Spatial Grid, and linear search
//! on various entity counts and query patterns.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_core::{
    spatial::{Aabb, Bvh, SpatialGrid, SpatialGridConfig, SpatialQuery},
    Velocity, World,
};
use engine_math::Vec3;

/// Create a world with entities distributed in a grid pattern.
fn create_grid_world(entity_count: usize) -> World {
    let mut world = World::new();
    world.register::<Aabb>();
    world.register::<Velocity>();

    let grid_size = (entity_count as f32).cbrt().ceil() as usize;

    for x in 0..grid_size {
        for y in 0..grid_size {
            for z in 0..grid_size {
                if x * grid_size * grid_size + y * grid_size + z >= entity_count {
                    break;
                }

                let entity = world.spawn();
                let pos = Vec3::new(x as f32 * 5.0, y as f32 * 5.0, z as f32 * 5.0);
                let aabb = Aabb::from_center_half_extents(pos, Vec3::new(1.0, 1.0, 1.0));
                world.add(entity, aabb);
                world.add(entity, Velocity::new(1.0, 0.0, 0.0));
            }
        }
    }

    world
}

/// Create a world with entities randomly distributed.
#[allow(dead_code)]
fn create_random_world(entity_count: usize) -> World {
    let mut world = World::new();
    world.register::<Aabb>();
    world.register::<Velocity>();

    // Simple pseudo-random distribution
    let mut seed = 12345u64;
    for _ in 0..entity_count {
        seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
        let x = ((seed >> 16) % 1000) as f32 - 500.0;
        seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
        let y = ((seed >> 16) % 1000) as f32 - 500.0;
        seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
        let z = ((seed >> 16) % 1000) as f32 - 500.0;

        let entity = world.spawn();
        let pos = Vec3::new(x, y, z);
        let aabb = Aabb::from_center_half_extents(pos, Vec3::new(1.0, 1.0, 1.0));
        world.add(entity, aabb);
        world.add(entity, Velocity::new(1.0, 0.0, 0.0));
    }

    world
}

fn bench_radius_query_linear(c: &mut Criterion) {
    let mut group = c.benchmark_group("radius_query_linear");

    for entity_count in [100, 1_000, 10_000, 100_000] {
        group.throughput(Throughput::Elements(entity_count as u64));

        let world = create_grid_world(entity_count);

        group.bench_with_input(BenchmarkId::from_parameter(entity_count), &entity_count, |b, _| {
            b.iter(|| {
                let results = world.spatial_query_radius_linear(
                    black_box(Vec3::new(25.0, 25.0, 25.0)),
                    black_box(20.0),
                );
                black_box(results);
            });
        });
    }

    group.finish();
}

fn bench_radius_query_bvh(c: &mut Criterion) {
    let mut group = c.benchmark_group("radius_query_bvh");

    for entity_count in [100, 1_000, 10_000, 100_000] {
        group.throughput(Throughput::Elements(entity_count as u64));

        let world = create_grid_world(entity_count);

        group.bench_with_input(BenchmarkId::from_parameter(entity_count), &entity_count, |b, _| {
            b.iter(|| {
                let results = world.spatial_query_radius_bvh(
                    black_box(Vec3::new(25.0, 25.0, 25.0)),
                    black_box(20.0),
                );
                black_box(results);
            });
        });
    }

    group.finish();
}

fn bench_radius_query_grid(c: &mut Criterion) {
    let mut group = c.benchmark_group("radius_query_grid");

    for entity_count in [100, 1_000, 10_000, 100_000] {
        group.throughput(Throughput::Elements(entity_count as u64));

        let world = create_grid_world(entity_count);
        let config = SpatialGridConfig { cell_size: 10.0, entities_per_cell: 16 };

        group.bench_with_input(BenchmarkId::from_parameter(entity_count), &entity_count, |b, _| {
            b.iter(|| {
                let results = world.spatial_query_radius_grid(
                    black_box(Vec3::new(25.0, 25.0, 25.0)),
                    black_box(20.0),
                    black_box(config),
                );
                black_box(results);
            });
        });
    }

    group.finish();
}

fn bench_bvh_build(c: &mut Criterion) {
    let mut group = c.benchmark_group("bvh_build");

    for entity_count in [100, 1_000, 10_000, 100_000] {
        group.throughput(Throughput::Elements(entity_count as u64));

        let world = create_grid_world(entity_count);

        group.bench_with_input(BenchmarkId::from_parameter(entity_count), &entity_count, |b, _| {
            b.iter(|| {
                let bvh = Bvh::build(black_box(&world));
                black_box(bvh);
            });
        });
    }

    group.finish();
}

fn bench_grid_build(c: &mut Criterion) {
    let mut group = c.benchmark_group("grid_build");

    for entity_count in [100, 1_000, 10_000, 100_000] {
        group.throughput(Throughput::Elements(entity_count as u64));

        let world = create_grid_world(entity_count);
        let config = SpatialGridConfig { cell_size: 10.0, entities_per_cell: 16 };

        group.bench_with_input(BenchmarkId::from_parameter(entity_count), &entity_count, |b, _| {
            b.iter(|| {
                let grid = SpatialGrid::build(black_box(&world), black_box(config));
                black_box(grid);
            });
        });
    }

    group.finish();
}

fn bench_raycast_linear(c: &mut Criterion) {
    let mut group = c.benchmark_group("raycast_linear");

    for entity_count in [100, 1_000, 10_000, 100_000] {
        group.throughput(Throughput::Elements(entity_count as u64));

        let world = create_grid_world(entity_count);

        group.bench_with_input(BenchmarkId::from_parameter(entity_count), &entity_count, |b, _| {
            b.iter(|| {
                let hits = world.spatial_raycast_linear(
                    black_box(Vec3::new(-10.0, 25.0, 25.0)),
                    black_box(Vec3::new(1.0, 0.0, 0.0)),
                    black_box(1000.0),
                );
                black_box(hits);
            });
        });
    }

    group.finish();
}

fn bench_raycast_bvh(c: &mut Criterion) {
    let mut group = c.benchmark_group("raycast_bvh");

    for entity_count in [100, 1_000, 10_000, 100_000] {
        group.throughput(Throughput::Elements(entity_count as u64));

        let world = create_grid_world(entity_count);

        group.bench_with_input(BenchmarkId::from_parameter(entity_count), &entity_count, |b, _| {
            b.iter(|| {
                let hits = world.spatial_raycast_bvh(
                    black_box(Vec3::new(-10.0, 25.0, 25.0)),
                    black_box(Vec3::new(1.0, 0.0, 0.0)),
                    black_box(1000.0),
                );
                black_box(hits);
            });
        });
    }

    group.finish();
}

fn bench_bvh_reuse(c: &mut Criterion) {
    let mut group = c.benchmark_group("bvh_reuse");

    for entity_count in [1_000, 10_000, 100_000] {
        group.throughput(Throughput::Elements(entity_count as u64));

        let world = create_grid_world(entity_count);

        group.bench_with_input(BenchmarkId::from_parameter(entity_count), &entity_count, |b, _| {
            // Build BVH once
            let bvh = Bvh::build(&world);

            b.iter(|| {
                // Reuse the BVH for multiple queries
                let results =
                    bvh.query_radius(black_box(Vec3::new(25.0, 25.0, 25.0)), black_box(20.0));
                black_box(results);
            });
        });
    }

    group.finish();
}

fn bench_grid_reuse(c: &mut Criterion) {
    let mut group = c.benchmark_group("grid_reuse");

    for entity_count in [1_000, 10_000, 100_000] {
        group.throughput(Throughput::Elements(entity_count as u64));

        let world = create_grid_world(entity_count);
        let config = SpatialGridConfig { cell_size: 10.0, entities_per_cell: 16 };

        group.bench_with_input(BenchmarkId::from_parameter(entity_count), &entity_count, |b, _| {
            // Build grid once
            let grid = SpatialGrid::build(&world, config);

            b.iter(|| {
                // Reuse the grid for multiple queries
                let results =
                    grid.query_radius(black_box(Vec3::new(25.0, 25.0, 25.0)), black_box(20.0));
                black_box(results);
            });
        });
    }

    group.finish();
}

fn bench_aabb_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("aabb_operations");

    let aabb1 = Aabb::from_center_half_extents(Vec3::ZERO, Vec3::ONE);
    let aabb2 = Aabb::from_center_half_extents(Vec3::new(1.5, 0.0, 0.0), Vec3::ONE);

    group.bench_function("intersects", |b| {
        b.iter(|| {
            let result = aabb1.intersects(black_box(&aabb2));
            black_box(result);
        });
    });

    group.bench_function("merge", |b| {
        b.iter(|| {
            let result = aabb1.merge(black_box(&aabb2));
            black_box(result);
        });
    });

    group.bench_function("contains_point", |b| {
        b.iter(|| {
            let result = aabb1.contains_point(black_box(Vec3::new(0.5, 0.5, 0.5)));
            black_box(result);
        });
    });

    group.bench_function("ray_intersection", |b| {
        b.iter(|| {
            let result = aabb1.ray_intersection(
                black_box(Vec3::new(-5.0, 0.0, 0.0)),
                black_box(Vec3::new(1.0, 0.0, 0.0)),
                black_box(100.0),
            );
            black_box(result);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_radius_query_linear,
    bench_radius_query_bvh,
    bench_radius_query_grid,
    bench_bvh_build,
    bench_grid_build,
    bench_raycast_linear,
    bench_raycast_bvh,
    bench_bvh_reuse,
    bench_grid_reuse,
    bench_aabb_operations,
);

criterion_main!(benches);
