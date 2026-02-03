//! Doppler effect benchmarks
//!
//! Measures performance of Doppler calculations to ensure they meet
//! the target of < 0.01ms overhead per entity.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use engine_audio::{AudioListener, AudioSystem, DopplerCalculator, Sound, DEFAULT_SPEED_OF_SOUND};
use engine_core::ecs::World;
use engine_core::math::{Transform, Vec3};
use std::time::Duration;

/// Benchmark Doppler pitch shift calculation
fn bench_doppler_pitch_shift(c: &mut Criterion) {
    let calc = DopplerCalculator::default();

    let listener_pos = Vec3::new(0.0, 0.0, 0.0);
    let listener_vel = Vec3::new(5.0, 0.0, 0.0);
    let emitter_pos = Vec3::new(100.0, 0.0, 0.0);
    let emitter_vel = Vec3::new(-20.0, 0.0, 0.0);

    c.bench_function("doppler_pitch_shift", |b| {
        b.iter(|| {
            calc.calculate_pitch_shift(
                black_box(listener_pos),
                black_box(listener_vel),
                black_box(emitter_pos),
                black_box(emitter_vel),
            )
        });
    });
}

/// Benchmark velocity calculation
fn bench_velocity_calculation(c: &mut Criterion) {
    let old_pos = Vec3::new(0.0, 0.0, 0.0);
    let new_pos = Vec3::new(10.0, 5.0, 3.0);
    let delta_time = 0.016;

    c.bench_function("velocity_calculation", |b| {
        b.iter(|| {
            DopplerCalculator::calculate_velocity(
                black_box(old_pos),
                black_box(new_pos),
                black_box(delta_time),
            )
        });
    });
}

/// Benchmark Doppler calculator creation
fn bench_doppler_calculator_creation(c: &mut Criterion) {
    c.bench_function("doppler_calculator_creation", |b| {
        b.iter(|| {
            let calc = DopplerCalculator::new(DEFAULT_SPEED_OF_SOUND, 1.0);
            black_box(calc)
        });
    });
}

/// Benchmark AudioSystem update with Doppler enabled
fn bench_audio_system_with_doppler(c: &mut Criterion) {
    let mut group = c.benchmark_group("audio_system_doppler_update");
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

                // Create entities with Doppler-enabled sounds
                for i in 0..count {
                    let entity = world.spawn();
                    let mut transform = Transform::default();
                    transform.position = Vec3::new(i as f32 * 2.0, 0.0, 0.0);
                    world.add(entity, transform);
                    world.add(entity, Sound::new("test.wav").spatial_3d(50.0).with_doppler(1.0));
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

/// Benchmark AudioSystem update without Doppler
fn bench_audio_system_no_doppler(c: &mut Criterion) {
    let mut group = c.benchmark_group("audio_system_no_doppler_update");
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

                // Create entities with Doppler disabled
                for i in 0..count {
                    let entity = world.spawn();
                    let mut transform = Transform::default();
                    transform.position = Vec3::new(i as f32 * 2.0, 0.0, 0.0);
                    world.add(entity, transform);
                    world.add(entity, Sound::new("test.wav").spatial_3d(50.0).without_doppler());
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

/// Benchmark Doppler overhead per entity
fn bench_doppler_per_entity_overhead(c: &mut Criterion) {
    let calc = DopplerCalculator::default();

    let listener_pos = Vec3::ZERO;
    let listener_vel = Vec3::ZERO;

    c.bench_function("doppler_per_entity_overhead", |b| {
        let mut entity_index = 0;
        b.iter(|| {
            let pos = Vec3::new(entity_index as f32 * 10.0, 0.0, 0.0);
            let vel = Vec3::new(-20.0, 0.0, 0.0);

            let shift = calc.calculate_pitch_shift(
                black_box(listener_pos),
                black_box(listener_vel),
                black_box(pos),
                black_box(vel),
            );

            entity_index += 1;
            black_box(shift)
        });
    });
}

/// Benchmark position tracking HashMap operations
fn bench_position_tracking(c: &mut Criterion) {
    use std::collections::HashMap;

    #[derive(Debug, Clone, Copy)]
    struct PositionHistory {
        position: Vec3,
        timestamp: f64,
    }

    let mut group = c.benchmark_group("position_tracking");

    // Benchmark insert
    group.bench_function("insert", |b| {
        let mut positions: HashMap<u32, PositionHistory> = HashMap::new();
        let mut entity_id = 0u32;

        b.iter(|| {
            positions.insert(
                entity_id,
                PositionHistory { position: Vec3::new(entity_id as f32, 0.0, 0.0), timestamp: 0.0 },
            );
            entity_id = entity_id.wrapping_add(1);
        });
    });

    // Benchmark lookup
    group.bench_function("lookup", |b| {
        let mut positions: HashMap<u32, PositionHistory> = HashMap::new();
        for i in 0..1000 {
            positions.insert(
                i,
                PositionHistory { position: Vec3::new(i as f32, 0.0, 0.0), timestamp: 0.0 },
            );
        }

        b.iter(|| {
            let entity_id = black_box(500u32);
            black_box(positions.get(&entity_id))
        });
    });

    group.finish();
}

/// Benchmark Doppler with varying scales
fn bench_doppler_scale_variation(c: &mut Criterion) {
    let mut group = c.benchmark_group("doppler_scale_variation");

    let listener_pos = Vec3::ZERO;
    let listener_vel = Vec3::ZERO;
    let emitter_pos = Vec3::new(100.0, 0.0, 0.0);
    let emitter_vel = Vec3::new(-30.0, 0.0, 0.0);

    for scale in [0.0, 0.5, 1.0, 2.0, 5.0].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(scale), scale, |b, &scale_val| {
            let calc = DopplerCalculator::new(DEFAULT_SPEED_OF_SOUND, scale_val);

            b.iter(|| {
                calc.calculate_pitch_shift(
                    black_box(listener_pos),
                    black_box(listener_vel),
                    black_box(emitter_pos),
                    black_box(emitter_vel),
                )
            });
        });
    }

    group.finish();
}

/// Benchmark Doppler with high-speed movement
fn bench_doppler_high_speed(c: &mut Criterion) {
    let calc = DopplerCalculator::default();

    let listener_pos = Vec3::ZERO;
    let listener_vel = Vec3::ZERO;
    let emitter_pos = Vec3::new(1000.0, 0.0, 0.0);

    // Supersonic velocity (Mach 2)
    let emitter_vel = Vec3::new(-686.0, 0.0, 0.0);

    c.bench_function("doppler_supersonic", |b| {
        b.iter(|| {
            calc.calculate_pitch_shift(
                black_box(listener_pos),
                black_box(listener_vel),
                black_box(emitter_pos),
                black_box(emitter_vel),
            )
        });
    });
}

/// Benchmark Doppler with 3D movement
fn bench_doppler_3d_movement(c: &mut Criterion) {
    let calc = DopplerCalculator::default();

    let listener_pos = Vec3::ZERO;
    let listener_vel = Vec3::new(5.0, 3.0, 2.0);
    let emitter_pos = Vec3::new(50.0, 30.0, 20.0);
    let emitter_vel = Vec3::new(-10.0, 5.0, -3.0);

    c.bench_function("doppler_3d_movement", |b| {
        b.iter(|| {
            calc.calculate_pitch_shift(
                black_box(listener_pos),
                black_box(listener_vel),
                black_box(emitter_pos),
                black_box(emitter_vel),
            )
        });
    });
}

/// Benchmark moving listener scenario
fn bench_doppler_moving_listener(c: &mut Criterion) {
    let calc = DopplerCalculator::default();

    let mut group = c.benchmark_group("doppler_moving_listener");

    for speed in [10.0, 50.0, 100.0, 200.0].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(speed), speed, |b, &spd| {
            let listener_pos = Vec3::ZERO;
            let listener_vel = Vec3::new(spd, 0.0, 0.0);
            let emitter_pos = Vec3::new(100.0, 0.0, 0.0);
            let emitter_vel = Vec3::ZERO;

            b.iter(|| {
                calc.calculate_pitch_shift(
                    black_box(listener_pos),
                    black_box(listener_vel),
                    black_box(emitter_pos),
                    black_box(emitter_vel),
                )
            });
        });
    }

    group.finish();
}

/// Benchmark worst-case scenario: many moving sources
fn bench_doppler_worst_case(c: &mut Criterion) {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Sound>();
    world.register::<AudioListener>();

    // Moving listener
    let camera = world.spawn();
    let mut cam_transform = Transform::default();
    cam_transform.position = Vec3::new(0.0, 1.7, 0.0); // Human height
    world.add(camera, cam_transform);
    world.add(camera, AudioListener::new());

    // 100 moving sound sources
    for i in 0..100 {
        let entity = world.spawn();
        let mut transform = Transform::default();
        let angle = (i as f32) * std::f32::consts::PI * 2.0 / 100.0;
        transform.position = Vec3::new(angle.cos() * 50.0, 0.0, angle.sin() * 50.0);
        world.add(entity, transform);
        world.add(entity, Sound::new("test.wav").spatial_3d(100.0).with_doppler(1.0));
    }

    let mut system = AudioSystem::new().unwrap();

    c.bench_function("doppler_worst_case_100_moving", |b| {
        b.iter(|| {
            // Move all sources slightly
            for (entity, transform) in world.query::<&mut Transform>() {
                if world.get::<AudioListener>(entity).is_none() {
                    transform.position.x += 0.1;
                }
            }

            system.update(black_box(&mut world), black_box(0.016));
        });
    });
}

criterion_group!(
    benches,
    bench_doppler_pitch_shift,
    bench_velocity_calculation,
    bench_doppler_calculator_creation,
    bench_audio_system_with_doppler,
    bench_audio_system_no_doppler,
    bench_doppler_per_entity_overhead,
    bench_position_tracking,
    bench_doppler_scale_variation,
    bench_doppler_high_speed,
    bench_doppler_3d_movement,
    bench_doppler_moving_listener,
    bench_doppler_worst_case,
);

criterion_main!(benches);
