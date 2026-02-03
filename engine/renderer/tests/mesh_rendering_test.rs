//! Integration test for mesh rendering (Phase 1.8)
//!
//! Tests the complete mesh rendering pipeline:
//! - Transform component integration
//! - Camera component integration
//! - MeshRenderer component integration
//! - Graphics pipeline with MVP matrix push constants
//! - GPU mesh upload and caching
//! - Depth buffer rendering

use engine_assets::{AssetId, MeshData};
use engine_core::{Camera, MeshRenderer, Quat, Transform, Vec3, World};
use engine_renderer::{
    DepthBuffer, GpuCache, GraphicsPipeline, RenderPass, RenderPassConfig, VulkanContext,
};

/// Test creating a graphics pipeline for mesh rendering
#[test]
fn test_create_mesh_pipeline() {
    // Create headless context
    let context = VulkanContext::new("test_mesh_pipeline", None, None).unwrap();

    // Create render pass with depth
    let render_pass = RenderPass::new(
        &context.device,
        RenderPassConfig {
            color_format: ash::vk::Format::B8G8R8A8_UNORM,
            depth_format: Some(ash::vk::Format::D32_SFLOAT),
            samples: ash::vk::SampleCountFlags::TYPE_1,
            load_op: ash::vk::AttachmentLoadOp::CLEAR,
            store_op: ash::vk::AttachmentStoreOp::STORE,
        },
    )
    .unwrap();

    // Create pipeline
    let pipeline = GraphicsPipeline::new_mesh_pipeline(
        &context.device,
        &render_pass,
        ash::vk::Extent2D { width: 800, height: 600 },
        Some(ash::vk::Format::D32_SFLOAT),
    )
    .unwrap();

    // Verify pipeline was created
    assert_ne!(pipeline.handle(), ash::vk::Pipeline::null());
    assert_ne!(pipeline.layout(), ash::vk::PipelineLayout::null());
}

/// Test depth buffer creation
#[test]
fn test_create_depth_buffer() {
    let context = VulkanContext::new("test_depth_buffer", None, None).unwrap();

    let depth_buffer = DepthBuffer::new(
        &context.device,
        &context.allocator,
        ash::vk::Extent2D { width: 1920, height: 1080 },
    )
    .unwrap();

    // Verify depth buffer was created
    assert_ne!(depth_buffer.image_view(), ash::vk::ImageView::null());
    assert_eq!(depth_buffer.format(), ash::vk::Format::D32_SFLOAT);
}

/// Test GPU mesh upload via GpuCache
#[test]
fn test_gpu_mesh_upload() {
    let context = VulkanContext::new("test_gpu_mesh", None, None).unwrap();
    let mut gpu_cache = GpuCache::new(&context).unwrap();

    // Create a simple cube mesh
    let cube = MeshData::cube();
    let asset_id = AssetId::from_seed_and_params(12345, b"test_mesh");

    // Upload to GPU
    gpu_cache.upload_mesh(&context, asset_id, &cube).unwrap();

    // Verify mesh is cached
    assert!(gpu_cache.contains(asset_id));

    // Verify mesh info
    let info = gpu_cache.get_mesh_info(asset_id).unwrap();
    assert_eq!(info.vertex_count, 24); // Cube has 24 vertices (6 faces * 4 corners)
    assert_eq!(info.index_count, 36); // Cube has 36 indices (6 faces * 2 triangles * 3 vertices)
}

/// Test Transform component MVP matrix calculation
#[test]
fn test_transform_mvp_matrix() {
    // Create transform
    let transform = Transform::new(Vec3::new(0.0, 0.0, -5.0), Quat::IDENTITY, Vec3::ONE);

    // Create camera
    let mut camera = Camera::new(std::f32::consts::FRAC_PI_4, 16.0 / 9.0);

    // Calculate matrices
    let model = transform.to_matrix();
    let view = camera.view_matrix(&transform);
    let projection = camera.projection_matrix();

    // Compose MVP
    let mvp = projection * view * model;

    // Verify MVP is not zero (just check determinant is non-zero)
    assert!(!mvp.is_nan());

    // Verify position is correct
    assert!((transform.position.x - 0.0).abs() < 0.001);
    assert!((transform.position.y - 0.0).abs() < 0.001);
    assert!((transform.position.z - (-5.0)).abs() < 0.001);
}

/// Test Camera component view matrix generation
#[test]
fn test_camera_view_matrix() {
    let mut camera = Camera::new(std::f32::consts::FRAC_PI_4, 16.0 / 9.0);

    // Camera at origin, looking down -Z
    let camera_transform = Transform::new(Vec3::new(0.0, 0.0, 0.0), Quat::IDENTITY, Vec3::ONE);

    let view = camera.view_matrix(&camera_transform);
    let projection = camera.projection_matrix();

    // Verify matrices are valid (not NaN)
    assert!(!view.is_nan());
    assert!(!projection.is_nan());

    // View-projection composition
    let vp = camera.view_projection_matrix(&camera_transform);
    assert_eq!(vp, projection * view);
}

/// Test MeshRenderer component creation
#[test]
fn test_mesh_renderer_component() {
    let mesh_id = 12345u64;
    let renderer = MeshRenderer::new(mesh_id);

    assert_eq!(renderer.mesh_id, mesh_id);
    assert!(renderer.is_visible());

    // Test visibility toggle
    let mut renderer_mut = renderer;
    renderer_mut.set_visible(false);
    assert!(!renderer_mut.is_visible());
}

/// Test ECS integration with rendering components
#[test]
fn test_ecs_rendering_components() {
    let mut world = World::new();

    // Spawn an entity with Transform + MeshRenderer
    let entity = world.spawn();
    world.add(entity, Transform::new(Vec3::new(1.0, 2.0, 3.0), Quat::IDENTITY, Vec3::ONE));
    world.add(entity, MeshRenderer::new(12345));

    // Spawn camera entity
    let camera_entity = world.spawn();
    world.add(
        camera_entity,
        Transform::new(Vec3::new(0.0, 0.0, 5.0), Quat::IDENTITY, Vec3::ONE),
    );
    world.add(camera_entity, Camera::new(std::f32::consts::FRAC_PI_4, 16.0 / 9.0));

    // Query renderable entities (query() returns an iterator directly)
    let count = world.query::<(&Transform, &MeshRenderer)>().count();
    assert_eq!(count, 1, "Should find 1 renderable entity");

    // Query camera
    let camera_count = world.query::<(&Transform, &Camera)>().count();
    assert_eq!(camera_count, 1, "Should find 1 camera");
}

/// Test MVP matrix push constant size
#[test]
fn test_mvp_push_constant_size() {
    // Verify Mat4 is exactly 64 bytes (required for push constants)
    assert_eq!(
        std::mem::size_of::<glam::Mat4>(),
        64,
        "Mat4 must be 64 bytes for push constants"
    );
}

/// Test complete mesh rendering setup (no actual rendering, just setup verification)
#[test]
fn test_mesh_rendering_setup() {
    // Create context
    let context = VulkanContext::new("test_mesh_rendering", None, None).unwrap();

    // Create GPU cache
    let mut gpu_cache = GpuCache::new(&context).unwrap();

    // Create mesh
    let cube = MeshData::cube();
    let asset_id = AssetId::from_seed_and_params(1, b"cube");

    // Upload mesh
    gpu_cache.upload_mesh(&context, asset_id, &cube).unwrap();

    // Create render pass with depth
    let render_pass = RenderPass::new(
        &context.device,
        RenderPassConfig {
            color_format: ash::vk::Format::B8G8R8A8_UNORM,
            depth_format: Some(ash::vk::Format::D32_SFLOAT),
            samples: ash::vk::SampleCountFlags::TYPE_1,
            load_op: ash::vk::AttachmentLoadOp::CLEAR,
            store_op: ash::vk::AttachmentStoreOp::STORE,
        },
    )
    .unwrap();

    // Create pipeline
    let pipeline = GraphicsPipeline::new_mesh_pipeline(
        &context.device,
        &render_pass,
        ash::vk::Extent2D { width: 800, height: 600 },
        Some(ash::vk::Format::D32_SFLOAT),
    )
    .unwrap();

    // Create depth buffer
    let depth_buffer = DepthBuffer::new(
        &context.device,
        &context.allocator,
        ash::vk::Extent2D { width: 800, height: 600 },
    )
    .unwrap();

    // Create ECS world with entities
    let mut world = World::new();

    // Spawn cube entity
    let cube_entity = world.spawn();
    world.add(
        cube_entity,
        Transform::new(Vec3::new(0.0, 0.0, -5.0), Quat::IDENTITY, Vec3::ONE),
    );
    world.add(cube_entity, MeshRenderer::new(1)); // Use u64 mesh ID directly

    // Spawn camera
    let camera_entity = world.spawn();
    world.add(
        camera_entity,
        Transform::new(Vec3::new(0.0, 0.0, 0.0), Quat::IDENTITY, Vec3::ONE),
    );
    world.add(camera_entity, Camera::new(std::f32::consts::FRAC_PI_4, 16.0 / 9.0));

    // Verify all components created successfully
    assert_ne!(pipeline.handle(), ash::vk::Pipeline::null());
    assert_ne!(depth_buffer.image_view(), ash::vk::ImageView::null());
    assert!(gpu_cache.contains(asset_id));

    // Query entities (query() returns an iterator directly)
    let renderable_count = world.query::<(&Transform, &MeshRenderer)>().count();
    let camera_count = world.query::<(&Transform, &Camera)>().count();

    assert_eq!(renderable_count, 1);
    assert_eq!(camera_count, 1);
}

/// Benchmark: MVP matrix calculation performance
#[test]
fn test_mvp_performance() {
    let transform = Transform::new(Vec3::new(1.0, 2.0, 3.0), Quat::IDENTITY, Vec3::ONE);
    let mut camera = Camera::new(std::f32::consts::FRAC_PI_4, 16.0 / 9.0);

    let start = std::time::Instant::now();
    for _ in 0..10000 {
        let model = transform.to_matrix();
        let view = camera.view_matrix(&transform);
        let projection = camera.projection_matrix();
        let _mvp = projection * view * model;
    }
    let elapsed = start.elapsed();

    // Should complete 10K matrix calculations in under 5ms (target: <0.5µs per calculation)
    assert!(
        elapsed.as_millis() < 5,
        "MVP calculation too slow: {:?} for 10K iterations",
        elapsed
    );
}
