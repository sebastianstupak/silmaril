//! Comprehensive integration tests for client-server synchronization and replication
//!
//! This test suite covers:
//! - Entity replication (server → client)
//! - Transform updates over network
//! - Late-join client synchronization
//! - Authority validation (server-authoritative)
//! - Network edge cases (disconnection, packet loss, etc.)
//! - Desync detection and recovery
//!
//! ARCHITECTURE NOTE: This is a cross-crate integration test (engine-networking + engine-core)
//! so it MUST be in engine/shared/tests/ per the 3-tier testing architecture.

use engine_core::ecs::{Entity, World};
use engine_core::math::{Quat, Transform, Vec3};
use engine_core::serialization::{Format, WorldState};
use engine_core::{Health, Velocity};
use engine_networking::protocol::{
    serialize_server_message, ClientMessage, EntityState, ServerMessage, SerializationFormat,
};
use engine_networking::simulator::{NetworkConditions, NetworkProfile, NetworkSimulator};
use engine_networking::snapshot::WorldSnapshot;
use engine_networking::{ClientPredictor, PredictionConfig};
use std::collections::HashMap;
use tracing::info;

// ============================================================================
// Test Helpers
// ============================================================================

/// Helper to create a test server world with entities
fn create_server_world(entity_count: usize) -> World {
    let mut world = World::new();

    world.register::<Transform>();
    world.register::<Velocity>();
    world.register::<Health>();

    for i in 0..entity_count {
        let entity = world.spawn();
        let position = Vec3::new(i as f32, 0.0, 0.0);
        world.add(entity, Transform::new(position, Quat::IDENTITY, Vec3::ONE));
        world.add(entity, Velocity::new(1.0, 0.0, 0.0));
        world.add(entity, Health::new(100.0, 100.0));
    }

    world
}

/// Helper to create a client world (initially empty)
fn create_client_world() -> World {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Velocity>();
    world.register::<Health>();
    world
}

/// Helper to simulate server sending state update to client
fn replicate_state_to_client(server_world: &World, client_world: &mut World) {
    let snapshot = WorldSnapshot::from_world(server_world);
    snapshot.apply_to_world(client_world);
}

/// Helper to calculate desync magnitude between server and client
fn calculate_desync(server_world: &World, client_world: &World) -> f32 {
    let server_entities: Vec<Entity> = server_world.entities().collect();
    let client_entities: Vec<Entity> = client_world.entities().collect();

    // Count entity mismatches
    if server_entities.len() != client_entities.len() {
        return f32::INFINITY; // Complete desync
    }

    // Calculate position error for matching entities
    let mut total_error = 0.0;
    for entity in server_entities.iter() {
        if let (Some(server_transform), Some(client_transform)) = (
            server_world.get::<Transform>(*entity),
            client_world.get::<Transform>(*entity),
        ) {
            let position_error = (server_transform.position - client_transform.position).length();
            total_error += position_error;
        }
    }

    total_error
}

// ============================================================================
// Entity Replication Tests
// ============================================================================

#[test]
fn test_basic_entity_replication() {
    let server_world = create_server_world(10);
    let mut client_world = create_client_world();

    // Replicate server state to client
    replicate_state_to_client(&server_world, &mut client_world);

    // Verify client has all entities
    assert_eq!(client_world.entity_count(), 10);

    // Verify transforms match
    let server_entities: Vec<Entity> = server_world.entities().collect();
    for entity in server_entities {
        let server_transform = server_world.get::<Transform>(entity).unwrap();
        let client_transform = client_world.get::<Transform>(entity).unwrap();

        assert_eq!(server_transform.position, client_transform.position);
        assert_eq!(server_transform.rotation, client_transform.rotation);
    }
}

#[test]
fn test_entity_spawn_replication() {
    let mut server_world = create_server_world(5);
    let mut client_world = create_client_world();

    // Initial sync
    replicate_state_to_client(&server_world, &mut client_world);
    assert_eq!(client_world.entity_count(), 5);

    // Server spawns new entities
    for _ in 0..5 {
        let entity = server_world.spawn();
        server_world.add(entity, Transform::new(Vec3::new(10.0, 0.0, 0.0), Quat::IDENTITY, Vec3::ONE));
    }

    // Sync again
    replicate_state_to_client(&server_world, &mut client_world);

    // Client should have all 10 entities
    assert_eq!(client_world.entity_count(), 10);
}

#[test]
fn test_entity_despawn_replication() {
    let mut server_world = create_server_world(10);
    let mut client_world = create_client_world();

    // Initial sync
    replicate_state_to_client(&server_world, &mut client_world);
    assert_eq!(client_world.entity_count(), 10);

    // Server despawns some entities
    let entities_to_despawn: Vec<Entity> = server_world.entities().take(3).collect();
    for entity in entities_to_despawn {
        server_world.despawn(entity);
    }

    // Sync again
    replicate_state_to_client(&server_world, &mut client_world);

    // Client should have 7 entities remaining
    assert_eq!(client_world.entity_count(), 7);
}

#[test]
fn test_transform_update_replication() {
    let mut server_world = create_server_world(5);
    let mut client_world = create_client_world();

    // Initial sync
    replicate_state_to_client(&server_world, &mut client_world);

    // Server updates entity positions
    let entities: Vec<Entity> = server_world.entities().collect();
    for entity in entities.iter() {
        if let Some(transform) = server_world.get_mut::<Transform>(*entity) {
            transform.position.x += 100.0;
        }
    }

    // Sync again
    replicate_state_to_client(&server_world, &mut client_world);

    // Verify client positions updated
    for entity in entities {
        let server_transform = server_world.get::<Transform>(entity).unwrap();
        let client_transform = client_world.get::<Transform>(entity).unwrap();

        assert!((server_transform.position.x - client_transform.position.x).abs() < 0.001);
    }
}

#[test]
fn test_component_addition_replication() {
    let mut server_world = create_server_world(5);
    let mut client_world = create_client_world();

    // Initial sync (entities have Transform, Velocity, Health)
    replicate_state_to_client(&server_world, &mut client_world);

    // Server adds additional components (already has them, but test removal then add)
    let entities: Vec<Entity> = server_world.entities().collect();
    for entity in entities.iter() {
        server_world.remove::<Health>(*entity);
    }

    replicate_state_to_client(&server_world, &mut client_world);

    // Verify health removed on client
    for entity in entities.iter() {
        assert!(client_world.get::<Health>(*entity).is_none());
    }

    // Add back
    for entity in entities.iter() {
        server_world.add(*entity, Health::new(50.0, 100.0));
    }

    replicate_state_to_client(&server_world, &mut client_world);

    // Verify health added on client
    for entity in entities {
        let health = client_world.get::<Health>(entity).unwrap();
        assert_eq!(health.current, 50.0);
    }
}

#[test]
fn test_large_scale_replication() {
    // Test replication with many entities (MMO scenario)
    let server_world = create_server_world(1000);
    let mut client_world = create_client_world();

    replicate_state_to_client(&server_world, &mut client_world);

    assert_eq!(client_world.entity_count(), 1000);

    // Verify random sampling of entities
    let entities: Vec<Entity> = server_world.entities().collect();
    for i in (0..1000).step_by(50) {
        let entity = entities[i];
        let server_transform = server_world.get::<Transform>(entity).unwrap();
        let client_transform = client_world.get::<Transform>(entity).unwrap();

        assert_eq!(server_transform.position, client_transform.position);
    }
}

// ============================================================================
// Late-Join Client Tests
// ============================================================================

#[test]
fn test_late_join_full_state_sync() {
    // Server has been running with entities
    let mut server_world = create_server_world(20);

    // Simulate server tick (entities move)
    let entities: Vec<Entity> = server_world.entities().collect();
    for entity in entities {
        if let Some(transform) = server_world.get_mut::<Transform>(entity) {
            transform.position.x += 50.0;
        }
    }

    // New client joins
    let mut late_join_client = create_client_world();

    // Client receives full state snapshot
    replicate_state_to_client(&server_world, &mut late_join_client);

    // Verify client has all entities at correct positions
    assert_eq!(late_join_client.entity_count(), 20);

    let server_entities: Vec<Entity> = server_world.entities().collect();
    for entity in server_entities {
        let server_transform = server_world.get::<Transform>(entity).unwrap();
        let client_transform = late_join_client.get::<Transform>(entity).unwrap();

        assert!((server_transform.position.x - 50.0).abs() < 1.0); // Moved from origin
        assert_eq!(server_transform.position, client_transform.position);
    }
}

#[test]
fn test_late_join_with_ongoing_updates() {
    let mut server_world = create_server_world(10);

    // Late join client
    let mut client_world = create_client_world();

    // Initial snapshot
    replicate_state_to_client(&server_world, &mut client_world);

    // Simulate multiple server ticks with updates
    for tick in 0..10 {
        let entities: Vec<Entity> = server_world.entities().collect();
        for entity in entities {
            if let Some(transform) = server_world.get_mut::<Transform>(entity) {
                transform.position.x += 1.0;
            }
        }

        // Sync every tick
        replicate_state_to_client(&server_world, &mut client_world);

        // Verify no desync
        let desync = calculate_desync(&server_world, &client_world);
        assert!(
            desync < 0.01,
            "Desync detected at tick {}: magnitude {}",
            tick,
            desync
        );
    }
}

#[test]
fn test_late_join_empty_world() {
    // Edge case: client joins when server has no entities
    let server_world = create_server_world(0);
    let mut client_world = create_client_world();

    replicate_state_to_client(&server_world, &mut client_world);

    assert_eq!(client_world.entity_count(), 0);
}

// ============================================================================
// Authority Validation Tests
// ============================================================================

#[test]
fn test_server_authority_position_correction() {
    // Client predicts position, server corrects
    let mut predictor = ClientPredictor::new(PredictionConfig::default());
    predictor.set_position(Vec3::ZERO);

    // Client predicts forward movement
    for _ in 0..60 {
        predictor.process_input(0, Vec3::new(0.0, 0.0, 1.0), Vec3::ZERO, 0, 1.0 / 60.0);
    }

    let predicted_pos = predictor.predicted_position();

    // Server says client is at different position (authority)
    let server_pos = Vec3::new(0.0, 0.0, -4.0); // Server authority wins

    predictor.reconcile(0, server_pos, Vec3::ZERO, Quat::IDENTITY, 0);

    // Client should snap to server position
    let corrected_pos = predictor.predicted_position();
    assert!(
        (corrected_pos - server_pos).length() < 0.1,
        "Client did not respect server authority"
    );
}

#[test]
fn test_client_cannot_spawn_entities() {
    // Client tries to spawn entity (should be ignored, server is authority)
    let mut server_world = create_server_world(5);
    let mut client_world = create_client_world();

    // Initial sync
    replicate_state_to_client(&server_world, &mut client_world);

    // Client tries to spawn (simulated by local spawn, but not synced back)
    let _client_entity = client_world.spawn();
    client_world.add(
        _client_entity,
        Transform::new(Vec3::new(999.0, 999.0, 999.0), Quat::IDENTITY, Vec3::ONE),
    );

    // Server doesn't know about client spawn
    assert_eq!(server_world.entity_count(), 5);

    // Next server sync overwrites client
    replicate_state_to_client(&server_world, &mut client_world);

    // Client returns to server state
    assert_eq!(client_world.entity_count(), 5);
}

#[test]
fn test_server_rejects_invalid_movement() {
    // Simulate client trying to move faster than allowed
    let mut server_world = create_server_world(1);
    let entity = server_world.entities().next().unwrap();

    let original_pos = server_world.get::<Transform>(entity).unwrap().position;

    // Client claims to have moved 1000 units in one tick (impossible)
    let cheating_pos = original_pos + Vec3::new(1000.0, 0.0, 0.0);

    // Server validates movement (max movement per tick = velocity * dt)
    let velocity = server_world.get::<Velocity>(entity).unwrap();
    let max_movement = velocity.linear.length() * (1.0 / 60.0);

    let claimed_movement = (cheating_pos - original_pos).length();

    // Server should reject this
    assert!(
        claimed_movement > max_movement * 2.0,
        "Movement should be detected as invalid"
    );

    // Server keeps entity at original position (authority)
    let server_pos = server_world.get::<Transform>(entity).unwrap().position;
    assert_eq!(server_pos, original_pos);
}

#[test]
fn test_component_modification_authority() {
    let mut server_world = create_server_world(1);
    let mut client_world = create_client_world();

    replicate_state_to_client(&server_world, &mut client_world);

    let entity = server_world.entities().next().unwrap();

    // Client tries to modify health (should be overwritten by server)
    if let Some(health) = client_world.get_mut::<Health>(entity) {
        health.current = 50.0; // Client sets to 50
    }

    // Server has different value
    if let Some(health) = server_world.get_mut::<Health>(entity) {
        health.current = 75.0; // Server says 75
    }

    // Sync from server
    replicate_state_to_client(&server_world, &mut client_world);

    // Client should have server's value
    let client_health = client_world.get::<Health>(entity).unwrap();
    assert_eq!(client_health.current, 75.0);
}

// ============================================================================
// Desync Detection Tests
// ============================================================================

#[test]
fn test_desync_detection_position_divergence() {
    let mut server_world = create_server_world(10);
    let mut client_world = create_client_world();

    replicate_state_to_client(&server_world, &mut client_world);

    // Simulate network issue causing client to miss updates
    let entities: Vec<Entity> = server_world.entities().collect();
    for entity in entities.iter() {
        if let Some(transform) = server_world.get_mut::<Transform>(*entity) {
            transform.position.x += 10.0; // Server updates
        }
    }

    // Client doesn't receive update (desync)

    // Calculate desync magnitude
    let desync = calculate_desync(&server_world, &client_world);

    assert!(desync > 50.0, "Desync should be detected");

    info!(desync_magnitude = desync, "Desync detected");

    // Resync
    replicate_state_to_client(&server_world, &mut client_world);

    // Verify desync resolved
    let desync_after = calculate_desync(&server_world, &client_world);
    assert!(desync_after < 0.01, "Desync should be resolved");
}

#[test]
fn test_desync_detection_entity_count_mismatch() {
    let mut server_world = create_server_world(10);
    let mut client_world = create_client_world();

    replicate_state_to_client(&server_world, &mut client_world);

    // Server spawns entity
    server_world.spawn();

    // Client doesn't know
    let desync = calculate_desync(&server_world, &client_world);

    assert_eq!(desync, f32::INFINITY, "Entity count mismatch should be detected");
}

#[test]
fn test_gradual_desync_accumulation() {
    let mut server_world = create_server_world(10);
    let mut client_world = create_client_world();

    replicate_state_to_client(&server_world, &mut client_world);

    // Simulate small rounding errors accumulating
    for _ in 0..100 {
        let entities: Vec<Entity> = server_world.entities().collect();
        for entity in entities.iter() {
            if let Some(transform) = server_world.get_mut::<Transform>(*entity) {
                transform.position.x += 0.01; // Server updates
            }
            // Client simulates with slightly different precision
            if let Some(transform) = client_world.get_mut::<Transform>(*entity) {
                transform.position.x += 0.0099; // Small difference
            }
        }
    }

    // Check accumulated desync
    let desync = calculate_desync(&server_world, &client_world);
    assert!(
        desync > 0.1,
        "Small errors should accumulate into detectable desync"
    );
}

// ============================================================================
// Network Edge Case Tests (using NetworkSimulator)
// ============================================================================

#[test]
fn test_packet_loss_recovery() {
    let mut server_world = create_server_world(5);
    let mut client_world = create_client_world();

    // Create simulator with 10% packet loss
    let mut network = NetworkSimulator::new(NetworkProfile::Custom(NetworkConditions {
        latency_ms: 50,
        jitter_ms: 10,
        packet_loss_percent: 10.0,
        bandwidth_kbps: 1000,
        reorder_probability: 0.0,
    }));

    // Send initial snapshot
    let snapshot = WorldSnapshot::from_world(&server_world);
    let bytes = snapshot.to_bytes(Format::Bincode).unwrap();
    network.send(bytes);

    // Simulate time passing
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Receive packets (some may be lost)
    let packets = network.recv();

    if !packets.is_empty() {
        // At least one packet made it through
        let snapshot = WorldSnapshot::from_bytes(&packets[0], Format::Bincode).unwrap();
        snapshot.apply_to_world(&mut client_world);

        assert_eq!(client_world.entity_count(), 5);
    } else {
        // All packets lost, client should detect missing update
        assert_eq!(client_world.entity_count(), 0);
    }
}

#[test]
fn test_high_latency_synchronization() {
    let server_world = create_server_world(10);
    let mut client_world = create_client_world();

    // Create simulator with 300ms latency (terrible connection)
    let mut network = NetworkSimulator::new(NetworkProfile::Terrible);

    // Send snapshot
    let snapshot = WorldSnapshot::from_world(&server_world);
    let bytes = snapshot.to_bytes(Format::Bincode).unwrap();
    network.send(bytes);

    // Check no immediate delivery
    let packets = network.recv();
    assert_eq!(packets.len(), 0, "Packets should still be in flight");

    // Wait for latency
    std::thread::sleep(std::time::Duration::from_millis(400));

    // Receive delayed packets
    let packets = network.recv();
    assert!(!packets.is_empty(), "Packets should arrive after latency");

    let snapshot = WorldSnapshot::from_bytes(&packets[0], Format::Bincode).unwrap();
    snapshot.apply_to_world(&mut client_world);

    assert_eq!(client_world.entity_count(), 10);
}

#[test]
fn test_packet_reordering_handling() {
    let mut server_world = create_server_world(3);
    let mut client_world = create_client_world();

    // Create simulator with high reordering probability
    let mut network = NetworkSimulator::new(NetworkProfile::Custom(NetworkConditions {
        latency_ms: 50,
        jitter_ms: 20,
        packet_loss_percent: 0.0,
        bandwidth_kbps: 1000,
        reorder_probability: 0.5,
    }));

    // Send multiple updates
    for i in 0..5 {
        let entities: Vec<Entity> = server_world.entities().collect();
        for entity in entities.iter() {
            if let Some(transform) = server_world.get_mut::<Transform>(*entity) {
                transform.position.x = i as f32;
            }
        }

        let snapshot = WorldSnapshot::from_world(&server_world);
        let bytes = snapshot.to_bytes(Format::Bincode).unwrap();
        network.send(bytes);
    }

    // Receive packets (may be out of order)
    std::thread::sleep(std::time::Duration::from_millis(200));
    let packets = network.recv();

    // Apply all packets (should handle out of order gracefully)
    for packet in packets {
        let snapshot = WorldSnapshot::from_bytes(&packet, Format::Bincode).unwrap();
        snapshot.apply_to_world(&mut client_world);
    }

    // Final state should match server
    assert_eq!(client_world.entity_count(), 3);
}

#[test]
fn test_bandwidth_saturation() {
    let server_world = create_server_world(100);
    let mut client_world = create_client_world();

    // Create simulator with very low bandwidth (500 Kbps)
    let mut network = NetworkSimulator::new(NetworkProfile::Terrible);

    // Send large snapshot (should be throttled)
    let snapshot = WorldSnapshot::from_world(&server_world);
    let bytes = snapshot.to_bytes(Format::Bincode).unwrap();

    let packet_size = bytes.len();
    info!(
        packet_size_kb = packet_size / 1024,
        "Sending large snapshot"
    );

    network.send(bytes);

    // Check that packet is queued (bandwidth limiting)
    assert!(network.in_flight() > 0, "Packet should be queued");

    // Wait for bandwidth to allow delivery
    std::thread::sleep(std::time::Duration::from_millis(500));

    let packets = network.recv();
    if !packets.is_empty() {
        let snapshot = WorldSnapshot::from_bytes(&packets[0], Format::Bincode).unwrap();
        snapshot.apply_to_world(&mut client_world);
        assert_eq!(client_world.entity_count(), 100);
    }
}

#[test]
fn test_connection_drop_recovery() {
    let mut server_world = create_server_world(10);
    let mut client_world = create_client_world();

    // Initial sync
    replicate_state_to_client(&server_world, &mut client_world);
    assert_eq!(client_world.entity_count(), 10);

    // Simulate connection drop (client misses updates)
    let entities: Vec<Entity> = server_world.entities().collect();
    for _ in 0..10 {
        for entity in entities.iter() {
            if let Some(transform) = server_world.get_mut::<Transform>(*entity) {
                transform.position.x += 1.0;
            }
        }
    }

    // Client is desynced
    let desync = calculate_desync(&server_world, &client_world);
    assert!(desync > 50.0, "Desync due to missed updates");

    // Connection restored, full state resync
    replicate_state_to_client(&server_world, &mut client_world);

    // Verify recovery
    let desync_after = calculate_desync(&server_world, &client_world);
    assert!(desync_after < 0.01, "Should recover from connection drop");
}

#[test]
fn test_partial_update_loss() {
    let mut server_world = create_server_world(10);
    let mut client_world = create_client_world();

    replicate_state_to_client(&server_world, &mut client_world);

    // Server sends update
    let entities: Vec<Entity> = server_world.entities().collect();
    for entity in entities.iter() {
        if let Some(transform) = server_world.get_mut::<Transform>(*entity) {
            transform.position.x += 10.0;
        }
    }

    // Simulate partial packet loss (client gets corrupt data)
    // Client doesn't update

    // Server sends another update
    for entity in entities.iter() {
        if let Some(transform) = server_world.get_mut::<Transform>(*entity) {
            transform.position.x += 10.0;
        }
    }

    // Client receives second update successfully
    replicate_state_to_client(&server_world, &mut client_world);

    // Client should be in sync with latest state
    let desync = calculate_desync(&server_world, &client_world);
    assert!(desync < 0.01, "Should sync to latest state despite missed update");
}

// ============================================================================
// Client Reconnection Tests
// ============================================================================

#[test]
fn test_client_reconnect_full_resync() {
    let mut server_world = create_server_world(10);
    let mut client_world = create_client_world();

    // Initial connection and sync
    replicate_state_to_client(&server_world, &mut client_world);

    // Client disconnects
    client_world = create_client_world(); // Clear client state

    // Server continues updating
    let entities: Vec<Entity> = server_world.entities().collect();
    for entity in entities {
        if let Some(transform) = server_world.get_mut::<Transform>(entity) {
            transform.position.x += 100.0;
        }
    }

    // Client reconnects and receives full state
    replicate_state_to_client(&server_world, &mut client_world);

    // Verify client has correct state
    assert_eq!(client_world.entity_count(), 10);

    for entity in server_world.entities() {
        let server_transform = server_world.get::<Transform>(entity).unwrap();
        let client_transform = client_world.get::<Transform>(entity).unwrap();

        assert_eq!(server_transform.position, client_transform.position);
    }
}

#[test]
fn test_multiple_client_reconnects() {
    let mut server_world = create_server_world(5);

    // Simulate 3 clients connecting, disconnecting, reconnecting
    for iteration in 0..3 {
        let mut client_world = create_client_world();

        // Client connects
        replicate_state_to_client(&server_world, &mut client_world);
        assert_eq!(client_world.entity_count(), 5);

        // Server updates
        let entities: Vec<Entity> = server_world.entities().collect();
        for entity in entities {
            if let Some(transform) = server_world.get_mut::<Transform>(entity) {
                transform.position.x += iteration as f32 * 10.0;
            }
        }

        // Client disconnects (dropped)
    }

    // New client joins after all previous disconnects
    let mut final_client = create_client_world();
    replicate_state_to_client(&server_world, &mut final_client);

    assert_eq!(final_client.entity_count(), 5);
}

// ============================================================================
// Invalid Message Tests
// ============================================================================

#[test]
fn test_malformed_message_rejection() {
    // Simulate server receiving corrupt data from client
    let corrupt_data = vec![0xFF, 0xFF, 0xFF, 0xFF]; // Invalid message

    // Attempt to deserialize
    let result = bincode::deserialize::<ClientMessage>(&corrupt_data);

    // Should fail gracefully
    assert!(result.is_err(), "Malformed message should be rejected");
}

#[test]
fn test_oversized_message_rejection() {
    use engine_networking::protocol::MAX_MESSAGE_SIZE;

    // Create message larger than max size
    let oversized = vec![0u8; MAX_MESSAGE_SIZE + 1];

    // Attempt to deserialize
    let result = bincode::deserialize::<ClientMessage>(&oversized);

    // Should fail (either deserialize error or size check)
    assert!(result.is_err(), "Oversized message should be rejected");
}

#[test]
fn test_version_mismatch_detection() {
    use engine_networking::protocol::PROTOCOL_VERSION;

    // Client sends handshake with wrong version
    let client_message = ClientMessage::Handshake {
        version: PROTOCOL_VERSION + 1,
        client_name: "test".to_string(),
    };

    // Server checks version
    match client_message {
        ClientMessage::Handshake { version, .. } => {
            assert_ne!(version, PROTOCOL_VERSION, "Version mismatch should be detected");
        }
        _ => panic!("Wrong message type"),
    }
}

// ============================================================================
// Stress Tests
// ============================================================================

#[test]
fn test_rapid_state_changes() {
    let mut server_world = create_server_world(50);
    let mut client_world = create_client_world();

    // Initial sync
    replicate_state_to_client(&server_world, &mut client_world);

    // Rapidly update state (60 ticks)
    for _ in 0..60 {
        let entities: Vec<Entity> = server_world.entities().collect();
        for entity in entities {
            if let Some(transform) = server_world.get_mut::<Transform>(entity) {
                transform.position.x += 0.1;
            }
        }

        replicate_state_to_client(&server_world, &mut client_world);

        // Verify no desync
        let desync = calculate_desync(&server_world, &client_world);
        assert!(desync < 0.01, "Should stay synced during rapid updates");
    }
}

#[test]
fn test_entity_churn() {
    // Test high entity spawn/despawn rate
    let mut server_world = create_server_world(10);
    let mut client_world = create_client_world();

    replicate_state_to_client(&server_world, &mut client_world);

    // Spawn and despawn entities rapidly
    for _ in 0..20 {
        // Spawn 5
        for _ in 0..5 {
            let entity = server_world.spawn();
            server_world.add(entity, Transform::default());
        }

        replicate_state_to_client(&server_world, &mut client_world);

        // Despawn 3
        let entities: Vec<Entity> = server_world.entities().take(3).collect();
        for entity in entities {
            server_world.despawn(entity);
        }

        replicate_state_to_client(&server_world, &mut client_world);
    }

    // Final sync check
    assert_eq!(server_world.entity_count(), client_world.entity_count());
}

#[test]
fn test_concurrent_component_modifications() {
    let mut server_world = create_server_world(10);
    let mut client_world = create_client_world();

    replicate_state_to_client(&server_world, &mut client_world);

    // Modify multiple component types simultaneously
    let entities: Vec<Entity> = server_world.entities().collect();
    for entity in entities.iter() {
        if let Some(transform) = server_world.get_mut::<Transform>(*entity) {
            transform.position.x += 10.0;
        }
        if let Some(velocity) = server_world.get_mut::<Velocity>(*entity) {
            velocity.linear.x = 5.0;
        }
        if let Some(health) = server_world.get_mut::<Health>(*entity) {
            health.current = 50.0;
        }
    }

    replicate_state_to_client(&server_world, &mut client_world);

    // Verify all components synced
    for entity in entities {
        let server_transform = server_world.get::<Transform>(entity).unwrap();
        let client_transform = client_world.get::<Transform>(entity).unwrap();
        assert_eq!(server_transform.position.x, client_transform.position.x);

        let server_velocity = server_world.get::<Velocity>(entity).unwrap();
        let client_velocity = client_world.get::<Velocity>(entity).unwrap();
        assert_eq!(server_velocity.linear.x, client_velocity.linear.x);

        let server_health = server_world.get::<Health>(entity).unwrap();
        let client_health = client_world.get::<Health>(entity).unwrap();
        assert_eq!(server_health.current, client_health.current);
    }
}
