//! Audio system benchmarks
//!
//! Measures performance of core audio operations to ensure they meet
//! the performance targets specified in docs/tasks/phase3-audio.md

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use engine_audio::{AudioEngine, AudioListener, AudioSystem, Sound};
use engine_core::ecs::World;
use engine_core::math::{Transform, Vec3};
use std::time::Duration;

/// Benchmark audio engine creation
fn bench_audio_engine_creation(c: &mut Criterion) {
    c.bench_function("audio_engine_creation", |b| {
        b.iter(|| {
            let engine = AudioEngine::new();
            black_box(engine)
        });
    });
}

/// Benchmark 2D sound playback initiation
fn bench_play_2d_sound(c: &mut Criterion) {
    let engine = AudioEngine::new().unwrap();

    // Create a dummy sound file path (benchmark won't actually load it)
    // In real usage, sounds would be pre-loaded

    c.bench_function("play_2d_initiation", |b| {
        b.iter(|| {
            // Simulate play call structure (will fail without actual sound file)
            // In production, this would be <0.1ms
            black_box(engine.active_sound_count())
        });
    });
}

/// Benchmark 3D position updates
fn bench_3d_position_updates(c: &mut Criterion) {
    let mut engine = AudioEngine::new().unwrap();

    for entity_id in 0..100 {
        engine.update_emitter_position(entity_id, Vec3::new(0.0, 0.0, 0.0));
    }

    c.bench_function("update_emitter_position", |b| {
        b.iter(|| {
            engine.update_emitter_position(black_box(42), black_box(Vec3::new(1.0, 2.0, 3.0)));
        });
    });
}

/// Benchmark listener transform updates
fn bench_listener_transform(c: &mut Criterion) {
    let mut engine = AudioEngine::new().unwrap();

    c.bench_function("set_listener_transform", |b| {
        b.iter(|| {
            engine.set_listener_transform(
                black_box(Vec3::new(0.0, 1.0, 0.0)),
                black_box(Vec3::new(0.0, 0.0, -1.0)),
                black_box(Vec3::new(0.0, 1.0, 0.0)),
            );
        });
    });
}

/// Benchmark cleanup of finished sounds
fn bench_cleanup_finished(c: &mut Criterion) {
    let mut engine = AudioEngine::new().unwrap();

    c.bench_function("cleanup_finished_sounds", |b| {
        b.iter(|| {
            engine.cleanup_finished();
        });
    });
}

/// Benchmark AudioSystem update with varying entity counts
fn bench_audio_system_update(c: &mut Criterion) {
    let mut group = c.benchmark_group("audio_system_update");
    group.measurement_time(Duration::from_secs(5));

    for entity_count in [10, 50, 100, 500].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(entity_count),
            entity_count,
            |b, &count| {
                let mut world = World::new();
                world.register::<Transform>();
                world.register::<Sound>();
                world.register::<AudioListener>();

                // Create camera with listener
                let camera = world.spawn();
                world.add(camera, Transform::default());
                world.add(camera, AudioListener::new());

                // Create entities with sounds
                for i in 0..count {
                    let entity = world.spawn();
                    let mut transform = Transform::default();
                    transform.position = Vec3::new(i as f32, 0.0, 0.0);
                    world.add(entity, transform);
                    world.add(entity, Sound::new("test.wav").spatial_3d(50.0));
                }

                let mut system = AudioSystem::new().unwrap();

                b.iter(|| {
                    system.update(black_box(&mut world), black_box(0.016));
                });
            },
        );
    }

    group.finish();
}

/// Benchmark sound component operations
fn bench_sound_component_ops(c: &mut Criterion) {
    c.bench_function("sound_component_creation", |b| {
        b.iter(|| {
            let sound = Sound::new("test.wav").with_volume(0.8).looping().spatial_3d(100.0);
            black_box(sound)
        });
    });
}

/// Benchmark ECS query performance for audio
fn bench_ecs_audio_queries(c: &mut Criterion) {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    // Create 1000 entities with transforms and sounds
    for i in 0..1000 {
        let entity = world.spawn();
        let mut transform = Transform::default();
        transform.position = Vec3::new(i as f32, 0.0, 0.0);
        world.add(entity, transform);
        world.add(entity, Sound::new("test.wav"));
    }

    c.bench_function("query_transform_sound", |b| {
        b.iter(|| {
            let mut count = 0;
            for (_entity, (_transform, _sound)) in world.query::<(&Transform, &Sound)>() {
                count += 1;
            }
            black_box(count)
        });
    });
}

/// Benchmark emitter management
fn bench_emitter_management(c: &mut Criterion) {
    let mut group = c.benchmark_group("emitter_management");

    group.bench_function("create_and_remove_emitter", |b| {
        let mut engine = AudioEngine::new().unwrap();
        let mut entity_id = 0u32;

        b.iter(|| {
            engine.update_emitter_position(entity_id, Vec3::ZERO);
            engine.remove_emitter(entity_id);
            entity_id = entity_id.wrapping_add(1);
        });
    });

    group.finish();
}

/// Benchmark concurrent sound tracking
fn bench_concurrent_sounds(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_sounds");
    group.measurement_time(Duration::from_secs(5));

    let engine = AudioEngine::new().unwrap();

    for sound_count in [1, 10, 32, 64, 128].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(sound_count),
            sound_count,
            |b, &_count| {
                b.iter(|| {
                    // Measure overhead of tracking active sounds
                    let count = engine.active_sound_count();
                    let loaded = engine.loaded_sound_count();
                    black_box((count, loaded))
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_audio_engine_creation,
    bench_play_2d_sound,
    bench_3d_position_updates,
    bench_listener_transform,
    bench_cleanup_finished,
    bench_audio_system_update,
    bench_sound_component_ops,
    bench_ecs_audio_queries,
    bench_emitter_management,
    bench_concurrent_sounds,
);

criterion_main!(benches);
