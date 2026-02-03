//! Benchmarks for procedural asset generation

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_assets::{
    ProceduralAssetGenerator, ProceduralAudioGenerator, ProceduralAudioParams,
    ProceduralMeshGenerator, ProceduralMeshParams, ProceduralTextureGenerator,
    ProceduralTextureParams,
};
use glam::Vec3;

// ============================================================================
// Mesh Generation Benchmarks
// ============================================================================

fn bench_mesh_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("procedural_mesh_generation");

    // Cube generation (simple)
    group.bench_function("cube_simple", |b| {
        let generator = ProceduralMeshGenerator::new();
        let params = ProceduralMeshParams::Cube { size: Vec3::new(1.0, 1.0, 1.0) };
        b.iter(|| generator.generate(black_box(0), black_box(&params)));
    });

    // Sphere with varying detail levels
    for (lat, lon) in [(8, 16), (16, 32), (32, 64)] {
        group.bench_with_input(
            BenchmarkId::new("sphere", format!("{lat}x{lon}")),
            &(lat, lon),
            |b, &(lat, lon)| {
                let generator = ProceduralMeshGenerator::new();
                let params = ProceduralMeshParams::Sphere {
                    radius: 1.0,
                    subdivisions_lat: lat,
                    subdivisions_lon: lon,
                };
                b.iter(|| generator.generate(black_box(0), black_box(&params)));
            },
        );
    }

    // Plane with varying subdivisions
    for subdivisions in [1, 10, 50, 100] {
        group.bench_with_input(
            BenchmarkId::new("plane", subdivisions),
            &subdivisions,
            |b, &subdivisions| {
                let generator = ProceduralMeshGenerator::new();
                let params = ProceduralMeshParams::Plane {
                    width: 10.0,
                    height: 10.0,
                    subdivisions_x: subdivisions,
                    subdivisions_y: subdivisions,
                };
                b.iter(|| generator.generate(black_box(0), black_box(&params)));
            },
        );
    }

    // Cylinder with varying segments
    for segments in [8, 16, 32, 64] {
        group.bench_with_input(
            BenchmarkId::new("cylinder", segments),
            &segments,
            |b, &segments| {
                let generator = ProceduralMeshGenerator::new();
                let params = ProceduralMeshParams::Cylinder { radius: 1.0, height: 2.0, segments };
                b.iter(|| generator.generate(black_box(0), black_box(&params)));
            },
        );
    }

    group.finish();
}

fn bench_mesh_vertex_count(c: &mut Criterion) {
    let mut group = c.benchmark_group("procedural_mesh_vertex_count");

    // Measure vertex generation throughput
    for vertex_target in [100, 1000, 10000, 100000] {
        // Calculate sphere subdivisions to approximate target vertex count
        // vertices = (lat+1) * (lon+1)
        let lat = ((vertex_target as f32).sqrt() / 2.0) as u32;
        let lon = lat * 2;

        group.throughput(Throughput::Elements(vertex_target as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(vertex_target),
            &(lat, lon),
            |b, &(lat, lon)| {
                let generator = ProceduralMeshGenerator::new();
                let params = ProceduralMeshParams::Sphere {
                    radius: 1.0,
                    subdivisions_lat: lat,
                    subdivisions_lon: lon,
                };
                b.iter(|| generator.generate(black_box(0), black_box(&params)));
            },
        );
    }

    group.finish();
}

fn bench_mesh_id_computation(c: &mut Criterion) {
    let mut group = c.benchmark_group("procedural_mesh_id_computation");

    let params =
        ProceduralMeshParams::Sphere { radius: 1.0, subdivisions_lat: 16, subdivisions_lon: 32 };

    group.bench_function("compute_id", |b| {
        b.iter(|| ProceduralMeshGenerator::compute_id(black_box(42), black_box(&params)));
    });

    group.finish();
}

// ============================================================================
// Texture Generation Benchmarks
// ============================================================================

fn bench_texture_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("procedural_texture_generation");

    // Perlin noise with varying sizes
    for size in [256, 512, 1024, 2048] {
        group.throughput(Throughput::Bytes((size * size * 4) as u64));
        group.bench_with_input(BenchmarkId::new("perlin_noise", size), &size, |b, &size| {
            let generator = ProceduralTextureGenerator::new();
            let params = ProceduralTextureParams::PerlinNoise {
                width: size,
                height: size,
                frequency: 0.1,
                octaves: 4,
            };
            b.iter(|| generator.generate(black_box(0), black_box(&params)));
        });
    }

    // Checkerboard (simple pattern)
    for size in [256, 512, 1024, 2048] {
        group.throughput(Throughput::Bytes((size * size * 4) as u64));
        group.bench_with_input(BenchmarkId::new("checkerboard", size), &size, |b, &size| {
            let generator = ProceduralTextureGenerator::new();
            let params = ProceduralTextureParams::Checkerboard {
                width: size,
                height: size,
                square_size: 32,
                color1: [255, 0, 0, 255],
                color2: [0, 0, 255, 255],
            };
            b.iter(|| generator.generate(black_box(0), black_box(&params)));
        });
    }

    // Solid color (fastest)
    for size in [256, 512, 1024, 2048] {
        group.throughput(Throughput::Bytes((size * size * 4) as u64));
        group.bench_with_input(BenchmarkId::new("solid_color", size), &size, |b, &size| {
            let generator = ProceduralTextureGenerator::new();
            let params = ProceduralTextureParams::SolidColor {
                width: size,
                height: size,
                color: [128, 128, 128, 255],
            };
            b.iter(|| generator.generate(black_box(0), black_box(&params)));
        });
    }

    group.finish();
}

fn bench_texture_octaves(c: &mut Criterion) {
    let mut group = c.benchmark_group("procedural_texture_octaves");

    // Measure impact of octave count on Perlin noise
    for octaves in [1, 2, 4, 8, 16] {
        group.bench_with_input(BenchmarkId::from_parameter(octaves), &octaves, |b, &octaves| {
            let generator = ProceduralTextureGenerator::new();
            let params = ProceduralTextureParams::PerlinNoise {
                width: 512,
                height: 512,
                frequency: 0.1,
                octaves,
            };
            b.iter(|| generator.generate(black_box(0), black_box(&params)));
        });
    }

    group.finish();
}

fn bench_texture_id_computation(c: &mut Criterion) {
    let mut group = c.benchmark_group("procedural_texture_id_computation");

    let params = ProceduralTextureParams::PerlinNoise {
        width: 1024,
        height: 1024,
        frequency: 0.1,
        octaves: 4,
    };

    group.bench_function("compute_id", |b| {
        b.iter(|| ProceduralTextureGenerator::compute_id(black_box(42), black_box(&params)));
    });

    group.finish();
}

// ============================================================================
// Audio Generation Benchmarks
// ============================================================================

fn bench_audio_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("procedural_audio_generation");

    // Sine wave with varying durations
    for duration in [0.1, 1.0, 10.0, 60.0] {
        let sample_count = (duration * 44100.0) as u64;
        group.throughput(Throughput::Elements(sample_count));
        group.bench_with_input(
            BenchmarkId::new("sine_wave", format!("{duration}s")),
            &duration,
            |b, &duration| {
                let generator = ProceduralAudioGenerator::new();
                let params = ProceduralAudioParams::SineWave {
                    frequency: 440.0,
                    duration,
                    sample_rate: 44100,
                };
                b.iter(|| generator.generate(black_box(0), black_box(&params)));
            },
        );
    }

    // Square wave
    group.bench_function("square_wave_1s", |b| {
        let generator = ProceduralAudioGenerator::new();
        let params = ProceduralAudioParams::SquareWave {
            frequency: 440.0,
            duration: 1.0,
            sample_rate: 44100,
        };
        b.iter(|| generator.generate(black_box(0), black_box(&params)));
    });

    // Sawtooth wave
    group.bench_function("sawtooth_wave_1s", |b| {
        let generator = ProceduralAudioGenerator::new();
        let params = ProceduralAudioParams::SawtoothWave {
            frequency: 440.0,
            duration: 1.0,
            sample_rate: 44100,
        };
        b.iter(|| generator.generate(black_box(0), black_box(&params)));
    });

    // White noise
    group.bench_function("white_noise_1s", |b| {
        let generator = ProceduralAudioGenerator::new();
        let params = ProceduralAudioParams::WhiteNoise { duration: 1.0, sample_rate: 44100 };
        b.iter(|| generator.generate(black_box(0), black_box(&params)));
    });

    group.finish();
}

fn bench_audio_sample_rates(c: &mut Criterion) {
    let mut group = c.benchmark_group("procedural_audio_sample_rates");

    // Measure impact of sample rate
    for sample_rate in [8000, 22050, 44100, 48000, 96000] {
        group.throughput(Throughput::Elements(sample_rate as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(sample_rate),
            &sample_rate,
            |b, &sample_rate| {
                let generator = ProceduralAudioGenerator::new();
                let params = ProceduralAudioParams::SineWave {
                    frequency: 440.0,
                    duration: 1.0,
                    sample_rate,
                };
                b.iter(|| generator.generate(black_box(0), black_box(&params)));
            },
        );
    }

    group.finish();
}

fn bench_audio_id_computation(c: &mut Criterion) {
    let mut group = c.benchmark_group("procedural_audio_id_computation");

    let params =
        ProceduralAudioParams::SineWave { frequency: 440.0, duration: 1.0, sample_rate: 44100 };

    group.bench_function("compute_id", |b| {
        b.iter(|| ProceduralAudioGenerator::compute_id(black_box(42), black_box(&params)));
    });

    group.finish();
}

// ============================================================================
// Memory Usage Benchmarks
// ============================================================================

fn bench_memory_vs_complexity(c: &mut Criterion) {
    let mut group = c.benchmark_group("procedural_memory_vs_complexity");

    // Mesh memory scaling
    for subdivisions in [10, 50, 100] {
        group.bench_with_input(
            BenchmarkId::new("mesh_memory", subdivisions),
            &subdivisions,
            |b, &subdivisions| {
                let generator = ProceduralMeshGenerator::new();
                let params = ProceduralMeshParams::Plane {
                    width: 10.0,
                    height: 10.0,
                    subdivisions_x: subdivisions,
                    subdivisions_y: subdivisions,
                };
                b.iter(|| {
                    let mesh = generator.generate(black_box(0), black_box(&params));
                    black_box(mesh.vertex_count());
                });
            },
        );
    }

    // Texture memory scaling
    for size in [256, 512, 1024] {
        group.bench_with_input(BenchmarkId::new("texture_memory", size), &size, |b, &size| {
            let generator = ProceduralTextureGenerator::new();
            let params = ProceduralTextureParams::SolidColor {
                width: size,
                height: size,
                color: [128, 128, 128, 255],
            };
            b.iter(|| {
                let texture = generator.generate(black_box(0), black_box(&params));
                black_box(texture.memory_size());
            });
        });
    }

    group.finish();
}

// ============================================================================
// Cache Hit Rate Simulation
// ============================================================================

fn bench_cache_hit_rate(c: &mut Criterion) {
    let mut group = c.benchmark_group("procedural_cache_simulation");

    // Simulate cache hits (same parameters)
    group.bench_function("cache_hit_same_params", |b| {
        let _generator = ProceduralMeshGenerator::new();
        let params = ProceduralMeshParams::Cube { size: Vec3::new(1.0, 1.0, 1.0) };

        b.iter(|| {
            // In a real cache, this would return cached result
            let id = ProceduralMeshGenerator::compute_id(black_box(0), black_box(&params));
            black_box(id);
        });
    });

    // Simulate cache misses (different seeds)
    group.bench_function("cache_miss_different_seeds", |b| {
        let _generator = ProceduralMeshGenerator::new();
        let params = ProceduralMeshParams::Cube { size: Vec3::new(1.0, 1.0, 1.0) };
        let mut seed = 0u64;

        b.iter(|| {
            seed = seed.wrapping_add(1);
            let id = ProceduralMeshGenerator::compute_id(black_box(seed), black_box(&params));
            black_box(id);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_mesh_generation,
    bench_mesh_vertex_count,
    bench_mesh_id_computation,
    bench_texture_generation,
    bench_texture_octaves,
    bench_texture_id_computation,
    bench_audio_generation,
    bench_audio_sample_rates,
    bench_audio_id_computation,
    bench_memory_vs_complexity,
    bench_cache_hit_rate,
);

criterion_main!(benches);
