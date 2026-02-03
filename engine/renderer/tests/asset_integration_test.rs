//! Integration tests for asset system → renderer bridge.
//!
//! Tests the integration between engine-assets and engine-renderer,
//! verifying that asset handles correctly resolve to GPU resources.

use engine_assets::{AssetManager, MeshData};
use engine_renderer::{AssetBridge, VulkanContext};
use std::sync::Arc;

/// Test that AssetBridge initializes correctly
#[test]
fn test_asset_bridge_creation() {
    // Try to create Vulkan context (skip if no Vulkan support)
    let context = match VulkanContext::new("AssetBridgeTest", None, None) {
        Ok(ctx) => ctx,
        Err(_) => {
            eprintln!("Skipping test - no Vulkan support");
            return;
        }
    };

    let asset_manager = Arc::new(AssetManager::new());
    let bridge = AssetBridge::new(context, asset_manager);

    // Verify initial state
    let stats = bridge.stats();
    assert_eq!(stats.mesh_count, 0);
    assert_eq!(stats.texture_count, 0);
    assert_eq!(stats.total_uploads, 0);
    assert_eq!(stats.cache_hits, 0);
}

/// Test loading and uploading a simple mesh
#[test]
fn test_mesh_upload() {
    // Try to create Vulkan context
    let context = match VulkanContext::new("MeshUploadTest", None, None) {
        Ok(ctx) => ctx,
        Err(_) => {
            eprintln!("Skipping test - no Vulkan support");
            return;
        }
    };

    let asset_manager = Arc::new(AssetManager::new());
    let mut bridge = AssetBridge::new(context, asset_manager.clone());

    // Create a simple cube mesh
    let mesh = MeshData::cube();
    let mesh_id = engine_assets::AssetId::from_content(b"test_cube");

    // Insert mesh directly into asset manager
    let _handle = asset_manager.meshes().insert(mesh_id, mesh);

    // Upload to GPU
    let result = bridge.get_or_upload_mesh(mesh_id);
    assert!(result.is_ok(), "Mesh upload should succeed");

    let gpu_mesh = result.unwrap();
    assert_eq!(gpu_mesh.vertex_count, 24); // Cube has 24 vertices (6 faces * 4 vertices)
    assert_eq!(gpu_mesh.index_count, 36); // Cube has 36 indices (6 faces * 2 triangles * 3 indices)

    // Verify stats
    let stats = bridge.stats();
    assert_eq!(stats.mesh_count, 1);
    assert_eq!(stats.total_uploads, 1);
    assert_eq!(stats.cache_hits, 0);
}

/// Test mesh caching (second access should hit cache)
#[test]
fn test_mesh_caching() {
    let context = match VulkanContext::new("MeshCachingTest", None, None) {
        Ok(ctx) => ctx,
        Err(_) => {
            eprintln!("Skipping test - no Vulkan support");
            return;
        }
    };

    let asset_manager = Arc::new(AssetManager::new());
    let mut bridge = AssetBridge::new(context, asset_manager.clone());

    // Create and insert mesh
    let mesh = MeshData::cube();
    let mesh_id = engine_assets::AssetId::from_content(b"cached_cube");
    let _handle = asset_manager.meshes().insert(mesh_id, mesh);

    // First access - should upload
    let _gpu_mesh1 = bridge.get_or_upload_mesh(mesh_id).unwrap();
    let stats_after_first = bridge.stats();
    assert_eq!(stats_after_first.total_uploads, 1);
    assert_eq!(stats_after_first.cache_hits, 0);

    // Second access - should hit cache
    let _gpu_mesh2 = bridge.get_or_upload_mesh(mesh_id).unwrap();
    let stats_after_second = bridge.stats();
    assert_eq!(stats_after_second.total_uploads, 1); // No new uploads
    assert_eq!(stats_after_second.cache_hits, 1); // Cache hit!

    // Verify cache hit rate
    assert!((stats_after_second.cache_hit_rate - 0.5).abs() < 0.01); // 1 hit / 2 accesses = 50%
}

/// Test mesh eviction
#[test]
fn test_mesh_eviction() {
    let context = match VulkanContext::new("MeshEvictionTest", None, None) {
        Ok(ctx) => ctx,
        Err(_) => {
            eprintln!("Skipping test - no Vulkan support");
            return;
        }
    };

    let asset_manager = Arc::new(AssetManager::new());
    let mut bridge = AssetBridge::new(context, asset_manager.clone());

    // Create and upload mesh
    let mesh = MeshData::cube();
    let mesh_id = engine_assets::AssetId::from_content(b"evict_cube");
    let _handle = asset_manager.meshes().insert(mesh_id, mesh);
    let _gpu_mesh = bridge.get_or_upload_mesh(mesh_id).unwrap();

    // Verify it's cached
    assert_eq!(bridge.stats().mesh_count, 1);

    // Evict the mesh
    bridge.evict_mesh(mesh_id);

    // Verify it's no longer cached
    assert_eq!(bridge.stats().mesh_count, 0);

    // Re-accessing should upload again
    let _gpu_mesh2 = bridge.get_or_upload_mesh(mesh_id).unwrap();
    assert_eq!(bridge.stats().total_uploads, 2); // Second upload
}

/// Test hot-reload simulation (reload mesh)
#[test]
fn test_mesh_reload() {
    let context = match VulkanContext::new("MeshReloadTest", None, None) {
        Ok(ctx) => ctx,
        Err(_) => {
            eprintln!("Skipping test - no Vulkan support");
            return;
        }
    };

    let asset_manager = Arc::new(AssetManager::new());
    let mut bridge = AssetBridge::new(context, asset_manager.clone());

    // Create and upload initial mesh
    let mesh = MeshData::cube();
    let mesh_id = engine_assets::AssetId::from_content(b"reload_cube");
    let _handle = asset_manager.meshes().insert(mesh_id, mesh.clone());
    let _gpu_mesh1 = bridge.get_or_upload_mesh(mesh_id).unwrap();

    // Simulate hot-reload: reload the mesh
    bridge.reload_mesh(mesh_id).expect("Reload should succeed");

    // Verify old resource was evicted
    assert_eq!(bridge.stats().mesh_count, 0);

    // Re-upload should work
    let _gpu_mesh2 = bridge.get_or_upload_mesh(mesh_id).unwrap();
    assert_eq!(bridge.stats().mesh_count, 1);
}

/// Test multiple meshes
#[test]
fn test_multiple_meshes() {
    let context = match VulkanContext::new("MultipleMeshesTest", None, None) {
        Ok(ctx) => ctx,
        Err(_) => {
            eprintln!("Skipping test - no Vulkan support");
            return;
        }
    };

    let asset_manager = Arc::new(AssetManager::new());
    let mut bridge = AssetBridge::new(context, asset_manager.clone());

    // Create multiple meshes
    let cube = MeshData::cube();
    let triangle = MeshData::triangle();
    let cube2 = MeshData::cube(); // Another cube with different ID

    let cube_id = engine_assets::AssetId::from_content(b"cube");
    let triangle_id = engine_assets::AssetId::from_content(b"triangle");
    let cube2_id = engine_assets::AssetId::from_content(b"cube2");

    asset_manager.meshes().insert(cube_id, cube);
    asset_manager.meshes().insert(triangle_id, triangle);
    asset_manager.meshes().insert(cube2_id, cube2);

    // Upload all three
    let _cube_gpu = bridge.get_or_upload_mesh(cube_id).unwrap();
    let _triangle_gpu = bridge.get_or_upload_mesh(triangle_id).unwrap();
    let _cube2_gpu = bridge.get_or_upload_mesh(cube2_id).unwrap();

    // Verify all are cached
    let stats = bridge.stats();
    assert_eq!(stats.mesh_count, 3);
    assert_eq!(stats.total_uploads, 3);

    // Clear cache
    bridge.clear();
    assert_eq!(bridge.stats().mesh_count, 0);
}

/// Test invalid mesh (empty vertices)
#[test]
fn test_invalid_mesh() {
    let context = match VulkanContext::new("InvalidMeshTest", None, None) {
        Ok(ctx) => ctx,
        Err(_) => {
            eprintln!("Skipping test - no Vulkan support");
            return;
        }
    };

    let asset_manager = Arc::new(AssetManager::new());
    let mut bridge = AssetBridge::new(context, asset_manager.clone());

    // Create empty mesh (invalid)
    let empty_mesh = MeshData { vertices: vec![], indices: vec![] };
    let mesh_id = engine_assets::AssetId::from_content(b"empty");
    asset_manager.meshes().insert(mesh_id, empty_mesh);

    // Upload should fail
    let result = bridge.get_or_upload_mesh(mesh_id);
    assert!(result.is_err(), "Empty mesh upload should fail");
}

/// Test asset not found error
#[test]
fn test_asset_not_found() {
    let context = match VulkanContext::new("AssetNotFoundTest", None, None) {
        Ok(ctx) => ctx,
        Err(_) => {
            eprintln!("Skipping test - no Vulkan support");
            return;
        }
    };

    let asset_manager = Arc::new(AssetManager::new());
    let mut bridge = AssetBridge::new(context, asset_manager);

    // Try to upload non-existent mesh
    let nonexistent_id = engine_assets::AssetId::from_content(b"does_not_exist");
    let result = bridge.get_or_upload_mesh(nonexistent_id);
    assert!(result.is_err(), "Non-existent mesh should return error");
}

/// Benchmark: mesh upload performance
#[test]
#[ignore] // Run with --ignored for benchmarks
fn bench_mesh_upload_performance() {
    let context = match VulkanContext::new_for_benchmarks("MeshBenchmark", None, None) {
        Ok(ctx) => ctx,
        Err(_) => {
            eprintln!("Skipping benchmark - no Vulkan support");
            return;
        }
    };

    let asset_manager = Arc::new(AssetManager::new());
    let mut bridge = AssetBridge::new(context, asset_manager.clone());

    // Create a simple mesh (cube has 24 vertices)
    let mesh = MeshData::cube();
    let mesh_id = engine_assets::AssetId::from_content(b"cube");
    asset_manager.meshes().insert(mesh_id, mesh);

    // Time the upload
    let start = std::time::Instant::now();
    let _gpu_mesh = bridge.get_or_upload_mesh(mesh_id).expect("Upload should succeed");
    let elapsed = start.elapsed();

    println!("Cube mesh upload time: {:?}", elapsed);

    // Target: < 5ms for small mesh
    assert!(elapsed.as_millis() < 5, "Upload took too long: {:?}", elapsed);
}
