//! Benchmarks for memory allocators
//!
//! Compares Arena, Pool, and Frame allocators against standard allocation.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_core::allocators::{Arena, FrameAllocator, PoolAllocator};
use engine_core::Transform;

fn bench_arena_vs_vec(c: &mut Criterion) {
    let mut group = c.benchmark_group("arena_vs_vec");

    for size in [100, 1000, 10000].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        // Vec allocation (baseline)
        group.bench_with_input(BenchmarkId::new("vec", size), size, |b, &size| {
            b.iter(|| {
                let mut v = Vec::with_capacity(size);
                for _ in 0..size {
                    v.push(black_box(Transform::default()));
                }
                black_box(v);
            });
        });

        // Arena allocation
        group.bench_with_input(BenchmarkId::new("arena", size), size, |b, &size| {
            b.iter(|| {
                let mut arena = Arena::new();
                let _slice = arena.alloc_slice::<Transform>(size);
                black_box(&arena);
            });
        });
    }

    group.finish();
}

fn bench_pool_vs_box(c: &mut Criterion) {
    let mut group = c.benchmark_group("pool_vs_box");

    for count in [100, 1000].iter() {
        group.throughput(Throughput::Elements(*count as u64));

        // Box allocation (baseline)
        group.bench_with_input(BenchmarkId::new("box", count), count, |b, &count| {
            b.iter(|| {
                let mut boxes = Vec::new();
                for _ in 0..count {
                    boxes.push(Box::new(black_box(Transform::default())));
                }
                black_box(boxes);
            });
        });

        // Pool allocation
        group.bench_with_input(BenchmarkId::new("pool", count), count, |b, &count| {
            b.iter(|| {
                let mut pool = PoolAllocator::<Transform>::with_capacity(count);
                let ptrs: Vec<*const Transform> = (0..count)
                    .map(|_| pool.alloc(black_box(Transform::default())) as *const _)
                    .collect();
                black_box(ptrs);
            });
        });
    }

    group.finish();
}

fn bench_frame_reset(c: &mut Criterion) {
    let mut group = c.benchmark_group("frame_reset");

    // Allocate and reset patterns
    group.bench_function("frame_1k_allocations", |b| {
        let mut frame = FrameAllocator::with_capacity(1024 * 1024);
        b.iter(|| {
            for _ in 0..1000 {
                let _val = frame.alloc(black_box(42u64));
            }
            frame.reset();
        });
    });

    group.bench_function("vec_1k_allocations", |b| {
        b.iter(|| {
            let mut v = Vec::new();
            for _ in 0..1000 {
                v.push(black_box(42u64));
            }
            black_box(v);
        });
    });

    group.finish();
}

fn bench_allocation_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("allocation_patterns");

    // Burst allocation pattern (common in game engines)
    group.bench_function("burst_arena", |b| {
        b.iter(|| {
            let mut arena = Arena::new();
            // Simulate per-frame burst allocations
            for _ in 0..10 {
                let _slice = arena.alloc_slice::<Transform>(100);
            }
            black_box(&arena);
        });
    });

    group.bench_function("burst_vec", |b| {
        b.iter(|| {
            let mut vecs = Vec::new();
            for _ in 0..10 {
                let mut v = Vec::with_capacity(100);
                for _ in 0..100 {
                    v.push(Transform::default());
                }
                vecs.push(v);
            }
            black_box(vecs);
        });
    });

    group.finish();
}

fn bench_pool_reuse(c: &mut Criterion) {
    let mut group = c.benchmark_group("pool_reuse");

    group.bench_function("pool_alloc_free_reuse", |b| {
        let mut pool = PoolAllocator::<Transform>::with_capacity(1000);
        b.iter(|| {
            // Allocate 100 items
            let ptrs: Vec<*mut Transform> = (0..100)
                .map(|_| pool.alloc(black_box(Transform::default())) as *mut Transform)
                .collect();

            // Free them all
            for &ptr in ptrs.iter() {
                unsafe {
                    pool.free(&mut *ptr);
                }
            }

            // Allocate again (should reuse)
            let ptrs2: Vec<*mut Transform> = (0..100)
                .map(|_| pool.alloc(black_box(Transform::default())) as *mut Transform)
                .collect();

            // Cleanup
            for &ptr in ptrs2.iter() {
                unsafe {
                    pool.free(&mut *ptr);
                }
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_arena_vs_vec,
    bench_pool_vs_box,
    bench_frame_reset,
    bench_allocation_patterns,
    bench_pool_reuse,
);
criterion_main!(benches);
