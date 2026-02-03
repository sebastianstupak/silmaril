//! Web Audio API benchmarks
//!
//! Browser-compatible benchmarks for Web Audio backend.
//! These benchmarks measure performance of audio operations in WASM.

#![cfg(target_arch = "wasm32")]

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use engine_audio::AudioEngine;
use glam::Vec3;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

/// Benchmark audio engine creation
fn bench_engine_creation(c: &mut Criterion) {
    c.bench_function("web_audio_engine_creation", |b| {
        b.iter(|| {
            let engine = AudioEngine::new();
            assert!(engine.is_ok());
        });
    });
}

/// Benchmark listener transform updates
fn bench_listener_transform(c: &mut Criterion) {
    let mut engine = AudioEngine::new().unwrap();

    c.bench_function("web_audio_listener_transform", |b| {
        b.iter(|| {
            engine.set_listener_transform(
                Vec3::new(1.0, 2.0, 3.0),
                Vec3::new(0.0, 0.0, -1.0),
                Vec3::new(0.0, 1.0, 0.0),
            );
        });
    });
}

/// Benchmark emitter position updates
fn bench_emitter_updates(c: &mut Criterion) {
    let mut engine = AudioEngine::new().unwrap();

    // Pre-create emitters
    for i in 0..100 {
        engine.update_emitter_position(i, Vec3::ZERO);
    }

    c.bench_function("web_audio_emitter_update", |b| {
        b.iter(|| {
            for i in 0..100 {
                engine.update_emitter_position(i, Vec3::new(i as f32, 0.0, 0.0));
            }
        });
    });
}

/// Benchmark emitter creation/removal
fn bench_emitter_lifecycle(c: &mut Criterion) {
    let mut group = c.benchmark_group("web_audio_emitter_lifecycle");

    for count in [10, 50, 100].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            b.iter(|| {
                let mut engine = AudioEngine::new().unwrap();

                // Create emitters
                for i in 0..count {
                    engine.update_emitter_position(i, Vec3::ZERO);
                }

                // Remove emitters
                for i in 0..count {
                    engine.remove_emitter(i);
                }
            });
        });
    }

    group.finish();
}

/// Benchmark cleanup operations
fn bench_cleanup(c: &mut Criterion) {
    c.bench_function("web_audio_cleanup_finished", |b| {
        let mut engine = AudioEngine::new().unwrap();

        b.iter(|| {
            engine.cleanup_finished();
        });
    });
}

/// Benchmark active sound count queries
fn bench_active_sound_count(c: &mut Criterion) {
    let engine = AudioEngine::new().unwrap();

    c.bench_function("web_audio_active_sound_count", |b| {
        b.iter(|| {
            let _ = engine.active_sound_count();
        });
    });
}

/// Benchmark is_playing queries
fn bench_is_playing(c: &mut Criterion) {
    let engine = AudioEngine::new().unwrap();

    c.bench_function("web_audio_is_playing", |b| {
        b.iter(|| {
            let _ = engine.is_playing(0);
        });
    });
}

criterion_group!(
    benches,
    bench_engine_creation,
    bench_listener_transform,
    bench_emitter_updates,
    bench_emitter_lifecycle,
    bench_cleanup,
    bench_active_sound_count,
    bench_is_playing,
);

criterion_main!(benches);

// Note: To run these benchmarks in a browser:
// 1. Install wasm-pack: cargo install wasm-pack
// 2. Build for browser: wasm-pack test --headless --firefox engine/audio
// 3. Or use wasm-bindgen-test-runner with criterion support
//
// For actual playback benchmarks, you would need to:
// 1. Serve test audio files
// 2. Load them in the benchmark setup
// 3. Measure playback, stopping, and streaming operations
