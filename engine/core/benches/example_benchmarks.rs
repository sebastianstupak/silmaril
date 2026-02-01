//! Example benchmarks demonstrating benchmark infrastructure
//!
//! Run with: cargo bench -p engine-core

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};

// =============================================================================
// Basic Benchmarks
// =============================================================================

fn bench_simple_operation(c: &mut Criterion) {
    c.bench_function("simple_addition", |b| {
        b.iter(|| {
            let a = black_box(42);
            let b = black_box(100);
            black_box(a + b)
        });
    });
}

fn bench_vector_creation(c: &mut Criterion) {
    c.bench_function("vec_creation_1000", |b| {
        b.iter(|| {
            let mut v = Vec::new();
            for i in 0..1000 {
                v.push(black_box(i));
            }
            black_box(v)
        });
    });
}

// =============================================================================
// Parameterized Benchmarks
// =============================================================================

fn bench_vector_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("vector_creation");

    for size in [10, 100, 1_000, 10_000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            size,
            |b, &size| {
                b.iter(|| {
                    let mut v = Vec::with_capacity(size);
                    for i in 0..size {
                        v.push(black_box(i));
                    }
                    black_box(v)
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// Iteration Benchmarks
// =============================================================================

fn bench_iteration_methods(c: &mut Criterion) {
    let data: Vec<i32> = (0..10_000).collect();

    let mut group = c.benchmark_group("iteration");

    group.bench_function("for_loop", |b| {
        b.iter(|| {
            let mut sum = 0;
            for i in 0..data.len() {
                sum += black_box(data[i]);
            }
            black_box(sum)
        });
    });

    group.bench_function("iterator", |b| {
        b.iter(|| {
            let sum: i32 = data.iter().copied().sum();
            black_box(sum)
        });
    });

    group.bench_function("par_iterator", |b| {
        use rayon::prelude::*;
        b.iter(|| {
            let sum: i32 = data.par_iter().copied().sum();
            black_box(sum)
        });
    });

    group.finish();
}

// =============================================================================
// Memory Allocation Benchmarks
// =============================================================================

fn bench_allocations(c: &mut Criterion) {
    let mut group = c.benchmark_group("allocations");

    group.bench_function("vec_push", |b| {
        b.iter(|| {
            let mut v = Vec::new();
            for i in 0..1000 {
                v.push(black_box(i));
            }
            black_box(v)
        });
    });

    group.bench_function("vec_with_capacity", |b| {
        b.iter(|| {
            let mut v = Vec::with_capacity(1000);
            for i in 0..1000 {
                v.push(black_box(i));
            }
            black_box(v)
        });
    });

    group.bench_function("vec_from_iter", |b| {
        b.iter(|| {
            let v: Vec<_> = (0..1000).collect();
            black_box(v)
        });
    });

    group.finish();
}

// =============================================================================
// Throughput Benchmarks
// =============================================================================

fn bench_data_processing(c: &mut Criterion) {
    use criterion::Throughput;

    let data: Vec<u8> = vec![42; 1024 * 1024]; // 1 MB

    let mut group = c.benchmark_group("data_processing");
    group.throughput(Throughput::Bytes(data.len() as u64));

    group.bench_function("checksum", |b| {
        b.iter(|| {
            let sum: u64 = data.iter().map(|&x| x as u64).sum();
            black_box(sum)
        });
    });

    group.bench_function("checksum_chunks", |b| {
        b.iter(|| {
            let sum: u64 = data
                .chunks(1024)
                .map(|chunk| chunk.iter().map(|&x| x as u64).sum::<u64>())
                .sum();
            black_box(sum)
        });
    });

    group.finish();
}

// =============================================================================
// Mock Component Benchmarks (for demonstration)
// =============================================================================

#[derive(Clone, Copy)]
struct MockPosition {
    x: f32,
    y: f32,
    z: f32,
}

impl MockPosition {
    fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    fn distance_to(&self, other: &Self) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

fn bench_position_operations(c: &mut Criterion) {
    let pos1 = MockPosition::new(1.0, 2.0, 3.0);
    let pos2 = MockPosition::new(4.0, 5.0, 6.0);

    let mut group = c.benchmark_group("position_operations");

    group.bench_function("distance_single", |b| {
        b.iter(|| {
            black_box(pos1.distance_to(&pos2))
        });
    });

    group.bench_function("distance_batch_1000", |b| {
        let positions: Vec<_> = (0..1000)
            .map(|i| MockPosition::new(i as f32, i as f32, i as f32))
            .collect();

        b.iter(|| {
            let mut total_distance = 0.0;
            for i in 0..positions.len() - 1 {
                total_distance += positions[i].distance_to(&positions[i + 1]);
            }
            black_box(total_distance)
        });
    });

    group.finish();
}

// =============================================================================
// Benchmark Groups
// =============================================================================

criterion_group!(
    basic_benches,
    bench_simple_operation,
    bench_vector_creation,
);

criterion_group!(
    parameterized_benches,
    bench_vector_sizes,
    bench_iteration_methods,
);

criterion_group!(
    allocation_benches,
    bench_allocations,
);

criterion_group!(
    throughput_benches,
    bench_data_processing,
);

criterion_group!(
    component_benches,
    bench_position_operations,
);

criterion_main!(
    basic_benches,
    parameterized_benches,
    allocation_benches,
    throughput_benches,
    component_benches,
);
