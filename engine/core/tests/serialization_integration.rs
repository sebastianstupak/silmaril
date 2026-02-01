//! Integration tests for world state serialization

use engine_core::ecs::World;
use engine_core::gameplay::Health;
use engine_core::math::Transform;
use engine_core::physics_components::Velocity;
use engine_core::serialization::{Format, Serializable, WorldState};

#[test]
fn test_world_snapshot_and_restore() {
    let mut world = World::new();

    // Register components
    world.register::<Transform>();
    world.register::<Health>();
    world.register::<Velocity>();

    // Create some entities with components
    let e1 = world.spawn();
    world.add(e1, Transform::default());
    world.add(e1, Health::new(100.0, 100.0));

    let e2 = world.spawn();
    world.add(e2, Transform::default());
    world.add(e2, Velocity::new(1.0, 2.0, 3.0));

    let e3 = world.spawn();
    world.add(e3, Health::new(50.0, 100.0));

    // Take snapshot
    let snapshot = WorldState::snapshot(&world);

    // Verify snapshot has correct counts
    assert_eq!(snapshot.entities.len(), 3);
    assert_eq!(snapshot.metadata.entity_count, 3);

    // Create a new world and restore
    let mut world2 = World::new();
    world2.register::<Transform>();
    world2.register::<Health>();
    world2.register::<Velocity>();

    snapshot.restore(&mut world2);

    // Verify restoration
    assert_eq!(world2.entity_count(), 3);
    assert!(world2.is_alive(e1));
    assert!(world2.is_alive(e2));
    assert!(world2.is_alive(e3));

    // Verify components were restored
    assert!(world2.has::<Transform>(e1));
    assert!(world2.has::<Health>(e1));
    assert!(world2.has::<Transform>(e2));
    assert!(world2.has::<Velocity>(e2));
    assert!(world2.has::<Health>(e3));
}

#[test]
fn test_yaml_serialization_roundtrip() {
    let mut world = World::new();
    world.register::<Health>();

    let e = world.spawn();
    world.add(e, Health::new(75.0, 100.0));

    let snapshot = WorldState::snapshot(&world);

    // Serialize to YAML
    let yaml_bytes = snapshot.serialize(Format::Yaml).unwrap();

    // Verify it's actual YAML (should contain readable text)
    let yaml_str = String::from_utf8(yaml_bytes.clone()).unwrap();
    assert!(yaml_str.contains("entities"));
    assert!(yaml_str.contains("components"));

    // Deserialize from YAML
    let restored = WorldState::deserialize(&yaml_bytes, Format::Yaml).unwrap();

    assert_eq!(snapshot.entities.len(), restored.entities.len());
    assert_eq!(snapshot.metadata.entity_count, restored.metadata.entity_count);
}

#[test]
fn test_bincode_serialization_roundtrip() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Velocity>();

    for i in 0..10 {
        let e = world.spawn();
        world.add(e, Transform::default());
        world.add(e, Velocity::new(i as f32, i as f32, i as f32));
    }

    let snapshot = WorldState::snapshot(&world);

    // Serialize to bincode
    let bytes = snapshot.serialize(Format::Bincode).unwrap();

    // Bincode should be more compact than YAML
    assert!(bytes.len() < 1000); // Reasonable size for 10 entities

    // Deserialize from bincode
    let restored = WorldState::deserialize(&bytes, Format::Bincode).unwrap();

    assert_eq!(snapshot.entities.len(), restored.entities.len());
    assert_eq!(snapshot.metadata.component_count, restored.metadata.component_count);
}

#[test]
fn test_empty_world_snapshot() {
    let world = World::new();

    let snapshot = WorldState::snapshot(&world);

    assert_eq!(snapshot.entities.len(), 0);
    assert_eq!(snapshot.metadata.entity_count, 0);
    assert_eq!(snapshot.metadata.component_count, 0);

    // Should be able to serialize empty world
    let yaml = snapshot.serialize(Format::Yaml).unwrap();
    assert!(yaml.len() > 0);

    let restored = WorldState::deserialize(&yaml, Format::Yaml).unwrap();
    assert_eq!(restored.entities.len(), 0);
}

#[test]
fn test_world_clear_and_restore() {
    let mut world = World::new();
    world.register::<Health>();

    let e1 = world.spawn();
    world.add(e1, Health::new(100.0, 100.0));

    let e2 = world.spawn();
    world.add(e2, Health::new(50.0, 50.0));

    let snapshot = WorldState::snapshot(&world);

    // Clear world
    world.clear();
    assert_eq!(world.entity_count(), 0);

    // Restore
    snapshot.restore(&mut world);
    assert_eq!(world.entity_count(), 2);
}
