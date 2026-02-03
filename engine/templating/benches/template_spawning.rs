//! Benchmarks for template spawning performance.
//!
//! This module benchmarks the template spawning process - loading templates
//! and spawning them into the ECS World. This measures real-world performance
//! including YAML parsing, entity creation, and component assignment.
//!
//! # Performance Targets
//!
//! - Small (1 entity): < 10µs
//! - Medium (100 entities): < 1ms
//! - Large (1000 entities): < 10ms
//! - With references (5 nested): < 100µs
//! - With overrides (many components): < 50µs
//! - Batch spawning (10 templates): < 500µs

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use engine_core::ecs::World;
use engine_core::gameplay::Health;
use engine_core::math::Transform;
use engine_core::rendering::{Camera, MeshRenderer};
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
  name: "Spawn Benchmark Template"
  description: "Generated for spawning benchmarks"
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
      tags: [spawned]
    overrides: {{}}
    children: {{}}
"#,
            i, i
        ));
    }

    yaml
}

/// Generates a template with multiple component overrides
fn generate_template_with_overrides() -> String {
    r#"metadata:
  name: "Override Template"

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
        MeshRenderer:
          mesh_id: 42
          visible: true
        Camera:
          fov: 60.0
          aspect: 1.77777
          near: 0.1
          far: 1000.0
      tags: []
    overrides: {}
    children: {}
"#
    .to_string()
}

/// Generates a base template for references
fn generate_base_template() -> String {
    r#"metadata:
  name: "Base Template"

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
      tags: [base]
    overrides: {}
    children: {}
"#
    .to_string()
}

/// Generates a template with N nested references
fn generate_template_with_references(ref_count: usize, ref_path: &str) -> String {
    let mut yaml = r#"metadata:
  name: "Reference Template"

entities:
"#
    .to_string();

    for i in 0..ref_count {
        yaml.push_str(&format!(
            r#"  Ref{}:
    source:
      template: "{}"
    overrides:
      Transform:
        position: [{}, 0, 0]
    children: {{}}
"#,
            i, ref_path, i
        ));
    }

    yaml
}

/// Benchmark: Spawn small template (1 entity)
fn bench_spawn_small(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let yaml = generate_simple_template(1);
    let template_path = create_template_file(&temp_dir, "small.yaml", &yaml);

    c.bench_function("spawn_small_1_entity", |b| {
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

/// Benchmark: Spawn medium template (100 entities)
fn bench_spawn_medium(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let yaml = generate_simple_template(100);
    let template_path = create_template_file(&temp_dir, "medium.yaml", &yaml);

    c.bench_function("spawn_medium_100_entities", |b| {
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

/// Benchmark: Spawn large template (1000 entities)
fn bench_spawn_large(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let yaml = generate_simple_template(1000);
    let template_path = create_template_file(&temp_dir, "large.yaml", &yaml);

    c.bench_function("spawn_large_1000_entities", |b| {
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

/// Benchmark: Spawn template with 5 nested references
fn bench_spawn_with_references(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();

    // Create base template
    let base_yaml = generate_base_template();
    let _base_path = create_template_file(&temp_dir, "base.yaml", &base_yaml);

    // Create template with 5 references using relative path (just filename)
    let ref_yaml = generate_template_with_references(5, "base.yaml");
    let ref_template_path = create_template_file(&temp_dir, "with_refs.yaml", &ref_yaml);

    c.bench_function("spawn_with_5_references", |b| {
        b.iter(|| {
            let mut world = World::new();
            world.register::<Transform>();
            world.register::<Health>();

            let mut loader = TemplateLoader::new();
            let instance = loader.load(&mut world, black_box(&ref_template_path)).unwrap();
            black_box(instance);
        });
    });
}

/// Benchmark: Spawn template with many component overrides
fn bench_spawn_with_overrides(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let yaml = generate_template_with_overrides();
    let template_path = create_template_file(&temp_dir, "overrides.yaml", &yaml);

    c.bench_function("spawn_with_component_overrides", |b| {
        b.iter(|| {
            let mut world = World::new();
            world.register::<Transform>();
            world.register::<Health>();
            world.register::<MeshRenderer>();
            world.register::<Camera>();

            let mut loader = TemplateLoader::new();
            let instance = loader.load(&mut world, black_box(&template_path)).unwrap();
            black_box(instance);
        });
    });
}

/// Benchmark: Batch spawning - spawn 10 templates in sequence
fn bench_batch_spawning(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();

    // Create 10 different template files
    let mut template_paths = Vec::new();
    for i in 0..10 {
        let yaml = generate_simple_template(10); // Each has 10 entities
        let path = create_template_file(&temp_dir, &format!("batch_{}.yaml", i), &yaml);
        template_paths.push(path);
    }

    c.bench_function("spawn_batch_10_templates", |b| {
        b.iter(|| {
            let mut world = World::new();
            world.register::<Transform>();
            world.register::<Health>();

            let mut loader = TemplateLoader::new();

            for path in &template_paths {
                let instance = loader.load(&mut world, black_box(path)).unwrap();
                black_box(instance);
            }
        });
    });
}

/// Benchmark: Spawn with varying entity counts (scaling test)
fn bench_spawn_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("spawn_scaling");

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

/// Benchmark: Compare against baseline (simple entity spawn)
fn bench_baseline_entity_spawn(c: &mut Criterion) {
    c.bench_function("baseline_spawn_1_entity", |b| {
        b.iter(|| {
            let mut world = World::new();
            world.register::<Transform>();
            world.register::<Health>();

            let entity = world.spawn();
            world.add(entity, Transform::default());
            world.add(entity, Health::new(100.0, 100.0));
            black_box(entity);
        });
    });

    c.bench_function("baseline_spawn_100_entities", |b| {
        b.iter(|| {
            let mut world = World::new();
            world.register::<Transform>();
            world.register::<Health>();

            for _ in 0..100 {
                let entity = world.spawn();
                world.add(entity, Transform::default());
                world.add(entity, Health::new(100.0, 100.0));
                black_box(entity);
            }
        });
    });

    c.bench_function("baseline_spawn_1000_entities", |b| {
        b.iter(|| {
            let mut world = World::new();
            world.register::<Transform>();
            world.register::<Health>();

            for _ in 0..1000 {
                let entity = world.spawn();
                world.add(entity, Transform::default());
                world.add(entity, Health::new(100.0, 100.0));
                black_box(entity);
            }
        });
    });
}

/// Benchmark: Spawn with cache warmup
fn bench_spawn_with_cache(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let yaml = generate_simple_template(100);
    let template_path = create_template_file(&temp_dir, "cached.yaml", &yaml);

    c.bench_function("spawn_with_cache_warmup_100_entities", |b| {
        // Warmup cache once
        let mut world_warmup = World::new();
        world_warmup.register::<Transform>();
        world_warmup.register::<Health>();
        let mut loader = TemplateLoader::new();
        loader.load(&mut world_warmup, &template_path).unwrap();

        b.iter(|| {
            let mut world = World::new();
            world.register::<Transform>();
            world.register::<Health>();

            // Reuse the warmed-up loader
            let instance = loader.load(&mut world, black_box(&template_path)).unwrap();
            black_box(instance);
        });
    });
}

criterion_group!(
    benches,
    bench_spawn_small,
    bench_spawn_medium,
    bench_spawn_large,
    bench_spawn_with_references,
    bench_spawn_with_overrides,
    bench_batch_spawning,
    bench_spawn_scaling,
    bench_baseline_entity_spawn,
    bench_spawn_with_cache,
);

criterion_main!(benches);
