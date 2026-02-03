//! Benchmarks for template loading performance.
//!
//! This module benchmarks the template loader across different scenarios:
//! - Small templates (1 entity)
//! - Medium templates (100 entities)
//! - Large templates (1000 entities)
//! - Templates with references
//! - Cache hit performance
//!
//! # Performance Targets
//!
//! - Small (1 entity): < 1ms
//! - Medium (100 entities): < 10ms
//! - Large (1000 entities): < 100ms
//! - Cache hit: < 0.1ms

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use engine_core::ecs::World;
use engine_core::gameplay::Health;
use engine_core::math::Transform;
use engine_templating::loader::TemplateLoader;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create a template file for benchmarking
fn create_template_file(dir: &TempDir, name: &str, content: &str) -> PathBuf {
    let path = dir.path().join(name);
    fs::write(&path, content).expect("Failed to write template file");
    path
}

/// Generates a simple template with N entities
fn generate_simple_template(entity_count: usize) -> String {
    let mut yaml = r#"metadata:
  name: "Benchmark Template"
  description: "Generated for benchmarking"
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

/// Generates a template with nested children
fn generate_nested_template(depth: usize) -> String {
    let mut yaml = r#"metadata:
  name: "Nested Template"

entities:
  Root:
    source:
      components:
        Transform:
          position: [0, 0, 0]
          rotation: [0, 0, 0, 1]
          scale: [1, 1, 1]
      tags: []
    overrides: {}
"#
    .to_string();

    fn generate_children(depth: usize, current: usize) -> String {
        if current >= depth {
            return "children: {}".to_string();
        }

        format!(
            r#"children:
      Child{}:
        source:
          components:
            Transform:
              position: [{}, 0, 0]
              rotation: [0, 0, 0, 1]
              scale: [1, 1, 1]
          tags: []
        overrides: {{}}
        {}"#,
            current,
            current,
            generate_children(depth, current + 1)
        )
    }

    yaml.push_str(&format!("    {}\n", generate_children(depth, 0)));

    yaml
}

/// Benchmark: Load small template (1 entity)
fn bench_load_small(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let yaml = generate_simple_template(1);
    let template_path = create_template_file(&temp_dir, "small.yaml", &yaml);

    c.bench_function("template_load_small_1_entity", |b| {
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

/// Benchmark: Load medium template (100 entities)
fn bench_load_medium(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let yaml = generate_simple_template(100);
    let template_path = create_template_file(&temp_dir, "medium.yaml", &yaml);

    c.bench_function("template_load_medium_100_entities", |b| {
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

/// Benchmark: Load large template (1000 entities)
fn bench_load_large(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let yaml = generate_simple_template(1000);
    let template_path = create_template_file(&temp_dir, "large.yaml", &yaml);

    c.bench_function("template_load_large_1000_entities", |b| {
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

/// Benchmark: Load template with references
fn bench_load_with_references(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();

    // Create a referenced template
    let referenced_yaml = r#"metadata:
  name: "Referenced Template"

entities:
  Root:
    source:
      components:
        Transform:
          position: [0, 0, 0]
          rotation: [0, 0, 0, 1]
          scale: [1, 1, 1]
        Health:
          current: 100.0
          max: 100.0
      tags: []
    overrides: {}
    children: {}
"#;
    let ref_path = create_template_file(&temp_dir, "referenced.yaml", referenced_yaml);

    // Create main template with 10 references
    let mut main_yaml = r#"metadata:
  name: "Main Template"

entities:
"#
    .to_string();

    for i in 0..10 {
        // Convert path to use forward slashes for YAML
        let template_path = ref_path.to_str().unwrap().replace('\\', "/");
        main_yaml.push_str(&format!(
            r#"  Ref{}:
    source:
      template: "{}"
    overrides:
      Transform:
        position: [{}, 0, 0]
    children: {{}}
"#,
            i, template_path, i
        ));
    }

    let main_path = create_template_file(&temp_dir, "main.yaml", &main_yaml);

    c.bench_function("template_load_with_10_references", |b| {
        b.iter(|| {
            let mut world = World::new();
            world.register::<Transform>();
            world.register::<Health>();

            let mut loader = TemplateLoader::new();
            let instance = loader.load(&mut world, black_box(&main_path)).unwrap();
            black_box(instance);
        });
    });
}

/// Benchmark: Cache hit performance
fn bench_cache_hit(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let yaml = generate_simple_template(10);
    let template_path = create_template_file(&temp_dir, "cached.yaml", &yaml);

    c.bench_function("template_load_cache_hit", |b| {
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
}

/// Benchmark: Load template with varying entity counts
fn bench_load_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("template_load_scaling");

    for entity_count in [1, 10, 50, 100, 500, 1000].iter() {
        let temp_dir = TempDir::new().unwrap();
        let yaml = generate_simple_template(*entity_count);
        let template_path = create_template_file(&temp_dir, "scaling.yaml", &yaml);

        group.bench_with_input(BenchmarkId::from_parameter(entity_count), entity_count, |b, _| {
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

    group.finish();
}

/// Benchmark: Despawn performance
fn bench_despawn(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let yaml = generate_simple_template(100);
    let template_path = create_template_file(&temp_dir, "despawn.yaml", &yaml);

    c.bench_function("template_despawn_100_entities", |b| {
        b.iter_batched(
            || {
                // Setup: Load template
                let mut world = World::new();
                world.register::<Transform>();
                world.register::<Health>();

                let mut loader = TemplateLoader::new();
                let instance = loader.load(&mut world, &template_path).unwrap();
                (world, instance)
            },
            |(mut world, instance)| {
                // Benchmark: Despawn
                instance.despawn(&mut world);
                black_box(world);
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

/// Benchmark: Nested children loading
fn bench_load_nested(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let yaml = generate_nested_template(5);
    let template_path = create_template_file(&temp_dir, "nested.yaml", &yaml);

    c.bench_function("template_load_nested_5_levels", |b| {
        b.iter(|| {
            let mut world = World::new();
            world.register::<Transform>();

            let mut loader = TemplateLoader::new();
            let instance = loader.load(&mut world, black_box(&template_path)).unwrap();
            black_box(instance);
        });
    });
}

criterion_group!(
    benches,
    bench_load_small,
    bench_load_medium,
    bench_load_large,
    bench_load_with_references,
    bench_cache_hit,
    bench_load_scaling,
    bench_despawn,
    bench_load_nested,
);

criterion_main!(benches);
