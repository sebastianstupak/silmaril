//! Memory usage benchmarks for template system.
//!
//! This benchmark suite measures:
//! 1. Memory used by Template loaded from YAML
//! 2. Memory used by Template from Bincode (when compiler is available)
//! 3. Memory overhead of template cache (10 templates)
//! 4. Memory of spawned entities in World
//!
//! # Performance Targets
//!
//! - Bincode < 50% of YAML memory (when available)
//! - Cache overhead < 1KB per template
//! - Spawned entities < 500 bytes per entity
//!
//! # Methodology
//!
//! We use custom memory tracking via a global allocator wrapper to measure
//! peak memory usage during template operations. Measurements include:
//! - Heap allocations during parsing
//! - In-memory size of Template structures
//! - World entity storage overhead
//!
//! Note: Some benchmarks are disabled until the compiler module (Task #10) is implemented.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_core::ecs::World;
use engine_core::gameplay::Health;
use engine_core::math::Transform;
use engine_core::rendering::{Camera, MeshRenderer};
use engine_templating::loader::TemplateLoader;
use engine_templating::template::{EntityDefinition, Template, TemplateMetadata};
use rustc_hash::FxHashMap;
use std::alloc::{GlobalAlloc, Layout, System};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use tempfile::TempDir;

/// Global memory tracking allocator
#[allow(dead_code)]
struct TrackingAllocator {
    current_usage: AtomicUsize,
    peak_usage: AtomicUsize,
}

#[allow(dead_code)]
impl TrackingAllocator {
    const fn new() -> Self {
        Self { current_usage: AtomicUsize::new(0), peak_usage: AtomicUsize::new(0) }
    }

    fn reset(&self) {
        self.current_usage.store(0, Ordering::SeqCst);
        self.peak_usage.store(0, Ordering::SeqCst);
    }

    fn current(&self) -> usize {
        self.current_usage.load(Ordering::SeqCst)
    }

    fn peak(&self) -> usize {
        self.peak_usage.load(Ordering::SeqCst)
    }
}

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc(layout);
        if !ptr.is_null() {
            let size = layout.size();
            let current = self.current_usage.fetch_add(size, Ordering::SeqCst) + size;
            let mut peak = self.peak_usage.load(Ordering::SeqCst);
            while current > peak {
                match self.peak_usage.compare_exchange_weak(
                    peak,
                    current,
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                ) {
                    Ok(_) => break,
                    Err(p) => peak = p,
                }
            }
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
        self.current_usage.fetch_sub(layout.size(), Ordering::SeqCst);
    }
}

#[cfg(not(test))] // Don't override allocator in tests
#[global_allocator]
static ALLOCATOR: TrackingAllocator = TrackingAllocator::new();

/// Creates a template with the specified number of entities
fn create_template(entity_count: usize) -> Template {
    let metadata = TemplateMetadata {
        name: Some(format!("Memory Benchmark Template ({})", entity_count)),
        description: Some("A template for memory benchmarking".to_string()),
        author: Some("Benchmark Suite".to_string()),
        version: Some("1.0.0".to_string()),
    };

    let mut template = Template::new(metadata);

    for i in 0..entity_count {
        let mut components = FxHashMap::default();

        // Add realistic component data
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

/// Helper to create a template file for benchmarking
fn create_template_file(dir: &TempDir, name: &str, entity_count: usize) -> PathBuf {
    let template = create_template(entity_count);
    let yaml = serde_yaml::to_string(&template).expect("Failed to serialize to YAML");
    let path = dir.path().join(name);
    fs::write(&path, yaml).expect("Failed to write template file");
    path
}

/// Benchmark 1: Memory used by Template loaded from YAML
fn bench_yaml_memory(c: &mut Criterion) {
    let mut group = c.benchmark_group("yaml_memory");
    group.sample_size(10); // Fewer samples for memory benchmarks

    for entity_count in [1, 10, 100, 1000].iter() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let template_path = create_template_file(&temp_dir, "template.yaml", *entity_count);

        group.throughput(Throughput::Elements(*entity_count as u64));
        group.bench_with_input(BenchmarkId::new("entities", entity_count), entity_count, |b, _| {
            b.iter_with_setup(
                || {
                    #[cfg(not(test))]
                    ALLOCATOR.reset();
                },
                |_| {
                    let yaml_content =
                        fs::read_to_string(&template_path).expect("Failed to read YAML");
                    let template: Template = serde_yaml::from_str(black_box(&yaml_content))
                        .expect("Failed to parse YAML");

                    #[cfg(not(test))]
                    let peak_bytes = ALLOCATOR.peak();
                    #[cfg(test)]
                    let peak_bytes = 0;

                    let bytes_per_entity =
                        if *entity_count > 0 { peak_bytes / *entity_count } else { 0 };

                    black_box((template, peak_bytes, bytes_per_entity))
                },
            );
        });
    }

    group.finish();
}

/// Benchmark 2: Memory used by Template from Bincode
///
/// NOTE: This benchmark is currently disabled because the compiler module
/// (Task #10) is not yet implemented. Re-enable this when the compiler is available.
///
/// ```rust,ignore
/// fn bench_bincode_memory(c: &mut Criterion) {
///     let mut group = c.benchmark_group("bincode_memory");
///     group.sample_size(10);
///
///     for entity_count in [1, 10, 100, 1000].iter() {
///         let temp_dir = TempDir::new().expect("Failed to create temp dir");
///         let yaml_path = create_template_file(&temp_dir, "template.yaml", *entity_count);
///         let bin_path = temp_dir.path().join("template.bin");
///
///         // Compile to bincode
///         let compiler = TemplateCompiler::new();
///         compiler.compile(&yaml_path, &bin_path).expect("Compilation failed");
///
///         group.throughput(Throughput::Elements(*entity_count as u64));
///         group.bench_with_input(
///             BenchmarkId::new("entities", entity_count),
///             entity_count,
///             |b, _| {
///                 b.iter_with_setup(
///                     || {
///                         ALLOCATOR.reset();
///                     },
///                     |_| {
///                         let template = compiler
///                             .load_compiled(black_box(&bin_path))
///                             .expect("Failed to load compiled template");
///
///                         let peak_bytes = ALLOCATOR.peak();
///                         let bytes_per_entity = if *entity_count > 0 {
///                             peak_bytes / *entity_count
///                         } else {
///                             0
///                         };
///
///                         black_box((template, peak_bytes, bytes_per_entity))
///                     },
///                 );
///             },
///         );
///     }
///
///     group.finish();
/// }
/// ```

/// Benchmark 3: Memory overhead of template cache (10 templates)
fn bench_cache_memory(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_memory");
    group.sample_size(10);

    for template_count in [1, 5, 10, 20].iter() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Create multiple template files
        let template_paths: Vec<PathBuf> = (0..*template_count)
            .map(|i| create_template_file(&temp_dir, &format!("template_{}.yaml", i), 10))
            .collect();

        group.throughput(Throughput::Elements(*template_count as u64));
        group.bench_with_input(
            BenchmarkId::new("templates", template_count),
            template_count,
            |b, _| {
                b.iter_with_setup(
                    || {
                        #[cfg(not(test))]
                        ALLOCATOR.reset();
                    },
                    |_| {
                        let mut loader = TemplateLoader::new();
                        let mut world = World::new();
                        world.register::<Transform>();
                        world.register::<Health>();
                        world.register::<MeshRenderer>();
                        world.register::<Camera>();

                        // Load all templates (populates cache)
                        for path in &template_paths {
                            let _instance = loader
                                .load(&mut world, black_box(path))
                                .expect("Failed to load template");
                        }

                        #[cfg(not(test))]
                        let peak_bytes = ALLOCATOR.peak();
                        #[cfg(test)]
                        let peak_bytes = 0;

                        let bytes_per_template =
                            if *template_count > 0 { peak_bytes / *template_count } else { 0 };

                        black_box((loader, peak_bytes, bytes_per_template))
                    },
                );
            },
        );
    }

    group.finish();
}

/// Benchmark 4: Memory of spawned entities in World
fn bench_spawned_entities_memory(c: &mut Criterion) {
    let mut group = c.benchmark_group("spawned_entities_memory");
    group.sample_size(10);

    for entity_count in [10, 100, 1000].iter() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let template_path = create_template_file(&temp_dir, "template.yaml", *entity_count);

        group.throughput(Throughput::Elements(*entity_count as u64));
        group.bench_with_input(BenchmarkId::new("entities", entity_count), entity_count, |b, _| {
            b.iter_with_setup(
                || {
                    #[cfg(not(test))]
                    ALLOCATOR.reset();

                    let mut world = World::new();
                    world.register::<Transform>();
                    world.register::<Health>();
                    world.register::<MeshRenderer>();
                    world.register::<Camera>();

                    let loader = TemplateLoader::new();

                    (world, loader)
                },
                |(mut world, mut loader)| {
                    #[cfg(not(test))]
                    let before_bytes = ALLOCATOR.current();

                    // Load template and spawn entities
                    let instance = loader
                        .load(&mut world, black_box(&template_path))
                        .expect("Failed to load template");

                    #[cfg(not(test))]
                    let after_bytes = ALLOCATOR.current();
                    #[cfg(not(test))]
                    let entity_bytes = after_bytes.saturating_sub(before_bytes);

                    #[cfg(test)]
                    let entity_bytes = 0;

                    let bytes_per_entity =
                        if *entity_count > 0 { entity_bytes / *entity_count } else { 0 };

                    black_box((world, instance, entity_bytes, bytes_per_entity))
                },
            );
        });
    }

    group.finish();
}

/// Benchmark: Compare in-memory size of Template (YAML-parsed vs direct construction)
fn bench_template_in_memory_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("template_in_memory_size");
    group.sample_size(10);

    for entity_count in [10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*entity_count as u64));

        // Measure YAML-parsed template memory
        group.bench_with_input(
            BenchmarkId::new("yaml_parsed", entity_count),
            entity_count,
            |b, &count| {
                let temp_dir = TempDir::new().expect("Failed to create temp dir");
                let template_path = create_template_file(&temp_dir, "template.yaml", count);

                b.iter_with_setup(
                    || {
                        #[cfg(not(test))]
                        ALLOCATOR.reset();
                    },
                    |_| {
                        let yaml_content =
                            fs::read_to_string(&template_path).expect("Failed to read YAML");
                        let template: Template = serde_yaml::from_str(black_box(&yaml_content))
                            .expect("Failed to parse YAML");

                        #[cfg(not(test))]
                        let memory_bytes = ALLOCATOR.current();
                        #[cfg(test)]
                        let memory_bytes = 0;

                        black_box((template, memory_bytes))
                    },
                );
            },
        );

        // Measure directly constructed template memory
        group.bench_with_input(
            BenchmarkId::new("direct_construct", entity_count),
            entity_count,
            |b, &count| {
                b.iter_with_setup(
                    || {
                        #[cfg(not(test))]
                        ALLOCATOR.reset();
                    },
                    |_| {
                        let template = create_template(count);

                        #[cfg(not(test))]
                        let memory_bytes = ALLOCATOR.current();
                        #[cfg(test)]
                        let memory_bytes = 0;

                        black_box((template, memory_bytes))
                    },
                );
            },
        );
    }

    group.finish();
}

/// Benchmark: Memory efficiency of cache vs repeated loading
fn bench_cache_vs_reload_memory(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_vs_reload_memory");
    group.sample_size(10);

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let template_path = create_template_file(&temp_dir, "template.yaml", 100);

    // Benchmark: Load same template 10 times WITHOUT cache (worst case)
    group.bench_function("reload_10_times_no_cache", |b| {
        b.iter_with_setup(
            || {
                #[cfg(not(test))]
                ALLOCATOR.reset();

                let mut world = World::new();
                world.register::<Transform>();
                world.register::<Health>();

                world
            },
            |mut world| {
                for _ in 0..10 {
                    let mut loader = TemplateLoader::new(); // Fresh loader each time
                    let _instance = loader
                        .load(&mut world, black_box(&template_path))
                        .expect("Failed to load template");
                }

                #[cfg(not(test))]
                let total_bytes = ALLOCATOR.peak();
                #[cfg(test)]
                let total_bytes = 0;

                black_box((world, total_bytes))
            },
        );
    });

    // Benchmark: Load same template 10 times WITH cache (best case)
    group.bench_function("reload_10_times_with_cache", |b| {
        b.iter_with_setup(
            || {
                #[cfg(not(test))]
                ALLOCATOR.reset();

                let mut world = World::new();
                world.register::<Transform>();
                world.register::<Health>();

                let loader = TemplateLoader::new();

                (world, loader)
            },
            |(mut world, mut loader)| {
                for _ in 0..10 {
                    let _instance = loader
                        .load(&mut world, black_box(&template_path))
                        .expect("Failed to load template");
                }

                #[cfg(not(test))]
                let total_bytes = ALLOCATOR.peak();
                #[cfg(test)]
                let total_bytes = 0;

                black_box((world, loader, total_bytes))
            },
        );
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_yaml_memory,
    // bench_bincode_memory, // Disabled until Task #10 (compiler module) is complete
    bench_cache_memory,
    bench_spawned_entities_memory,
    bench_template_in_memory_size,
    bench_cache_vs_reload_memory,
);

criterion_main!(benches);
