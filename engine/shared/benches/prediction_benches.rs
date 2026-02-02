//! Benchmarks for client-side prediction
//!
//! Performance targets:
//! - Input buffering: < 1µs per input
//! - State reconciliation: < 100µs
//! - Input replay: < 1ms for 60 inputs
//! - Prediction overhead: < 5% of normal physics step

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_core::ecs::EntityAllocator;
use engine_math::{Quat, Vec3};
use engine_physics::{
    prediction::{InputBuffer, PlayerInput, PredictionSystem},
    Collider, PhysicsConfig, PhysicsWorld, RigidBody,
};

/// Benchmark input buffering performance
fn bench_input_buffering(c: &mut Criterion) {
    let mut group = c.benchmark_group("input_buffering");

    group.bench_function("add_single_input", |b| {
        let mut buffer = InputBuffer::new();
        let mut timestamp = 0u64;

        b.iter(|| {
            buffer.add_input(
                black_box(timestamp),
                black_box(Vec3::new(1.0, 0.0, 0.0)),
                black_box(false),
                black_box(0.016),
            );
            timestamp += 16;
        });
    });

    group.bench_function("add_100_inputs", |b| {
        b.iter(|| {
            let mut buffer = InputBuffer::new();
            for i in 0..100 {
                buffer.add_input(
                    black_box(i * 16),
                    black_box(Vec3::new(1.0, 0.0, 0.0)),
                    black_box(false),
                    black_box(0.016),
                );
            }
        });
    });

    group.finish();
}

/// Benchmark input retrieval performance
fn bench_input_retrieval(c: &mut Criterion) {
    let mut group = c.benchmark_group("input_retrieval");

    // Pre-populate buffer
    let mut buffer = InputBuffer::new();
    for i in 0..100 {
        buffer.add_input(i * 16, Vec3::new(1.0, 0.0, 0.0), false, 0.016);
    }

    group.bench_function("get_inputs_from_start", |b| {
        b.iter(|| {
            let inputs = buffer.get_inputs_from(black_box(0));
            black_box(inputs);
        });
    });

    group.bench_function("get_inputs_from_middle", |b| {
        b.iter(|| {
            let inputs = buffer.get_inputs_from(black_box(50));
            black_box(inputs);
        });
    });

    group.bench_function("remove_before", |b| {
        b.iter(|| {
            // Create fresh buffer for each iteration
            let mut test_buffer = InputBuffer::new();
            for i in 0..100 {
                test_buffer.add_input(i * 16, Vec3::new(1.0, 0.0, 0.0), false, 0.016);
            }
            test_buffer.remove_before(black_box(25));
        });
    });

    group.finish();
}

/// Benchmark state reconciliation
fn bench_reconciliation(c: &mut Criterion) {
    let mut group = c.benchmark_group("reconciliation");

    let mut allocator = EntityAllocator::new();
    let entity = allocator.allocate();

    let config = PhysicsConfig::default();
    let mut physics = PhysicsWorld::new(config);

    let physics_id = 1;
    physics.add_rigidbody(physics_id, &RigidBody::dynamic(1.0), Vec3::ZERO, Quat::IDENTITY);
    physics.add_collider(physics_id, &Collider::sphere(0.5));

    let mut system = PredictionSystem::new();
    system.start_prediction(entity, physics_id, Vec3::ZERO, Quat::IDENTITY, Vec3::ZERO);

    // Add inputs
    for i in 0..60 {
        system.add_input_and_predict(i * 16, Vec3::new(1.0, 0.0, 0.0), false, 0.016, &mut physics);
        physics.step(0.016);
    }

    group.bench_function("reconcile_no_error", |b| {
        b.iter(|| {
            // Create fresh physics world and system for each iteration
            let config = PhysicsConfig::default();
            let mut test_physics = PhysicsWorld::new(config);
            test_physics.add_rigidbody(
                physics_id,
                &RigidBody::dynamic(1.0),
                Vec3::ZERO,
                Quat::IDENTITY,
            );

            let mut test_system = PredictionSystem::new();
            test_system.start_prediction(
                entity,
                physics_id,
                Vec3::ZERO,
                Quat::IDENTITY,
                Vec3::ZERO,
            );

            // Reconcile with exact match (no error)
            test_system.reconcile(
                black_box(55),
                black_box(Vec3::ZERO),
                black_box(Quat::IDENTITY),
                black_box(Vec3::ZERO),
                &mut test_physics,
            );
        });
    });

    group.bench_function("reconcile_small_error", |b| {
        b.iter(|| {
            // Create fresh physics world and system for each iteration
            let config = PhysicsConfig::default();
            let mut test_physics = PhysicsWorld::new(config);
            test_physics.add_rigidbody(
                physics_id,
                &RigidBody::dynamic(1.0),
                Vec3::ZERO,
                Quat::IDENTITY,
            );

            let mut test_system = PredictionSystem::new();
            test_system.start_prediction(
                entity,
                physics_id,
                Vec3::ZERO,
                Quat::IDENTITY,
                Vec3::ZERO,
            );

            // Add inputs to system
            for i in 0..60 {
                test_system.add_input_and_predict(
                    i * 16,
                    Vec3::new(1.0, 0.0, 0.0),
                    false,
                    0.016,
                    &mut test_physics,
                );
                test_physics.step(0.016);
            }

            // Reconcile with small error
            test_system.reconcile(
                black_box(50),
                black_box(Vec3::new(0.05, 0.0, 0.0)),
                black_box(Quat::IDENTITY),
                black_box(Vec3::ZERO),
                &mut test_physics,
            );
        });
    });

    group.finish();
}

/// Benchmark input replay performance
fn bench_input_replay(c: &mut Criterion) {
    let mut group = c.benchmark_group("input_replay");
    group.throughput(Throughput::Elements(1));

    let mut allocator = EntityAllocator::new();
    let entity = allocator.allocate();

    let config = PhysicsConfig::default();

    // Benchmark different numbers of replayed inputs
    for input_count in [10, 30, 60, 120] {
        group.bench_with_input(
            BenchmarkId::from_parameter(input_count),
            &input_count,
            |b, &count| {
                let mut physics = PhysicsWorld::new(config.clone());
                let physics_id = 1;
                physics.add_rigidbody(
                    physics_id,
                    &RigidBody::dynamic(1.0),
                    Vec3::ZERO,
                    Quat::IDENTITY,
                );
                physics.add_collider(physics_id, &Collider::sphere(0.5));

                let mut system = PredictionSystem::new();
                system.start_prediction(entity, physics_id, Vec3::ZERO, Quat::IDENTITY, Vec3::ZERO);

                // Add inputs
                for i in 0..count {
                    system.add_input_and_predict(
                        i * 16,
                        Vec3::new(1.0, 0.0, 0.0),
                        false,
                        0.016,
                        &mut physics,
                    );
                    physics.step(0.016);
                }

                b.iter(|| {
                    // Trigger replay from middle
                    system.reconcile(
                        black_box((count / 2) as u32),
                        black_box(Vec3::new(0.1, 0.0, 0.0)),
                        black_box(Quat::IDENTITY),
                        black_box(Vec3::ZERO),
                        &mut physics,
                    );
                });
            },
        );
    }

    group.finish();
}

/// Benchmark prediction overhead vs normal physics
fn bench_prediction_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("prediction_overhead");

    let config = PhysicsConfig::default();
    let mut allocator = EntityAllocator::new();
    let entity = allocator.allocate();

    // Baseline: normal physics step
    group.bench_function("physics_step_baseline", |b| {
        let mut physics = PhysicsWorld::new(config.clone());
        let physics_id = 1;
        physics.add_rigidbody(physics_id, &RigidBody::dynamic(1.0), Vec3::ZERO, Quat::IDENTITY);
        physics.add_collider(physics_id, &Collider::sphere(0.5));

        b.iter(|| {
            physics.apply_force(black_box(physics_id), black_box(Vec3::new(10.0, 0.0, 0.0)));
            physics.step(black_box(0.016));
        });
    });

    // With prediction
    group.bench_function("physics_step_with_prediction", |b| {
        let mut physics = PhysicsWorld::new(config.clone());
        let physics_id = 1;
        physics.add_rigidbody(physics_id, &RigidBody::dynamic(1.0), Vec3::ZERO, Quat::IDENTITY);
        physics.add_collider(physics_id, &Collider::sphere(0.5));

        let mut system = PredictionSystem::new();
        system.start_prediction(entity, physics_id, Vec3::ZERO, Quat::IDENTITY, Vec3::ZERO);

        b.iter(|| {
            system.add_input_and_predict(
                black_box(0),
                black_box(Vec3::new(1.0, 0.0, 0.0)),
                black_box(false),
                black_box(0.016),
                &mut physics,
            );
            physics.step(black_box(0.016));
        });
    });

    group.finish();
}

/// Benchmark error smoothing
fn bench_error_smoothing(c: &mut Criterion) {
    let mut group = c.benchmark_group("error_smoothing");

    let mut allocator = EntityAllocator::new();
    let entity = allocator.allocate();

    let mut world = engine_core::ecs::World::new();
    world.register::<engine_math::Transform>();
    world.add(entity, engine_math::Transform::new(Vec3::ZERO, Quat::IDENTITY, Vec3::ONE));

    let mut system = PredictionSystem::new();
    system.start_prediction(entity, 1, Vec3::ZERO, Quat::IDENTITY, Vec3::ZERO);

    // Create error
    if let Some(state) = system.predicted_state() {
        // Note: predicted_state() returns immutable reference, so we can't modify it directly
        // This benchmark might need to be redesigned or removed
        let _ = state; // Suppress unused variable warning
    }

    group.bench_function("apply_error_smoothing", |b| {
        b.iter(|| {
            system.apply_error_smoothing(black_box(&mut world), black_box(0.016));
        });
    });

    group.finish();
}

/// Benchmark player input serialization (for network send)
fn bench_input_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("input_serialization");

    let input = PlayerInput::new(42, 1000, Vec3::new(1.0, 0.5, -0.3), true, 0.016);

    group.bench_function("serialize_input", |b| {
        b.iter(|| {
            let serialized = bincode::serialize(&black_box(&input)).unwrap();
            black_box(serialized);
        });
    });

    let serialized = bincode::serialize(&input).unwrap();

    group.bench_function("deserialize_input", |b| {
        b.iter(|| {
            let deserialized: PlayerInput = bincode::deserialize(&black_box(&serialized)).unwrap();
            black_box(deserialized);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_input_buffering,
    bench_input_retrieval,
    bench_reconciliation,
    bench_input_replay,
    bench_prediction_overhead,
    bench_error_smoothing,
    bench_input_serialization,
);
criterion_main!(benches);
