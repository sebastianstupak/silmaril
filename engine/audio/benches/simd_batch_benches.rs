//! SIMD batch operation benchmarks
//!
//! These benchmarks compare batch operations vs single operations to
//! demonstrate the performance benefits of SIMD-optimized code.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_audio::simd_batch::*;
use engine_audio::DopplerCalculator;
use glam::Vec3;
use std::time::Duration;

// ============================================================================
// BATCH VS SINGLE OPERATION COMPARISONS
// ============================================================================

fn bench_batch_vs_single_velocity(c: &mut Criterion) {
    let mut group = c.benchmark_group("velocity_batch_vs_single");
    group.measurement_time(Duration::from_secs(5));

    for count in [10, 50, 100, 500, 1000].iter() {
        group.throughput(Throughput::Elements(*count as u64));

        let old_positions: Vec<Vec3> = (0..*count).map(|i| Vec3::new(i as f32, 0.0, 0.0)).collect();
        let new_positions: Vec<Vec3> =
            (0..*count).map(|i| Vec3::new((i + 1) as f32, 0.0, 0.0)).collect();
        let delta_time = 0.016;

        // Batch operation
        group.bench_with_input(BenchmarkId::new("batch", count), count, |b, _| {
            b.iter(|| {
                let velocities = batch_calculate_velocities(
                    black_box(&old_positions),
                    black_box(&new_positions),
                    black_box(delta_time),
                );
                black_box(velocities);
            });
        });

        // Single operations
        group.bench_with_input(BenchmarkId::new("single", count), count, |b, _| {
            b.iter(|| {
                let mut velocities = Vec::with_capacity(*count);
                for i in 0..*count {
                    let vel = DopplerCalculator::calculate_velocity(
                        black_box(old_positions[i]),
                        black_box(new_positions[i]),
                        black_box(delta_time),
                    );
                    velocities.push(vel);
                }
                black_box(velocities);
            });
        });
    }

    group.finish();
}

fn bench_batch_distance_calculations(c: &mut Criterion) {
    let mut group = c.benchmark_group("distance_batch_operations");
    group.measurement_time(Duration::from_secs(5));

    for count in [10, 50, 100, 500, 1000].iter() {
        group.throughput(Throughput::Elements(*count as u64));

        let listener_pos = Vec3::ZERO;
        let emitter_positions: Vec<Vec3> = (0..*count)
            .map(|i| {
                let angle = (i as f32) * std::f32::consts::PI * 2.0 / *count as f32;
                Vec3::new(angle.cos() * 50.0, 0.0, angle.sin() * 50.0)
            })
            .collect();

        group.bench_with_input(BenchmarkId::new("batch_distance_sq", count), count, |b, _| {
            b.iter(|| {
                let distances = batch_calculate_distances_sq(
                    black_box(listener_pos),
                    black_box(&emitter_positions),
                );
                black_box(distances);
            });
        });

        // Compare with manual loop
        group.bench_with_input(BenchmarkId::new("manual_distance_sq", count), count, |b, _| {
            b.iter(|| {
                let mut distances = Vec::with_capacity(*count);
                for &pos in &emitter_positions {
                    let delta = pos - listener_pos;
                    distances.push(delta.length_squared());
                }
                black_box(distances);
            });
        });
    }

    group.finish();
}

fn bench_batch_direction_calculations(c: &mut Criterion) {
    let mut group = c.benchmark_group("direction_batch_operations");
    group.measurement_time(Duration::from_secs(5));

    for count in [10, 50, 100, 500, 1000].iter() {
        group.throughput(Throughput::Elements(*count as u64));

        let from_pos = Vec3::ZERO;
        let to_positions: Vec<Vec3> = (0..*count)
            .map(|i| {
                let angle = (i as f32) * std::f32::consts::PI * 2.0 / *count as f32;
                Vec3::new(angle.cos() * 50.0, 0.0, angle.sin() * 50.0)
            })
            .collect();

        group.bench_with_input(BenchmarkId::new("batch_directions", count), count, |b, _| {
            b.iter(|| {
                let directions =
                    batch_calculate_directions(black_box(from_pos), black_box(&to_positions));
                black_box(directions);
            });
        });
    }

    group.finish();
}

fn bench_batch_attenuation_calculations(c: &mut Criterion) {
    let mut group = c.benchmark_group("attenuation_batch_operations");
    group.measurement_time(Duration::from_secs(5));

    for count in [10, 50, 100, 500, 1000].iter() {
        group.throughput(Throughput::Elements(*count as u64));

        let distances_sq: Vec<f32> = (0..*count)
            .map(|i| {
                let dist = (i as f32) * 100.0 / *count as f32;
                dist * dist
            })
            .collect();
        let max_distance = 100.0;

        group.bench_with_input(BenchmarkId::new("batch_attenuation", count), count, |b, _| {
            b.iter(|| {
                let attenuations =
                    batch_calculate_attenuation(black_box(&distances_sq), black_box(max_distance));
                black_box(attenuations);
            });
        });
    }

    group.finish();
}

fn bench_batch_radial_velocity(c: &mut Criterion) {
    let mut group = c.benchmark_group("radial_velocity_batch_operations");
    group.measurement_time(Duration::from_secs(5));

    for count in [10, 50, 100, 500, 1000].iter() {
        group.throughput(Throughput::Elements(*count as u64));

        let velocities: Vec<Vec3> = (0..*count)
            .map(|i| {
                let angle = (i as f32) * std::f32::consts::PI * 2.0 / *count as f32;
                Vec3::new(angle.cos() * 10.0, 0.0, angle.sin() * 10.0)
            })
            .collect();

        let directions: Vec<Vec3> = (0..*count)
            .map(|i| {
                let angle = (i as f32) * std::f32::consts::PI * 2.0 / *count as f32;
                Vec3::new(angle.cos(), 0.0, angle.sin())
            })
            .collect();

        group.bench_with_input(BenchmarkId::new("batch_radial_velocity", count), count, |b, _| {
            b.iter(|| {
                let radial_vels = batch_calculate_radial_velocities(
                    black_box(&velocities),
                    black_box(&directions),
                );
                black_box(radial_vels);
            });
        });

        // Compare with manual loop
        group.bench_with_input(BenchmarkId::new("manual_radial_velocity", count), count, |b, _| {
            b.iter(|| {
                let mut radial_vels = Vec::with_capacity(*count);
                for i in 0..*count {
                    radial_vels.push(velocities[i].dot(directions[i]));
                }
                black_box(radial_vels);
            });
        });
    }

    group.finish();
}

// ============================================================================
// FULL PIPELINE BENCHMARKS
// ============================================================================

fn bench_full_spatial_audio_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_spatial_pipeline");
    group.measurement_time(Duration::from_secs(10));

    for count in [10, 50, 100, 500].iter() {
        group.throughput(Throughput::Elements(*count as u64));

        let listener_pos = Vec3::ZERO;
        let old_positions: Vec<Vec3> = (0..*count)
            .map(|i| {
                let angle = (i as f32) * std::f32::consts::PI * 2.0 / *count as f32;
                Vec3::new(angle.cos() * 50.0, 0.0, angle.sin() * 50.0)
            })
            .collect();

        let new_positions: Vec<Vec3> =
            old_positions.iter().map(|&pos| pos + Vec3::new(0.1, 0.0, 0.0)).collect();

        let delta_time = 0.016;
        let max_distance = 100.0;

        group.bench_with_input(BenchmarkId::new("batch_pipeline", count), count, |b, _| {
            b.iter(|| {
                // Full spatial audio pipeline using batch operations
                let velocities = batch_calculate_velocities(
                    black_box(&old_positions),
                    black_box(&new_positions),
                    black_box(delta_time),
                );

                let distances_sq = batch_calculate_distances_sq(
                    black_box(listener_pos),
                    black_box(&new_positions),
                );

                let attenuations =
                    batch_calculate_attenuation(black_box(&distances_sq), black_box(max_distance));

                let directions =
                    batch_calculate_directions(black_box(listener_pos), black_box(&new_positions));

                let radial_velocities = batch_calculate_radial_velocities(
                    black_box(&velocities),
                    black_box(&directions),
                );

                black_box((attenuations, radial_velocities));
            });
        });
    }

    group.finish();
}

// ============================================================================
// CACHE EFFICIENCY BENCHMARKS
// ============================================================================

fn bench_cache_efficiency(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_efficiency");

    // Sequential access (cache-friendly)
    group.bench_function("sequential_1000_velocities", |b| {
        let old_positions: Vec<Vec3> = (0..1000).map(|i| Vec3::new(i as f32, 0.0, 0.0)).collect();
        let new_positions: Vec<Vec3> =
            (0..1000).map(|i| Vec3::new((i + 1) as f32, 0.0, 0.0)).collect();

        b.iter(|| {
            let velocities = batch_calculate_velocities(
                black_box(&old_positions),
                black_box(&new_positions),
                black_box(0.016),
            );
            black_box(velocities);
        });
    });

    group.finish();
}

// ============================================================================
// MEMORY BANDWIDTH BENCHMARKS
// ============================================================================

fn bench_memory_bandwidth(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_bandwidth");
    group.measurement_time(Duration::from_secs(10));

    for count in [100, 1000, 10000].iter() {
        group.throughput(Throughput::Bytes((*count * std::mem::size_of::<Vec3>() * 2) as u64));

        let old_positions: Vec<Vec3> = (0..*count).map(|i| Vec3::new(i as f32, 0.0, 0.0)).collect();
        let new_positions: Vec<Vec3> =
            (0..*count).map(|i| Vec3::new((i + 1) as f32, 0.0, 0.0)).collect();

        group.bench_with_input(
            BenchmarkId::new("batch_velocity_throughput", count),
            count,
            |b, _| {
                b.iter(|| {
                    let velocities = batch_calculate_velocities(
                        black_box(&old_positions),
                        black_box(&new_positions),
                        black_box(0.016),
                    );
                    black_box(velocities);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_batch_vs_single_velocity,
    bench_batch_distance_calculations,
    bench_batch_direction_calculations,
    bench_batch_attenuation_calculations,
    bench_batch_radial_velocity,
    bench_full_spatial_audio_pipeline,
    bench_cache_efficiency,
    bench_memory_bandwidth,
);

criterion_main!(benches);
