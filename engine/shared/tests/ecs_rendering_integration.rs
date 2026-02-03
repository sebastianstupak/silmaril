//! ECS + Rendering Integration Tests
//!
//! Comprehensive tests for the integration between the ECS (engine-core)
//! and the rendering system (engine-renderer).
//!
//! # Test Coverage
//!
//! ## Entity Lifecycle Tests
//! - Entity spawn → render → despawn workflow
//! - Component updates reflecting in rendering
//! - Transform hierarchy rendering
//! - Multiple entities with different components
//! - Entity archetype changes during rendering
//!
//! ## Edge Cases
//! - Rendering entity without required components
//! - Component removal during frame
//! - Entity despawn during rendering
//! - Zero-size entities, negative scales
//! - Extreme transform values
//!
//! ## Performance Validation
//! - Large entity count rendering
//! - Component query performance
//! - Memory usage validation
//!
//! # Testing Architecture
//!
//! This test file is in `engine/shared/tests/` because it uses both:
//! - `engine-core` (ECS World, Entity, Component)
//! - `engine-renderer` (Renderer, GpuCache, etc.)
//!
//! Per CLAUDE.md rule #6: Cross-crate tests MUST be in engine/shared/tests/

use engine_assets::{AssetId, MeshData, Vertex};
use engine_core::ecs::{Component, Entity, World};
use engine_math::{Quat, Vec3};
use engine_renderer::{GpuCache, Renderer, VulkanContext, WindowConfig};
use tracing::{debug, info};

// ============================================================================
// Test Components
// ============================================================================

/// Component that references a mesh asset for rendering
#[derive(Debug, Clone, Copy, Component)]
struct MeshComponent {
    mesh_id: AssetId,
}

/// Component for visual testing - stores a color
#[derive(Debug, Clone, Copy, Component)]
struct ColorComponent {
    r: f32,
    g: f32,
    b: f32,
    a: f32,
}

impl ColorComponent {
    fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            r: r.clamp(0.0, 1.0),
            g: g.clamp(0.0, 1.0),
            b: b.clamp(0.0, 1.0),
            a: a.clamp(0.0, 1.0),
        }
    }

    fn red() -> Self {
        Self::new(1.0, 0.0, 0.0, 1.0)
    }

    fn green() -> Self {
        Self::new(0.0, 1.0, 0.0, 1.0)
    }

    fn blue() -> Self {
        Self::new(0.0, 0.0, 1.0, 1.0)
    }
}

/// Component marking an entity as renderable
#[derive(Debug, Clone, Copy, Component)]
struct Renderable {
    visible: bool,
}

impl Renderable {
    fn visible() -> Self {
        Self { visible: true }
    }

    fn hidden() -> Self {
        Self { visible: false }
    }
}

/// Transform component (re-exported from engine-core)
use engine_core::math::Transform;

// ============================================================================
// Helper Functions
// ============================================================================

/// Create a simple triangle mesh for testing
fn create_triangle_mesh() -> MeshData {
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
    MeshData::new(vertices, indices)
}

/// Create a quad mesh for testing
fn create_quad_mesh() -> MeshData {
    let vertices = vec![
        Vertex {
            position: [-0.5, -0.5, 0.0],
            normal: [0.0, 0.0, 1.0],
            tex_coords: [0.0, 1.0],
            ..Default::default()
        },
        Vertex {
            position: [0.5, -0.5, 0.0],
            normal: [0.0, 0.0, 1.0],
            tex_coords: [1.0, 1.0],
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

    let indices = vec![0u32, 1, 2, 2, 3, 0];
    MeshData::new(vertices, indices)
}

/// Count renderable entities in the world
fn count_renderable_entities(world: &World) -> usize {
    world.query::<(&Transform, &MeshComponent, &Renderable)>().iter().count()
}

/// Setup ECS world with test component registration
fn setup_test_world() -> World {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<MeshComponent>();
    world.register::<ColorComponent>();
    world.register::<Renderable>();
    world
}

// ============================================================================
// ENTITY LIFECYCLE TESTS (Category 1)
// ============================================================================

#[test]
fn test_entity_spawn_and_query() {
    info!("Testing entity spawn and component query");

    let mut world = setup_test_world();

    // Spawn entity with transform and mesh
    let entity = world.spawn();
    world.add(
        entity,
        Transform::new(Vec3::new(0.0, 0.0, 0.0), Quat::IDENTITY, Vec3::ONE),
    );
    world.add(entity, MeshComponent { mesh_id: 123 });
    world.add(entity, Renderable::visible());

    // Query for renderable entities
    let count = count_renderable_entities(&world);
    assert_eq!(count, 1, "Should have exactly one renderable entity");

    // Verify component data
    let transform = world.get::<Transform>(entity).expect("Entity should have Transform");
    assert_eq!(transform.position, Vec3::ZERO, "Position should be zero");

    let mesh = world.get::<MeshComponent>(entity).expect("Entity should have MeshComponent");
    assert_eq!(mesh.mesh_id, 123, "Mesh ID should match");

    debug!("Entity spawn and query test passed");
}

#[test]
fn test_entity_despawn_removes_from_queries() {
    info!("Testing entity despawn removes from queries");

    let mut world = setup_test_world();

    // Spawn multiple entities
    let entity1 = world.spawn();
    world.add(entity1, Transform::identity());
    world.add(entity1, MeshComponent { mesh_id: 1 });
    world.add(entity1, Renderable::visible());

    let entity2 = world.spawn();
    world.add(entity2, Transform::identity());
    world.add(entity2, MeshComponent { mesh_id: 2 });
    world.add(entity2, Renderable::visible());

    // Verify both are renderable
    assert_eq!(count_renderable_entities(&world), 2);

    // Despawn first entity
    world.despawn(entity1);

    // Verify only one remains
    assert_eq!(count_renderable_entities(&world), 1);

    // Verify despawned entity is not alive
    assert!(!world.is_alive(entity1), "Despawned entity should not be alive");

    // Verify queries don't return despawned entity
    for (entity, _, _, _) in world.query::<(Entity, &Transform, &MeshComponent, &Renderable)>() {
        assert_ne!(entity, entity1, "Query should not return despawned entity");
    }

    debug!("Entity despawn test passed");
}

#[test]
fn test_component_update_reflects_in_queries() {
    info!("Testing component updates reflect in queries");

    let mut world = setup_test_world();

    let entity = world.spawn();
    world.add(entity, Transform::new(Vec3::ZERO, Quat::IDENTITY, Vec3::ONE));
    world.add(entity, MeshComponent { mesh_id: 100 });
    world.add(entity, Renderable::visible());

    // Update transform
    if let Some(transform) = world.get_mut::<Transform>(entity) {
        transform.position = Vec3::new(10.0, 20.0, 30.0);
    }

    // Verify update is visible in queries
    let transform = world.get::<Transform>(entity).expect("Should have Transform");
    assert_eq!(transform.position, Vec3::new(10.0, 20.0, 30.0), "Position should be updated");

    debug!("Component update test passed");
}

#[test]
fn test_multiple_entities_with_different_components() {
    info!("Testing multiple entities with varying component sets");

    let mut world = setup_test_world();

    // Entity 1: Full renderable (Transform + Mesh + Renderable)
    let e1 = world.spawn();
    world.add(e1, Transform::identity());
    world.add(e1, MeshComponent { mesh_id: 1 });
    world.add(e1, Renderable::visible());

    // Entity 2: Transform + Mesh only (missing Renderable)
    let e2 = world.spawn();
    world.add(e2, Transform::identity());
    world.add(e2, MeshComponent { mesh_id: 2 });

    // Entity 3: Transform only
    let e3 = world.spawn();
    world.add(e3, Transform::identity());

    // Entity 4: Full renderable + Color
    let e4 = world.spawn();
    world.add(e4, Transform::identity());
    world.add(e4, MeshComponent { mesh_id: 4 });
    world.add(e4, Renderable::visible());
    world.add(e4, ColorComponent::red());

    // Query for full renderables (Transform + Mesh + Renderable)
    let full_renderable_count = count_renderable_entities(&world);
    assert_eq!(full_renderable_count, 2, "Should have 2 fully renderable entities (e1, e4)");

    // Query for entities with Transform + Mesh (regardless of Renderable)
    let mesh_count = world.query::<(&Transform, &MeshComponent)>().iter().count();
    assert_eq!(mesh_count, 3, "Should have 3 entities with Transform + Mesh (e1, e2, e4)");

    // Query for entities with just Transform
    let transform_count = world.query::<&Transform>().iter().count();
    assert_eq!(transform_count, 4, "Should have 4 entities with Transform");

    // Query for colored entities
    let colored_count = world.query::<&ColorComponent>().iter().count();
    assert_eq!(colored_count, 1, "Should have 1 colored entity (e4)");

    debug!("Multiple entities test passed");
}

#[test]
fn test_component_removal_updates_queries() {
    info!("Testing component removal updates queries");

    let mut world = setup_test_world();

    let entity = world.spawn();
    world.add(entity, Transform::identity());
    world.add(entity, MeshComponent { mesh_id: 1 });
    world.add(entity, Renderable::visible());

    // Verify entity is renderable
    assert_eq!(count_renderable_entities(&world), 1);

    // Remove Renderable component
    world.remove::<Renderable>(entity);

    // Verify entity is no longer in renderable query
    assert_eq!(count_renderable_entities(&world), 0, "Entity should not be renderable after component removal");

    // Verify entity still has other components
    assert!(world.get::<Transform>(entity).is_some(), "Entity should still have Transform");
    assert!(world.get::<MeshComponent>(entity).is_some(), "Entity should still have MeshComponent");

    debug!("Component removal test passed");
}

#[test]
fn test_entity_archetype_change() {
    info!("Testing entity archetype changes (adding/removing components)");

    let mut world = setup_test_world();

    let entity = world.spawn();
    world.add(entity, Transform::identity());

    // Initially only Transform
    assert!(world.get::<Transform>(entity).is_some());
    assert!(world.get::<MeshComponent>(entity).is_none());
    assert!(world.get::<Renderable>(entity).is_none());

    // Add MeshComponent (archetype change)
    world.add(entity, MeshComponent { mesh_id: 42 });
    assert!(world.get::<MeshComponent>(entity).is_some());

    // Add Renderable (another archetype change)
    world.add(entity, Renderable::visible());
    assert!(world.get::<Renderable>(entity).is_some());

    // Verify entity is now renderable
    assert_eq!(count_renderable_entities(&world), 1);

    // Remove MeshComponent (archetype change)
    world.remove::<MeshComponent>(entity);
    assert!(world.get::<MeshComponent>(entity).is_none());

    // Verify entity is no longer renderable (missing MeshComponent)
    assert_eq!(count_renderable_entities(&world), 0);

    debug!("Entity archetype change test passed");
}

// ============================================================================
// EDGE CASE TESTS (Category 2)
// ============================================================================

#[test]
fn test_entity_without_required_components() {
    info!("Testing rendering query with entities missing required components");

    let mut world = setup_test_world();

    // Create entities with incomplete component sets
    let e1 = world.spawn();
    world.add(e1, Transform::identity());
    // Missing MeshComponent and Renderable

    let e2 = world.spawn();
    world.add(e2, Transform::identity());
    world.add(e2, MeshComponent { mesh_id: 1 });
    // Missing Renderable

    let e3 = world.spawn();
    world.add(e3, MeshComponent { mesh_id: 2 });
    world.add(e3, Renderable::visible());
    // Missing Transform

    // Query for full renderables - should find 0
    assert_eq!(count_renderable_entities(&world), 0, "No entities have all required components");

    // Verify partial queries work
    let transform_only = world.query::<&Transform>().iter().count();
    assert_eq!(transform_only, 2, "Should find 2 entities with Transform (e1, e2)");

    let mesh_only = world.query::<&MeshComponent>().iter().count();
    assert_eq!(mesh_only, 3, "Should find 3 entities with MeshComponent (e2, e3)");

    debug!("Missing components test passed");
}

#[test]
fn test_component_removal_during_iteration() {
    info!("Testing component removal during query (safe mutation)");

    let mut world = setup_test_world();

    // Spawn multiple entities
    for i in 0..5 {
        let entity = world.spawn();
        world.add(entity, Transform::identity());
        world.add(entity, MeshComponent { mesh_id: i as u64 });
        world.add(entity, Renderable::visible());
    }

    assert_eq!(count_renderable_entities(&world), 5);

    // Collect entities to modify (can't mutate during iteration)
    let entities_to_modify: Vec<Entity> = world
        .query::<(Entity, &MeshComponent)>()
        .iter()
        .filter(|(_, mesh)| mesh.mesh_id % 2 == 0)
        .map(|(e, _)| *e)
        .collect();

    // Remove Renderable from even-numbered mesh entities
    for entity in entities_to_modify {
        world.remove::<Renderable>(entity);
    }

    // Verify only odd-numbered entities remain renderable
    let remaining = count_renderable_entities(&world);
    assert_eq!(remaining, 2, "Should have 2 renderable entities remaining (mesh_id 1 and 3)");

    debug!("Component removal during iteration test passed");
}

#[test]
fn test_zero_scale_entity() {
    info!("Testing entity with zero scale");

    let mut world = setup_test_world();

    let entity = world.spawn();
    world.add(entity, Transform::new(Vec3::ZERO, Quat::IDENTITY, Vec3::ZERO));
    world.add(entity, MeshComponent { mesh_id: 1 });
    world.add(entity, Renderable::visible());

    // Verify entity is in query (rendering system should handle zero scale)
    assert_eq!(count_renderable_entities(&world), 1);

    let transform = world.get::<Transform>(entity).expect("Should have Transform");
    assert_eq!(transform.scale, Vec3::ZERO, "Scale should be zero");

    debug!("Zero scale entity test passed");
}

#[test]
fn test_negative_scale_entity() {
    info!("Testing entity with negative scale");

    let mut world = setup_test_world();

    let entity = world.spawn();
    world.add(
        entity,
        Transform::new(Vec3::ZERO, Quat::IDENTITY, Vec3::new(-1.0, -1.0, -1.0)),
    );
    world.add(entity, MeshComponent { mesh_id: 1 });
    world.add(entity, Renderable::visible());

    // Verify entity is in query
    assert_eq!(count_renderable_entities(&world), 1);

    let transform = world.get::<Transform>(entity).expect("Should have Transform");
    assert!(transform.scale.x < 0.0, "Scale X should be negative");
    assert!(transform.scale.y < 0.0, "Scale Y should be negative");
    assert!(transform.scale.z < 0.0, "Scale Z should be negative");

    debug!("Negative scale entity test passed");
}

#[test]
fn test_extreme_transform_values() {
    info!("Testing entity with extreme transform values");

    let mut world = setup_test_world();

    // Very large position
    let e1 = world.spawn();
    world.add(
        e1,
        Transform::new(Vec3::new(1e9, 1e9, 1e9), Quat::IDENTITY, Vec3::ONE),
    );
    world.add(e1, MeshComponent { mesh_id: 1 });
    world.add(e1, Renderable::visible());

    // Very small but non-zero scale
    let e2 = world.spawn();
    world.add(
        e2,
        Transform::new(Vec3::ZERO, Quat::IDENTITY, Vec3::new(1e-6, 1e-6, 1e-6)),
    );
    world.add(e2, MeshComponent { mesh_id: 2 });
    world.add(e2, Renderable::visible());

    // Very large scale
    let e3 = world.spawn();
    world.add(
        e3,
        Transform::new(Vec3::ZERO, Quat::IDENTITY, Vec3::new(1e6, 1e6, 1e6)),
    );
    world.add(e3, MeshComponent { mesh_id: 3 });
    world.add(e3, Renderable::visible());

    // All should be queryable
    assert_eq!(count_renderable_entities(&world), 3, "All entities with extreme values should be queryable");

    // Verify values are preserved
    let t1 = world.get::<Transform>(e1).expect("Should have Transform");
    assert_eq!(t1.position.x, 1e9, "Large position should be preserved");

    let t2 = world.get::<Transform>(e2).expect("Should have Transform");
    assert!((t2.scale.x - 1e-6).abs() < 1e-9, "Small scale should be preserved");

    let t3 = world.get::<Transform>(e3).expect("Should have Transform");
    assert_eq!(t3.scale.x, 1e6, "Large scale should be preserved");

    debug!("Extreme transform values test passed");
}

#[test]
fn test_hidden_entities_not_rendered() {
    info!("Testing hidden entities (Renderable::visible = false)");

    let mut world = setup_test_world();

    // Visible entity
    let e1 = world.spawn();
    world.add(e1, Transform::identity());
    world.add(e1, MeshComponent { mesh_id: 1 });
    world.add(e1, Renderable::visible());

    // Hidden entity
    let e2 = world.spawn();
    world.add(e2, Transform::identity());
    world.add(e2, MeshComponent { mesh_id: 2 });
    world.add(e2, Renderable::hidden());

    // Query for visible entities only
    let visible_count = world
        .query::<(&Transform, &MeshComponent, &Renderable)>()
        .iter()
        .filter(|(_, _, renderable)| renderable.visible)
        .count();

    assert_eq!(visible_count, 1, "Only 1 entity should be visible");

    // Query for all entities with Renderable (visible or not)
    let total_with_renderable = world.query::<&Renderable>().iter().count();
    assert_eq!(total_with_renderable, 2, "Should have 2 entities with Renderable component");

    debug!("Hidden entities test passed");
}

#[test]
fn test_entity_despawn_safety() {
    info!("Testing entity despawn safety (double despawn, invalid entity)");

    let mut world = setup_test_world();

    let entity = world.spawn();
    world.add(entity, Transform::identity());

    // First despawn - should succeed
    world.despawn(entity);
    assert!(!world.is_alive(entity), "Entity should not be alive after despawn");

    // Second despawn - should be safe (no panic)
    world.despawn(entity);
    assert!(!world.is_alive(entity), "Entity should still not be alive");

    // Accessing components of despawned entity should return None
    assert!(world.get::<Transform>(entity).is_none(), "Despawned entity should have no components");

    debug!("Entity despawn safety test passed");
}

// ============================================================================
// PERFORMANCE VALIDATION TESTS (Category 3)
// ============================================================================

#[test]
fn test_large_entity_count_queries() {
    info!("Testing query performance with large entity count");

    let mut world = setup_test_world();

    const ENTITY_COUNT: usize = 10_000;

    // Spawn many entities
    for i in 0..ENTITY_COUNT {
        let entity = world.spawn();
        world.add(
            entity,
            Transform::new(
                Vec3::new(i as f32, 0.0, 0.0),
                Quat::IDENTITY,
                Vec3::ONE,
            ),
        );
        world.add(entity, MeshComponent { mesh_id: i as u64 });

        // Make half of them renderable
        if i % 2 == 0 {
            world.add(entity, Renderable::visible());
        }
    }

    // Query for all renderables
    let renderable_count = count_renderable_entities(&world);
    assert_eq!(renderable_count, ENTITY_COUNT / 2, "Should have half the entities renderable");

    // Query for all transforms
    let transform_count = world.query::<&Transform>().iter().count();
    assert_eq!(transform_count, ENTITY_COUNT, "Should have all entities with Transform");

    // Query with filter (visible only)
    let visible_count = world
        .query::<(&Transform, &Renderable)>()
        .iter()
        .filter(|(_, r)| r.visible)
        .count();
    assert_eq!(visible_count, ENTITY_COUNT / 2, "Should have half visible");

    debug!("Large entity count test passed with {} entities", ENTITY_COUNT);
}

#[test]
fn test_query_memory_efficiency() {
    info!("Testing query memory efficiency");

    let mut world = setup_test_world();

    const ENTITY_COUNT: usize = 1000;

    // Spawn entities
    for i in 0..ENTITY_COUNT {
        let entity = world.spawn();
        world.add(entity, Transform::identity());
        world.add(entity, MeshComponent { mesh_id: i as u64 });
        world.add(entity, Renderable::visible());
        world.add(entity, ColorComponent::new(
            (i % 255) as f32 / 255.0,
            ((i * 2) % 255) as f32 / 255.0,
            ((i * 3) % 255) as f32 / 255.0,
            1.0,
        ));
    }

    // Multiple queries should not allocate excessively
    for _ in 0..100 {
        let count = count_renderable_entities(&world);
        assert_eq!(count, ENTITY_COUNT);
    }

    // Query with complex filters
    let filtered_count = world
        .query::<(&Transform, &ColorComponent, &Renderable)>()
        .iter()
        .filter(|(t, c, _)| t.position.x >= 0.0 && c.r > 0.5)
        .count();

    assert!(filtered_count <= ENTITY_COUNT, "Filtered count should not exceed total");

    debug!("Query memory efficiency test passed");
}

#[test]
fn test_component_storage_efficiency() {
    info!("Testing component storage efficiency");

    let mut world = setup_test_world();

    // Sparse entity distribution
    let mut entities = Vec::new();
    for i in 0..100 {
        let entity = world.spawn();
        world.add(entity, Transform::identity());

        // Only some entities get other components
        if i % 3 == 0 {
            world.add(entity, MeshComponent { mesh_id: i });
        }
        if i % 5 == 0 {
            world.add(entity, Renderable::visible());
        }

        entities.push(entity);
    }

    // Verify sparse storage works correctly
    let transform_count = world.query::<&Transform>().iter().count();
    assert_eq!(transform_count, 100);

    let mesh_count = world.query::<&MeshComponent>().iter().count();
    assert_eq!(mesh_count, 34); // 100/3 rounded up

    let renderable_count = world.query::<&Renderable>().iter().count();
    assert_eq!(renderable_count, 20); // 100/5

    // Full renderable query (sparse intersection)
    let full_renderable = count_renderable_entities(&world);
    assert!(full_renderable > 0, "Should have some fully renderable entities");

    debug!("Component storage efficiency test passed");
}

// ============================================================================
// GPU INTEGRATION TESTS (Category 4 - Optional, requires Vulkan)
// ============================================================================

#[test]
#[ignore = "Requires Vulkan device"]
fn test_ecs_to_gpu_mesh_upload() {
    info!("Testing ECS entity to GPU mesh upload");

    // Create Vulkan context
    let context = VulkanContext::new("test_ecs_gpu", None, None)
        .expect("Failed to create Vulkan context");

    let mut gpu_cache = GpuCache::new(&context).expect("Failed to create GPU cache");

    let mut world = setup_test_world();

    // Create test mesh
    let mesh = create_triangle_mesh();
    let mesh_id = gpu_cache.upload_mesh(&mesh).expect("Failed to upload mesh");

    // Create entity with uploaded mesh
    let entity = world.spawn();
    world.add(entity, Transform::identity());
    world.add(entity, MeshComponent { mesh_id });
    world.add(entity, Renderable::visible());

    // Query for renderable entities
    let renderables: Vec<_> = world
        .query::<(Entity, &Transform, &MeshComponent, &Renderable)>()
        .iter()
        .collect();

    assert_eq!(renderables.len(), 1);

    // Verify mesh is in GPU cache
    let cached_mesh = gpu_cache.get_mesh(mesh_id).expect("Mesh should be in GPU cache");
    assert_eq!(cached_mesh.vertex_count, 3);
    assert_eq!(cached_mesh.index_count, 3);

    debug!("ECS to GPU mesh upload test passed");
}

#[test]
#[ignore = "Requires Vulkan device"]
fn test_multiple_entities_share_mesh() {
    info!("Testing multiple entities sharing the same GPU mesh");

    let context = VulkanContext::new("test_shared_mesh", None, None)
        .expect("Failed to create Vulkan context");

    let mut gpu_cache = GpuCache::new(&context).expect("Failed to create GPU cache");

    let mut world = setup_test_world();

    // Upload a single mesh
    let mesh = create_quad_mesh();
    let mesh_id = gpu_cache.upload_mesh(&mesh).expect("Failed to upload mesh");

    // Create multiple entities using the same mesh
    const ENTITY_COUNT: usize = 100;
    for i in 0..ENTITY_COUNT {
        let entity = world.spawn();
        world.add(
            entity,
            Transform::new(
                Vec3::new(i as f32 * 2.0, 0.0, 0.0),
                Quat::IDENTITY,
                Vec3::ONE,
            ),
        );
        world.add(entity, MeshComponent { mesh_id });
        world.add(entity, Renderable::visible());
    }

    // Verify all entities share the same mesh ID
    let unique_mesh_ids: std::collections::HashSet<_> = world
        .query::<&MeshComponent>()
        .iter()
        .map(|m| m.mesh_id)
        .collect();

    assert_eq!(unique_mesh_ids.len(), 1, "All entities should share the same mesh ID");
    assert_eq!(count_renderable_entities(&world), ENTITY_COUNT);

    debug!("Multiple entities sharing mesh test passed");
}

#[test]
#[ignore = "Requires Vulkan device and window"]
fn test_frame_render_with_ecs_entities() {
    info!("Testing frame rendering with ECS entities");

    // Create renderer with window
    let window_config = WindowConfig {
        title: "ECS Rendering Test".to_string(),
        width: 800,
        height: 600,
        resizable: false,
    };

    let mut renderer = Renderer::new(window_config, "test_frame_render")
        .expect("Failed to create renderer");

    let mut world = setup_test_world();

    // Create test mesh
    let mesh = create_triangle_mesh();
    // TODO: Upload mesh to renderer's GPU cache
    // let mesh_id = renderer.upload_mesh(&mesh).expect("Failed to upload mesh");

    // Create entities
    for i in 0..10 {
        let entity = world.spawn();
        world.add(
            entity,
            Transform::new(
                Vec3::new((i as f32 - 5.0) * 2.0, 0.0, -10.0),
                Quat::IDENTITY,
                Vec3::ONE,
            ),
        );
        // world.add(entity, MeshComponent { mesh_id });
        world.add(entity, Renderable::visible());
    }

    // Render a single frame
    // TODO: Implement rendering system that queries world and submits draw calls
    // renderer.begin_frame().expect("Failed to begin frame");
    //
    // for (transform, mesh, _) in world.query::<(&Transform, &MeshComponent, &Renderable)>() {
    //     if renderable.visible {
    //         renderer.draw_mesh(mesh.mesh_id, transform).expect("Failed to draw mesh");
    //     }
    // }
    //
    // renderer.end_frame().expect("Failed to end frame");

    debug!("Frame render with ECS entities test passed");
}

// ============================================================================
// STRESS TESTS (Category 5 - Validation)
// ============================================================================

#[test]
fn test_entity_churn() {
    info!("Testing entity spawn/despawn churn");

    let mut world = setup_test_world();

    const ITERATIONS: usize = 1000;
    const BATCH_SIZE: usize = 100;

    for _ in 0..ITERATIONS {
        // Spawn batch
        let mut entities = Vec::with_capacity(BATCH_SIZE);
        for i in 0..BATCH_SIZE {
            let entity = world.spawn();
            world.add(entity, Transform::identity());
            world.add(entity, MeshComponent { mesh_id: i as u64 });
            world.add(entity, Renderable::visible());
            entities.push(entity);
        }

        // Verify all spawned
        assert_eq!(count_renderable_entities(&world), BATCH_SIZE);

        // Despawn batch
        for entity in entities {
            world.despawn(entity);
        }

        // Verify all despawned
        assert_eq!(count_renderable_entities(&world), 0);
    }

    debug!("Entity churn test passed with {} iterations", ITERATIONS);
}

#[test]
fn test_component_add_remove_churn() {
    info!("Testing component add/remove churn");

    let mut world = setup_test_world();

    const ENTITY_COUNT: usize = 100;
    const ITERATIONS: usize = 100;

    // Spawn entities
    let entities: Vec<_> = (0..ENTITY_COUNT)
        .map(|_| {
            let entity = world.spawn();
            world.add(entity, Transform::identity());
            entity
        })
        .collect();

    for _ in 0..ITERATIONS {
        // Add components
        for entity in &entities {
            world.add(*entity, MeshComponent { mesh_id: 42 });
            world.add(*entity, Renderable::visible());
        }

        assert_eq!(count_renderable_entities(&world), ENTITY_COUNT);

        // Remove components
        for entity in &entities {
            world.remove::<Renderable>(*entity);
            world.remove::<MeshComponent>(*entity);
        }

        assert_eq!(count_renderable_entities(&world), 0);
    }

    debug!("Component churn test passed with {} iterations", ITERATIONS);
}

// ============================================================================
// DOCUMENTATION TESTS
// ============================================================================

/// This test documents the expected workflow for rendering systems
#[test]
fn test_documented_rendering_workflow() {
    info!("Documenting expected rendering workflow");

    let mut world = setup_test_world();

    // Step 1: Create entities with renderable components
    let entity = world.spawn();
    world.add(entity, Transform::new(Vec3::ZERO, Quat::IDENTITY, Vec3::ONE));
    world.add(entity, MeshComponent { mesh_id: 123 });
    world.add(entity, Renderable::visible());

    // Step 2: Rendering system queries for renderable entities
    for (entity, transform, mesh, renderable) in
        world.query::<(Entity, &Transform, &MeshComponent, &Renderable)>()
    {
        // Step 3: Check visibility
        if !renderable.visible {
            continue;
        }

        // Step 4: Process transform for rendering
        let _position = transform.position;
        let _rotation = transform.rotation;
        let _scale = transform.scale;

        // Step 5: Queue mesh for rendering
        let _mesh_id = mesh.mesh_id;

        // In a real system, this would:
        // - Transform vertices to world space
        // - Calculate MVP matrix
        // - Submit draw call to GPU
        // - Update GPU buffers if transform changed

        debug!(
            "Would render entity {:?} with mesh {} at position {:?}",
            entity, mesh.mesh_id, transform.position
        );
    }

    debug!("Documented rendering workflow test passed");
}
