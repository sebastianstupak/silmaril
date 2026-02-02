//! Font loading and operations benchmarks

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use engine_assets::{FontData, FontMetrics, FontStyle, FontWeight};

/// Benchmark font weight conversion operations
fn bench_font_weight_conversion(c: &mut Criterion) {
    let mut group = c.benchmark_group("font_weight");

    group.bench_function("from_value", |b| {
        b.iter(|| {
            for value in [100, 200, 300, 400, 500, 600, 700, 800, 900] {
                black_box(FontWeight::from_value(value));
            }
        });
    });

    group.bench_function("to_value", |b| {
        let weights = [
            FontWeight::Thin,
            FontWeight::ExtraLight,
            FontWeight::Light,
            FontWeight::Normal,
            FontWeight::Medium,
            FontWeight::SemiBold,
            FontWeight::Bold,
            FontWeight::ExtraBold,
            FontWeight::Black,
        ];
        b.iter(|| {
            for weight in &weights {
                black_box(weight.to_value());
            }
        });
    });

    group.finish();
}

/// Benchmark font metrics calculations
fn bench_font_metrics(c: &mut Criterion) {
    let mut group = c.benchmark_group("font_metrics");

    let metrics = FontMetrics::new(800, -200, 100, 2048);

    group.bench_function("line_height", |b| {
        b.iter(|| black_box(metrics.line_height()));
    });

    group.bench_function("create", |b| {
        b.iter(|| {
            black_box(FontMetrics::new(
                black_box(800),
                black_box(-200),
                black_box(100),
                black_box(2048),
            ))
        });
    });

    group.finish();
}

/// Benchmark font data construction
fn bench_font_data_construction(c: &mut Criterion) {
    let mut group = c.benchmark_group("font_construction");

    let small_data = vec![0u8; 1024]; // 1KB
    let medium_data = vec![0u8; 100_000]; // 100KB
    let large_data = vec![0u8; 1_000_000]; // 1MB

    group.bench_function("small_1kb", |b| {
        b.iter(|| {
            FontData::new(
                black_box("Test Font".to_string()),
                black_box(FontStyle::Normal),
                black_box(FontWeight::Normal),
                black_box(small_data.clone()),
                black_box(FontMetrics::new(800, -200, 100, 2048)),
            )
        });
    });

    group.bench_function("medium_100kb", |b| {
        b.iter(|| {
            FontData::new(
                black_box("Test Font".to_string()),
                black_box(FontStyle::Normal),
                black_box(FontWeight::Normal),
                black_box(medium_data.clone()),
                black_box(FontMetrics::new(800, -200, 100, 2048)),
            )
        });
    });

    group.bench_function("large_1mb", |b| {
        b.iter(|| {
            FontData::new(
                black_box("Test Font".to_string()),
                black_box(FontStyle::Normal),
                black_box(FontWeight::Normal),
                black_box(large_data.clone()),
                black_box(FontMetrics::new(800, -200, 100, 2048)),
            )
        });
    });

    group.finish();
}

/// Benchmark font data serialization
fn bench_font_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("font_serialization");

    let test_data = vec![0u8; 10_000];
    let font = FontData::new(
        "Benchmark Font".to_string(),
        FontStyle::Normal,
        FontWeight::Normal,
        test_data,
        FontMetrics::new(800, -200, 100, 2048),
    );

    group.bench_function("serialize_10kb", |b| {
        b.iter(|| bincode::serialize(black_box(&font)).unwrap());
    });

    let serialized = bincode::serialize(&font).unwrap();
    group.bench_function("deserialize_10kb", |b| {
        b.iter(|| bincode::deserialize::<FontData>(black_box(&serialized)).unwrap());
    });

    // Test with larger font
    let large_data = vec![0u8; 100_000];
    let large_font = FontData::new(
        "Large Font".to_string(),
        FontStyle::Bold,
        FontWeight::Bold,
        large_data,
        FontMetrics::new(1000, -300, 150, 2048),
    );

    group.bench_function("serialize_100kb", |b| {
        b.iter(|| bincode::serialize(black_box(&large_font)).unwrap());
    });

    let large_serialized = bincode::serialize(&large_font).unwrap();
    group.bench_function("deserialize_100kb", |b| {
        b.iter(|| bincode::deserialize::<FontData>(black_box(&large_serialized)).unwrap());
    });

    group.finish();
}

/// Benchmark font memory usage calculation
fn bench_font_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("font_operations");

    let test_data = vec![0u8; 10_000];
    let font = FontData::new(
        "Test Font".to_string(),
        FontStyle::Normal,
        FontWeight::Normal,
        test_data,
        FontMetrics::new(800, -200, 100, 2048),
    );

    group.bench_function("memory_usage", |b| {
        b.iter(|| black_box(font.memory_usage()));
    });

    // Note: glyph_count() requires re-parsing font data, which would fail for mock data
    // In production, this would be benchmarked with real font files

    group.finish();
}

/// Benchmark font style comparisons
fn bench_font_comparisons(c: &mut Criterion) {
    let mut group = c.benchmark_group("font_comparisons");

    let metrics1 = FontMetrics::new(800, -200, 100, 2048);
    let metrics2 = FontMetrics::new(800, -200, 100, 2048);

    group.bench_function("metrics_equality", |b| {
        b.iter(|| black_box(metrics1 == metrics2));
    });

    let style1 = FontStyle::Normal;
    let style2 = FontStyle::Italic;

    group.bench_function("style_equality", |b| {
        b.iter(|| {
            black_box(style1 == style2);
            black_box(style1 != style2);
        });
    });

    let weight1 = FontWeight::Normal;
    let weight2 = FontWeight::Bold;

    group.bench_function("weight_equality", |b| {
        b.iter(|| {
            black_box(weight1 == weight2);
            black_box(weight1 != weight2);
        });
    });

    group.finish();
}

// Note: Benchmarks for actual TTF/OTF parsing require real font files
// and would measure:
// - TTF parsing speed (small, medium, large fonts)
// - OTF parsing speed
// - Glyph count extraction
// - Font metadata extraction
//
// Example with real fonts (commented out):
//
// fn bench_ttf_parsing(c: &mut Criterion) {
//     let font_bytes = include_bytes!("../test_data/test_font.ttf");
//     c.bench_function("parse_ttf", |b| {
//         b.iter(|| FontData::from_ttf(black_box(font_bytes)).unwrap());
//     });
// }

criterion_group!(
    benches,
    bench_font_weight_conversion,
    bench_font_metrics,
    bench_font_data_construction,
    bench_font_serialization,
    bench_font_operations,
    bench_font_comparisons,
);
criterion_main!(benches);
