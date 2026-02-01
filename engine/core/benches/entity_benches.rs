use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
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

criterion_group!(
    benches,
    bench_entity_allocate,
    bench_entity_allocate_reuse,
    bench_entity_is_alive,
    bench_entity_free,
    bench_bulk_allocate,
    bench_allocate_free_allocate
);

criterion_main!(benches);
