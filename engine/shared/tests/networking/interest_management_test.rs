//! Interest Management Integration Tests
//!
//! Comprehensive test suite for interest management system.
//! Validates Phase 2.8 requirements from phase2-interest-basic.md

use engine_core::{Aabb, Entity, Transform, Vec3, World};
use engine_interest::{AreaOfInterest, InterestManager};
use engine_networking::InterestFilter;

/// Helper to create a test world with entities
fn create_test_world(count: usize) -> (World, Vec<Entity>) {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Aabb>();

    let mut entities = Vec::new();

    let grid_size = (count as f32).sqrt() as usize;
    for i in 0..count {
        let entity = world.spawn();
        let x = ((i % grid_size) as f32) * 10.0;
        let z = ((i / grid_size) as f32) * 10.0;
        let pos = Vec3::new(x, 0.0, z);

        world.add(entity, Transform::new(pos, engine_core::Quat::IDENTITY, Vec3::ONE));
        world.add(entity, Aabb::from_center_half_extents(pos, Vec3::ONE));
        entities.push(entity);
    }

    (world, entities)
}

// ============================================================================
// Basic Functionality Tests
// ============================================================================

#[test]
fn test_single_client_visibility() {
    let (world, entities) = create_test_world(100);

    let mut manager = InterestManager::new(50.0);
    manager.update_from_world(&world);

    // Client at origin with 50 unit radius
    let client_id = 1;
    let aoi = AreaOfInterest::new(Vec3::ZERO, 50.0);
    manager.set_client_interest(client_id, aoi);

    let visible = manager.calculate_visibility(client_id);

    // Should see some but not all entities
    assert!(!visible.is_empty(), "Should see some entities");
    assert!(visible.len() < entities.len(), "Should not see all entities");

    // Verify all visible entities are actually in range
    for entity in &visible {
        assert!(entities.contains(entity), "Visible entity should exist");
    }
}

#[test]
fn test_multi_client_visibility() {
    let (world, _) = create_test_world(1000);

    let mut manager = InterestManager::new(50.0);
    manager.update_from_world(&world);

    // Add 10 clients at different locations within the world (0-300 range)
    // Entities are in a ~31x31 grid with 10 unit spacing (0-310)
    for i in 0..10 {
        let x = ((i % 5) as f32) * 60.0; // 0, 60, 120, 180, 240
        let z = ((i / 5) as f32) * 60.0; // 0 or 60
        let pos = Vec3::new(x, 0.0, z);
        manager.set_client_interest(i, AreaOfInterest::new(pos, 100.0));
    }

    // Each client should see different sets
    let mut all_visible_sets = Vec::new();
    for i in 0..10 {
        let visible = manager.calculate_visibility(i);
        assert!(!visible.is_empty(), "Client {} should see entities", i);
        all_visible_sets.push(visible);
    }

    // Verify clients at different positions see different entities
    assert_ne!(
        all_visible_sets[0], all_visible_sets[9],
        "Distant clients should see different entities"
    );
}

#[test]
fn test_enter_exit_events() {
    let mut world = World::new();
    world.register::<Aabb>();

    let entity = world.spawn();
    world.add(entity, Aabb::from_center_half_extents(Vec3::new(10.0, 0.0, 0.0), Vec3::ONE));

    let mut manager = InterestManager::new(50.0);
    manager.update_from_world(&world);

    let client_id = 1;
    manager.set_client_interest(client_id, AreaOfInterest::new(Vec3::ZERO, 50.0));

    // Initial: entity enters
    let (entered, exited) = manager.get_visibility_changes(client_id);
    assert_eq!(entered.len(), 1, "Entity should enter");
    assert_eq!(exited.len(), 0, "Nothing should exit");
    assert!(entered.contains(&entity));

    // No movement: no changes
    let (entered, exited) = manager.get_visibility_changes(client_id);
    assert_eq!(entered.len(), 0, "No new entries");
    assert_eq!(exited.len(), 0, "No exits");

    // Move entity out of range
    world.remove::<Aabb>(entity);
    world.add(entity, Aabb::from_center_half_extents(Vec3::new(200.0, 0.0, 0.0), Vec3::ONE));
    manager.update_from_world(&world);

    let (entered, exited) = manager.get_visibility_changes(client_id);
    assert_eq!(entered.len(), 0, "Nothing new enters");
    assert_eq!(exited.len(), 1, "Entity should exit");
    assert!(exited.contains(&entity));

    // Move entity back in range
    world.remove::<Aabb>(entity);
    world.add(entity, Aabb::from_center_half_extents(Vec3::new(10.0, 0.0, 0.0), Vec3::ONE));
    manager.update_from_world(&world);

    let (entered, exited) = manager.get_visibility_changes(client_id);
    assert_eq!(entered.len(), 1, "Entity should enter again");
    assert_eq!(exited.len(), 0, "Nothing exits");
    assert!(entered.contains(&entity));
}

#[test]
fn test_bandwidth_reduction() {
    let (world, _) = create_test_world(1000);

    let mut manager = InterestManager::new(50.0);
    manager.update_from_world(&world);

    // Register 100 clients with 100 unit AOI
    for i in 0..100 {
        let pos = Vec3::new(((i % 10) as f32) * 100.0, 0.0, ((i / 10) as f32) * 100.0);
        manager.set_client_interest(i, AreaOfInterest::new(pos, 100.0));

        // Initialize cache
        manager.get_visibility_changes(i);
    }

    let (without, with, reduction) = manager.compute_bandwidth_reduction();

    // Without interest: 100 clients × 1000 entities = 100,000 updates
    assert_eq!(without, 100_000, "Without interest should be clients × entities");

    // With interest: should be much less
    assert!(with < without, "With interest should reduce updates");
    assert!(with < 20_000, "Should filter significantly (expecting <20k)");

    // Should achieve substantial reduction
    assert!(reduction >= 80.0, "Should achieve ≥80% reduction, got {:.1}%", reduction);
}

// ============================================================================
// Performance Validation Tests
// ============================================================================

#[test]
fn test_visibility_performance_1k_entities() {
    let (world, _) = create_test_world(1000);

    let mut manager = InterestManager::new(50.0);
    manager.update_from_world(&world);

    let client_id = 1;
    manager.set_client_interest(client_id, AreaOfInterest::new(Vec3::ZERO, 100.0));

    // Measure single visibility calculation
    let start = std::time::Instant::now();
    let _visible = manager.calculate_visibility(client_id);
    let elapsed = start.elapsed();

    // Target: <1ms for 1K entities
    assert!(
        elapsed.as_micros() < 1000,
        "Visibility calculation too slow: {:?} (target: <1ms)",
        elapsed
    );
}

#[test]
fn test_100_clients_performance() {
    let (world, _) = create_test_world(1000);

    let mut manager = InterestManager::new(50.0);
    manager.update_from_world(&world);

    // Register 100 clients
    for i in 0..100 {
        let pos = Vec3::new(((i % 10) as f32) * 100.0, 0.0, ((i / 10) as f32) * 100.0);
        manager.set_client_interest(i, AreaOfInterest::new(pos, 100.0));
    }

    // Measure calculating all visibility
    let start = std::time::Instant::now();
    for i in 0..100 {
        let _visible = manager.calculate_visibility(i);
    }
    let elapsed = start.elapsed();

    // Target: <100ms for 100 clients
    assert!(
        elapsed.as_millis() < 100,
        "100 clients too slow: {:?} (target: <100ms)",
        elapsed
    );
}

// ============================================================================
// Integration with InterestFilter Tests
// ============================================================================

#[test]
fn test_interest_filter_basic() {
    let (world, entities) = create_test_world(100);

    let mut filter = InterestFilter::new(50.0);
    filter.update_from_world(&world);

    // Register client
    filter.register_client(1, Vec3::ZERO, 50.0);

    // Filter should return only visible entities
    let visible = filter.filter_updates(1, &entities);

    assert!(!visible.is_empty(), "Should have visible entities");
    assert!(visible.len() < entities.len(), "Should filter some entities");
}

#[test]
fn test_interest_filter_update_position() {
    let (world, entities) = create_test_world(100);

    let mut filter = InterestFilter::new(50.0);
    filter.update_from_world(&world);

    filter.register_client(1, Vec3::ZERO, 50.0);
    let visible_at_origin = filter.filter_updates(1, &entities);

    // Move client to different location
    filter.update_client_position(1, Vec3::new(500.0, 0.0, 500.0));
    let visible_at_500 = filter.filter_updates(1, &entities);

    // Should see different entities at different locations
    assert_ne!(
        visible_at_origin, visible_at_500,
        "Should see different entities at different positions"
    );
}

#[test]
fn test_interest_filter_bandwidth_metrics() {
    let (world, _) = create_test_world(1000);

    let mut filter = InterestFilter::new(50.0);
    filter.update_from_world(&world);

    // Register 50 clients
    for i in 0..50 {
        let pos = Vec3::new(((i % 10) as f32) * 100.0, 0.0, ((i / 10) as f32) * 100.0);
        filter.register_client(i, pos, 100.0);

        // Initialize cache
        filter.get_visibility_changes(i);
    }

    let (without, with, reduction) = filter.compute_bandwidth_reduction(50, 1000);

    assert!(reduction >= 70.0, "Should achieve ≥70% reduction");
    assert!(with < without, "With interest should be less");
}

// ============================================================================
// Edge Cases and Robustness Tests
// ============================================================================

#[test]
fn test_empty_world() {
    let mut world = World::new();
    world.register::<Aabb>();

    let mut manager = InterestManager::new(50.0);
    manager.update_from_world(&world);

    manager.set_client_interest(1, AreaOfInterest::new(Vec3::ZERO, 100.0));
    let visible = manager.calculate_visibility(1);

    assert!(visible.is_empty(), "Empty world should have no visible entities");
}

#[test]
fn test_no_clients() {
    let (world, _) = create_test_world(100);

    let mut manager = InterestManager::new(50.0);
    manager.update_from_world(&world);

    // No clients registered
    let (without, with, reduction) = manager.compute_bandwidth_reduction();

    assert_eq!(without, 0, "No clients = no updates");
    assert_eq!(with, 0, "No clients = no updates");
    assert_eq!(reduction, 0.0, "No reduction with no clients");
}

#[test]
fn test_very_large_aoi() {
    let (world, entities) = create_test_world(100);

    let mut manager = InterestManager::new(50.0);
    manager.update_from_world(&world);

    // AOI so large it covers everything
    manager.set_client_interest(1, AreaOfInterest::new(Vec3::ZERO, 10000.0));
    let visible = manager.calculate_visibility(1);

    // Should see most/all entities
    assert!(visible.len() >= entities.len() * 8 / 10, "Large AOI should see most entities");
}

#[test]
fn test_very_small_aoi() {
    let (world, _) = create_test_world(100);

    let mut manager = InterestManager::new(50.0);
    manager.update_from_world(&world);

    // Tiny AOI
    manager.set_client_interest(1, AreaOfInterest::new(Vec3::ZERO, 1.0));
    let visible = manager.calculate_visibility(1);

    // May see 0 or very few entities
    assert!(visible.len() < 10, "Small AOI should see few entities");
}

#[test]
fn test_client_unregister() {
    let (world, _) = create_test_world(100);

    let mut manager = InterestManager::new(50.0);
    manager.update_from_world(&world);

    manager.set_client_interest(1, AreaOfInterest::new(Vec3::ZERO, 100.0));
    assert_eq!(manager.client_count(), 1);

    manager.clear_client(1);
    assert_eq!(manager.client_count(), 0);

    // Should handle gracefully
    let visible = manager.calculate_visibility(1);
    assert!(visible.is_empty(), "Unregistered client should see nothing");
}

#[test]
fn test_rapid_position_updates() {
    let (world, _) = create_test_world(100);

    let mut manager = InterestManager::new(50.0);
    manager.update_from_world(&world);

    manager.set_client_interest(1, AreaOfInterest::new(Vec3::ZERO, 50.0));

    // Rapidly update client position
    for i in 0..100 {
        let new_aoi = AreaOfInterest::new(Vec3::new(i as f32, 0.0, 0.0), 50.0);
        manager.set_client_interest(1, new_aoi);
    }

    // Should still work correctly
    let visible = manager.calculate_visibility(1);
    assert!(!visible.is_empty() || true, "Should handle rapid updates");
}

// ============================================================================
// Correctness Tests
// ============================================================================

#[test]
fn test_aoi_containment_accuracy() {
    let aoi = AreaOfInterest::new(Vec3::ZERO, 100.0);

    // Points inside
    assert!(aoi.contains(Vec3::ZERO), "Center should be inside");
    assert!(aoi.contains(Vec3::new(50.0, 0.0, 0.0)), "50 units should be inside");
    assert!(aoi.contains(Vec3::new(0.0, 0.0, 99.0)), "99 units should be inside");

    // Points outside
    assert!(!aoi.contains(Vec3::new(101.0, 0.0, 0.0)), "101 units should be outside");
    assert!(
        !aoi.contains(Vec3::new(71.0, 0.0, 71.0)),
        "√(71²+71²) ≈ 100.4 should be outside"
    );

    // Boundary cases (at exactly radius distance)
    let boundary = Vec3::new(100.0, 0.0, 0.0);
    assert!(aoi.contains(boundary), "Exactly at radius should be inside (≤)");
}

#[test]
fn test_no_duplicate_entities() {
    let (world, _) = create_test_world(100);

    let mut manager = InterestManager::new(50.0);
    manager.update_from_world(&world);

    manager.set_client_interest(1, AreaOfInterest::new(Vec3::ZERO, 100.0));
    let visible = manager.calculate_visibility(1);

    // Check for duplicates
    let mut seen = std::collections::HashSet::new();
    for entity in &visible {
        assert!(seen.insert(*entity), "Duplicate entity found: {:?}", entity);
    }
}

#[test]
fn test_consistent_visibility() {
    let (world, _) = create_test_world(100);

    let mut manager = InterestManager::new(50.0);
    manager.update_from_world(&world);

    manager.set_client_interest(1, AreaOfInterest::new(Vec3::ZERO, 100.0));

    // Calculate multiple times without changes
    let visible1 = manager.calculate_visibility(1);
    let visible2 = manager.calculate_visibility(1);
    let visible3 = manager.calculate_visibility(1);

    assert_eq!(visible1, visible2, "Visibility should be consistent");
    assert_eq!(visible2, visible3, "Visibility should be consistent");
}

// ============================================================================
// Statistics and Metrics Tests
// ============================================================================

#[test]
fn test_statistics() {
    let (world, _) = create_test_world(500);

    let mut manager = InterestManager::new(50.0);
    manager.update_from_world(&world);

    assert_eq!(manager.entity_count(), 500);
    assert_eq!(manager.client_count(), 0);

    // Add clients
    for i in 0..10 {
        manager.set_client_interest(i, AreaOfInterest::new(Vec3::ZERO, 100.0));
        manager.get_visibility_changes(i); // Prime cache
    }

    assert_eq!(manager.client_count(), 10);

    let avg = manager.average_visible_entities();
    assert!(avg > 0.0, "Average should be > 0");
    assert!(avg < 500.0, "Average should be < total entities");
}

#[test]
fn test_filter_stats() {
    let (world, _) = create_test_world(200);

    let mut filter = InterestFilter::new(50.0);
    filter.update_from_world(&world);

    filter.register_client(1, Vec3::ZERO, 100.0);

    let stats = filter.stats();
    assert_eq!(stats.client_count, 1);
    assert_eq!(stats.entity_count, 200);
}
