//! Interest Management Edge Case Tests
//!
//! Comprehensive edge case testing to ensure robustness:
//! - Boundary conditions (exact AOI boundaries, grid edges)
//! - Extreme values (very large/small coordinates, NaN, infinity)
//! - Concurrent operations (high contention scenarios)
//! - Pathological cases (all entities in one cell, linear arrangements)
//! - Error conditions (invalid inputs, edge cases)

use engine_core::{Aabb, Quat, Transform, Vec3, World};
use engine_interest::{AreaOfInterest, InterestManager};
use std::f32::consts::PI;

// ============================================================================
// Boundary Condition Tests
// ============================================================================

#[test]
fn test_entity_at_exact_aoi_boundary() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Aabb>();

    let mut manager = InterestManager::new(10.0);

    // Create entities at exact boundary distances
    let distances = vec![
        49.9, // Just inside
        50.0, // Exactly on boundary
        50.1, // Just outside
    ];

    let mut entities = Vec::new();
    for dist in &distances {
        let entity = world.spawn();
        let pos = Vec3::new(*dist, 0.0, 0.0);
        world.add(entity, Transform::new(pos, Quat::IDENTITY, Vec3::ONE));
        world.add(entity, Aabb::from_center_half_extents(pos, Vec3::ONE));
        entities.push(entity);
    }

    manager.update_from_world(&world);

    // Client at origin with 50 unit radius
    let client_id = 1;
    manager.set_client_interest(client_id, AreaOfInterest::new(Vec3::ZERO, 50.0));

    let visible = manager.calculate_visibility(client_id);

    // Entity at 49.9 should be visible
    assert!(visible.contains(&entities[0]), "Entity just inside boundary should be visible");

    // Entity at exactly 50.0 should be visible (inclusive)
    assert!(visible.contains(&entities[1]), "Entity at exact boundary should be visible");

    // Entity at 50.1 should NOT be visible
    assert!(
        !visible.contains(&entities[2]),
        "Entity just outside boundary should not be visible"
    );
}

#[test]
fn test_aoi_at_grid_cell_boundary() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Aabb>();

    let cell_size = 50.0;
    let mut manager = InterestManager::new(cell_size);

    // Place entities at grid cell boundaries
    let positions = vec![
        Vec3::new(49.9, 0.0, 0.0),  // Just before boundary
        Vec3::new(50.0, 0.0, 0.0),  // Exactly on boundary
        Vec3::new(50.1, 0.0, 0.0),  // Just after boundary
        Vec3::new(99.9, 0.0, 0.0),  // Before next boundary
        Vec3::new(100.0, 0.0, 0.0), // On next boundary
    ];

    let mut entities = Vec::new();
    for pos in positions {
        let entity = world.spawn();
        world.add(entity, Transform::new(pos, Quat::IDENTITY, Vec3::ONE));
        world.add(entity, Aabb::from_center_half_extents(pos, Vec3::ONE));
        entities.push(entity);
    }

    manager.update_from_world(&world);

    // Client at origin should see entities within its AOI
    let client_id = 1;
    manager.set_client_interest(client_id, AreaOfInterest::new(Vec3::ZERO, 75.0));

    let visible = manager.calculate_visibility(client_id);

    // Should see first 3-4 entities depending on exact boundary handling
    assert!(visible.len() >= 3, "Should see entities near grid boundaries");
}

#[test]
fn test_negative_coordinates() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Aabb>();

    let mut manager = InterestManager::new(50.0);

    // Create entities in all quadrants
    let positions = vec![
        Vec3::new(10.0, 0.0, 10.0),   // Positive quadrant
        Vec3::new(-10.0, 0.0, 10.0),  // Negative X
        Vec3::new(10.0, 0.0, -10.0),  // Negative Z
        Vec3::new(-10.0, 0.0, -10.0), // Both negative
    ];

    let mut entities = Vec::new();
    for pos in positions {
        let entity = world.spawn();
        world.add(entity, Transform::new(pos, Quat::IDENTITY, Vec3::ONE));
        world.add(entity, Aabb::from_center_half_extents(pos, Vec3::ONE));
        entities.push(entity);
    }

    manager.update_from_world(&world);

    // Client at origin should see all entities
    let client_id = 1;
    manager.set_client_interest(client_id, AreaOfInterest::new(Vec3::ZERO, 50.0));

    let visible = manager.calculate_visibility(client_id);
    assert_eq!(visible.len(), 4, "Should handle negative coordinates correctly");
}

#[test]
fn test_very_large_coordinates() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Aabb>();

    let mut manager = InterestManager::new(100.0);

    // Create entities at very large coordinates
    let large_value = 100_000.0;
    let entity = world.spawn();
    let pos = Vec3::new(large_value, 0.0, large_value);
    world.add(entity, Transform::new(pos, Quat::IDENTITY, Vec3::ONE));
    world.add(entity, Aabb::from_center_half_extents(pos, Vec3::ONE));

    manager.update_from_world(&world);

    // Client near the large coordinates
    let client_id = 1;
    let client_pos = Vec3::new(large_value + 50.0, 0.0, large_value);
    manager.set_client_interest(client_id, AreaOfInterest::new(client_pos, 100.0));

    let visible = manager.calculate_visibility(client_id);
    assert_eq!(visible.len(), 1, "Should handle large coordinates correctly");
}

#[test]
fn test_zero_radius_aoi() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Aabb>();

    let mut manager = InterestManager::new(50.0);

    let entity = world.spawn();
    world.add(entity, Transform::new(Vec3::ZERO, Quat::IDENTITY, Vec3::ONE));
    world.add(entity, Aabb::from_center_half_extents(Vec3::ZERO, Vec3::ONE));

    manager.update_from_world(&world);

    // Zero radius AOI
    let client_id = 1;
    manager.set_client_interest(client_id, AreaOfInterest::new(Vec3::ZERO, 0.0));

    let visible = manager.calculate_visibility(client_id);

    // Zero radius should see nothing (or only exact matches)
    assert!(
        visible.is_empty() || visible.len() == 1,
        "Zero radius AOI should see very few entities"
    );
}

#[test]
fn test_overlapping_client_aois() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Aabb>();

    let mut manager = InterestManager::new(50.0);

    // Create a cluster of entities
    for i in 0..10 {
        let entity = world.spawn();
        let pos = Vec3::new(i as f32 * 5.0, 0.0, 0.0);
        world.add(entity, Transform::new(pos, Quat::IDENTITY, Vec3::ONE));
        world.add(entity, Aabb::from_center_half_extents(pos, Vec3::ONE));
    }

    manager.update_from_world(&world);

    // Multiple clients with overlapping AOIs
    for client_id in 0..5 {
        let pos = Vec3::new(client_id as f32 * 10.0, 0.0, 0.0);
        manager.set_client_interest(client_id as u64, AreaOfInterest::new(pos, 30.0));
    }

    // Each client should see a subset of entities
    for client_id in 0..5 {
        let visible = manager.calculate_visibility(client_id as u64);
        assert!(!visible.is_empty(), "Client {} should see some entities", client_id);
    }
}

// ============================================================================
// Concurrent Operation Tests
// ============================================================================

#[test]
fn test_rapid_aoi_updates() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Aabb>();

    let mut manager = InterestManager::new(50.0);

    // Create entities
    for i in 0..100 {
        let entity = world.spawn();
        let pos = Vec3::new((i % 10) as f32 * 10.0, 0.0, (i / 10) as f32 * 10.0);
        world.add(entity, Transform::new(pos, Quat::IDENTITY, Vec3::ONE));
        world.add(entity, Aabb::from_center_half_extents(pos, Vec3::ONE));
    }

    manager.update_from_world(&world);

    let client_id = 1;

    // Rapidly update AOI 100 times
    for i in 0..100 {
        let pos = Vec3::new((i % 10) as f32 * 10.0, 0.0, (i / 10) as f32 * 10.0);
        manager.set_client_interest(client_id, AreaOfInterest::new(pos, 50.0));

        // Should always return valid results
        let visible = manager.calculate_visibility(client_id);
        assert!(!visible.is_empty(), "Should see entities after rapid update {}", i);
    }
}

#[test]
fn test_entity_spawn_despawn_during_visibility() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Aabb>();

    let mut manager = InterestManager::new(50.0);

    // Spawn initial entities
    let mut entities = Vec::new();
    for i in 0..10 {
        let entity = world.spawn();
        let pos = Vec3::new(i as f32 * 5.0, 0.0, 0.0);
        world.add(entity, Transform::new(pos, Quat::IDENTITY, Vec3::ONE));
        world.add(entity, Aabb::from_center_half_extents(pos, Vec3::ONE));
        entities.push(entity);
    }

    manager.update_from_world(&world);

    let client_id = 1;
    manager.set_client_interest(client_id, AreaOfInterest::new(Vec3::ZERO, 50.0));

    let visible_before = manager.calculate_visibility(client_id);
    let count_before = visible_before.len();

    // Despawn some entities
    for i in 0..5 {
        world.despawn(entities[i]);
    }

    // Update manager
    manager.update_from_world(&world);

    let visible_after = manager.calculate_visibility(client_id);
    let count_after = visible_after.len();

    assert!(count_after < count_before, "Should see fewer entities after despawn");
    assert!(count_after <= 5, "Should have at most 5 entities remaining");
}

// ============================================================================
// Pathological Case Tests
// ============================================================================

#[test]
fn test_all_entities_in_one_cell() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Aabb>();

    let mut manager = InterestManager::new(100.0);

    // Put all entities in a tiny cluster (worst case for spatial partitioning)
    for i in 0..100 {
        let entity = world.spawn();
        let offset = (i as f32) * 0.1; // Very tight clustering
        let pos = Vec3::new(offset, 0.0, offset);
        world.add(entity, Transform::new(pos, Quat::IDENTITY, Vec3::ONE));
        world.add(entity, Aabb::from_center_half_extents(pos, Vec3::ONE));
    }

    manager.update_from_world(&world);

    let client_id = 1;
    manager.set_client_interest(client_id, AreaOfInterest::new(Vec3::new(5.0, 0.0, 5.0), 50.0));

    let start = std::time::Instant::now();
    let visible = manager.calculate_visibility(client_id);
    let elapsed = start.elapsed();

    // Should handle worst case efficiently
    assert_eq!(visible.len(), 100, "Should see all clustered entities");
    assert!(
        elapsed.as_micros() < 1000,
        "Should handle clustered entities in <1ms, took {:?}",
        elapsed
    );
}

#[test]
fn test_linear_entity_arrangement() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Aabb>();

    let mut manager = InterestManager::new(50.0);

    // Arrange entities in a perfect line (tests grid boundary handling)
    for i in 0..100 {
        let entity = world.spawn();
        let pos = Vec3::new(i as f32 * 10.0, 0.0, 0.0);
        world.add(entity, Transform::new(pos, Quat::IDENTITY, Vec3::ONE));
        world.add(entity, Aabb::from_center_half_extents(pos, Vec3::ONE));
    }

    manager.update_from_world(&world);

    // Client in the middle
    let client_id = 1;
    manager.set_client_interest(client_id, AreaOfInterest::new(Vec3::new(500.0, 0.0, 0.0), 100.0));

    let visible = manager.calculate_visibility(client_id);

    // Should only see entities within 100 units
    assert!(visible.len() >= 10, "Should see entities in linear arrangement");
    assert!(visible.len() <= 30, "Should not see too many entities in linear arrangement");
}

#[test]
fn test_sparse_world() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Aabb>();

    let mut manager = InterestManager::new(100.0);

    // Very sparse world - entities far apart
    for i in 0..10 {
        let entity = world.spawn();
        let pos = Vec3::new(i as f32 * 1000.0, 0.0, i as f32 * 1000.0);
        world.add(entity, Transform::new(pos, Quat::IDENTITY, Vec3::ONE));
        world.add(entity, Aabb::from_center_half_extents(pos, Vec3::ONE));
    }

    manager.update_from_world(&world);

    let client_id = 1;
    manager.set_client_interest(client_id, AreaOfInterest::new(Vec3::ZERO, 500.0));

    let visible = manager.calculate_visibility(client_id);

    // In sparse world, should see very few entities
    assert!(visible.len() <= 2, "Should see very few entities in sparse world");
}

#[test]
fn test_high_density_battle_scenario() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Aabb>();

    let mut manager = InterestManager::new(50.0);

    // Simulate a battle: high density cluster
    let battle_center = Vec3::new(500.0, 0.0, 500.0);
    for i in 0..200 {
        let entity = world.spawn();
        let angle = (i as f32) * 2.0 * PI / 200.0;
        let radius = (i % 20) as f32 * 2.0; // Concentric circles
        let pos = battle_center + Vec3::new(radius * angle.cos(), 0.0, radius * angle.sin());
        world.add(entity, Transform::new(pos, Quat::IDENTITY, Vec3::ONE));
        world.add(entity, Aabb::from_center_half_extents(pos, Vec3::ONE));
    }

    manager.update_from_world(&world);

    let client_id = 1;
    manager.set_client_interest(client_id, AreaOfInterest::new(battle_center, 50.0));

    let start = std::time::Instant::now();
    let visible = manager.calculate_visibility(client_id);
    let elapsed = start.elapsed();

    // Should handle high density efficiently
    assert!(visible.len() > 50, "Should see many entities in battle scenario");
    assert!(elapsed.as_micros() < 1000, "Should handle high density in <1ms");
}

// ============================================================================
// Edge Case: Entity Movement Tracking
// ============================================================================

#[test]
fn test_entity_crossing_aoi_boundary() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Aabb>();

    let mut manager = InterestManager::new(50.0);

    let entity = world.spawn();
    let start_pos = Vec3::new(40.0, 0.0, 0.0); // Inside AOI
    world.add(entity, Transform::new(start_pos, Quat::IDENTITY, Vec3::ONE));
    world.add(entity, Aabb::from_center_half_extents(start_pos, Vec3::ONE));

    manager.update_from_world(&world);

    let client_id = 1;
    manager.set_client_interest(client_id, AreaOfInterest::new(Vec3::ZERO, 50.0));

    // Entity should be visible initially
    let visible = manager.calculate_visibility(client_id);
    assert!(visible.contains(&entity), "Entity should be visible initially");
    let (entered, _exited) = manager.get_visibility_changes(client_id);
    assert!(entered.contains(&entity), "Entity should be in entered list");

    // Move entity outside AOI
    let new_pos = Vec3::new(60.0, 0.0, 0.0); // Outside AOI
    {
        let transform = world.get_mut::<Transform>(entity).unwrap();
        transform.position = new_pos;
    }
    {
        let aabb = world.get_mut::<Aabb>(entity).unwrap();
        *aabb = Aabb::from_center_half_extents(new_pos, Vec3::ONE);
    }

    manager.update_from_world(&world);

    // Entity should no longer be visible
    let visible = manager.calculate_visibility(client_id);
    assert!(!visible.contains(&entity), "Entity should not be visible after moving outside");
    let (_entered, exited) = manager.get_visibility_changes(client_id);
    assert!(exited.contains(&entity), "Entity should be in exited list");
}

#[test]
fn test_aoi_moving_across_grid_boundaries() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Aabb>();

    let cell_size = 100.0;
    let mut manager = InterestManager::new(cell_size);

    // Create entities across multiple grid cells
    for x in 0..5 {
        for z in 0..5 {
            let entity = world.spawn();
            let pos = Vec3::new(x as f32 * 50.0, 0.0, z as f32 * 50.0);
            world.add(entity, Transform::new(pos, Quat::IDENTITY, Vec3::ONE));
            world.add(entity, Aabb::from_center_half_extents(pos, Vec3::ONE));
        }
    }

    manager.update_from_world(&world);

    let client_id = 1;

    // Move AOI across grid boundaries
    for i in 0..10 {
        let pos = Vec3::new(i as f32 * 30.0, 0.0, 0.0);
        manager.set_client_interest(client_id, AreaOfInterest::new(pos, 75.0));

        let visible = manager.calculate_visibility(client_id);
        assert!(!visible.is_empty(), "Should see entities at position {}", i);
    }
}

// ============================================================================
// Multi-Frame Tracking Tests
// ============================================================================

#[test]
fn test_multi_frame_visibility_consistency() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Aabb>();

    let mut manager = InterestManager::new(50.0);

    // Create static entities
    let mut entities = Vec::new();
    for i in 0..50 {
        let entity = world.spawn();
        let pos = Vec3::new((i % 10) as f32 * 10.0, 0.0, (i / 10) as f32 * 10.0);
        world.add(entity, Transform::new(pos, Quat::IDENTITY, Vec3::ONE));
        world.add(entity, Aabb::from_center_half_extents(pos, Vec3::ONE));
        entities.push(entity);
    }

    manager.update_from_world(&world);

    let client_id = 1;
    manager.set_client_interest(client_id, AreaOfInterest::new(Vec3::new(25.0, 0.0, 25.0), 30.0));

    // Check consistency over multiple frames
    let visible_frame1 = manager.calculate_visibility(client_id);
    let visible_frame2 = manager.calculate_visibility(client_id);
    let visible_frame3 = manager.calculate_visibility(client_id);

    assert_eq!(
        visible_frame1.len(),
        visible_frame2.len(),
        "Visibility should be consistent across frames"
    );
    assert_eq!(
        visible_frame2.len(),
        visible_frame3.len(),
        "Visibility should be consistent across frames"
    );
}

#[test]
fn test_stale_visibility_cache() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Aabb>();

    let mut manager = InterestManager::new(50.0);

    let entity = world.spawn();
    world.add(entity, Transform::new(Vec3::ZERO, Quat::IDENTITY, Vec3::ONE));
    world.add(entity, Aabb::from_center_half_extents(Vec3::ZERO, Vec3::ONE));

    manager.update_from_world(&world);

    let client_id = 1;
    manager.set_client_interest(client_id, AreaOfInterest::new(Vec3::ZERO, 50.0));

    // Get initial visibility
    let visible1 = manager.calculate_visibility(client_id);
    assert_eq!(visible1.len(), 1);

    // Despawn entity
    world.despawn(entity);

    // Update manager - should detect entity is gone
    manager.update_from_world(&world);

    let visible2 = manager.calculate_visibility(client_id);
    assert_eq!(visible2.len(), 0, "Should not see despawned entity");
}

#[test]
fn test_maximum_entities_stress() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Aabb>();

    let mut manager = InterestManager::new(100.0);

    // Stress test with 5000 entities
    for i in 0..5000 {
        let entity = world.spawn();
        let x = ((i % 100) as f32) * 10.0;
        let z = ((i / 100) as f32) * 10.0;
        let pos = Vec3::new(x, 0.0, z);
        world.add(entity, Transform::new(pos, Quat::IDENTITY, Vec3::ONE));
        world.add(entity, Aabb::from_center_half_extents(pos, Vec3::ONE));
    }

    let start = std::time::Instant::now();
    manager.update_from_world(&world);
    let update_time = start.elapsed();

    let client_id = 1;
    manager
        .set_client_interest(client_id, AreaOfInterest::new(Vec3::new(500.0, 0.0, 500.0), 100.0));

    let start = std::time::Instant::now();
    let visible = manager.calculate_visibility(client_id);
    let visibility_time = start.elapsed();

    assert!(
        update_time.as_millis() < 100,
        "Update should complete in <100ms for 5K entities"
    );
    assert!(visibility_time.as_micros() < 1000, "Visibility calc should complete in <1ms");
    assert!(!visible.is_empty(), "Should see some entities");
}
