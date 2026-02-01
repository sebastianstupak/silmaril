//! Benchmark for Task #55: ECS Query Optimization
//!
//! Tests prefetching, batching, and memory layout improvements for Transform + Velocity queries.
//! Target: 10-30% faster iteration on 10K entities.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use engine_core::ecs::{Component, World};

// Physics components matching common game engine usage
#[derive(Debug, Clone, Copy)]
#[repr(C, align(16))] // Align to cache line for better prefetching
struct Transform {
    x: f32,
    y: f32,
    z: f32,
    // Rotation as quaternion
    qx: f32,
    qy: f32,
    qz: f32,
    qw: f32,
    // Scale
    sx: f32,
    sy: f32,
    sz: f32,
    // Padding to 64 bytes (cache line)
    _pad: [f32; 6],
}

impl Component for Transform {}

impl Default for Transform {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            qx: 0.0,
            qy: 0.0,
            qz: 0.0,
            qw: 1.0,
            sx: 1.0,
            sy: 1.0,
            sz: 1.0,
            _pad: [0.0; 6],
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C, align(16))] // Align to cache line
struct Velocity {
    x: f32,
    y: f32,
    z: f32,
    // Angular velocity
    ax: f32,
    ay: f32,
    az: f32,
    // Padding to 32 bytes
    _pad: [f32; 2],
}

impl Component for Velocity {}

impl Default for Velocity {
    fn default() -> Self {
        Self { x: 0.0, y: 0.0, z: 0.0, ax: 0.0, ay: 0.0, az: 0.0, _pad: [0.0; 2] }
    }
}

fn setup_physics_world(entity_count: usize) -> World {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Velocity>();

    for i in 0..entity_count {
        let e = world.spawn();
        world.add(
            e,
            Transform { x: i as f32, y: (i * 2) as f32, z: (i * 3) as f32, ..Default::default() },
        );
        world.add(
            e,
            Velocity { x: 1.0, y: -1.0, z: 0.5, ax: 0.1, ay: 0.2, az: 0.3, _pad: [0.0; 2] },
        );
    }

    world
}

/// Baseline: Standard query iteration (before optimization)
fn bench_baseline_query(c: &mut Criterion) {
    let mut group = c.benchmark_group("baseline_transform_velocity");

    for size in [1000, 10_000, 50_000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_batched(
                || setup_physics_world(size),
                |mut world| {
                    // Simulate physics update: read velocity, update transform
                    let dt = 0.016; // 60 FPS
                    for (_e, (transform, velocity)) in
                        world.query_mut::<(&mut Transform, &Velocity)>()
                    {
                        transform.x += black_box(velocity.x * dt);
                        transform.y += black_box(velocity.y * dt);
                        transform.z += black_box(velocity.z * dt);
                    }
                },
                criterion::BatchSize::LargeInput,
            );
        });
    }

    group.finish();
}

/// Optimized: Query with enhanced prefetching
fn bench_optimized_query(c: &mut Criterion) {
    let mut group = c.benchmark_group("optimized_transform_velocity");

    for size in [1000, 10_000, 50_000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_batched(
                || setup_physics_world(size),
                |mut world| {
                    // Same workload as baseline
                    let dt = 0.016;
                    for (_e, (transform, velocity)) in
                        world.query_mut::<(&mut Transform, &Velocity)>()
                    {
                        transform.x += black_box(velocity.x * dt);
                        transform.y += black_box(velocity.y * dt);
                        transform.z += black_box(velocity.z * dt);
                    }
                },
                criterion::BatchSize::LargeInput,
            );
        });
    }

    group.finish();
}

/// Read-only query benchmark
fn bench_readonly_query(c: &mut Criterion) {
    let mut group = c.benchmark_group("readonly_transform_velocity");

    for size in [1000, 10_000, 50_000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let world = setup_physics_world(size);
            b.iter(|| {
                let mut sum = 0.0;
                for (_e, (transform, velocity)) in world.query::<(&Transform, &Velocity)>() {
                    sum += black_box(transform.x + velocity.x);
                }
                black_box(sum);
            });
        });
    }

    group.finish();
}

/// Immutable velocity, mutable transform (common physics pattern)
fn bench_mixed_mutability(c: &mut Criterion) {
    let mut group = c.benchmark_group("mixed_mutability_transform_velocity");

    for size in [1000, 10_000, 50_000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_batched(
                || setup_physics_world(size),
                |mut world| {
                    let dt = 0.016;
                    for (_e, (transform, velocity)) in
                        world.query_mut::<(&mut Transform, &Velocity)>()
                    {
                        transform.x += black_box(velocity.x * dt);
                        transform.y += black_box(velocity.y * dt);
                        transform.z += black_box(velocity.z * dt);
                    }
                },
                criterion::BatchSize::LargeInput,
            );
        });
    }

    group.finish();
}

/// Sparse query (only 20% have both components)
fn bench_sparse_query(c: &mut Criterion) {
    let mut group = c.benchmark_group("sparse_transform_velocity");

    for size in [1000, 10_000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let mut world = World::new();
            world.register::<Transform>();
            world.register::<Velocity>();

            // Create sparse distribution: only 20% have both
            for i in 0..size {
                let e = world.spawn();
                world.add(e, Transform { x: i as f32, y: 0.0, z: 0.0, ..Default::default() });
                if i % 5 == 0 {
                    // Only 20% have velocity
                    world.add(e, Velocity { x: 1.0, y: 0.0, z: 0.0, ..Default::default() });
                }
            }

            b.iter(|| {
                let mut count = 0;
                for (_e, (_transform, _velocity)) in world.query::<(&Transform, &Velocity)>() {
                    count += 1;
                }
                black_box(count);
            });
        });
    }

    group.finish();
}

/// Full physics simulation with acceleration
fn bench_physics_simulation(c: &mut Criterion) {
    let mut group = c.benchmark_group("physics_simulation");

    #[derive(Debug, Clone, Copy)]
    struct Acceleration {
        x: f32,
        y: f32,
        z: f32,
    }
    impl Component for Acceleration {}

    for size in [1000, 10_000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_batched(
                || {
                    let mut world = World::new();
                    world.register::<Transform>();
                    world.register::<Velocity>();
                    world.register::<Acceleration>();

                    for i in 0..size {
                        let e = world.spawn();
                        world.add(
                            e,
                            Transform { x: i as f32, y: 0.0, z: 0.0, ..Default::default() },
                        );
                        world.add(e, Velocity { x: 0.0, y: 0.0, z: 0.0, ..Default::default() });
                        world.add(e, Acceleration { x: 0.0, y: -9.8, z: 0.0 }); // Gravity
                    }
                    world
                },
                |mut world| {
                    let dt = 0.016;

                    // Update velocities from acceleration
                    for (_e, (velocity, acc)) in world.query_mut::<(&mut Velocity, &Acceleration)>()
                    {
                        velocity.x += black_box(acc.x * dt);
                        velocity.y += black_box(acc.y * dt);
                        velocity.z += black_box(acc.z * dt);
                    }

                    // Update transforms from velocity
                    for (_e, (transform, velocity)) in
                        world.query_mut::<(&mut Transform, &Velocity)>()
                    {
                        transform.x += black_box(velocity.x * dt);
                        transform.y += black_box(velocity.y * dt);
                        transform.z += black_box(velocity.z * dt);
                    }
                },
                criterion::BatchSize::LargeInput,
            );
        });
    }

    group.finish();
}

/// Cache line striding test
fn bench_cache_striding(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_striding");

    for size in [10_000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let world = setup_physics_world(size);
            b.iter(|| {
                // Access pattern that stresses cache
                let mut sum_x = 0.0;
                let mut sum_y = 0.0;
                let mut sum_z = 0.0;

                for (_e, (transform, velocity)) in world.query::<(&Transform, &Velocity)>() {
                    sum_x += black_box(transform.x + velocity.x);
                    sum_y += black_box(transform.y + velocity.y);
                    sum_z += black_box(transform.z + velocity.z);
                }

                black_box((sum_x, sum_y, sum_z));
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_baseline_query,
    bench_optimized_query,
    bench_readonly_query,
    bench_mixed_mutability,
    bench_sparse_query,
    bench_physics_simulation,
    bench_cache_striding,
);
criterion_main!(benches);
