//! Vec3 scalar operation benchmarks
//!
//! Benchmarks all core Vec3 operations to establish performance baselines.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_math::{Vec3, Vec3Ext};

fn bench_vec3_add(c: &mut Criterion) {
    let mut group = c.benchmark_group("vec3_add");

    let a = Vec3::new(1.0, 2.0, 3.0);
    let b = Vec3::new(4.0, 5.0, 6.0);

    group.bench_function("scalar", |bench| {
        bench.iter(|| black_box(black_box(a) + black_box(b)));
    });

    group.finish();
}

fn bench_vec3_sub(c: &mut Criterion) {
    let mut group = c.benchmark_group("vec3_sub");

    let a = Vec3::new(1.0, 2.0, 3.0);
    let b = Vec3::new(4.0, 5.0, 6.0);

    group.bench_function("scalar", |bench| {
        bench.iter(|| black_box(black_box(a) - black_box(b)));
    });

    group.finish();
}

fn bench_vec3_mul_scalar(c: &mut Criterion) {
    let mut group = c.benchmark_group("vec3_mul_scalar");

    let v = Vec3::new(1.0, 2.0, 3.0);
    let scalar = 2.5;

    group.bench_function("scalar", |bench| {
        bench.iter(|| black_box(black_box(v) * black_box(scalar)));
    });

    group.finish();
}

fn bench_vec3_dot(c: &mut Criterion) {
    let mut group = c.benchmark_group("vec3_dot");

    let a = Vec3::new(1.0, 2.0, 3.0);
    let b = Vec3::new(4.0, 5.0, 6.0);

    group.bench_function("scalar", |bench| {
        bench.iter(|| black_box(black_box(a).dot(black_box(b))));
    });

    group.finish();
}

fn bench_vec3_cross(c: &mut Criterion) {
    let mut group = c.benchmark_group("vec3_cross");

    let a = Vec3::new(1.0, 2.0, 3.0);
    let b = Vec3::new(4.0, 5.0, 6.0);

    group.bench_function("scalar", |bench| {
        bench.iter(|| black_box(black_box(a).cross(black_box(b))));
    });

    group.finish();
}

fn bench_vec3_magnitude(c: &mut Criterion) {
    let mut group = c.benchmark_group("vec3_magnitude");

    let v = Vec3::new(3.0, 4.0, 5.0);

    group.bench_function("scalar", |bench| {
        bench.iter(|| black_box(black_box(v).magnitude()));
    });

    group.finish();
}

fn bench_vec3_normalize(c: &mut Criterion) {
    let mut group = c.benchmark_group("vec3_normalize");

    let v = Vec3::new(3.0, 4.0, 5.0);

    group.bench_function("scalar", |bench| {
        bench.iter(|| black_box(black_box(v).normalize()));
    });

    group.finish();
}

fn bench_vec3_physics_integration(c: &mut Criterion) {
    let mut group = c.benchmark_group("vec3_physics_integration");

    for size in [100, 1000, 10000].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |bench, &size| {
            let mut positions: Vec<Vec3> =
                (0..size).map(|i| Vec3::new(i as f32, i as f32, i as f32)).collect();
            let velocities: Vec<Vec3> = (0..size)
                .map(|i| Vec3::new(i as f32 * 0.1, i as f32 * 0.1, i as f32 * 0.1))
                .collect();
            let dt = 0.016; // 60 FPS

            bench.iter(|| {
                for i in 0..size {
                    positions[i] = positions[i] + velocities[i] * dt;
                }
                black_box(&positions);
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_vec3_add,
    bench_vec3_sub,
    bench_vec3_mul_scalar,
    bench_vec3_dot,
    bench_vec3_cross,
    bench_vec3_magnitude,
    bench_vec3_normalize,
    bench_vec3_physics_integration,
);
criterion_main!(benches);
