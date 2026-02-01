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

criterion_group!(
    benches,
    bench_transform_point,
    bench_transform_vector,
    bench_inverse_transform_point,
    bench_inverse_transform_vector,
    bench_compose,
    bench_lerp,
    bench_batch_transform_points,
    bench_transform_chain,
);
criterion_main!(benches);
