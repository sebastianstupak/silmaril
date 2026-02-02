//! Benchmarks for deterministic physics mode
//!
//! Measures:
//! - Deterministic mode overhead vs normal mode
//! - State hashing performance
//! - Snapshot creation performance
//! - Replay performance

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_math::{Quat, Vec3};
use engine_physics::{
    create_snapshot, hash_physics_state, restore_snapshot, Collider, PhysicsConfig, PhysicsInput,
    PhysicsWorld, ReplayPlayer, ReplayRecorder, RigidBody,
};

fn bench_deterministic_mode_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("deterministic_mode_overhead");

    // Normal mode
    group.bench_function("normal_mode", |b| {
        let config = PhysicsConfig::default();
        let mut world = PhysicsWorld::new(config);

        let rb = RigidBody::dynamic(1.0);
        for i in 0..100 {
            world.add_rigidbody(i, &rb, Vec3::new(i as f32, 10.0, 0.0), Quat::IDENTITY);
            world.add_collider(i, &Collider::sphere(0.5));
        }

        let dt = 1.0 / 60.0;

        b.iter(|| {
            world.step(black_box(dt));
        });
    });

    // Deterministic mode
    group.bench_function("deterministic_mode", |b| {
        let config = PhysicsConfig::default().with_deterministic(true);
        let mut world = PhysicsWorld::new(config);

        let rb = RigidBody::dynamic(1.0);
        for i in 0..100 {
            world.add_rigidbody(i, &rb, Vec3::new(i as f32, 10.0, 0.0), Quat::IDENTITY);
            world.add_collider(i, &Collider::sphere(0.5));
        }

        let dt = 1.0 / 60.0;

        b.iter(|| {
            world.step(black_box(dt));
        });
    });

    group.finish();
}

fn bench_state_hashing(c: &mut Criterion) {
    let mut group = c.benchmark_group("state_hashing");

    for num_entities in [10, 100, 1000] {
        group.throughput(Throughput::Elements(num_entities));

        group.bench_with_input(
            BenchmarkId::from_parameter(num_entities),
            &num_entities,
            |b, &num_entities| {
                let config = PhysicsConfig::default().with_deterministic(true);
                let mut world = PhysicsWorld::new(config);

                let rb = RigidBody::dynamic(1.0);
                for i in 0..num_entities {
                    world.add_rigidbody(i, &rb, Vec3::new(i as f32, 10.0, 0.0), Quat::IDENTITY);
                    world.add_collider(i, &Collider::sphere(0.5));
                }

                b.iter(|| {
                    black_box(hash_physics_state(&world));
                });
            },
        );
    }

    group.finish();
}

fn bench_snapshot_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("snapshot_creation");

    for num_entities in [10, 100, 1000] {
        group.throughput(Throughput::Elements(num_entities));

        group.bench_with_input(
            BenchmarkId::from_parameter(num_entities),
            &num_entities,
            |b, &num_entities| {
                let config = PhysicsConfig::default().with_deterministic(true);
                let mut world = PhysicsWorld::new(config);

                let rb = RigidBody::dynamic(1.0);
                for i in 0..num_entities {
                    world.add_rigidbody(i, &rb, Vec3::new(i as f32, 10.0, 0.0), Quat::IDENTITY);
                    world.add_collider(i, &Collider::sphere(0.5));
                }

                b.iter(|| {
                    black_box(create_snapshot(&world, 0));
                });
            },
        );
    }

    group.finish();
}

fn bench_snapshot_restore(c: &mut Criterion) {
    let mut group = c.benchmark_group("snapshot_restore");

    for num_entities in [10, 100, 1000] {
        group.throughput(Throughput::Elements(num_entities));

        group.bench_with_input(
            BenchmarkId::from_parameter(num_entities),
            &num_entities,
            |b, &num_entities| {
                let config = PhysicsConfig::default().with_deterministic(true);
                let mut world = PhysicsWorld::new(config);

                let rb = RigidBody::dynamic(1.0);
                for i in 0..num_entities {
                    world.add_rigidbody(i, &rb, Vec3::new(i as f32, 10.0, 0.0), Quat::IDENTITY);
                    world.add_collider(i, &Collider::sphere(0.5));
                }

                let snapshot = create_snapshot(&world, 0);

                b.iter(|| {
                    restore_snapshot(&mut world, black_box(&snapshot)).unwrap();
                });
            },
        );
    }

    group.finish();
}

fn bench_replay_recording(c: &mut Criterion) {
    let mut group = c.benchmark_group("replay_recording");

    group.bench_function("record_and_commit", |b| {
        let config = PhysicsConfig::default().with_deterministic(true);
        let mut world = PhysicsWorld::new(config);

        let rb = RigidBody::dynamic(1.0);
        world.add_rigidbody(1, &rb, Vec3::new(0.0, 10.0, 0.0), Quat::IDENTITY);
        world.add_collider(1, &Collider::sphere(0.5));

        let mut recorder = ReplayRecorder::new();
        recorder.record_initial_snapshot(&world);

        let dt = 1.0 / 60.0;

        b.iter(|| {
            recorder.record_input(PhysicsInput::ApplyForce {
                entity_id: 1,
                force: Vec3::new(0.0, 10.0, 0.0),
            });
            world.step(dt);
            recorder.commit_frame(black_box(&world));
        });
    });

    group.finish();
}

fn bench_replay_playback(c: &mut Criterion) {
    let mut group = c.benchmark_group("replay_playback");

    group.bench_function("playback_with_verification", |b| {
        // Set up recording
        let config = PhysicsConfig::default().with_deterministic(true);
        let mut world = PhysicsWorld::new(config.clone());

        let rb = RigidBody::dynamic(1.0);
        world.add_rigidbody(1, &rb, Vec3::new(0.0, 10.0, 0.0), Quat::IDENTITY);
        world.add_collider(1, &Collider::sphere(0.5));

        let mut recorder = ReplayRecorder::new();
        recorder.record_initial_snapshot(&world);

        let dt = 1.0 / 60.0;
        for _ in 0..60 {
            recorder.record_input(PhysicsInput::ApplyForce {
                entity_id: 1,
                force: Vec3::new(0.0, 10.0, 0.0),
            });
            world.step(dt);
            recorder.commit_frame(&world);
        }

        let initial_snapshot = recorder.initial_snapshot().unwrap().clone();
        let frames = recorder.frames().to_vec();

        b.iter(|| {
            let mut replay_world = PhysicsWorld::new(config.clone());
            restore_snapshot(&mut replay_world, &initial_snapshot).unwrap();

            let mut player = ReplayPlayer::new(initial_snapshot.clone(), frames.clone(), true);

            while let Some(inputs) = player.next_frame() {
                for input in inputs {
                    match input {
                        PhysicsInput::ApplyForce { entity_id, force } => {
                            replay_world.apply_force(*entity_id, *force);
                        }
                        _ => {}
                    }
                }
                replay_world.step(dt);
                player.verify_hash(&replay_world).unwrap();
            }
        });
    });

    group.finish();
}

fn bench_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_usage");

    group.bench_function("1000_frames", |b| {
        b.iter(|| {
            let config = PhysicsConfig::default().with_deterministic(true);
            let mut world = PhysicsWorld::new(config);

            let rb = RigidBody::dynamic(1.0);
            world.add_rigidbody(1, &rb, Vec3::ZERO, Quat::IDENTITY);

            let mut recorder = ReplayRecorder::new();
            recorder.record_initial_snapshot(&world);

            let dt = 1.0 / 60.0;
            for _ in 0..1000 {
                recorder.record_input(PhysicsInput::ApplyForce {
                    entity_id: 1,
                    force: Vec3::new(0.0, 10.0, 0.0),
                });
                world.step(dt);
                recorder.commit_frame(&world);
            }

            black_box(recorder.memory_usage())
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_deterministic_mode_overhead,
    bench_state_hashing,
    bench_snapshot_creation,
    bench_snapshot_restore,
    bench_replay_recording,
    bench_replay_playback,
    bench_memory_usage
);

criterion_main!(benches);
