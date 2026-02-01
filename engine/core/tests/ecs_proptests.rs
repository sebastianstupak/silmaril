//! Property-based tests for ECS (Entity Component System)
//!
//! These tests verify correctness properties of the ECS implementation:
//! - Entity allocate/free cycles maintain correctness
//! - Component add/remove sequences maintain data integrity
//! - Query correctness with random component combinations

use engine_core::ecs::{Component, EntityAllocator, World};
use proptest::prelude::*;

// ============================================================================
// Test Components
// ============================================================================

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

// ============================================================================
// Custom Strategies
// ============================================================================

fn arb_position() -> impl Strategy<Value = Position> {
    (any::<f32>(), any::<f32>(), any::<f32>()).prop_map(|(x, y, z)| Position { x, y, z })
}

// ============================================================================
// Property Test 1: Entity Allocation is Always Unique
// ============================================================================

proptest! {
    #[test]
    fn prop_entity_allocation_unique(count in 1usize..1000) {
        let mut allocator = EntityAllocator::new();
        let mut entities = Vec::new();

        for _ in 0..count {
            entities.push(allocator.allocate());
        }

        // All entities should be alive
        for entity in &entities {
            prop_assert!(allocator.is_alive(*entity));
        }

        // All entities should be unique (considering both ID and generation)
        for i in 0..entities.len() {
            for j in (i + 1)..entities.len() {
                prop_assert_ne!(
                    entities[i],
                    entities[j],
                    "Entities should be unique"
                );
            }
        }
    }
}

// ============================================================================
// Property Test 2: Entity Free/Allocate Cycles Maintain Correctness
// ============================================================================

proptest! {
    #[test]
    fn prop_entity_free_allocate_cycles(
        allocate_counts in prop::collection::vec(1usize..50, 1..20)
    ) {
        let mut allocator = EntityAllocator::new();
        let mut all_allocated = Vec::new();

        for &count in &allocate_counts {
            // Allocate entities
            let mut batch = Vec::new();
            for _ in 0..count {
                let entity = allocator.allocate();
                prop_assert!(allocator.is_alive(entity));
                batch.push(entity);
            }

            // Free half of them
            let to_free = batch.len() / 2;
            for i in 0..to_free {
                let freed = allocator.free(batch[i]);
                prop_assert!(freed, "First free should succeed");
                prop_assert!(!allocator.is_alive(batch[i]), "Entity should be dead after free");

                // Double free should return false
                let double_free = allocator.free(batch[i]);
                prop_assert!(!double_free, "Double free should return false");
            }

            // Keep track of entities that are still alive
            all_allocated.extend(batch.into_iter().skip(to_free));
        }

        // All entities we kept should still be alive
        for entity in &all_allocated {
            prop_assert!(
                allocator.is_alive(*entity),
                "Entity {:?} should still be alive",
                entity
            );
        }
    }
}

// ============================================================================
// Property Test 3: Batch Allocation Correctness
// ============================================================================

proptest! {
    #[test]
    fn prop_entity_batch_allocation(batch_size in 1usize..500) {
        let mut allocator = EntityAllocator::new();

        let entities = allocator.allocate_batch(batch_size);

        // Should allocate exactly the requested count
        prop_assert_eq!(entities.len(), batch_size);

        // All entities should be alive
        for entity in &entities {
            prop_assert!(allocator.is_alive(*entity));
        }

        // All entities should be unique
        for i in 0..entities.len() {
            for j in (i + 1)..entities.len() {
                prop_assert_ne!(entities[i], entities[j]);
            }
        }
    }
}

// ============================================================================
// Property Test 4: Entity Generation Increments Correctly
// ============================================================================

proptest! {
    #[test]
    fn prop_entity_generation_increments(cycle_count in 1usize..20) {
        let mut allocator = EntityAllocator::new();

        let first_entity = allocator.allocate();
        let first_id = first_entity.id();
        let mut last_generation = first_entity.generation();

        allocator.free(first_entity);

        for _ in 1..cycle_count {
            let entity = allocator.allocate();

            // Should reuse the same ID
            prop_assert_eq!(entity.id(), first_id);

            // Generation should increment
            prop_assert!(
                entity.generation() > last_generation,
                "Generation should increment: {} -> {}",
                last_generation,
                entity.generation()
            );

            last_generation = entity.generation();
            allocator.free(entity);
        }
    }
}

// ============================================================================
// Property Test 5: Component Add/Get Roundtrip
// ============================================================================

proptest! {
    #[test]
    fn prop_component_add_get_roundtrip(
        entity_count in 1usize..100,
        position in arb_position(),
    ) {
        let mut world = World::new();
        world.register::<Position>();

        let mut entities = Vec::new();
        for _ in 0..entity_count {
            let entity = world.spawn();
            world.add(entity, position);
            entities.push(entity);
        }

        // All entities should have the component
        for entity in &entities {
            let retrieved = world.get::<Position>(*entity);
            prop_assert!(retrieved.is_some());
            prop_assert_eq!(*retrieved.unwrap(), position);
        }
    }
}

// ============================================================================
// Property Test 6: Component Add/Remove Sequences
// ============================================================================

proptest! {
    #[test]
    fn prop_component_add_remove_sequences(
        operations in prop::collection::vec(any::<bool>(), 1..100)
    ) {
        let mut world = World::new();
        world.register::<Position>();

        let entity = world.spawn();
        let mut should_have_component = false;

        for &should_add in &operations {
            if should_add {
                world.add(entity, Position { x: 1.0, y: 2.0, z: 3.0 });
                should_have_component = true;
            } else if should_have_component {
                let removed = world.remove::<Position>(entity);
                prop_assert!(removed.is_some(), "Remove should succeed when component exists");
                should_have_component = false;
            }

            // Verify component state matches expectations
            let has_component = world.has::<Position>(entity);
            prop_assert_eq!(
                has_component,
                should_have_component,
                "Component existence should match expected state"
            );
        }
    }
}

// ============================================================================
// Property Test 7: Multiple Component Types
// ============================================================================

proptest! {
    #[test]
    fn prop_multiple_component_types(
        entity_count in 1usize..50,
        add_position in any::<bool>(),
        add_velocity in any::<bool>(),
        add_health in any::<bool>(),
    ) {
        let mut world = World::new();
        world.register::<Position>();
        world.register::<Velocity>();
        world.register::<Health>();

        let mut entities = Vec::new();
        for _ in 0..entity_count {
            let entity = world.spawn();

            if add_position {
                world.add(entity, Position { x: 0.0, y: 0.0, z: 0.0 });
            }
            if add_velocity {
                world.add(entity, Velocity { x: 1.0, y: 1.0, z: 1.0 });
            }
            if add_health {
                world.add(entity, Health { current: 100.0, max: 100.0 });
            }

            entities.push(entity);
        }

        // Verify all entities have the expected components
        for entity in &entities {
            prop_assert_eq!(world.has::<Position>(*entity), add_position);
            prop_assert_eq!(world.has::<Velocity>(*entity), add_velocity);
            prop_assert_eq!(world.has::<Health>(*entity), add_health);
        }
    }
}

// ============================================================================
// Property Test 8: Entity Despawn Removes All Components
// ============================================================================

proptest! {
    #[test]
    fn prop_despawn_removes_components(entity_count in 1usize..100) {
        let mut world = World::new();
        world.register::<Position>();
        world.register::<Velocity>();
        world.register::<Health>();

        let mut entities = Vec::new();
        for _ in 0..entity_count {
            let entity = world.spawn();
            world.add(entity, Position { x: 0.0, y: 0.0, z: 0.0 });
            world.add(entity, Velocity { x: 1.0, y: 1.0, z: 1.0 });
            world.add(entity, Health { current: 100.0, max: 100.0 });
            entities.push(entity);
        }

        // Despawn all entities
        for entity in &entities {
            let despawned = world.despawn(*entity);
            prop_assert!(despawned, "Despawn should succeed");
            prop_assert!(!world.is_alive(*entity), "Entity should be dead");

            // Components should no longer exist
            prop_assert!(!world.has::<Position>(*entity));
            prop_assert!(!world.has::<Velocity>(*entity));
            prop_assert!(!world.has::<Health>(*entity));
        }

        // Entity count should be zero
        prop_assert_eq!(world.entity_count(), 0);
    }
}

// ============================================================================
// Property Test 9: Component Mutation Correctness
// ============================================================================

proptest! {
    #[test]
    fn prop_component_mutation(
        entity_count in 1usize..50,
        positions in prop::collection::vec(arb_position(), 1..50),
    ) {
        let mut world = World::new();
        world.register::<Position>();

        let mut entities = Vec::new();
        for _ in 0..entity_count.min(positions.len()) {
            let entity = world.spawn();
            world.add(entity, positions[0]);
            entities.push(entity);
        }

        // Mutate each entity with different positions
        for (i, entity) in entities.iter().enumerate() {
            let position_index = i % positions.len();
            let new_position = positions[position_index];

            if let Some(pos) = world.get_mut::<Position>(*entity) {
                *pos = new_position;
            }

            // Verify mutation took effect
            let retrieved = world.get::<Position>(*entity).unwrap();
            prop_assert_eq!(*retrieved, new_position);
        }
    }
}

// ============================================================================
// Property Test 10: World Clear Removes Everything
// ============================================================================

proptest! {
    #[test]
    fn prop_world_clear(entity_count in 1usize..100) {
        let mut world = World::new();
        world.register::<Position>();
        world.register::<Velocity>();

        let mut entities = Vec::new();
        for _ in 0..entity_count {
            let entity = world.spawn();
            world.add(entity, Position { x: 0.0, y: 0.0, z: 0.0 });
            world.add(entity, Velocity { x: 1.0, y: 1.0, z: 1.0 });
            entities.push(entity);
        }

        prop_assert_eq!(world.entity_count(), entity_count);

        // Clear the world
        world.clear();

        // All entities should be dead
        for entity in &entities {
            prop_assert!(!world.is_alive(*entity));
        }

        prop_assert_eq!(world.entity_count(), 0);
    }
}

// ============================================================================
// Property Test 11: Entity Allocator Alive Count
// ============================================================================

proptest! {
    #[test]
    fn prop_allocator_alive_count(
        allocate_count in 1usize..100,
        free_ratio in 0.0f32..1.0,
    ) {
        let mut allocator = EntityAllocator::new();

        // Allocate entities
        let mut entities = Vec::new();
        for _ in 0..allocate_count {
            entities.push(allocator.allocate());
        }

        prop_assert_eq!(allocator.alive_count(), allocate_count);

        // Free a ratio of them
        let to_free = (allocate_count as f32 * free_ratio) as usize;
        for i in 0..to_free {
            allocator.free(entities[i]);
        }

        let expected_alive = allocate_count - to_free;
        prop_assert_eq!(
            allocator.alive_count(),
            expected_alive,
            "Alive count should be {} - {} = {}",
            allocate_count,
            to_free,
            expected_alive
        );
    }
}

// ============================================================================
// Property Test 12: Component Replacement
// ============================================================================

proptest! {
    #[test]
    fn prop_component_replacement(
        entity_count in 1usize..50,
        old_pos in arb_position(),
        new_pos in arb_position(),
    ) {
        let mut world = World::new();
        world.register::<Position>();

        let mut entities = Vec::new();
        for _ in 0..entity_count {
            let entity = world.spawn();
            world.add(entity, old_pos);
            entities.push(entity);
        }

        // Replace component on all entities
        for entity in &entities {
            world.add(*entity, new_pos);

            // Should have new value
            let retrieved = world.get::<Position>(*entity).unwrap();
            prop_assert_eq!(*retrieved, new_pos);
        }
    }
}

// ============================================================================
// Property Test 13: Entity Allocator Clear
// ============================================================================

proptest! {
    #[test]
    fn prop_allocator_clear(entity_count in 1usize..100) {
        let mut allocator = EntityAllocator::new();

        // Allocate entities
        let mut entities = Vec::new();
        for _ in 0..entity_count {
            entities.push(allocator.allocate());
        }

        prop_assert_eq!(allocator.len(), entity_count);
        prop_assert_eq!(allocator.alive_count(), entity_count);

        // Clear allocator
        allocator.clear();

        prop_assert_eq!(allocator.len(), 0);
        prop_assert_eq!(allocator.alive_count(), 0);
        prop_assert!(allocator.is_empty());

        // Old entities should no longer be alive
        for entity in &entities {
            prop_assert!(!allocator.is_alive(*entity));
        }
    }
}
