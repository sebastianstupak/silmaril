//! Raycast performance benchmarks
//!
//! Tests raycast performance scaling with scene complexity.
//! Performance targets:
//! - Single raycast: < 10µs
//! - 100 raycasts: < 1ms
//! - Trigger detection: < 100µs per pair

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use engine_math::{Quat, Vec3};
use engine_physics::{Collider, PhysicsConfig, PhysicsWorld, RigidBody};

fn create_test_scene(world: &mut PhysicsWorld, object_count: usize) {
    // Create ground plane
    let ground = 0u64;
    let rb = RigidBody::static_body();
    world.add_rigidbody(ground, &rb, Vec3::new(0.0, -0.5, 0.0), Quat::IDENTITY);

    let ground_collider = Collider::box_collider(Vec3::new(100.0, 0.5, 100.0));
    world.add_collider(ground, &ground_collider);

    // Create grid of objects
    let grid_size = (object_count as f32).sqrt().ceil() as usize;
    let spacing = 5.0;

    for i in 0..object_count {
        let entity_id = (i + 1) as u64;
        let x = (i % grid_size) as f32 * spacing - (grid_size as f32 * spacing / 2.0);
        let z = (i / grid_size) as f32 * spacing - (grid_size as f32 * spacing / 2.0);
        let y = 2.0;

        let rb = RigidBody::static_body();
        world.add_rigidbody(entity_id, &rb, Vec3::new(x, y, z), Quat::IDENTITY);

        let collider = Collider::box_collider(Vec3::new(1.0, 1.0, 1.0));
        world.add_collider(entity_id, &collider);
    }

    // Update query pipeline
    world.step(0.0);
}

fn bench_raycast_single(c: &mut Criterion) {
    let mut group = c.benchmark_group("raycast_single");

    for object_count in [10, 50, 100, 500, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(object_count),
            object_count,
            |b, &count| {
                let config = PhysicsConfig::default();
                let mut world = PhysicsWorld::new(config);
                create_test_scene(&mut world, count);

                let origin = Vec3::new(0.0, 10.0, 0.0);
                let direction = Vec3::new(0.0, -1.0, 0.0);
                let max_distance = 20.0;

                b.iter(|| {
                    black_box(world.raycast(
                        black_box(origin),
                        black_box(direction),
                        black_box(max_distance),
                    ));
                });
            },
        );
    }

    group.finish();
}

fn bench_raycast_all(c: &mut Criterion) {
    let mut group = c.benchmark_group("raycast_all");

    for object_count in [10, 50, 100, 500].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(object_count),
            object_count,
            |b, &count| {
                let config = PhysicsConfig::default();
                let mut world = PhysicsWorld::new(config);
                create_test_scene(&mut world, count);

                let origin = Vec3::new(0.0, 10.0, 0.0);
                let direction = Vec3::new(0.0, -1.0, 0.0);
                let max_distance = 20.0;

                b.iter(|| {
                    black_box(world.raycast_all(
                        black_box(origin),
                        black_box(direction),
                        black_box(max_distance),
                    ));
                });
            },
        );
    }

    group.finish();
}

fn bench_raycast_batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("raycast_batch");
    group.sample_size(50);

    for ray_count in [10, 50, 100, 200].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(ray_count),
            ray_count,
            |b, &count| {
                let config = PhysicsConfig::default();
                let mut world = PhysicsWorld::new(config);
                create_test_scene(&mut world, 100);

                // Generate random ray directions
                let rays: Vec<(Vec3, Vec3)> = (0..count)
                    .map(|i| {
                        let angle = (i as f32) * std::f32::consts::TAU / (count as f32);
                        let dir = Vec3::new(angle.cos(), -1.0, angle.sin()).normalize();
                        (Vec3::new(0.0, 10.0, 0.0), dir)
                    })
                    .collect();

                let max_distance = 20.0;

                b.iter(|| {
                    for (origin, direction) in &rays {
                        black_box(world.raycast(
                            black_box(*origin),
                            black_box(*direction),
                            black_box(max_distance),
                        ));
                    }
                });
            },
        );
    }

    group.finish();
}

fn bench_trigger_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("trigger_detection");

    for trigger_count in [1, 5, 10, 20].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(trigger_count),
            trigger_count,
            |b, &count| {
                let config = PhysicsConfig::default();
                let mut world = PhysicsWorld::new(config);

                // Create triggers
                for i in 0..count {
                    let trigger = i as u64;
                    let rb = RigidBody::static_body();
                    world.add_rigidbody(
                        trigger,
                        &rb,
                        Vec3::new((i as f32) * 3.0, 0.0, 0.0),
                        Quat::IDENTITY,
                    );

                    let collider =
                        Collider::sensor(engine_physics::ColliderShape::Box { half_extents: Vec3::new(1.0, 1.0, 1.0) });
                    world.add_collider(trigger, &collider);
                }

                // Create dynamic objects inside triggers
                for i in 0..count {
                    let object = (count + i) as u64;
                    let rb = RigidBody::dynamic(1.0);
                    world.add_rigidbody(
                        object,
                        &rb,
                        Vec3::new((i as f32) * 3.0, 0.0, 0.0),
                        Quat::IDENTITY,
                    );

                    let collider = Collider::sphere(0.5);
                    world.add_collider(object, &collider);
                }

                // Initial step to populate trigger pairs
                world.step(1.0 / 60.0);

                b.iter(|| {
                    black_box(world.step(black_box(1.0 / 60.0)));
                });
            },
        );
    }

    group.finish();
}

fn bench_raycast_empty_world(c: &mut Criterion) {
    c.bench_function("raycast_empty_world", |b| {
        let config = PhysicsConfig::default();
        let world = PhysicsWorld::new(config);

        let origin = Vec3::new(0.0, 10.0, 0.0);
        let direction = Vec3::new(0.0, -1.0, 0.0);
        let max_distance = 20.0;

        b.iter(|| {
            black_box(world.raycast(
                black_box(origin),
                black_box(direction),
                black_box(max_distance),
            ));
        });
    });
}

fn bench_raycast_ground_only(c: &mut Criterion) {
    c.bench_function("raycast_ground_only", |b| {
        let config = PhysicsConfig::default();
        let mut world = PhysicsWorld::new(config);

        // Just ground
        let ground = 1u64;
        let rb = RigidBody::static_body();
        world.add_rigidbody(ground, &rb, Vec3::new(0.0, -0.5, 0.0), Quat::IDENTITY);

        let ground_collider = Collider::box_collider(Vec3::new(100.0, 0.5, 100.0));
        world.add_collider(ground, &ground_collider);

        world.step(0.0);

        let origin = Vec3::new(0.0, 10.0, 0.0);
        let direction = Vec3::new(0.0, -1.0, 0.0);
        let max_distance = 20.0;

        b.iter(|| {
            black_box(world.raycast(
                black_box(origin),
                black_box(direction),
                black_box(max_distance),
            ));
        });
    });
}

fn bench_raycast_miss(c: &mut Criterion) {
    c.bench_function("raycast_miss", |b| {
        let config = PhysicsConfig::default();
        let mut world = PhysicsWorld::new(config);
        create_test_scene(&mut world, 100);

        // Ray pointing upward (will miss everything)
        let origin = Vec3::new(0.0, 10.0, 0.0);
        let direction = Vec3::new(0.0, 1.0, 0.0);
        let max_distance = 20.0;

        b.iter(|| {
            black_box(world.raycast(
                black_box(origin),
                black_box(direction),
                black_box(max_distance),
            ));
        });
    });
}

criterion_group!(
    raycast_benches,
    bench_raycast_single,
    bench_raycast_all,
    bench_raycast_batch,
    bench_raycast_empty_world,
    bench_raycast_ground_only,
    bench_raycast_miss,
);

criterion_group!(trigger_benches, bench_trigger_detection,);

criterion_main!(raycast_benches, trigger_benches);
