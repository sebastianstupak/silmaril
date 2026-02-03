//! End-to-End Integration Tests
//!
//! These tests validate complete workflows across multiple engine systems.

use engine_core::ecs::World;
use engine_core::serialization::{WorldState, SerializationFormat};
use engine_math::{Transform, Vec3};

/// Test complete rendering pipeline from ECS to transforms
#[test]
fn test_e2e_rendering_pipeline() {
    let mut world = World::new();

    // Create entities with transforms
    let cube = world.spawn();
    world.add_component(cube, Transform {
        position: Vec3::new(0.0, 0.0, -5.0),
        rotation: glam::Quat::IDENTITY,
        scale: Vec3::ONE,
    }).unwrap();

    // Verify entity exists and is queryable
    assert!(world.is_alive(cube));
    let transform = world.get_component::<Transform>(cube).unwrap();
    assert_eq!(transform.position, Vec3::new(0.0, 0.0, -5.0));
}

/// Test gameplay scenario with player movement
#[test]
fn test_e2e_player_movement() {
    let mut world = World::new();

    let player = world.spawn();
    world.add_component(player, Transform {
        position: Vec3::ZERO,
        rotation: glam::Quat::IDENTITY,
        scale: Vec3::ONE,
    }).unwrap();

    // Simulate 60 frames of movement
    for frame in 0..60 {
        let mut transform = world.get_component_mut::<Transform>(player).unwrap();
        transform.position.x += 0.1; // Move right
        drop(transform);

        // Verify position updates correctly
        let pos = world.get_component::<Transform>(player).unwrap().position;
        let expected = (frame + 1) as f32 * 0.1;
        assert!((pos.x - expected).abs() < 0.001,
            "Frame {}: expected x={}, got x={}", frame, expected, pos.x);
    }

    // Final position should be x=6.0
    let final_pos = world.get_component::<Transform>(player).unwrap().position;
    assert!((final_pos.x - 6.0).abs() < 0.01);
}

/// Test network serialization roundtrip
#[test]
fn test_e2e_network_serialization() {
    let mut world = World::new();

    // Create 10 entities with transforms
    for i in 0..10 {
        let entity = world.spawn();
        world.add_component(entity, Transform {
            position: Vec3::new(i as f32, 0.0, 0.0),
            rotation: glam::Quat::IDENTITY,
            scale: Vec3::ONE,
        }).unwrap();
    }

    // Serialize to binary
    let serialized = WorldState::from_world(&world, 42);
    let bytes = serialized.to_bytes(SerializationFormat::Bincode).unwrap();

    // Deserialize to new world
    let deserialized = WorldState::from_bytes(&bytes, SerializationFormat::Bincode).unwrap();
    assert_eq!(deserialized.frame_number(), 42);

    let mut new_world = World::new();
    deserialized.apply_to_world(&mut new_world).unwrap();

    // Verify same entity count
    let original_count = world.query::<&Transform>().iter().count();
    let new_count = new_world.query::<&Transform>().iter().count();
    assert_eq!(original_count, new_count);
    assert_eq!(new_count, 10);
}

/// Test 1000 entity stress scenario
#[test]
fn test_e2e_1000_entity_stress() {
    let mut world = World::new();

    let start = std::time::Instant::now();

    // Create 1000 entities
    for i in 0..1000 {
        let entity = world.spawn();
        world.add_component(entity, Transform {
            position: Vec3::new(i as f32, 0.0, 0.0),
            rotation: glam::Quat::IDENTITY,
            scale: Vec3::ONE,
        }).unwrap();
    }

    let creation_time = start.elapsed();

    // Should be fast (<50ms for 1000 entities)
    assert!(creation_time.as_millis() < 50,
        "Entity creation too slow: {:?}", creation_time);

    // Query all transforms
    let query_start = std::time::Instant::now();
    let count = world.query::<&Transform>().iter().count();
    let query_time = query_start.elapsed();

    assert_eq!(count, 1000);
    assert!(query_time.as_millis() < 10,
        "Query too slow: {:?}", query_time);
}

/// Test entity lifecycle management
#[test]
fn test_e2e_entity_lifecycle() {
    let mut world = World::new();

    // Spawn entities
    let entities: Vec<_> = (0..10).map(|_| world.spawn()).collect();

    // All should be alive
    for entity in &entities {
        assert!(world.is_alive(*entity));
    }

    // Despawn half
    for entity in entities.iter().take(5) {
        world.despawn(*entity).unwrap();
    }

    // First half dead, second half alive
    for (i, entity) in entities.iter().enumerate() {
        if i < 5 {
            assert!(!world.is_alive(*entity));
        } else {
            assert!(world.is_alive(*entity));
        }
    }
}

/// Test YAML serialization for debugging
#[test]
fn test_e2e_yaml_serialization() {
    let mut world = World::new();

    let entity = world.spawn();
    world.add_component(entity, Transform {
        position: Vec3::new(1.0, 2.0, 3.0),
        rotation: glam::Quat::IDENTITY,
        scale: Vec3::ONE,
    }).unwrap();

    // Serialize to YAML
    let serialized = WorldState::from_world(&world, 123);
    let yaml = serialized.to_yaml().unwrap();

    // Should be human-readable
    assert!(yaml.contains("frame_number"));

    // Deserialize back
    let deserialized = WorldState::from_yaml(&yaml).unwrap();
    assert_eq!(deserialized.frame_number(), 123);
}

/// Test component add/remove cycles
#[test]
fn test_e2e_component_cycling() {
    let mut world = World::new();
    let entity = world.spawn();

    // Add and remove 100 times
    for i in 0..100 {
        world.add_component(entity, Transform {
            position: Vec3::new(i as f32, 0.0, 0.0),
            rotation: glam::Quat::IDENTITY,
            scale: Vec3::ONE,
        }).unwrap();

        assert!(world.has_component::<Transform>(entity));

        world.remove_component::<Transform>(entity).unwrap();

        assert!(!world.has_component::<Transform>(entity));
    }

    // Entity should still be alive
    assert!(world.is_alive(entity));
}

/// Test multi-entity rendering scenario
#[test]
fn test_e2e_100_entity_rendering() {
    let mut world = World::new();

    // Create 100 entities in a grid
    for x in 0..10 {
        for z in 0..10 {
            let entity = world.spawn();
            world.add_component(entity, Transform {
                position: Vec3::new(x as f32 * 2.0, 0.0, z as f32 * 2.0),
                rotation: glam::Quat::IDENTITY,
                scale: Vec3::ONE,
            }).unwrap();
        }
    }

    // Query all entities
    let transforms: Vec<_> = world.query::<&Transform>()
        .iter()
        .collect();

    assert_eq!(transforms.len(), 100);

    // Verify grid pattern
    let mut positions: Vec<_> = transforms.iter()
        .map(|t| t.position)
        .collect();
    positions.sort_by(|a, b| {
        a.x.partial_cmp(&b.x).unwrap()
            .then(a.z.partial_cmp(&b.z).unwrap())
    });

    // First entity should be at (0, 0, 0)
    assert!((positions[0].x - 0.0).abs() < 0.01);
    assert!((positions[0].z - 0.0).abs() < 0.01);

    // Last entity should be at (18, 0, 18)
    assert!((positions[99].x - 18.0).abs() < 0.01);
    assert!((positions[99].z - 18.0).abs() < 0.01);
}
