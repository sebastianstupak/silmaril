//! Benchmarks for hot-reload workflow performance.
//!
//! This module benchmarks the complete hot-reload developer experience:
//! 1. File change detection
//! 2. Template reload from disk
//! 3. Entity despawn (old template)
//! 4. Entity spawn (new template)
//! 5. Full hot-reload cycle (end-to-end)
//!
//! # Performance Targets
//!
//! - File change detection: < 1ms
//! - Template reload (medium): < 10ms
//! - Despawn + respawn: < 20ms
//! - Full hot-reload cycle: < 50ms (for responsive dev experience)
//!
//! These benchmarks ensure that developers get near-instant feedback when
//! iterating on templates during development.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use engine_core::ecs::World;
use engine_core::gameplay::Health;
use engine_core::math::Transform;
use engine_templating::loader::{TemplateInstance, TemplateLoader};
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
fn generate_simple_template(entity_count: usize, version: u32) -> String {
    let mut yaml = format!(
        r#"metadata:
  name: "Hot Reload Template"
  description: "Version {}"
  version: "1.{}"

entities:
"#,
        version, version
    );

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
          current: {}
          max: 100.0
      tags: [hot_reload_test]
    overrides: {{}}
    children: {{}}
"#,
            i,
            i,
            50.0 + (version as f32 * 10.0)
        ));
    }

    yaml
}

/// Benchmark: Detect file change via metadata modification time
fn bench_detect_file_change(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let yaml = generate_simple_template(10, 1);
    let template_path = create_template_file(&temp_dir, "detect_change.yaml", &yaml);

    c.bench_function("hot_reload_detect_file_change", |b| {
        b.iter(|| {
            // Get file metadata to detect changes
            let metadata = fs::metadata(black_box(&template_path)).unwrap();
            let modified_time = metadata.modified().unwrap();
            black_box(modified_time);
        });
    });
}

/// Benchmark: Reload changed template from disk
fn bench_reload_template(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();

    let mut group = c.benchmark_group("hot_reload_template");

    for entity_count in [1, 10, 50, 100].iter() {
        let yaml = generate_simple_template(*entity_count, 1);
        let path = create_template_file(&temp_dir, &format!("reload_{}.yaml", entity_count), &yaml);

        group.bench_with_input(BenchmarkId::from_parameter(entity_count), entity_count, |b, _| {
            b.iter(|| {
                let mut world = World::new();
                world.register::<Transform>();
                world.register::<Health>();

                // Simulate reload: clear cache and load fresh
                let mut loader = TemplateLoader::new();
                loader.clear_cache();

                let instance = loader.load(&mut world, black_box(&path)).unwrap();
                black_box(instance);
            });
        });
    }

    group.finish();
}

/// Benchmark: Despawn old entities + spawn new entities
fn bench_despawn_respawn(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let yaml_v1 = generate_simple_template(100, 1);
    let yaml_v2 = generate_simple_template(100, 2);

    let path_v1 = create_template_file(&temp_dir, "despawn_v1.yaml", &yaml_v1);
    let path_v2 = create_template_file(&temp_dir, "despawn_v2.yaml", &yaml_v2);

    c.bench_function("hot_reload_despawn_respawn_100_entities", |b| {
        b.iter_batched(
            || {
                // Setup: Load v1 template
                let mut world = World::new();
                world.register::<Transform>();
                world.register::<Health>();

                let mut loader = TemplateLoader::new();
                let instance_v1 = loader.load(&mut world, &path_v1).unwrap();
                (world, loader, instance_v1)
            },
            |(mut world, mut loader, instance_v1)| {
                // Benchmark: Despawn v1 and spawn v2
                instance_v1.despawn(&mut world);

                loader.clear_cache();
                let instance_v2 = loader.load(&mut world, black_box(&path_v2)).unwrap();
                black_box((world, instance_v2));
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

/// Benchmark: Full hot-reload cycle (detect → reload → despawn → spawn)
fn bench_full_hot_reload_cycle(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let yaml_v1 = generate_simple_template(50, 1);
    let template_path = create_template_file(&temp_dir, "hot_reload_cycle.yaml", &yaml_v1);

    c.bench_function("hot_reload_full_cycle_50_entities", |b| {
        b.iter_batched(
            || {
                // Setup: Load initial template
                let mut world = World::new();
                world.register::<Transform>();
                world.register::<Health>();

                let mut loader = TemplateLoader::new();
                let instance = loader.load(&mut world, &template_path).unwrap();

                // Simulate file change
                let yaml_v2 = generate_simple_template(50, 2);
                fs::write(&template_path, yaml_v2).unwrap();

                (world, loader, instance)
            },
            |(mut world, mut loader, old_instance)| {
                // Step 1: Detect file change
                let metadata = fs::metadata(black_box(&template_path)).unwrap();
                let _modified_time = metadata.modified().unwrap();

                // Step 2: Reload template (invalidate cache)
                loader.clear_cache();

                // Step 3: Despawn old entities
                old_instance.despawn(&mut world);

                // Step 4: Spawn new entities
                let new_instance = loader.load(&mut world, black_box(&template_path)).unwrap();

                black_box((world, new_instance));
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

/// Benchmark: Cache invalidation time
fn bench_cache_invalidation(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let yaml = generate_simple_template(100, 1);
    let template_path = create_template_file(&temp_dir, "cache_invalidation.yaml", &yaml);

    c.bench_function("hot_reload_cache_invalidation", |b| {
        b.iter_batched(
            || {
                let mut world = World::new();
                world.register::<Transform>();
                world.register::<Health>();

                let mut loader = TemplateLoader::new();
                // Pre-populate cache
                loader.load(&mut world, &template_path).unwrap();
                loader
            },
            |mut loader| {
                // Benchmark: Clear cache
                loader.clear_cache();
                black_box(loader);
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

/// Benchmark: Full hot-reload cycle with varying template sizes
fn bench_full_cycle_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("hot_reload_full_cycle_scaling");

    for entity_count in [1, 10, 50, 100, 500].iter() {
        let temp_dir = TempDir::new().unwrap();
        let yaml_v1 = generate_simple_template(*entity_count, 1);
        let template_path = create_template_file(
            &temp_dir,
            &format!("cycle_scaling_{}.yaml", entity_count),
            &yaml_v1,
        );

        group.bench_with_input(BenchmarkId::from_parameter(entity_count), entity_count, |b, _| {
            b.iter_batched(
                || {
                    let mut world = World::new();
                    world.register::<Transform>();
                    world.register::<Health>();

                    let mut loader = TemplateLoader::new();
                    let instance = loader.load(&mut world, &template_path).unwrap();

                    // Simulate file change
                    let yaml_v2 = generate_simple_template(*entity_count, 2);
                    fs::write(&template_path, yaml_v2).unwrap();

                    (world, loader, instance)
                },
                |(mut world, mut loader, old_instance)| {
                    // Full hot-reload cycle
                    let _metadata = fs::metadata(black_box(&template_path)).unwrap();
                    loader.clear_cache();
                    old_instance.despawn(&mut world);
                    let new_instance = loader.load(&mut world, black_box(&template_path)).unwrap();
                    black_box((world, new_instance));
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

/// Benchmark: File modification timestamp check (polling strategy)
fn bench_file_timestamp_check(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let yaml = generate_simple_template(10, 1);
    let template_path = create_template_file(&temp_dir, "timestamp.yaml", &yaml);

    // Get initial timestamp
    let initial_metadata = fs::metadata(&template_path).unwrap();
    let initial_time = initial_metadata.modified().unwrap();

    c.bench_function("hot_reload_timestamp_check", |b| {
        b.iter(|| {
            let metadata = fs::metadata(black_box(&template_path)).unwrap();
            let current_time = metadata.modified().unwrap();
            let _has_changed = current_time > initial_time;
            black_box(_has_changed);
        });
    });
}

/// Benchmark: Simulate rapid iteration (multiple reloads in sequence)
fn bench_rapid_iteration(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let template_path = temp_dir.path().join("rapid_iteration.yaml");

    c.bench_function("hot_reload_rapid_iteration_10_cycles", |b| {
        b.iter(|| {
            let mut world = World::new();
            world.register::<Transform>();
            world.register::<Health>();

            let mut loader = TemplateLoader::new();
            let mut instance: Option<TemplateInstance> = None;

            // Simulate 10 rapid iterations
            for version in 0..10 {
                // Despawn previous version
                if let Some(old_instance) = instance.take() {
                    old_instance.despawn(&mut world);
                }

                // Write new version
                let yaml = generate_simple_template(20, version);
                fs::write(&template_path, yaml).unwrap();

                // Reload
                loader.clear_cache();
                instance = Some(loader.load(&mut world, black_box(&template_path)).unwrap());
            }

            black_box((world, instance));
        });
    });
}

/// Benchmark: Hot-reload with template references
fn bench_hot_reload_with_references(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();

    // Create referenced template
    let referenced_yaml = r#"metadata:
  name: "Referenced Component"

entities:
  Root:
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
    let ref_path = create_template_file(&temp_dir, "referenced_component.yaml", referenced_yaml);

    // Create main template with references
    let main_yaml = format!(
        r#"metadata:
  name: "Main Template with References"

entities:
  Ref0:
    template: "{}"
    overrides: {{}}
    children: {{}}
  Ref1:
    template: "{}"
    overrides: {{}}
    children: {{}}
  Ref2:
    template: "{}"
    overrides: {{}}
    children: {{}}
"#,
        ref_path.display(),
        ref_path.display(),
        ref_path.display()
    );

    let main_path = create_template_file(&temp_dir, "main_with_refs.yaml", &main_yaml);

    c.bench_function("hot_reload_with_references", |b| {
        b.iter_batched(
            || {
                let mut world = World::new();
                world.register::<Transform>();
                world.register::<Health>();

                let mut loader = TemplateLoader::new();
                let instance = loader.load(&mut world, &main_path).unwrap();

                // Modify referenced template
                let new_referenced_yaml = r#"metadata:
  name: "Referenced Component (Modified)"

entities:
  Root:
    components:
      Transform:
        position: [1, 1, 1]
        rotation: [0, 0, 0, 1]
        scale: [2, 2, 2]
      Health:
        current: 150.0
        max: 150.0
    tags: [modified]
    overrides: {}
    children: {}
"#;
                fs::write(&ref_path, new_referenced_yaml).unwrap();

                (world, loader, instance)
            },
            |(mut world, mut loader, old_instance)| {
                // Hot-reload cycle
                loader.clear_cache();
                old_instance.despawn(&mut world);
                let new_instance = loader.load(&mut world, black_box(&main_path)).unwrap();
                black_box((world, new_instance));
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

/// Benchmark: Memory overhead of tracking file timestamps
fn bench_timestamp_tracking_overhead(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let mut paths = Vec::new();

    // Create 100 template files
    for i in 0..100 {
        let yaml = generate_simple_template(1, i);
        let path = create_template_file(&temp_dir, &format!("tracked_{}.yaml", i), &yaml);
        paths.push(path);
    }

    c.bench_function("hot_reload_track_100_file_timestamps", |b| {
        b.iter(|| {
            let mut timestamps = Vec::with_capacity(100);

            for path in &paths {
                if let Ok(metadata) = fs::metadata(path) {
                    if let Ok(modified) = metadata.modified() {
                        timestamps.push((path.clone(), modified));
                    }
                }
            }

            black_box(timestamps);
        });
    });
}

criterion_group!(
    benches,
    bench_detect_file_change,
    bench_reload_template,
    bench_despawn_respawn,
    bench_full_hot_reload_cycle,
    bench_cache_invalidation,
    bench_full_cycle_scaling,
    bench_file_timestamp_check,
    bench_rapid_iteration,
    bench_hot_reload_with_references,
    bench_timestamp_tracking_overhead,
);

criterion_main!(benches);
