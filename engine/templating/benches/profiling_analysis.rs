//! Profiling analysis benchmark to identify bottlenecks in template loading.
//!
//! This benchmark uses criterion's profiler support to generate detailed
//! performance data that can be analyzed to find optimization opportunities.
//!
//! Run with: cargo bench --bench profiling_analysis -- --profile-time=5

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use engine_core::ecs::World;
use engine_core::gameplay::Health;
use engine_core::math::Transform;
use engine_templating::loader::TemplateLoader;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create a template file
fn create_template_file(dir: &TempDir, name: &str, content: &str) -> PathBuf {
    let path = dir.path().join(name);
    fs::write(&path, content).expect("Failed to write template file");
    path
}

/// Generate a template with N entities for profiling
fn generate_template(entity_count: usize) -> String {
    let mut yaml = r#"metadata:
  name: "Profiling Template"
  description: "Template for profiling analysis"
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
      tags: [benchmark, entity]
    overrides: {{}}
    children: {{}}
"#,
            i, i
        ));
    }

    yaml
}

/// Profile the entire loading pipeline
fn profile_full_pipeline(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let yaml = generate_template(100);
    let template_path = create_template_file(&temp_dir, "profile.yaml", &yaml);

    c.bench_function("profile_full_pipeline_100_entities", |b| {
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

/// Profile just YAML parsing
fn profile_yaml_parsing(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let yaml = generate_template(100);
    let template_path = create_template_file(&temp_dir, "yaml_parse.yaml", &yaml);

    let yaml_content = fs::read_to_string(&template_path).unwrap();

    c.bench_function("profile_yaml_parsing_100_entities", |b| {
        b.iter(|| {
            let template: engine_templating::template::Template =
                serde_yaml::from_str(black_box(&yaml_content)).unwrap();
            black_box(template);
        });
    });
}

/// Profile component parsing and entity spawning
fn profile_entity_spawning(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let yaml = generate_template(100);
    let template_path = create_template_file(&temp_dir, "spawn.yaml", &yaml);

    c.bench_function("profile_entity_spawning_100_entities", |b| {
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

criterion_group!(benches, profile_full_pipeline, profile_yaml_parsing, profile_entity_spawning,);

criterion_main!(benches);
