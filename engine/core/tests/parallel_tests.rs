//! Integration tests for parallel query iteration
//!
//! Tests thread safety, correctness, and data race freedom.

use engine_core::ecs::{Component, ParallelWorld, World};
use rayon::prelude::*;

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

#[derive(Debug, Clone, Copy, PartialEq)]
struct Health {
    current: f32,
    max: f32,
}
impl Component for Health {}

#[test]
fn test_parallel_iter_correctness() {
    let mut world = World::new();
    world.register::<Position>();

    // Spawn entities with known values
    for i in 0..1000 {
        let entity = world.spawn();
        world.add(
            entity,
            Position {
                x: i as f32,
                y: i as f32 * 2.0,
                z: i as f32 * 3.0,
            },
        );
    }

    // Parallel sum should match sequential sum
    let seq_sum: f32 = world.query::<&Position>().map(|(_, pos)| pos.x).sum();

    let par_sum: f32 = world
        .par_query::<Position>()
        .map(|(_, pos)| pos.x)
        .sum();

    assert_eq!(seq_sum, par_sum);
}

#[test]
fn test_parallel_iter_mut_correctness() {
    let mut world = World::new();
    world.register::<Position>();

    // Spawn entities
    for i in 0..1000 {
        let entity = world.spawn();
        world.add(
            entity,
            Position {
                x: i as f32,
                y: 0.0,
                z: 0.0,
            },
        );
    }

    // Apply parallel mutation
    world.query::<&mut Position>().par_iter_mut().for_each(|(_, pos)| {
        pos.y = pos.x * 2.0;
        pos.z = pos.x * 3.0;
    });

    // Verify all mutations were applied
    for (_, pos) in world.query::<&Position>() {
        assert_eq!(pos.y, pos.x * 2.0);
        assert_eq!(pos.z, pos.x * 3.0);
    }
}

#[test]
fn test_parallel_two_component_correctness() {
    let mut world = World::new();
    world.register::<Position>();
    world.register::<Velocity>();

    // Spawn entities
    for i in 0..1000 {
        let entity = world.spawn();
        world.add(
            entity,
            Position {
                x: i as f32,
                y: 0.0,
                z: 0.0,
            },
        );
        world.add(
            entity,
            Velocity {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
        );
    }

    // Sequential sum
    let seq_sum: f32 = world
        .query::<(&Position, &Velocity)>()
        .map(|(_, (pos, vel))| pos.x + vel.x)
        .sum();

    // Parallel sum
    let par_sum: f32 = world
        .query::<(&Position, &Velocity)>()
        .par_iter()
        .map(|(_, (pos, vel))| pos.x + vel.x)
        .sum();

    assert_eq!(seq_sum, par_sum);
}

#[test]
fn test_parallel_mixed_mutability() {
    let mut world = World::new();
    world.register::<Position>();
    world.register::<Velocity>();

    // Spawn entities
    for i in 0..1000 {
        let entity = world.spawn();
        world.add(
            entity,
            Position {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
        );
        world.add(
            entity,
            Velocity {
                x: i as f32,
                y: i as f32 * 2.0,
                z: i as f32 * 3.0,
            },
        );
    }

    // Apply parallel physics update
    world
        .query::<(&mut Position, &Velocity)>()
        .par_iter_mut()
        .for_each(|(_, (pos, vel))| {
            pos.x += vel.x;
            pos.y += vel.y;
            pos.z += vel.z;
        });

    // Verify updates
    for (_, (pos, vel)) in world.query::<(&Position, &Velocity)>() {
        assert_eq!(pos.x, vel.x);
        assert_eq!(pos.y, vel.y);
        assert_eq!(pos.z, vel.z);
    }
}

#[test]
fn test_parallel_empty_query() {
    let mut world = World::new();
    world.register::<Position>();

    // No entities - should handle gracefully
    let count = world.query::<&Position>().par_iter().count();
    assert_eq!(count, 0);

    let sum: f32 = world
        .query::<&Position>()
        .par_iter()
        .map(|(_, pos)| pos.x)
        .sum();
    assert_eq!(sum, 0.0);
}

#[test]
fn test_parallel_filter_map() {
    let mut world = World::new();
    world.register::<Position>();
    world.register::<Health>();

    // Spawn entities - some with Health, some without
    for i in 0..1000 {
        let entity = world.spawn();
        world.add(
            entity,
            Position {
                x: i as f32,
                y: 0.0,
                z: 0.0,
            },
        );

        // Only add Health to even entities
        if i % 2 == 0 {
            world.add(entity, Health { current: 100.0, max: 100.0 });
        }
    }

    // Count entities with Position (should be all 1000)
    let pos_count = world.query::<&Position>().par_iter().count();
    assert_eq!(pos_count, 1000);

    // Count entities with both Position and Health (should be 500)
    let both_count = world
        .query::<(&Position, &Health)>()
        .par_iter()
        .count();
    assert_eq!(both_count, 500);
}

#[test]
fn test_parallel_collect() {
    let mut world = World::new();
    world.register::<Position>();

    // Spawn entities
    for i in 0..100 {
        let entity = world.spawn();
        world.add(
            entity,
            Position {
                x: i as f32,
                y: 0.0,
                z: 0.0,
            },
        );
    }

    // Collect parallel results
    let positions: Vec<f32> = world
        .query::<&Position>()
        .par_iter()
        .map(|(_, pos)| pos.x)
        .collect();

    assert_eq!(positions.len(), 100);

    // Results may be in any order due to parallelism, but all values should be present
    let mut sorted = positions.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

    for (i, &val) in sorted.iter().enumerate() {
        assert_eq!(val, i as f32);
    }
}

#[test]
fn test_parallel_reduce() {
    let mut world = World::new();
    world.register::<Position>();

    // Spawn entities
    for i in 0..1000 {
        let entity = world.spawn();
        world.add(
            entity,
            Position {
                x: i as f32,
                y: 0.0,
                z: 0.0,
            },
        );
    }

    // Find max using parallel reduce
    let max = world
        .query::<&Position>()
        .par_iter()
        .map(|(_, pos)| pos.x)
        .reduce(|| 0.0, |a, b| a.max(b));

    assert_eq!(max, 999.0);

    // Find min
    let min = world
        .query::<&Position>()
        .par_iter()
        .map(|(_, pos)| pos.x)
        .reduce(|| f32::MAX, |a, b| a.min(b));

    assert_eq!(min, 0.0);
}

#[test]
fn test_parallel_any_all() {
    let mut world = World::new();
    world.register::<Position>();

    // Spawn entities
    for i in 0..100 {
        let entity = world.spawn();
        world.add(
            entity,
            Position {
                x: i as f32,
                y: 0.0,
                z: 0.0,
            },
        );
    }

    // Test any
    let has_large = world
        .query::<&Position>()
        .par_iter()
        .any(|(_, pos)| pos.x > 50.0);
    assert!(has_large);

    let has_negative = world
        .query::<&Position>()
        .par_iter()
        .any(|(_, pos)| pos.x < 0.0);
    assert!(!has_negative);

    // Test all
    let all_non_negative = world
        .query::<&Position>()
        .par_iter()
        .all(|(_, pos)| pos.x >= 0.0);
    assert!(all_non_negative);

    let all_large = world
        .query::<&Position>()
        .par_iter()
        .all(|(_, pos)| pos.x > 50.0);
    assert!(!all_large);
}

#[test]
fn test_parallel_entity_order_independence() {
    // Verify that results are correct regardless of parallel chunk distribution
    let mut world = World::new();
    world.register::<Position>();

    // Spawn entities
    for i in 0..1000 {
        let entity = world.spawn();
        world.add(
            entity,
            Position {
                x: i as f32,
                y: i as f32,
                z: i as f32,
            },
        );
    }

    // Run multiple times - results should be deterministic
    for _ in 0..10 {
        let sum: f32 = world
            .query::<&Position>()
            .par_iter()
            .map(|(_, pos)| pos.x)
            .sum();
        assert_eq!(sum, 499_500.0); // Sum of 0..1000
    }
}

#[test]
fn test_parallel_sparse_entities() {
    // Test with sparse entity IDs (some despawned)
    let mut world = World::new();
    world.register::<Position>();

    let mut entities = Vec::new();

    // Spawn entities
    for i in 0..100 {
        let entity = world.spawn();
        world.add(entity, Position { x: i as f32, y: 0.0, z: 0.0 });
        entities.push(entity);
    }

    // Despawn every other entity
    for i in (0..100).step_by(2) {
        world.despawn(entities[i]);
    }

    // Should still iterate correctly over remaining entities
    let count = world.query::<&Position>().par_iter().count();
    assert_eq!(count, 50);

    let sum: f32 = world
        .query::<&Position>()
        .par_iter()
        .map(|(_, pos)| pos.x)
        .sum();

    // Sum of odd numbers 1 + 3 + 5 + ... + 99 = 2500
    assert_eq!(sum, 2500.0);
}

#[test]
fn test_parallel_large_workload() {
    // Test with a realistic large workload
    let mut world = World::new();
    world.register::<Position>();
    world.register::<Velocity>();

    // Spawn 10,000 entities
    for i in 0..10_000 {
        let entity = world.spawn();
        world.add(entity, Position { x: 0.0, y: 0.0, z: 0.0 });
        world.add(
            entity,
            Velocity {
                x: (i % 100) as f32,
                y: ((i / 100) % 100) as f32,
                z: (i / 10000) as f32,
            },
        );
    }

    // Apply physics update in parallel
    world
        .query::<(&mut Position, &Velocity)>()
        .par_iter_mut()
        .for_each(|(_, (pos, vel))| {
            let dt = 0.016;
            pos.x += vel.x * dt;
            pos.y += vel.y * dt;
            pos.z += vel.z * dt;
        });

    // Verify all entities were updated
    let count = world
        .query::<&Position>()
        .par_iter()
        .filter(|(_, pos)| pos.x > 0.0 || pos.y > 0.0)
        .count();

    assert!(count > 0); // At least some entities should have moved
}

#[test]
fn test_parallel_thread_pool_isolation() {
    // Verify that parallel queries work with custom thread pools
    let mut world = World::new();
    world.register::<Position>();

    for i in 0..1000 {
        let entity = world.spawn();
        world.add(entity, Position { x: i as f32, y: 0.0, z: 0.0 });
    }

    // Use a custom thread pool with 4 threads
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();

    let sum = pool.install(|| {
        world
            .query::<&Position>()
            .par_iter()
            .map(|(_, pos)| pos.x)
            .sum::<f32>()
    });

    assert_eq!(sum, 499_500.0);
}

// Note: Miri tests for data race detection would be added here if Rayon supported it
// Currently, Rayon is not compatible with Miri due to its use of thread-local storage
// and other features that Miri doesn't support.
//
// In practice, we rely on:
// 1. Type system guarantees (Send/Sync bounds)
// 2. Manual verification that indices are disjoint
// 3. ThreadSanitizer testing in CI (not Miri)
