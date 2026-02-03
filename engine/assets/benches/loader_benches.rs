//! Benchmarks for asset loading strategies.
//!
//! Measures performance of sync, async, and streaming loading modes.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_assets::{AssetManager, EnhancedLoader, MeshData};
use std::io::Write;
use std::sync::Arc;
use tempfile::NamedTempFile;

fn create_test_obj(vertex_count: usize) -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();

    // Create mesh with specified vertex count
    for i in 0..vertex_count {
        let x = (i as f32 * 0.1).sin();
        let y = (i as f32 * 0.1).cos();
        let z = (i as f32 * 0.05).sin();
        writeln!(file, "v {x} {y} {z}").unwrap();
    }

    writeln!(file, "vn 0 0 1").unwrap();
    writeln!(file, "vt 0 0").unwrap();

    // Create triangles
    for i in 0..(vertex_count.saturating_sub(2)) {
        writeln!(file, "f {} {} {}", i + 1, i + 2, i + 3).unwrap();
    }

    file.flush().unwrap();
    file
}

fn bench_sync_load_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("sync_load_throughput");

    for size in [10, 100, 1000] {
        group.throughput(Throughput::Elements(1));

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.iter(|| {
                let loader = EnhancedLoader::default();
                let test_file = create_test_obj(size);
                let result = loader.load_sync::<MeshData>(black_box(test_file.path()));
                black_box(result)
            });
        });
    }

    group.finish();
}

#[cfg(feature = "async")]
fn bench_async_load_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("async_load_throughput");

    for size in [10, 100, 1000] {
        group.throughput(Throughput::Elements(1));

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            let runtime = tokio::runtime::Runtime::new().unwrap();

            b.to_async(&runtime).iter(|| async {
                let loader = EnhancedLoader::default();
                let test_file = create_test_obj(size);
                let result = loader.load_async::<MeshData>(black_box(test_file.path())).await;
                black_box(result)
            });
        });
    }

    group.finish();
}

#[cfg(feature = "async")]
fn bench_streaming_time_to_first_lod(c: &mut Criterion) {
    let mut group = c.benchmark_group("streaming_time_to_first_lod");
    group.sample_size(50); // Reduce samples for async tests

    for lod_count in [2, 3, 5] {
        group.bench_with_input(
            BenchmarkId::from_parameter(lod_count),
            &lod_count,
            |b, &lod_count| {
                let runtime = tokio::runtime::Runtime::new().unwrap();

                b.to_async(&runtime).iter(|| async move {
                    let loader = EnhancedLoader::default();
                    let test_file = create_test_obj(100);
                    let handle = loader
                        .load_streaming::<MeshData>(black_box(test_file.path()), lod_count)
                        .await
                        .unwrap();

                    // Measure time until LOD 0 is available
                    let lod0 = handle.get_lod(0).await;
                    black_box(lod0)
                });
            },
        );
    }

    group.finish();
}

#[cfg(feature = "async")]
fn bench_concurrent_loads_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_loads_scaling");
    group.sample_size(20); // Fewer samples for expensive concurrent tests

    for thread_count in [1, 2, 4, 8] {
        group.throughput(Throughput::Elements(thread_count));

        group.bench_with_input(
            BenchmarkId::from_parameter(thread_count),
            &thread_count,
            |b, &thread_count| {
                let runtime = tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(thread_count as usize)
                    .build()
                    .unwrap();

                b.to_async(&runtime).iter(|| async move {
                    let loader = Arc::new(EnhancedLoader::default());
                    let mut tasks = vec![];

                    for _ in 0..thread_count {
                        let loader_clone = Arc::clone(&loader);
                        let test_file = create_test_obj(100);
                        let path = test_file.path().to_path_buf();

                        let task = tokio::spawn(async move {
                            let _file = test_file;
                            loader_clone.load_async::<MeshData>(&path).await
                        });

                        tasks.push(task);
                    }

                    for task in tasks {
                        let _ = task.await;
                    }

                    black_box(loader)
                });
            },
        );
    }

    group.finish();
}

fn bench_memory_overhead_during_loading(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_overhead");

    group.bench_function("load_10_assets", |b| {
        b.iter(|| {
            let loader = EnhancedLoader::default();
            let mut handles = vec![];

            for _ in 0..10 {
                let test_file = create_test_obj(100);
                if let Ok(handle) = loader.load_sync::<MeshData>(test_file.path()) {
                    handles.push(handle);
                }
            }

            black_box(handles)
        });
    });

    group.finish();
}

fn bench_cache_hit_rate(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_hit_rate");

    group.bench_function("repeated_loads_same_file", |b| {
        let loader = EnhancedLoader::default();
        let test_file = create_test_obj(100);
        let path = test_file.path().to_path_buf();

        // First load to populate cache
        let _ = loader.load_sync::<MeshData>(&path);

        b.iter(|| {
            // Subsequent loads should hit cache
            let result = loader.load_sync::<MeshData>(black_box(&path));
            black_box(result)
        });
    });

    group.finish();
}

fn bench_sync_vs_async_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("sync_vs_async");
    group.sample_size(30);

    let test_file = create_test_obj(500);
    let path = test_file.path().to_path_buf();

    group.bench_function("sync_load_500_vertices", |b| {
        b.iter(|| {
            let loader = EnhancedLoader::default();
            let result = loader.load_sync::<MeshData>(black_box(&path));
            black_box(result)
        });
    });

    #[cfg(feature = "async")]
    group.bench_function("async_load_500_vertices", |b| {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        b.to_async(&runtime).iter(|| async {
            let loader = EnhancedLoader::default();
            let result = loader.load_async::<MeshData>(black_box(&path)).await;
            black_box(result)
        });
    });

    group.finish();
}

#[cfg(feature = "async")]
fn bench_streaming_lod_progression(c: &mut Criterion) {
    let mut group = c.benchmark_group("streaming_lod_progression");
    group.sample_size(20);

    group.bench_function("stream_3_lods_complete", |b| {
        let runtime = tokio::runtime::Runtime::new().unwrap();

        b.to_async(&runtime).iter(|| async {
            let loader = EnhancedLoader::default();
            let test_file = create_test_obj(200);
            let handle = loader.load_streaming::<MeshData>(test_file.path(), 3).await.unwrap();

            // Wait for all LODs to complete
            while !handle.is_complete() {
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            }

            black_box(handle)
        });
    });

    group.finish();
}

// Configure benchmark groups
criterion_group!(
    benches,
    bench_sync_load_throughput,
    bench_memory_overhead_during_loading,
    bench_cache_hit_rate,
    bench_sync_vs_async_comparison,
);

#[cfg(feature = "async")]
criterion_group!(
    async_benches,
    bench_async_load_throughput,
    bench_streaming_time_to_first_lod,
    bench_concurrent_loads_scaling,
    bench_streaming_lod_progression,
);

#[cfg(feature = "async")]
criterion_main!(benches, async_benches);

#[cfg(not(feature = "async"))]
criterion_main!(benches);
