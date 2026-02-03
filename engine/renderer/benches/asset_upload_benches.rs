//! Benchmarks for asset upload performance.
//!
//! Measures GPU upload time for meshes and textures.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use engine_assets::{AssetManager, MeshData};
use engine_renderer::{AssetBridge, VulkanContext};
use std::sync::Arc;

/// Setup Vulkan context for benchmarks
fn setup_benchmark_context() -> Option<(VulkanContext, Arc<AssetManager>, AssetBridge)> {
    // Create context without validation layers for accurate benchmarks
    let context = VulkanContext::new_for_benchmarks("AssetBenchmark", None, None).ok()?;
    let asset_manager = Arc::new(AssetManager::new());
    let bridge = AssetBridge::new(context.clone(), asset_manager.clone());
    Some((context, asset_manager, bridge))
}

/// Benchmark mesh upload for various mesh sizes
fn bench_mesh_upload(c: &mut Criterion) {
    let setup = match setup_benchmark_context() {
        Some(s) => s,
        None => {
            eprintln!("Skipping benchmark - no Vulkan support");
            return;
        }
    };

    let (_context, asset_manager, mut bridge) = setup;

    // Create meshes of different sizes
    let meshes = vec![("triangle_3v", MeshData::triangle(), 3), ("cube_24v", MeshData::cube(), 24)];

    let mut group = c.benchmark_group("mesh_upload");
    group.sample_size(20); // Fewer samples for GPU operations

    for (name, mesh, vertex_count) in meshes {
        let mesh_id = engine_assets::AssetId::from_content(name.as_bytes());
        asset_manager.meshes().insert(mesh_id, mesh);

        group.bench_with_input(BenchmarkId::from_parameter(name), &mesh_id, |b, &id| {
            b.iter(|| {
                // Clear cache to force re-upload each iteration
                bridge.evict_mesh(id);
                // Upload mesh
                bridge.get_or_upload_mesh(black_box(id)).expect("Upload failed")
            });
        });

        // Report upload time per vertex
        println!("{}: {} vertices", name, vertex_count);
    }

    group.finish();
}

/// Benchmark mesh cache hit performance
fn bench_mesh_cache_hit(c: &mut Criterion) {
    let setup = match setup_benchmark_context() {
        Some(s) => s,
        None => {
            eprintln!("Skipping benchmark - no Vulkan support");
            return;
        }
    };

    let (_context, asset_manager, mut bridge) = setup;

    // Create and upload a mesh once
    let mesh = MeshData::cube();
    let mesh_id = engine_assets::AssetId::from_content(b"cached_cube");
    asset_manager.meshes().insert(mesh_id, mesh);
    bridge.get_or_upload_mesh(mesh_id).expect("Initial upload failed");

    // Benchmark cache hits
    c.bench_function("mesh_cache_hit", |b| {
        b.iter(|| {
            // This should be a cache hit (no upload)
            bridge.get_or_upload_mesh(black_box(mesh_id)).expect("Cache hit failed")
        });
    });
}

/// Benchmark hot-reload performance
fn bench_mesh_hot_reload(c: &mut Criterion) {
    let setup = match setup_benchmark_context() {
        Some(s) => s,
        None => {
            eprintln!("Skipping benchmark - no Vulkan support");
            return;
        }
    };

    let (_context, asset_manager, mut bridge) = setup;

    // Create mesh
    let mesh = MeshData::cube();
    let mesh_id = engine_assets::AssetId::from_content(b"reload_cube");
    asset_manager.meshes().insert(mesh_id, mesh.clone());

    // Upload initially
    bridge.get_or_upload_mesh(mesh_id).expect("Initial upload failed");

    c.bench_function("mesh_hot_reload", |b| {
        b.iter(|| {
            // Reload mesh (evict + re-upload)
            bridge.reload_mesh(black_box(mesh_id)).expect("Reload failed");
            bridge.get_or_upload_mesh(black_box(mesh_id)).expect("Re-upload failed")
        });
    });
}

/// Benchmark asset handle resolution overhead
fn bench_asset_handle_resolution(c: &mut Criterion) {
    let setup = match setup_benchmark_context() {
        Some(s) => s,
        None => {
            eprintln!("Skipping benchmark - no Vulkan support");
            return;
        }
    };

    let (_context, asset_manager, mut bridge) = setup;

    // Create and upload multiple meshes
    let mesh_ids: Vec<_> = (0..10)
        .map(|i| {
            let mesh = MeshData::cube();
            let id = engine_assets::AssetId::from_content(format!("mesh_{}", i).as_bytes());
            asset_manager.meshes().insert(id, mesh);
            bridge.get_or_upload_mesh(id).expect("Upload failed");
            id
        })
        .collect();

    c.bench_function("asset_handle_resolution", |b| {
        let mut idx = 0;
        b.iter(|| {
            let mesh_id = mesh_ids[idx % mesh_ids.len()];
            idx += 1;
            // Resolve asset handle to GPU resource
            bridge.get_or_upload_mesh(black_box(mesh_id)).expect("Resolution failed")
        });
    });
}

/// Benchmark shared resource pooling efficiency
fn bench_shared_resource_pooling(c: &mut Criterion) {
    let setup = match setup_benchmark_context() {
        Some(s) => s,
        None => {
            eprintln!("Skipping benchmark - no Vulkan support");
            return;
        }
    };

    let (_context, asset_manager, mut bridge) = setup;

    // Create a shared mesh (same data, different handles)
    let mesh = MeshData::cube();
    let shared_id = engine_assets::AssetId::from_content(b"shared_cube");
    asset_manager.meshes().insert(shared_id, mesh);

    c.bench_function("shared_resource_pooling", |b| {
        b.iter(|| {
            // Access same mesh multiple times (should be cheap due to caching)
            for _ in 0..10 {
                bridge.get_or_upload_mesh(black_box(shared_id)).expect("Access failed");
            }
        });
    });
}

/// Benchmark GPU resource eviction overhead
fn bench_gpu_eviction(c: &mut Criterion) {
    let setup = match setup_benchmark_context() {
        Some(s) => s,
        None => {
            eprintln!("Skipping benchmark - no Vulkan support");
            return;
        }
    };

    let (_context, asset_manager, mut bridge) = setup;

    // Upload a mesh
    let mesh = MeshData::cube();
    let mesh_id = engine_assets::AssetId::from_content(b"cube");
    asset_manager.meshes().insert(mesh_id, mesh);
    bridge.get_or_upload_mesh(mesh_id).expect("Upload failed");

    c.bench_function("gpu_eviction", |b| {
        b.iter(|| {
            // Evict GPU resource
            bridge.evict_mesh(black_box(mesh_id));
            // Re-upload for next iteration
            bridge.get_or_upload_mesh(black_box(mesh_id)).expect("Re-upload failed");
        });
    });
}

criterion_group!(
    benches,
    bench_mesh_upload,
    bench_mesh_cache_hit,
    bench_mesh_hot_reload,
    bench_asset_handle_resolution,
    bench_shared_resource_pooling,
    bench_gpu_eviction
);
criterion_main!(benches);
