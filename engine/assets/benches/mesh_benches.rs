use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
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

fn bench_gltf_loading(c: &mut Criterion) {
    let triangle_gltf = include_bytes!("../test_data/triangle.gltf");
    let triangle_bin = include_bytes!("../test_data/triangle.bin");

    let cube_gltf = include_bytes!("../test_data/cube.gltf");
    let cube_bin = include_bytes!("../test_data/cube.bin");

    let embedded_gltf = include_bytes!("../test_data/triangle_embedded.gltf");

    c.bench_function("gltf_load_simple_triangle", |b| {
        b.iter(|| {
            let mesh = MeshData::from_gltf(
                black_box(triangle_gltf),
                Some(black_box(triangle_bin.as_slice())),
            )
            .unwrap();
            black_box(mesh);
        });
    });

    c.bench_function("gltf_load_cube", |b| {
        b.iter(|| {
            let mesh =
                MeshData::from_gltf(black_box(cube_gltf), Some(black_box(cube_bin.as_slice())))
                    .unwrap();
            black_box(mesh);
        });
    });

    c.bench_function("gltf_load_embedded", |b| {
        b.iter(|| {
            let mesh = MeshData::from_gltf(black_box(embedded_gltf), None).unwrap();
            black_box(mesh);
        });
    });
}

fn bench_binary_format(c: &mut Criterion) {
    let triangle = MeshData::triangle();
    let cube = MeshData::cube();

    // Create a large mesh for benchmarking
    let mut large_mesh = MeshData::with_capacity(1000, 3000);
    for i in 0..1000 {
        let t = i as f32 / 1000.0;
        large_mesh.vertices.push(engine_assets::Vertex::new(
            glam::Vec3::new(t, t * 2.0, t * 3.0),
            glam::Vec3::new(0.0, 1.0, 0.0),
            glam::Vec2::new(t, 1.0 - t),
        ));
    }
    for i in 0..999 {
        large_mesh.indices.push(i as u32);
        large_mesh.indices.push((i + 1) as u32);
        large_mesh.indices.push(0);
    }

    // Serialize benchmarks
    c.bench_function("binary_serialize_triangle", |b| {
        b.iter(|| {
            let binary = black_box(&triangle).to_binary();
            black_box(binary);
        });
    });

    c.bench_function("binary_serialize_cube", |b| {
        b.iter(|| {
            let binary = black_box(&cube).to_binary();
            black_box(binary);
        });
    });

    c.bench_function("binary_serialize_large", |b| {
        b.iter(|| {
            let binary = black_box(&large_mesh).to_binary();
            black_box(binary);
        });
    });

    // Deserialize benchmarks
    let triangle_binary = triangle.to_binary();
    let cube_binary = cube.to_binary();
    let large_binary = large_mesh.to_binary();

    c.bench_function("binary_deserialize_triangle", |b| {
        b.iter(|| {
            let mesh = MeshData::from_binary(black_box(&triangle_binary)).unwrap();
            black_box(mesh);
        });
    });

    c.bench_function("binary_deserialize_cube", |b| {
        b.iter(|| {
            let mesh = MeshData::from_binary(black_box(&cube_binary)).unwrap();
            black_box(mesh);
        });
    });

    c.bench_function("binary_deserialize_large", |b| {
        b.iter(|| {
            let mesh = MeshData::from_binary(black_box(&large_binary)).unwrap();
            black_box(mesh);
        });
    });

    // Roundtrip benchmarks
    c.bench_function("binary_roundtrip_triangle", |b| {
        b.iter(|| {
            let binary = black_box(&triangle).to_binary();
            let mesh = MeshData::from_binary(&binary).unwrap();
            black_box(mesh);
        });
    });

    c.bench_function("binary_roundtrip_cube", |b| {
        b.iter(|| {
            let binary = black_box(&cube).to_binary();
            let mesh = MeshData::from_binary(&binary).unwrap();
            black_box(mesh);
        });
    });
}

fn bench_format_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("format_comparison");

    // Compare loading the same mesh from different formats
    let obj_triangle = r#"
        v 0.0 -0.5 0.0
        v 0.5 0.5 0.0
        v -0.5 0.5 0.0
        vn 0.0 0.0 1.0
        vt 0.5 1.0
        vt 1.0 0.0
        vt 0.0 0.0
        f 1/1/1 2/2/1 3/3/1
    "#;

    let gltf_triangle = include_bytes!("../test_data/triangle.gltf");
    let gltf_bin = include_bytes!("../test_data/triangle.bin");

    let triangle = MeshData::triangle();
    let binary_triangle = triangle.to_binary();

    group.bench_function("obj", |b| {
        b.iter(|| {
            let mesh = MeshData::from_obj(black_box(obj_triangle)).unwrap();
            black_box(mesh);
        });
    });

    group.bench_function("gltf", |b| {
        b.iter(|| {
            let mesh =
                MeshData::from_gltf(black_box(gltf_triangle), Some(black_box(gltf_bin.as_slice())))
                    .unwrap();
            black_box(mesh);
        });
    });

    group.bench_function("binary", |b| {
        b.iter(|| {
            let mesh = MeshData::from_binary(black_box(&binary_triangle)).unwrap();
            black_box(mesh);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_mesh_creation,
    bench_obj_loading,
    bench_mesh_queries,
    bench_mesh_allocation,
    bench_gltf_loading,
    bench_binary_format,
    bench_format_comparison
);
criterion_main!(benches);
