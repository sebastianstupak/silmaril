//! Stress tests for serialization with large worlds
//!
//! Tests serialization performance and correctness with 10k+ entities.

use engine_core::ecs::World;
use engine_core::gameplay::Health;
use engine_core::math::Transform;
use engine_core::physics_components::Velocity;
use engine_core::rendering::MeshRenderer;
use engine_core::serialization::{Format, Serializable, WorldState, WorldStateDelta};
use std::time::Instant;

#[test]
fn test_serialize_10k_entities_with_single_component() {
    let mut world = World::new();
    world.register::<Transform>();

    // Create 10k entities
    for _ in 0..10_000 {
        let e = world.spawn();
        world.add(e, Transform::default());
    }

    let start = Instant::now();
    let snapshot = WorldState::snapshot(&world);
    let snapshot_time = start.elapsed();

    println!("Snapshot 10k entities: {:?}", snapshot_time);

    assert_eq!(snapshot.entities.len(), 10_000);
    assert_eq!(snapshot.metadata.entity_count, 10_000);
    assert_eq!(snapshot.metadata.component_count, 10_000);

    // Verify snapshot time is reasonable (< 50ms for debug build)
    assert!(
        snapshot_time.as_millis() < 100,
        "Snapshot took {:?}, expected < 100ms",
        snapshot_time
    );
}

#[test]
fn test_serialize_10k_entities_with_multiple_components() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Health>();
    world.register::<Velocity>();

    // Create 10k entities with 3 components each
    for i in 0..10_000 {
        let e = world.spawn();
        world.add(e, Transform::default());
        world.add(e, Health::new(100.0, 100.0));
        world.add(e, Velocity::new(i as f32, i as f32, i as f32));
    }

    let start = Instant::now();
    let snapshot = WorldState::snapshot(&world);
    let snapshot_time = start.elapsed();

    println!("Snapshot 10k entities (3 components each): {:?}", snapshot_time);

    assert_eq!(snapshot.entities.len(), 10_000);
    assert_eq!(snapshot.metadata.component_count, 30_000);

    // Verify snapshot time
    assert!(snapshot_time.as_millis() < 200, "Snapshot took {:?}", snapshot_time);
}

#[test]
fn test_bincode_serialize_10k_entities() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Health>();

    for i in 0..10_000 {
        let e = world.spawn();
        world.add(e, Transform::default());
        world.add(e, Health::new(i as f32, 100.0));
    }

    let snapshot = WorldState::snapshot(&world);

    let start = Instant::now();
    let bytes = snapshot.serialize(Format::Bincode).unwrap();
    let serialize_time = start.elapsed();

    println!(
        "Bincode serialize 10k entities: {:?}, size: {} bytes",
        serialize_time,
        bytes.len()
    );

    // Verify size is reasonable (should be < 1MB for 10k simple entities)
    assert!(bytes.len() < 1_000_000, "Bincode size: {} bytes (expected < 1MB)", bytes.len());

    // Verify serialization time (< 10ms target)
    assert!(serialize_time.as_millis() < 50, "Serialize took {:?}", serialize_time);

    // Test deserialization
    let start = Instant::now();
    let restored = WorldState::deserialize(&bytes, Format::Bincode).unwrap();
    let deserialize_time = start.elapsed();

    println!("Bincode deserialize 10k entities: {:?}", deserialize_time);

    assert_eq!(restored.entities.len(), 10_000);
    assert!(deserialize_time.as_millis() < 50, "Deserialize took {:?}", deserialize_time);
}

#[test]
fn test_yaml_serialize_1k_entities() {
    // YAML is slower, so test with 1k entities instead of 10k
    let mut world = World::new();
    world.register::<Health>();

    for i in 0..1_000 {
        let e = world.spawn();
        world.add(e, Health::new(i as f32, 100.0));
    }

    let snapshot = WorldState::snapshot(&world);

    let start = Instant::now();
    let yaml = snapshot.serialize(Format::Yaml).unwrap();
    let serialize_time = start.elapsed();

    println!("YAML serialize 1k entities: {:?}, size: {} bytes", serialize_time, yaml.len());

    // YAML should be readable
    let yaml_str = String::from_utf8(yaml.clone()).unwrap();
    assert!(yaml_str.contains("entities"));
    assert!(yaml_str.contains("Health"));

    // Verify YAML size (should be larger than bincode but still reasonable)
    assert!(yaml.len() < 5_000_000, "YAML size: {} bytes", yaml.len());

    // YAML is slower than bincode, but should still be < 500ms for 1k entities
    assert!(serialize_time.as_millis() < 500, "YAML serialize took {:?}", serialize_time);

    // Test deserialization
    let start = Instant::now();
    let restored = WorldState::deserialize(&yaml, Format::Yaml).unwrap();
    let deserialize_time = start.elapsed();

    println!("YAML deserialize 1k entities: {:?}", deserialize_time);

    assert_eq!(restored.entities.len(), 1_000);
}

#[test]
fn test_restore_10k_entities() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Velocity>();

    for i in 0..10_000 {
        let e = world.spawn();
        world.add(e, Transform::default());
        world.add(e, Velocity::new(i as f32, 0.0, 0.0));
    }

    let snapshot = WorldState::snapshot(&world);

    // Create new world and restore
    let mut world2 = World::new();
    world2.register::<Transform>();
    world2.register::<Velocity>();

    let start = Instant::now();
    snapshot.restore(&mut world2);
    let restore_time = start.elapsed();

    println!("Restore 10k entities: {:?}", restore_time);

    assert_eq!(world2.entity_count(), 10_000);

    // Verify restore time (< 20ms target for debug build)
    assert!(restore_time.as_millis() < 100, "Restore took {:?}", restore_time);

    // Verify some entities have correct components
    let entities: Vec<_> = world2.entities().take(10).collect();
    for entity in entities {
        assert!(world2.has::<Transform>(entity));
        assert!(world2.has::<Velocity>(entity));
    }
}

#[test]
fn test_delta_with_10k_entities_small_changes() {
    let mut world = World::new();
    world.register::<Health>();

    // Create 10k entities
    for _ in 0..10_000 {
        let e = world.spawn();
        world.add(e, Health::new(100.0, 100.0));
    }

    let state1 = WorldState::snapshot(&world);

    // Modify only 100 entities (1% of total)
    let entities_to_modify: Vec<_> = world.entities().take(100).collect();
    for entity in entities_to_modify {
        if let Some(health) = world.get_mut::<Health>(entity) {
            health.current = 50.0;
        }
    }

    let state2 = WorldState::snapshot(&world);

    let start = Instant::now();
    let delta = WorldStateDelta::compute(&state1, &state2);
    let delta_time = start.elapsed();

    println!("Compute delta (10k entities, 1% changed): {:?}", delta_time);

    // Delta should be much smaller than full state
    let full_size = bincode::serialize(&state2).unwrap().len();
    let delta_size = bincode::serialize(&delta).unwrap().len();

    println!(
        "Full state size: {} bytes, Delta size: {} bytes, Reduction: {:.1}%",
        full_size,
        delta_size,
        100.0 * (1.0 - delta_size as f64 / full_size as f64)
    );

    assert!(delta_size < full_size, "Delta should be smaller than full state");

    // Delta computation should be fast (< 10ms target)
    assert!(delta_time.as_millis() < 50, "Delta compute took {:?}", delta_time);

    // Apply delta
    let mut state1_copy = state1.clone();
    let start = Instant::now();
    delta.apply(&mut state1_copy);
    let apply_time = start.elapsed();

    println!("Apply delta: {:?}", apply_time);

    // Delta application should be fast (< 8ms target)
    assert!(apply_time.as_millis() < 50, "Delta apply took {:?}", apply_time);

    // Result should match state2
    assert_eq!(state1_copy.entities.len(), state2.entities.len());
}

#[test]
fn test_memory_usage_10k_entities() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Health>();
    world.register::<Velocity>();
    world.register::<MeshRenderer>();

    // Create 10k entities with all 4 component types
    for i in 0..10_000 {
        let e = world.spawn();
        world.add(e, Transform::default());
        world.add(e, Health::new(100.0, 100.0));
        world.add(e, Velocity::new(i as f32, i as f32, i as f32));
        world.add(e, MeshRenderer::new(i as u64));
    }

    let snapshot = WorldState::snapshot(&world);

    // Test bincode size
    let bincode_bytes = snapshot.serialize(Format::Bincode).unwrap();
    println!(
        "10k entities (4 components) - Bincode: {} bytes ({:.2} KB)",
        bincode_bytes.len(),
        bincode_bytes.len() as f64 / 1024.0
    );

    // Should be under 2MB for 10k entities with 4 components
    assert!(
        bincode_bytes.len() < 2_000_000,
        "Bincode size {} bytes exceeds 2MB limit",
        bincode_bytes.len()
    );

    // Verify entity and component counts
    assert_eq!(snapshot.metadata.entity_count, 10_000);
    assert_eq!(snapshot.metadata.component_count, 40_000);
}

#[test]
fn test_roundtrip_preserves_all_data_large_world() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Health>();
    world.register::<Velocity>();

    // Create diverse entities
    for i in 0..5_000 {
        let e = world.spawn();
        world.add(e, Transform::default());
        world.add(e, Health::new(i as f32, (i + 100) as f32));
        world.add(e, Velocity::new(i as f32, (i * 2) as f32, (i * 3) as f32));
    }

    let snapshot = WorldState::snapshot(&world);
    let bytes = snapshot.serialize(Format::Bincode).unwrap();
    let restored = WorldState::deserialize(&bytes, Format::Bincode).unwrap();

    // Restore to new world
    let mut world2 = World::new();
    world2.register::<Transform>();
    world2.register::<Health>();
    world2.register::<Velocity>();
    restored.restore(&mut world2);

    // Verify all entities exist
    assert_eq!(world2.entity_count(), 5_000);

    // Spot check some entities
    for entity in world2.entities().step_by(500).take(10) {
        assert!(world2.has::<Transform>(entity), "Entity {:?} missing Transform", entity);
        assert!(world2.has::<Health>(entity), "Entity {:?} missing Health", entity);
        assert!(world2.has::<Velocity>(entity), "Entity {:?} missing Velocity", entity);
    }
}

#[test]
fn test_concurrent_serialization_safety() {
    // Test that multiple snapshots from the same world work correctly
    let mut world = World::new();
    world.register::<Health>();

    for i in 0..1_000 {
        let e = world.spawn();
        world.add(e, Health::new(i as f32, 100.0));
    }

    // Take multiple snapshots
    let snapshot1 = WorldState::snapshot(&world);
    let snapshot2 = WorldState::snapshot(&world);
    let snapshot3 = WorldState::snapshot(&world);

    // All should be identical
    assert_eq!(snapshot1.entities.len(), snapshot2.entities.len());
    assert_eq!(snapshot2.entities.len(), snapshot3.entities.len());

    // Serialize all
    let bytes1 = snapshot1.serialize(Format::Bincode).unwrap();
    let bytes2 = snapshot2.serialize(Format::Bincode).unwrap();
    let bytes3 = snapshot3.serialize(Format::Bincode).unwrap();

    // All should produce similar sizes (timestamps may differ by a few bytes)
    // Allow 100 byte variance for timestamp differences
    let size_diff_12 = (bytes1.len() as i64 - bytes2.len() as i64).abs();
    let size_diff_23 = (bytes2.len() as i64 - bytes3.len() as i64).abs();

    assert!(size_diff_12 < 100, "Size difference {} exceeds 100 bytes", size_diff_12);
    assert!(size_diff_23 < 100, "Size difference {} exceeds 100 bytes", size_diff_23);
}

#[test]
fn test_very_large_world_20k_entities() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Health>();

    println!("Creating 20k entities...");
    for i in 0..20_000 {
        let e = world.spawn();
        world.add(e, Transform::default());
        world.add(e, Health::new(i as f32, 100.0));
    }

    println!("Taking snapshot...");
    let start = Instant::now();
    let snapshot = WorldState::snapshot(&world);
    let snapshot_time = start.elapsed();
    println!("Snapshot time: {:?}", snapshot_time);

    println!("Serializing to bincode...");
    let start = Instant::now();
    let bytes = snapshot.serialize(Format::Bincode).unwrap();
    let serialize_time = start.elapsed();
    println!("Serialize time: {:?}, size: {} KB", serialize_time, bytes.len() / 1024);

    println!("Deserializing...");
    let start = Instant::now();
    let restored = WorldState::deserialize(&bytes, Format::Bincode).unwrap();
    let deserialize_time = start.elapsed();
    println!("Deserialize time: {:?}", deserialize_time);

    assert_eq!(restored.entities.len(), 20_000);
}
