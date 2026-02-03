//! Benchmarks for Android audio backend
//!
//! These benchmarks measure performance on Android devices.
//! Run with: cargo bench --target aarch64-linux-android

#![cfg(target_os = "android")]

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use engine_audio::AudioEngine;
use glam::Vec3;
use std::path::Path;

fn bench_backend_creation(c: &mut Criterion) {
    c.bench_function("android_backend_creation", |b| {
        b.iter(|| {
            let engine = AudioEngine::new();
            black_box(engine)
        })
    });
}

fn bench_listener_update(c: &mut Criterion) {
    let mut engine = AudioEngine::new().expect("Failed to create audio engine");

    c.bench_function("android_listener_update", |b| {
        b.iter(|| {
            engine.set_listener_transform(
                black_box(Vec3::new(1.0, 2.0, 3.0)),
                black_box(Vec3::NEG_Z),
                black_box(Vec3::Y),
            )
        })
    });
}

fn bench_emitter_update(c: &mut Criterion) {
    let mut engine = AudioEngine::new().expect("Failed to create audio engine");

    c.bench_function("android_emitter_update", |b| {
        b.iter(|| {
            engine.update_emitter_position(black_box(42), black_box(Vec3::new(5.0, 0.0, 0.0)))
        })
    });
}

fn bench_many_emitters(c: &mut Criterion) {
    let mut group = c.benchmark_group("android_emitter_scaling");

    for emitter_count in [10, 50, 100, 500, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(emitter_count),
            emitter_count,
            |b, &count| {
                let mut engine = AudioEngine::new().expect("Failed to create audio engine");

                b.iter(|| {
                    for entity_id in 0..count {
                        engine.update_emitter_position(
                            black_box(entity_id as u32),
                            black_box(Vec3::new(
                                (entity_id as f32).sin() * 10.0,
                                0.0,
                                (entity_id as f32).cos() * 10.0,
                            )),
                        );
                    }
                });
            },
        );
    }

    group.finish();
}

fn bench_is_playing_check(c: &mut Criterion) {
    let engine = AudioEngine::new().expect("Failed to create audio engine");

    c.bench_function("android_is_playing_check", |b| {
        b.iter(|| {
            let playing = engine.is_playing(black_box(999));
            black_box(playing)
        })
    });
}

fn bench_cleanup_finished(c: &mut Criterion) {
    let mut engine = AudioEngine::new().expect("Failed to create audio engine");

    c.bench_function("android_cleanup_finished", |b| b.iter(|| engine.cleanup_finished()));
}

fn bench_active_sound_count(c: &mut Criterion) {
    let engine = AudioEngine::new().expect("Failed to create audio engine");

    c.bench_function("android_active_sound_count", |b| {
        b.iter(|| {
            let count = engine.active_sound_count();
            black_box(count)
        })
    });
}

// Benchmarks requiring actual audio files (ignored by default)

#[cfg(feature = "device_benchmarks")]
fn bench_load_wav(c: &mut Criterion) {
    c.bench_function("android_load_wav", |b| {
        b.iter(|| {
            let mut engine = AudioEngine::new().expect("Failed to create audio engine");
            let result = engine.load_sound("test", Path::new("/sdcard/bench_test.wav"));
            black_box(result)
        })
    });
}

#[cfg(feature = "device_benchmarks")]
fn bench_load_ogg(c: &mut Criterion) {
    c.bench_function("android_load_ogg", |b| {
        b.iter(|| {
            let mut engine = AudioEngine::new().expect("Failed to create audio engine");
            let result = engine.load_sound("test", Path::new("/sdcard/bench_test.ogg"));
            black_box(result)
        })
    });
}

#[cfg(feature = "device_benchmarks")]
fn bench_load_mp3(c: &mut Criterion) {
    c.bench_function("android_load_mp3", |b| {
        b.iter(|| {
            let mut engine = AudioEngine::new().expect("Failed to create audio engine");
            let result = engine.load_sound("test", Path::new("/sdcard/bench_test.mp3"));
            black_box(result)
        })
    });
}

#[cfg(feature = "device_benchmarks")]
fn bench_play_2d(c: &mut Criterion) {
    let mut engine = AudioEngine::new().expect("Failed to create audio engine");
    engine
        .load_sound("test", Path::new("/sdcard/bench_test.wav"))
        .expect("Failed to load test sound");

    c.bench_function("android_play_2d", |b| {
        b.iter(|| {
            let instance = engine.play_2d("test", black_box(1.0), black_box(false));
            if let Ok(id) = instance {
                engine.stop(id, None);
            }
            black_box(instance)
        })
    });
}

#[cfg(feature = "device_benchmarks")]
fn bench_play_3d(c: &mut Criterion) {
    let mut engine = AudioEngine::new().expect("Failed to create audio engine");
    engine
        .load_sound("test", Path::new("/sdcard/bench_test.wav"))
        .expect("Failed to load test sound");

    c.bench_function("android_play_3d", |b| {
        b.iter(|| {
            let instance = engine.play_3d(
                black_box(1),
                "test",
                black_box(Vec3::new(5.0, 0.0, 0.0)),
                black_box(1.0),
                black_box(false),
                black_box(100.0),
            );
            if let Ok(id) = instance {
                engine.stop(id, None);
            }
            black_box(instance)
        })
    });
}

#[cfg(feature = "device_benchmarks")]
fn bench_many_concurrent_sounds(c: &mut Criterion) {
    let mut group = c.benchmark_group("android_concurrent_sounds");

    for sound_count in [10, 50, 100, 200].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(sound_count),
            sound_count,
            |b, &count| {
                let mut engine = AudioEngine::new().expect("Failed to create audio engine");
                engine
                    .load_sound("test", Path::new("/sdcard/bench_test.wav"))
                    .expect("Failed to load test sound");

                b.iter(|| {
                    let mut instances = Vec::new();

                    for _ in 0..count {
                        if let Ok(id) = engine.play_2d("test", 0.1, false) {
                            instances.push(id);
                        }
                    }

                    // Clean up
                    for id in instances {
                        engine.stop(id, None);
                    }
                    engine.cleanup_finished();
                });
            },
        );
    }

    group.finish();
}

#[cfg(feature = "device_benchmarks")]
fn bench_3d_audio_calculation(c: &mut Criterion) {
    let mut engine = AudioEngine::new().expect("Failed to create audio engine");
    engine
        .load_sound("test", Path::new("/sdcard/bench_test.wav"))
        .expect("Failed to load test sound");

    // Create a 3D sound instance
    let instance_id = engine
        .play_3d(1, "test", Vec3::new(10.0, 0.0, 0.0), 1.0, true, 100.0)
        .expect("Failed to play 3D sound");

    c.bench_function("android_3d_audio_update", |b| {
        b.iter(|| {
            // Simulate listener movement
            engine.set_listener_transform(
                black_box(Vec3::new(1.0, 0.0, 0.0)),
                black_box(Vec3::NEG_Z),
                black_box(Vec3::Y),
            );

            // Simulate emitter movement
            engine.update_emitter_position(black_box(1), black_box(Vec3::new(11.0, 1.0, 1.0)));
        })
    });

    engine.stop(instance_id, None);
}

#[cfg(feature = "device_benchmarks")]
fn bench_streaming_playback(c: &mut Criterion) {
    let mut engine = AudioEngine::new().expect("Failed to create audio engine");

    c.bench_function("android_streaming_start", |b| {
        b.iter(|| {
            let instance = engine.play_stream(
                Path::new("/sdcard/bench_music.ogg"),
                black_box(0.5),
                black_box(false),
            );
            if let Ok(id) = instance {
                engine.stop(id, None);
            }
            black_box(instance)
        })
    });
}

// Register benchmarks
criterion_group!(
    basic_benches,
    bench_backend_creation,
    bench_listener_update,
    bench_emitter_update,
    bench_many_emitters,
    bench_is_playing_check,
    bench_cleanup_finished,
    bench_active_sound_count,
);

#[cfg(feature = "device_benchmarks")]
criterion_group!(
    device_benches,
    bench_load_wav,
    bench_load_ogg,
    bench_load_mp3,
    bench_play_2d,
    bench_play_3d,
    bench_many_concurrent_sounds,
    bench_3d_audio_calculation,
    bench_streaming_playback,
);

#[cfg(feature = "device_benchmarks")]
criterion_main!(basic_benches, device_benches);

#[cfg(not(feature = "device_benchmarks"))]
criterion_main!(basic_benches);
