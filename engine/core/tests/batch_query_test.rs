//! Integration test for batch query iteration (SIMD processing support)

use engine_core::ecs::{Component, World};

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
struct Position {
    x: f32,
    y: f32,
    z: f32,
}

impl Component for Position {}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
struct Velocity {
    x: f32,
    y: f32,
    z: f32,
}

impl Component for Velocity {}

#[test]
fn test_batch4_iteration() {
    let mut world = World::new();
    world.register::<Position>();
    world.register::<Velocity>();

    // Create 12 entities (3 full batches of 4)
    for i in 0..12 {
        let entity = world.spawn();
        world.add(entity, Position { x: i as f32, y: (i * 2) as f32, z: (i * 3) as f32 });
    }

    // Verify batch iteration
    let mut batch_count = 0;
    let mut total_entities = 0;

    for (entities, positions) in world.query_batch4::<Position>() {
        batch_count += 1;
        assert_eq!(entities.len(), 4, "Each batch should have exactly 4 entities");
        assert_eq!(positions.len(), 4, "Each batch should have exactly 4 components");
        total_entities += entities.len();

        // Verify data integrity
        for pos in positions.iter() {
            assert!(pos.x >= 0.0 && pos.x < 12.0, "Position x should be in range");
        }
    }

    assert_eq!(batch_count, 3, "Should have exactly 3 batches of 4");
    assert_eq!(total_entities, 12, "Should process all 12 entities");
}

#[test]
fn test_batch8_iteration() {
    let mut world = World::new();
    world.register::<Position>();

    // Create 16 entities (2 full batches of 8)
    for i in 0..16 {
        let entity = world.spawn();
        world.add(entity, Position { x: i as f32, y: (i * 2) as f32, z: (i * 3) as f32 });
    }

    // Verify batch iteration
    let mut batch_count = 0;
    let mut total_entities = 0;

    for (entities, positions) in world.query_batch8::<Position>() {
        batch_count += 1;
        assert_eq!(entities.len(), 8, "Each batch should have exactly 8 entities");
        assert_eq!(positions.len(), 8, "Each batch should have exactly 8 components");
        total_entities += entities.len();
    }

    assert_eq!(batch_count, 2, "Should have exactly 2 batches of 8");
    assert_eq!(total_entities, 16, "Should process all 16 entities");
}

#[test]
fn test_batch_simd_processing() {
    let mut world = World::new();
    world.register::<Position>();
    world.register::<Velocity>();

    // Create 8 entities with positions and velocities
    for i in 0..8 {
        let entity = world.spawn();
        world.add(entity, Position { x: i as f32, y: 0.0, z: 0.0 });
        world.add(entity, Velocity { x: 0.1, y: 0.2, z: 0.3 });
    }

    // Simulate SIMD-style batch processing
    for (_, positions) in world.query_batch4::<Position>() {
        // In real SIMD: convert to Vec3x4, process in parallel
        let sum_x: f32 = positions.iter().map(|p| p.x).sum();

        // Verify batch contains expected data
        assert_eq!(positions.len(), 4);
        assert!(sum_x >= 0.0, "Sum should be non-negative");
    }
}

#[test]
fn test_batch_with_partial_remainder() {
    let mut world = World::new();
    world.register::<Position>();

    // Create 10 entities (2 batches of 4 + 2 remainder, not processed in batch)
    for i in 0..10 {
        let entity = world.spawn();
        world.add(entity, Position { x: i as f32, y: 0.0, z: 0.0 });
    }

    let mut batch_count = 0;
    for (_entities, _positions) in world.query_batch4::<Position>() {
        batch_count += 1;
    }

    // Should only process complete batches (8 entities in 2 batches)
    assert_eq!(batch_count, 2, "Should have 2 complete batches, remainder not processed");
}
