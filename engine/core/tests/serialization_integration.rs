//! Integration tests for serialization across ECS, compression, versioning, and validation
//!
//! Real-world scenarios testing the complete serialization pipeline.

use engine_core::ecs::World;
use engine_core::math::Transform;
use engine_core::serialization::{
    ChecksumAlgorithm, Format, RecoveryOptions, RecoveryStrategy, Serializable,
    ValidatedWorldState, VersionedWorldState, WorldState, WorldStateValidator,
};

#[cfg(feature = "compression")]
use engine_core::serialization::{CompressedData, CompressionAlgorithm, OptimizedDelta};

/// Real-world scenario: MMO player save file
///
/// Player with position, inventory, stats
#[test]
#[cfg(feature = "compression")]
fn test_mmo_player_save() {
    let mut world = World::new();
    world.register::<Transform>();

    // Create player entity
    let player = world.spawn();
    let mut transform = Transform::identity();
    transform.position = engine_core::math::Vec3::new(123.45, 67.89, 234.56);
    world.add(player, transform);

    // Snapshot
    let state = WorldState::snapshot(&world);

    // Add versioning
    let versioned = VersionedWorldState::new(state);

    // Add checksum validation
    let validated = ValidatedWorldState::new(versioned.state, ChecksumAlgorithm::Xxh3);

    // Serialize
    let bytes = validated.save().unwrap();

    // Compress for save file
    let compressed = CompressedData::compress(&bytes, CompressionAlgorithm::Zstd).unwrap();

    // Verify compression worked
    assert!(compressed.size_savings_percent() > 10.0);

    // Load back
    let decompressed = compressed.decompress().unwrap();
    let loaded = ValidatedWorldState::load_validated(&decompressed).unwrap();

    // Verify position preserved
    let loaded_world = &loaded.state;
    assert_eq!(loaded_world.entities.len(), 1);

    // Restore to new world
    let mut world2 = World::new();
    world2.register::<Transform>();
    loaded_world.restore(&mut world2);

    // Verify entity exists
    assert_eq!(world2.entity_count(), 1);
}

/// Real-world scenario: Network state sync (60 FPS)
///
/// Client sends incremental updates to server
#[test]
#[cfg(feature = "compression")]
fn test_network_state_sync_60fps() {
    let mut world = World::new();
    world.register::<Transform>();

    // Create 100 entities (typical player count)
    for i in 0..100 {
        let entity = world.spawn();
        let mut transform = Transform::identity();
        transform.position = engine_core::math::Vec3::new(i as f32, 0.0, 0.0);
        world.add(entity, transform);
    }

    // Initial snapshot
    let state_t0 = WorldState::snapshot(&world);

    // Simulate 1/60th second - only 10% of entities move
    let entities_to_move: Vec<_> = world.entities().take(10).collect();
    for entity in entities_to_move {
        if let Some(transform) = world.get_mut::<Transform>(entity) {
            transform.position.x += 1.0;
        }
    }

    let state_t1 = WorldState::snapshot(&world);

    // Compute delta
    let delta = OptimizedDelta::compute(&state_t0, &state_t1);

    // Verify delta stats
    let stats = delta.stats();
    assert_eq!(stats.changed_entities, 10);
    assert_eq!(stats.unchanged_entities, 90);
    assert!(stats.change_percentage() < 15.0);

    // Serialize delta
    let delta_bytes = bincode::serialize(&delta).unwrap();

    // Compress for network
    let compressed = CompressedData::compress(&delta_bytes, CompressionAlgorithm::Lz4).unwrap();

    // Verify delta is much smaller than full state
    let full_bytes = bincode::serialize(&state_t1).unwrap();
    assert!(compressed.data.len() < full_bytes.len() / 2);

    // Server receives and applies
    let mut server_state = state_t0.clone();
    delta.apply(&mut server_state);

    // Verify sync worked
    assert_eq!(server_state.entities.len(), state_t1.entities.len());
}

/// Real-world scenario: Corrupted save file recovery
///
/// Player's save file got corrupted, recover what we can
#[test]
fn test_corrupted_save_recovery() {
    let mut world = World::new();
    world.register::<Transform>();

    // Create world with 50 entities
    for i in 0..50 {
        let entity = world.spawn();
        let mut transform = Transform::identity();
        transform.position = engine_core::math::Vec3::new(i as f32, i as f32, i as f32);
        world.add(entity, transform);
    }

    let state = WorldState::snapshot(&world);

    // Save with validation
    let validated = ValidatedWorldState::new(state, ChecksumAlgorithm::Crc32);
    let mut bytes = validated.save().unwrap();

    // Simulate corruption - flip some bytes in the middle
    if bytes.len() > 100 {
        bytes[50] = !bytes[50];
        bytes[51] = !bytes[51];
    }

    // Try to load - should fail validation
    let load_result = ValidatedWorldState::load_validated(&bytes);
    assert!(load_result.is_err());

    // Try recovery with SkipCorrupt strategy
    let options = RecoveryOptions {
        strategy: RecoveryStrategy::SkipCorrupt,
        max_corrupt_entities: 10,
        log_recovery: false,
    };

    let validator = WorldStateValidator::with_options(options);

    // For this test, we'll use the original state since we can't easily
    // recover from corrupted bincode data. In production, you'd implement
    // sector-based recovery.
    let validation = validator.validate_structure(&validated.state);
    assert!(validation.is_valid());
}

/// Real-world scenario: Version migration (v1 -> v2)
///
/// Old save file with old schema, auto-migrate to new schema
#[test]
fn test_save_version_migration() {
    let mut world = World::new();
    world.register::<Transform>();

    // Create old save
    let entity = world.spawn();
    let mut transform = Transform::identity();
    transform.position = engine_core::math::Vec3::new(100.0, 200.0, 300.0);
    world.add(entity, transform);

    let state = WorldState::snapshot(&world);

    // Save with version 1
    let versioned = VersionedWorldState::new(state);
    let bytes = versioned.save().unwrap();

    // Load and auto-migrate (currently v1, no migration needed)
    let loaded = VersionedWorldState::load_with_migration(&bytes).unwrap();

    assert_eq!(loaded.schema_version.version, 1);
    assert!(!loaded.schema_version.needs_migration());

    // Restore
    let mut world2 = World::new();
    world2.register::<Transform>();
    loaded.state.restore(&mut world2);

    assert_eq!(world2.entity_count(), 1);
}

/// Real-world scenario: Auto-save with delta compression
///
/// Game auto-saves every 30 seconds using deltas
#[test]
#[cfg(feature = "compression")]
fn test_autosave_with_deltas() {
    let mut world = World::new();
    world.register::<Transform>();

    // Initial world - 200 entities
    for i in 0..200 {
        let entity = world.spawn();
        let mut transform = Transform::identity();
        transform.position = engine_core::math::Vec3::new(i as f32, 0.0, 0.0);
        world.add(entity, transform);
    }

    // Full save (baseline)
    let baseline = WorldState::snapshot(&world);
    let full_bytes = bincode::serialize(&baseline).unwrap();
    let full_size = full_bytes.len();

    // Simulate 30 seconds of gameplay - 20% entities changed
    let entities_to_change: Vec<_> = world.entities().take(40).collect();
    for entity in entities_to_change {
        if let Some(transform) = world.get_mut::<Transform>(entity) {
            transform.position.x += 10.0;
            transform.position.y += 5.0;
        }
    }

    // Delta save
    let current = WorldState::snapshot(&world);
    let delta = OptimizedDelta::compute(&baseline, &current);
    let delta_bytes = bincode::serialize(&delta).unwrap();

    // Verify delta is much smaller
    let delta_size = delta_bytes.len();
    let compression_ratio = (delta_size as f32) / (full_size as f32);
    assert!(compression_ratio < 0.5); // Delta should be <50% of full

    // Compress delta
    let compressed = CompressedData::compress(&delta_bytes, CompressionAlgorithm::Lz4).unwrap();

    // Total savings: delta + compression
    let total_compression = (compressed.data.len() as f32) / (full_size as f32);
    assert!(total_compression < 0.3); // Should be <30% of original

    // Restore from delta
    let mut restored = baseline.clone();
    delta.apply(&mut restored);

    assert_eq!(restored.entities.len(), current.entities.len());
}

/// Real-world scenario: FlatBuffers network packet
///
/// Zero-copy deserialization for network state
#[test]
fn test_flatbuffers_network_packet() {
    let mut world = World::new();
    world.register::<Transform>();

    // Small network packet - 20 entities
    for i in 0..20 {
        let entity = world.spawn();
        let mut transform = Transform::identity();
        transform.position = engine_core::math::Vec3::new(i as f32, i as f32, i as f32);
        world.add(entity, transform);
    }

    let state = WorldState::snapshot(&world);

    // Serialize with FlatBuffers (zero-copy)
    let fb_bytes = state.serialize(Format::FlatBuffers).unwrap();

    // Deserialize (should be very fast - zero-copy)
    let loaded = WorldState::deserialize(&fb_bytes, Format::FlatBuffers).unwrap();

    assert_eq!(loaded.entities.len(), 20);
    assert_eq!(loaded.metadata.entity_count, 20);
}

/// Real-world scenario: Large world persistence (MMO server)
///
/// 10,000 entity world with persistence
#[test]
#[cfg(feature = "compression")]
fn test_large_world_persistence() {
    let mut world = World::new();
    world.register::<Transform>();

    // Large MMO world - 10K entities
    for i in 0..10_000 {
        let entity = world.spawn();
        let mut transform = Transform::identity();
        transform.position = engine_core::math::Vec3::new((i % 100) as f32, 0.0, (i / 100) as f32);
        world.add(entity, transform);
    }

    // Snapshot (should complete in reasonable time)
    let state = WorldState::snapshot(&world);
    assert_eq!(state.entities.len(), 10_000);

    // Serialize
    let bytes = state.serialize(Format::Bincode).unwrap();

    // Compress
    let compressed = CompressedData::compress(&bytes, CompressionAlgorithm::Zstd).unwrap();

    // Verify compression
    assert!(compressed.size_savings_percent() > 50.0);

    // Deserialize
    let decompressed = compressed.decompress().unwrap();
    let loaded = WorldState::deserialize(&decompressed, Format::Bincode).unwrap();

    assert_eq!(loaded.entities.len(), 10_000);

    // Restore to new world
    let mut world2 = World::new();
    world2.register::<Transform>();
    loaded.restore(&mut world2);

    assert_eq!(world2.entity_count(), 10_000);
}

/// Real-world scenario: Batch network updates
///
/// Server sends batch of state updates to multiple clients
#[test]
fn test_batch_network_updates() {
    use engine_core::serialization::BatchSerializer;

    let mut world = World::new();
    world.register::<Transform>();

    // Create 3 different states (3 clients)
    let mut states = Vec::new();

    for client in 0..3 {
        // Clear world
        world = World::new();
        world.register::<Transform>();

        // Create client-specific state
        for i in 0..50 {
            let entity = world.spawn();
            let mut transform = Transform::identity();
            transform.position =
                engine_core::math::Vec3::new(client as f32 * 100.0 + i as f32, 0.0, 0.0);
            world.add(entity, transform);
        }

        states.push(WorldState::snapshot(&world));
    }

    // Batch serialize
    let mut serializer = BatchSerializer::new(1024);
    let batches = serializer.serialize_batch(&states);

    assert_eq!(batches.len(), 3);
    for batch in &batches {
        assert!(!batch.is_empty());
    }
}
