//! Performance benchmarks for Phase A.2 - Enhanced Profiling Metrics
//!
//! Validates that metrics collection overhead meets performance targets:
//! - Metrics collection overhead: < 100ns per frame
//! - Frame metrics generation: < 1μs
//! - No measurable impact on physics step performance

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use engine_math::{Quat, Vec3};
use engine_physics::{Collider, PhysicsConfig, PhysicsWorld, RigidBody};

fn bench_metrics_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("metrics/overhead");

    // Benchmark: Physics step WITHOUT metrics
    let mut world_no_metrics = PhysicsWorld::new(PhysicsConfig::default());
    world_no_metrics.add_rigidbody(0, &RigidBody::static_body(), Vec3::ZERO, Quat::IDENTITY);
    world_no_metrics.add_collider(0, &Collider::box_collider(Vec3::new(10.0, 0.5, 10.0)));

    for i in 1..=100 {
        world_no_metrics.add_rigidbody(
            i,
            &RigidBody::dynamic(1.0),
            Vec3::new((i % 10) as f32 * 2.0, (i / 10) as f32 * 2.0, 0.0),
            Quat::IDENTITY,
        );
        world_no_metrics.add_collider(i, &Collider::box_collider(Vec3::ONE));
    }

    group.bench_function("physics_step_no_metrics", |b| {
        b.iter(|| {
            world_no_metrics.step(black_box(1.0 / 60.0));
        });
    });

    // Benchmark: Physics step WITH metrics enabled
    let mut world_with_metrics = PhysicsWorld::new(PhysicsConfig::default());
    world_with_metrics.add_rigidbody(0, &RigidBody::static_body(), Vec3::ZERO, Quat::IDENTITY);
    world_with_metrics.add_collider(0, &Collider::box_collider(Vec3::new(10.0, 0.5, 10.0)));

    for i in 1..=100 {
        world_with_metrics.add_rigidbody(
            i,
            &RigidBody::dynamic(1.0),
            Vec3::new((i % 10) as f32 * 2.0, (i / 10) as f32 * 2.0, 0.0),
            Quat::IDENTITY,
        );
        world_with_metrics.add_collider(i, &Collider::box_collider(Vec3::ONE));
    }

    world_with_metrics.enable_metrics();

    group.bench_function("physics_step_with_metrics", |b| {
        b.iter(|| {
            world_with_metrics.step(black_box(1.0 / 60.0));
        });
    });

    group.finish();
}

fn bench_metrics_collection(c: &mut Criterion) {
    let mut group = c.benchmark_group("metrics/collection");

    // Setup world with metrics enabled
    let mut world = PhysicsWorld::new(PhysicsConfig::default());
    world.enable_metrics();

    world.add_rigidbody(0, &RigidBody::static_body(), Vec3::ZERO, Quat::IDENTITY);
    world.add_collider(0, &Collider::box_collider(Vec3::new(10.0, 0.5, 10.0)));

    for i in 1..=100 {
        world.add_rigidbody(
            i,
            &RigidBody::dynamic(1.0),
            Vec3::new((i % 10) as f32 * 2.0, (i / 10) as f32 * 2.0, 0.0),
            Quat::IDENTITY,
        );
        world.add_collider(i, &Collider::box_collider(Vec3::ONE));
    }

    // Run a few steps to get steady state
    for _ in 0..10 {
        world.step(1.0 / 60.0);
    }

    // Benchmark: last_frame_metrics() call
    group.bench_function("last_frame_metrics", |b| {
        b.iter(|| {
            let _metrics = world.last_frame_metrics();
        });
    });

    group.finish();
}

fn bench_metrics_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("metrics/serialization");

    // Setup world and get metrics
    let mut world = PhysicsWorld::new(PhysicsConfig::default());
    world.enable_metrics();

    world.add_rigidbody(0, &RigidBody::static_body(), Vec3::ZERO, Quat::IDENTITY);
    world.add_collider(0, &Collider::box_collider(Vec3::new(10.0, 0.5, 10.0)));

    for i in 1..=100 {
        world.add_rigidbody(
            i,
            &RigidBody::dynamic(1.0),
            Vec3::new((i % 10) as f32 * 2.0, (i / 10) as f32 * 2.0, 0.0),
            Quat::IDENTITY,
        );
        world.add_collider(i, &Collider::box_collider(Vec3::ONE));
    }

    world.step(1.0 / 60.0);
    let metrics = world.last_frame_metrics().unwrap();

    // Benchmark: JSON serialization
    group.bench_function("json_serialize", |b| {
        b.iter(|| {
            let _json = serde_json::to_string(black_box(&metrics)).unwrap();
        });
    });

    // Benchmark: JSON deserialization
    let json = serde_json::to_string(&metrics).unwrap();
    group.bench_function("json_deserialize", |b| {
        b.iter(|| {
            let _metrics: engine_physics::FrameMetrics =
                serde_json::from_str(black_box(&json)).unwrap();
        });
    });

    group.finish();
}

fn bench_metrics_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("metrics/scaling");

    for body_count in [10, 50, 100, 500, 1000].iter() {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());
        world.enable_metrics();

        world.add_rigidbody(0, &RigidBody::static_body(), Vec3::ZERO, Quat::IDENTITY);
        world.add_collider(0, &Collider::box_collider(Vec3::new(50.0, 0.5, 50.0)));

        for i in 1..=*body_count {
            world.add_rigidbody(
                i,
                &RigidBody::dynamic(1.0),
                Vec3::new((i % 10) as f32 * 2.0, (i / 10) as f32 * 2.0 + 10.0, 0.0),
                Quat::IDENTITY,
            );
            world.add_collider(i, &Collider::box_collider(Vec3::ONE));
        }

        // Run to steady state
        for _ in 0..10 {
            world.step(1.0 / 60.0);
        }

        group.bench_with_input(format!("{}_bodies", body_count), body_count, |b, _| {
            b.iter(|| {
                world.step(black_box(1.0 / 60.0));
                let _metrics = world.last_frame_metrics();
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_metrics_overhead,
    bench_metrics_collection,
    bench_metrics_serialization,
    bench_metrics_scaling,
);
criterion_main!(benches);
