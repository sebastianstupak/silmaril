//! Texture loading and mipmap generation benchmarks

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use engine_assets::{TextureData, TextureFormat};
use image::{ImageBuffer, Rgba};

/// Benchmark PNG loading
fn bench_png_load(c: &mut Criterion) {
    let mut group = c.benchmark_group("texture_png_load");

    for size in [256, 512, 1024].iter() {
        // Create test PNG
        let img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_fn(*size, *size, |x, y| {
            Rgba([(x % 256) as u8, (y % 256) as u8, ((x + y) % 256) as u8, 255])
        });

        let mut png_data = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut png_data), image::ImageFormat::Png)
            .unwrap();

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}x{}", size, size)),
            size,
            |b, _| {
                b.iter(|| TextureData::from_image_bytes(black_box(&png_data)).unwrap());
            },
        );
    }

    group.finish();
}

/// Benchmark JPEG loading
fn bench_jpeg_load(c: &mut Criterion) {
    let mut group = c.benchmark_group("texture_jpeg_load");

    for size in [256, 512, 1024].iter() {
        // Create test JPEG
        let img: ImageBuffer<image::Rgb<u8>, Vec<u8>> =
            ImageBuffer::from_fn(*size, *size, |x, y| {
                image::Rgb([(x % 256) as u8, (y % 256) as u8, ((x + y) % 256) as u8])
            });

        let mut jpg_data = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut jpg_data), image::ImageFormat::Jpeg)
            .unwrap();

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}x{}", size, size)),
            size,
            |b, _| {
                b.iter(|| TextureData::from_image_bytes(black_box(&jpg_data)).unwrap());
            },
        );
    }

    group.finish();
}

/// Benchmark mipmap generation
fn bench_mipmap_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("texture_mipmap_generation");

    for size in [256, 512, 1024, 2048].iter() {
        let data = vec![128u8; (size * size * 4) as usize];
        let texture = TextureData::new(*size, *size, TextureFormat::RGBA8Unorm, data).unwrap();

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}x{}", size, size)),
            size,
            |b, _| {
                b.iter(|| black_box(texture.clone()).generate_mipmaps().unwrap());
            },
        );
    }

    group.finish();
}

/// Benchmark texture memory size calculation
fn bench_memory_size(c: &mut Criterion) {
    c.bench_function("texture_memory_size_calculation", |b| {
        let data = vec![128u8; 1024 * 1024 * 4];
        let texture = TextureData::new(1024, 1024, TextureFormat::RGBA8Unorm, data).unwrap();

        b.iter(|| black_box(&texture).memory_size());
    });
}

criterion_group!(
    benches,
    bench_png_load,
    bench_jpeg_load,
    bench_mipmap_generation,
    bench_memory_size
);
criterion_main!(benches);
