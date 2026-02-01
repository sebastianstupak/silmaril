//! Standalone benchmark to find optimal PARALLEL_THRESHOLD.
//!
//! This benchmark is self-contained and doesn't depend on other engine components
//! that may have compilation issues.

use criterion::{
    black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput,
};
use rayon::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

impl Vec3 {
    fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    fn mul_scalar(self, scalar: f32) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
        }
    }

    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Transform {
    position: Vec3,
}

impl Transform {
    fn identity() -> Self {
        Self {
            position: Vec3::new(0.0, 0.0, 0.0),
        }
    }
}

/// Process entities sequentially with SIMD-style batching.
fn process_sequential(transforms: &mut [Transform], velocities: &[Vec3], dt: f32) {
    let count = transforms.len();
    let mut i = 0;

    const BATCH_SIZE_8: usize = 8;
    const BATCH_SIZE_4: usize = 4;

    // Process batches of 8
    while i + BATCH_SIZE_8 <= count {
        for j in 0..BATCH_SIZE_8 {
            let vel_scaled = velocities[i + j].mul_scalar(dt);
            transforms[i + j].position = transforms[i + j].position.add(vel_scaled);
        }
        i += BATCH_SIZE_8;
    }

    // Process batches of 4
    while i + BATCH_SIZE_4 <= count {
        for j in 0..BATCH_SIZE_4 {
            let vel_scaled = velocities[i + j].mul_scalar(dt);
            transforms[i + j].position = transforms[i + j].position.add(vel_scaled);
        }
        i += BATCH_SIZE_4;
    }

    // Process remainder
    while i < count {
        let vel_scaled = velocities[i].mul_scalar(dt);
        transforms[i].position = transforms[i].position.add(vel_scaled);
        i += 1;
    }
}

/// Process entities in parallel using rayon.
fn process_parallel(transforms: &mut [Transform], velocities: &[Vec3], dt: f32) {
    const CHUNK_SIZE: usize = 512;

    transforms
        .par_chunks_mut(CHUNK_SIZE)
        .zip(velocities.par_chunks(CHUNK_SIZE))
        .for_each(|(transform_chunk, velocity_chunk)| {
            process_sequential(transform_chunk, velocity_chunk, dt);
        });
}

/// Process with configurable threshold.
fn process_with_threshold(
    transforms: &mut [Transform],
    velocities: &[Vec3],
    dt: f32,
    threshold: usize,
) {
    if transforms.len() >= threshold {
        process_parallel(transforms, velocities, dt);
    } else {
        process_sequential(transforms, velocities, dt);
    }
}

/// Benchmark to find crossover point.
fn bench_crossover_point(c: &mut Criterion) {
    let mut group = c.benchmark_group("crossover_point");

    let entity_counts = vec![
        500, 750, 1_000, 1_250, 1_500, 1_750, 2_000, 2_500, 3_000, 4_000, 5_000,
    ];

    for entity_count in entity_counts.iter() {
        let count = *entity_count;
        group.throughput(Throughput::Elements(count as u64));

        group.bench_function(BenchmarkId::new("sequential", count), |b| {
            let mut transforms = vec![Transform::identity(); count];
            let velocities = vec![Vec3::new(1.0, 2.0, 3.0); count];
            b.iter(|| {
                process_sequential(
                    black_box(&mut transforms),
                    black_box(&velocities),
                    black_box(0.016),
                );
            });
        });

        group.bench_function(BenchmarkId::new("parallel", count), |b| {
            let mut transforms = vec![Transform::identity(); count];
            let velocities = vec![Vec3::new(1.0, 2.0, 3.0); count];
            b.iter(|| {
                process_parallel(
                    black_box(&mut transforms),
                    black_box(&velocities),
                    black_box(0.016),
                );
            });
        });
    }

    group.finish();
}

/// Benchmark different thresholds.
fn bench_threshold_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("threshold_comparison");

    let thresholds = vec![1_000, 1_500, 2_000, 2_500, 3_000, 4_000, 5_000];
    let entity_counts = vec![500, 1_000, 2_000, 3_000, 5_000, 7_500, 10_000, 20_000];

    for entity_count in entity_counts.iter() {
        let count = *entity_count;
        group.throughput(Throughput::Elements(count as u64));

        for threshold in thresholds.iter() {
            let id = BenchmarkId::new(format!("threshold_{}", threshold), count);

            group.bench_with_input(id, &(count, threshold), |b, &(cnt, &thresh)| {
                let mut transforms = vec![Transform::identity(); cnt];
                let velocities = vec![Vec3::new(1.0, 2.0, 3.0); cnt];

                b.iter(|| {
                    process_with_threshold(
                        black_box(&mut transforms),
                        black_box(&velocities),
                        black_box(0.016),
                        thresh,
                    );
                });
            });
        }
    }

    group.finish();
}

/// Benchmark parallel overhead at small counts.
fn bench_parallel_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_overhead");

    let entity_counts = vec![100, 250, 500, 750, 1_000, 1_500, 2_000];

    for entity_count in entity_counts.iter() {
        let count = *entity_count;
        group.throughput(Throughput::Elements(count as u64));

        group.bench_function(BenchmarkId::new("sequential", count), |b| {
            let mut transforms = vec![Transform::identity(); count];
            let velocities = vec![Vec3::new(1.0, 2.0, 3.0); count];
            b.iter(|| {
                process_sequential(
                    black_box(&mut transforms),
                    black_box(&velocities),
                    black_box(0.016),
                );
            });
        });

        group.bench_function(BenchmarkId::new("parallel", count), |b| {
            let mut transforms = vec![Transform::identity(); count];
            let velocities = vec![Vec3::new(1.0, 2.0, 3.0); count];
            b.iter(|| {
                process_parallel(
                    black_box(&mut transforms),
                    black_box(&velocities),
                    black_box(0.016),
                );
            });
        });
    }

    group.finish();
}

/// Detailed analysis in target range (1K-10K entities).
fn bench_target_range_detailed(c: &mut Criterion) {
    let mut group = c.benchmark_group("target_range_detailed");

    let thresholds = vec![1_500, 2_000, 2_500, 3_000];
    let entity_counts = vec![1_000, 2_000, 3_000, 5_000, 7_500, 10_000];

    for entity_count in entity_counts.iter() {
        let count = *entity_count;
        group.throughput(Throughput::Elements(count as u64));

        // Baseline: always sequential
        group.bench_function(BenchmarkId::new("baseline_sequential", count), |b| {
            let mut transforms = vec![Transform::identity(); count];
            let velocities = vec![Vec3::new(1.0, 2.0, 3.0); count];
            b.iter(|| {
                process_sequential(
                    black_box(&mut transforms),
                    black_box(&velocities),
                    black_box(0.016),
                );
            });
        });

        // Baseline: always parallel
        group.bench_function(BenchmarkId::new("baseline_parallel", count), |b| {
            let mut transforms = vec![Transform::identity(); count];
            let velocities = vec![Vec3::new(1.0, 2.0, 3.0); count];
            b.iter(|| {
                process_parallel(
                    black_box(&mut transforms),
                    black_box(&velocities),
                    black_box(0.016),
                );
            });
        });

        // Test different thresholds
        for threshold in thresholds.iter() {
            let id = BenchmarkId::new(format!("threshold_{}", threshold), count);

            group.bench_with_input(id, &(count, threshold), |b, &(cnt, &thresh)| {
                let mut transforms = vec![Transform::identity(); cnt];
                let velocities = vec![Vec3::new(1.0, 2.0, 3.0); cnt];

                b.iter(|| {
                    process_with_threshold(
                        black_box(&mut transforms),
                        black_box(&velocities),
                        black_box(0.016),
                        thresh,
                    );
                });
            });
        }
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_crossover_point,
    bench_threshold_comparison,
    bench_parallel_overhead,
    bench_target_range_detailed
);

criterion_main!(benches);
