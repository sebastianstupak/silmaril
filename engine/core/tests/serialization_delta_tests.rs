//! Comprehensive tests for delta compression
//!
//! Tests all edge cases and validates delta compression efficiency.

use engine_core::ecs::World;
use engine_core::gameplay::Health;
use engine_core::physics_components::Velocity;
use engine_core::serialization::{WorldState, WorldStateDelta};

#[test]
fn test_delta_with_no_changes() {
    let mut world = World::new();
    world.register::<Health>();

    for i in 0..100 {
        let e = world.spawn();
        world.add(e, Health::new(i as f32, 100.0));
    }

    let state1 = WorldState::snapshot(&world);
    let state2 = WorldState::snapshot(&world);

    let delta = WorldStateDelta::compute(&state1, &state2);

    // No changes should result in empty delta
    assert_eq!(delta.added_entities.len(), 0, "No entities should be added");
    assert_eq!(delta.removed_entities.len(), 0, "No entities should be removed");
    assert_eq!(delta.modified_components.len(), 0, "No components should be modified");
    assert_eq!(delta.removed_components.len(), 0, "No components should be removed");

    // Delta should be much smaller than full state
    assert!(delta.is_smaller_than(&state2), "Empty delta should be smaller than full state");
}

#[test]
fn test_delta_with_all_entities_added() {
    let world1 = World::new();
    let state1 = WorldState::snapshot(&world1);

    let mut world2 = World::new();
    world2.register::<Health>();

    for i in 0..50 {
        let e = world2.spawn();
        world2.add(e, Health::new(i as f32, 100.0));
    }

    let state2 = WorldState::snapshot(&world2);

    let delta = WorldStateDelta::compute(&state1, &state2);

    // All entities should be added
    assert_eq!(delta.added_entities.len(), 50, "All 50 entities should be in added list");
    assert_eq!(delta.removed_entities.len(), 0);

    // Apply delta to empty state
    let mut state1_copy = state1.clone();
    delta.apply(&mut state1_copy);

    assert_eq!(state1_copy.entities.len(), 50);
    assert_eq!(state1_copy.metadata.entity_count, 50);
}

#[test]
fn test_delta_with_all_entities_removed() {
    let mut world = World::new();
    world.register::<Health>();

    let mut entities = Vec::new();
    for i in 0..50 {
        let e = world.spawn();
        world.add(e, Health::new(i as f32, 100.0));
        entities.push(e);
    }

    let state1 = WorldState::snapshot(&world);

    // Remove all entities
    for entity in entities {
        world.despawn(entity);
    }

    let state2 = WorldState::snapshot(&world);

    let delta = WorldStateDelta::compute(&state1, &state2);

    // All entities should be removed
    assert_eq!(delta.removed_entities.len(), 50, "All 50 entities should be in removed list");
    assert_eq!(delta.added_entities.len(), 0);

    // Apply delta
    let mut state1_copy = state1.clone();
    delta.apply(&mut state1_copy);

    assert_eq!(state1_copy.entities.len(), 0);
    assert_eq!(state1_copy.metadata.entity_count, 0);
}

#[test]
fn test_delta_with_component_modifications() {
    let mut world = World::new();
    world.register::<Health>();

    for _i in 0..100 {
        let e = world.spawn();
        world.add(e, Health::new(100.0, 100.0));
    }

    let state1 = WorldState::snapshot(&world);

    // Modify 10 entities
    let entities_to_modify: Vec<_> = world.entities().take(10).collect();
    for entity in entities_to_modify {
        if let Some(health) = world.get_mut::<Health>(entity) {
            health.current = 50.0;
        }
    }

    let state2 = WorldState::snapshot(&world);

    let delta = WorldStateDelta::compute(&state1, &state2);

    // Should have modified components
    assert!(delta.modified_components.len() > 0, "Should have modified components");
    assert!(
        delta.modified_components.len() <= 10,
        "Should have at most 10 modified entities"
    );

    // Apply delta
    let mut state1_copy = state1.clone();
    delta.apply(&mut state1_copy);

    // Entity count should match
    assert_eq!(state1_copy.entities.len(), state2.entities.len());
}

#[test]
fn test_delta_with_component_additions() {
    let mut world = World::new();
    world.register::<Health>();
    world.register::<Velocity>();

    // Create entities with only Health
    for i in 0..50 {
        let e = world.spawn();
        world.add(e, Health::new(i as f32, 100.0));
    }

    let state1 = WorldState::snapshot(&world);

    // Add Velocity to 25 entities
    let entities_to_add: Vec<_> = world.entities().take(25).collect();
    for entity in entities_to_add {
        world.add(entity, Velocity::new(1.0, 2.0, 3.0));
    }

    let state2 = WorldState::snapshot(&world);

    let delta = WorldStateDelta::compute(&state1, &state2);

    // Should have modified components (additions)
    assert!(delta.modified_components.len() > 0, "Should have component additions");

    // Apply delta
    let mut state1_copy = state1.clone();
    delta.apply(&mut state1_copy);

    // Component count should increase
    assert!(state1_copy.metadata.component_count > state1.metadata.component_count);
}

#[test]
fn test_delta_with_component_removals() {
    let mut world = World::new();
    world.register::<Health>();
    world.register::<Velocity>();

    // Create entities with both components
    for i in 0..50 {
        let e = world.spawn();
        world.add(e, Health::new(i as f32, 100.0));
        world.add(e, Velocity::new(i as f32, 0.0, 0.0));
    }

    let state1 = WorldState::snapshot(&world);

    // Remove Velocity from 25 entities
    let entities_to_remove: Vec<_> = world.entities().take(25).collect();
    for entity in entities_to_remove {
        world.remove::<Velocity>(entity);
    }

    let state2 = WorldState::snapshot(&world);

    let delta = WorldStateDelta::compute(&state1, &state2);

    // Should have removed components
    assert!(delta.removed_components.len() > 0, "Should have component removals");

    // Apply delta
    let mut state1_copy = state1.clone();
    delta.apply(&mut state1_copy);

    // Component count should decrease
    assert!(state1_copy.metadata.component_count < state1.metadata.component_count);
}

#[test]
fn test_delta_with_mixed_changes() {
    let mut world = World::new();
    world.register::<Health>();
    world.register::<Velocity>();

    // Create initial state
    let mut entities = Vec::new();
    for _i in 0..100 {
        let e = world.spawn();
        world.add(e, Health::new(100.0, 100.0));
        entities.push(e);
    }

    let state1 = WorldState::snapshot(&world);

    // Add 20 new entities
    for i in 0..20 {
        let e = world.spawn();
        world.add(e, Health::new(i as f32, 50.0));
        world.add(e, Velocity::new(i as f32, 0.0, 0.0));
    }

    // Remove 10 old entities
    for i in 0..10 {
        world.despawn(entities[i]);
    }

    // Modify 30 entities
    let entities_to_modify: Vec<_> = world.entities().skip(10).take(30).collect();
    for entity in entities_to_modify {
        if let Some(health) = world.get_mut::<Health>(entity) {
            health.current = 75.0;
        }
    }

    let state2 = WorldState::snapshot(&world);

    let delta = WorldStateDelta::compute(&state1, &state2);

    // Should have all types of changes
    assert_eq!(delta.added_entities.len(), 20, "Should have 20 added entities");
    assert_eq!(delta.removed_entities.len(), 10, "Should have 10 removed entities");
    assert!(delta.modified_components.len() > 0, "Should have modified components");

    // Apply delta
    let mut state1_copy = state1.clone();
    delta.apply(&mut state1_copy);

    // Final count: 100 - 10 + 20 = 110
    assert_eq!(state1_copy.entities.len(), 110);
}

#[test]
fn test_delta_efficiency_small_changes() {
    let mut world = World::new();
    world.register::<Health>();

    for _i in 0..1000 {
        let e = world.spawn();
        world.add(e, Health::new(100.0, 100.0));
    }

    let state1 = WorldState::snapshot(&world);

    // Modify only 1% of entities
    let entities_to_modify: Vec<_> = world.entities().take(10).collect();
    for entity in entities_to_modify {
        if let Some(health) = world.get_mut::<Health>(entity) {
            health.current = 50.0;
        }
    }

    let state2 = WorldState::snapshot(&world);

    let delta = WorldStateDelta::compute(&state1, &state2);

    // Delta should be significantly smaller
    let full_size = bincode::serialize(&state2).unwrap().len();
    let delta_size = bincode::serialize(&delta).unwrap().len();

    println!(
        "Full state: {} bytes, Delta: {} bytes, Ratio: {:.1}%",
        full_size,
        delta_size,
        100.0 * delta_size as f64 / full_size as f64
    );

    // Delta should be < 20% of full state for 1% changes
    assert!(
        delta_size < full_size / 5,
        "Delta ({} bytes) should be < 20% of full state ({} bytes)",
        delta_size,
        full_size
    );
}

#[test]
fn test_delta_efficiency_many_changes() {
    let mut world = World::new();
    world.register::<Health>();

    for _i in 0..1000 {
        let e = world.spawn();
        world.add(e, Health::new(100.0, 100.0));
    }

    let state1 = WorldState::snapshot(&world);

    // Modify 90% of entities
    let entities_to_modify: Vec<_> = world.entities().take(900).collect();
    for entity in entities_to_modify {
        if let Some(health) = world.get_mut::<Health>(entity) {
            health.current = 50.0;
        }
    }

    let state2 = WorldState::snapshot(&world);

    let delta = WorldStateDelta::compute(&state1, &state2);

    // With many changes, full state might be more efficient
    // But delta should still work correctly
    let full_size = bincode::serialize(&state2).unwrap().len();
    let delta_size = bincode::serialize(&delta).unwrap().len();

    println!("Full state: {} bytes, Delta: {} bytes (90% changed)", full_size, delta_size);

    // Delta should still apply correctly even if it's not smaller
    let mut state1_copy = state1.clone();
    delta.apply(&mut state1_copy);

    assert_eq!(state1_copy.entities.len(), state2.entities.len());
}

#[test]
fn test_delta_apply_is_commutative() {
    let mut world = World::new();
    world.register::<Health>();

    for i in 0..50 {
        let e = world.spawn();
        world.add(e, Health::new(i as f32, 100.0));
    }

    let state1 = WorldState::snapshot(&world);

    // Make changes
    let entities_to_modify: Vec<_> = world.entities().take(25).collect();
    for entity in entities_to_modify {
        if let Some(health) = world.get_mut::<Health>(entity) {
            health.current = 50.0;
        }
    }

    let state2 = WorldState::snapshot(&world);

    let delta = WorldStateDelta::compute(&state1, &state2);

    // Apply delta multiple ways
    let mut result1 = state1.clone();
    delta.apply(&mut result1);

    let mut result2 = state1.clone();
    delta.apply(&mut result2);

    // Both should be identical
    assert_eq!(result1.entities.len(), result2.entities.len());
    assert_eq!(result1.metadata.component_count, result2.metadata.component_count);
}

#[test]
fn test_delta_versioning() {
    let mut world = World::new();
    world.register::<Health>();

    for i in 0..10 {
        let e = world.spawn();
        world.add(e, Health::new(i as f32, 100.0));
    }

    let state1 = WorldState::snapshot(&world);

    let all_entities: Vec<_> = world.entities().collect();
    for entity in all_entities {
        if let Some(health) = world.get_mut::<Health>(entity) {
            health.current = 50.0;
        }
    }

    let state2 = WorldState::snapshot(&world);

    let delta = WorldStateDelta::compute(&state1, &state2);

    // Versions should be tracked
    assert_eq!(delta.base_version, state1.metadata.version);
    assert_eq!(delta.target_version, state2.metadata.version);

    // After applying delta, version should update
    let mut state1_copy = state1.clone();
    delta.apply(&mut state1_copy);

    assert_eq!(state1_copy.metadata.version, delta.target_version);
}

#[test]
fn test_delta_with_entity_id_reuse() {
    let mut world = World::new();
    world.register::<Health>();

    // Create and remove some entities to populate free list
    let mut entities = Vec::new();
    for i in 0..20 {
        let e = world.spawn();
        world.add(e, Health::new(i as f32, 100.0));
        entities.push(e);
    }

    // Remove half
    for i in 0..10 {
        world.despawn(entities[i]);
    }

    let state1 = WorldState::snapshot(&world);

    // Spawn new entities (will reuse IDs)
    for i in 0..15 {
        let e = world.spawn();
        world.add(e, Health::new((i + 100) as f32, 100.0));
    }

    let state2 = WorldState::snapshot(&world);

    let delta = WorldStateDelta::compute(&state1, &state2);

    // Should handle ID reuse correctly
    assert_eq!(delta.added_entities.len(), 15);

    // Apply delta
    let mut state1_copy = state1.clone();
    delta.apply(&mut state1_copy);

    // Should have correct final count: 10 + 15 = 25
    assert_eq!(state1_copy.entities.len(), 25);
}

#[test]
fn test_delta_serialize_roundtrip() {
    let mut world = World::new();
    world.register::<Health>();

    for _i in 0..100 {
        let e = world.spawn();
        world.add(e, Health::new(100.0, 100.0));
    }

    let state1 = WorldState::snapshot(&world);

    let entities_to_modify: Vec<_> = world.entities().take(50).collect();
    for entity in entities_to_modify {
        if let Some(health) = world.get_mut::<Health>(entity) {
            health.current = 50.0;
        }
    }

    let state2 = WorldState::snapshot(&world);

    let delta = WorldStateDelta::compute(&state1, &state2);

    // Serialize and deserialize delta
    let delta_bytes = bincode::serialize(&delta).unwrap();
    let delta_restored: WorldStateDelta = bincode::deserialize(&delta_bytes).unwrap();

    // Apply both deltas and verify they produce the same result
    let mut result1 = state1.clone();
    delta.apply(&mut result1);

    let mut result2 = state1.clone();
    delta_restored.apply(&mut result2);

    assert_eq!(result1.entities.len(), result2.entities.len());
    assert_eq!(result1.metadata.component_count, result2.metadata.component_count);
}
