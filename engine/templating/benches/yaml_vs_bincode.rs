//! Benchmarks comparing YAML vs Bincode template loading performance.
//!
//! This benchmark suite measures:
//! - Load time (parse time) for YAML vs Bincode
//! - File size comparison
//! - Memory usage comparison
//!
//! Expected results:
//! - Bincode should be 10-50x faster to parse
//! - Bincode should be 50-80% smaller than YAML

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_templating::compiler::TemplateCompiler;
use engine_templating::template::{EntityDefinition, Template, TemplateMetadata};
use rustc_hash::FxHashMap;
use std::fs;
use tempfile::TempDir;

/// Creates a template with the specified number of entities
fn create_template(entity_count: usize) -> Template {
    let metadata = TemplateMetadata {
        name: Some(format!("Benchmark Template ({})", entity_count)),
        description: Some("A template for benchmarking".to_string()),
        author: Some("Benchmark Suite".to_string()),
        version: Some("1.0.0".to_string()),
    };

    let mut template = Template::new(metadata);

    for i in 0..entity_count {
        let mut components = FxHashMap::default();

        // Add some component data to make it realistic
        let transform_data: FxHashMap<&str, Vec<f64>> = FxHashMap::from_iter([
            ("position", vec![i as f64, 0.0, 0.0]),
            ("rotation", vec![0.0, 0.0, 0.0, 1.0]),
            ("scale", vec![1.0, 1.0, 1.0]),
        ]);
        components.insert(
            "Transform".to_string(),
            serde_yaml::to_value(transform_data).unwrap_or(serde_yaml::Value::Null),
        );

        let health_data: FxHashMap<&str, f64> =
            FxHashMap::from_iter([("current", 100.0), ("max", 100.0)]);
        components.insert(
            "Health".to_string(),
            serde_yaml::to_value(health_data).unwrap_or(serde_yaml::Value::Null),
        );

        let entity = EntityDefinition::new_inline(
            components,
            vec![format!("entity_{}", i), "replicate".to_string()],
        );

        template.add_entity(format!("Entity_{}", i), entity);
    }

    template
}

/// Benchmark loading YAML templates
fn bench_yaml_load(c: &mut Criterion) {
    let mut group = c.benchmark_group("yaml_load");

    for entity_count in [1, 10, 100, 1000].iter() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let yaml_path = temp_dir.path().join("template.yaml");

        // Create and save template
        let template = create_template(*entity_count);
        let yaml = serde_yaml::to_string(&template).expect("Failed to serialize to YAML");
        let yaml_size = yaml.len();
        fs::write(&yaml_path, yaml).expect("Failed to write YAML");

        group.throughput(Throughput::Bytes(yaml_size as u64));
        group.bench_with_input(BenchmarkId::new("entities", entity_count), entity_count, |b, _| {
            b.iter(|| {
                let yaml_content = fs::read_to_string(&yaml_path).expect("Failed to read YAML");
                let _template: Template =
                    serde_yaml::from_str(black_box(&yaml_content)).expect("Failed to parse YAML");
            });
        });
    }

    group.finish();
}

/// Benchmark loading Bincode templates
fn bench_bincode_load(c: &mut Criterion) {
    let mut group = c.benchmark_group("bincode_load");

    for entity_count in [1, 10, 100, 1000].iter() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let yaml_path = temp_dir.path().join("template.yaml");
        let bin_path = temp_dir.path().join("template.bin");

        // Create, save, and compile template
        let template = create_template(*entity_count);
        let yaml = serde_yaml::to_string(&template).expect("Failed to serialize to YAML");
        fs::write(&yaml_path, yaml).expect("Failed to write YAML");

        let compiler = TemplateCompiler::new();
        compiler.compile(&yaml_path, &bin_path).expect("Compilation failed");

        let bin_size = fs::metadata(&bin_path).expect("Failed to read metadata").len();

        group.throughput(Throughput::Bytes(bin_size));
        group.bench_with_input(BenchmarkId::new("entities", entity_count), entity_count, |b, _| {
            b.iter(|| {
                let _template = compiler
                    .load_compiled(black_box(&bin_path))
                    .expect("Failed to load compiled template");
            });
        });
    }

    group.finish();
}

/// Benchmark template compilation
fn bench_compile(c: &mut Criterion) {
    let mut group = c.benchmark_group("compile");

    for entity_count in [1, 10, 100, 1000].iter() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let yaml_path = temp_dir.path().join("template.yaml");

        // Create and save template
        let template = create_template(*entity_count);
        let yaml = serde_yaml::to_string(&template).expect("Failed to serialize to YAML");
        let yaml_size = yaml.len();
        fs::write(&yaml_path, &yaml).expect("Failed to write YAML");

        group.throughput(Throughput::Bytes(yaml_size as u64));
        group.bench_with_input(BenchmarkId::new("entities", entity_count), entity_count, |b, _| {
            let compiler = TemplateCompiler::new();
            b.iter(|| {
                let bin_path = temp_dir.path().join(format!("template_{}.bin", entity_count));
                let _result = compiler
                    .compile(black_box(&yaml_path), black_box(&bin_path))
                    .expect("Compilation failed");
            });
        });
    }

    group.finish();
}

/// Compare file sizes between YAML and Bincode
fn bench_file_size_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("file_size_comparison");

    for entity_count in [1, 10, 100, 1000].iter() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let yaml_path = temp_dir.path().join("template.yaml");
        let bin_path = temp_dir.path().join("template.bin");

        // Create and save template
        let template = create_template(*entity_count);
        let yaml = serde_yaml::to_string(&template).expect("Failed to serialize to YAML");
        fs::write(&yaml_path, &yaml).expect("Failed to write YAML");

        let compiler = TemplateCompiler::new();
        compiler.compile(&yaml_path, &bin_path).expect("Compilation failed");

        let yaml_size = fs::metadata(&yaml_path).expect("Failed to read metadata").len();
        let bin_size = fs::metadata(&bin_path).expect("Failed to read metadata").len();

        let compression_ratio = (bin_size as f64 / yaml_size as f64) * 100.0;

        // This is just for information, not a real benchmark
        group.bench_with_input(
            BenchmarkId::new("size_info", entity_count),
            entity_count,
            |b, _| {
                b.iter(|| {
                    // Print size information
                    println!(
                        "\nEntity count: {}\n  YAML: {} bytes\n  Bincode: {} bytes\n  Compression: {:.1}%",
                        entity_count, yaml_size, bin_size, compression_ratio
                    );
                });
            },
        );
    }

    group.finish();
}

/// Benchmark memory usage during parsing
fn bench_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_usage");
    group.sample_size(10); // Fewer samples for memory benchmarks

    for entity_count in [100, 1000].iter() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let yaml_path = temp_dir.path().join("template.yaml");
        let bin_path = temp_dir.path().join("template.bin");

        // Create and save template
        let template = create_template(*entity_count);
        let yaml = serde_yaml::to_string(&template).expect("Failed to serialize to YAML");
        fs::write(&yaml_path, &yaml).expect("Failed to write YAML");

        let compiler = TemplateCompiler::new();
        compiler.compile(&yaml_path, &bin_path).expect("Compilation failed");

        // Benchmark YAML parsing memory
        group.bench_with_input(
            BenchmarkId::new("yaml_entities", entity_count),
            entity_count,
            |b, _| {
                b.iter(|| {
                    let yaml_content = fs::read_to_string(&yaml_path).expect("Failed to read YAML");
                    let _template: Template = serde_yaml::from_str(black_box(&yaml_content))
                        .expect("Failed to parse YAML");
                    // Template is dropped here, freeing memory
                });
            },
        );

        // Benchmark Bincode loading memory
        group.bench_with_input(
            BenchmarkId::new("bincode_entities", entity_count),
            entity_count,
            |b, _| {
                b.iter(|| {
                    let _template = compiler
                        .load_compiled(black_box(&bin_path))
                        .expect("Failed to load compiled template");
                    // Template is dropped here, freeing memory
                });
            },
        );
    }

    group.finish();
}

/// Benchmark roundtrip (YAML → Bincode → Template)
fn bench_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("roundtrip");

    for entity_count in [1, 10, 100].iter() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let yaml_path = temp_dir.path().join("template.yaml");
        let bin_path = temp_dir.path().join("template.bin");

        // Create and save template
        let template = create_template(*entity_count);
        let yaml = serde_yaml::to_string(&template).expect("Failed to serialize to YAML");
        fs::write(&yaml_path, &yaml).expect("Failed to write YAML");

        group.bench_with_input(BenchmarkId::new("entities", entity_count), entity_count, |b, _| {
            let compiler = TemplateCompiler::new();
            b.iter(|| {
                // Compile
                compiler
                    .compile(black_box(&yaml_path), black_box(&bin_path))
                    .expect("Compilation failed");

                // Load
                let _template = compiler
                    .load_compiled(black_box(&bin_path))
                    .expect("Failed to load compiled template");
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_yaml_load,
    bench_bincode_load,
    bench_compile,
    bench_file_size_comparison,
    bench_memory_usage,
    bench_roundtrip,
);

criterion_main!(benches);
