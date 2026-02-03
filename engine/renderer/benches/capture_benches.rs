//! Benchmarks for frame capture system
//!
//! Measures performance of:
//! - PNG encoding
//! - JPEG encoding
//! - Raw data throughput

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_renderer::{CaptureFormat, FrameEncoder};

/// Benchmark PNG encoding at different resolutions
fn bench_png_encoding(c: &mut Criterion) {
    let mut group = c.benchmark_group("png_encoding");

    let resolutions = [
        ("720p", 1280, 720),
        ("1080p", 1920, 1080),
        ("1440p", 2560, 1440),
        ("4K", 3840, 2160),
    ];

    for (name, width, height) in resolutions {
        let size = (width * height * 4) as usize;
        let data = vec![128u8; size]; // Gray image

        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            &(width, height),
            |b, &(w, h)| {
                b.iter(|| {
                    let result =
                        FrameEncoder::encode_png(black_box(&data), black_box(w), black_box(h));
                    assert!(result.is_ok());
                });
            },
        );
    }

    group.finish();
}

/// Benchmark JPEG encoding at different quality levels
fn bench_jpeg_encoding(c: &mut Criterion) {
    let mut group = c.benchmark_group("jpeg_encoding");

    let width = 1920;
    let height = 1080;
    let size = (width * height * 4) as usize;
    let data = vec![128u8; size];

    group.throughput(Throughput::Bytes(size as u64));

    for quality in [50, 75, 90, 100] {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("quality_{}", quality)),
            &quality,
            |b, &q| {
                b.iter(|| {
                    let result = FrameEncoder::encode_jpeg(
                        black_box(&data),
                        black_box(width),
                        black_box(height),
                        black_box(q),
                    );
                    assert!(result.is_ok());
                });
            },
        );
    }

    group.finish();
}

/// Benchmark PNG vs JPEG at same resolution
fn bench_format_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("format_comparison");

    let width = 1920;
    let height = 1080;
    let size = (width * height * 4) as usize;
    let data = vec![128u8; size];

    group.throughput(Throughput::Bytes(size as u64));

    group.bench_function("png", |b| {
        b.iter(|| {
            let result =
                FrameEncoder::encode_png(black_box(&data), black_box(width), black_box(height));
            assert!(result.is_ok());
        });
    });

    group.bench_function("jpeg_q90", |b| {
        b.iter(|| {
            let result = FrameEncoder::encode_jpeg(
                black_box(&data),
                black_box(width),
                black_box(height),
                90,
            );
            assert!(result.is_ok());
        });
    });

    group.finish();
}

/// Benchmark file save operations
fn bench_save_to_file(c: &mut Criterion) {
    let mut group = c.benchmark_group("save_to_file");

    let width = 1920;
    let height = 1080;
    let size = (width * height * 4) as usize;
    let data = vec![128u8; size];

    let temp_dir = std::env::temp_dir().join("capture_bench");
    std::fs::create_dir_all(&temp_dir).ok();

    group.throughput(Throughput::Bytes(size as u64));

    group.bench_function("png", |b| {
        let path = temp_dir.join("bench.png");
        b.iter(|| {
            let result = FrameEncoder::save_to_file(
                black_box(&data),
                black_box(width),
                black_box(height),
                black_box(&path),
                black_box(CaptureFormat::Png),
            );
            assert!(result.is_ok());
        });
    });

    group.bench_function("jpeg_q85", |b| {
        let path = temp_dir.join("bench.jpg");
        b.iter(|| {
            let result = FrameEncoder::save_to_file(
                black_box(&data),
                black_box(width),
                black_box(height),
                black_box(&path),
                black_box(CaptureFormat::Jpeg { quality: 85 }),
            );
            assert!(result.is_ok());
        });
    });

    // Cleanup
    std::fs::remove_dir_all(temp_dir).ok();

    group.finish();
}

criterion_group!(
    benches,
    bench_png_encoding,
    bench_jpeg_encoding,
    bench_format_comparison,
    bench_save_to_file
);
criterion_main!(benches);
