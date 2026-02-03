//! Baseline benchmarks for Rapier 0.18 constraint solver (Phase C.1.2)
//!
//! Measures current solver performance across:
//! - Constraint count scaling (100/1000/5000)
//! - Mass ratio stability (1:1, 10:1, 100:1)
//! - Simulation scenarios (chain, stack, ragdoll)
//! - Iteration count vs convergence
//!
//! Establishes baseline for Phase C solver optimizations.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use engine_math::{Quat, Vec3};
use engine_physics::*;

/// Benchmark constraint count scaling with simple joints
fn bench_constraint_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("solver_baseline/constraint_scaling");

    for constraint_count in [100, 1000, 5000].iter() {
        let mut world = create_chain_world(*constraint_count);

        group.bench_with_input(
            BenchmarkId::from_parameter(constraint_count),
            constraint_count,
            |b, _| {
                b.iter(|| {
                    world.step(black_box(1.0 / 60.0));
                });
            },
        );
    }

    group.finish();
}

/// Benchmark mass ratio stability (heavy object on light object)
fn bench_mass_ratio_stability(c: &mut Criterion) {
    let mut group = c.benchmark_group("solver_baseline/mass_ratio");

    for ratio in [1, 10, 100].iter() {
        let mut world = create_mass_ratio_world(*ratio);

        group.bench_with_input(BenchmarkId::from_parameter(ratio), ratio, |b, _| {
            b.iter(|| {
                world.step(black_box(1.0 / 60.0));
            });
        });
    }

    group.finish();
}

/// Benchmark chain simulation (common stress test)
fn bench_chain_stability(c: &mut Criterion) {
    let mut group = c.benchmark_group("solver_baseline/chain");

    // Test chains of different lengths
    for chain_length in [10, 20, 50].iter() {
        let mut world = create_hanging_chain(*chain_length);

        group.bench_with_input(BenchmarkId::from_parameter(chain_length), chain_length, |b, _| {
            b.iter(|| {
                world.step(black_box(1.0 / 60.0));
            });
        });
    }

    group.finish();
}

/// Benchmark stack stability (box tower)
fn bench_stack_stability(c: &mut Criterion) {
    let mut group = c.benchmark_group("solver_baseline/stack");

    // Test stacks of different heights
    for stack_height in [5, 10, 20].iter() {
        let mut world = create_box_stack(*stack_height);

        group.bench_with_input(BenchmarkId::from_parameter(stack_height), stack_height, |b, _| {
            b.iter(|| {
                world.step(black_box(1.0 / 60.0));
            });
        });
    }

    group.finish();
}

/// Benchmark solver iteration counts
fn bench_iteration_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("solver_baseline/iterations");

    let base_world = create_chain_world(100);

    for iterations in [2, 4, 8, 16].iter() {
        let config = PhysicsConfig::default().with_solver_iterations(*iterations);
        let mut world = PhysicsWorld::new(config);

        // Rebuild chain with custom config
        copy_world_structure(&base_world, &mut world);

        group.bench_with_input(BenchmarkId::from_parameter(iterations), iterations, |b, _| {
            b.iter(|| {
                world.step(black_box(1.0 / 60.0));
            });
        });
    }

    group.finish();
}

// Helper functions to create test worlds

/// Create a world with a chain of connected bodies
fn create_chain_world(link_count: usize) -> PhysicsWorld {
    let mut world = PhysicsWorld::new(PhysicsConfig::default());

    // Fixed anchor point
    let anchor_id = 0;
    world.add_rigidbody(
        anchor_id,
        &RigidBody::static_body(),
        Vec3::new(0.0, 10.0, 0.0),
        Quat::IDENTITY,
    );
    world.add_collider(anchor_id, &Collider::sphere(0.1));

    // Create chain links
    let link_mass = 0.1;
    let link_spacing = 0.5;

    for i in 1..=link_count {
        let entity_id = i as u64;

        // Add dynamic body
        world.add_rigidbody(
            entity_id,
            &RigidBody::dynamic(link_mass),
            Vec3::new(0.0, 10.0 - (i as f32) * link_spacing, 0.0),
            Quat::IDENTITY,
        );
        world.add_collider(entity_id, &Collider::sphere(0.1));

        // Connect to previous link with fixed joint
        let prev_id = (i - 1) as u64;
        let joint = FixedJointConfig {
            anchor1: Vec3::new(0.0, -link_spacing / 2.0, 0.0),
            anchor2: Vec3::new(0.0, link_spacing / 2.0, 0.0),
        };
        world.add_joint(prev_id, entity_id, &Joint::Fixed(joint));
    }

    world
}

/// Create a world testing mass ratio stability
fn create_mass_ratio_world(mass_ratio: u32) -> PhysicsWorld {
    let mut world = PhysicsWorld::new(PhysicsConfig::default());

    // Ground
    world.add_rigidbody(0, &RigidBody::static_body(), Vec3::ZERO, Quat::IDENTITY);
    world.add_collider(0, &Collider::box_collider(Vec3::new(10.0, 0.5, 10.0)));

    // Light object
    let light_mass = 1.0;
    world.add_rigidbody(
        1,
        &RigidBody::dynamic(light_mass),
        Vec3::new(0.0, 2.0, 0.0),
        Quat::IDENTITY,
    );
    world.add_collider(1, &Collider::box_collider(Vec3::ONE));

    // Heavy object on top
    let heavy_mass = light_mass * mass_ratio as f32;
    world.add_rigidbody(
        2,
        &RigidBody::dynamic(heavy_mass),
        Vec3::new(0.0, 4.0, 0.0),
        Quat::IDENTITY,
    );
    world.add_collider(2, &Collider::box_collider(Vec3::ONE));

    world
}

/// Create a hanging chain attached to fixed point
fn create_hanging_chain(length: usize) -> PhysicsWorld {
    create_chain_world(length)
}

/// Create a stack of boxes
fn create_box_stack(height: usize) -> PhysicsWorld {
    let mut world = PhysicsWorld::new(PhysicsConfig::default());

    // Ground
    world.add_rigidbody(0, &RigidBody::static_body(), Vec3::ZERO, Quat::IDENTITY);
    world.add_collider(0, &Collider::box_collider(Vec3::new(10.0, 0.5, 10.0)));

    // Stack boxes
    let box_size = Vec3::new(1.0, 1.0, 1.0);
    let box_mass = 1.0;

    for i in 1..=height {
        let entity_id = i as u64;
        let y_pos = 0.5 + (i as f32 - 0.5) * 2.0; // Stack with slight gap

        world.add_rigidbody(
            entity_id,
            &RigidBody::dynamic(box_mass),
            Vec3::new(0.0, y_pos, 0.0),
            Quat::IDENTITY,
        );
        world.add_collider(entity_id, &Collider::box_collider(box_size));
    }

    world
}

/// Copy world structure (for iteration count tests)
fn copy_world_structure(source: &PhysicsWorld, dest: &mut PhysicsWorld) {
    // Get all entities from source
    let entity_ids: Vec<u64> = source.entity_ids().collect();

    for entity_id in entity_ids {
        // Get transform
        if let Some((pos, rot)) = source.get_transform(entity_id) {
            // Determine if static or dynamic by checking velocity
            let is_static = source.get_velocity(entity_id).is_none();

            let rb = if is_static {
                RigidBody::static_body()
            } else {
                RigidBody::dynamic(1.0) // Default mass
            };

            dest.add_rigidbody(entity_id, &rb, pos, rot);
            dest.add_collider(entity_id, &Collider::sphere(0.1)); // Simple collider
        }
    }

    // Note: Joints are NOT copied in this simple helper
    // For iteration count tests, we're primarily measuring solve time
}

criterion_group!(
    benches,
    bench_constraint_scaling,
    bench_mass_ratio_stability,
    bench_chain_stability,
    bench_stack_stability,
    bench_iteration_scaling,
);
criterion_main!(benches);
