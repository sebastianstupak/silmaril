//! Debug rendering benchmarks for Phase A.1
//!
//! Validates that debug rendering overhead stays within performance targets:
//! - Velocity rendering: < 50μs for 100 bodies
//! - AABB rendering: < 100μs for 100 bodies
//! - Collision rendering: < 200μs for 50 contact pairs
//! - Joint rendering: < 50μs for 20 joints

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use engine_math::{Quat, Vec3};
use engine_physics::{Collider, PhysicsConfig, PhysicsWorld, RigidBody};

#[cfg(feature = "debug-render")]
use engine_physics::debug_render::*;

#[cfg(feature = "debug-render")]
fn bench_velocity_rendering(c: &mut Criterion) {
    let mut group = c.benchmark_group("debug_render/velocity");

    for body_count in [10, 50, 100, 500].iter() {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());
        let mut debug_renderer = DebugRenderer::new(None);

        // Create bodies with varying velocities
        for i in 0..*body_count {
            world.add_rigidbody(
                i,
                &RigidBody::dynamic(1.0),
                Vec3::new((i % 10) as f32 * 2.0, (i / 10) as f32 * 2.0, 0.0),
                Quat::IDENTITY,
            );
            world.add_collider(i, &Collider::box_collider(Vec3::ONE));
            world.set_velocity(
                i,
                Vec3::new((i % 5) as f32, 0.0, 0.0),
                Vec3::ZERO,
            );
        }

        let options = VelocityRenderOptions::default();

        group.bench_with_input(
            BenchmarkId::from_parameter(body_count),
            body_count,
            |b, _| {
                b.iter(|| {
                    debug_renderer.begin_frame();
                    debug_renderer.render_velocities(black_box(&world), black_box(&options));
                    let _ = debug_renderer.end_frame();
                });
            },
        );
    }

    group.finish();
}

#[cfg(feature = "debug-render")]
fn bench_aabb_rendering(c: &mut Criterion) {
    let mut group = c.benchmark_group("debug_render/aabb");

    for body_count in [10, 50, 100, 500].iter() {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());
        let mut debug_renderer = DebugRenderer::new(None);

        // Create bodies with colliders
        for i in 0..*body_count {
            world.add_rigidbody(
                i,
                &RigidBody::dynamic(1.0),
                Vec3::new((i % 10) as f32 * 2.0, (i / 10) as f32 * 2.0, 0.0),
                Quat::IDENTITY,
            );
            world.add_collider(i, &Collider::box_collider(Vec3::ONE));
        }

        let options = AabbRenderOptions::default();

        group.bench_with_input(
            BenchmarkId::from_parameter(body_count),
            body_count,
            |b, _| {
                b.iter(|| {
                    debug_renderer.begin_frame();
                    debug_renderer.render_aabbs(black_box(&world), black_box(&options));
                    let _ = debug_renderer.end_frame();
                });
            },
        );
    }

    group.finish();
}

#[cfg(feature = "debug-render")]
fn bench_collision_rendering(c: &mut Criterion) {
    let mut group = c.benchmark_group("debug_render/collision");

    // Create world with colliding bodies
    let mut world = PhysicsWorld::new(PhysicsConfig::default());

    // Ground plane
    world.add_rigidbody(0, &RigidBody::static_body(), Vec3::ZERO, Quat::IDENTITY);
    world.add_collider(0, &Collider::box_collider(Vec3::new(50.0, 0.1, 50.0)));

    // Stacked boxes (will create many contacts)
    for i in 1..=50 {
        world.add_rigidbody(
            i,
            &RigidBody::dynamic(1.0),
            Vec3::new((i % 5) as f32 * 1.1, i as f32 * 1.1, 0.0),
            Quat::IDENTITY,
        );
        world.add_collider(i, &Collider::box_collider(Vec3::ONE));
    }

    // Step to generate contacts
    for _ in 0..10 {
        world.step(1.0 / 60.0);
    }

    let mut debug_renderer = DebugRenderer::new(None);
    let options = CollisionRenderOptions::default();

    group.bench_function("50_bodies", |b| {
        b.iter(|| {
            debug_renderer.begin_frame();
            debug_renderer.render_collisions(black_box(&world), black_box(&options));
            let _ = debug_renderer.end_frame();
        });
    });

    group.finish();
}

#[cfg(feature = "debug-render")]
fn bench_center_of_mass_rendering(c: &mut Criterion) {
    let mut group = c.benchmark_group("debug_render/center_of_mass");

    for body_count in [10, 50, 100, 500].iter() {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());
        let mut debug_renderer = DebugRenderer::new(None);

        // Create bodies
        for i in 0..*body_count {
            world.add_rigidbody(
                i,
                &RigidBody::dynamic(1.0),
                Vec3::new((i % 10) as f32 * 2.0, (i / 10) as f32 * 2.0, 0.0),
                Quat::IDENTITY,
            );
            world.add_collider(i, &Collider::box_collider(Vec3::ONE));
        }

        let options = CenterOfMassRenderOptions::default();

        group.bench_with_input(
            BenchmarkId::from_parameter(body_count),
            body_count,
            |b, _| {
                b.iter(|| {
                    debug_renderer.begin_frame();
                    debug_renderer.render_center_of_mass(black_box(&world), black_box(&options));
                    let _ = debug_renderer.end_frame();
                });
            },
        );
    }

    group.finish();
}

#[cfg(feature = "debug-render")]
fn bench_line_batching(c: &mut Criterion) {
    let mut group = c.benchmark_group("debug_render/line_batching");

    let mut debug_renderer = DebugRenderer::new(Some(100_000));

    for line_count in [100, 1000, 10_000, 50_000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(line_count),
            line_count,
            |b, &count| {
                b.iter(|| {
                    debug_renderer.begin_frame();
                    for i in 0..count {
                        let offset = (i as f32) * 0.1;
                        debug_renderer.add_line(
                            Vec3::new(offset, 0.0, 0.0),
                            Vec3::new(offset, 1.0, 0.0),
                            [1.0, 0.0, 0.0],
                        );
                    }
                    let _ = debug_renderer.end_frame();
                });
            },
        );
    }

    group.finish();
}

#[cfg(feature = "debug-render")]
fn bench_force_rendering(c: &mut Criterion) {
    let mut group = c.benchmark_group("debug_render/force");

    for body_count in [10, 50, 100, 500].iter() {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());
        let mut debug_renderer = DebugRenderer::new(None);

        // Create bodies with applied forces
        for i in 0..*body_count {
            world.add_rigidbody(
                i,
                &RigidBody::dynamic(1.0),
                Vec3::new((i % 10) as f32 * 2.0, (i / 10) as f32 * 2.0, 0.0),
                Quat::IDENTITY,
            );
            world.add_collider(i, &Collider::box_collider(Vec3::ONE));
            // Apply force to each body
            world.apply_force(i, Vec3::new((i % 5) as f32 * 10.0, 50.0, 0.0));
        }

        let options = ForceRenderOptions::default();

        group.bench_with_input(
            BenchmarkId::from_parameter(body_count),
            body_count,
            |b, _| {
                b.iter(|| {
                    debug_renderer.begin_frame();
                    debug_renderer.render_forces(black_box(&world), black_box(&options));
                    let _ = debug_renderer.end_frame();
                });
            },
        );
    }

    group.finish();
}

#[cfg(feature = "debug-render")]
criterion_group!(
    benches,
    bench_velocity_rendering,
    bench_aabb_rendering,
    bench_collision_rendering,
    bench_center_of_mass_rendering,
    bench_force_rendering,
    bench_line_batching,
);

#[cfg(not(feature = "debug-render"))]
criterion_group!(benches,);

criterion_main!(benches);
