//! Benchmarks for template validation.
//!
//! Measures validation performance for templates of different sizes:
//! - Small template (1 entity): < 1ms
//! - Medium template (100 entities): < 10ms
//! - Large template (1000 entities): < 50ms

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use engine_templating::template::{EntityDefinition, Template, TemplateMetadata};
use engine_templating::validator::TemplateValidator;
use rustc_hash::FxHashMap;
use std::fs;
use tempfile::TempDir;

/// Creates a template with a specified number of entities.
fn create_template(entity_count: usize) -> Template {
    let metadata = TemplateMetadata {
        name: Some(format!("Benchmark Template ({} entities)", entity_count)),
        description: Some("Template for validation benchmarking".to_string()),
        author: Some("Benchmark".to_string()),
        version: Some("1.0".to_string()),
    };

    let mut template = Template::new(metadata);

    for i in 0..entity_count {
        let mut components = FxHashMap::default();
        components.insert("Transform".to_string(), serde_yaml::Value::Null);
        components.insert("Health".to_string(), serde_yaml::Value::Null);

        let entity = EntityDefinition::new_inline(components, vec![format!("entity_{}", i)]);

        template.add_entity(format!("Entity_{}", i), entity);
    }

    template
}

/// Writes a template to a temporary file and returns the path.
fn write_template_to_temp(template: &Template) -> (TempDir, std::path::PathBuf) {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let path = temp_dir.path().join("template.yaml");
    let yaml = serde_yaml::to_string(template).expect("Failed to serialize template");
    fs::write(&path, yaml).expect("Failed to write template file");
    (temp_dir, path)
}

fn bench_validate_small(c: &mut Criterion) {
    let validator = TemplateValidator::new();
    let template = create_template(1);
    let (_temp_dir, path) = write_template_to_temp(&template);

    c.bench_function("validate_small_template", |b| {
        b.iter(|| {
            let report = validator.validate(black_box(&path)).expect("Validation failed");
            black_box(report);
        });
    });
}

fn bench_validate_medium(c: &mut Criterion) {
    let validator = TemplateValidator::new();
    let template = create_template(100);
    let (_temp_dir, path) = write_template_to_temp(&template);

    c.bench_function("validate_medium_template", |b| {
        b.iter(|| {
            let report = validator.validate(black_box(&path)).expect("Validation failed");
            black_box(report);
        });
    });
}

fn bench_validate_large(c: &mut Criterion) {
    let validator = TemplateValidator::new();
    let template = create_template(1000);
    let (_temp_dir, path) = write_template_to_temp(&template);

    c.bench_function("validate_large_template", |b| {
        b.iter(|| {
            let report = validator.validate(black_box(&path)).expect("Validation failed");
            black_box(report);
        });
    });
}

fn bench_validate_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("validate_scaling");

    for size in [1, 10, 50, 100, 500, 1000].iter() {
        let validator = TemplateValidator::new();
        let template = create_template(*size);
        let (_temp_dir, path) = write_template_to_temp(&template);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let report = validator.validate(black_box(&path)).expect("Validation failed");
                black_box(report);
            });
        });
    }

    group.finish();
}

fn bench_validate_with_children(c: &mut Criterion) {
    let validator = TemplateValidator::new();

    // Create template with nested children (10 parents, each with 10 children)
    let metadata = TemplateMetadata {
        name: Some("Nested Template".to_string()),
        description: Some("Template with nested entities".to_string()),
        author: Some("Benchmark".to_string()),
        version: Some("1.0".to_string()),
    };

    let mut template = Template::new(metadata);

    for i in 0..10 {
        let mut parent =
            EntityDefinition::new_inline(FxHashMap::default(), vec![format!("parent_{}", i)]);

        for j in 0..10 {
            let mut components = FxHashMap::default();
            components.insert("Transform".to_string(), serde_yaml::Value::Null);

            let child =
                EntityDefinition::new_inline(components, vec![format!("child_{}_{}", i, j)]);

            parent.add_child(format!("Child_{}", j), child);
        }

        template.add_entity(format!("Parent_{}", i), parent);
    }

    let (_temp_dir, path) = write_template_to_temp(&template);

    c.bench_function("validate_with_children", |b| {
        b.iter(|| {
            let report = validator.validate(black_box(&path)).expect("Validation failed");
            black_box(report);
        });
    });
}

fn bench_validate_unknown_components(c: &mut Criterion) {
    let validator = TemplateValidator::new();

    // Create template with many unknown components (should be slower due to errors)
    let metadata = TemplateMetadata {
        name: Some("Invalid Template".to_string()),
        description: None,
        author: None,
        version: None,
    };

    let mut template = Template::new(metadata);

    for i in 0..100 {
        let mut components = FxHashMap::default();
        components.insert(format!("UnknownComponent_{}", i), serde_yaml::Value::Null);

        let entity = EntityDefinition::new_inline(components, vec![]);
        template.add_entity(format!("Entity_{}", i), entity);
    }

    let (_temp_dir, path) = write_template_to_temp(&template);

    c.bench_function("validate_unknown_components", |b| {
        b.iter(|| {
            let report = validator.validate(black_box(&path)).expect("Validation failed");
            black_box(report);
        });
    });
}

criterion_group!(
    benches,
    bench_validate_small,
    bench_validate_medium,
    bench_validate_large,
    bench_validate_scaling,
    bench_validate_with_children,
    bench_validate_unknown_components,
);
criterion_main!(benches);
