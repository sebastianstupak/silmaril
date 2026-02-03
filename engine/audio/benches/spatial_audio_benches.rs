//! Benchmarks for spatial audio performance
//!
//! Single-crate benchmarks testing AudioEngine 3D audio capabilities.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use engine_audio::AudioEngine;
use glam::Vec3;

fn bench_listener_transform_update(c: &mut Criterion) {
    let mut engine = AudioEngine::new().unwrap();

    c.bench_function("listener_transform_update", |b| {
        b.iter(|| {
            engine.set_listener_transform(
                black_box(Vec3::new(1.0, 2.0, 3.0)),
                black_box(Vec3::new(0.0, 0.0, -1.0)),
                black_box(Vec3::new(0.0, 1.0, 0.0)),
            );
        });
    });
}

fn bench_emitter_position_update(c: &mut Criterion) {
    let mut engine = AudioEngine::new().unwrap();

    // Create emitters by playing sounds
    for i in 0..100 {
        let _ = engine.play_3d(i, "test", Vec3::new(i as f32, 0.0, 0.0), 1.0, false, 100.0);
    }

    let mut position = Vec3::new(5.0, 0.0, 0.0);

    c.bench_function("emitter_position_update", |b| {
        b.iter(|| {
            position.x += 0.1;
            engine.update_emitter_position(black_box(42), black_box(position));
        });
    });
}

fn bench_emitter_creation_removal(c: &mut Criterion) {
    let mut group = c.benchmark_group("emitter_lifecycle");

    for count in [10, 50, 100].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            b.iter(|| {
                let mut engine = AudioEngine::new().unwrap();

                // Create emitters
                for i in 0..count {
                    engine.update_emitter_position(i, Vec3::new(i as f32, 0.0, 0.0));
                }

                // Remove emitters
                for i in 0..count {
                    engine.remove_emitter(black_box(i));
                }
            });
        });
    }

    group.finish();
}

fn bench_spatial_distance_calculations(c: &mut Criterion) {
    let mut group = c.benchmark_group("spatial_distance");

    // Test different listener-emitter distances
    for distance in [10.0, 50.0, 100.0, 500.0].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(*distance as u32),
            distance,
            |b, &distance| {
                let mut engine = AudioEngine::new().unwrap();

                b.iter(|| {
                    engine.set_listener_transform(
                        black_box(Vec3::ZERO),
                        black_box(Vec3::new(0.0, 0.0, -1.0)),
                        black_box(Vec3::new(0.0, 1.0, 0.0)),
                    );

                    engine.update_emitter_position(
                        black_box(1),
                        black_box(Vec3::new(distance, 0.0, 0.0)),
                    );
                });
            },
        );
    }

    group.finish();
}

fn bench_cleanup_finished(c: &mut Criterion) {
    let mut engine = AudioEngine::new().unwrap();

    c.bench_function("cleanup_finished", |b| {
        b.iter(|| {
            engine.cleanup_finished();
        });
    });
}

fn bench_active_sound_count(c: &mut Criterion) {
    let mut engine = AudioEngine::new().unwrap();

    c.bench_function("active_sound_count", |b| {
        b.iter(|| {
            let count = engine.active_sound_count();
            black_box(count);
        });
    });
}

fn bench_is_playing_check(c: &mut Criterion) {
    let engine = AudioEngine::new().unwrap();

    c.bench_function("is_playing_check", |b| {
        b.iter(|| {
            let playing = engine.is_playing(black_box(42));
            black_box(playing);
        });
    });
}

fn bench_many_emitter_updates(c: &mut Criterion) {
    let mut group = c.benchmark_group("many_emitter_updates");

    for count in [10, 50, 100, 500].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            let mut engine = AudioEngine::new().unwrap();

            // Create emitters
            for i in 0..count {
                engine.update_emitter_position(i, Vec3::new(i as f32, 0.0, 0.0));
            }

            b.iter(|| {
                // Update all emitter positions
                for i in 0..count {
                    let new_pos = Vec3::new((i as f32) + 0.1, 0.0, 0.0);
                    engine.update_emitter_position(black_box(i), black_box(new_pos));
                }
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_listener_transform_update,
    bench_emitter_position_update,
    bench_emitter_creation_removal,
    bench_spatial_distance_calculations,
    bench_cleanup_finished,
    bench_active_sound_count,
    bench_is_playing_check,
    bench_many_emitter_updates,
);

criterion_main!(benches);
