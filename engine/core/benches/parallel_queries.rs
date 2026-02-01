//! Benchmarks for parallel query iteration
//!
//! Tests parallel query performance vs single-threaded across different entity counts.
//! Target: 6-8x speedup on 8-core systems.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_core::ecs::{Component, ParallelWorld, World};
use rayon::prelude::*;

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
struct Mass {
    value: f32,
}
impl Component for Mass {}

/// Helper to create a world with N entities
fn create_world_with_entities(entity_count: usize) -> World {
    let mut world = World::new();
    world.register::<Position>();
    world.register::<Velocity>();
    world.register::<Mass>();

    for i in 0..entity_count {
        let entity = world.spawn();
        world.add(
            entity,
            Position {
                x: i as f32,
                y: i as f32 * 2.0,
                z: i as f32 * 3.0,
            },
        );
        world.add(
            entity,
            Velocity {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
        );
        world.add(entity, Mass { value: 1.0 + i as f32 * 0.01 });
    }

    world
}

/// Benchmark single-component query iteration (immutable)
fn bench_single_component_iter(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_component_immutable");

    for entity_count in [1_000, 10_000, 100_000].iter() {
        let world = create_world_with_entities(*entity_count);
        group.throughput(Throughput::Elements(*entity_count as u64));

        // Single-threaded (using rayon with 1 thread)
        group.bench_with_input(
            BenchmarkId::new("sequential", entity_count),
            entity_count,
            |b, _| {
                b.iter(|| {
                    rayon::ThreadPoolBuilder::new()
                        .num_threads(1)
                        .build()
                        .unwrap()
                        .install(|| {
                            let sum: f32 = world
                                .par_query::<Position>()
                                .map(|(_, pos)| pos.x)
                                .sum();
                            black_box(sum);
                        });
                });
            },
        );

        // Parallel (default thread count)
        group.bench_with_input(
            BenchmarkId::new("parallel", entity_count),
            entity_count,
            |b, _| {
                b.iter(|| {
                    let sum: f32 = world
                        .par_query::<Position>()
                        .map(|(_, pos)| pos.x)
                        .sum();
                    black_box(sum);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark single-component query iteration (mutable)
fn bench_single_component_iter_mut(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_component_mutable");

    for entity_count in [1_000, 10_000, 100_000].iter() {
        group.throughput(Throughput::Elements(*entity_count as u64));

        // Single-threaded
        group.bench_with_input(
            BenchmarkId::new("sequential", entity_count),
            entity_count,
            |b, _| {
                let mut world = create_world_with_entities(*entity_count);
                b.iter(|| {
                    rayon::ThreadPoolBuilder::new()
                        .num_threads(1)
                        .build()
                        .unwrap()
                        .install(|| {
                            world.par_query_mut::<Position>().for_each(|(_, pos)| {
                                pos.x += 1.0;
                                pos.y += 1.0;
                                pos.z += 1.0;
                                black_box(pos);
                            });
                        });
                });
            },
        );

        // Parallel
        group.bench_with_input(
            BenchmarkId::new("parallel", entity_count),
            entity_count,
            |b, _| {
                let mut world = create_world_with_entities(*entity_count);
                b.iter(|| {
                    world.par_query_mut::<Position>().for_each(|(_, pos)| {
                        pos.x += 1.0;
                        pos.y += 1.0;
                        pos.z += 1.0;
                        black_box(pos);
                    });
                });
            },
        );
    }

    group.finish();
}

/// Benchmark two-component query iteration (immutable)
fn bench_two_component_iter(c: &mut Criterion) {
    let mut group = c.benchmark_group("two_component_immutable");

    for entity_count in [1_000, 10_000, 100_000].iter() {
        let world = create_world_with_entities(*entity_count);
        group.throughput(Throughput::Elements(*entity_count as u64));

        // Single-threaded
        group.bench_with_input(
            BenchmarkId::new("sequential", entity_count),
            entity_count,
            |b, _| {
                b.iter(|| {
                    rayon::ThreadPoolBuilder::new()
                        .num_threads(1)
                        .build()
                        .unwrap()
                        .install(|| {
                            let sum: f32 = world
                                .par_query2::<Position, Velocity>()
                                .map(|(_, (pos, vel))| pos.x + vel.x)
                                .sum();
                            black_box(sum);
                        });
                });
            },
        );

        // Parallel
        group.bench_with_input(
            BenchmarkId::new("parallel", entity_count),
            entity_count,
            |b, _| {
                b.iter(|| {
                    let sum: f32 = world
                        .par_query2::<Position, Velocity>()
                        .map(|(_, (pos, vel))| pos.x + vel.x)
                        .sum();
                    black_box(sum);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark two-component query iteration (mixed mutability)
fn bench_two_component_iter_mut(c: &mut Criterion) {
    let mut group = c.benchmark_group("two_component_mixed_mut");

    for entity_count in [1_000, 10_000, 100_000].iter() {
        group.throughput(Throughput::Elements(*entity_count as u64));

        // Single-threaded
        group.bench_with_input(
            BenchmarkId::new("sequential", entity_count),
            entity_count,
            |b, _| {
                let mut world = create_world_with_entities(*entity_count);
                b.iter(|| {
                    rayon::ThreadPoolBuilder::new()
                        .num_threads(1)
                        .build()
                        .unwrap()
                        .install(|| {
                            world
                                .par_query2_mut::<Position, Velocity>()
                                .for_each(|(_, (pos, vel))| {
                                    pos.x += vel.x;
                                    pos.y += vel.y;
                                    pos.z += vel.z;
                                    black_box(pos);
                                });
                        });
                });
            },
        );

        // Parallel
        group.bench_with_input(
            BenchmarkId::new("parallel", entity_count),
            entity_count,
            |b, _| {
                let mut world = create_world_with_entities(*entity_count);
                b.iter(|| {
                    world
                        .par_query2_mut::<Position, Velocity>()
                        .for_each(|(_, (pos, vel))| {
                            pos.x += vel.x;
                            pos.y += vel.y;
                            pos.z += vel.z;
                            black_box(pos);
                        });
                });
            },
        );
    }

    group.finish();
}

/// Benchmark realistic physics-style workload (2-component)
fn bench_physics_workload(c: &mut Criterion) {
    let mut group = c.benchmark_group("physics_workload");

    for entity_count in [1_000, 10_000, 100_000].iter() {
        group.throughput(Throughput::Elements(*entity_count as u64));

        // Single-threaded
        group.bench_with_input(
            BenchmarkId::new("sequential", entity_count),
            entity_count,
            |b, _| {
                let mut world = create_world_with_entities(*entity_count);
                b.iter(|| {
                    rayon::ThreadPoolBuilder::new()
                        .num_threads(1)
                        .build()
                        .unwrap()
                        .install(|| {
                            world
                                .par_query2_mut::<Position, Velocity>()
                                .for_each(|(_, (pos, vel))| {
                                    // Simulate simple physics step
                                    let dt = 0.016; // 60 FPS
                                    pos.x += vel.x * dt;
                                    pos.y += vel.y * dt;
                                    pos.z += vel.z * dt;
                                    black_box(pos);
                                });
                        });
                });
            },
        );

        // Parallel
        group.bench_with_input(
            BenchmarkId::new("parallel", entity_count),
            entity_count,
            |b, _| {
                let mut world = create_world_with_entities(*entity_count);
                b.iter(|| {
                    world
                        .par_query2_mut::<Position, Velocity>()
                        .for_each(|(_, (pos, vel))| {
                            // Simulate simple physics step
                            let dt = 0.016; // 60 FPS
                            pos.x += vel.x * dt;
                            pos.y += vel.y * dt;
                            pos.z += vel.z * dt;
                            black_box(pos);
                        });
                });
            },
        );
    }

    group.finish();
}

/// Benchmark parallel overhead for small entity counts
fn bench_parallel_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_overhead");

    for entity_count in [10, 50, 100, 500].iter() {
        let world = create_world_with_entities(*entity_count);
        group.throughput(Throughput::Elements(*entity_count as u64));

        // Single-threaded
        group.bench_with_input(
            BenchmarkId::new("sequential", entity_count),
            entity_count,
            |b, _| {
                b.iter(|| {
                    rayon::ThreadPoolBuilder::new()
                        .num_threads(1)
                        .build()
                        .unwrap()
                        .install(|| {
                            let sum: f32 = world
                                .par_query::<Position>()
                                .map(|(_, pos)| pos.x)
                                .sum();
                            black_box(sum);
                        });
                });
            },
        );

        // Parallel
        group.bench_with_input(
            BenchmarkId::new("parallel", entity_count),
            entity_count,
            |b, _| {
                b.iter(|| {
                    let sum: f32 = world
                        .par_query::<Position>()
                        .map(|(_, pos)| pos.x)
                        .sum();
                    black_box(sum);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark speedup scaling with different thread counts
fn bench_thread_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("thread_scaling");
    let entity_count = 100_000;
    let world = create_world_with_entities(entity_count);

    group.throughput(Throughput::Elements(entity_count as u64));

    // Benchmark with different thread pool sizes
    for thread_count in [1, 2, 4, 8].iter() {
        group.bench_with_input(
            BenchmarkId::new("threads", thread_count),
            thread_count,
            |b, &threads| {
                b.iter(|| {
                    rayon::ThreadPoolBuilder::new()
                        .num_threads(threads)
                        .build()
                        .unwrap()
                        .install(|| {
                            let sum: f32 = world
                                .par_query2::<Position, Velocity>()
                                .map(|(_, (pos, vel))| pos.x + vel.x)
                                .sum();
                            black_box(sum);
                        });
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_single_component_iter,
    bench_single_component_iter_mut,
    bench_two_component_iter,
    bench_two_component_iter_mut,
    bench_physics_workload,
    bench_parallel_overhead,
    bench_thread_scaling,
);

criterion_main!(benches);
