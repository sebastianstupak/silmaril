//! Benchmarks for procedural asset generation

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_assets::{
    MeshData, ProceduralAsset, ProceduralMeshParams, ProceduralTextureParams, TextureData,
};

fn bench_procedural_cube(c: &mut Criterion) {
    let mut group = c.benchmark_group("procedural_cube");

    for size in [1.0, 2.0, 5.0, 10.0].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let params = ProceduralMeshParams::Cube { size };
            let seed = 12345;

            b.iter(|| {
                let mesh = MeshData::generate(black_box(seed), black_box(&params));
                black_box(mesh);
            });
        });
    }

    group.finish();
}

fn bench_procedural_sphere(c: &mut Criterion) {
    let mut group = c.benchmark_group("procedural_sphere");

    for subdivisions in [(8, 16), (16, 32), (32, 64)].iter() {
        let (lat, lon) = subdivisions;
        group.bench_with_input(
            BenchmarkId::new("subdivisions", format!("{}x{}", lat, lon)),
            subdivisions,
            |b, &(lat, lon)| {
                let params = ProceduralMeshParams::Sphere {
                    radius: 1.0,
                    lat_subdivisions: lat,
                    lon_subdivisions: lon,
                };
                let seed = 12345;

                b.iter(|| {
                    let mesh = MeshData::generate(black_box(seed), black_box(&params));
                    black_box(mesh);
                });
            },
        );
    }

    group.finish();
}

fn bench_procedural_terrain(c: &mut Criterion) {
    let mut group = c.benchmark_group("procedural_terrain");
    group.sample_size(20); // Terrain generation is slower

    for resolution in [32, 64, 128].iter() {
        group.throughput(Throughput::Elements((*resolution * *resolution) as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(resolution),
            resolution,
            |b, &resolution| {
                let params = ProceduralMeshParams::Terrain {
                    width: 100.0,
                    height: 100.0,
                    width_res: resolution,
                    height_res: resolution,
                    max_height: 10.0,
                    octaves: 4,
                    frequency: 0.05,
                };
                let seed = 12345;

                b.iter(|| {
                    let mesh = MeshData::generate(black_box(seed), black_box(&params));
                    black_box(mesh);
                });
            },
        );
    }

    group.finish();
}

fn bench_procedural_terrain_octaves(c: &mut Criterion) {
    let mut group = c.benchmark_group("procedural_terrain_octaves");
    group.sample_size(20);

    for octaves in [1, 2, 4, 8].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(octaves), octaves, |b, &octaves| {
            let params = ProceduralMeshParams::Terrain {
                width: 100.0,
                height: 100.0,
                width_res: 64,
                height_res: 64,
                max_height: 10.0,
                octaves,
                frequency: 0.05,
            };
            let seed = 12345;

            b.iter(|| {
                let mesh = MeshData::generate(black_box(seed), black_box(&params));
                black_box(mesh);
            });
        });
    }

    group.finish();
}

fn bench_procedural_noise_texture(c: &mut Criterion) {
    let mut group = c.benchmark_group("procedural_noise_texture");
    group.sample_size(20);

    for size in [128, 256, 512].iter() {
        group.throughput(Throughput::Elements((*size * *size) as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let params = ProceduralTextureParams::Noise {
                width: size,
                height: size,
                octaves: 4,
                frequency: 0.05,
            };
            let seed = 12345;

            b.iter(|| {
                let texture = TextureData::generate(black_box(seed), black_box(&params));
                black_box(texture);
            });
        });
    }

    group.finish();
}

fn bench_procedural_noise_texture_octaves(c: &mut Criterion) {
    let mut group = c.benchmark_group("procedural_noise_texture_octaves");
    group.sample_size(20);

    for octaves in [1, 2, 4, 8].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(octaves), octaves, |b, &octaves| {
            let params = ProceduralTextureParams::Noise {
                width: 256,
                height: 256,
                octaves,
                frequency: 0.05,
            };
            let seed = 12345;

            b.iter(|| {
                let texture = TextureData::generate(black_box(seed), black_box(&params));
                black_box(texture);
            });
        });
    }

    group.finish();
}

fn bench_procedural_checkerboard_texture(c: &mut Criterion) {
    let mut group = c.benchmark_group("procedural_checkerboard_texture");

    for size in [128, 256, 512, 1024].iter() {
        group.throughput(Throughput::Elements((*size * *size) as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let params = ProceduralTextureParams::Checkerboard {
                width: size,
                height: size,
                square_size: 32,
                color1: [255, 0, 0, 255],
                color2: [0, 255, 0, 255],
            };
            let seed = 12345;

            b.iter(|| {
                let texture = TextureData::generate(black_box(seed), black_box(&params));
                black_box(texture);
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_procedural_cube,
    bench_procedural_sphere,
    bench_procedural_terrain,
    bench_procedural_terrain_octaves,
    bench_procedural_noise_texture,
    bench_procedural_noise_texture_octaves,
    bench_procedural_checkerboard_texture,
);
criterion_main!(benches);
