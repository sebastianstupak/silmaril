//! Cache optimization benchmarks
//!
//! These benchmarks measure the impact of cache optimizations:
//! - Memory access patterns (sequential vs random)
//! - Prefetching effectiveness
//! - Cache line utilization
//! - Memory allocation patterns

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use engine_core::ecs::{Component, Entity, World};

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
#[allow(dead_code)]
struct Health {
    current: f32,
    max: f32,
}

impl Component for Health {}

/// Benchmark sequential access pattern (cache-friendly)
fn bench_sequential_access(c: &mut Criterion) {
    let mut group = c.benchmark_group("sequential_access");

    for entity_count in [100, 1000, 10000, 100000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(entity_count),
            &entity_count,
            |b, &count| {
                let mut world = World::new();
                world.register::<Position>();

                // Create entities sequentially
                for i in 0..count {
                    let entity = world.spawn();
                    world.add(entity, Position { x: i as f32, y: 0.0, z: 0.0 });
                }

                b.iter(|| {
                    // Sequential iteration - cache-friendly
                    let mut sum = 0.0_f32;
                    for (_entity, pos) in world.query::<&Position>() {
                        sum += black_box(pos.x + pos.y + pos.z);
                    }
                    black_box(sum);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark random access pattern (cache-unfriendly)
fn bench_random_access(c: &mut Criterion) {
    let mut group = c.benchmark_group("random_access");

    for entity_count in [100, 1000, 10000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(entity_count),
            &entity_count,
            |b, &count| {
                let mut world = World::new();
                world.register::<Position>();

                // Create entities
                let entities: Vec<Entity> = (0..count)
                    .map(|i| {
                        let entity = world.spawn();
                        world.add(entity, Position { x: i as f32, y: 0.0, z: 0.0 });
                        entity
                    })
                    .collect();

                b.iter(|| {
                    // Random access - cache-unfriendly
                    let mut sum = 0.0_f32;
                    for i in (0..count).step_by(7) {
                        // Step pattern to defeat prefetcher
                        if let Some(pos) = world.get::<Position>(entities[i]) {
                            sum += black_box(pos.x + pos.y + pos.z);
                        }
                    }
                    black_box(sum);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark two-component iteration (measures prefetch effectiveness)
fn bench_two_component_iteration(c: &mut Criterion) {
    let mut group = c.benchmark_group("two_component_iteration");

    for entity_count in [1000, 10000, 100000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(entity_count),
            &entity_count,
            |b, &count| {
                let mut world = World::new();
                world.register::<Position>();
                world.register::<Velocity>();

                // Create entities with both components
                for i in 0..count {
                    let entity = world.spawn();
                    world.add(entity, Position { x: i as f32, y: 0.0, z: 0.0 });
                    world.add(entity, Velocity { x: 1.0, y: 0.0, z: 0.0 });
                }

                b.iter(|| {
                    // Iterate both components - tests prefetch of second component
                    let mut sum = 0.0_f32;
                    for (_entity, (pos, vel)) in world.query::<(&Position, &Velocity)>() {
                        sum += black_box(pos.x * vel.x + pos.y * vel.y + pos.z * vel.z);
                    }
                    black_box(sum);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark entity allocation patterns
fn bench_allocation_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("allocation_patterns");

    group.bench_function("allocate_without_capacity", |b| {
        b.iter(|| {
            let mut world = World::new();
            world.register::<Position>();

            // Allocate without pre-reserving capacity
            for i in 0..1000 {
                let entity = world.spawn();
                world.add(entity, Position { x: i as f32, y: 0.0, z: 0.0 });
            }
            black_box(world);
        });
    });

    group.bench_function("allocate_with_capacity", |b| {
        b.iter(|| {
            let mut world = World::new();
            world.register::<Position>();

            // With SparseSet::new() now pre-allocating DEFAULT_CAPACITY,
            // this should have fewer reallocations
            for i in 0..1000 {
                let entity = world.spawn();
                world.add(entity, Position { x: i as f32, y: 0.0, z: 0.0 });
            }
            black_box(world);
        });
    });

    group.finish();
}

/// Benchmark cache line utilization
fn bench_cache_line_utilization(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_line_utilization");

    // Small components that fit many per cache line
    group.bench_function("small_components", |b| {
        let mut world = World::new();
        world.register::<Health>();

        for i in 0..10000 {
            let entity = world.spawn();
            world.add(entity, Health { current: i as f32, max: 100.0 });
        }

        b.iter(|| {
            let mut sum = 0.0_f32;
            for (_entity, health) in world.query::<&Health>() {
                sum += black_box(health.current);
            }
            black_box(sum);
        });
    });

    // Larger components that fill cache lines
    group.bench_function("large_components", |b| {
        #[derive(Debug, Clone, Copy)]
        struct LargeComponent {
            data: [f32; 16], // 64 bytes - fills one cache line
        }
        impl Component for LargeComponent {}

        let mut world = World::new();
        world.register::<LargeComponent>();

        for i in 0..10000 {
            let entity = world.spawn();
            world.add(entity, LargeComponent { data: [i as f32; 16] });
        }

        b.iter(|| {
            let mut sum = 0.0_f32;
            for (_entity, comp) in world.query::<&LargeComponent>() {
                sum += black_box(comp.data[0]);
            }
            black_box(sum);
        });
    });

    group.finish();
}

/// Benchmark realistic physics simulation pattern
fn bench_physics_simulation(c: &mut Criterion) {
    c.bench_function("physics_simulation", |b| {
        let mut world = World::new();
        world.register::<Position>();
        world.register::<Velocity>();

        for i in 0..10000 {
            let entity = world.spawn();
            world.add(entity, Position { x: i as f32, y: 0.0, z: 0.0 });
            world.add(entity, Velocity { x: 1.0, y: 0.0, z: 0.0 });
        }

        b.iter(|| {
            // Simulate physics update - this tests prefetch effectiveness
            let dt = black_box(0.016_f32); // 60 FPS
            for (_entity, (pos, vel)) in world.query_mut::<(&mut Position, &Velocity)>() {
                pos.x += vel.x * dt;
                pos.y += vel.y * dt;
                pos.z += vel.z * dt;
            }
        });
    });
}

criterion_group!(
    cache_benches,
    bench_sequential_access,
    bench_random_access,
    bench_two_component_iteration,
    bench_allocation_patterns,
    bench_cache_line_utilization,
    bench_physics_simulation,
);

criterion_main!(cache_benches);
