//! Benchmarks for asset validation system

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use engine_assets::{
    AssetValidator, AudioData, AudioFormat, MaterialData, MeshData, TextureData, TextureFormat,
};
use glam::{Vec2, Vec3};

// ============================================================================
// Validation Speed Benchmarks
// ============================================================================

fn bench_mesh_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("mesh_validation");

    for &vertex_count in &[100, 1000, 10000] {
        let mut mesh = MeshData::with_capacity(vertex_count, vertex_count * 3);
        for i in 0..vertex_count {
            mesh.vertices.push(engine_assets::Vertex::new(
                Vec3::new(i as f32, i as f32, i as f32),
                Vec3::Z,
                Vec2::ZERO,
            ));
        }
        for i in 0..(vertex_count * 3) {
            mesh.indices.push((i % vertex_count) as u32);
        }

        group.bench_with_input(
            BenchmarkId::new("validate_data", vertex_count),
            &mesh,
            |b, mesh| {
                b.iter(|| {
                    black_box(mesh.validate_data()).unwrap();
                });
            },
        );

        group.bench_with_input(BenchmarkId::new("validate_all", vertex_count), &mesh, |b, mesh| {
            b.iter(|| {
                black_box(mesh.validate_all());
            });
        });
    }

    group.finish();
}

fn bench_texture_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("texture_validation");

    for &size in &[64, 256, 1024] {
        let data = vec![128u8; (size * size * 4) as usize];
        let texture = TextureData::new(size, size, TextureFormat::RGBA8Unorm, data).unwrap();

        group.bench_with_input(BenchmarkId::new("validate_data", size), &texture, |b, texture| {
            b.iter(|| {
                black_box(texture.validate_data()).unwrap();
            });
        });

        group.bench_with_input(BenchmarkId::new("validate_all", size), &texture, |b, texture| {
            b.iter(|| {
                black_box(texture.validate_all());
            });
        });
    }

    group.finish();
}

fn bench_material_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("material_validation");

    let mut material = MaterialData::new("benchmark_material");
    material.base_color_texture = Some("albedo.png".to_string());
    material.normal_texture = Some("normal.png".to_string());
    material.metallic_factor = 0.5;
    material.roughness_factor = 0.7;

    group.bench_function("validate_data", |b| {
        b.iter(|| {
            black_box(material.validate_data()).unwrap();
        });
    });

    group.bench_function("validate_all", |b| {
        b.iter(|| {
            black_box(material.validate_all());
        });
    });

    group.finish();
}

fn bench_audio_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("audio_validation");

    for &sample_count in &[1000, 10000, 100000] {
        let data = vec![0u8; sample_count * 2]; // 16-bit samples
        let audio = AudioData::new(44100, 2, AudioFormat::PCM16, data);

        group.bench_with_input(
            BenchmarkId::new("validate_data", sample_count),
            &audio,
            |b, audio| {
                b.iter(|| {
                    black_box(audio.validate_data()).unwrap();
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("validate_all", sample_count),
            &audio,
            |b, audio| {
                b.iter(|| {
                    black_box(audio.validate_all());
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Checksum Computation Benchmarks
// ============================================================================

fn bench_mesh_checksum(c: &mut Criterion) {
    let mut group = c.benchmark_group("mesh_checksum");

    for &vertex_count in &[100, 1000, 10000] {
        let mut mesh = MeshData::with_capacity(vertex_count, vertex_count * 3);
        for i in 0..vertex_count {
            mesh.vertices.push(engine_assets::Vertex::new(
                Vec3::new(i as f32, i as f32, i as f32),
                Vec3::Z,
                Vec2::ZERO,
            ));
        }
        for i in 0..(vertex_count * 3) {
            mesh.indices.push((i % vertex_count) as u32);
        }

        group.bench_with_input(
            BenchmarkId::new("compute_checksum", vertex_count),
            &mesh,
            |b, mesh| {
                b.iter(|| {
                    black_box(mesh.compute_checksum());
                });
            },
        );
    }

    group.finish();
}

fn bench_texture_checksum(c: &mut Criterion) {
    let mut group = c.benchmark_group("texture_checksum");

    for &size in &[64, 256, 1024] {
        let data = vec![128u8; (size * size * 4) as usize];
        let texture = TextureData::new(size, size, TextureFormat::RGBA8Unorm, data).unwrap();

        group.bench_with_input(
            BenchmarkId::new("compute_checksum", size),
            &texture,
            |b, texture| {
                b.iter(|| {
                    black_box(texture.compute_checksum());
                });
            },
        );
    }

    group.finish();
}

fn bench_material_checksum(c: &mut Criterion) {
    let mut group = c.benchmark_group("material_checksum");

    let mut material = MaterialData::new("benchmark_material");
    material.base_color_texture = Some("albedo.png".to_string());
    material.normal_texture = Some("normal.png".to_string());

    group.bench_function("compute_checksum", |b| {
        b.iter(|| {
            black_box(material.compute_checksum());
        });
    });

    group.finish();
}

fn bench_audio_checksum(c: &mut Criterion) {
    let mut group = c.benchmark_group("audio_checksum");

    for &sample_count in &[1000, 10000, 100000] {
        let data = vec![0u8; sample_count * 2];
        let audio = AudioData::new(44100, 2, AudioFormat::PCM16, data);

        group.bench_with_input(
            BenchmarkId::new("compute_checksum", sample_count),
            &audio,
            |b, audio| {
                b.iter(|| {
                    black_box(audio.compute_checksum());
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Format Validation Benchmarks
// ============================================================================

fn bench_format_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("format_validation");

    // Mesh binary format
    let mesh = MeshData::cube();
    let mesh_binary = mesh.to_binary();

    group.bench_function("mesh_validate_format", |b| {
        b.iter(|| {
            black_box(MeshData::validate_format(&mesh_binary)).unwrap();
        });
    });

    // Material YAML format
    let material = MaterialData::new("test");
    let material_yaml = material.to_yaml().unwrap();

    group.bench_function("material_validate_format", |b| {
        b.iter(|| {
            black_box(MaterialData::validate_format(material_yaml.as_bytes())).unwrap();
        });
    });

    group.finish();
}

criterion_group!(
    validation_benches,
    bench_mesh_validation,
    bench_texture_validation,
    bench_material_validation,
    bench_audio_validation,
    bench_mesh_checksum,
    bench_texture_checksum,
    bench_material_checksum,
    bench_audio_checksum,
    bench_format_validation,
);

criterion_main!(validation_benches);
