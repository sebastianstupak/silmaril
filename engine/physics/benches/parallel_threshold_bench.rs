//! Benchmark to find optimal PARALLEL_THRESHOLD for physics integration.
//!
//! This benchmark tests different parallel thresholds to determine the point where
//! parallel processing overhead is offset by the benefits of multi-threading.
//!
//! Test configurations:
//! - Thresholds: 1K, 2K, 3K, 5K, 10K
//! - Entity counts: 500, 1K, 2K, 5K, 10K, 20K
//! - Target: 10-30% improvement for 1K-10K entity range

use criterion::{
    black_box, criterion_group, criterion_main, BenchmarkId, Criterion, PlotConfiguration,
    Throughput,
};
use engine_core::math::Transform;
use engine_math::Vec3;
use engine_physics::systems::integration_simd::{process_parallel, process_sequential};

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

/// Benchmark different thresholds at various entity counts.
fn bench_threshold_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("threshold_comparison");
    group
        .plot_config(PlotConfiguration::default().summary_scale(criterion::AxisScale::Logarithmic));

    let thresholds = vec![1_000, 2_000, 3_000, 5_000, 10_000];
    let entity_counts = vec![500, 1_000, 2_000, 5_000, 10_000, 20_000];

    for entity_count in entity_counts.iter() {
        group.throughput(Throughput::Elements(*entity_count as u64));

        for threshold in thresholds.iter() {
            let id = BenchmarkId::new(
                format!("threshold_{}", threshold),
                format!("entities_{}", entity_count),
            );

            group.bench_with_input(id, &(entity_count, threshold), |b, &(&count, &thresh)| {
                let mut transforms = vec![Transform::identity(); count];
                let velocities = vec![Vec3::new(1.0, 2.0, 3.0); count];

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

/// Benchmark sequential vs parallel at specific entity counts.
fn bench_sequential_vs_parallel_detailed(c: &mut Criterion) {
    let mut group = c.benchmark_group("sequential_vs_parallel_detailed");

    let entity_counts = vec![500, 1_000, 2_000, 3_000, 5_000, 7_500, 10_000, 15_000, 20_000];

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

/// Benchmark to find the crossover point where parallel becomes faster.
fn bench_crossover_point(c: &mut Criterion) {
    let mut group = c.benchmark_group("crossover_point");

    // Fine-grained test around expected crossover point
    let entity_counts =
        vec![800, 900, 1_000, 1_200, 1_500, 1_800, 2_000, 2_500, 3_000, 3_500, 4_000, 5_000];

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

/// Benchmark parallel overhead at small entity counts.
fn bench_parallel_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_overhead");

    // Test very small counts to measure pure overhead
    let entity_counts = vec![10, 50, 100, 250, 500, 750, 1_000];

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

/// Benchmark optimal threshold candidates at target range.
fn bench_optimal_threshold_candidates(c: &mut Criterion) {
    let mut group = c.benchmark_group("optimal_threshold_candidates");

    // Test the most promising threshold values in the target range
    let thresholds = vec![1_500, 2_000, 2_500, 3_000, 3_500, 4_000];
    let entity_counts = vec![1_000, 2_000, 3_000, 5_000, 7_500, 10_000];

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

criterion_group!(
    benches,
    bench_threshold_comparison,
    bench_sequential_vs_parallel_detailed,
    bench_crossover_point,
    bench_parallel_overhead,
    bench_optimal_threshold_candidates
);

criterion_main!(benches);
