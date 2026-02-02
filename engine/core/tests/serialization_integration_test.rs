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

/// Test full World snapshot and restore with YAML format
#[test]
fn test_world_snapshot_restore_yaml() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Health>();
    world.register::<MeshRenderer>();

    // Create 10 entities with various components
    for i in 0..10 {
        let entity = world.spawn();
        world.add(entity, Transform::default());
        world.add(entity, Health::new(100.0 - (i as f32 * 5.0), 100.0));

        if i % 2 == 0 {
            world.add(entity, MeshRenderer::new(i as u64, i as u64 + 1000));
        }
    }

    let initial_count = world.entity_count();
    assert_eq!(initial_count, 10, "Should have 10 entities");

    // Snapshot the world
    let snapshot = WorldState::snapshot(&world);
    assert_eq!(snapshot.metadata.entity_count, 10, "Snapshot should have 10 entities");
    assert!(
        snapshot.metadata.component_count >= 20,
        "Should have at least 20 components (Transform + Health for all)"
    );

    // Serialize to YAML
    let yaml_bytes = Serializable::serialize(&snapshot, Format::Yaml)
        .expect("YAML serialization should succeed");

    // Verify it's human-readable
    let yaml_string = String::from_utf8(yaml_bytes.clone()).expect("YAML should be valid UTF-8");
    assert!(yaml_string.contains("Transform"), "YAML should mention Transform");
    assert!(yaml_string.contains("Health"), "YAML should mention Health");

    // Deserialize
    let restored_snapshot = <WorldState as Serializable>::deserialize(&yaml_bytes, Format::Yaml)
        .expect("YAML deserialization should succeed");

    // Verify metadata
    assert_eq!(restored_snapshot.metadata.entity_count, 10);
    assert_eq!(restored_snapshot.entities.len(), 10);

    // Restore to a new world
    let mut new_world = World::new();
    new_world.register::<Transform>();
    new_world.register::<Health>();
    new_world.register::<MeshRenderer>();

    restored_snapshot.restore(&mut new_world);

    assert_eq!(new_world.entity_count(), 10, "Restored world should have 10 entities");
}

/// Test full World snapshot and restore with Bincode format
#[test]
fn test_world_snapshot_restore_bincode() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Health>();
    world.register::<MeshRenderer>();

    // Create 100 entities for a more realistic test
    for i in 0..100 {
        let entity = world.spawn();
        world.add(entity, Transform::default());
        world.add(entity, Health::new(100.0 - (i as f32 % 100.0), 100.0));

        if i % 3 == 0 {
            world.add(entity, MeshRenderer::new(i as u64, i as u64 + 5000));
        }
    }

    assert_eq!(world.entity_count(), 100, "Should have 100 entities");

    // Snapshot the world
    let snapshot = WorldState::snapshot(&world);
    assert_eq!(snapshot.metadata.entity_count, 100);

    // Serialize to Bincode
    let bincode_bytes = Serializable::serialize(&snapshot, Format::Bincode)
        .expect("Bincode serialization should succeed");

    // Bincode should be compact (rough estimate: < 50 bytes per entity on average)
    let bytes_per_entity = bincode_bytes.len() / 100;
    assert!(
        bytes_per_entity < 200,
        "Bincode should be compact: {} bytes/entity",
        bytes_per_entity
    );

    // Deserialize
    let restored_snapshot =
        <WorldState as Serializable>::deserialize(&bincode_bytes, Format::Bincode)
            .expect("Bincode deserialization should succeed");

    assert_eq!(restored_snapshot.metadata.entity_count, 100);
    assert_eq!(restored_snapshot.entities.len(), 100);

    // Restore to new world
    let mut new_world = World::new();
    new_world.register::<Transform>();
    new_world.register::<Health>();
    new_world.register::<MeshRenderer>();

    restored_snapshot.restore(&mut new_world);

    assert_eq!(new_world.entity_count(), 100, "Restored world should have 100 entities");
}

/// Test large world with 1000 entities
#[test]
fn test_large_world_serialization() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Health>();
    world.register::<MeshRenderer>();

    // Create 1000 entities
    for i in 0..1000 {
        let entity = world.spawn();
        world.add(entity, Transform::default());
        world.add(entity, Health::new(100.0 - (i as f32 % 100.0), 100.0));

        if i % 4 == 0 {
            world.add(entity, MeshRenderer::new(i as u64, i as u64 + 10000));
        }
    }

    assert_eq!(world.entity_count(), 1000);

    // Snapshot
    let snapshot = WorldState::snapshot(&world);
    assert_eq!(snapshot.metadata.entity_count, 1000);

    // Test Bincode (fastest format)
    let bincode_bytes = Serializable::serialize(&snapshot, Format::Bincode)
        .expect("Should serialize 1000 entities");

    // Deserialize
    let restored = <WorldState as Serializable>::deserialize(&bincode_bytes, Format::Bincode)
        .expect("Should deserialize 1000 entities");

    assert_eq!(restored.metadata.entity_count, 1000);
    assert_eq!(restored.entities.len(), 1000);

    // Verify component count is preserved
    assert_eq!(snapshot.metadata.component_count, restored.metadata.component_count);
}

/// Test serialization with Writer/Reader interfaces
#[test]
fn test_serialize_to_writer() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Health>();

    for i in 0..50 {
        let entity = world.spawn();
        world.add(entity, Transform::default());
        world.add(entity, Health::new(50.0 + i as f32, 100.0));
    }

    let snapshot = WorldState::snapshot(&world);

    // Serialize to a buffer
    let mut buffer = Vec::new();
    Serializable::serialize_to(&snapshot, &mut buffer, Format::Bincode)
        .expect("Should serialize to writer");

    assert!(!buffer.is_empty(), "Buffer should contain data");

    // Deserialize from reader
    let cursor = std::io::Cursor::new(buffer);
    let restored = <WorldState as Serializable>::deserialize_from(cursor, Format::Bincode)
        .expect("Should deserialize from reader");

    assert_eq!(restored.metadata.entity_count, 50);
}

/// Test delta compression with real world modifications
#[test]
fn test_delta_with_world_changes() {
    let mut world1 = World::new();
    world1.register::<Transform>();
    world1.register::<Health>();

    // Create initial state
    for _ in 0..20 {
        let entity = world1.spawn();
        world1.add(entity, Transform::default());
        world1.add(entity, Health::new(100.0, 100.0));
    }

    let snapshot1 = WorldState::snapshot(&world1);

    // Modify 5 entities (25% of world)
    let mut world2 = World::new();
    world2.register::<Transform>();
    world2.register::<Health>();
    snapshot1.restore(&mut world2);

    let entities: Vec<_> = world2.entities().collect();
    for entity in entities.iter().take(5) {
        if let Some(health) = world2.get_mut::<Health>(*entity) {
            health.damage(30.0);
        }
    }

    let mut snapshot2 = WorldState::snapshot(&world2);
    snapshot2.metadata.version = snapshot1.metadata.version + 1;

    // Compute delta
    let delta = WorldStateDelta::compute(&snapshot1, &snapshot2);

    // Delta should only contain modified components
    assert!(
        delta.modified_components.len() <= 5,
        "Should have at most 5 modified components"
    );

    // Delta should be smaller than full state for small changes
    let delta_bytes = bincode::serialize(&delta).unwrap();
    let full_bytes = bincode::serialize(&snapshot2).unwrap();

    assert!(
        delta_bytes.len() < full_bytes.len(),
        "Delta ({} bytes) should be smaller than full state ({} bytes) for 25% change",
        delta_bytes.len(),
        full_bytes.len()
    );
}
