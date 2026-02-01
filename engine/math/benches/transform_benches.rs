use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use engine_math::{Quat, QuatExt, Transform, Vec3};

fn bench_transform_point(c: &mut Criterion) {
    let transform = Transform::new(
        Vec3::new(10.0, 20.0, 30.0),
        Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), 0.5),
        Vec3::new(2.0, 2.0, 2.0),
    );
    let point = Vec3::new(1.0, 2.0, 3.0);

    c.bench_function("transform_point", |b| {
        b.iter(|| black_box(transform.transform_point(black_box(point))))
    });
}

fn bench_transform_point_scalar_baseline(c: &mut Criterion) {
    let transform = Transform::new(
        Vec3::new(10.0, 20.0, 30.0),
        Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), 0.5),
        Vec3::new(2.0, 2.0, 2.0),
    );
    let point = Vec3::new(1.0, 2.0, 3.0);

    // Scalar transform_point implementation (original, before Affine3A)
    c.bench_function("transform_point_scalar_baseline", |b| {
        b.iter(|| {
            let point = black_box(point);
            // Scale first
            let scaled = Vec3::new(
                point.x * transform.scale.x,
                point.y * transform.scale.y,
                point.z * transform.scale.z,
            );
            // Then rotate
            let rotated = transform.rotation.rotate_vec3(scaled);
            // Finally translate
            black_box(transform.position + rotated)
        })
    });
}

fn bench_transform_vector(c: &mut Criterion) {
    let transform = Transform::new(
        Vec3::new(10.0, 20.0, 30.0),
        Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), 0.5),
        Vec3::new(2.0, 2.0, 2.0),
    );
    let vector = Vec3::new(1.0, 0.0, 0.0);

    c.bench_function("transform_vector", |b| {
        b.iter(|| black_box(transform.transform_vector(black_box(vector))))
    });
}

fn bench_inverse_transform_point(c: &mut Criterion) {
    let transform = Transform::new(
        Vec3::new(10.0, 20.0, 30.0),
        Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), 0.5),
        Vec3::new(2.0, 2.0, 2.0),
    );
    let point = Vec3::new(1.0, 2.0, 3.0);

    c.bench_function("inverse_transform_point", |b| {
        b.iter(|| black_box(transform.inverse_transform_point(black_box(point))))
    });
}

fn bench_inverse_transform_vector(c: &mut Criterion) {
    let transform = Transform::new(
        Vec3::new(10.0, 20.0, 30.0),
        Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), 0.5),
        Vec3::new(2.0, 2.0, 2.0),
    );
    let vector = Vec3::new(1.0, 0.0, 0.0);

    c.bench_function("inverse_transform_vector", |b| {
        b.iter(|| black_box(transform.inverse_transform_vector(black_box(vector))))
    });
}

fn bench_compose(c: &mut Criterion) {
    let t1 = Transform::new(
        Vec3::new(1.0, 2.0, 3.0),
        Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), 0.5),
        Vec3::new(1.5, 1.5, 1.5),
    );
    let t2 = Transform::new(
        Vec3::new(4.0, 5.0, 6.0),
        Quat::from_axis_angle(Vec3::new(1.0, 0.0, 0.0), 0.3),
        Vec3::new(2.0, 2.0, 2.0),
    );

    c.bench_function("compose", |b| b.iter(|| black_box(t1.compose(black_box(&t2)))));
}

fn bench_compose_scalar_baseline(c: &mut Criterion) {
    let t1 = Transform::new(
        Vec3::new(1.0, 2.0, 3.0),
        Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), 0.5),
        Vec3::new(1.5, 1.5, 1.5),
    );
    let t2 = Transform::new(
        Vec3::new(4.0, 5.0, 6.0),
        Quat::from_axis_angle(Vec3::new(1.0, 0.0, 0.0), 0.3),
        Vec3::new(2.0, 2.0, 2.0),
    );

    // Scalar compose implementation (original, before Affine3A)
    c.bench_function("compose_scalar_baseline", |b| {
        b.iter(|| {
            black_box(Transform::new(
                // position: other.position + other.rotation * (other.scale * self.position)
                t2.transform_point(t1.position),
                // rotation: other.rotation * self.rotation
                t2.rotation * t1.rotation,
                // scale: component-wise multiply
                Vec3::new(
                    t1.scale.x * t2.scale.x,
                    t1.scale.y * t2.scale.y,
                    t1.scale.z * t2.scale.z,
                ),
            ))
        })
    });
}

fn bench_lerp(c: &mut Criterion) {
    let t1 = Transform::identity();
    let t2 = Transform::new(
        Vec3::new(10.0, 20.0, 30.0),
        Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), 1.0),
        Vec3::new(2.0, 2.0, 2.0),
    );

    c.bench_function("lerp", |b| b.iter(|| black_box(t1.lerp(black_box(&t2), black_box(0.5)))));
}

fn bench_batch_transform_points(c: &mut Criterion) {
    let transform = Transform::new(
        Vec3::new(10.0, 20.0, 30.0),
        Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), 0.5),
        Vec3::new(2.0, 2.0, 2.0),
    );

    let mut group = c.benchmark_group("batch_transform_points");

    for size in [10, 100, 1000].iter() {
        let points: Vec<Vec3> = (0..*size)
            .map(|i| Vec3::new(i as f32, i as f32 * 2.0, i as f32 * 3.0))
            .collect();

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let mut results = Vec::with_capacity(points.len());
                for point in &points {
                    results.push(transform.transform_point(*point));
                }
                black_box(results)
            })
        });
    }
    group.finish();
}

fn bench_transform_chain(c: &mut Criterion) {
    let transforms: Vec<Transform> = (0..10)
        .map(|i| {
            Transform::new(
                Vec3::new(i as f32, 0.0, 0.0),
                Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), i as f32 * 0.1),
                Vec3::new(1.0, 1.0, 1.0),
            )
        })
        .collect();

    c.bench_function("transform_chain", |b| {
        b.iter(|| {
            let mut result = transforms[0];
            for t in &transforms[1..] {
                result = result.compose(t);
            }
            black_box(result)
        })
    });
}

fn bench_look_at(c: &mut Criterion) {
    let mut transform = Transform::identity();
    transform.position = Vec3::new(5.0, 3.0, 2.0);
    let target = Vec3::new(10.0, 5.0, 8.0);
    let up = Vec3::new(0.0, 1.0, 0.0);

    c.bench_function("look_at", |b| {
        b.iter(|| {
            let mut t = black_box(transform);
            t.look_at(black_box(target), black_box(up));
            black_box(t)
        })
    });
}

fn bench_look_at_batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_look_at");

    for size in [10, 100, 1000].iter() {
        let initial_transforms: Vec<Transform> = (0..*size)
            .map(|i| {
                let mut t = Transform::identity();
                t.position = Vec3::new(i as f32, 0.0, 0.0);
                t
            })
            .collect();

        let targets: Vec<Vec3> = (0..*size)
            .map(|i| Vec3::new(i as f32 + 10.0, i as f32 * 0.5, i as f32 * 2.0))
            .collect();

        let up = Vec3::new(0.0, 1.0, 0.0);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let mut transforms = initial_transforms.clone();
                for (t, target) in transforms.iter_mut().zip(&targets) {
                    t.look_at(*target, up);
                }
                black_box(transforms)
            })
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_transform_point,
    bench_transform_point_scalar_baseline,
    bench_transform_vector,
    bench_inverse_transform_point,
    bench_inverse_transform_vector,
    bench_compose,
    bench_compose_scalar_baseline,
    bench_lerp,
    bench_batch_transform_points,
    bench_transform_chain,
    bench_look_at,
    bench_look_at_batch,
);
criterion_main!(benches);
