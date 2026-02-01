//! Benchmark for component get() optimization
//!
//! Compares old vs optimized get() implementations to measure
//! the 3x improvement target (49ns -> 15-20ns).

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_core::ecs::change_detection::Tick;
use engine_core::ecs::{Component, Entity, SparseSet, World};

#[derive(Clone, Copy, Debug)]
struct Position {
    x: f32,
    y: f32,
    z: f32,
}

impl Component for Position {}

#[derive(Clone, Copy, Debug)]
struct Velocity {
    x: f32,
    y: f32,
    z: f32,
}

impl Component for Velocity {}

/// Benchmark single component get() operation
fn bench_component_get_single(c: &mut Criterion) {
    let mut group = c.benchmark_group("component_get_single");
    group.throughput(Throughput::Elements(1));

    for size in [100, 1000, 10000, 100000].iter() {
        // Prepare storage with components
        let mut storage = SparseSet::<Position>::with_capacity(*size);
        for i in 0..*size {
            storage.insert(
                Entity::new(i as u32, 0),
                Position { x: i as f32, y: i as f32, z: i as f32 },
                Tick::new(),
            );
        }

        group.bench_with_input(BenchmarkId::new("get_standard", size), &storage, |b, storage| {
            b.iter(|| {
                // Access middle entity to avoid cache effects
                let entity = Entity::new((size / 2) as u32, 0);
                let pos = storage.get(entity);
                black_box(pos);
            });
        });

        group.bench_with_input(
            BenchmarkId::new("get_unchecked_fast", size),
            &storage,
            |b, storage| {
                b.iter(|| {
                    // SAFETY: We know the entity exists and is valid
                    let entity = Entity::new((size / 2) as u32, 0);
                    let pos = unsafe { storage.get_unchecked_fast(entity) };
                    black_box(pos);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark batch component get() operations
fn bench_component_get_batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("component_get_batch");

    for size in [100, 1000, 10000, 100000].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        // Prepare storage
        let mut storage = SparseSet::<Position>::with_capacity(*size);
        for i in 0..*size {
            storage.insert(
                Entity::new(i as u32, 0),
                Position { x: i as f32, y: i as f32, z: i as f32 },
                Tick::new(),
            );
        }

        group.bench_with_input(BenchmarkId::new("get_standard", size), &storage, |b, storage| {
            b.iter(|| {
                let mut sum = 0.0;
                for i in 0..*size {
                    if let Some(pos) = storage.get(Entity::new(i as u32, 0)) {
                        sum += pos.x;
                    }
                }
                black_box(sum);
            });
        });

        group.bench_with_input(
            BenchmarkId::new("get_unchecked_fast", size),
            &storage,
            |b, storage| {
                b.iter(|| {
                    let mut sum = 0.0;
                    // SAFETY: We know all entities [0..size) exist
                    for i in 0..*size {
                        let pos = unsafe { storage.get_unchecked_fast(Entity::new(i as u32, 0)) };
                        sum += pos.x;
                    }
                    black_box(sum);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark query iteration with optimized get()
fn bench_query_iteration_optimized(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_iteration_optimized");

    for size in [100, 1000, 10000, 100000].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        // Setup world
        let mut world = World::new();
        world.register::<Position>();

        for i in 0..*size {
            let entity = world.spawn();
            world.add(entity, Position { x: i as f32, y: i as f32, z: i as f32 });
        }

        group.bench_with_input(BenchmarkId::new("standard", size), &world, |b, world| {
            b.iter(|| {
                let mut sum = 0.0;
                for (_entity, pos) in world.query::<&Position>() {
                    sum += pos.x + pos.y + pos.z;
                }
                black_box(sum);
            });
        });
    }

    group.finish();
}

/// Benchmark two-component query iteration
fn bench_query_two_components(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_two_components");

    for size in [100, 1000, 10000, 100000].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        // Setup world
        let mut world = World::new();
        world.register::<Position>();
        world.register::<Velocity>();

        for i in 0..*size {
            let entity = world.spawn();
            world.add(entity, Position { x: i as f32, y: i as f32, z: i as f32 });
            world.add(entity, Velocity { x: 1.0, y: 2.0, z: 3.0 });
        }

        group.bench_with_input(BenchmarkId::new("standard", size), &world, |b, world| {
            b.iter(|| {
                let mut sum = 0.0;
                for (_entity, (pos, vel)) in world.query::<(&Position, &Velocity)>() {
                    sum += pos.x + vel.x;
                }
                black_box(sum);
            });
        });
    }

    group.finish();
}

/// Benchmark random access pattern (worst case for cache)
fn bench_component_get_random(c: &mut Criterion) {
    let mut group = c.benchmark_group("component_get_random");

    for size in [1000, 10000, 100000].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        // Prepare storage
        let mut storage = SparseSet::<Position>::with_capacity(*size);
        for i in 0..*size {
            storage.insert(
                Entity::new(i as u32, 0),
                Position { x: i as f32, y: i as f32, z: i as f32 },
                Tick::new(),
            );
        }

        // Generate pseudo-random access pattern
        let indices: Vec<u32> =
            (0..*size as u32).map(|i| (i * 2654435761) % (*size as u32)).collect();

        group.bench_with_input(
            BenchmarkId::new("get_standard", size),
            &(&storage, &indices),
            |b, (storage, indices)| {
                b.iter(|| {
                    let mut sum = 0.0;
                    for &idx in indices.iter() {
                        if let Some(pos) = storage.get(Entity::new(idx, 0)) {
                            sum += pos.x;
                        }
                    }
                    black_box(sum);
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("get_unchecked_fast", size),
            &(&storage, &indices),
            |b, (storage, indices)| {
                b.iter(|| {
                    let mut sum = 0.0;
                    // SAFETY: We know all indices are valid
                    for &idx in indices.iter() {
                        let pos = unsafe { storage.get_unchecked_fast(Entity::new(idx, 0)) };
                        sum += pos.x;
                    }
                    black_box(sum);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark cache effects with different component sizes
fn bench_component_size_effects(c: &mut Criterion) {
    let mut group = c.benchmark_group("component_size_effects");

    #[derive(Clone, Copy)]
    struct Small {
        x: f32,
    }
    impl Component for Small {}

    #[derive(Clone, Copy)]
    struct Medium {
        data: [f32; 8],
    }
    impl Component for Medium {}

    #[derive(Clone, Copy)]
    struct Large {
        data: [f32; 32],
    }
    impl Component for Large {}

    let size = 10000;
    group.throughput(Throughput::Elements(size as u64));

    // Small component (4 bytes)
    let mut small_storage = SparseSet::<Small>::with_capacity(size);
    for i in 0..size {
        small_storage.insert(Entity::new(i as u32, 0), Small { x: i as f32 }, Tick::new());
    }

    group.bench_function("small_4bytes", |b| {
        b.iter(|| {
            let mut sum = 0.0;
            for i in 0..size {
                if let Some(s) = small_storage.get(Entity::new(i as u32, 0)) {
                    sum += s.x;
                }
            }
            black_box(sum);
        });
    });

    // Medium component (32 bytes)
    let mut medium_storage = SparseSet::<Medium>::with_capacity(size);
    for i in 0..size {
        medium_storage.insert(
            Entity::new(i as u32, 0),
            Medium { data: [i as f32; 8] },
            Tick::new(),
        );
    }

    group.bench_function("medium_32bytes", |b| {
        b.iter(|| {
            let mut sum = 0.0;
            for i in 0..size {
                if let Some(m) = medium_storage.get(Entity::new(i as u32, 0)) {
                    sum += m.data[0];
                }
            }
            black_box(sum);
        });
    });

    // Large component (128 bytes)
    let mut large_storage = SparseSet::<Large>::with_capacity(size);
    for i in 0..size {
        large_storage.insert(Entity::new(i as u32, 0), Large { data: [i as f32; 32] }, Tick::new());
    }

    group.bench_function("large_128bytes", |b| {
        b.iter(|| {
            let mut sum = 0.0;
            for i in 0..size {
                if let Some(l) = large_storage.get(Entity::new(i as u32, 0)) {
                    sum += l.data[0];
                }
            }
            black_box(sum);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_component_get_single,
    bench_component_get_batch,
    bench_query_iteration_optimized,
    bench_query_two_components,
    bench_component_get_random,
    bench_component_size_effects,
);
criterion_main!(benches);
