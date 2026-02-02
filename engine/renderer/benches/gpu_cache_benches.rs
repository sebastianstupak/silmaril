//! Benchmarks for GPU mesh cache performance
//!
//! Tests upload speed, cache lookup performance, and memory cleanup.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_assets::{AssetId, MeshData, Vertex};
use engine_renderer::{GpuCache, VulkanContext};
use glam::{Vec2, Vec3};

/// Create a mesh with specified vertex count
fn create_mesh(vertex_count: usize) -> MeshData {
    let mut vertices = Vec::with_capacity(vertex_count);
    for i in 0..vertex_count {
        let angle = (i as f32 / vertex_count as f32) * std::f32::consts::TAU;
        vertices.push(Vertex {
            position: Vec3::new(angle.cos(), angle.sin(), 0.0),
            normal: Vec3::Z,
            uv: Vec2::new(0.5, 0.5),
        });
    }

    // Create triangle indices
    let mut indices = Vec::new();
    for i in 0..(vertex_count - 2) {
        indices.push(0);
        indices.push((i + 1) as u32);
        indices.push((i + 2) as u32);
    }

    MeshData { vertices, indices }
}

fn bench_mesh_upload(c: &mut Criterion) {
    // Skip if no Vulkan
    let context = match VulkanContext::new("GpuCacheBench", None, None) {
        Ok(ctx) => ctx,
        Err(_) => {
            eprintln!("Skipping GPU cache benchmarks - no Vulkan support");
            return;
        }
    };

    let mut group = c.benchmark_group("gpu_cache_upload");

    // Benchmark different mesh sizes
    for vertex_count in [100, 1_000, 10_000, 100_000].iter() {
        let mesh = create_mesh(*vertex_count);
        let asset_id = AssetId::from_bytes(format!("mesh_{}", vertex_count).as_bytes());

        group.throughput(Throughput::Elements(*vertex_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_vertices", vertex_count)),
            vertex_count,
            |b, _| {
                let mut cache = GpuCache::new(&context).expect("Cache creation failed");
                b.iter(|| {
                    cache
                        .upload_mesh(&context, black_box(asset_id), black_box(&mesh))
                        .expect("Upload failed");
                    cache.evict(asset_id); // Evict for next iteration
                });
            },
        );
    }

    group.finish();
}

fn bench_cache_lookup(c: &mut Criterion) {
    // Skip if no Vulkan
    let context = match VulkanContext::new("GpuCacheLookupBench", None, None) {
        Ok(ctx) => ctx,
        Err(_) => return,
    };

    let mut cache = GpuCache::new(&context).expect("Cache creation failed");

    // Populate cache with multiple meshes
    for i in 0..100 {
        let mesh = create_mesh(1000);
        let asset_id = AssetId::from_bytes(format!("lookup_mesh_{}", i).as_bytes());
        cache.upload_mesh(&context, asset_id, &mesh).expect("Upload failed");
    }

    c.bench_function("cache_contains_hit", |b| {
        let asset_id = AssetId::from_bytes(b"lookup_mesh_50");
        b.iter(|| cache.contains(black_box(asset_id)));
    });

    c.bench_function("cache_contains_miss", |b| {
        let asset_id = AssetId::from_bytes(b"nonexistent_mesh");
        b.iter(|| cache.contains(black_box(asset_id)));
    });

    c.bench_function("cache_get_mesh_info", |b| {
        let asset_id = AssetId::from_bytes(b"lookup_mesh_50");
        b.iter(|| cache.get_mesh_info(black_box(asset_id)));
    });

    c.bench_function("cache_get_buffers", |b| {
        let asset_id = AssetId::from_bytes(b"lookup_mesh_50");
        b.iter(|| cache.get_buffers(black_box(asset_id)));
    });
}

fn bench_cache_eviction(c: &mut Criterion) {
    // Skip if no Vulkan
    let context = match VulkanContext::new("GpuCacheEvictionBench", None, None) {
        Ok(ctx) => ctx,
        Err(_) => return,
    };

    c.bench_function("cache_evict_single", |b| {
        b.iter_batched(
            || {
                let mut cache = GpuCache::new(&context).expect("Cache creation failed");
                let mesh = create_mesh(1000);
                let asset_id = AssetId::from_bytes(b"evict_mesh");
                cache.upload_mesh(&context, asset_id, &mesh).expect("Upload failed");
                (cache, asset_id)
            },
            |(mut cache, asset_id)| {
                cache.evict(black_box(asset_id));
            },
            criterion::BatchSize::SmallInput,
        );
    });

    c.bench_function("cache_clear_100_meshes", |b| {
        b.iter_batched(
            || {
                let mut cache = GpuCache::new(&context).expect("Cache creation failed");
                for i in 0..100 {
                    let mesh = create_mesh(1000);
                    let asset_id = AssetId::from_bytes(format!("clear_mesh_{}", i).as_bytes());
                    cache.upload_mesh(&context, asset_id, &mesh).expect("Upload failed");
                }
                cache
            },
            |mut cache| {
                cache.clear();
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_duplicate_upload(c: &mut Criterion) {
    // Skip if no Vulkan
    let context = match VulkanContext::new("GpuCacheDuplicateBench", None, None) {
        Ok(ctx) => ctx,
        Err(_) => return,
    };

    let mesh = create_mesh(10_000);
    let asset_id = AssetId::from_bytes(b"duplicate_mesh");

    c.bench_function("duplicate_upload_idempotent", |b| {
        let mut cache = GpuCache::new(&context).expect("Cache creation failed");
        // First upload
        cache.upload_mesh(&context, asset_id, &mesh).expect("First upload failed");

        b.iter(|| {
            // Subsequent uploads should be fast (cached)
            cache
                .upload_mesh(&context, black_box(asset_id), black_box(&mesh))
                .expect("Duplicate upload failed");
        });
    });
}

fn bench_concurrent_access(c: &mut Criterion) {
    // Skip if no Vulkan
    let context = match VulkanContext::new("GpuCacheConcurrentBench", None, None) {
        Ok(ctx) => ctx,
        Err(_) => return,
    };

    let mut cache = GpuCache::new(&context).expect("Cache creation failed");

    // Populate with multiple meshes
    for i in 0..10 {
        let mesh = create_mesh(1000);
        let asset_id = AssetId::from_bytes(format!("concurrent_mesh_{}", i).as_bytes());
        cache.upload_mesh(&context, asset_id, &mesh).expect("Upload failed");
    }

    c.bench_function("interleaved_lookups", |b| {
        let asset_ids: Vec<AssetId> = (0..10)
            .map(|i| AssetId::from_bytes(format!("concurrent_mesh_{}", i).as_bytes()))
            .collect();

        b.iter(|| {
            for asset_id in &asset_ids {
                black_box(cache.get_mesh_info(black_box(*asset_id)));
            }
        });
    });
}

criterion_group!(
    benches,
    bench_mesh_upload,
    bench_cache_lookup,
    bench_cache_eviction,
    bench_duplicate_upload,
    bench_concurrent_access
);
criterion_main!(benches);
