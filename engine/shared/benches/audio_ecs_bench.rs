//! Cross-crate benchmark: Audio + ECS performance
//!
//! MANDATORY: Uses engine-audio + engine-core, so it MUST be in engine/shared/benches/
//!
//! Benchmarks:
//! - AudioSystem::update() at various entity counts (1, 10, 100, 1000 entities)
//! - Listener update overhead
//! - Emitter position sync overhead
//! - Doppler effect calculations
//! - Full frame simulation (ECS + audio)
//!
//! Performance Targets:
//! - < 1ms for 100 entities
//! - < 10ms for 1000 entities
//! - Listener update: < 100µs
//! - Emitter position sync (100 entities): < 500µs

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use engine_audio::{AudioListener, AudioSystem, Sound};
use engine_core::ecs::World;
use engine_core::math::{Quat, Transform, Vec3};

const DELTA_TIME: f32 = 0.016; // 60 FPS

fn setup_world_with_sounds(count: usize) -> (World, AudioSystem) {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    // Add camera with listener
    let camera = world.spawn();
    world.add(camera, Transform::default());
    world.add(camera, AudioListener::new());

    // Add entities with sounds
    for i in 0..count {
        let entity = world.spawn();
        world.add(
            entity,
            Transform {
                position: Vec3::new(i as f32 * 2.0, 0.0, 0.0),
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
            },
        );

        let mut sound = Sound::new("test.wav").spatial_3d(100.0);
        // Simulate some sounds playing
        if i % 3 == 0 {
            sound.instance_id = Some(i as u64);
        }
        world.add(entity, sound);
    }

    let audio_system = AudioSystem::new().unwrap();

    (world, audio_system)
}

fn bench_audio_system_update(c: &mut Criterion) {
    let mut group = c.benchmark_group("audio_system_update");

    for count in [1, 10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            let (mut world, mut audio_system) = setup_world_with_sounds(count);

            b.iter(|| {
                audio_system.update(black_box(&mut world), DELTA_TIME);
            });
        });
    }

    group.finish();
}

fn bench_listener_update_only(c: &mut Criterion) {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<AudioListener>();

    let camera = world.spawn();
    world.add(
        camera,
        Transform {
            position: Vec3::new(5.0, 2.0, 3.0),
            rotation: Quat::from_rotation_y(0.5),
            scale: Vec3::ONE,
        },
    );
    world.add(camera, AudioListener::new());

    let mut audio_system = AudioSystem::new().unwrap();

    c.bench_function("listener_update_only", |b| {
        b.iter(|| {
            audio_system.update(black_box(&mut world), DELTA_TIME);
        });
    });
}

fn bench_emitter_position_updates(c: &mut Criterion) {
    let mut group = c.benchmark_group("emitter_position_updates");

    for count in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            let (mut world, mut audio_system) = setup_world_with_sounds(count);

            // Mark all sounds as playing to force position updates
            for (_entity, sound) in world.query_mut::<&mut Sound>() {
                sound.instance_id = Some(1);
            }

            b.iter(|| {
                // Move all entities
                for (_entity, transform) in world.query_mut::<&mut Transform>() {
                    transform.position.x += 0.1;
                }

                audio_system.update(black_box(&mut world), DELTA_TIME);
            });
        });
    }

    group.finish();
}

fn bench_mixed_spatial_nonspatial(c: &mut Criterion) {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let camera = world.spawn();
    world.add(camera, Transform::default());
    world.add(camera, AudioListener::new());

    // Add mix of spatial and non-spatial sounds
    for i in 0..100 {
        let entity = world.spawn();
        world.add(
            entity,
            Transform {
                position: Vec3::new(i as f32, 0.0, 0.0),
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
            },
        );

        let mut sound = if i % 2 == 0 {
            Sound::new("spatial.wav").spatial_3d(100.0)
        } else {
            Sound::new("ui.wav").non_spatial()
        };
        sound.instance_id = Some(i as u64);
        world.add(entity, sound);
    }

    let mut audio_system = AudioSystem::new().unwrap();

    c.bench_function("mixed_spatial_nonspatial", |b| {
        b.iter(|| {
            audio_system.update(black_box(&mut world), DELTA_TIME);
        });
    });
}

fn bench_world_query_overhead(c: &mut Criterion) {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    // Populate world
    for i in 0..1000 {
        let entity = world.spawn();
        world.add(entity, Transform::default());
        if i % 2 == 0 {
            world.add(entity, Sound::new("test.wav"));
        }
        if i % 10 == 0 {
            world.add(entity, AudioListener::default());
        }
    }

    c.bench_function("world_query_overhead", |b| {
        b.iter(|| {
            // Measure query overhead
            let listener_count = world.query::<(&Transform, &AudioListener)>().count();
            let sound_count = world.query::<(&Transform, &Sound)>().count();
            black_box((listener_count, sound_count));
        });
    });
}

fn bench_cleanup_finished_sounds(c: &mut Criterion) {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let mut audio_system = AudioSystem::new().unwrap();

    c.bench_function("cleanup_finished_sounds", |b| {
        b.iter(|| {
            audio_system.update(black_box(&mut world), DELTA_TIME);
        });
    });
}

fn bench_doppler_calculation_overhead(c: &mut Criterion) {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let camera = world.spawn();
    world.add(camera, Transform::default());
    world.add(camera, AudioListener::new());

    // Add entities with Doppler-enabled sounds
    for i in 0..100 {
        let entity = world.spawn();
        world.add(
            entity,
            Transform {
                position: Vec3::new(i as f32 * 5.0, 0.0, 0.0),
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
            },
        );

        let mut sound = Sound::new("car.wav").spatial_3d(100.0).with_doppler(1.0);
        sound.instance_id = Some(i as u64); // Mark as playing
        world.add(entity, sound);
    }

    let mut audio_system = AudioSystem::new().unwrap();

    c.bench_function("doppler_calculation_overhead", |b| {
        b.iter(|| {
            // Move all entities to trigger Doppler calculations
            for (_entity, transform) in world.query_mut::<&mut Transform>() {
                transform.position.x += 1.0;
            }

            audio_system.update(black_box(&mut world), DELTA_TIME);
        });
    });
}

fn bench_full_frame_simulation(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_frame_simulation");

    for count in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            let (mut world, mut audio_system) = setup_world_with_sounds(count);

            // Mark 50% as playing
            let mut i = 0;
            for (_entity, sound) in world.query_mut::<&mut Sound>() {
                if i % 2 == 0 {
                    sound.instance_id = Some(i as u64);
                }
                i += 1;
            }

            b.iter(|| {
                // Simulate full frame: move entities + update audio
                for (_entity, transform) in world.query_mut::<&mut Transform>() {
                    transform.position.x += 0.1;
                    transform.position.y = (transform.position.x * 0.1).sin() * 2.0;
                }

                audio_system.update(black_box(&mut world), DELTA_TIME);
            });
        });
    }

    group.finish();
}

fn bench_auto_play_processing(c: &mut Criterion) {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    let camera = world.spawn();
    world.add(camera, Transform::default());
    world.add(camera, AudioListener::new());

    // Add entities with auto-play sounds
    for i in 0..50 {
        let entity = world.spawn();
        world.add(
            entity,
            Transform {
                position: Vec3::new(i as f32 * 2.0, 0.0, 0.0),
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
            },
        );

        world.add(entity, Sound::new("autoplay.wav").auto_play().spatial_3d(50.0));
    }

    let mut audio_system = AudioSystem::new().unwrap();

    c.bench_function("auto_play_processing", |b| {
        b.iter(|| {
            audio_system.update(black_box(&mut world), DELTA_TIME);
        });
    });
}

criterion_group!(
    benches,
    bench_audio_system_update,
    bench_listener_update_only,
    bench_emitter_position_updates,
    bench_mixed_spatial_nonspatial,
    bench_world_query_overhead,
    bench_cleanup_finished_sounds,
    bench_doppler_calculation_overhead,
    bench_full_frame_simulation,
    bench_auto_play_processing,
);

criterion_main!(benches);
