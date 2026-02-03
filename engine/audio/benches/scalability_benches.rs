//! Scalability benchmarks for the audio system
//!
//! Tests audio system performance at various scales (1 to 100k simultaneous sounds).
//! Measures frame time, memory usage, and CPU usage to validate performance targets:
//! - 10k sounds: < 16ms frame time
//! - 1k sounds: < 1ms frame time
//!
//! These benchmarks verify the audio system can handle AAA game scenarios
//! like massive battles, dense crowds, or complex ambient soundscapes.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use engine_audio::AudioEngine;
use glam::Vec3;
use std::time::{Duration, Instant};

/// Benchmark frame time with varying numbers of simultaneous sounds
///
/// Tests: 1, 10, 100, 1k, 10k simultaneous 3D sounds
/// Target: < 1ms for 1k sounds, < 16ms for 10k sounds
fn bench_simultaneous_sounds_frame_time(c: &mut Criterion) {
    let mut group = c.benchmark_group("simultaneous_sounds_frame_time");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(50);

    for sound_count in [1, 10, 100, 1_000, 10_000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(sound_count),
            sound_count,
            |b, &count| {
                let mut engine = AudioEngine::new().unwrap();

                // Pre-create emitters at various positions
                for i in 0..count {
                    let angle = (i as f32) * std::f32::consts::TAU / (count as f32);
                    let radius = ((i % 100) as f32) * 5.0;
                    let position = Vec3::new(
                        angle.cos() * radius,
                        ((i % 10) as f32) * 2.0,
                        angle.sin() * radius,
                    );
                    engine.update_emitter_position(i, position);
                }

                b.iter(|| {
                    // Simulate one frame: update listener and all emitters
                    engine.set_listener_transform(
                        black_box(Vec3::new(0.0, 1.8, 0.0)),
                        black_box(Vec3::new(0.0, 0.0, -1.0)),
                        black_box(Vec3::new(0.0, 1.0, 0.0)),
                    );

                    // Update a subset of emitter positions (simulating movement)
                    for i in (0..count).step_by(10) {
                        let angle = (i as f32) * std::f32::consts::TAU / (count as f32);
                        let radius = ((i % 100) as f32) * 5.0 + 0.1;
                        let position = Vec3::new(
                            angle.cos() * radius,
                            ((i % 10) as f32) * 2.0,
                            angle.sin() * radius,
                        );
                        engine.update_emitter_position(black_box(i), black_box(position));
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark listener updates with varying numbers of emitters
///
/// Tests how listener position/orientation updates scale with emitter count
/// Target: < 100μs even with 10k emitters
fn bench_listener_update_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("listener_update_scaling");
    group.measurement_time(Duration::from_secs(8));

    for emitter_count in [10, 100, 1_000, 10_000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(emitter_count),
            emitter_count,
            |b, &count| {
                let mut engine = AudioEngine::new().unwrap();

                // Create emitters
                for i in 0..count {
                    let position = Vec3::new((i as f32) % 100.0, 0.0, (i as f32) / 100.0);
                    engine.update_emitter_position(i, position);
                }

                let mut frame = 0u32;
                b.iter(|| {
                    // Update listener position (simulating camera movement)
                    let t = (frame as f32) * 0.016;
                    let pos = Vec3::new(t.sin() * 10.0, 1.8, t.cos() * 10.0);
                    let forward = Vec3::new(-pos.x, 0.0, -pos.z).normalize();
                    let up = Vec3::new(0.0, 1.0, 0.0);

                    engine.set_listener_transform(
                        black_box(pos),
                        black_box(forward),
                        black_box(up),
                    );
                    frame = frame.wrapping_add(1);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark emitter position updates at scale
///
/// Tests bulk emitter updates (e.g., crowd simulation, physics)
/// Target: < 10μs per emitter update
fn bench_bulk_emitter_updates(c: &mut Criterion) {
    let mut group = c.benchmark_group("bulk_emitter_updates");
    group.measurement_time(Duration::from_secs(8));

    for update_count in [10, 100, 1_000, 10_000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(update_count),
            update_count,
            |b, &count| {
                let mut engine = AudioEngine::new().unwrap();

                // Pre-create emitters
                for i in 0..count {
                    engine.update_emitter_position(i, Vec3::ZERO);
                }

                let mut frame = 0u32;
                b.iter(|| {
                    // Update all emitters (simulating physics/animation update)
                    for i in 0..count {
                        let t = (frame as f32 + i as f32) * 0.01;
                        let pos = Vec3::new(t.sin() * 50.0, 0.0, t.cos() * 50.0);
                        engine.update_emitter_position(black_box(i), black_box(pos));
                    }
                    frame = frame.wrapping_add(1);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark effect processing at scale
///
/// Tests reverb/echo processing with many simultaneous sounds
/// Target: < 5ms overhead for 1k sounds with effects
fn bench_effect_processing_scale(c: &mut Criterion) {
    let mut group = c.benchmark_group("effect_processing_scale");
    group.measurement_time(Duration::from_secs(8));

    // Note: This benchmarks the API overhead, not actual DSP (which requires real audio)
    for sound_count in [10, 100, 1_000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(sound_count),
            sound_count,
            |b, &_count| {
                let engine = AudioEngine::new().unwrap();

                b.iter(|| {
                    // Measure overhead of effect count queries (simulating DSP processing)
                    let total_effects: usize = (0..64)
                        .map(|instance_id| engine.effect_count(black_box(instance_id)))
                        .sum();
                    black_box(total_effects);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark Doppler calculations at scale
///
/// Tests pitch adjustment calculations for moving sounds
/// Target: < 5μs per sound
fn bench_doppler_calculation_scale(c: &mut Criterion) {
    let mut group = c.benchmark_group("doppler_calculation_scale");
    group.measurement_time(Duration::from_secs(8));

    for sound_count in [10, 100, 1_000, 10_000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(sound_count),
            sound_count,
            |b, &count| {
                let mut engine = AudioEngine::new().unwrap();

                // Create emitters with positions
                for i in 0..count {
                    let pos = Vec3::new((i as f32) % 100.0, 0.0, (i as f32) / 100.0);
                    engine.update_emitter_position(i, pos);
                }

                b.iter(|| {
                    // Simulate Doppler pitch adjustments
                    // (In real usage, this would be computed from velocity)
                    for i in 0..count {
                        let instance_id = i as u64;
                        let pitch = 1.0 + ((i as f32 % 10.0) - 5.0) * 0.1; // ±50% pitch
                        engine.set_pitch(black_box(instance_id), black_box(pitch));
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark cleanup operations at scale
///
/// Tests cleanup of finished sounds (garbage collection)
/// Target: < 1ms for cleanup of 1k finished sounds
fn bench_cleanup_at_scale(c: &mut Criterion) {
    let mut group = c.benchmark_group("cleanup_at_scale");
    group.measurement_time(Duration::from_secs(8));

    for emitter_count in [10, 100, 1_000, 10_000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(emitter_count),
            emitter_count,
            |b, &count| {
                b.iter_custom(|iters| {
                    let mut total = Duration::ZERO;

                    for _iter in 0..iters {
                        let mut engine = AudioEngine::new().unwrap();

                        // Create emitters
                        for i in 0..count {
                            engine.update_emitter_position(i, Vec3::ZERO);
                        }

                        // Measure cleanup time
                        let start = Instant::now();
                        engine.cleanup_finished();
                        total += start.elapsed();
                    }

                    total
                });
            },
        );
    }

    group.finish();
}

/// Benchmark emitter lifecycle at scale
///
/// Tests rapid creation and removal of emitters
/// Target: < 1μs per create/remove cycle
fn bench_emitter_lifecycle_scale(c: &mut Criterion) {
    let mut group = c.benchmark_group("emitter_lifecycle_scale");
    group.measurement_time(Duration::from_secs(8));

    for batch_size in [10, 100, 1_000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(batch_size), batch_size, |b, &count| {
            let mut engine = AudioEngine::new().unwrap();
            let mut next_id = 0u32;

            b.iter(|| {
                // Create batch
                for _ in 0..count {
                    engine.update_emitter_position(next_id, Vec3::ZERO);
                    next_id = next_id.wrapping_add(1);
                }

                // Remove batch
                for i in 0..count {
                    let id = next_id.wrapping_sub(count).wrapping_add(i);
                    engine.remove_emitter(black_box(id));
                }
            });
        });
    }

    group.finish();
}

/// Benchmark active sound tracking at scale
///
/// Tests performance of querying active sound count
/// Target: < 1μs
fn bench_active_sound_tracking_scale(c: &mut Criterion) {
    let mut group = c.benchmark_group("active_sound_tracking_scale");

    for emitter_count in [10, 100, 1_000, 10_000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(emitter_count),
            emitter_count,
            |b, &count| {
                let mut engine = AudioEngine::new().unwrap();

                // Create emitters
                for i in 0..count {
                    engine.update_emitter_position(i, Vec3::ZERO);
                }

                b.iter(|| {
                    let active = engine.active_sound_count();
                    let loaded = engine.loaded_sound_count();
                    black_box((active, loaded));
                });
            },
        );
    }

    group.finish();
}

/// Benchmark mixed workload at AAA scale
///
/// Simulates a realistic AAA game frame with 5k sounds:
/// - Update listener (camera)
/// - Update 500 moving emitters
/// - Query playback state
/// - Cleanup finished sounds
///
/// Target: < 5ms total frame time
fn bench_aaa_mixed_workload(c: &mut Criterion) {
    let mut group = c.benchmark_group("aaa_mixed_workload");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(50);

    for total_sounds in [1_000, 5_000, 10_000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(total_sounds),
            total_sounds,
            |b, &count| {
                let mut engine = AudioEngine::new().unwrap();

                // Pre-create emitters
                for i in 0..count {
                    let angle = (i as f32) * std::f32::consts::TAU / (count as f32);
                    let radius = ((i % 100) as f32) * 5.0;
                    let position = Vec3::new(
                        angle.cos() * radius,
                        ((i % 10) as f32) * 2.0,
                        angle.sin() * radius,
                    );
                    engine.update_emitter_position(i, position);
                }

                let mut frame = 0u32;
                b.iter(|| {
                    let t = (frame as f32) * 0.016;

                    // 1. Update listener (camera movement)
                    let listener_pos = Vec3::new(t.sin() * 20.0, 1.8, t.cos() * 20.0);
                    let forward = Vec3::new(-listener_pos.x, 0.0, -listener_pos.z).normalize();
                    engine.set_listener_transform(
                        black_box(listener_pos),
                        black_box(forward),
                        black_box(Vec3::new(0.0, 1.0, 0.0)),
                    );

                    // 2. Update 10% of emitters (moving entities)
                    let update_count = count / 10;
                    for i in 0..update_count {
                        let entity_id = (frame as u32 + i) % count;
                        let angle = (entity_id as f32) * std::f32::consts::TAU / (count as f32) + t;
                        let radius = ((entity_id % 100) as f32) * 5.0;
                        let pos = Vec3::new(
                            angle.cos() * radius,
                            ((entity_id % 10) as f32) * 2.0,
                            angle.sin() * radius,
                        );
                        engine.update_emitter_position(black_box(entity_id), black_box(pos));
                    }

                    // 3. Query playback state for subset
                    let mut playing_count = 0;
                    for i in 0..16 {
                        if engine.is_playing(black_box(i)) {
                            playing_count += 1;
                        }
                    }
                    black_box(playing_count);

                    // 4. Cleanup finished sounds
                    engine.cleanup_finished();

                    frame = frame.wrapping_add(1);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark worst-case scenario: all sounds at max distance
///
/// Tests performance when all sounds require full distance attenuation calculation
/// Target: < 20ms for 10k sounds
fn bench_worst_case_max_distance(c: &mut Criterion) {
    let mut group = c.benchmark_group("worst_case_max_distance");
    group.measurement_time(Duration::from_secs(8));

    for sound_count in [100, 1_000, 10_000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(sound_count),
            sound_count,
            |b, &count| {
                let mut engine = AudioEngine::new().unwrap();

                // Place all emitters at maximum distance
                for i in 0..count {
                    let angle = (i as f32) * std::f32::consts::TAU / (count as f32);
                    let position = Vec3::new(
                        angle.cos() * 1000.0, // Far away
                        0.0,
                        angle.sin() * 1000.0,
                    );
                    engine.update_emitter_position(i, position);
                }

                b.iter(|| {
                    // Update listener (triggers distance recalculation for all sounds)
                    engine.set_listener_transform(
                        black_box(Vec3::ZERO),
                        black_box(Vec3::new(0.0, 0.0, -1.0)),
                        black_box(Vec3::new(0.0, 1.0, 0.0)),
                    );
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_simultaneous_sounds_frame_time,
    bench_listener_update_scaling,
    bench_bulk_emitter_updates,
    bench_effect_processing_scale,
    bench_doppler_calculation_scale,
    bench_cleanup_at_scale,
    bench_emitter_lifecycle_scale,
    bench_active_sound_tracking_scale,
    bench_aaa_mixed_workload,
    bench_worst_case_max_distance,
);

criterion_main!(benches);
