//! Property-based tests for serialization
//!
//! Uses proptest to validate serialization invariants with randomly generated data.

use engine_core::ecs::World;
use engine_core::gameplay::Health;
use engine_core::math::Transform;
use engine_core::physics_components::Velocity;
use engine_core::serialization::{Format, Serializable, WorldState, WorldStateDelta};
use proptest::prelude::*;

/// Strategy for generating valid Health components
fn health_strategy() -> impl Strategy<Value = Health> {
    (0.0f32..=1000.0, 0.0f32..=1000.0).prop_map(|(current, max)| {
        let actual_max = max.max(1.0); // Ensure max > 0
        let actual_current = current.min(actual_max); // Ensure current <= max
        Health::new(actual_current, actual_max)
    })
}

/// Strategy for generating valid Velocity components
fn velocity_strategy() -> impl Strategy<Value = Velocity> {
    (-1000.0f32..=1000.0, -1000.0f32..=1000.0, -1000.0f32..=1000.0)
        .prop_map(|(x, y, z)| Velocity::new(x, y, z))
}

/// Strategy for generating valid Transform components
#[allow(dead_code)]
fn transform_strategy() -> impl Strategy<Value = Transform> {
    Just(Transform::default()) // Use default for now, can be extended
}

proptest! {
    /// Test that YAML serialization is a proper roundtrip
    #[test]
    fn test_yaml_roundtrip_health(health in health_strategy()) {
        let mut world = World::new();
        world.register::<Health>();

        let entity = world.spawn();
        world.add(entity, health);

        let snapshot = WorldState::snapshot(&world);
        let yaml = snapshot.serialize(Format::Yaml).unwrap();
        let restored = WorldState::deserialize(&yaml, Format::Yaml).unwrap();

        prop_assert_eq!(snapshot.entities.len(), restored.entities.len());
        prop_assert_eq!(snapshot.metadata.component_count, restored.metadata.component_count);
    }

    /// Test that Bincode serialization is a proper roundtrip
    #[test]
    fn test_bincode_roundtrip_health(health in health_strategy()) {
        let mut world = World::new();
        world.register::<Health>();

        let entity = world.spawn();
        world.add(entity, health);

        let snapshot = WorldState::snapshot(&world);
        let bytes = snapshot.serialize(Format::Bincode).unwrap();
        let restored = WorldState::deserialize(&bytes, Format::Bincode).unwrap();

        prop_assert_eq!(snapshot.entities.len(), restored.entities.len());
        prop_assert_eq!(snapshot.metadata.component_count, restored.metadata.component_count);
    }

    /// Test that snapshot and restore preserves entity count
    #[test]
    fn test_snapshot_preserves_entity_count(entity_count in 0usize..100) {
        let mut world = World::new();
        world.register::<Transform>();

        for _ in 0..entity_count {
            let e = world.spawn();
            world.add(e, Transform::default());
        }

        let snapshot = WorldState::snapshot(&world);
        prop_assert_eq!(snapshot.entities.len(), entity_count);
        prop_assert_eq!(snapshot.metadata.entity_count, entity_count);

        let mut world2 = World::new();
        world2.register::<Transform>();
        snapshot.restore(&mut world2);

        prop_assert_eq!(world2.entity_count(), entity_count);
    }

    /// Test that velocity components survive roundtrip
    #[test]
    fn test_velocity_roundtrip(velocity in velocity_strategy()) {
        let mut world = World::new();
        world.register::<Velocity>();

        let entity = world.spawn();
        world.add(entity, velocity);

        let snapshot = WorldState::snapshot(&world);
        let bytes = snapshot.serialize(Format::Bincode).unwrap();
        let restored = WorldState::deserialize(&bytes, Format::Bincode).unwrap();

        let mut world2 = World::new();
        world2.register::<Velocity>();
        restored.restore(&mut world2);

        // Entity should exist
        prop_assert!(world2.is_alive(entity));
        prop_assert!(world2.has::<Velocity>(entity));

        // Velocity should match
        let restored_vel = world2.get::<Velocity>(entity).unwrap();
        prop_assert!((restored_vel.x - velocity.x).abs() < 0.001);
        prop_assert!((restored_vel.y - velocity.y).abs() < 0.001);
        prop_assert!((restored_vel.z - velocity.z).abs() < 0.001);
    }

    /// Test delta compression with random entity counts
    #[test]
    fn test_delta_with_random_entities(
        initial_count in 0usize..50,
        added_count in 0usize..50,
        removed_count in 0usize..50,
    ) {
        let mut world1 = World::new();
        world1.register::<Health>();

        // Create initial entities
        let mut entities = Vec::new();
        for _ in 0..initial_count {
            let e = world1.spawn();
            world1.add(e, Health::new(100.0, 100.0));
            entities.push(e);
        }

        let state1 = WorldState::snapshot(&world1);

        // Add new entities
        for _ in 0..added_count {
            let e = world1.spawn();
            world1.add(e, Health::new(50.0, 100.0));
        }

        // Remove some entities
        let to_remove = removed_count.min(entities.len());
        for i in 0..to_remove {
            world1.despawn(entities[i]);
        }

        let state2 = WorldState::snapshot(&world1);

        // Compute delta
        let delta = WorldStateDelta::compute(&state1, &state2);

        // Apply delta to a copy of state1
        let mut state1_copy = state1.clone();
        delta.apply(&mut state1_copy);

        // Should match state2
        prop_assert_eq!(state1_copy.entities.len(), state2.entities.len());
    }

    /// Test that empty world serialization works
    #[test]
    fn test_empty_world_roundtrip(_dummy in 0u32..10) {
        let world = World::new();
        let snapshot = WorldState::snapshot(&world);

        let yaml = snapshot.serialize(Format::Yaml).unwrap();
        let restored = WorldState::deserialize(&yaml, Format::Yaml).unwrap();

        prop_assert_eq!(restored.entities.len(), 0);
        prop_assert_eq!(restored.metadata.entity_count, 0);
    }

    /// Test that component count is preserved
    #[test]
    fn test_component_count_preserved(
        entity_count in 1usize..50,
        components_per_entity in 1usize..4,
    ) {
        let mut world = World::new();
        world.register::<Transform>();
        world.register::<Health>();
        world.register::<Velocity>();

        for _ in 0..entity_count {
            let e = world.spawn();

            // Add components based on count
            if components_per_entity >= 1 {
                world.add(e, Transform::default());
            }
            if components_per_entity >= 2 {
                world.add(e, Health::new(100.0, 100.0));
            }
            if components_per_entity >= 3 {
                world.add(e, Velocity::new(0.0, 0.0, 0.0));
            }
        }

        let snapshot = WorldState::snapshot(&world);
        let expected_count = entity_count * components_per_entity.min(3);

        prop_assert_eq!(snapshot.metadata.component_count, expected_count);

        // Roundtrip should preserve count
        let bytes = snapshot.serialize(Format::Bincode).unwrap();
        let restored = WorldState::deserialize(&bytes, Format::Bincode).unwrap();

        prop_assert_eq!(restored.metadata.component_count, expected_count);
    }

    /// Test that serialization is deterministic
    #[test]
    fn test_serialization_deterministic(health in health_strategy()) {
        let mut world = World::new();
        world.register::<Health>();

        let entity = world.spawn();
        world.add(entity, health);

        let snapshot = WorldState::snapshot(&world);

        // Serialize twice
        let bytes1 = snapshot.serialize(Format::Bincode).unwrap();
        let bytes2 = snapshot.serialize(Format::Bincode).unwrap();

        // Should be identical
        prop_assert_eq!(bytes1, bytes2);
    }

    /// Test that delta is idempotent
    #[test]
    fn test_delta_idempotent(entity_count in 1usize..20) {
        let mut world1 = World::new();
        world1.register::<Health>();

        for _ in 0..entity_count {
            let e = world1.spawn();
            world1.add(e, Health::new(100.0, 100.0));
        }

        let state1 = WorldState::snapshot(&world1);

        // Modify some entities
        let entities_to_modify: Vec<_> = world1.entities().take(entity_count / 2).collect();
        for entity in entities_to_modify {
            if let Some(health) = world1.get_mut::<Health>(entity) {
                health.current = 50.0;
            }
        }

        let state2 = WorldState::snapshot(&world1);

        let delta = WorldStateDelta::compute(&state1, &state2);

        // Apply delta once
        let mut state1_copy1 = state1.clone();
        delta.apply(&mut state1_copy1);

        // Apply delta again (should be no-op)
        let mut state1_copy2 = state1_copy1.clone();
        delta.apply(&mut state1_copy2);

        // Both should be identical
        prop_assert_eq!(state1_copy1.entities.len(), state1_copy2.entities.len());
        prop_assert_eq!(
            state1_copy1.metadata.component_count,
            state1_copy2.metadata.component_count
        );
    }
}

#[cfg(test)]
mod manual_property_tests {
    use super::*;

    #[test]
    fn test_large_health_values() {
        let health = Health::new(999999.0, 1000000.0);

        let mut world = World::new();
        world.register::<Health>();
        let entity = world.spawn();
        world.add(entity, health);

        let snapshot = WorldState::snapshot(&world);
        let bytes = snapshot.serialize(Format::Bincode).unwrap();
        let restored = WorldState::deserialize(&bytes, Format::Bincode).unwrap();

        assert_eq!(snapshot.entities.len(), restored.entities.len());
    }

    #[test]
    fn test_zero_health() {
        let health = Health::new(0.0, 100.0);

        let mut world = World::new();
        world.register::<Health>();
        let entity = world.spawn();
        world.add(entity, health);

        let snapshot = WorldState::snapshot(&world);
        let bytes = snapshot.serialize(Format::Bincode).unwrap();
        let restored = WorldState::deserialize(&bytes, Format::Bincode).unwrap();

        assert_eq!(snapshot.entities.len(), restored.entities.len());
    }

    #[test]
    fn test_negative_velocity() {
        let velocity = Velocity::new(-100.0, -200.0, -300.0);

        let mut world = World::new();
        world.register::<Velocity>();
        let entity = world.spawn();
        world.add(entity, velocity);

        let snapshot = WorldState::snapshot(&world);
        let bytes = snapshot.serialize(Format::Bincode).unwrap();
        let restored = WorldState::deserialize(&bytes, Format::Bincode).unwrap();

        assert_eq!(snapshot.entities.len(), restored.entities.len());
    }
}
