use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use engine_assets::MeshData;

fn bench_mesh_creation(c: &mut Criterion) {
    c.bench_function("mesh_cube_creation", |b| {
        b.iter(|| {
            let mesh = MeshData::cube();
            black_box(mesh);
        });
    });

    c.bench_function("mesh_triangle_creation", |b| {
        b.iter(|| {
            let mesh = MeshData::triangle();
            black_box(mesh);
        });
    });
}

fn bench_obj_loading(c: &mut Criterion) {
    let simple_obj = r#"
        v 0.0 -0.5 0.0
        v 0.5 0.5 0.0
        v -0.5 0.5 0.0
        f 1 2 3
    "#;

    let complex_obj = r#"
        v -1.0 -1.0 1.0
        v 1.0 -1.0 1.0
        v 1.0 1.0 1.0
        v -1.0 1.0 1.0
        v 1.0 -1.0 -1.0
        v -1.0 -1.0 -1.0
        v -1.0 1.0 -1.0
        v 1.0 1.0 -1.0
        vn 0.0 0.0 1.0
        vn 0.0 0.0 -1.0
        vn 0.0 1.0 0.0
        vn 0.0 -1.0 0.0
        vn 1.0 0.0 0.0
        vn -1.0 0.0 0.0
        vt 0.0 0.0
        vt 1.0 0.0
        vt 1.0 1.0
        vt 0.0 1.0
        f 1/1/1 2/2/1 3/3/1 4/4/1
        f 5/1/2 6/2/2 7/3/2 8/4/2
    "#;

    c.bench_function("obj_load_simple", |b| {
        b.iter(|| {
            let mesh = MeshData::from_obj(black_box(simple_obj)).unwrap();
            black_box(mesh);
        });
    });

    c.bench_function("obj_load_complex", |b| {
        b.iter(|| {
            let mesh = MeshData::from_obj(black_box(complex_obj)).unwrap();
            black_box(mesh);
        });
    });
}

fn bench_mesh_queries(c: &mut Criterion) {
    let cube = MeshData::cube();

    c.bench_function("mesh_bounding_box", |b| {
        b.iter(|| {
            let bbox = black_box(&cube).bounding_box();
            black_box(bbox);
        });
    });

    c.bench_function("mesh_centroid", |b| {
        b.iter(|| {
            let centroid = black_box(&cube).centroid();
            black_box(centroid);
        });
    });
}

fn bench_mesh_allocation(c: &mut Criterion) {
    let mut group = c.benchmark_group("mesh_allocation");

    for size in [100, 1000, 10000] {
        group.bench_with_input(BenchmarkId::new("with_capacity", size), &size, |b, &size| {
            b.iter(|| {
                let mesh = MeshData::with_capacity(black_box(size), black_box(size * 3));
                black_box(mesh);
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_mesh_creation,
    bench_obj_loading,
    bench_mesh_queries,
    bench_mesh_allocation
);
criterion_main!(benches);
