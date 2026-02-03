//! Hot-reload system benchmarks.
//!
//! Measures:
//! - File change detection latency
//! - Reload throughput (assets/sec)
//! - Memory overhead of watchers
//! - Batch reload performance

#![cfg(feature = "hot-reload")]

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_assets::{AssetId, AssetManager, HotReloadConfig, HotReloader};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

fn create_simple_obj() -> String {
    r#"# Simple mesh
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 0.0 1.0 0.0
vn 0.0 0.0 1.0
vt 0.0 0.0
vt 1.0 0.0
vt 0.0 1.0
f 1/1/1 2/2/1 3/3/1
"#
    .to_string()
}

fn bench_hot_reloader_creation(c: &mut Criterion) {
    let manager = Arc::new(AssetManager::new());

    c.bench_function("hot_reloader_creation", |b| {
        b.iter(|| {
            let config = HotReloadConfig::default();
            let _reloader = HotReloader::new(Arc::clone(&manager), config);
        });
    });
}

fn bench_watch_registration(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let manager = Arc::new(AssetManager::new());

    c.bench_function("watch_registration", |b| {
        b.iter(|| {
            let config = HotReloadConfig::default();
            let mut reloader = HotReloader::new(Arc::clone(&manager), config).unwrap();
            let _ = reloader.watch(temp_dir.path());
            let _ = reloader.unwatch(temp_dir.path());
        });
    });
}

fn bench_asset_registration(c: &mut Criterion) {
    let manager = Arc::new(AssetManager::new());
    let config = HotReloadConfig::default();
    let mut reloader = HotReloader::new(manager, config).unwrap();

    let mut group = c.benchmark_group("asset_registration");

    for count in [1, 10, 100, 1000] {
        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            b.iter(|| {
                for i in 0..count {
                    let path = PathBuf::from(format!("asset{i}.obj"));
                    let id = AssetId::from_content(format!("asset{i}").as_bytes());
                    reloader.register_asset(black_box(path), black_box(id));
                }
                // Clean up
                for i in 0..count {
                    let path = PathBuf::from(format!("asset{i}.obj"));
                    reloader.unregister_asset(&path);
                }
            });
        });
    }

    group.finish();
}

fn bench_event_processing(c: &mut Criterion) {
    let manager = Arc::new(AssetManager::new());

    let mut group = c.benchmark_group("event_processing");

    for batch_enabled in [false, true] {
        let label = if batch_enabled { "batching_on" } else { "batching_off" };

        group.bench_function(label, |b| {
            b.iter(|| {
                let config =
                    HotReloadConfig { enable_batching: batch_enabled, ..Default::default() };
                let mut reloader = HotReloader::new(Arc::clone(&manager), config).unwrap();

                // Process events (even though there are none)
                reloader.process_events();
            });
        });
    }

    group.finish();
}

fn bench_path_mapping_lookup(c: &mut Criterion) {
    let manager = Arc::new(AssetManager::new());
    let config = HotReloadConfig::default();
    let mut reloader = HotReloader::new(manager, config).unwrap();

    // Register some assets
    for i in 0..1000 {
        let path = PathBuf::from(format!("asset{i}.obj"));
        let id = AssetId::from_content(format!("asset{i}").as_bytes());
        reloader.register_asset(path, id);
    }

    c.bench_function("path_mapping_lookup", |b| {
        b.iter(|| {
            let path = PathBuf::from("asset500.obj");
            black_box(reloader.path_to_id.get(&path));
        });
    });
}

fn bench_stats_collection(c: &mut Criterion) {
    let manager = Arc::new(AssetManager::new());
    let config = HotReloadConfig::default();
    let reloader = HotReloader::new(manager, config).unwrap();

    c.bench_function("stats_collection", |b| {
        b.iter(|| {
            let _stats = reloader.stats();
        });
    });
}

fn bench_debouncing_overhead(c: &mut Criterion) {
    let manager = Arc::new(AssetManager::new());

    let mut group = c.benchmark_group("debouncing_overhead");

    for debounce_ms in [0, 100, 300, 500] {
        group.bench_with_input(
            BenchmarkId::from_parameter(debounce_ms),
            &debounce_ms,
            |b, &debounce_ms| {
                b.iter(|| {
                    let config = HotReloadConfig {
                        debounce_duration: Duration::from_millis(debounce_ms),
                        ..Default::default()
                    };
                    let _reloader = HotReloader::new(Arc::clone(&manager), config).unwrap();
                });
            },
        );
    }

    group.finish();
}

fn bench_memory_overhead(c: &mut Criterion) {
    let manager = Arc::new(AssetManager::new());

    let mut group = c.benchmark_group("memory_overhead");

    for asset_count in [10, 100, 1000, 10000] {
        group.throughput(Throughput::Elements(asset_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(asset_count),
            &asset_count,
            |b, &asset_count| {
                b.iter(|| {
                    let config = HotReloadConfig::default();
                    let mut reloader = HotReloader::new(Arc::clone(&manager), config).unwrap();

                    for i in 0..asset_count {
                        let path = PathBuf::from(format!("asset{i}.obj"));
                        let id = AssetId::from_content(format!("asset{i}").as_bytes());
                        reloader.register_asset(path, id);
                    }

                    black_box(&reloader);
                });
            },
        );
    }

    group.finish();
}

fn bench_reload_queue_operations(c: &mut Criterion) {
    let manager = Arc::new(AssetManager::new());

    let mut group = c.benchmark_group("reload_queue_operations");

    for queue_size in [1, 5, 10, 50] {
        group.throughput(Throughput::Elements(queue_size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(queue_size),
            &queue_size,
            |b, &queue_size| {
                b.iter(|| {
                    let config = HotReloadConfig {
                        enable_batching: true,
                        max_batch_size: queue_size,
                        ..Default::default()
                    };
                    let mut reloader = HotReloader::new(Arc::clone(&manager), config).unwrap();

                    // Simulate queuing
                    for i in 0..queue_size {
                        let path = PathBuf::from(format!("asset{i}.obj"));
                        reloader.reload_queue.push_back((
                            path,
                            engine_assets::AssetType::Mesh,
                            std::time::Instant::now(),
                        ));
                    }

                    black_box(&reloader);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_hot_reloader_creation,
    bench_watch_registration,
    bench_asset_registration,
    bench_event_processing,
    bench_path_mapping_lookup,
    bench_stats_collection,
    bench_debouncing_overhead,
    bench_memory_overhead,
    bench_reload_queue_operations,
);
criterion_main!(benches);
