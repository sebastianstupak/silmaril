//! Tests for component get() optimization
//!
//! Verifies that the optimized get_unchecked_fast() maintains correctness
//! and safety invariants.

use engine_core::ecs::change_detection::Tick;
use engine_core::ecs::{Component, Entity, EntityAllocator, SparseSet, World};

#[derive(Clone, Copy, Debug, PartialEq)]
struct Position {
    x: f32,
    y: f32,
    z: f32,
}

impl Component for Position {}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Velocity {
    x: f32,
    y: f32,
    z: f32,
}

impl Component for Velocity {}

// Helper to create entities for testing (since Entity::new is private)
fn create_test_entities(count: usize) -> (EntityAllocator, Vec<Entity>) {
    let mut allocator = EntityAllocator::new();
    let entities = (0..count).map(|_| allocator.allocate()).collect();
    (allocator, entities)
}

#[test]
fn test_get_unchecked_fast_correctness() {
    let mut storage = SparseSet::<Position>::new();
    let (allocator, entities) = create_test_entities(100);

    // Insert components
    for (i, &entity) in entities.iter().enumerate() {
        storage.insert(
            entity,
            Position { x: i as f32, y: i as f32 * 2.0, z: i as f32 * 3.0 },
            Tick::new(),
        );
    }

    // Verify unchecked fast matches regular get
    for (i, &entity) in entities.iter().enumerate() {
        let regular = storage.get(entity).unwrap();
        let unchecked = unsafe { storage.get_unchecked_fast(entity) };

        assert_eq!(regular.x, unchecked.x);
        assert_eq!(regular.y, unchecked.y);
        assert_eq!(regular.z, unchecked.z);
        assert_eq!(regular.x, i as f32);
    }
}

#[test]
fn test_get_unchecked_fast_mut_correctness() {
    let mut storage = SparseSet::<Position>::new();

    // Insert components
    for i in 0..100 {
        storage.insert(
            Entity::new(i, 0),
            Position { x: i as f32, y: i as f32, z: i as f32 },
            Tick::new(),
        );
    }

    // Modify via unchecked_fast_mut
    for i in 0..100 {
        let entity = Entity::new(i, 0);
        unsafe {
            let pos = storage.get_unchecked_fast_mut(entity);
            pos.x += 10.0;
        }
    }

    // Verify modifications
    for i in 0..100 {
        let entity = Entity::new(i, 0);
        let pos = storage.get(entity).unwrap();
        assert_eq!(pos.x, i as f32 + 10.0);
    }
}

#[test]
fn test_sparse_entity_ids() {
    let mut storage = SparseSet::<Position>::new();

    // Use sparse entity IDs
    let sparse_ids = vec![0, 10, 100, 1000, 5000];
    for &id in &sparse_ids {
        storage.insert(
            Entity::new(id, 0),
            Position { x: id as f32, y: id as f32, z: id as f32 },
            Tick::new(),
        );
    }

    // Verify unchecked fast works with sparse IDs
    for &id in &sparse_ids {
        let entity = Entity::new(id, 0);
        let pos = unsafe { storage.get_unchecked_fast(entity) };
        assert_eq!(pos.x, id as f32);
    }
}

#[test]
fn test_query_iteration_optimized() {
    let mut world = World::new();
    world.register::<Position>();

    // Create entities
    for i in 0..1000 {
        let entity = world.spawn();
        world.add(entity, Position { x: i as f32, y: i as f32, z: i as f32 });
    }

    // Query and verify
    let mut count = 0;
    let mut sum = 0.0;
    for (_entity, pos) in world.query::<&Position>() {
        sum += pos.x;
        count += 1;
    }

    assert_eq!(count, 1000);
    assert_eq!(sum, (0..1000).sum::<i32>() as f32);
}

#[test]
fn test_query_two_components_optimized() {
    let mut world = World::new();
    world.register::<Position>();
    world.register::<Velocity>();

    // Create entities with both components
    for i in 0..1000 {
        let entity = world.spawn();
        world.add(entity, Position { x: i as f32, y: i as f32, z: i as f32 });
        world.add(entity, Velocity { x: 1.0, y: 2.0, z: 3.0 });
    }

    // Query and verify
    let mut count = 0;
    for (_entity, (pos, vel)) in world.query::<(&Position, &Velocity)>() {
        assert_eq!(vel.x, 1.0);
        assert_eq!(vel.y, 2.0);
        assert_eq!(vel.z, 3.0);
        count += 1;
    }

    assert_eq!(count, 1000);
}

#[test]
fn test_removal_correctness() {
    let mut storage = SparseSet::<Position>::new();

    // Insert components
    for i in 0..100 {
        storage.insert(
            Entity::new(i, 0),
            Position { x: i as f32, y: i as f32, z: i as f32 },
            Tick::new(),
        );
    }

    // Remove some components
    for i in (0..100).step_by(2) {
        storage.remove(Entity::new(i, 0));
    }

    // Verify remaining components work with unchecked_fast
    for i in (1..100).step_by(2) {
        let entity = Entity::new(i, 0);
        let pos = unsafe { storage.get_unchecked_fast(entity) };
        assert_eq!(pos.x, i as f32);
    }
}

#[test]
fn test_replace_correctness() {
    let mut storage = SparseSet::<Position>::new();
    let entity = Entity::new(0, 0);

    // Insert initial value
    storage.insert(entity, Position { x: 1.0, y: 2.0, z: 3.0 }, Tick::new());

    // Replace value
    storage.insert(entity, Position { x: 10.0, y: 20.0, z: 30.0 }, Tick::new());

    // Verify unchecked_fast returns new value
    let pos = unsafe { storage.get_unchecked_fast(entity) };
    assert_eq!(pos.x, 10.0);
    assert_eq!(pos.y, 20.0);
    assert_eq!(pos.z, 30.0);
}

#[test]
fn test_large_dataset_correctness() {
    let mut storage = SparseSet::<Position>::with_capacity(100_000);

    // Insert large dataset
    for i in 0..100_000 {
        storage.insert(
            Entity::new(i, 0),
            Position { x: i as f32, y: i as f32, z: i as f32 },
            Tick::new(),
        );
    }

    // Verify random access
    for i in (0..100_000).step_by(1000) {
        let entity = Entity::new(i, 0);
        let regular = storage.get(entity).unwrap();
        let unchecked = unsafe { storage.get_unchecked_fast(entity) };
        assert_eq!(regular.x, unchecked.x);
    }
}

#[test]
fn test_iteration_after_modifications() {
    let mut world = World::new();
    world.register::<Position>();

    // Create entities
    let entities: Vec<_> = (0..1000)
        .map(|i| {
            let entity = world.spawn();
            world.add(entity, Position { x: i as f32, y: 0.0, z: 0.0 });
            entity
        })
        .collect();

    // Modify some entities
    for &entity in entities.iter().step_by(2) {
        if let Some(pos) = world.get_mut::<Position>(entity) {
            pos.x += 100.0;
        }
    }

    // Verify query still works correctly
    let mut count = 0;
    for (entity, pos) in world.query::<&Position>() {
        let idx = entity.id() as usize;
        if idx % 2 == 0 {
            assert!(pos.x >= 100.0);
        } else {
            assert!(pos.x < 100.0);
        }
        count += 1;
    }

    assert_eq!(count, 1000);
}

#[test]
fn test_empty_storage() {
    let storage = SparseSet::<Position>::new();

    // Query empty storage should not panic
    let count = storage.iter().count();
    assert_eq!(count, 0);
}

#[test]
fn test_single_entity() {
    let mut storage = SparseSet::<Position>::new();
    let entity = Entity::new(0, 0);

    storage.insert(entity, Position { x: 1.0, y: 2.0, z: 3.0 }, Tick::new());

    let pos = unsafe { storage.get_unchecked_fast(entity) };
    assert_eq!(pos.x, 1.0);
}

#[test]
fn test_concurrent_safe_reads() {
    use std::sync::Arc;
    use std::thread;

    let mut storage = SparseSet::<Position>::new();

    // Insert components
    for i in 0..1000 {
        storage.insert(
            Entity::new(i, 0),
            Position { x: i as f32, y: i as f32, z: i as f32 },
            Tick::new(),
        );
    }

    let storage = Arc::new(storage);

    // Spawn multiple reader threads
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let storage = Arc::clone(&storage);
            thread::spawn(move || {
                let mut sum = 0.0;
                for i in 0..1000 {
                    let entity = Entity::new(i, 0);
                    if let Some(pos) = storage.get(entity) {
                        sum += pos.x;
                    }
                }
                sum
            })
        })
        .collect();

    // Verify all threads get same result
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    let expected: f32 = (0..1000).sum::<i32>() as f32;

    for result in results {
        assert_eq!(result, expected);
    }
}
