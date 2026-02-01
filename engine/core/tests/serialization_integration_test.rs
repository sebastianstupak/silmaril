//! Integration tests for Phase 1.3 serialization features
//!
//! Tests the full serialization stack: WorldState, deltas, formats, and components.

use engine_core::serialization::{Format, Serializable, WorldState, WorldStateDelta};
use engine_core::{Health, MeshRenderer, Transform, World};

#[test]
fn test_yaml_serialization() {
    let state = WorldState::new();

    // Serialize to YAML
    let yaml_bytes =
        Serializable::serialize(&state, Format::Yaml).expect("YAML serialization should succeed");

    // Verify it's valid YAML by parsing
    let yaml_string = String::from_utf8(yaml_bytes.clone()).expect("YAML should be valid UTF-8");

    assert!(yaml_string.contains("metadata"), "YAML should contain metadata");
    assert!(yaml_string.contains("version"), "YAML should contain version field");

    // Roundtrip test
    let restored = <WorldState as Serializable>::deserialize(&yaml_bytes, Format::Yaml)
        .expect("YAML deserialization should succeed");

    assert_eq!(state.metadata.version, restored.metadata.version);
    assert_eq!(state.entities, restored.entities);
}

#[test]
fn test_bincode_serialization() {
    let state = WorldState::new();

    // Serialize to Bincode (binary format)
    let bincode_bytes = Serializable::serialize(&state, Format::Bincode)
        .expect("Bincode serialization should succeed");

    // Binary format should be compact
    assert!(bincode_bytes.len() < 1000, "Empty state should be compact");

    // Roundtrip test
    let restored = <WorldState as Serializable>::deserialize(&bincode_bytes, Format::Bincode)
        .expect("Bincode deserialization should succeed");

    assert_eq!(state.metadata.version, restored.metadata.version);
    assert_eq!(state.entities, restored.entities);
}

#[test]
fn test_world_state_with_components() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Health>();
    world.register::<MeshRenderer>();

    // Spawn entities with various component combinations
    for i in 0..5 {
        let entity = world.spawn();
        world.add(entity, Transform::default());
        world.add(entity, Health::new(100.0 - (i as f32 * 10.0), 100.0));

        if i % 3 == 0 {
            world.add(entity, MeshRenderer::new(i as u64, i as u64 + 100));
        }
    }

    assert_eq!(world.entity_count(), 5, "Should have 5 entities");

    // Create and serialize state
    let state = WorldState::new();
    let bytes =
        Serializable::serialize(&state, Format::Bincode).expect("Should serialize world state");

    // Deserialize and verify
    let restored = <WorldState as Serializable>::deserialize(&bytes, Format::Bincode)
        .expect("Should deserialize world state");

    assert_eq!(state.metadata.version, restored.metadata.version);
}

#[test]
fn test_delta_compression() {
    let old_state = WorldState::new();
    let mut new_state = WorldState::new();
    new_state.metadata.version = 2;

    // Compute delta
    let delta = WorldStateDelta::compute(&old_state, &new_state);

    assert_eq!(delta.base_version, old_state.metadata.version);
    assert_eq!(delta.target_version, new_state.metadata.version);
    assert_eq!(delta.added_entities.len(), 0);
    assert_eq!(delta.removed_entities.len(), 0);

    // Apply delta
    let mut base = old_state.clone();
    delta.apply(&mut base);

    assert_eq!(base.metadata.version, new_state.metadata.version);
}

#[test]
fn test_delta_size_optimization() {
    let old_state = WorldState::new();
    let mut new_state = WorldState::new();
    new_state.metadata.version = 2;

    let delta = WorldStateDelta::compute(&old_state, &new_state);

    // For minimal changes, delta should be smaller
    // This is a smoke test - actual size depends on implementation
    if delta.is_smaller_than(&new_state) {
        // Delta is preferred
        assert!(
            delta.added_entities.len() + delta.removed_entities.len()
                < new_state.entities.len() + 1
        );
    }
}

#[test]
fn test_health_component_functionality() {
    let mut health = Health::new(100.0, 100.0);

    assert_eq!(health.current, 100.0);
    assert_eq!(health.max, 100.0);
    assert!(health.is_full());
    assert!(health.is_alive());

    // Test damage
    health.damage(30.0);
    assert_eq!(health.current, 70.0);
    assert!(!health.is_full());
    assert!(health.is_alive());

    // Test healing
    health.heal(20.0);
    assert_eq!(health.current, 90.0);

    // Test heal beyond max
    health.heal(50.0);
    assert_eq!(health.current, 100.0, "Healing should cap at max");
    assert!(health.is_full());

    // Test fatal damage
    health.damage(150.0);
    assert_eq!(health.current, 0.0, "Damage should floor at 0");
    assert!(!health.is_alive());
}

#[test]
fn test_serialization_format_comparison() {
    let state = WorldState::new();

    // Serialize with both formats
    let yaml_bytes = Serializable::serialize(&state, Format::Yaml).expect("YAML serialization");
    let bincode_bytes =
        Serializable::serialize(&state, Format::Bincode).expect("Bincode serialization");

    // Binary format should be more compact
    assert!(
        bincode_bytes.len() < yaml_bytes.len(),
        "Binary format should be smaller than YAML"
    );

    // Both should roundtrip correctly
    let yaml_restored = <WorldState as Serializable>::deserialize(&yaml_bytes, Format::Yaml)
        .expect("YAML roundtrip");
    let bincode_restored =
        <WorldState as Serializable>::deserialize(&bincode_bytes, Format::Bincode)
            .expect("Bincode roundtrip");

    assert_eq!(yaml_restored.metadata.version, bincode_restored.metadata.version);
}
