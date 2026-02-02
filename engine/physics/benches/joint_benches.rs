//! Joint benchmarks
//!
//! Measures performance of joint creation, removal, and constraint solving.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_math::{Quat, Vec3};
use engine_physics::{Collider, JointBuilder, PhysicsConfig, PhysicsWorld, RigidBody};

/// Benchmark joint creation
fn bench_joint_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("joint_creation");

    // Fixed joint creation
    group.bench_function("fixed_joint_create", |b| {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());

        world.add_rigidbody(1, &RigidBody::static_body(), Vec3::ZERO, Quat::IDENTITY);
        world.add_rigidbody(2, &RigidBody::dynamic(1.0), Vec3::new(1.0, 0.0, 0.0), Quat::IDENTITY);

        b.iter(|| {
            let joint = JointBuilder::fixed().build();
            let handle = world.add_joint(1, 2, &joint);
            black_box(handle);
            if let Some(h) = handle {
                world.remove_joint(h);
            }
        });
    });

    // Revolute joint creation
    group.bench_function("revolute_joint_create", |b| {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());

        world.add_rigidbody(1, &RigidBody::static_body(), Vec3::ZERO, Quat::IDENTITY);
        world.add_rigidbody(2, &RigidBody::dynamic(1.0), Vec3::new(1.0, 0.0, 0.0), Quat::IDENTITY);

        b.iter(|| {
            let joint = JointBuilder::revolute().axis(Vec3::Y).build();
            let handle = world.add_joint(1, 2, &joint);
            black_box(handle);
            if let Some(h) = handle {
                world.remove_joint(h);
            }
        });
    });

    // Prismatic joint creation
    group.bench_function("prismatic_joint_create", |b| {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());

        world.add_rigidbody(1, &RigidBody::static_body(), Vec3::ZERO, Quat::IDENTITY);
        world.add_rigidbody(2, &RigidBody::dynamic(1.0), Vec3::new(1.0, 0.0, 0.0), Quat::IDENTITY);

        b.iter(|| {
            let joint = JointBuilder::prismatic().axis(Vec3::Y).build();
            let handle = world.add_joint(1, 2, &joint);
            black_box(handle);
            if let Some(h) = handle {
                world.remove_joint(h);
            }
        });
    });

    // Spherical joint creation
    group.bench_function("spherical_joint_create", |b| {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());

        world.add_rigidbody(1, &RigidBody::static_body(), Vec3::ZERO, Quat::IDENTITY);
        world.add_rigidbody(2, &RigidBody::dynamic(1.0), Vec3::new(1.0, 0.0, 0.0), Quat::IDENTITY);

        b.iter(|| {
            let joint = JointBuilder::spherical().build();
            let handle = world.add_joint(1, 2, &joint);
            black_box(handle);
            if let Some(h) = handle {
                world.remove_joint(h);
            }
        });
    });

    group.finish();
}

/// Benchmark joint removal
fn bench_joint_removal(c: &mut Criterion) {
    c.bench_function("joint_removal", |b| {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());

        world.add_rigidbody(1, &RigidBody::static_body(), Vec3::ZERO, Quat::IDENTITY);
        world.add_rigidbody(2, &RigidBody::dynamic(1.0), Vec3::new(1.0, 0.0, 0.0), Quat::IDENTITY);

        b.iter(|| {
            let joint = JointBuilder::revolute().axis(Vec3::Y).build();
            let handle = world.add_joint(1, 2, &joint).unwrap();

            let start = std::time::Instant::now();
            world.remove_joint(handle);
            black_box(start.elapsed());
        });
    });
}

/// Benchmark physics step with varying joint counts
fn bench_physics_step_with_joints(c: &mut Criterion) {
    let mut group = c.benchmark_group("physics_step_with_joints");

    for joint_count in [10, 50, 100, 500, 1000].iter() {
        group.throughput(Throughput::Elements(*joint_count as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(joint_count),
            joint_count,
            |b, &count| {
                let mut world = PhysicsWorld::new(PhysicsConfig::default());

                // Create chain of bodies connected by revolute joints
                for i in 0..count {
                    let anchor_id = i * 2;
                    let body_id = i * 2 + 1;

                    world.add_rigidbody(
                        anchor_id,
                        &RigidBody::static_body(),
                        Vec3::new(0.0, (count - i) as f32, 0.0),
                        Quat::IDENTITY,
                    );
                    world.add_collider(anchor_id, &Collider::sphere(0.25));

                    world.add_rigidbody(
                        body_id,
                        &RigidBody::dynamic(0.5),
                        Vec3::new(0.0, (count - i - 1) as f32, 0.0),
                        Quat::IDENTITY,
                    );
                    world.add_collider(body_id, &Collider::sphere(0.25));

                    let joint = JointBuilder::spherical()
                        .anchor1(Vec3::new(0.0, -0.5, 0.0))
                        .anchor2(Vec3::new(0.0, 0.5, 0.0))
                        .build();
                    world.add_joint(anchor_id, body_id, &joint);
                }

                b.iter(|| {
                    world.step(black_box(1.0 / 60.0));
                });
            },
        );
    }

    group.finish();
}

/// Benchmark joint constraint solving overhead
fn bench_joint_solving_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("joint_solving_overhead");

    // Baseline: physics step with no joints
    group.bench_function("no_joints", |b| {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());

        // Create 100 dynamic bodies
        for i in 0..100 {
            world.add_rigidbody(
                i,
                &RigidBody::dynamic(1.0),
                Vec3::new((i % 10) as f32, (i / 10) as f32, 0.0),
                Quat::IDENTITY,
            );
            world.add_collider(i, &Collider::sphere(0.25));
        }

        b.iter(|| {
            world.step(black_box(1.0 / 60.0));
        });
    });

    // With 100 joints
    group.bench_function("100_joints", |b| {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());

        // Create pairs of bodies with joints
        for i in 0..100 {
            let body1_id = i * 2;
            let body2_id = i * 2 + 1;

            world.add_rigidbody(
                body1_id,
                &RigidBody::static_body(),
                Vec3::new((i % 10) as f32 * 2.0, (i / 10) as f32, 0.0),
                Quat::IDENTITY,
            );
            world.add_collider(body1_id, &Collider::sphere(0.25));

            world.add_rigidbody(
                body2_id,
                &RigidBody::dynamic(1.0),
                Vec3::new((i % 10) as f32 * 2.0 + 1.0, (i / 10) as f32, 0.0),
                Quat::IDENTITY,
            );
            world.add_collider(body2_id, &Collider::sphere(0.25));

            let joint = JointBuilder::revolute().axis(Vec3::Y).build();
            world.add_joint(body1_id, body2_id, &joint);
        }

        b.iter(|| {
            world.step(black_box(1.0 / 60.0));
        });
    });

    // With 1000 joints
    group.bench_function("1000_joints", |b| {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());

        // Create pairs of bodies with joints
        for i in 0..1000 {
            let body1_id = i * 2;
            let body2_id = i * 2 + 1;

            world.add_rigidbody(
                body1_id,
                &RigidBody::static_body(),
                Vec3::new((i % 10) as f32 * 2.0, ((i / 10) % 10) as f32, (i / 100) as f32),
                Quat::IDENTITY,
            );
            world.add_collider(body1_id, &Collider::sphere(0.25));

            world.add_rigidbody(
                body2_id,
                &RigidBody::dynamic(1.0),
                Vec3::new((i % 10) as f32 * 2.0 + 1.0, ((i / 10) % 10) as f32, (i / 100) as f32),
                Quat::IDENTITY,
            );
            world.add_collider(body2_id, &Collider::sphere(0.25));

            let joint = JointBuilder::revolute().axis(Vec3::Y).build();
            world.add_joint(body1_id, body2_id, &joint);
        }

        b.iter(|| {
            world.step(black_box(1.0 / 60.0));
        });
    });

    group.finish();
}

/// Benchmark different joint types in simulation
fn bench_joint_types_simulation(c: &mut Criterion) {
    let mut group = c.benchmark_group("joint_types_simulation");

    // Fixed joints
    group.bench_function("fixed_joints_100", |b| {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());

        for i in 0..100 {
            world.add_rigidbody(
                i * 2,
                &RigidBody::static_body(),
                Vec3::new(i as f32, 0.0, 0.0),
                Quat::IDENTITY,
            );
            world.add_rigidbody(
                i * 2 + 1,
                &RigidBody::dynamic(1.0),
                Vec3::new(i as f32, 1.0, 0.0),
                Quat::IDENTITY,
            );
            world.add_collider(i * 2, &Collider::sphere(0.25));
            world.add_collider(i * 2 + 1, &Collider::sphere(0.25));

            let joint = JointBuilder::fixed().build();
            world.add_joint(i * 2, i * 2 + 1, &joint);
        }

        b.iter(|| {
            world.step(black_box(1.0 / 60.0));
        });
    });

    // Revolute joints
    group.bench_function("revolute_joints_100", |b| {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());

        for i in 0..100 {
            world.add_rigidbody(
                i * 2,
                &RigidBody::static_body(),
                Vec3::new(i as f32, 0.0, 0.0),
                Quat::IDENTITY,
            );
            world.add_rigidbody(
                i * 2 + 1,
                &RigidBody::dynamic(1.0),
                Vec3::new(i as f32, 1.0, 0.0),
                Quat::IDENTITY,
            );
            world.add_collider(i * 2, &Collider::sphere(0.25));
            world.add_collider(i * 2 + 1, &Collider::sphere(0.25));

            let joint = JointBuilder::revolute().axis(Vec3::Y).build();
            world.add_joint(i * 2, i * 2 + 1, &joint);
        }

        b.iter(|| {
            world.step(black_box(1.0 / 60.0));
        });
    });

    // Spherical joints
    group.bench_function("spherical_joints_100", |b| {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());

        for i in 0..100 {
            world.add_rigidbody(
                i * 2,
                &RigidBody::static_body(),
                Vec3::new(i as f32, 0.0, 0.0),
                Quat::IDENTITY,
            );
            world.add_rigidbody(
                i * 2 + 1,
                &RigidBody::dynamic(1.0),
                Vec3::new(i as f32, 1.0, 0.0),
                Quat::IDENTITY,
            );
            world.add_collider(i * 2, &Collider::sphere(0.25));
            world.add_collider(i * 2 + 1, &Collider::sphere(0.25));

            let joint = JointBuilder::spherical().build();
            world.add_joint(i * 2, i * 2 + 1, &joint);
        }

        b.iter(|| {
            world.step(black_box(1.0 / 60.0));
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_joint_creation,
    bench_joint_removal,
    bench_physics_step_with_joints,
    bench_joint_solving_overhead,
    bench_joint_types_simulation
);
criterion_main!(benches);
