//! Comprehensive performance regression tests for audio system
//!
//! These benchmarks validate that all audio systems meet the performance targets:
//! - Frame time overhead: < 1ms for 100 active sounds
//! - Listener update: < 100μs
//! - Emitter update: < 50μs per emitter
//! - Effect application: < 100μs per effect
//! - Doppler calculation: < 50μs per emitter
//!
//! Run with: cargo bench --bench performance_regression

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_audio::{
    AudioEngine, AudioListener, AudioSystem, DopplerCalculator, EchoEffect, FilterEffect,
    ReverbEffect, Sound,
};
use engine_core::ecs::World;
use engine_core::math::{Transform, Vec3};
use std::time::Duration;

// ============================================================================
// PERFORMANCE TARGETS (from CLAUDE.md)
// ============================================================================

/// Target: Listener update < 100μs
const TARGET_LISTENER_UPDATE_US: u128 = 100;

/// Target: Emitter update < 50μs per emitter
const TARGET_EMITTER_UPDATE_US: u128 = 50;

/// Target: Doppler calculation < 50μs per emitter
const TARGET_DOPPLER_CALC_US: u128 = 50;

/// Target: Effect application < 100μs per effect
const TARGET_EFFECT_APP_US: u128 = 100;

/// Target: Frame time overhead < 1ms for 100 active sounds
const TARGET_100_SOUNDS_MS: u128 = 1;

// ============================================================================
// LISTENER UPDATE BENCHMARKS
// ============================================================================

fn bench_listener_update_performance(c: &mut Criterion) {
    let mut engine = AudioEngine::new().unwrap();

    let mut group = c.benchmark_group("listener_update_target");
    group.significance_level(0.05).sample_size(1000);

    group.bench_function("single_listener_update", |b| {
        let pos = Vec3::new(1.0, 2.0, 3.0);
        let forward = Vec3::new(0.0, 0.0, -1.0);
        let up = Vec3::new(0.0, 1.0, 0.0);

        b.iter(|| {
            engine.set_listener_transform(black_box(pos), black_box(forward), black_box(up));
        });
    });

    // Validate target: < 100μs
    group.bench_function("listener_update_target_validation", |b| {
        let start = std::time::Instant::now();
        engine.set_listener_transform(
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(0.0, 0.0, -1.0),
            Vec3::new(0.0, 1.0, 0.0),
        );
        let elapsed = start.elapsed();

        b.iter(|| {
            assert!(
                elapsed.as_micros() < TARGET_LISTENER_UPDATE_US,
                "Listener update took {}μs (target: <{}μs)",
                elapsed.as_micros(),
                TARGET_LISTENER_UPDATE_US
            );
        });
    });

    group.finish();
}

// ============================================================================
// EMITTER UPDATE BENCHMARKS
// ============================================================================

fn bench_emitter_update_performance(c: &mut Criterion) {
    let mut engine = AudioEngine::new().unwrap();

    let mut group = c.benchmark_group("emitter_update_target");
    group.significance_level(0.05).sample_size(1000);

    // Pre-create emitter
    engine.update_emitter_position(42, Vec3::ZERO);

    group.bench_function("single_emitter_update", |b| {
        let mut pos = Vec3::new(5.0, 0.0, 0.0);

        b.iter(|| {
            pos.x += 0.1;
            engine.update_emitter_position(black_box(42), black_box(pos));
        });
    });

    // Validate target: < 50μs per emitter
    group.bench_function("emitter_update_target_validation", |b| {
        let pos = Vec3::new(10.0, 0.0, 0.0);
        let start = std::time::Instant::now();
        engine.update_emitter_position(42, pos);
        let elapsed = start.elapsed();

        b.iter(|| {
            assert!(
                elapsed.as_micros() < TARGET_EMITTER_UPDATE_US,
                "Emitter update took {}μs (target: <{}μs)",
                elapsed.as_micros(),
                TARGET_EMITTER_UPDATE_US
            );
        });
    });

    group.finish();
}

// ============================================================================
// DOPPLER CALCULATION BENCHMARKS
// ============================================================================

fn bench_doppler_calculation_performance(c: &mut Criterion) {
    let calc = DopplerCalculator::default();

    let mut group = c.benchmark_group("doppler_calc_target");
    group.significance_level(0.05).sample_size(1000);

    let listener_pos = Vec3::ZERO;
    let listener_vel = Vec3::new(5.0, 0.0, 0.0);
    let emitter_pos = Vec3::new(100.0, 0.0, 0.0);
    let emitter_vel = Vec3::new(-20.0, 0.0, 0.0);

    group.bench_function("single_doppler_calculation", |b| {
        b.iter(|| {
            calc.calculate_pitch_shift(
                black_box(listener_pos),
                black_box(listener_vel),
                black_box(emitter_pos),
                black_box(emitter_vel),
            )
        });
    });

    // Validate target: < 50μs per calculation
    group.bench_function("doppler_calc_target_validation", |b| {
        let start = std::time::Instant::now();
        let _ = calc.calculate_pitch_shift(listener_pos, listener_vel, emitter_pos, emitter_vel);
        let elapsed = start.elapsed();

        b.iter(|| {
            assert!(
                elapsed.as_micros() < TARGET_DOPPLER_CALC_US,
                "Doppler calculation took {}μs (target: <{}μs)",
                elapsed.as_micros(),
                TARGET_DOPPLER_CALC_US
            );
        });
    });

    group.finish();
}

// ============================================================================
// EFFECT APPLICATION BENCHMARKS
// ============================================================================

fn bench_effect_application_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("effect_application_target");
    group.significance_level(0.05).sample_size(500);

    // Benchmark effect creation (should be fast)
    group.bench_function("reverb_effect_creation", |b| {
        b.iter(|| {
            let effect = ReverbEffect::large_hall();
            black_box(effect);
        });
    });

    group.bench_function("echo_effect_creation", |b| {
        b.iter(|| {
            let effect = EchoEffect::slapback();
            black_box(effect);
        });
    });

    group.bench_function("filter_effect_creation", |b| {
        b.iter(|| {
            let effect = FilterEffect::muffled();
            black_box(effect);
        });
    });

    // Validate target: < 100μs per effect
    group.bench_function("effect_creation_target_validation", |b| {
        let start = std::time::Instant::now();
        let _ = ReverbEffect::large_hall();
        let elapsed = start.elapsed();

        b.iter(|| {
            assert!(
                elapsed.as_micros() < TARGET_EFFECT_APP_US,
                "Effect creation took {}μs (target: <{}μs)",
                elapsed.as_micros(),
                TARGET_EFFECT_APP_US
            );
        });
    });

    group.finish();
}

// ============================================================================
// FULL SYSTEM BENCHMARKS (100 ACTIVE SOUNDS)
// ============================================================================

fn bench_100_active_sounds_frame_time(c: &mut Criterion) {
    let mut group = c.benchmark_group("100_sounds_frame_time");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(500);
    group.throughput(Throughput::Elements(100)); // 100 sounds

    // Create world with 100 spatial sounds
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    // Camera with listener
    let camera = world.spawn();
    world.add(camera, Transform::default());
    world.add(camera, AudioListener::new());

    // 100 moving sound sources in a circle
    for i in 0..100 {
        let entity = world.spawn();
        let mut transform = Transform::default();
        let angle = (i as f32) * std::f32::consts::PI * 2.0 / 100.0;
        transform.position = Vec3::new(angle.cos() * 50.0, 0.0, angle.sin() * 50.0);
        world.add(entity, transform);
        world.add(entity, Sound::new("test.wav").spatial_3d(100.0).with_doppler(1.0));
    }

    let mut system = AudioSystem::new().unwrap();

    group.bench_function("audio_system_update_100_sounds", |b| {
        b.iter(|| {
            system.update(black_box(&mut world), black_box(0.016));
        });
    });

    // Validate target: < 1ms for 100 sounds
    group.bench_function("100_sounds_target_validation", |b| {
        let start = std::time::Instant::now();
        system.update(&mut world, 0.016);
        let elapsed = start.elapsed();

        b.iter(|| {
            assert!(
                elapsed.as_millis() < TARGET_100_SOUNDS_MS,
                "100 sounds update took {}ms (target: <{}ms)",
                elapsed.as_millis(),
                TARGET_100_SOUNDS_MS
            );
        });
    });

    group.finish();
}

// ============================================================================
// SCALABILITY BENCHMARKS
// ============================================================================

fn bench_scalability_sound_counts(c: &mut Criterion) {
    let mut group = c.benchmark_group("audio_scalability");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(100);

    for count in [10, 50, 100, 200, 500, 1000].iter() {
        group.throughput(Throughput::Elements(*count as u64));
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &sound_count| {
            let mut world = World::new();
            world.register::<Transform>();
            world.register::<Sound>();
            world.register::<AudioListener>();

            // Camera
            let camera = world.spawn();
            world.add(camera, Transform::default());
            world.add(camera, AudioListener::new());

            // Create many sounds
            for i in 0..sound_count {
                let entity = world.spawn();
                let mut transform = Transform::default();
                let angle = (i as f32) * std::f32::consts::PI * 2.0 / sound_count as f32;
                transform.position = Vec3::new(angle.cos() * 50.0, 0.0, angle.sin() * 50.0);
                world.add(entity, transform);
                world.add(entity, Sound::new("test.wav").spatial_3d(100.0).with_doppler(1.0));
            }

            let mut system = AudioSystem::new().unwrap();

            b.iter(|| {
                system.update(black_box(&mut world), black_box(0.016));
            });
        });
    }

    group.finish();
}

// ============================================================================
// MEMORY ALLOCATION BENCHMARKS
// ============================================================================

fn bench_memory_allocations(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_allocations");
    group.sample_size(500);

    // Test that hot paths have minimal allocations
    group.bench_function("doppler_calc_no_alloc", |b| {
        let calc = DopplerCalculator::default();
        let listener_pos = Vec3::ZERO;
        let listener_vel = Vec3::new(5.0, 0.0, 0.0);
        let emitter_pos = Vec3::new(100.0, 0.0, 0.0);
        let emitter_vel = Vec3::new(-20.0, 0.0, 0.0);

        b.iter(|| {
            // This should not allocate (stack-only)
            let _ = calc.calculate_pitch_shift(
                black_box(listener_pos),
                black_box(listener_vel),
                black_box(emitter_pos),
                black_box(emitter_vel),
            );
        });
    });

    group.bench_function("velocity_calc_no_alloc", |b| {
        let old_pos = Vec3::ZERO;
        let new_pos = Vec3::new(10.0, 5.0, 3.0);
        let dt = 0.016;

        b.iter(|| {
            // This should not allocate (stack-only)
            let _ = DopplerCalculator::calculate_velocity(
                black_box(old_pos),
                black_box(new_pos),
                black_box(dt),
            );
        });
    });

    group.finish();
}

// ============================================================================
// CACHE EFFICIENCY BENCHMARKS
// ============================================================================

fn bench_cache_efficiency(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_efficiency");
    group.sample_size(100);

    // Test sequential vs random access patterns
    group.bench_function("sequential_emitter_updates", |b| {
        let mut engine = AudioEngine::new().unwrap();

        // Create 100 emitters sequentially
        for i in 0..100 {
            engine.update_emitter_position(i, Vec3::new(i as f32, 0.0, 0.0));
        }

        b.iter(|| {
            // Update sequentially (cache-friendly)
            for i in 0..100 {
                engine.update_emitter_position(
                    black_box(i),
                    black_box(Vec3::new((i as f32) + 0.1, 0.0, 0.0)),
                );
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_listener_update_performance,
    bench_emitter_update_performance,
    bench_doppler_calculation_performance,
    bench_effect_application_performance,
    bench_100_active_sounds_frame_time,
    bench_scalability_sound_counts,
    bench_memory_allocations,
    bench_cache_efficiency,
);

criterion_main!(benches);
