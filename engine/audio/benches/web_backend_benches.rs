//! Web Audio API backend performance benchmarks
//!
//! These benchmarks measure the performance characteristics of the Web Audio backend.
//! They are designed to run in browser environments via wasm-pack.

#![cfg(target_arch = "wasm32")]

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use engine_audio::platform::web::WebAudioBackend;
use engine_audio::platform::AudioBackend;
use glam::Vec3;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

/// Benchmark backend initialization
fn bench_backend_creation(c: &mut Criterion) {
    c.bench_function("web_backend_creation", |b| {
        b.iter(|| {
            let backend = WebAudioBackend::new();
            black_box(backend)
        });
    });
}

/// Benchmark listener position updates
fn bench_listener_updates(c: &mut Criterion) {
    let mut backend = WebAudioBackend::new().unwrap();

    c.bench_function("listener_transform_update", |b| {
        let mut i = 0.0;
        b.iter(|| {
            i += 1.0;
            let position = Vec3::new(i, 0.0, 0.0);
            let forward = Vec3::new(0.0, 0.0, -1.0);
            let up = Vec3::new(0.0, 1.0, 0.0);

            backend.set_listener_transform(black_box(position), black_box(forward), black_box(up));
        });
    });
}

/// Benchmark emitter position updates
fn bench_emitter_updates(c: &mut Criterion) {
    let mut backend = WebAudioBackend::new().unwrap();

    // Pre-create emitters by attempting to update them
    for entity_id in 0..10 {
        backend.update_emitter_position(entity_id, Vec3::ZERO);
    }

    c.bench_function("emitter_position_update", |b| {
        let mut i = 0.0;
        b.iter(|| {
            i += 1.0;
            let position = Vec3::new(i, 0.0, 0.0);
            backend.update_emitter_position(black_box(0), black_box(position));
        });
    });
}

/// Benchmark emitter management (add/remove)
fn bench_emitter_management(c: &mut Criterion) {
    let mut backend = WebAudioBackend::new().unwrap();

    c.bench_function("emitter_add_remove", |b| {
        let mut entity_id = 0u32;
        b.iter(|| {
            entity_id = entity_id.wrapping_add(1);

            // Add emitter
            backend.update_emitter_position(black_box(entity_id), black_box(Vec3::ZERO));

            // Remove emitter
            backend.remove_emitter(black_box(entity_id));
        });
    });
}

/// Benchmark gain node creation
fn bench_gain_node_creation(c: &mut Criterion) {
    let backend = WebAudioBackend::new().unwrap();

    c.bench_function("gain_node_creation", |b| {
        b.iter(|| {
            let gain = backend.create_gain(black_box(1.0));
            black_box(gain)
        });
    });
}

/// Benchmark panner node creation
fn bench_panner_node_creation(c: &mut Criterion) {
    let backend = WebAudioBackend::new().unwrap();

    c.bench_function("panner_node_creation", |b| {
        b.iter(|| {
            let position = Vec3::new(10.0, 0.0, 5.0);
            let panner = backend.create_panner(black_box(position), black_box(100.0));
            black_box(panner)
        });
    });
}

/// Benchmark cleanup of finished sounds
fn bench_cleanup_finished(c: &mut Criterion) {
    let mut backend = WebAudioBackend::new().unwrap();

    c.bench_function("cleanup_finished_sounds", |b| {
        b.iter(|| {
            backend.cleanup_finished();
        });
    });
}

/// Benchmark active sound count queries
fn bench_active_sound_count(c: &mut Criterion) {
    let backend = WebAudioBackend::new().unwrap();

    c.bench_function("active_sound_count", |b| {
        b.iter(|| {
            let count = backend.active_sound_count();
            black_box(count)
        });
    });
}

/// Benchmark loaded sound count queries
fn bench_loaded_sound_count(c: &mut Criterion) {
    let backend = WebAudioBackend::new().unwrap();

    c.bench_function("loaded_sound_count", |b| {
        b.iter(|| {
            let count = backend.loaded_sound_count();
            black_box(count)
        });
    });
}

/// Benchmark is_playing queries
fn bench_is_playing_queries(c: &mut Criterion) {
    let backend = WebAudioBackend::new().unwrap();

    c.bench_function("is_playing_query", |b| {
        b.iter(|| {
            let is_playing = backend.is_playing(black_box(0));
            black_box(is_playing)
        });
    });
}

/// Benchmark stop operations
fn bench_stop_operations(c: &mut Criterion) {
    let mut backend = WebAudioBackend::new().unwrap();

    c.bench_function("stop_nonexistent_sound", |b| {
        let mut id = 0u64;
        b.iter(|| {
            id = id.wrapping_add(1);
            backend.stop(black_box(id), None);
        });
    });
}

/// Benchmark multiple concurrent emitters
fn bench_multiple_emitters(c: &mut Criterion) {
    let mut group = c.benchmark_group("multiple_emitters");

    for emitter_count in [1, 10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(emitter_count),
            emitter_count,
            |b, &count| {
                let mut backend = WebAudioBackend::new().unwrap();

                // Pre-create emitters
                for entity_id in 0..count {
                    backend.update_emitter_position(
                        entity_id,
                        Vec3::new(entity_id as f32 * 10.0, 0.0, 0.0),
                    );
                }

                b.iter(|| {
                    // Update all emitters
                    for entity_id in 0..count {
                        let position = Vec3::new(entity_id as f32 * 10.0, 1.0, 0.0);
                        backend.update_emitter_position(black_box(entity_id), black_box(position));
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark listener updates with various vector magnitudes
fn bench_listener_vector_magnitudes(c: &mut Criterion) {
    let mut group = c.benchmark_group("listener_vector_magnitudes");

    for magnitude in [0.1, 1.0, 10.0, 100.0].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(magnitude), magnitude, |b, &mag| {
            let mut backend = WebAudioBackend::new().unwrap();

            b.iter(|| {
                let position = Vec3::new(mag, 0.0, 0.0);
                let forward = Vec3::new(0.0, 0.0, -mag);
                let up = Vec3::new(0.0, mag, 0.0);

                backend.set_listener_transform(
                    black_box(position),
                    black_box(forward),
                    black_box(up),
                );
            });
        });
    }

    group.finish();
}

/// Benchmark distance attenuation calculations (via panner node creation)
fn bench_distance_attenuation(c: &mut Criterion) {
    let mut group = c.benchmark_group("distance_attenuation");

    for max_distance in [10.0, 100.0, 1000.0, 10000.0].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(max_distance),
            max_distance,
            |b, &max_dist| {
                let backend = WebAudioBackend::new().unwrap();

                b.iter(|| {
                    let position = Vec3::new(50.0, 0.0, 0.0);
                    let panner = backend.create_panner(black_box(position), black_box(max_dist));
                    black_box(panner)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark memory overhead tracking
fn bench_memory_tracking(c: &mut Criterion) {
    let mut backend = WebAudioBackend::new().unwrap();

    c.bench_function("memory_tracking_queries", |b| {
        b.iter(|| {
            let active = backend.active_sound_count();
            let loaded = backend.loaded_sound_count();
            black_box((active, loaded))
        });
    });
}

criterion_group!(
    benches,
    bench_backend_creation,
    bench_listener_updates,
    bench_emitter_updates,
    bench_emitter_management,
    bench_gain_node_creation,
    bench_panner_node_creation,
    bench_cleanup_finished,
    bench_active_sound_count,
    bench_loaded_sound_count,
    bench_is_playing_queries,
    bench_stop_operations,
    bench_multiple_emitters,
    bench_listener_vector_magnitudes,
    bench_distance_attenuation,
    bench_memory_tracking,
);

criterion_main!(benches);
