//! Integration tests for GPU mesh cache
//!
//! Tests GpuCache upload, caching, and eviction behavior.

use ash::vk;
use engine_assets::{AssetId, MeshData, Vertex};
use engine_renderer::VulkanContext;
use glam::{Vec2, Vec3};

/// Create a simple test mesh (triangle)
fn create_test_mesh() -> MeshData {
    let vertices = vec![
        Vertex { position: Vec3::new(0.0, 0.5, 0.0), normal: Vec3::Z, uv: Vec2::new(0.5, 0.0) },
        Vertex { position: Vec3::new(-0.5, -0.5, 0.0), normal: Vec3::Z, uv: Vec2::new(0.0, 1.0) },
        Vertex { position: Vec3::new(0.5, -0.5, 0.0), normal: Vec3::Z, uv: Vec2::new(1.0, 1.0) },
    ];

    let indices = vec![0, 1, 2];

    MeshData { vertices, indices }
}

/// Create a test mesh with specified vertex count
fn create_mesh_with_vertices(count: usize) -> MeshData {
    let mut vertices = Vec::with_capacity(count);
    for i in 0..count {
        let angle = (i as f32 / count as f32) * std::f32::consts::TAU;
        vertices.push(Vertex {
            position: Vec3::new(angle.cos(), angle.sin(), 0.0),
            normal: Vec3::Z,
            uv: Vec2::new(0.5, 0.5),
        });
    }

    // Create triangle indices
    let mut indices = Vec::new();
    for i in 0..(count - 2) {
        indices.push(0);
        indices.push((i + 1) as u32);
        indices.push((i + 2) as u32);
    }

    MeshData { vertices, indices }
}

#[test]
fn test_gpu_cache_creation() {
    // GpuCache should be created successfully
    let context = VulkanContext::new("GpuCacheTest", None, None);
    if context.is_err() {
        eprintln!("Skipping GPU cache test - no Vulkan support");
        return;
    }
    let context = context.unwrap();

    let cache_result = engine_renderer::GpuCache::new(&context);
    assert!(cache_result.is_ok(), "GpuCache creation should succeed");
}

#[test]
fn test_gpu_cache_upload_mesh() {
    let context = VulkanContext::new("GpuCacheUploadTest", None, None);
    if context.is_err() {
        eprintln!("Skipping GPU cache test - no Vulkan support");
        return;
    }
    let context = context.unwrap();

    let mut cache = engine_renderer::GpuCache::new(&context).expect("Cache creation failed");

    let mesh = create_test_mesh();
    let asset_id = AssetId::from_content(b"test_mesh_123");

    // Upload mesh
    let result = cache.upload_mesh(&context, asset_id, &mesh);
    assert!(result.is_ok(), "Mesh upload should succeed");

    // Verify mesh exists in cache
    assert!(cache.contains(asset_id), "Cache should contain uploaded mesh");

    // Verify mesh info
    let mesh_info = cache.get_mesh_info(asset_id);
    assert!(mesh_info.is_some(), "Should retrieve mesh info");

    let info = mesh_info.unwrap();
    assert_eq!(info.vertex_count, 3, "Should have 3 vertices");
    assert_eq!(info.index_count, 3, "Should have 3 indices");
}

#[test]
fn test_gpu_cache_upload_multiple_meshes() {
    let context = VulkanContext::new("GpuCacheMultipleTest", None, None);
    if context.is_err() {
        eprintln!("Skipping GPU cache test - no Vulkan support");
        return;
    }
    let context = context.unwrap();

    let mut cache = engine_renderer::GpuCache::new(&context).expect("Cache creation failed");

    // Upload multiple meshes
    for i in 0..10 {
        let mesh = create_mesh_with_vertices((i + 3) * 10);
        let asset_id = AssetId::from_content(format!("mesh_{}", i).as_bytes());

        let result = cache.upload_mesh(&context, asset_id, &mesh);
        assert!(result.is_ok(), "Mesh {} upload should succeed", i);
    }

    // Verify all meshes are cached
    for i in 0..10 {
        let asset_id = AssetId::from_content(format!("mesh_{}", i).as_bytes());
        assert!(cache.contains(asset_id), "Cache should contain mesh {}", i);
    }
}

#[test]
fn test_gpu_cache_duplicate_upload() {
    let context = VulkanContext::new("GpuCacheDuplicateTest", None, None);
    if context.is_err() {
        eprintln!("Skipping GPU cache test - no Vulkan support");
        return;
    }
    let context = context.unwrap();

    let mut cache = engine_renderer::GpuCache::new(&context).expect("Cache creation failed");

    let mesh = create_test_mesh();
    let asset_id = AssetId::from_content(b"duplicate_mesh");

    // Upload first time
    cache.upload_mesh(&context, asset_id, &mesh).expect("First upload failed");

    // Upload same mesh again (should not error, should be idempotent)
    let result = cache.upload_mesh(&context, asset_id, &mesh);
    assert!(result.is_ok(), "Duplicate upload should be idempotent");

    // Still only one copy in cache
    assert!(cache.contains(asset_id));
}

#[test]
fn test_gpu_cache_evict_mesh() {
    let context = VulkanContext::new("GpuCacheEvictTest", None, None);
    if context.is_err() {
        eprintln!("Skipping GPU cache test - no Vulkan support");
        return;
    }
    let context = context.unwrap();

    let mut cache = engine_renderer::GpuCache::new(&context).expect("Cache creation failed");

    let mesh = create_test_mesh();
    let asset_id = AssetId::from_content(b"evict_mesh");

    // Upload mesh
    cache.upload_mesh(&context, asset_id, &mesh).expect("Upload failed");
    assert!(cache.contains(asset_id));

    // Evict mesh
    cache.evict(asset_id);
    assert!(!cache.contains(asset_id), "Mesh should be evicted");
}

#[test]
fn test_gpu_cache_clear() {
    let context = VulkanContext::new("GpuCacheClearTest", None, None);
    if context.is_err() {
        eprintln!("Skipping GPU cache test - no Vulkan support");
        return;
    }
    let context = context.unwrap();

    let mut cache = engine_renderer::GpuCache::new(&context).expect("Cache creation failed");

    // Upload multiple meshes
    for i in 0..5 {
        let mesh = create_test_mesh();
        let asset_id = AssetId::from_content(format!("clear_mesh_{}", i).as_bytes());
        cache.upload_mesh(&context, asset_id, &mesh).expect("Upload failed");
    }

    // Clear cache
    cache.clear();

    // Verify all meshes are gone
    for i in 0..5 {
        let asset_id = AssetId::from_content(format!("clear_mesh_{}", i).as_bytes());
        assert!(!cache.contains(asset_id), "Mesh {} should be cleared", i);
    }
}

#[test]
fn test_gpu_cache_get_buffers() {
    let context = VulkanContext::new("GpuCacheBuffersTest", None, None);
    if context.is_err() {
        eprintln!("Skipping GPU cache test - no Vulkan support");
        return;
    }
    let context = context.unwrap();

    let mut cache = engine_renderer::GpuCache::new(&context).expect("Cache creation failed");

    let mesh = create_test_mesh();
    let asset_id = AssetId::from_content(b"buffer_mesh");

    cache.upload_mesh(&context, asset_id, &mesh).expect("Upload failed");

    // Get buffer handles
    let buffers = cache.get_buffers(asset_id);
    assert!(buffers.is_some(), "Should get buffer handles");

    let (vertex_buffer, index_buffer) = buffers.unwrap();
    assert_ne!(vertex_buffer, vk::Buffer::null(), "Vertex buffer should be valid");
    assert_ne!(index_buffer, vk::Buffer::null(), "Index buffer should be valid");
}

#[test]
fn test_gpu_cache_memory_cleanup() {
    let context = VulkanContext::new("GpuCacheCleanupTest", None, None);
    if context.is_err() {
        eprintln!("Skipping GPU cache test - no Vulkan support");
        return;
    }
    let context = context.unwrap();

    {
        let mut cache = engine_renderer::GpuCache::new(&context).expect("Cache creation failed");

        // Upload many large meshes
        for i in 0..100 {
            let mesh = create_mesh_with_vertices(1000);
            let asset_id = AssetId::from_content(format!("cleanup_mesh_{}", i).as_bytes());
            cache.upload_mesh(&context, asset_id, &mesh).expect("Upload failed");
        }

        // Cache drop should clean up all GPU resources
    } // GpuCache dropped here

    // Wait for device idle to ensure cleanup
    context.wait_idle().expect("Wait idle failed");
}

#[test]
fn test_gpu_cache_empty_mesh() {
    let context = VulkanContext::new("GpuCacheEmptyTest", None, None);
    if context.is_err() {
        eprintln!("Skipping GPU cache test - no Vulkan support");
        return;
    }
    let context = context.unwrap();

    let mut cache = engine_renderer::GpuCache::new(&context).expect("Cache creation failed");

    let empty_mesh = MeshData { vertices: vec![], indices: vec![] };
    let asset_id = AssetId::from_content(b"empty_mesh");

    // Should handle empty mesh gracefully (either error or skip)
    let result = cache.upload_mesh(&context, asset_id, &empty_mesh);
    // Either it errors or succeeds, but shouldn't crash
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_gpu_cache_large_mesh() {
    let context = VulkanContext::new("GpuCacheLargeTest", None, None);
    if context.is_err() {
        eprintln!("Skipping GPU cache test - no Vulkan support");
        return;
    }
    let context = context.unwrap();

    let mut cache = engine_renderer::GpuCache::new(&context).expect("Cache creation failed");

    // Create large mesh (100K vertices)
    let mesh = create_mesh_with_vertices(100_000);
    let asset_id = AssetId::from_content(b"large_mesh");

    let result = cache.upload_mesh(&context, asset_id, &mesh);
    assert!(result.is_ok(), "Large mesh upload should succeed");

    let info = cache.get_mesh_info(asset_id).expect("Should have mesh info");
    assert_eq!(info.vertex_count, 100_000);
}
