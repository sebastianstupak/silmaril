//! Property-based tests for serialization
//!
//! These tests use proptest to verify correctness properties across a wide range of inputs:
//! - Serialization roundtrip correctness
//! - Delta encoding/decoding correctness
//! - Format consistency (YAML, Bincode)

use engine_core::ecs::EntityAllocator;
use engine_core::serialization::{
    ComponentData, Format, Serializable, WorldState, WorldStateDelta,
};
use engine_core::{Health, Transform, Vec3, Velocity};
use proptest::prelude::*;

// ============================================================================
// Custom Strategies for Component Generation
// ============================================================================

/// Generate a random Transform component with reasonable bounds
fn arb_transform() -> impl Strategy<Value = Transform> {
    (
        prop::array::uniform3(-1000.0f32..1000.0),
        prop::array::uniform4(-1.0f32..1.0),
        prop::array::uniform3(0.01f32..100.0),
    )
        .prop_map(|(pos, rot, scale)| {
            let mut transform = Transform::default();
            transform.position = Vec3::new(pos[0], pos[1], pos[2]);
            // Normalize quaternion to ensure validity
            let quat_len = (rot[0] * rot[0] + rot[1] * rot[1] + rot[2] * rot[2] + rot[3] * rot[3])
                .sqrt()
                .max(0.0001);
            transform.rotation.x = rot[0] / quat_len;
            transform.rotation.y = rot[1] / quat_len;
            transform.rotation.z = rot[2] / quat_len;
            transform.rotation.w = rot[3] / quat_len;
            transform.scale = Vec3::new(scale[0], scale[1], scale[2]);
            transform
        })
}

/// Generate a random Health component
fn arb_health() -> impl Strategy<Value = Health> {
    (1.0f32..1000.0, 0.0f32..1.0)
        .prop_map(|(max, current_ratio)| Health::new(max * current_ratio, max))
}

/// Generate a random Velocity component
fn arb_velocity() -> impl Strategy<Value = Velocity> {
    prop::array::uniform3(-100.0f32..100.0).prop_map(|vel| Velocity::new(vel[0], vel[1], vel[2]))
}

/// Generate a random ComponentData variant
fn arb_component_data() -> impl Strategy<Value = ComponentData> {
    prop_oneof![
        arb_transform().prop_map(ComponentData::Transform),
        arb_health().prop_map(ComponentData::Health),
        arb_velocity().prop_map(ComponentData::Velocity),
    ]
}

/// Generate a vector of random components (avoiding duplicates by type)
fn arb_component_vec() -> impl Strategy<Value = Vec<ComponentData>> {
    prop::collection::vec(arb_component_data(), 0..4).prop_map(|mut components| {
        // Deduplicate by type (keep only one of each type)
        let mut seen_types = std::collections::HashSet::new();
        components.retain(|c| seen_types.insert(std::any::TypeId::from(c.type_id())));
        components
    })
}

// ============================================================================
// Property Test 1: Transform Serialization Roundtrip
// ============================================================================

proptest! {
    #[test]
    fn prop_transform_bincode_roundtrip(transform in arb_transform()) {
        let component_data = ComponentData::Transform(transform);

        // Serialize
        let bytes = bincode::serialize(&component_data).expect("serialization should succeed");

        // Deserialize
        let decoded: ComponentData = bincode::deserialize(&bytes).expect("deserialization should succeed");

        // Verify roundtrip
        prop_assert_eq!(component_data, decoded);
    }
}

// ============================================================================
// Property Test 2: Health Serialization Roundtrip
// ============================================================================

proptest! {
    #[test]
    fn prop_health_bincode_roundtrip(health in arb_health()) {
        let component_data = ComponentData::Health(health);

        let bytes = bincode::serialize(&component_data).expect("serialization should succeed");
        let decoded: ComponentData = bincode::deserialize(&bytes).expect("deserialization should succeed");

        prop_assert_eq!(component_data, decoded);
    }
}

// ============================================================================
// Property Test 3: Velocity Serialization Roundtrip
// ============================================================================

proptest! {
    #[test]
    fn prop_velocity_bincode_roundtrip(velocity in arb_velocity()) {
        let component_data = ComponentData::Velocity(velocity);

        let bytes = bincode::serialize(&component_data).expect("serialization should succeed");
        let decoded: ComponentData = bincode::deserialize(&bytes).expect("deserialization should succeed");

        prop_assert_eq!(component_data, decoded);
    }
}

// ============================================================================
// Property Test 4: WorldState YAML Roundtrip with Random Entity Counts
// ============================================================================

proptest! {
    #[test]
    fn prop_world_state_yaml_roundtrip(entity_count in 1usize..50) {
        let mut world_state = WorldState::new();
        let mut allocator = EntityAllocator::new();

        // Add random entities with metadata
        for _ in 0..entity_count {
            let entity = allocator.allocate();
            world_state.entities.push(engine_core::serialization::EntityMetadata {
                entity,
                generation: entity.generation(),
                alive: true,
            });
            world_state.components.insert(entity, vec![
                ComponentData::Transform(Transform::default()),
            ]);
        }

        world_state.metadata.entity_count = entity_count;
        world_state.metadata.component_count = entity_count;

        // Serialize to YAML
        let yaml_bytes = Serializable::serialize(&world_state, Format::Yaml)
            .expect("YAML serialization should succeed");

        // Deserialize from YAML
        let decoded = <WorldState as Serializable>::deserialize(&yaml_bytes, Format::Yaml)
            .expect("YAML deserialization should succeed");

        // Verify entity counts match
        prop_assert_eq!(world_state.entities.len(), decoded.entities.len());
        prop_assert_eq!(world_state.metadata.entity_count, decoded.metadata.entity_count);
        prop_assert_eq!(world_state.metadata.version, decoded.metadata.version);
    }
}

// ============================================================================
// Property Test 5: WorldState Bincode Roundtrip with Random Entity Counts
// ============================================================================

proptest! {
    #[test]
    fn prop_world_state_bincode_roundtrip(entity_count in 1usize..100) {
        let mut world_state = WorldState::new();
        let mut allocator = EntityAllocator::new();

        for _ in 0..entity_count {
            let entity = allocator.allocate();
            world_state.entities.push(engine_core::serialization::EntityMetadata {
                entity,
                generation: entity.generation(),
                alive: true,
            });
            world_state.components.insert(entity, vec![
                ComponentData::Transform(Transform::default()),
            ]);
        }

        world_state.metadata.entity_count = entity_count;
        world_state.metadata.component_count = entity_count;

        // Serialize to Bincode
        let bytes = Serializable::serialize(&world_state, Format::Bincode)
            .expect("Bincode serialization should succeed");

        // Deserialize from Bincode
        let decoded = <WorldState as Serializable>::deserialize(&bytes, Format::Bincode)
            .expect("Bincode deserialization should succeed");

        // Verify entity counts match
        prop_assert_eq!(world_state.entities.len(), decoded.entities.len());
        prop_assert_eq!(world_state.metadata.entity_count, decoded.metadata.entity_count);
    }
}

// ============================================================================
// Property Test 6: WorldState Bincode Roundtrip with Large Entity Counts
// ============================================================================

proptest! {
    #[test]
    fn prop_world_state_large_entity_roundtrip(entity_count in 100usize..1000) {
        let mut world_state = WorldState::new();
        let mut allocator = EntityAllocator::new();

        for _ in 0..entity_count {
            let entity = allocator.allocate();
            world_state.entities.push(engine_core::serialization::EntityMetadata {
                entity,
                generation: entity.generation(),
                alive: true,
            });
            world_state.components.insert(entity, vec![
                ComponentData::Transform(Transform::default()),
            ]);
        }

        world_state.metadata.entity_count = entity_count;
        world_state.metadata.component_count = entity_count;

        let bytes = Serializable::serialize(&world_state, Format::Bincode)
            .expect("Bincode serialization should succeed");
        let decoded = <WorldState as Serializable>::deserialize(&bytes, Format::Bincode)
            .expect("Bincode deserialization should succeed");

        prop_assert_eq!(world_state.entities.len(), decoded.entities.len());
    }
}

// ============================================================================
// Property Test 7: ComponentData Vector Serialization
// ============================================================================

proptest! {
    #[test]
    fn prop_component_vec_roundtrip(components in arb_component_vec()) {
        let bytes = bincode::serialize(&components).expect("serialization should succeed");
        let decoded: Vec<ComponentData> = bincode::deserialize(&bytes)
            .expect("deserialization should succeed");

        prop_assert_eq!(components, decoded);
    }
}

// ============================================================================
// Property Test 8: Delta Encoding Correctness (Empty to Populated)
// ============================================================================

proptest! {
    #[test]
    fn prop_delta_empty_to_populated(entity_count in 1usize..50) {
        let old_state = WorldState::new();
        let mut new_state = WorldState::new();
        let mut allocator = EntityAllocator::new();

        // Populate new state
        for _ in 0..entity_count {
            let entity = allocator.allocate();
            new_state.entities.push(engine_core::serialization::EntityMetadata {
                entity,
                generation: entity.generation(),
                alive: true,
            });
            new_state.components.insert(entity, vec![
                ComponentData::Transform(Transform::default()),
            ]);
        }
        new_state.metadata.version = 2;
        new_state.metadata.entity_count = entity_count;

        // Compute delta
        let delta = WorldStateDelta::compute(&old_state, &new_state);

        // Delta should show all entities as added
        prop_assert_eq!(delta.added_entities.len(), entity_count);
        prop_assert_eq!(delta.removed_entities.len(), 0);

        // Apply delta to old state
        let mut reconstructed = old_state.clone();
        delta.apply(&mut reconstructed);

        // Verify reconstruction matches new state
        prop_assert_eq!(reconstructed.entities.len(), new_state.entities.len());
        prop_assert_eq!(reconstructed.metadata.version, new_state.metadata.version);
    }
}

// ============================================================================
// Property Test 9: Delta Encoding Correctness (Populated to Empty)
// ============================================================================

proptest! {
    #[test]
    fn prop_delta_populated_to_empty(entity_count in 1usize..50) {
        let mut old_state = WorldState::new();
        let mut allocator = EntityAllocator::new();

        // Populate old state
        for _ in 0..entity_count {
            let entity = allocator.allocate();
            old_state.entities.push(engine_core::serialization::EntityMetadata {
                entity,
                generation: entity.generation(),
                alive: true,
            });
            old_state.components.insert(entity, vec![
                ComponentData::Transform(Transform::default()),
            ]);
        }
        old_state.metadata.version = 1;
        old_state.metadata.entity_count = entity_count;

        let mut new_state = WorldState::new();
        new_state.metadata.version = 2;

        // Compute delta
        let delta = WorldStateDelta::compute(&old_state, &new_state);

        // Delta should show all entities as removed
        prop_assert_eq!(delta.added_entities.len(), 0);
        prop_assert_eq!(delta.removed_entities.len(), entity_count);

        // Apply delta
        let mut reconstructed = old_state.clone();
        delta.apply(&mut reconstructed);

        // Verify reconstruction is empty
        prop_assert_eq!(reconstructed.entities.len(), 0);
        prop_assert_eq!(reconstructed.metadata.version, new_state.metadata.version);
    }
}

// ============================================================================
// Property Test 10: Delta Encoding Idempotence
// ============================================================================

proptest! {
    #[test]
    fn prop_delta_idempotence(entity_count in 1usize..30) {
        let mut state = WorldState::new();
        let mut allocator = EntityAllocator::new();

        for _ in 0..entity_count {
            let entity = allocator.allocate();
            state.entities.push(engine_core::serialization::EntityMetadata {
                entity,
                generation: entity.generation(),
                alive: true,
            });
            state.components.insert(entity, vec![
                ComponentData::Transform(Transform::default()),
            ]);
        }
        state.metadata.entity_count = entity_count;

        // Delta from state to itself should be minimal
        let delta = WorldStateDelta::compute(&state, &state);

        // No changes should be detected
        prop_assert_eq!(delta.added_entities.len(), 0);
        prop_assert_eq!(delta.removed_entities.len(), 0);

        // Applying this delta should not change the state
        let mut reconstructed = state.clone();
        delta.apply(&mut reconstructed);

        prop_assert_eq!(reconstructed.entities.len(), state.entities.len());
    }
}

// ============================================================================
// Property Test 11: Delta Serialization Roundtrip
// ============================================================================

proptest! {
    #[test]
    fn prop_delta_serialization_roundtrip(
        old_count in 0usize..30,
        new_count in 0usize..30,
    ) {
        let mut old_state = WorldState::new();
        let mut new_state = WorldState::new();
        let mut old_allocator = EntityAllocator::new();
        let mut new_allocator = EntityAllocator::new();

        for _ in 0..old_count {
            let entity = old_allocator.allocate();
            old_state.entities.push(engine_core::serialization::EntityMetadata {
                entity,
                generation: entity.generation(),
                alive: true,
            });
        }

        for _ in 0..new_count {
            let entity = new_allocator.allocate();
            new_state.entities.push(engine_core::serialization::EntityMetadata {
                entity,
                generation: entity.generation(),
                alive: true,
            });
        }

        new_state.metadata.version = 2;

        // Compute delta
        let delta = WorldStateDelta::compute(&old_state, &new_state);

        // Serialize delta
        let bytes = bincode::serialize(&delta).expect("delta serialization should succeed");

        // Deserialize delta
        let decoded: WorldStateDelta = bincode::deserialize(&bytes)
            .expect("delta deserialization should succeed");

        // Verify roundtrip
        prop_assert_eq!(delta.base_version, decoded.base_version);
        prop_assert_eq!(delta.target_version, decoded.target_version);
        prop_assert_eq!(delta.added_entities.len(), decoded.added_entities.len());
        prop_assert_eq!(delta.removed_entities.len(), decoded.removed_entities.len());
    }
}
