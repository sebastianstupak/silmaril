//! Tests for query performance optimizations
//!
//! Verifies that:
//! 1. Prefetching doesn't break correctness
//! 2. Batch iteration works correctly
//! 3. Fast-path optimizations maintain correctness
//! 4. Cache locality improvements don't affect behavior

use engine_core::ecs::{Component, World};

#[derive(Debug, Clone, Copy, PartialEq)]
struct Position {
    x: f32,
    y: f32,
    z: f32,
}

impl Component for Position {}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Velocity {
    x: f32,
    y: f32,
    z: f32,
}

impl Component for Velocity {}

#[test]
fn test_prefetching_maintains_correctness() {
    let mut world = World::new();
    world.register::<Position>();

    // Create entities
    for i in 0..1000 {
        let entity = world.spawn();
        world.add(entity, Position { x: i as f32, y: 0.0, z: 0.0 });
    }

    // Query with prefetching should return all entities
    let count: usize = world.query::<&Position>().count();
    assert_eq!(count, 1000);

    // Values should be correct
    let mut sum = 0.0;
    for (_e, pos) in world.query::<&Position>() {
        sum += pos.x;
    }
    // Sum of 0..1000 = 999*1000/2 = 499500
    assert_eq!(sum, 499500.0);
}

#[test]
fn test_batch_iterator_4() {
    let mut world = World::new();
    world.register::<Position>();

    // Create exactly 12 entities (3 batches of 4)
    for i in 0..12 {
        let entity = world.spawn();
        world.add(entity, Position { x: i as f32, y: 0.0, z: 0.0 });
    }

    // Batch iteration should return 3 batches
    let batches: Vec<_> = world.query_batch4::<Position>().collect();
    assert_eq!(batches.len(), 3);

    // Each batch should have 4 positions
    for (entities, positions) in batches {
        assert_eq!(entities.len(), 4);
        assert_eq!(positions.len(), 4);

        // All positions should be valid
        for pos in positions {
            assert!(pos.x >= 0.0 && pos.x < 12.0);
        }
    }
}

#[test]
fn test_batch_iterator_8() {
    let mut world = World::new();
    world.register::<Position>();

    // Create exactly 16 entities (2 batches of 8)
    for i in 0..16 {
        let entity = world.spawn();
        world.add(entity, Position { x: i as f32, y: 0.0, z: 0.0 });
    }

    // Batch iteration should return 2 batches
    let batches: Vec<_> = world.query_batch8::<Position>().collect();
    assert_eq!(batches.len(), 2);

    // Each batch should have 8 positions
    for (entities, positions) in batches {
        assert_eq!(entities.len(), 8);
        assert_eq!(positions.len(), 8);

        // All positions should be valid
        for pos in positions {
            assert!(pos.x >= 0.0 && pos.x < 16.0);
        }
    }
}

#[test]
fn test_batch_iterator_partial_batch() {
    let mut world = World::new();
    world.register::<Position>();

    // Create 10 entities (not a multiple of 4)
    for i in 0..10 {
        let entity = world.spawn();
        world.add(entity, Position { x: i as f32, y: 0.0, z: 0.0 });
    }

    // Batch-4 should return 2 full batches, skip the partial batch
    let batches: Vec<_> = world.query_batch4::<Position>().collect();
    assert_eq!(batches.len(), 2);

    // Verify we got 8 total entities (2 batches * 4)
    let total_entities: usize = batches.iter().map(|(entities, _)| entities.len()).sum();
    assert_eq!(total_entities, 8);
}

#[test]
fn test_fast_path_single_component() {
    let mut world = World::new();
    world.register::<Position>();

    // Create many entities to test fast path performance
    for i in 0..10000 {
        let entity = world.spawn();
        world.add(entity, Position { x: i as f32, y: (i * 2) as f32, z: (i * 3) as f32 });
    }

    // Fast path should maintain correct count
    let count: usize = world.query::<&Position>().count();
    assert_eq!(count, 10000);

    // Fast path should maintain correct iteration
    let mut prev_x = -1.0;
    for (_e, pos) in world.query::<&Position>() {
        assert!(pos.x > prev_x);
        prev_x = pos.x;
    }
}

#[test]
fn test_two_component_query_with_prefetch() {
    let mut world = World::new();
    world.register::<Position>();
    world.register::<Velocity>();

    // Create entities with both components
    for i in 0..1000 {
        let entity = world.spawn();
        world.add(entity, Position { x: i as f32, y: 0.0, z: 0.0 });
        world.add(entity, Velocity { x: 1.0, y: 0.0, z: 0.0 });
    }

    // Two-component query with prefetching should return all entities
    let count: usize = world.query::<(&Position, &Velocity)>().count();
    assert_eq!(count, 1000);

    // Values should be correct
    for (_e, (pos, vel)) in world.query::<(&Position, &Velocity)>() {
        assert!(pos.x >= 0.0 && pos.x < 1000.0);
        assert_eq!(vel.x, 1.0);
    }
}

#[test]
fn test_cache_locality_doesnt_affect_behavior() {
    let mut world = World::new();
    world.register::<Position>();

    // Create entities in non-sequential order
    let entities: Vec<_> = (0..100).map(|_| world.spawn()).collect();

    // Add components in reverse order
    for (i, &entity) in entities.iter().rev().enumerate() {
        world.add(entity, Position { x: i as f32, y: 0.0, z: 0.0 });
    }

    // Query should still return all entities
    let count: usize = world.query::<&Position>().count();
    assert_eq!(count, 100);

    // All positions should be present
    let positions: Vec<_> = world.query::<&Position>().map(|(_, pos)| pos.x).collect();
    assert_eq!(positions.len(), 100);
}

#[test]
fn test_sparse_set_batch_access() {
    use engine_core::ecs::SparseSet;

    let mut storage = SparseSet::<Position>::new();

    // Add 8 components
    for i in 0..8 {
        let entity = engine_core::ecs::Entity::new(i, 0);
        storage.insert(entity, Position { x: i as f32, y: 0.0, z: 0.0 });
    }

    // Batch access should work
    if let Some((entities, components)) = storage.get_batch::<4>(0) {
        assert_eq!(entities.len(), 4);
        assert_eq!(components.len(), 4);

        for (i, comp) in components.iter().enumerate() {
            assert_eq!(comp.x, i as f32);
        }
    } else {
        panic!("Batch access failed");
    }
}

#[test]
fn test_batch_iterator_empty_world() {
    let world = World::new();

    // Batch iteration on empty world should work
    let batches: Vec<_> = world.query_batch4::<Position>().collect();
    assert_eq!(batches.len(), 0);

    let batches: Vec<_> = world.query_batch8::<Position>().collect();
    assert_eq!(batches.len(), 0);
}

#[test]
fn test_all_optimizations_together() {
    let mut world = World::new();
    world.register::<Position>();
    world.register::<Velocity>();

    // Create a realistic scenario: 5000 entities
    for i in 0..5000 {
        let entity = world.spawn();
        world.add(entity, Position { x: i as f32, y: 0.0, z: 0.0 });

        // Only half have velocity (sparse)
        if i % 2 == 0 {
            world.add(entity, Velocity { x: 1.0, y: 0.0, z: 0.0 });
        }
    }

    // Test 1: Single-component fast path
    let pos_count: usize = world.query::<&Position>().count();
    assert_eq!(pos_count, 5000);

    // Test 2: Two-component with prefetch
    let both_count: usize = world.query::<(&Position, &Velocity)>().count();
    assert_eq!(both_count, 2500);

    // Test 3: Batch iteration
    let batch_count: usize =
        world.query_batch4::<Position>().map(|(entities, _)| entities.len()).sum();
    assert!(batch_count > 0);
    assert!(batch_count <= 5000);

    // Test 4: All together - verify correctness
    let mut total_x = 0.0;
    for (_e, pos) in world.query::<&Position>() {
        total_x += pos.x;
    }
    // Sum should match expected value
    let expected_sum: f32 = (0..5000).map(|i| i as f32).sum();
    assert_eq!(total_x, expected_sum);
}
