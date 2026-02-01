//! Comprehensive benchmarks for SparseSet data structure
//!
//! Benchmarks all core operations at various scales and densities.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_core::ecs::{Component, Entity, SparseSet};

#[derive(Clone, Copy)]
struct Position {
    x: f32,
    y: f32,
    z: f32,
}

impl Component for Position {}

#[derive(Clone, Copy)]
struct Velocity {
    x: f32,
    y: f32,
    z: f32,
}

impl Component for Velocity {}

// Large component to test cache behavior
#[derive(Clone, Copy)]
struct LargeComponent {
    data: [f32; 64], // 256 bytes
}

impl Component for LargeComponent {}

// Benchmark insertion at different scales
fn bench_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("sparse_set_insert");

    for size in [100, 1000, 10000, 100000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let mut storage = SparseSet::<Position>::with_capacity(size);
                for i in 0..size {
                    storage.insert(
                        Entity::new(i as u32, 0),
                        Position { x: i as f32, y: i as f32, z: i as f32 },
                    );
                }
                black_box(storage);
            });
        });
    }
    group.finish();
}

// Benchmark get operations
fn bench_get(c: &mut Criterion) {
    let mut group = c.benchmark_group("sparse_set_get");

    for size in [100, 1000, 10000, 100000].iter() {
        // Prepare storage
        let mut storage = SparseSet::<Position>::with_capacity(*size);
        for i in 0..*size {
            storage.insert(
                Entity::new(i as u32, 0),
                Position { x: i as f32, y: i as f32, z: i as f32 },
            );
        }

        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let mut sum = 0.0;
                for i in 0..size {
                    if let Some(pos) = storage.get(Entity::new(i as u32, 0)) {
                        sum += pos.x;
                    }
                }
                black_box(sum);
            });
        });
    }
    group.finish();
}

// Benchmark get_mut operations
fn bench_get_mut(c: &mut Criterion) {
    let mut group = c.benchmark_group("sparse_set_get_mut");

    for size in [100, 1000, 10000, 100000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_batched(
                || {
                    let mut storage = SparseSet::<Position>::with_capacity(size);
                    for i in 0..size {
                        storage.insert(
                            Entity::new(i as u32, 0),
                            Position { x: i as f32, y: i as f32, z: i as f32 },
                        );
                    }
                    storage
                },
                |mut storage| {
                    for i in 0..size {
                        if let Some(pos) = storage.get_mut(Entity::new(i as u32, 0)) {
                            pos.x += 1.0;
                        }
                    }
                    black_box(storage);
                },
                criterion::BatchSize::LargeInput,
            );
        });
    }
    group.finish();
}

// Benchmark remove operations
fn bench_remove(c: &mut Criterion) {
    let mut group = c.benchmark_group("sparse_set_remove");

    for size in [100, 1000, 10000, 100000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_batched(
                || {
                    let mut storage = SparseSet::<Position>::with_capacity(size);
                    for i in 0..size {
                        storage.insert(
                            Entity::new(i as u32, 0),
                            Position { x: i as f32, y: i as f32, z: i as f32 },
                        );
                    }
                    storage
                },
                |mut storage| {
                    for i in 0..size {
                        storage.remove(Entity::new(i as u32, 0));
                    }
                    black_box(storage);
                },
                criterion::BatchSize::LargeInput,
            );
        });
    }
    group.finish();
}

// Benchmark sequential iteration
fn bench_iteration(c: &mut Criterion) {
    let mut group = c.benchmark_group("sparse_set_iteration");

    for size in [100, 1000, 10000, 100000].iter() {
        // Prepare storage
        let mut storage = SparseSet::<Position>::with_capacity(*size);
        for i in 0..*size {
            storage.insert(
                Entity::new(i as u32, 0),
                Position { x: i as f32, y: i as f32, z: i as f32 },
            );
        }

        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &storage, |b, storage| {
            b.iter(|| {
                let mut sum = 0.0;
                for (_entity, pos) in storage.iter() {
                    sum += pos.x + pos.y + pos.z;
                }
                black_box(sum);
            });
        });
    }
    group.finish();
}

// Benchmark mutable iteration
fn bench_iteration_mut(c: &mut Criterion) {
    let mut group = c.benchmark_group("sparse_set_iteration_mut");

    for size in [100, 1000, 10000, 100000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_batched(
                || {
                    let mut storage = SparseSet::<Position>::with_capacity(size);
                    for i in 0..size {
                        storage.insert(
                            Entity::new(i as u32, 0),
                            Position { x: i as f32, y: i as f32, z: i as f32 },
                        );
                    }
                    storage
                },
                |mut storage| {
                    for (_entity, pos) in storage.iter_mut() {
                        pos.x += 1.0;
                        pos.y += 1.0;
                        pos.z += 1.0;
                    }
                    black_box(storage);
                },
                criterion::BatchSize::LargeInput,
            );
        });
    }
    group.finish();
}

// Benchmark sparse vs dense access patterns
fn bench_sparse_vs_dense(c: &mut Criterion) {
    let mut group = c.benchmark_group("sparse_set_density");

    let size = 10000;

    // Dense: 100% filled
    group.bench_function("dense_100%", |b| {
        let mut storage = SparseSet::<Position>::with_capacity(size);
        for i in 0..size {
            storage.insert(
                Entity::new(i as u32, 0),
                Position { x: i as f32, y: i as f32, z: i as f32 },
            );
        }

        b.iter(|| {
            let mut sum = 0.0;
            for (_entity, pos) in storage.iter() {
                sum += pos.x;
            }
            black_box(sum);
        });
    });

    // Sparse: 10% filled
    group.bench_function("sparse_10%", |b| {
        let mut storage = SparseSet::<Position>::with_capacity(size / 10);
        for i in (0..size).step_by(10) {
            storage.insert(
                Entity::new(i as u32, 0),
                Position { x: i as f32, y: i as f32, z: i as f32 },
            );
        }

        b.iter(|| {
            let mut sum = 0.0;
            for (_entity, pos) in storage.iter() {
                sum += pos.x;
            }
            black_box(sum);
        });
    });

    group.finish();
}

// Benchmark random access patterns
fn bench_random_access(c: &mut Criterion) {
    let mut group = c.benchmark_group("sparse_set_random_access");

    for size in [1000, 10000, 100000].iter() {
        // Prepare storage
        let mut storage = SparseSet::<Position>::with_capacity(*size);
        for i in 0..*size {
            storage.insert(
                Entity::new(i as u32, 0),
                Position { x: i as f32, y: i as f32, z: i as f32 },
            );
        }

        // Generate pseudo-random access pattern
        let indices: Vec<u32> =
            (0..*size as u32).map(|i| (i * 2654435761) % (*size as u32)).collect();

        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
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
    }
    group.finish();
}

// Benchmark random removal
fn bench_random_removal(c: &mut Criterion) {
    let mut group = c.benchmark_group("sparse_set_random_removal");

    for size in [1000, 10000].iter() {
        // Generate pseudo-random removal order
        let indices: Vec<u32> =
            (0..*size as u32).map(|i| (i * 2654435761) % (*size as u32)).collect();

        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_batched(
                || {
                    let mut storage = SparseSet::<Position>::with_capacity(size);
                    for i in 0..size {
                        storage.insert(
                            Entity::new(i as u32, 0),
                            Position { x: i as f32, y: i as f32, z: i as f32 },
                        );
                    }
                    storage
                },
                |mut storage| {
                    for &idx in indices.iter() {
                        storage.remove(Entity::new(idx, 0));
                    }
                    black_box(storage);
                },
                criterion::BatchSize::LargeInput,
            );
        });
    }
    group.finish();
}

// Benchmark bulk insertion
fn bench_bulk_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("sparse_set_bulk_insert");

    let size = 10000;
    group.throughput(Throughput::Elements(size as u64));

    group.bench_function("with_capacity", |b| {
        b.iter(|| {
            let mut storage = SparseSet::<Position>::with_capacity(size);
            for i in 0..size {
                storage.insert(
                    Entity::new(i as u32, 0),
                    Position { x: i as f32, y: i as f32, z: i as f32 },
                );
            }
            black_box(storage);
        });
    });

    group.bench_function("without_capacity", |b| {
        b.iter(|| {
            let mut storage = SparseSet::<Position>::new();
            for i in 0..size {
                storage.insert(
                    Entity::new(i as u32, 0),
                    Position { x: i as f32, y: i as f32, z: i as f32 },
                );
            }
            black_box(storage);
        });
    });

    group.finish();
}

// Benchmark large components
fn bench_large_component(c: &mut Criterion) {
    let mut group = c.benchmark_group("sparse_set_large_component");

    let size = 10000;
    group.throughput(Throughput::Elements(size as u64));

    group.bench_function("iteration", |b| {
        let mut storage = SparseSet::<LargeComponent>::with_capacity(size);
        for i in 0..size {
            storage.insert(Entity::new(i as u32, 0), LargeComponent { data: [i as f32; 64] });
        }

        b.iter(|| {
            let mut sum = 0.0;
            for (_entity, comp) in storage.iter() {
                sum += comp.data[0];
            }
            black_box(sum);
        });
    });

    group.finish();
}

// Benchmark contains operation
fn bench_contains(c: &mut Criterion) {
    let mut group = c.benchmark_group("sparse_set_contains");

    for size in [100, 1000, 10000, 100000].iter() {
        let mut storage = SparseSet::<Position>::with_capacity(*size);
        for i in 0..*size {
            storage.insert(
                Entity::new(i as u32, 0),
                Position { x: i as f32, y: i as f32, z: i as f32 },
            );
        }

        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let mut count = 0;
                for i in 0..size {
                    if storage.contains(Entity::new(i as u32, 0)) {
                        count += 1;
                    }
                }
                black_box(count);
            });
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_insert,
    bench_get,
    bench_get_mut,
    bench_remove,
    bench_iteration,
    bench_iteration_mut,
    bench_sparse_vs_dense,
    bench_random_access,
    bench_random_removal,
    bench_bulk_insert,
    bench_large_component,
    bench_contains,
);
criterion_main!(benches);
