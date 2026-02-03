//! iOS audio backend performance benchmarks
//!
//! These benchmarks measure the performance of the iOS Core Audio backend
//! to ensure it meets AAA game engine performance targets.
//!
//! Performance targets:
//! - Listener transform update: < 100μs
//! - Emitter position update: < 50μs
//! - Sound playback initiation: < 1ms
//! - Cleanup finished sounds: < 500μs (for 100 sounds)

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use engine_audio::platform::{create_audio_backend, AudioBackend};
use glam::Vec3;
use std::time::Duration;

/// Benchmark listener transform updates
fn bench_listener_transform_update(c: &mut Criterion) {
    #[cfg(target_os = "ios")]
    {
        let mut backend = create_audio_backend().expect("Failed to create backend");

        c.bench_function("ios_listener_transform_update", |b| {
            b.iter(|| {
                backend.set_listener_transform(
                    black_box(Vec3::new(1.0, 2.0, 3.0)),
                    black_box(Vec3::new(0.0, 0.0, -1.0)),
                    black_box(Vec3::new(0.0, 1.0, 0.0)),
                );
            });
        });
    }

    #[cfg(not(target_os = "ios"))]
    {
        c.bench_function("ios_listener_transform_update", |b| {
            b.iter(|| {
                // No-op on non-iOS
                black_box(());
            });
        });
    }
}

/// Benchmark emitter position updates
fn bench_emitter_position_update(c: &mut Criterion) {
    #[cfg(target_os = "ios")]
    {
        let mut backend = create_audio_backend().expect("Failed to create backend");

        // Create emitter
        backend.update_emitter_position(1, Vec3::ZERO);

        c.bench_function("ios_emitter_position_update", |b| {
            b.iter(|| {
                backend.update_emitter_position(black_box(1), black_box(Vec3::new(5.0, 0.0, 0.0)));
            });
        });
    }

    #[cfg(not(target_os = "ios"))]
    {
        c.bench_function("ios_emitter_position_update", |b| {
            b.iter(|| {
                black_box(());
            });
        });
    }
}

/// Benchmark emitter creation and removal
fn bench_emitter_lifecycle(c: &mut Criterion) {
    #[cfg(target_os = "ios")]
    {
        let mut backend = create_audio_backend().expect("Failed to create backend");
        let mut entity_id = 0u32;

        c.bench_function("ios_emitter_create_and_remove", |b| {
            b.iter(|| {
                backend.update_emitter_position(entity_id, Vec3::ZERO);
                backend.remove_emitter(entity_id);
                entity_id = entity_id.wrapping_add(1);
            });
        });
    }

    #[cfg(not(target_os = "ios"))]
    {
        c.bench_function("ios_emitter_create_and_remove", |b| {
            b.iter(|| {
                black_box(());
            });
        });
    }
}

/// Benchmark cleanup of finished sounds
fn bench_cleanup_finished(c: &mut Criterion) {
    #[cfg(target_os = "ios")]
    {
        let mut backend = create_audio_backend().expect("Failed to create backend");

        c.bench_function("ios_cleanup_finished", |b| {
            b.iter(|| {
                backend.cleanup_finished();
            });
        });
    }

    #[cfg(not(target_os = "ios"))]
    {
        c.bench_function("ios_cleanup_finished", |b| {
            b.iter(|| {
                black_box(());
            });
        });
    }
}

/// Benchmark active sound count query
fn bench_active_sound_count(c: &mut Criterion) {
    #[cfg(target_os = "ios")]
    {
        let backend = create_audio_backend().expect("Failed to create backend");

        c.bench_function("ios_active_sound_count", |b| {
            b.iter(|| {
                let count = backend.active_sound_count();
                black_box(count)
            });
        });
    }

    #[cfg(not(target_os = "ios"))]
    {
        c.bench_function("ios_active_sound_count", |b| {
            b.iter(|| {
                black_box(0);
            });
        });
    }
}

/// Benchmark loaded sound count query
fn bench_loaded_sound_count(c: &mut Criterion) {
    #[cfg(target_os = "ios")]
    {
        let backend = create_audio_backend().expect("Failed to create backend");

        c.bench_function("ios_loaded_sound_count", |b| {
            b.iter(|| {
                let count = backend.loaded_sound_count();
                black_box(count)
            });
        });
    }

    #[cfg(not(target_os = "ios"))]
    {
        c.bench_function("ios_loaded_sound_count", |b| {
            b.iter(|| {
                black_box(0);
            });
        });
    }
}

/// Benchmark is_playing check
fn bench_is_playing_check(c: &mut Criterion) {
    #[cfg(target_os = "ios")]
    {
        let backend = create_audio_backend().expect("Failed to create backend");

        c.bench_function("ios_is_playing_check", |b| {
            b.iter(|| {
                let playing = backend.is_playing(black_box(1));
                black_box(playing)
            });
        });
    }

    #[cfg(not(target_os = "ios"))]
    {
        c.bench_function("ios_is_playing_check", |b| {
            b.iter(|| {
                black_box(false);
            });
        });
    }
}

/// Benchmark backend creation
fn bench_backend_creation(c: &mut Criterion) {
    #[cfg(target_os = "ios")]
    {
        c.bench_function("ios_backend_creation", |b| {
            b.iter(|| {
                let backend = create_audio_backend();
                black_box(backend)
            });
        });
    }

    #[cfg(not(target_os = "ios"))]
    {
        c.bench_function("ios_backend_creation", |b| {
            b.iter(|| {
                black_box(());
            });
        });
    }
}

/// Benchmark multiple emitter position updates
fn bench_multiple_emitter_updates(c: &mut Criterion) {
    let mut group = c.benchmark_group("ios_multiple_emitter_updates");
    group.measurement_time(Duration::from_secs(5));

    #[cfg(target_os = "ios")]
    {
        for emitter_count in [10, 50, 100, 200].iter() {
            group.bench_with_input(
                BenchmarkId::from_parameter(emitter_count),
                emitter_count,
                |b, &count| {
                    let mut backend = create_audio_backend().expect("Failed to create backend");

                    // Create emitters
                    for i in 0..count {
                        backend.update_emitter_position(i, Vec3::ZERO);
                    }

                    b.iter(|| {
                        // Update all emitters
                        for i in 0..count {
                            backend.update_emitter_position(i, Vec3::new(i as f32, 0.0, 0.0));
                        }
                    });
                },
            );
        }
    }

    #[cfg(not(target_os = "ios"))]
    {
        group.bench_function("mock", |b| {
            b.iter(|| {
                black_box(());
            });
        });
    }

    group.finish();
}

/// Benchmark listener transform updates with varying frequencies
fn bench_listener_transform_frequency(c: &mut Criterion) {
    let mut group = c.benchmark_group("ios_listener_transform_frequency");
    group.measurement_time(Duration::from_secs(5));

    #[cfg(target_os = "ios")]
    {
        for update_count in [1, 10, 60, 120].iter() {
            group.bench_with_input(
                BenchmarkId::from_parameter(update_count),
                update_count,
                |b, &count| {
                    let mut backend = create_audio_backend().expect("Failed to create backend");

                    b.iter(|| {
                        for i in 0..count {
                            let offset = i as f32 * 0.1;
                            backend.set_listener_transform(
                                Vec3::new(offset, 0.0, 0.0),
                                Vec3::new(0.0, 0.0, -1.0),
                                Vec3::new(0.0, 1.0, 0.0),
                            );
                        }
                    });
                },
            );
        }
    }

    #[cfg(not(target_os = "ios"))]
    {
        group.bench_function("mock", |b| {
            b.iter(|| {
                black_box(());
            });
        });
    }

    group.finish();
}

// Benchmarks that require test audio files

/// Benchmark sound loading (requires test assets)
#[cfg(all(target_os = "ios", feature = "bench_with_assets"))]
fn bench_sound_loading(c: &mut Criterion) {
    use std::path::Path;

    c.bench_function("ios_load_sound", |b| {
        let mut backend = create_audio_backend().expect("Failed to create backend");
        let mut sound_id = 0;

        b.iter(|| {
            let name = format!("test_sound_{}", sound_id);
            let _ = backend.load_sound(&name, Path::new("test_assets/test.wav"));
            sound_id += 1;
        });
    });
}

/// Benchmark 2D sound playback (requires test assets)
#[cfg(all(target_os = "ios", feature = "bench_with_assets"))]
fn bench_play_2d_sound(c: &mut Criterion) {
    use std::path::Path;

    let mut backend = create_audio_backend().expect("Failed to create backend");
    backend
        .load_sound("test", Path::new("test_assets/test.wav"))
        .expect("Failed to load test sound");

    c.bench_function("ios_play_2d_sound", |b| {
        b.iter(|| {
            let instance_id = backend.play_2d("test", black_box(1.0), false);
            if let Ok(id) = instance_id {
                backend.stop(id, None);
            }
        });
    });
}

/// Benchmark 3D sound playback (requires test assets)
#[cfg(all(target_os = "ios", feature = "bench_with_assets"))]
fn bench_play_3d_sound(c: &mut Criterion) {
    use std::path::Path;

    let mut backend = create_audio_backend().expect("Failed to create backend");
    backend
        .load_sound("test", Path::new("test_assets/test.wav"))
        .expect("Failed to load test sound");

    c.bench_function("ios_play_3d_sound", |b| {
        b.iter(|| {
            let instance_id = backend.play_3d(
                black_box(1),
                "test",
                black_box(Vec3::new(5.0, 0.0, 0.0)),
                black_box(1.0),
                false,
                50.0,
            );
            if let Ok(id) = instance_id {
                backend.stop(id, None);
            }
        });
    });
}

criterion_group!(
    benches,
    bench_listener_transform_update,
    bench_emitter_position_update,
    bench_emitter_lifecycle,
    bench_cleanup_finished,
    bench_active_sound_count,
    bench_loaded_sound_count,
    bench_is_playing_check,
    bench_backend_creation,
    bench_multiple_emitter_updates,
    bench_listener_transform_frequency,
);

criterion_main!(benches);
