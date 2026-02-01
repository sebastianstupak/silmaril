use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use engine_core::ecs::EntityAllocator;

fn bench_entity_allocate(c: &mut Criterion) {
    c.bench_function("entity_allocate", |b| {
        let mut alloc = EntityAllocator::new();
        b.iter(|| {
            black_box(alloc.allocate());
        });
    });
}

fn bench_entity_allocate_reuse(c: &mut Criterion) {
    let mut alloc = EntityAllocator::new();
    // Pre-allocate and free some entities to populate free list
    let entities: Vec<_> = (0..1000).map(|_| alloc.allocate()).collect();
    for entity in entities {
        alloc.free(entity);
    }

    c.bench_function("entity_allocate_reuse", |b| {
        b.iter(|| {
            black_box(alloc.allocate());
        });
    });
}

fn bench_entity_is_alive(c: &mut Criterion) {
    let mut alloc = EntityAllocator::new();
    let entity = alloc.allocate();

    c.bench_function("entity_is_alive", |b| {
        b.iter(|| {
            black_box(alloc.is_alive(entity));
        });
    });
}

fn bench_entity_free(c: &mut Criterion) {
    c.bench_function("entity_free", |b| {
        b.iter_batched(
            || {
                let mut alloc = EntityAllocator::new();
                let entity = alloc.allocate();
                (alloc, entity)
            },
            |(mut alloc, entity)| {
                black_box(alloc.free(entity));
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_bulk_allocate(c: &mut Criterion) {
    let mut group = c.benchmark_group("bulk_allocate");

    for count in [100, 1_000, 10_000, 100_000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            b.iter(|| {
                let mut alloc = EntityAllocator::new();
                for _ in 0..count {
                    black_box(alloc.allocate());
                }
            });
        });
    }

    group.finish();
}

fn bench_allocate_free_allocate(c: &mut Criterion) {
    c.bench_function("allocate_free_allocate", |b| {
        let mut alloc = EntityAllocator::new();
        b.iter(|| {
            let entity = alloc.allocate();
            alloc.free(entity);
            black_box(alloc.allocate());
        });
    });
}

fn bench_allocate_batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("allocate_batch");

    for count in [10, 100, 1_000, 10_000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            b.iter(|| {
                let mut alloc = EntityAllocator::new();
                black_box(alloc.allocate_batch(count));
            });
        });
    }

    group.finish();
}

fn bench_allocate_batch_vs_loop(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_vs_loop");

    for count in [100, 1_000, 10_000].iter() {
        group.bench_with_input(
            BenchmarkId::new("batch", count),
            count,
            |b, &count| {
                b.iter(|| {
                    let mut alloc = EntityAllocator::new();
                    black_box(alloc.allocate_batch(count));
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("loop", count),
            count,
            |b, &count| {
                b.iter(|| {
                    let mut alloc = EntityAllocator::new();
                    let mut entities = Vec::with_capacity(count);
                    for _ in 0..count {
                        entities.push(alloc.allocate());
                    }
                    black_box(entities);
                });
            },
        );
    }

    group.finish();
}

fn bench_allocate_batch_with_reuse(c: &mut Criterion) {
    let mut group = c.benchmark_group("allocate_batch_reuse");

    for count in [100, 1_000, 10_000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            b.iter_batched(
                || {
                    let mut alloc = EntityAllocator::new();
                    // Pre-populate half the batch size in the free list
                    let entities = alloc.allocate_batch(count / 2);
                    for entity in entities {
                        alloc.free(entity);
                    }
                    alloc
                },
                |mut alloc| {
                    black_box(alloc.allocate_batch(count));
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_entity_allocate,
    bench_entity_allocate_reuse,
    bench_entity_is_alive,
    bench_entity_free,
    bench_bulk_allocate,
    bench_allocate_free_allocate,
    bench_allocate_batch,
    bench_allocate_batch_vs_loop,
    bench_allocate_batch_with_reuse
);

criterion_main!(benches);
