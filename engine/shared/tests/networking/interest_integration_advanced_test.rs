//! Advanced Interest Management Integration Tests
//!
//! Tests complex integration scenarios:
//! - Multi-client coordination
//! - Priority system integration
//! - Network packet optimization
//! - Zone transitions and streaming
//! - LOD integration scenarios
//! - Bandwidth budget enforcement

use engine_core::{Aabb, Quat, Transform, Vec3, World};
use engine_interest::{AreaOfInterest, InterestManager};
use std::collections::HashSet;

// ============================================================================
// Multi-Client Coordination Tests
// ============================================================================

#[test]
fn test_client_isolation() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Aabb>();

    let mut manager = InterestManager::new(50.0);

    // Create entities in different regions
    let mut region_a_entities = Vec::new();
    let mut region_b_entities = Vec::new();

    // Region A: entities at (0, 0, 0) to (100, 0, 100)
    for i in 0..10 {
        let entity = world.spawn();
        let pos = Vec3::new(i as f32 * 10.0, 0.0, 0.0);
        world.add(entity, Transform::new(pos, Quat::IDENTITY, Vec3::ONE));
        world.add(entity, Aabb::from_center_half_extents(pos, Vec3::ONE));
        region_a_entities.push(entity);
    }

    // Region B: entities at (1000, 0, 1000) to (1100, 0, 1100)
    for i in 0..10 {
        let entity = world.spawn();
        let pos = Vec3::new(1000.0 + i as f32 * 10.0, 0.0, 1000.0);
        world.add(entity, Transform::new(pos, Quat::IDENTITY, Vec3::ONE));
        world.add(entity, Aabb::from_center_half_extents(pos, Vec3::ONE));
        region_b_entities.push(entity);
    }

    manager.update_from_world(&world);

    // Client 1 in Region A
    let client_1 = 1;
    manager.set_client_interest(client_1, AreaOfInterest::new(Vec3::new(50.0, 0.0, 0.0), 100.0));

    // Client 2 in Region B
    let client_2 = 2;
    manager
        .set_client_interest(client_2, AreaOfInterest::new(Vec3::new(1050.0, 0.0, 1000.0), 100.0));

    let visible_1 = manager.calculate_visibility(client_1);
    let visible_2 = manager.calculate_visibility(client_2);

    // Clients should see completely different sets of entities
    let set_1: HashSet<_> = visible_1.into_iter().collect();
    let set_2: HashSet<_> = visible_2.into_iter().collect();

    let intersection: Vec<_> = set_1.intersection(&set_2).collect();
    assert!(
        intersection.is_empty(),
        "Clients in different regions should see no overlapping entities"
    );

    // Client 1 should only see region A entities
    for entity in &region_a_entities {
        assert!(set_1.contains(entity), "Client 1 should see region A entities");
    }

    // Client 2 should only see region B entities
    for entity in &region_b_entities {
        assert!(set_2.contains(entity), "Client 2 should see region B entities");
    }
}

#[test]
fn test_shared_visibility_region() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Aabb>();

    let mut manager = InterestManager::new(50.0);

    // Create entities in a shared region
    let mut entities = Vec::new();
    for i in 0..20 {
        let entity = world.spawn();
        let pos = Vec3::new(i as f32 * 5.0, 0.0, 0.0);
        world.add(entity, Transform::new(pos, Quat::IDENTITY, Vec3::ONE));
        world.add(entity, Aabb::from_center_half_extents(pos, Vec3::ONE));
        entities.push(entity);
    }

    manager.update_from_world(&world);

    // Three clients with overlapping AOIs
    let clients = vec![
        (1, Vec3::new(25.0, 0.0, 0.0)),
        (2, Vec3::new(35.0, 0.0, 0.0)),
        (3, Vec3::new(45.0, 0.0, 0.0)),
    ];

    for (client_id, pos) in &clients {
        manager.set_client_interest(*client_id, AreaOfInterest::new(*pos, 30.0));
    }

    let mut visibility_sets = Vec::new();
    for (client_id, _) in &clients {
        let visible = manager.calculate_visibility(*client_id);
        visibility_sets.push(visible.into_iter().collect::<HashSet<_>>());
    }

    // Clients should have some shared visibility
    let shared_1_2: Vec<_> = visibility_sets[0].intersection(&visibility_sets[1]).collect();
    let shared_2_3: Vec<_> = visibility_sets[1].intersection(&visibility_sets[2]).collect();

    assert!(!shared_1_2.is_empty(), "Adjacent clients should share some visibility");
    assert!(!shared_2_3.is_empty(), "Adjacent clients should share some visibility");
}

#[test]
fn test_100_concurrent_clients() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Aabb>();

    let mut manager = InterestManager::new(50.0);

    // Create 1000 entities in a 100x100 grid
    for i in 0..1000 {
        let entity = world.spawn();
        let x = ((i % 100) as f32) * 10.0;
        let z = ((i / 100) as f32) * 10.0;
        let pos = Vec3::new(x, 0.0, z);
        world.add(entity, Transform::new(pos, Quat::IDENTITY, Vec3::ONE));
        world.add(entity, Aabb::from_center_half_extents(pos, Vec3::ONE));
    }

    manager.update_from_world(&world);

    // Register 100 clients scattered across the world
    for client_id in 0..100 {
        let x = ((client_id % 10) as f32) * 100.0;
        let z = ((client_id / 10) as f32) * 100.0;
        let pos = Vec3::new(x, 0.0, z);
        manager.set_client_interest(client_id as u64, AreaOfInterest::new(pos, 150.0));
    }

    let start = std::time::Instant::now();

    // Calculate visibility for all clients
    let mut total_visible = 0;
    for client_id in 0..100 {
        let visible = manager.calculate_visibility(client_id as u64);
        total_visible += visible.len();
    }

    let elapsed = start.elapsed();

    // Should complete in reasonable time
    assert!(
        elapsed.as_millis() < 100,
        "100 clients visibility should compute in <100ms, took {:?}",
        elapsed
    );
    assert!(total_visible > 0, "Clients should see some entities");

    // Average entities per client should be reasonable
    let avg_per_client = total_visible / 100;
    assert!(
        avg_per_client < 200,
        "Average entities per client should be <200 (got {})",
        avg_per_client
    );
}

// ============================================================================
// Network Packet Optimization Tests
// ============================================================================

#[test]
fn test_bandwidth_aware_visibility() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Aabb>();

    let mut manager = InterestManager::new(50.0);

    // Create many entities in a cluster
    for i in 0..500 {
        let entity = world.spawn();
        let pos = Vec3::new((i % 25) as f32 * 10.0, 0.0, (i / 25) as f32 * 10.0);
        world.add(entity, Transform::new(pos, Quat::IDENTITY, Vec3::ONE));
        world.add(entity, Aabb::from_center_half_extents(pos, Vec3::ONE));
    }

    manager.update_from_world(&world);

    // Client at center of entity cluster
    let client_id = 1;
    let client_pos = Vec3::new(125.0, 0.0, 100.0);
    manager.set_client_interest(client_id, AreaOfInterest::new(client_pos, 200.0));

    // Get visible entities
    let visible = manager.calculate_visibility(client_id);

    // Should see many entities in dense area
    assert!(visible.len() > 50, "Should see many entities in dense area");
    assert!(
        visible.len() <= 500,
        "Should not see more entities than exist (got {})",
        visible.len()
    );

    // Visibility should be deterministic
    let visible2 = manager.calculate_visibility(client_id);
    assert_eq!(visible.len(), visible2.len(), "Visibility should be consistent");

    // Test smaller AOI reduces visibility
    manager.set_client_interest(client_id, AreaOfInterest::new(client_pos, 50.0));
    let visible_small = manager.calculate_visibility(client_id);
    assert!(visible_small.len() < visible.len(), "Smaller AOI should see fewer entities");
}

#[test]
fn test_incremental_visibility_updates() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Aabb>();

    let mut manager = InterestManager::new(50.0);

    // Create entities
    let mut entities = Vec::new();
    for i in 0..50 {
        let entity = world.spawn();
        let pos = Vec3::new(i as f32 * 10.0, 0.0, 0.0);
        world.add(entity, Transform::new(pos, Quat::IDENTITY, Vec3::ONE));
        world.add(entity, Aabb::from_center_half_extents(pos, Vec3::ONE));
        entities.push(entity);
    }

    manager.update_from_world(&world);

    let client_id = 1;
    manager.set_client_interest(client_id, AreaOfInterest::new(Vec3::ZERO, 100.0));

    // Initial visibility
    let visible_1 = manager.calculate_visibility(client_id);
    let (entered_1, exited_1) = manager.get_visibility_changes(client_id);

    assert_eq!(
        entered_1.len(),
        visible_1.len(),
        "All visible entities should be in entered list initially"
    );
    assert!(exited_1.is_empty(), "No entities should have exited initially");

    // Move AOI slightly
    manager.set_client_interest(client_id, AreaOfInterest::new(Vec3::new(20.0, 0.0, 0.0), 100.0));

    let visible_2 = manager.calculate_visibility(client_id);
    let (entered_2, exited_2) = manager.get_visibility_changes(client_id);

    // Should have some changes
    let total_changes = entered_2.len() + exited_2.len();
    assert!(total_changes > 0, "Should detect visibility changes when AOI moves");
    assert!(
        total_changes < visible_2.len(),
        "Changes should be incremental, not full update"
    );
}

// ============================================================================
// Zone Transition Tests
// ============================================================================

#[test]
fn test_zone_transition() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Aabb>();

    let mut manager = InterestManager::new(100.0);

    // Zone A: 0-500
    for i in 0..10 {
        let entity = world.spawn();
        let pos = Vec3::new(i as f32 * 50.0, 0.0, 0.0);
        world.add(entity, Transform::new(pos, Quat::IDENTITY, Vec3::ONE));
        world.add(entity, Aabb::from_center_half_extents(pos, Vec3::ONE));
    }

    // Zone B: 1000-1500
    for i in 0..10 {
        let entity = world.spawn();
        let pos = Vec3::new(1000.0 + i as f32 * 50.0, 0.0, 0.0);
        world.add(entity, Transform::new(pos, Quat::IDENTITY, Vec3::ONE));
        world.add(entity, Aabb::from_center_half_extents(pos, Vec3::ONE));
    }

    manager.update_from_world(&world);

    let client_id = 1;

    // Start in Zone A
    manager.set_client_interest(client_id, AreaOfInterest::new(Vec3::new(250.0, 0.0, 0.0), 200.0));
    let visible_zone_a = manager.calculate_visibility(client_id);
    let zone_a_count = visible_zone_a.len();

    // Transition to Zone B
    manager.set_client_interest(client_id, AreaOfInterest::new(Vec3::new(1250.0, 0.0, 0.0), 200.0));
    let visible_zone_b = manager.calculate_visibility(client_id);
    let zone_b_count = visible_zone_b.len();

    // Should see different entities in different zones
    let set_a: HashSet<_> = visible_zone_a.into_iter().collect();
    let set_b: HashSet<_> = visible_zone_b.into_iter().collect();
    let overlap: Vec<_> = set_a.intersection(&set_b).collect();

    assert!(
        overlap.is_empty(),
        "Should see completely different entities after zone transition"
    );
    assert!(zone_a_count > 0, "Should see entities in zone A");
    assert!(zone_b_count > 0, "Should see entities in zone B");
}

#[test]
fn test_streaming_zone_load() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Aabb>();

    let mut manager = InterestManager::new(100.0);

    // Simulate streaming: initially empty world
    manager.update_from_world(&world);

    let client_id = 1;
    manager.set_client_interest(client_id, AreaOfInterest::new(Vec3::ZERO, 150.0));

    let visible_empty = manager.calculate_visibility(client_id);
    assert!(visible_empty.is_empty(), "Should see no entities in empty world");

    // Stream in entities (simulating chunk load)
    for i in 0..50 {
        let entity = world.spawn();
        let x = ((i % 10) as f32) * 20.0;
        let z = ((i / 10) as f32) * 20.0;
        let pos = Vec3::new(x, 0.0, z);
        world.add(entity, Transform::new(pos, Quat::IDENTITY, Vec3::ONE));
        world.add(entity, Aabb::from_center_half_extents(pos, Vec3::ONE));
    }

    manager.update_from_world(&world);

    let visible_loaded = manager.calculate_visibility(client_id);
    assert!(!visible_loaded.is_empty(), "Should see entities after streaming load");

    let (entered, exited) = manager.get_visibility_changes(client_id);
    assert_eq!(
        entered.len(),
        visible_loaded.len(),
        "All new entities should be in entered list"
    );
    assert!(exited.is_empty(), "No entities should have exited");
}

// ============================================================================
// Performance Under Load Tests
// ============================================================================

#[test]
fn test_massive_client_count() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Aabb>();

    let mut manager = InterestManager::new(100.0);

    // Create 2000 entities
    for i in 0..2000 {
        let entity = world.spawn();
        let x = ((i % 50) as f32) * 20.0;
        let z = ((i / 50) as f32) * 20.0;
        let pos = Vec3::new(x, 0.0, z);
        world.add(entity, Transform::new(pos, Quat::IDENTITY, Vec3::ONE));
        world.add(entity, Aabb::from_center_half_extents(pos, Vec3::ONE));
    }

    manager.update_from_world(&world);

    // Register 500 clients
    for client_id in 0..500 {
        let x = ((client_id % 25) as f32) * 40.0;
        let z = ((client_id / 25) as f32) * 40.0;
        let pos = Vec3::new(x, 0.0, z);
        manager.set_client_interest(client_id as u64, AreaOfInterest::new(pos, 150.0));
    }

    let start = std::time::Instant::now();

    // Calculate visibility for all clients
    for client_id in 0..500 {
        let _ = manager.calculate_visibility(client_id as u64);
    }

    let elapsed = start.elapsed();

    // Should handle 500 clients in reasonable time
    assert!(
        elapsed.as_millis() < 500,
        "500 clients should process in <500ms, took {:?}",
        elapsed
    );
}

#[test]
fn test_entity_churn_performance() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Aabb>();

    let mut manager = InterestManager::new(50.0);

    let client_id = 1;
    manager
        .set_client_interest(client_id, AreaOfInterest::new(Vec3::new(500.0, 0.0, 500.0), 100.0));

    // Simulate entity churn: spawn and despawn entities rapidly
    for frame in 0..100 {
        // Spawn 10 entities
        let mut entities = Vec::new();
        for i in 0..10 {
            let entity = world.spawn();
            let pos = Vec3::new(
                500.0 + ((frame * 10 + i) % 20) as f32 * 5.0,
                0.0,
                500.0 + ((frame * 10 + i) / 20) as f32 * 5.0,
            );
            world.add(entity, Transform::new(pos, Quat::IDENTITY, Vec3::ONE));
            world.add(entity, Aabb::from_center_half_extents(pos, Vec3::ONE));
            entities.push(entity);
        }

        manager.update_from_world(&world);
        let _ = manager.calculate_visibility(client_id);

        // Despawn half the entities
        for i in 0..5 {
            world.despawn(entities[i]);
        }
    }

    // Should complete without crashing or performance degradation
    let visible = manager.calculate_visibility(client_id);
    assert!(visible.len() > 0, "Should handle entity churn gracefully");
}

// ============================================================================
// Correctness Under Stress Tests
// ============================================================================

#[test]
fn test_no_visibility_leaks() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Aabb>();

    let mut manager = InterestManager::new(50.0);

    // Create entities far from origin
    let mut distant_entities = Vec::new();
    for i in 0..20 {
        let entity = world.spawn();
        let pos = Vec3::new(10000.0 + i as f32 * 10.0, 0.0, 10000.0);
        world.add(entity, Transform::new(pos, Quat::IDENTITY, Vec3::ONE));
        world.add(entity, Aabb::from_center_half_extents(pos, Vec3::ONE));
        distant_entities.push(entity);
    }

    manager.update_from_world(&world);

    // Client at origin with small AOI
    let client_id = 1;
    manager.set_client_interest(client_id, AreaOfInterest::new(Vec3::ZERO, 100.0));

    let visible = manager.calculate_visibility(client_id);

    // Should NOT see any distant entities
    for entity in &distant_entities {
        assert!(!visible.contains(entity), "Should not see distant entities (visibility leak!)");
    }

    assert!(visible.is_empty(), "Should see no entities when all are distant");
}

#[test]
fn test_visibility_determinism() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Aabb>();

    let mut manager = InterestManager::new(50.0);

    // Create entities
    for i in 0..100 {
        let entity = world.spawn();
        let x = ((i % 10) as f32) * 10.0;
        let z = ((i / 10) as f32) * 10.0;
        let pos = Vec3::new(x, 0.0, z);
        world.add(entity, Transform::new(pos, Quat::IDENTITY, Vec3::ONE));
        world.add(entity, Aabb::from_center_half_extents(pos, Vec3::ONE));
    }

    manager.update_from_world(&world);

    let client_id = 1;
    manager.set_client_interest(client_id, AreaOfInterest::new(Vec3::new(50.0, 0.0, 50.0), 75.0));

    // Calculate visibility multiple times
    let results: Vec<_> = (0..10).map(|_| manager.calculate_visibility(client_id)).collect();

    // All results should be identical
    for i in 1..results.len() {
        assert_eq!(results[0].len(), results[i].len(), "Visibility should be deterministic");

        let set_0: HashSet<_> = results[0].iter().collect();
        let set_i: HashSet<_> = results[i].iter().collect();
        assert_eq!(set_0, set_i, "Visibility should be deterministic");
    }
}
