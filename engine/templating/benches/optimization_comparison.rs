//! Benchmark comparing original vs optimized template loader.
//!
//! This benchmark directly compares the performance improvements from:
//! 1. YAML AST caching
//! 2. Arc-based template sharing (instead of cloning)
//! 3. Static dispatch for component parsing
//! 4. String interning for component names
//!
//! Target: 2x overall improvement

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use engine_core::ecs::World;
use engine_core::gameplay::Health;
use engine_core::math::Transform;
use engine_templating::{TemplateLoader, TemplateLoaderOptimized};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create a template file
fn create_template_file(dir: &TempDir, name: &str, content: &str) -> PathBuf {
    let path = dir.path().join(name);
    fs::write(&path, content).expect("Failed to write template file");
    path
}

/// Generate a template with N entities
fn generate_template(entity_count: usize) -> String {
    let mut yaml = r#"metadata:
  name: "Optimization Comparison"
  description: "Comparing original vs optimized loader"
  version: "1.0"

entities:
"#
    .to_string();

    for i in 0..entity_count {
        yaml.push_str(&format!(
            r#"  Entity{}:
    source:
      components:
        Transform:
          position: [{}, 0, 0]
          rotation: [0, 0, 0, 1]
          scale: [1, 1, 1]
        Health:
          current: 100.0
          max: 100.0
      tags: [benchmark]
    overrides: {{}}
    children: {{}}
"#,
            i, i
        ));
    }

    yaml
}

/// Benchmark: Original loader - Small template
fn bench_original_small(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let yaml = generate_template(1);
    let template_path = create_template_file(&temp_dir, "original_small.yaml", &yaml);

    c.bench_function("original_small_1_entity", |b| {
        b.iter(|| {
            let mut world = World::new();
            world.register::<Transform>();
            world.register::<Health>();

            let mut loader = TemplateLoader::new();
            let instance = loader.load(&mut world, black_box(&template_path)).unwrap();
            black_box(instance);
        });
    });
}

/// Benchmark: Optimized loader - Small template
fn bench_optimized_small(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let yaml = generate_template(1);
    let template_path = create_template_file(&temp_dir, "optimized_small.yaml", &yaml);

    c.bench_function("optimized_small_1_entity", |b| {
        b.iter(|| {
            let mut world = World::new();
            world.register::<Transform>();
            world.register::<Health>();

            let mut loader = TemplateLoaderOptimized::new();
            let instance = loader.load(&mut world, black_box(&template_path)).unwrap();
            black_box(instance);
        });
    });
}

/// Benchmark: Original loader - Medium template
fn bench_original_medium(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let yaml = generate_template(100);
    let template_path = create_template_file(&temp_dir, "original_medium.yaml", &yaml);

    c.bench_function("original_medium_100_entities", |b| {
        b.iter(|| {
            let mut world = World::new();
            world.register::<Transform>();
            world.register::<Health>();

            let mut loader = TemplateLoader::new();
            let instance = loader.load(&mut world, black_box(&template_path)).unwrap();
            black_box(instance);
        });
    });
}

/// Benchmark: Optimized loader - Medium template
fn bench_optimized_medium(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let yaml = generate_template(100);
    let template_path = create_template_file(&temp_dir, "optimized_medium.yaml", &yaml);

    c.bench_function("optimized_medium_100_entities", |b| {
        b.iter(|| {
            let mut world = World::new();
            world.register::<Transform>();
            world.register::<Health>();

            let mut loader = TemplateLoaderOptimized::new();
            let instance = loader.load(&mut world, black_box(&template_path)).unwrap();
            black_box(instance);
        });
    });
}

/// Benchmark: Original loader - Large template
fn bench_original_large(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let yaml = generate_template(1000);
    let template_path = create_template_file(&temp_dir, "original_large.yaml", &yaml);

    c.bench_function("original_large_1000_entities", |b| {
        b.iter(|| {
            let mut world = World::new();
            world.register::<Transform>();
            world.register::<Health>();

            let mut loader = TemplateLoader::new();
            let instance = loader.load(&mut world, black_box(&template_path)).unwrap();
            black_box(instance);
        });
    });
}

/// Benchmark: Optimized loader - Large template
fn bench_optimized_large(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let yaml = generate_template(1000);
    let template_path = create_template_file(&temp_dir, "optimized_large.yaml", &yaml);

    c.bench_function("optimized_large_1000_entities", |b| {
        b.iter(|| {
            let mut world = World::new();
            world.register::<Transform>();
            world.register::<Health>();

            let mut loader = TemplateLoaderOptimized::new();
            let instance = loader.load(&mut world, black_box(&template_path)).unwrap();
            black_box(instance);
        });
    });
}

/// Benchmark: Cache hit performance comparison
fn bench_cache_hit_comparison(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let yaml = generate_template(10);
    let template_path = create_template_file(&temp_dir, "cache.yaml", &yaml);

    let mut group = c.benchmark_group("cache_hit_comparison");

    // Original loader cache hit
    group.bench_function("original_cache_hit", |b| {
        let mut world = World::new();
        world.register::<Transform>();
        world.register::<Health>();

        let mut loader = TemplateLoader::new();
        // Pre-populate cache
        loader.load(&mut world, &template_path).unwrap();

        b.iter(|| {
            let instance = loader.load(&mut world, black_box(&template_path)).unwrap();
            black_box(instance);
        });
    });

    // Optimized loader cache hit
    group.bench_function("optimized_cache_hit", |b| {
        let mut world = World::new();
        world.register::<Transform>();
        world.register::<Health>();

        let mut loader = TemplateLoaderOptimized::new();
        // Pre-populate cache
        loader.load(&mut world, &template_path).unwrap();

        b.iter(|| {
            let instance = loader.load(&mut world, black_box(&template_path)).unwrap();
            black_box(instance);
        });
    });

    group.finish();
}

/// Benchmark: Scaling comparison
fn bench_scaling_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("scaling_comparison");

    for size in [10, 50, 100, 500, 1000].iter() {
        let temp_dir = TempDir::new().unwrap();
        let yaml = generate_template(*size);
        let original_path = create_template_file(&temp_dir, "original.yaml", &yaml);
        let optimized_path = create_template_file(&temp_dir, "optimized.yaml", &yaml);

        group.bench_with_input(BenchmarkId::new("original", size), size, |b, _| {
            b.iter(|| {
                let mut world = World::new();
                world.register::<Transform>();
                world.register::<Health>();

                let mut loader = TemplateLoader::new();
                let instance = loader.load(&mut world, black_box(&original_path)).unwrap();
                black_box(instance);
            });
        });

        group.bench_with_input(BenchmarkId::new("optimized", size), size, |b, _| {
            b.iter(|| {
                let mut world = World::new();
                world.register::<Transform>();
                world.register::<Health>();

                let mut loader = TemplateLoaderOptimized::new();
                let instance = loader.load(&mut world, black_box(&optimized_path)).unwrap();
                black_box(instance);
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_original_small,
    bench_optimized_small,
    bench_original_medium,
    bench_optimized_medium,
    bench_original_large,
    bench_optimized_large,
    bench_cache_hit_comparison,
    bench_scaling_comparison,
);

criterion_main!(benches);
