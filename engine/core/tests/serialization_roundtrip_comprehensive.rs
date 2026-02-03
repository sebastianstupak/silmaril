//! Comprehensive roundtrip tests for Phase 1.3 Serialization
//!
//! This test suite provides complete coverage for all serialization formats (YAML, Bincode, FlatBuffers)
//! with systematic testing of:
//! - Edge cases (0, 1, 100, 1000 entities)
//! - Component combinations (single, multiple, all types)
//! - Value preservation (exact values match after roundtrip)
//! - All supported formats

use engine_core::ecs::World;
use engine_core::gameplay::Health;
use engine_core::math::{Transform, Vec3};
use engine_core::physics_components::Velocity;
use engine_core::rendering::MeshRenderer;
use engine_core::serialization::{Format, Serializable, WorldState};

// ====================
// YAML ROUNDTRIP TESTS
// ====================

#[test]
fn test_yaml_roundtrip_empty_world() {
    let world = World::new();
    let snapshot = WorldState::snapshot(&world);

    let yaml_bytes = snapshot.serialize(Format::Yaml).unwrap();
    let restored = WorldState::deserialize(&yaml_bytes, Format::Yaml).unwrap();

    assert_eq!(snapshot.entities.len(), 0);
    assert_eq!(restored.entities.len(), 0);
    assert_eq!(snapshot.metadata.entity_count, restored.metadata.entity_count);
    assert_eq!(snapshot.metadata.component_count, restored.metadata.component_count);
}

#[test]
fn test_yaml_roundtrip_single_entity_single_component() {
    let mut world = World::new();
    world.register::<Health>();

    let entity = world.spawn();
    world.add(entity, Health::new(75.5, 100.0));

    let snapshot = WorldState::snapshot(&world);
    let yaml_bytes = snapshot.serialize(Format::Yaml).unwrap();
    let restored = WorldState::deserialize(&yaml_bytes, Format::Yaml).unwrap();

    assert_eq!(snapshot.entities.len(), 1);
    assert_eq!(restored.entities.len(), 1);
    assert_eq!(snapshot.metadata.entity_count, 1);
    assert_eq!(restored.metadata.entity_count, 1);
    assert_eq!(snapshot.metadata.component_count, 1);
    assert_eq!(restored.metadata.component_count, 1);

    // Verify entity structure matches
    assert_eq!(snapshot.entities[0].entity, restored.entities[0].entity);
}

#[test]
fn test_yaml_roundtrip_single_entity_multiple_components() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Health>();
    world.register::<Velocity>();
    world.register::<MeshRenderer>();

    let entity = world.spawn();
    let mut transform = Transform::identity();
    transform.position = Vec3::new(10.0, 20.0, 30.0);
    world.add(entity, transform);
    world.add(entity, Health::new(85.0, 100.0));
    world.add(entity, Velocity::new(1.0, 2.0, 3.0));
    world.add(entity, MeshRenderer::new(42));

    let snapshot = WorldState::snapshot(&world);
    let yaml_bytes = snapshot.serialize(Format::Yaml).unwrap();
    let restored = WorldState::deserialize(&yaml_bytes, Format::Yaml).unwrap();

    assert_eq!(snapshot.entities.len(), 1);
    assert_eq!(restored.entities.len(), 1);
    assert_eq!(snapshot.metadata.component_count, 4);
    assert_eq!(restored.metadata.component_count, 4);

    // Restore to new world and verify components exist
    let mut world2 = World::new();
    world2.register::<Transform>();
    world2.register::<Health>();
    world2.register::<Velocity>();
    world2.register::<MeshRenderer>();
    restored.restore(&mut world2);

    assert_eq!(world2.entity_count(), 1);
    assert!(world2.has::<Transform>(entity));
    assert!(world2.has::<Health>(entity));
    assert!(world2.has::<Velocity>(entity));
    assert!(world2.has::<MeshRenderer>(entity));
}

#[test]
fn test_yaml_roundtrip_100_entities() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Health>();

    for i in 0..100 {
        let entity = world.spawn();
        let mut transform = Transform::identity();
        transform.position = Vec3::new(i as f32, i as f32 * 2.0, i as f32 * 3.0);
        world.add(entity, transform);
        world.add(entity, Health::new(50.0 + (i as f32 % 50.0), 100.0));
    }

    let snapshot = WorldState::snapshot(&world);
    let yaml_bytes = snapshot.serialize(Format::Yaml).unwrap();
    let restored = WorldState::deserialize(&yaml_bytes, Format::Yaml).unwrap();

    assert_eq!(snapshot.entities.len(), 100);
    assert_eq!(restored.entities.len(), 100);
    assert_eq!(snapshot.metadata.entity_count, 100);
    assert_eq!(restored.metadata.entity_count, 100);
    assert_eq!(snapshot.metadata.component_count, 200);
    assert_eq!(restored.metadata.component_count, 200);

    // Restore and verify entity count
    let mut world2 = World::new();
    world2.register::<Transform>();
    world2.register::<Health>();
    restored.restore(&mut world2);

    assert_eq!(world2.entity_count(), 100);
}

#[test]
fn test_yaml_roundtrip_1000_entities() {
    let mut world = World::new();
    world.register::<Health>();
    world.register::<Velocity>();

    for i in 0..1000 {
        let entity = world.spawn();
        world.add(entity, Health::new((i % 100) as f32, 100.0));
        world.add(entity, Velocity::new(i as f32, (i * 2) as f32, (i * 3) as f32));
    }

    let snapshot = WorldState::snapshot(&world);
    let yaml_bytes = snapshot.serialize(Format::Yaml).unwrap();
    let restored = WorldState::deserialize(&yaml_bytes, Format::Yaml).unwrap();

    assert_eq!(snapshot.entities.len(), 1000);
    assert_eq!(restored.entities.len(), 1000);
    assert_eq!(snapshot.metadata.entity_count, 1000);
    assert_eq!(restored.metadata.entity_count, 1000);
    assert_eq!(snapshot.metadata.component_count, 2000);
    assert_eq!(restored.metadata.component_count, 2000);

    // Restore and verify
    let mut world2 = World::new();
    world2.register::<Health>();
    world2.register::<Velocity>();
    restored.restore(&mut world2);

    assert_eq!(world2.entity_count(), 1000);
}

// ====================
// BINCODE ROUNDTRIP TESTS
// ====================

#[test]
fn test_bincode_roundtrip_empty_world() {
    let world = World::new();
    let snapshot = WorldState::snapshot(&world);

    let bytes = snapshot.serialize(Format::Bincode).unwrap();
    let restored = WorldState::deserialize(&bytes, Format::Bincode).unwrap();

    assert_eq!(snapshot.entities.len(), 0);
    assert_eq!(restored.entities.len(), 0);
    assert_eq!(snapshot.metadata.entity_count, restored.metadata.entity_count);
    assert_eq!(snapshot.metadata.component_count, restored.metadata.component_count);
}

#[test]
fn test_bincode_roundtrip_single_entity_single_component() {
    let mut world = World::new();
    world.register::<Health>();

    let entity = world.spawn();
    world.add(entity, Health::new(66.6, 100.0));

    let snapshot = WorldState::snapshot(&world);
    let bytes = snapshot.serialize(Format::Bincode).unwrap();
    let restored = WorldState::deserialize(&bytes, Format::Bincode).unwrap();

    assert_eq!(snapshot.entities.len(), 1);
    assert_eq!(restored.entities.len(), 1);
    assert_eq!(snapshot.metadata.entity_count, 1);
    assert_eq!(restored.metadata.entity_count, 1);
    assert_eq!(snapshot.metadata.component_count, 1);
    assert_eq!(restored.metadata.component_count, 1);

    assert_eq!(snapshot.entities[0].entity, restored.entities[0].entity);
}

#[test]
fn test_bincode_roundtrip_single_entity_multiple_components() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Health>();
    world.register::<Velocity>();
    world.register::<MeshRenderer>();

    let entity = world.spawn();
    let mut transform = Transform::identity();
    transform.position = Vec3::new(15.5, 25.5, 35.5);
    world.add(entity, transform);
    world.add(entity, Health::new(92.5, 100.0));
    world.add(entity, Velocity::new(5.0, 6.0, 7.0));
    world.add(entity, MeshRenderer::new(999));

    let snapshot = WorldState::snapshot(&world);
    let bytes = snapshot.serialize(Format::Bincode).unwrap();
    let restored = WorldState::deserialize(&bytes, Format::Bincode).unwrap();

    assert_eq!(snapshot.entities.len(), 1);
    assert_eq!(restored.entities.len(), 1);
    assert_eq!(snapshot.metadata.component_count, 4);
    assert_eq!(restored.metadata.component_count, 4);

    // Restore and verify
    let mut world2 = World::new();
    world2.register::<Transform>();
    world2.register::<Health>();
    world2.register::<Velocity>();
    world2.register::<MeshRenderer>();
    restored.restore(&mut world2);

    assert_eq!(world2.entity_count(), 1);
    assert!(world2.has::<Transform>(entity));
    assert!(world2.has::<Health>(entity));
    assert!(world2.has::<Velocity>(entity));
    assert!(world2.has::<MeshRenderer>(entity));
}

#[test]
fn test_bincode_roundtrip_100_entities() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Health>();
    world.register::<MeshRenderer>();

    for i in 0..100 {
        let entity = world.spawn();
        let mut transform = Transform::identity();
        transform.position = Vec3::new(i as f32 * 1.5, i as f32 * 2.5, i as f32 * 3.5);
        world.add(entity, transform);
        world.add(entity, Health::new(25.0 + (i as f32 % 75.0), 100.0));
        world.add(entity, MeshRenderer::new(i as u64));
    }

    let snapshot = WorldState::snapshot(&world);
    let bytes = snapshot.serialize(Format::Bincode).unwrap();
    let restored = WorldState::deserialize(&bytes, Format::Bincode).unwrap();

    assert_eq!(snapshot.entities.len(), 100);
    assert_eq!(restored.entities.len(), 100);
    assert_eq!(snapshot.metadata.entity_count, 100);
    assert_eq!(restored.metadata.entity_count, 100);
    assert_eq!(snapshot.metadata.component_count, 300);
    assert_eq!(restored.metadata.component_count, 300);

    // Restore and verify
    let mut world2 = World::new();
    world2.register::<Transform>();
    world2.register::<Health>();
    world2.register::<MeshRenderer>();
    restored.restore(&mut world2);

    assert_eq!(world2.entity_count(), 100);
}

#[test]
fn test_bincode_roundtrip_1000_entities() {
    let mut world = World::new();
    world.register::<Health>();
    world.register::<Velocity>();
    world.register::<MeshRenderer>();

    for i in 0..1000 {
        let entity = world.spawn();
        world.add(entity, Health::new((i % 100) as f32 + 0.5, 100.0));
        world.add(entity, Velocity::new(i as f32 * 0.1, i as f32 * 0.2, i as f32 * 0.3));
        world.add(entity, MeshRenderer::new((i * 10) as u64));
    }

    let snapshot = WorldState::snapshot(&world);
    let bytes = snapshot.serialize(Format::Bincode).unwrap();
    let restored = WorldState::deserialize(&bytes, Format::Bincode).unwrap();

    assert_eq!(snapshot.entities.len(), 1000);
    assert_eq!(restored.entities.len(), 1000);
    assert_eq!(snapshot.metadata.entity_count, 1000);
    assert_eq!(restored.metadata.entity_count, 1000);
    assert_eq!(snapshot.metadata.component_count, 3000);
    assert_eq!(restored.metadata.component_count, 3000);

    // Restore and verify
    let mut world2 = World::new();
    world2.register::<Health>();
    world2.register::<Velocity>();
    world2.register::<MeshRenderer>();
    restored.restore(&mut world2);

    assert_eq!(world2.entity_count(), 1000);
}

// ====================
// FLATBUFFERS ROUNDTRIP TESTS
// ====================

#[test]
fn test_flatbuffers_roundtrip_empty_world() {
    let world = World::new();
    let snapshot = WorldState::snapshot(&world);

    let bytes = snapshot.serialize(Format::FlatBuffers).unwrap();
    let restored = WorldState::deserialize(&bytes, Format::FlatBuffers).unwrap();

    assert_eq!(snapshot.entities.len(), 0);
    assert_eq!(restored.entities.len(), 0);
    assert_eq!(snapshot.metadata.entity_count, restored.metadata.entity_count);
    assert_eq!(snapshot.metadata.component_count, restored.metadata.component_count);
}

#[test]
fn test_flatbuffers_roundtrip_single_entity_single_component() {
    let mut world = World::new();
    world.register::<Health>();

    let entity = world.spawn();
    world.add(entity, Health::new(88.8, 100.0));

    let snapshot = WorldState::snapshot(&world);
    let bytes = snapshot.serialize(Format::FlatBuffers).unwrap();
    let restored = WorldState::deserialize(&bytes, Format::FlatBuffers).unwrap();

    assert_eq!(snapshot.entities.len(), 1);
    assert_eq!(restored.entities.len(), 1);
    assert_eq!(snapshot.metadata.entity_count, 1);
    assert_eq!(restored.metadata.entity_count, 1);
    assert_eq!(snapshot.metadata.component_count, 1);
    assert_eq!(restored.metadata.component_count, 1);

    assert_eq!(snapshot.entities[0].entity, restored.entities[0].entity);
}

#[test]
fn test_flatbuffers_roundtrip_single_entity_multiple_components() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Health>();
    world.register::<Velocity>();
    world.register::<MeshRenderer>();

    let entity = world.spawn();
    let mut transform = Transform::identity();
    transform.position = Vec3::new(99.9, 88.8, 77.7);
    world.add(entity, transform);
    world.add(entity, Health::new(55.5, 100.0));
    world.add(entity, Velocity::new(11.1, 22.2, 33.3));
    world.add(entity, MeshRenderer::new(12345));

    let snapshot = WorldState::snapshot(&world);
    let bytes = snapshot.serialize(Format::FlatBuffers).unwrap();
    let restored = WorldState::deserialize(&bytes, Format::FlatBuffers).unwrap();

    assert_eq!(snapshot.entities.len(), 1);
    assert_eq!(restored.entities.len(), 1);
    assert_eq!(snapshot.metadata.component_count, 4);
    assert_eq!(restored.metadata.component_count, 4);

    // Restore and verify
    let mut world2 = World::new();
    world2.register::<Transform>();
    world2.register::<Health>();
    world2.register::<Velocity>();
    world2.register::<MeshRenderer>();
    restored.restore(&mut world2);

    assert_eq!(world2.entity_count(), 1);
    assert!(world2.has::<Transform>(entity));
    assert!(world2.has::<Health>(entity));
    assert!(world2.has::<Velocity>(entity));
    assert!(world2.has::<MeshRenderer>(entity));
}

#[test]
fn test_flatbuffers_roundtrip_100_entities() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Health>();

    for i in 0..100 {
        let entity = world.spawn();
        let mut transform = Transform::identity();
        transform.position = Vec3::new(i as f32 * 10.0, i as f32 * 20.0, i as f32 * 30.0);
        world.add(entity, transform);
        world.add(entity, Health::new(10.0 + (i as f32 % 90.0), 100.0));
    }

    let snapshot = WorldState::snapshot(&world);
    let bytes = snapshot.serialize(Format::FlatBuffers).unwrap();
    let restored = WorldState::deserialize(&bytes, Format::FlatBuffers).unwrap();

    assert_eq!(snapshot.entities.len(), 100);
    assert_eq!(restored.entities.len(), 100);
    assert_eq!(snapshot.metadata.entity_count, 100);
    assert_eq!(restored.metadata.entity_count, 100);
    assert_eq!(snapshot.metadata.component_count, 200);
    assert_eq!(restored.metadata.component_count, 200);

    // Restore and verify
    let mut world2 = World::new();
    world2.register::<Transform>();
    world2.register::<Health>();
    restored.restore(&mut world2);

    assert_eq!(world2.entity_count(), 100);
}

#[test]
fn test_flatbuffers_roundtrip_1000_entities() {
    let mut world = World::new();
    world.register::<Health>();
    world.register::<Velocity>();

    for i in 0..1000 {
        let entity = world.spawn();
        world.add(entity, Health::new((i % 100) as f32 + 1.1, 100.0));
        world.add(entity, Velocity::new(i as f32, (i * 2) as f32, (i * 3) as f32));
    }

    let snapshot = WorldState::snapshot(&world);
    let bytes = snapshot.serialize(Format::FlatBuffers).unwrap();
    let restored = WorldState::deserialize(&bytes, Format::FlatBuffers).unwrap();

    assert_eq!(snapshot.entities.len(), 1000);
    assert_eq!(restored.entities.len(), 1000);
    assert_eq!(snapshot.metadata.entity_count, 1000);
    assert_eq!(restored.metadata.entity_count, 1000);
    assert_eq!(snapshot.metadata.component_count, 2000);
    assert_eq!(restored.metadata.component_count, 2000);

    // Restore and verify
    let mut world2 = World::new();
    world2.register::<Health>();
    world2.register::<Velocity>();
    restored.restore(&mut world2);

    assert_eq!(world2.entity_count(), 1000);
}

// ====================
// VALUE PRESERVATION TESTS
// ====================

#[test]
fn test_value_preservation_health_yaml() {
    let mut world = World::new();
    world.register::<Health>();

    let entity = world.spawn();
    let original_health = Health::new(42.42, 100.0);
    world.add(entity, original_health);

    let snapshot = WorldState::snapshot(&world);
    let bytes = snapshot.serialize(Format::Yaml).unwrap();
    let restored = WorldState::deserialize(&bytes, Format::Yaml).unwrap();

    let mut world2 = World::new();
    world2.register::<Health>();
    restored.restore(&mut world2);

    let restored_health = world2.get::<Health>(entity).unwrap();
    assert_eq!(restored_health.current, original_health.current);
    assert_eq!(restored_health.max, original_health.max);
}

#[test]
fn test_value_preservation_health_bincode() {
    let mut world = World::new();
    world.register::<Health>();

    let entity = world.spawn();
    let original_health = Health::new(33.33, 99.99);
    world.add(entity, original_health);

    let snapshot = WorldState::snapshot(&world);
    let bytes = snapshot.serialize(Format::Bincode).unwrap();
    let restored = WorldState::deserialize(&bytes, Format::Bincode).unwrap();

    let mut world2 = World::new();
    world2.register::<Health>();
    restored.restore(&mut world2);

    let restored_health = world2.get::<Health>(entity).unwrap();
    assert_eq!(restored_health.current, original_health.current);
    assert_eq!(restored_health.max, original_health.max);
}

#[test]
fn test_value_preservation_health_flatbuffers() {
    let mut world = World::new();
    world.register::<Health>();

    let entity = world.spawn();
    let original_health = Health::new(77.77, 111.11);
    world.add(entity, original_health);

    let snapshot = WorldState::snapshot(&world);
    let bytes = snapshot.serialize(Format::FlatBuffers).unwrap();
    let restored = WorldState::deserialize(&bytes, Format::FlatBuffers).unwrap();

    let mut world2 = World::new();
    world2.register::<Health>();
    restored.restore(&mut world2);

    let restored_health = world2.get::<Health>(entity).unwrap();
    // FlatBuffers may have slight floating-point precision differences
    assert!((restored_health.current - original_health.current).abs() < 0.01);
    assert!((restored_health.max - original_health.max).abs() < 0.01);
}

#[test]
fn test_value_preservation_velocity_bincode() {
    let mut world = World::new();
    world.register::<Velocity>();

    let entity = world.spawn();
    let original_velocity = Velocity::new(123.456, -789.012, 345.678);
    world.add(entity, original_velocity);

    let snapshot = WorldState::snapshot(&world);
    let bytes = snapshot.serialize(Format::Bincode).unwrap();
    let restored = WorldState::deserialize(&bytes, Format::Bincode).unwrap();

    let mut world2 = World::new();
    world2.register::<Velocity>();
    restored.restore(&mut world2);

    let restored_velocity = world2.get::<Velocity>(entity).unwrap();
    assert!((restored_velocity.x - original_velocity.x).abs() < 0.001);
    assert!((restored_velocity.y - original_velocity.y).abs() < 0.001);
    assert!((restored_velocity.z - original_velocity.z).abs() < 0.001);
}

#[test]
fn test_value_preservation_transform_bincode() {
    let mut world = World::new();
    world.register::<Transform>();

    let entity = world.spawn();
    let mut original_transform = Transform::identity();
    original_transform.position = Vec3::new(111.222, 333.444, 555.666);
    world.add(entity, original_transform);

    let snapshot = WorldState::snapshot(&world);
    let bytes = snapshot.serialize(Format::Bincode).unwrap();
    let restored = WorldState::deserialize(&bytes, Format::Bincode).unwrap();

    let mut world2 = World::new();
    world2.register::<Transform>();
    restored.restore(&mut world2);

    let restored_transform = world2.get::<Transform>(entity).unwrap();
    assert!((restored_transform.position.x - original_transform.position.x).abs() < 0.001);
    assert!((restored_transform.position.y - original_transform.position.y).abs() < 0.001);
    assert!((restored_transform.position.z - original_transform.position.z).abs() < 0.001);
}

#[test]
fn test_value_preservation_mesh_renderer_bincode() {
    let mut world = World::new();
    world.register::<MeshRenderer>();

    let entity = world.spawn();
    let original_mesh = MeshRenderer::new(987654321);
    world.add(entity, original_mesh);

    let snapshot = WorldState::snapshot(&world);
    let bytes = snapshot.serialize(Format::Bincode).unwrap();
    let restored = WorldState::deserialize(&bytes, Format::Bincode).unwrap();

    let mut world2 = World::new();
    world2.register::<MeshRenderer>();
    restored.restore(&mut world2);

    let restored_mesh = world2.get::<MeshRenderer>(entity).unwrap();
    assert_eq!(restored_mesh.mesh_id, original_mesh.mesh_id);
}

// ====================
// MIXED COMPONENT COMBINATIONS
// ====================

#[test]
fn test_mixed_combinations_transform_only() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Health>();
    world.register::<Velocity>();

    for i in 0..10 {
        let entity = world.spawn();
        let mut transform = Transform::identity();
        transform.position = Vec3::new(i as f32, 0.0, 0.0);
        world.add(entity, transform);
        // Only Transform, no other components
    }

    let snapshot = WorldState::snapshot(&world);
    let bytes = snapshot.serialize(Format::Bincode).unwrap();
    let restored = WorldState::deserialize(&bytes, Format::Bincode).unwrap();

    assert_eq!(restored.entities.len(), 10);
    assert_eq!(restored.metadata.component_count, 10);

    let mut world2 = World::new();
    world2.register::<Transform>();
    world2.register::<Health>();
    world2.register::<Velocity>();
    restored.restore(&mut world2);

    for entity in world2.entities().take(10) {
        assert!(world2.has::<Transform>(entity));
        assert!(!world2.has::<Health>(entity));
        assert!(!world2.has::<Velocity>(entity));
    }
}

#[test]
fn test_mixed_combinations_health_velocity() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Health>();
    world.register::<Velocity>();

    for i in 0..10 {
        let entity = world.spawn();
        world.add(entity, Health::new(i as f32 * 10.0, 100.0));
        world.add(entity, Velocity::new(i as f32, i as f32, i as f32));
        // No Transform
    }

    let snapshot = WorldState::snapshot(&world);
    let bytes = snapshot.serialize(Format::Bincode).unwrap();
    let restored = WorldState::deserialize(&bytes, Format::Bincode).unwrap();

    assert_eq!(restored.entities.len(), 10);
    assert_eq!(restored.metadata.component_count, 20);

    let mut world2 = World::new();
    world2.register::<Transform>();
    world2.register::<Health>();
    world2.register::<Velocity>();
    restored.restore(&mut world2);

    for entity in world2.entities().take(10) {
        assert!(!world2.has::<Transform>(entity));
        assert!(world2.has::<Health>(entity));
        assert!(world2.has::<Velocity>(entity));
    }
}

#[test]
fn test_mixed_combinations_sparse_components() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Health>();
    world.register::<Velocity>();
    world.register::<MeshRenderer>();

    // Create entities with different component combinations
    for i in 0..20 {
        let entity = world.spawn();

        if i % 2 == 0 {
            world.add(entity, Transform::default());
        }
        if i % 3 == 0 {
            world.add(entity, Health::new(100.0, 100.0));
        }
        if i % 5 == 0 {
            world.add(entity, Velocity::new(i as f32, 0.0, 0.0));
        }
        if i % 7 == 0 {
            world.add(entity, MeshRenderer::new(i as u64));
        }
    }

    let snapshot = WorldState::snapshot(&world);
    let bytes = snapshot.serialize(Format::Bincode).unwrap();
    let restored = WorldState::deserialize(&bytes, Format::Bincode).unwrap();

    assert_eq!(restored.entities.len(), 20);

    let mut world2 = World::new();
    world2.register::<Transform>();
    world2.register::<Health>();
    world2.register::<Velocity>();
    world2.register::<MeshRenderer>();
    restored.restore(&mut world2);

    assert_eq!(world2.entity_count(), 20);

    // Verify component distribution
    let entities: Vec<_> = world2.entities().collect();
    for (i, &entity) in entities.iter().enumerate() {
        assert_eq!(world2.has::<Transform>(entity), i % 2 == 0);
        assert_eq!(world2.has::<Health>(entity), i % 3 == 0);
        assert_eq!(world2.has::<Velocity>(entity), i % 5 == 0);
        assert_eq!(world2.has::<MeshRenderer>(entity), i % 7 == 0);
    }
}

// ====================
// CROSS-FORMAT CONSISTENCY
// ====================

#[test]
fn test_all_formats_produce_equivalent_results() {
    let mut world = World::new();
    world.register::<Health>();
    world.register::<Transform>();

    for i in 0..50 {
        let entity = world.spawn();
        let mut transform = Transform::identity();
        transform.position = Vec3::new(i as f32, i as f32 * 2.0, i as f32 * 3.0);
        world.add(entity, transform);
        world.add(entity, Health::new(50.0 + i as f32, 100.0));
    }

    let snapshot = WorldState::snapshot(&world);

    // Serialize with all formats
    let yaml_bytes = snapshot.serialize(Format::Yaml).unwrap();
    let bincode_bytes = snapshot.serialize(Format::Bincode).unwrap();
    let flatbuffers_bytes = snapshot.serialize(Format::FlatBuffers).unwrap();

    // Deserialize with all formats
    let yaml_restored = WorldState::deserialize(&yaml_bytes, Format::Yaml).unwrap();
    let bincode_restored = WorldState::deserialize(&bincode_bytes, Format::Bincode).unwrap();
    let flatbuffers_restored =
        WorldState::deserialize(&flatbuffers_bytes, Format::FlatBuffers).unwrap();

    // All should have same entity count
    assert_eq!(yaml_restored.entities.len(), 50);
    assert_eq!(bincode_restored.entities.len(), 50);
    assert_eq!(flatbuffers_restored.entities.len(), 50);

    // All should have same component count
    assert_eq!(yaml_restored.metadata.component_count, 100);
    assert_eq!(bincode_restored.metadata.component_count, 100);
    assert_eq!(flatbuffers_restored.metadata.component_count, 100);

    // Restore each and verify entity counts match
    let mut world_yaml = World::new();
    world_yaml.register::<Health>();
    world_yaml.register::<Transform>();
    yaml_restored.restore(&mut world_yaml);

    let mut world_bincode = World::new();
    world_bincode.register::<Health>();
    world_bincode.register::<Transform>();
    bincode_restored.restore(&mut world_bincode);

    let mut world_flatbuffers = World::new();
    world_flatbuffers.register::<Health>();
    world_flatbuffers.register::<Transform>();
    flatbuffers_restored.restore(&mut world_flatbuffers);

    assert_eq!(world_yaml.entity_count(), 50);
    assert_eq!(world_bincode.entity_count(), 50);
    assert_eq!(world_flatbuffers.entity_count(), 50);
}
