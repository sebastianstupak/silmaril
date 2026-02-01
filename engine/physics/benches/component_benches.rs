//! Component operation benchmarks
//!
//! Establishes baseline performance for physics components.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use engine_physics::{RigidBody, Collider, PhysicsMaterial};
use engine_math::Vec3;

/// Benchmark RigidBody creation
fn bench_rigidbody_creation(c: &mut Criterion) {
    c.bench_function("rigidbody_dynamic_create", |b| {
        b.iter(|| {
            black_box(RigidBody::dynamic(1.0));
        });
    });

    c.bench_function("rigidbody_kinematic_create", |b| {
        b.iter(|| {
            black_box(RigidBody::kinematic());
        });
    });

    c.bench_function("rigidbody_static_create", |b| {
        b.iter(|| {
            black_box(RigidBody::static_body());
        });
    });
}

/// Benchmark impulse application
fn bench_impulse_application(c: &mut Criterion) {
    let mut group = c.benchmark_group("impulse_application");

    for count in [1, 10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*count as u64));

        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            let mut bodies: Vec<RigidBody> = (0..count)
                .map(|_| RigidBody::dynamic(1.0))
                .collect();

            b.iter(|| {
                for body in &mut bodies {
                    body.apply_impulse(black_box(Vec3::new(10.0, 0.0, 0.0)));
                }
            });
        });
    }

    group.finish();
}

/// Benchmark material combining
fn bench_material_combine(c: &mut Criterion) {
    let ice = PhysicsMaterial::ICE;
    let rubber = PhysicsMaterial::RUBBER;
    let metal = PhysicsMaterial::METAL;
    let wood = PhysicsMaterial::WOOD;

    c.bench_function("material_combine_ice_rubber", |b| {
        b.iter(|| {
            black_box(ice.combine(&rubber));
        });
    });

    c.bench_function("material_combine_metal_wood", |b| {
        b.iter(|| {
            black_box(metal.combine(&wood));
        });
    });

    // Benchmark batch combining (simulating many collision pairs)
    let mut group = c.benchmark_group("material_combine_batch");

    for count in [10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*count as u64));

        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            let materials: Vec<PhysicsMaterial> = (0..count)
                .map(|i| if i % 2 == 0 { ice } else { rubber })
                .collect();

            b.iter(|| {
                for i in 0..materials.len() - 1 {
                    black_box(materials[i].combine(&materials[i + 1]));
                }
            });
        });
    }

    group.finish();
}

/// Benchmark collider creation
fn bench_collider_creation(c: &mut Criterion) {
    c.bench_function("collider_box_create", |b| {
        b.iter(|| {
            black_box(Collider::box_collider(Vec3::ONE));
        });
    });

    c.bench_function("collider_sphere_create", |b| {
        b.iter(|| {
            black_box(Collider::sphere(1.0));
        });
    });

    c.bench_function("collider_capsule_create", |b| {
        b.iter(|| {
            black_box(Collider::capsule(2.0, 0.5));
        });
    });
}

/// Benchmark collision layer checks
fn bench_collision_layer_check(c: &mut Criterion) {
    let collider = Collider::sphere(1.0)
        .with_layer(2)
        .with_mask(0b10101010);

    c.bench_function("collision_layer_check", |b| {
        b.iter(|| {
            for layer in 0..32 {
                black_box(collider.can_collide_with(1 << layer));
            }
        });
    });

    // Batch layer checking
    let mut group = c.benchmark_group("collision_layer_check_batch");

    for count in [10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*count as u64 * 32));

        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            let colliders: Vec<Collider> = (0..count)
                .map(|i| {
                    Collider::sphere(1.0)
                        .with_layer(i % 8)
                        .with_mask(0xFFFFFFFF)
                })
                .collect();

            b.iter(|| {
                for collider in &colliders {
                    for layer in 0..32 {
                        black_box(collider.can_collide_with(1 << layer));
                    }
                }
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_rigidbody_creation,
    bench_impulse_application,
    bench_material_combine,
    bench_collider_creation,
    bench_collision_layer_check
);
criterion_main!(benches);
