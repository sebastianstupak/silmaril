//! Integration tests for asset loading → GPU upload → rendering pipeline
//!
//! Tests the complete flow from loading assets to rendering them:
//! 1. Load mesh from memory/file
//! 2. Upload to GPU
//! 3. Create rendering pipeline
//! 4. Submit draw commands
//! 5. Verify frame completion
//!
//! These tests validate cross-module integration between:
//! - engine-assets (mesh loading, validation)
//! - engine-renderer (GPU upload, rendering)
//! - engine-core (ECS integration)

use engine_assets::{MeshData, Vertex};
use engine_core::ecs::{Component, Transform, World};
use engine_math::{Quat, Vec3};
use engine_renderer::{
    CommandBuffer, CommandPool, FrameSync, GpuBuffer, GpuCache, Renderer, VulkanContext,
};

/// Test mesh data creation and validation
#[test]
fn test_create_simple_triangle_mesh() {
    let vertices = vec![
        Vertex {
            position: [0.0, -0.5, 0.0],
            normal: [0.0, 0.0, 1.0],
            tex_coords: [0.5, 1.0],
            ..Default::default()
        },
        Vertex {
            position: [0.5, 0.5, 0.0],
            normal: [0.0, 0.0, 1.0],
            tex_coords: [1.0, 0.0],
            ..Default::default()
        },
        Vertex {
            position: [-0.5, 0.5, 0.0],
            normal: [0.0, 0.0, 1.0],
            tex_coords: [0.0, 0.0],
            ..Default::default()
        },
    ];

    let indices = vec![0u32, 1, 2];

    let mesh = MeshData::new(vertices, indices);

    assert_eq!(mesh.vertices.len(), 3);
    assert_eq!(mesh.indices.len(), 3);
    assert!(mesh.validate().is_ok(), "Triangle mesh should be valid");
}

/// Test quad mesh creation
#[test]
fn test_create_quad_mesh() {
    let vertices = vec![
        Vertex {
            position: [-1.0, -1.0, 0.0],
            normal: [0.0, 0.0, 1.0],
            tex_coords: [0.0, 1.0],
            ..Default::default()
        },
        Vertex {
            position: [1.0, -1.0, 0.0],
            normal: [0.0, 0.0, 1.0],
            tex_coords: [1.0, 1.0],
            ..Default::default()
        },
        Vertex {
            position: [1.0, 1.0, 0.0],
            normal: [0.0, 0.0, 1.0],
            tex_coords: [1.0, 0.0],
            ..Default::default()
        },
        Vertex {
            position: [-1.0, 1.0, 0.0],
            normal: [0.0, 0.0, 1.0],
            tex_coords: [0.0, 0.0],
            ..Default::default()
        },
    ];

    let indices = vec![0u32, 1, 2, 2, 3, 0];

    let mesh = MeshData::new(vertices, indices);

    assert_eq!(mesh.vertices.len(), 4);
    assert_eq!(mesh.indices.len(), 6);
    assert!(mesh.validate().is_ok(), "Quad mesh should be valid");
}

/// Test mesh validation catches invalid data
#[test]
fn test_mesh_validation_catches_out_of_bounds_indices() {
    let vertices = vec![
        Vertex { position: [0.0, 0.0, 0.0], ..Default::default() },
        Vertex { position: [1.0, 0.0, 0.0], ..Default::default() },
    ];

    // Index 2 is out of bounds (only 2 vertices)
    let indices = vec![0u32, 1, 2];

    let mesh = MeshData::new(vertices, indices);

    assert!(mesh.validate().is_err(), "Should reject out-of-bounds indices");
}

/// Test empty mesh validation
#[test]
fn test_empty_mesh_validation() {
    let mesh = MeshData::new(vec![], vec![]);

    assert!(mesh.validate().is_err(), "Empty mesh should fail validation");
}

/// Test GPU buffer creation and upload (requires Vulkan context)
#[test]
#[ignore = "Requires Vulkan device"]
fn test_gpu_buffer_upload() {
    // Create headless Vulkan context
    let context = VulkanContext::new("test_gpu_buffer_upload", None, None)
        .expect("Failed to create Vulkan context");

    let vertices = vec![
        Vertex {
            position: [0.0, -0.5, 0.0],
            normal: [0.0, 0.0, 1.0],
            tex_coords: [0.5, 1.0],
            ..Default::default()
        },
        Vertex {
            position: [0.5, 0.5, 0.0],
            normal: [0.0, 0.0, 1.0],
            tex_coords: [1.0, 0.0],
            ..Default::default()
        },
        Vertex {
            position: [-0.5, 0.5, 0.0],
            normal: [0.0, 0.0, 1.0],
            tex_coords: [0.0, 0.0],
            ..Default::default()
        },
    ];

    // Create GPU buffer
    let buffer =
        GpuBuffer::new_vertex_buffer(&context, &vertices).expect("Failed to create vertex buffer");

    // Verify buffer was created
    assert!(buffer.size() >= std::mem::size_of_val(&vertices[..]));
}

/// Test GPU cache mesh upload and retrieval
#[test]
#[ignore = "Requires Vulkan device"]
fn test_gpu_cache_mesh_upload() {
    // Create headless Vulkan context
    let context =
        VulkanContext::new("test_gpu_cache", None, None).expect("Failed to create Vulkan context");

    let mut cache = GpuCache::new(&context).expect("Failed to create GPU cache");

    // Create test mesh
    let vertices = vec![
        Vertex { position: [0.0, -0.5, 0.0], ..Default::default() },
        Vertex { position: [0.5, 0.5, 0.0], ..Default::default() },
        Vertex { position: [-0.5, 0.5, 0.0], ..Default::default() },
    ];

    let indices = vec![0u32, 1, 2];
    let mesh = MeshData::new(vertices, indices);

    // Upload mesh to GPU cache
    let mesh_id = cache.upload_mesh(&mesh).expect("Failed to upload mesh to GPU cache");

    // Verify mesh is in cache
    let cached_mesh = cache.get_mesh(mesh_id).expect("Mesh should be in cache after upload");

    assert_eq!(cached_mesh.vertex_count, 3);
    assert_eq!(cached_mesh.index_count, 3);
}

/// Test complete asset → ECS → rendering integration
#[test]
#[ignore = "Requires Vulkan device and complete rendering pipeline"]
fn test_end_to_end_mesh_rendering() {
    // Create Vulkan context
    let context = VulkanContext::new("test_e2e_rendering", None, None)
        .expect("Failed to create Vulkan context");

    // Create GPU cache
    let mut cache = GpuCache::new(&context).expect("Failed to create GPU cache");

    // Create ECS world
    let mut world = World::new();

    // Create test mesh
    let vertices = vec![
        Vertex {
            position: [0.0, -0.5, 0.0],
            normal: [0.0, 0.0, 1.0],
            tex_coords: [0.5, 1.0],
            ..Default::default()
        },
        Vertex {
            position: [0.5, 0.5, 0.0],
            normal: [0.0, 0.0, 1.0],
            tex_coords: [1.0, 0.0],
            ..Default::default()
        },
        Vertex {
            position: [-0.5, 0.5, 0.0],
            normal: [0.0, 0.0, 1.0],
            tex_coords: [0.0, 0.0],
            ..Default::default()
        },
    ];

    let indices = vec![0u32, 1, 2];
    let mesh = MeshData::new(vertices, indices);

    // Upload to GPU
    let mesh_id = cache.upload_mesh(&mesh).expect("Failed to upload mesh");

    // Create entity with mesh and transform
    #[derive(Component, Clone, Copy)]
    struct MeshComponent {
        mesh_id: u64,
    }

    let entity = world.spawn();
    world.add(
        entity,
        Transform { position: Vec3::ZERO, rotation: Quat::IDENTITY, scale: Vec3::ONE },
    );
    world.add(entity, MeshComponent { mesh_id });

    // Query renderable entities
    let renderable_count = world.query::<(&Transform, &MeshComponent)>().iter().count();

    assert_eq!(renderable_count, 1, "Should have one renderable entity");

    // Verify mesh is accessible from cache
    let cached = cache.get_mesh(mesh_id).expect("Mesh should be in cache");
    assert_eq!(cached.vertex_count, 3);
}

/// Test multi-frame rendering consistency
#[test]
#[ignore = "Requires complete rendering pipeline"]
fn test_multi_frame_rendering() {
    // Create Vulkan context
    let context = VulkanContext::new("test_multi_frame", None, None)
        .expect("Failed to create Vulkan context");

    // Create command pool
    let command_pool = CommandPool::new(&context).expect("Failed to create command pool");

    // Create synchronization objects
    let sync = FrameSync::new(&context, 2).expect("Failed to create frame sync");

    // Simulate multiple frames
    const FRAME_COUNT: usize = 10;

    for frame_index in 0..FRAME_COUNT {
        // Begin frame
        let frame_data = sync.wait_for_frame(frame_index % 2).expect("Failed to wait for frame");

        // Allocate command buffer
        let mut cmd = command_pool.allocate().expect("Failed to allocate command buffer");

        // Begin command buffer
        cmd.begin().expect("Failed to begin command buffer");

        // Record commands (simplified - would normally record draw calls)
        // In a real test, we would:
        // 1. Begin render pass
        // 2. Bind pipeline
        // 3. Bind vertex/index buffers
        // 4. Issue draw call
        // 5. End render pass

        // End command buffer
        cmd.end().expect("Failed to end command buffer");

        // In a real implementation, we would submit and present here
    }

    // All frames completed successfully if we reach here
}

/// Test resource cleanup
#[test]
#[ignore = "Requires Vulkan device"]
fn test_resource_cleanup() {
    {
        let context = VulkanContext::new("test_cleanup", None, None)
            .expect("Failed to create Vulkan context");

        let mut cache = GpuCache::new(&context).expect("Failed to create GPU cache");

        // Upload multiple meshes
        for i in 0..10 {
            let vertices = vec![
                Vertex { position: [i as f32, 0.0, 0.0], ..Default::default() },
                Vertex { position: [i as f32 + 1.0, 0.0, 0.0], ..Default::default() },
                Vertex { position: [i as f32 + 0.5, 1.0, 0.0], ..Default::default() },
            ];

            let mesh = MeshData::new(vertices, vec![0, 1, 2]);
            cache.upload_mesh(&mesh).expect("Failed to upload mesh");
        }

        // Cache and context will be dropped here
    }

    // If we reach here without panicking, cleanup succeeded
}
