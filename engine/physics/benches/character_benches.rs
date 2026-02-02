//! Benchmarks for character controller performance.
//!
//! Performance targets:
//! - Single character update: < 50µs
//! - Ground detection: < 10µs
//! - 1000 characters: < 50ms total
//!
//! Run with: cargo bench --bench character_benches

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_math::{Quat, Vec3};
use engine_physics::{CharacterController, Collider, PhysicsConfig, PhysicsWorld, RigidBody};

/// Create a physics world with ground and N characters
fn create_world_with_characters(
    num_characters: usize,
) -> (PhysicsWorld, Vec<u64>, Vec<CharacterController>) {
    let mut world = PhysicsWorld::new(PhysicsConfig::default());

    // Create ground plane
    let ground_id = 0;
    world.add_rigidbody(
        ground_id,
        &RigidBody::static_body(),
        Vec3::new(0.0, -0.5, 0.0),
        Quat::IDENTITY,
    );
    world.add_collider(ground_id, &Collider::box_collider(Vec3::new(100.0, 0.5, 100.0)));

    // Create characters in a grid
    let mut entity_ids = Vec::new();
    let mut controllers = Vec::new();

    let grid_size = (num_characters as f32).sqrt().ceil() as i32;
    let spacing = 2.0;

    for i in 0..num_characters {
        let x = (i as i32 % grid_size) as f32 * spacing;
        let z = (i as i32 / grid_size) as f32 * spacing;

        let entity_id = (i + 1) as u64;

        world.add_rigidbody(
            entity_id,
            &RigidBody::kinematic(),
            Vec3::new(x, 1.0, z),
            Quat::IDENTITY,
        );
        world.add_collider(entity_id, &Collider::capsule(0.9, 0.4));

        let mut controller = CharacterController::new(5.0, 10.0);
        controller.set_movement_input(Vec3::new(1.0, 0.0, 1.0)); // Diagonal movement

        entity_ids.push(entity_id);
        controllers.push(controller);
    }

    // Initialize physics
    world.step(1.0 / 60.0);

    (world, entity_ids, controllers)
}

fn bench_single_character_update(c: &mut Criterion) {
    let (mut world, entity_ids, mut controllers) = create_world_with_characters(1);

    c.bench_function("character_update_single", |b| {
        b.iter(|| {
            controllers[0].update(
                black_box(&mut world),
                black_box(entity_ids[0]),
                black_box(1.0 / 60.0),
            );
        });
    });
}

fn bench_ground_detection(c: &mut Criterion) {
    let (mut world, entity_ids, mut controllers) = create_world_with_characters(1);

    c.bench_function("character_ground_check", |b| {
        b.iter(|| {
            // Just update to trigger ground check
            controllers[0].update(
                black_box(&mut world),
                black_box(entity_ids[0]),
                black_box(1.0 / 60.0),
            );
        });
    });
}

fn bench_character_movement_only(c: &mut Criterion) {
    let (mut world, entity_ids, mut controllers) = create_world_with_characters(1);

    c.bench_function("character_movement_calculation", |b| {
        b.iter(|| {
            // Set movement input and update
            controllers[0].set_movement_input(black_box(Vec3::new(1.0, 0.0, 1.0)));
            controllers[0].update(
                black_box(&mut world),
                black_box(entity_ids[0]),
                black_box(1.0 / 60.0),
            );
        });
    });
}

fn bench_character_jump(c: &mut Criterion) {
    let (mut world, entity_ids, mut controllers) = create_world_with_characters(1);

    // Ensure grounded
    controllers[0].update(&mut world, entity_ids[0], 1.0 / 60.0);

    c.bench_function("character_jump", |b| {
        b.iter(|| {
            // Reset grounded state for fair benchmark
            controllers[0].grounded = true;
            black_box(controllers[0].jump());
        });
    });
}

fn bench_multiple_characters(c: &mut Criterion) {
    let mut group = c.benchmark_group("character_scaling");

    for num_chars in [1, 10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*num_chars as u64));

        group.bench_with_input(BenchmarkId::from_parameter(num_chars), num_chars, |b, &num| {
            let (mut world, entity_ids, mut controllers) = create_world_with_characters(num);

            b.iter(|| {
                for i in 0..num {
                    controllers[i].update(
                        black_box(&mut world),
                        black_box(entity_ids[i]),
                        black_box(1.0 / 60.0),
                    );
                }
            });
        });
    }

    group.finish();
}

fn bench_character_with_physics_step(c: &mut Criterion) {
    let mut group = c.benchmark_group("character_with_physics");

    for num_chars in [1, 10, 100].iter() {
        group.throughput(Throughput::Elements(*num_chars as u64));

        group.bench_with_input(BenchmarkId::from_parameter(num_chars), num_chars, |b, &num| {
            let (mut world, entity_ids, mut controllers) = create_world_with_characters(num);

            b.iter(|| {
                // Update all character controllers
                for i in 0..num {
                    controllers[i].update(
                        black_box(&mut world),
                        black_box(entity_ids[i]),
                        black_box(1.0 / 60.0),
                    );
                }

                // Step physics
                world.step(black_box(1.0 / 60.0));
            });
        });
    }

    group.finish();
}

fn bench_input_normalization(c: &mut Criterion) {
    let mut controller = CharacterController::default();

    c.bench_function("character_input_normalization", |b| {
        b.iter(|| {
            controller.set_movement_input(black_box(Vec3::new(100.0, 50.0, 100.0)));
        });
    });
}

fn bench_character_state_queries(c: &mut Criterion) {
    let controller = CharacterController::default();

    c.bench_function("character_state_queries", |b| {
        b.iter(|| {
            black_box(controller.is_grounded());
            black_box(controller.vertical_velocity());
            black_box(controller.movement_input());
            black_box(controller.was_grounded());
        });
    });
}

criterion_group!(
    benches,
    bench_single_character_update,
    bench_ground_detection,
    bench_character_movement_only,
    bench_character_jump,
    bench_multiple_characters,
    bench_character_with_physics_step,
    bench_input_normalization,
    bench_character_state_queries,
);

criterion_main!(benches);
