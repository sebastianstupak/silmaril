//! Advanced Physics Benchmarks
//!
//! Comprehensive benchmarks to identify optimization opportunities:
//! - Memory allocation patterns
//! - Cache performance
//! - Batch raycast scaling (1000+ rays)
//! - Advanced joint scenarios
//! - Collision detection overhead

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_math::{Quat, Vec3};
use engine_physics::{Collider, JointBuilder, PhysicsConfig, PhysicsWorld, RigidBody};

/// Benchmark: Batch Raycast Scaling (1-1000 rays)
///
/// Measures raycast performance at various batch sizes.
/// AAA Target: <1ms for 1000 raycasts
fn bench_batch_raycast_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_raycast_scaling");

    // Create world with ground and obstacles
    let mut world = PhysicsWorld::new(PhysicsConfig::default());

    // Ground
    world.add_rigidbody(0, &RigidBody::static_body(), Vec3::new(0.0, -1.0, 0.0), Quat::IDENTITY);
    world.add_collider(0, &Collider::box_collider(Vec3::new(100.0, 1.0, 100.0)));

    // Add 100 obstacles in a grid
    for x in 0..10 {
        for z in 0..10 {
            let id = (x * 10 + z + 1) as u64;
            world.add_rigidbody(
                id,
                &RigidBody::static_body(),
                Vec3::new(x as f32 * 5.0 - 25.0, 2.0, z as f32 * 5.0 - 25.0),
                Quat::IDENTITY,
            );
            world.add_collider(id, &Collider::box_collider(Vec3::new(1.0, 1.0, 1.0)));
        }
    }

    world.step(0.0); // Update query pipeline

    // Benchmark different batch sizes
    for &count in &[1, 10, 100, 500, 1000] {
        group.throughput(Throughput::Elements(count));

        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |bench, &ray_count| {
            bench.iter(|| {
                let mut hit_count = 0;

                for i in 0..ray_count {
                    let angle = (i as f32 / ray_count as f32) * 2.0 * std::f32::consts::PI;
                    let dir = Vec3::new(angle.cos(), -0.5, angle.sin()).normalize();

                    if world.raycast(Vec3::new(0.0, 10.0, 0.0), dir, 50.0).is_some() {
                        hit_count += 1;
                    }
                }

                black_box(hit_count);
            });
        });
    }

    group.finish();
}

/// Benchmark: Memory Allocation Patterns
///
/// Measures allocation overhead when creating/destroying physics objects.
/// Identifies potential for object pooling or custom allocators.
fn bench_memory_allocation_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_allocation");

    group.bench_function("create_destroy_rigidbody", |bench| {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());
        let mut next_id = 1u64;

        bench.iter(|| {
            // Create rigidbody
            let id = next_id;
            next_id += 1;

            let rb = RigidBody::dynamic(1.0);
            world.add_rigidbody(id, &rb, Vec3::new(0.0, 5.0, 0.0), Quat::IDENTITY);
            world.add_collider(id, &Collider::sphere(0.5));

            // Immediately destroy
            world.remove_rigidbody(id);
        });
    });

    group.bench_function("create_destroy_100_batch", |bench| {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());

        bench.iter(|| {
            // Create 100 rigidbodies
            for i in 0..100 {
                let rb = RigidBody::dynamic(1.0);
                world.add_rigidbody(1000 + i, &rb, Vec3::new(i as f32, 5.0, 0.0), Quat::IDENTITY);
                world.add_collider(1000 + i, &Collider::sphere(0.5));
            }

            // Destroy all 100
            for i in 0..100 {
                world.remove_rigidbody(1000 + i);
            }
        });
    });

    group.finish();
}

/// Benchmark: Advanced Joint Scenarios
///
/// Tests complex joint configurations that appear in real games:
/// - Ragdolls (multiple joints)
/// - Chains (connected joints)
/// - Motors with high stiffness
fn bench_advanced_joint_scenarios(c: &mut Criterion) {
    let mut group = c.benchmark_group("advanced_joints");

    // Benchmark: Joint Chain (10 links)
    group.bench_function("joint_chain_10_links", |bench| {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());

        // Create chain of 10 connected bodies
        for i in 0..10 {
            let id = i as u64;
            let rb = if i == 0 {
                RigidBody::static_body() // First body is fixed
            } else {
                RigidBody::dynamic(1.0)
            };

            world.add_rigidbody(id, &rb, Vec3::new(i as f32 * 2.0, 5.0, 0.0), Quat::IDENTITY);
            world.add_collider(id, &Collider::box_collider(Vec3::new(0.5, 0.5, 0.5)));

            // Connect to previous body
            if i > 0 {
                let joint = JointBuilder::revolute()
                    .axis(Vec3::Z)
                    .anchor1(Vec3::new(1.0, 0.0, 0.0))
                    .anchor2(Vec3::new(-1.0, 0.0, 0.0))
                    .build();

                world.add_joint((i - 1) as u64, i as u64, &joint);
            }
        }

        bench.iter(|| {
            world.step(black_box(1.0 / 60.0));
        });
    });

    // Benchmark: Ragdoll Simulation (14 joints, typical humanoid)
    group.bench_function("ragdoll_14_joints", |bench| {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());

        // Create simplified ragdoll
        // Head
        world.add_rigidbody(0, &RigidBody::dynamic(1.0), Vec3::new(0.0, 7.0, 0.0), Quat::IDENTITY);
        world.add_collider(0, &Collider::sphere(0.5));

        // Torso
        world.add_rigidbody(1, &RigidBody::dynamic(5.0), Vec3::new(0.0, 5.0, 0.0), Quat::IDENTITY);
        world.add_collider(1, &Collider::box_collider(Vec3::new(1.0, 1.5, 0.5)));

        // Connect head to torso
        let neck = JointBuilder::spherical()
            .anchor1(Vec3::new(0.0, -0.5, 0.0))
            .anchor2(Vec3::new(0.0, 1.5, 0.0))
            .build();
        world.add_joint(0, 1, &neck);

        // Arms (4 segments = 4 bodies + 4 joints)
        for side in [-1.0, 1.0] {
            // Upper arm
            let upper_id = if side < 0.0 { 2 } else { 4 };
            world.add_rigidbody(
                upper_id,
                &RigidBody::dynamic(1.0),
                Vec3::new(side * 1.5, 5.5, 0.0),
                Quat::IDENTITY,
            );
            world.add_collider(upper_id, &Collider::box_collider(Vec3::new(0.3, 0.8, 0.3)));

            // Lower arm
            let lower_id = upper_id + 1;
            world.add_rigidbody(
                lower_id,
                &RigidBody::dynamic(0.8),
                Vec3::new(side * 1.5, 4.0, 0.0),
                Quat::IDENTITY,
            );
            world.add_collider(lower_id, &Collider::box_collider(Vec3::new(0.3, 0.8, 0.3)));

            // Shoulder joint
            let shoulder = JointBuilder::revolute()
                .axis(Vec3::Z)
                .anchor1(Vec3::new(side * 1.0, 1.0, 0.0))
                .anchor2(Vec3::new(0.0, 0.8, 0.0))
                .build();
            world.add_joint(1, upper_id, &shoulder);

            // Elbow joint
            let elbow = JointBuilder::revolute()
                .axis(Vec3::Z)
                .anchor1(Vec3::new(0.0, -0.8, 0.0))
                .anchor2(Vec3::new(0.0, 0.8, 0.0))
                .build();
            world.add_joint(upper_id, lower_id, &elbow);
        }

        // Legs (same pattern as arms)
        for side in [-1.0, 1.0] {
            let upper_id = if side < 0.0 { 6 } else { 8 };
            world.add_rigidbody(
                upper_id,
                &RigidBody::dynamic(2.0),
                Vec3::new(side * 0.7, 3.0, 0.0),
                Quat::IDENTITY,
            );
            world.add_collider(upper_id, &Collider::box_collider(Vec3::new(0.4, 1.0, 0.4)));

            let lower_id = upper_id + 1;
            world.add_rigidbody(
                lower_id,
                &RigidBody::dynamic(1.5),
                Vec3::new(side * 0.7, 1.0, 0.0),
                Quat::IDENTITY,
            );
            world.add_collider(lower_id, &Collider::box_collider(Vec3::new(0.4, 1.0, 0.4)));

            // Hip joint
            let hip = JointBuilder::revolute()
                .axis(Vec3::Z)
                .anchor1(Vec3::new(side * 0.7, -1.5, 0.0))
                .anchor2(Vec3::new(0.0, 1.0, 0.0))
                .build();
            world.add_joint(1, upper_id, &hip);

            // Knee joint
            let knee = JointBuilder::revolute()
                .axis(Vec3::Z)
                .anchor1(Vec3::new(0.0, -1.0, 0.0))
                .anchor2(Vec3::new(0.0, 1.0, 0.0))
                .build();
            world.add_joint(upper_id, lower_id, &knee);
        }

        bench.iter(|| {
            world.step(black_box(1.0 / 60.0));
        });
    });

    group.finish();
}

/// Benchmark: Collision Detection Overhead
///
/// Measures pure collision detection cost (broad phase + narrow phase).
/// Helps identify if collision detection is the bottleneck.
fn bench_collision_detection_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("collision_detection");

    // Benchmark: Dense grid (many potential collisions)
    group.bench_function("dense_grid_100_bodies", |bench| {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());

        // Create 100 bodies in a 10x10 grid (very close together)
        for x in 0..10 {
            for z in 0..10 {
                let id = (x * 10 + z) as u64;
                let rb = RigidBody::dynamic(1.0);
                world.add_rigidbody(
                    id,
                    &rb,
                    Vec3::new(x as f32 * 1.2, 5.0, z as f32 * 1.2),
                    Quat::IDENTITY,
                );
                world.add_collider(id, &Collider::sphere(0.5));
            }
        }

        bench.iter(|| {
            world.step(black_box(1.0 / 60.0));
        });
    });

    // Benchmark: Sparse distribution (few collisions)
    group.bench_function("sparse_distribution_100_bodies", |bench| {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());

        // Create 100 bodies far apart (minimal collisions)
        for i in 0..100 {
            let rb = RigidBody::dynamic(1.0);
            world.add_rigidbody(i, &rb, Vec3::new(i as f32 * 10.0, 5.0, 0.0), Quat::IDENTITY);
            world.add_collider(i, &Collider::sphere(0.5));
        }

        bench.iter(|| {
            world.step(black_box(1.0 / 60.0));
        });
    });

    group.finish();
}

/// Benchmark: Physics Step Breakdown
///
/// Measures individual phases of physics step to identify bottlenecks:
/// - Broad phase
/// - Narrow phase
/// - Constraint solving
/// - Integration
fn bench_physics_step_breakdown(c: &mut Criterion) {
    let mut group = c.benchmark_group("physics_step_breakdown");

    // Create representative world (1000 bodies, some dynamic, some static)
    let mut world = PhysicsWorld::new(PhysicsConfig::default());

    // Ground
    world.add_rigidbody(0, &RigidBody::static_body(), Vec3::new(0.0, -1.0, 0.0), Quat::IDENTITY);
    world.add_collider(0, &Collider::box_collider(Vec3::new(100.0, 1.0, 100.0)));

    // 999 dynamic bodies
    for i in 1..1000 {
        let rb = RigidBody::dynamic(1.0);
        world.add_rigidbody(
            i,
            &rb,
            Vec3::new((i % 32) as f32 * 2.0, 5.0 + (i / 32) as f32, ((i / 32) % 32) as f32 * 2.0),
            Quat::IDENTITY,
        );
        world.add_collider(i, &Collider::sphere(0.5));
    }

    group.bench_function("full_step_1000_bodies", |bench| {
        bench.iter(|| {
            world.step(black_box(1.0 / 60.0));
        });
    });

    // Note: Individual phase benchmarking would require exposing internal APIs
    // For now, we measure the full step and use profiling tools for breakdown

    group.finish();
}

criterion_group!(
    benches,
    bench_batch_raycast_scaling,
    bench_memory_allocation_patterns,
    bench_advanced_joint_scenarios,
    bench_collision_detection_overhead,
    bench_physics_step_breakdown,
);
criterion_main!(benches);
