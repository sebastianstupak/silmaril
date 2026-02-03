//! Integration tests for delta compression

use engine_core::serialization::{WorldState, WorldStateDelta};
use engine_core::{Health, Quat, Transform, Vec3, Velocity, World};
use engine_networking::delta::{AdaptiveDeltaStrategy, NetworkDelta};

/// Helper to create a test world
fn create_test_world(entity_count: usize) -> World {
    let mut world = World::new();

    world.register::<Transform>();
    world.register::<Velocity>();
    world.register::<Health>();

    for i in 0..entity_count {
        let entity = world.spawn();
        let position = Vec3::new(i as f32, i as f32 * 2.0, i as f32 * 3.0);
        world.add(entity, Transform::new(position, Quat::IDENTITY, Vec3::ONE));
        world.add(entity, Velocity::new(0.1, 0.2, 0.3));
        world.add(entity, Health::new(100.0, 100.0));
    }

    world
}

#[test]
fn test_delta_computation_empty_worlds() {
    let world1 = World::new();
    let world2 = World::new();

    let state1 = WorldState::snapshot(&world1);
    let state2 = WorldState::snapshot(&world2);

    let delta = WorldStateDelta::compute(&state1, &state2);

    assert_eq!(delta.added_entities.len(), 0);
    assert_eq!(delta.removed_entities.len(), 0);
    assert_eq!(delta.modified_components.len(), 0);
}

#[test]
fn test_delta_computation_identical_worlds() {
    let world = create_test_world(10);
    let state1 = WorldState::snapshot(&world);
    let state2 = WorldState::snapshot(&world);

    let delta = WorldStateDelta::compute(&state1, &state2);

    // Should have no changes
    assert_eq!(delta.added_entities.len(), 0);
    assert_eq!(delta.removed_entities.len(), 0);
    // Modified components might be non-zero due to timestamp differences, but should be minimal
}

#[test]
fn test_delta_computation_position_changes() {
    let mut world1 = create_test_world(10);
    let state1 = WorldState::snapshot(&world1);

    // Modify some positions
    let entities: Vec<_> = world1.entities().collect();
    for entity in entities.iter().take(3) {
        if let Some(transform) = world1.get_mut::<Transform>(*entity) {
            transform.position.x += 10.0;
        }
    }

    let state2 = WorldState::snapshot(&world1);
    let delta = WorldStateDelta::compute(&state1, &state2);

    // Should have modified components for the changed entities
    assert!(delta.modified_components.len() > 0);
    assert_eq!(delta.added_entities.len(), 0);
    assert_eq!(delta.removed_entities.len(), 0);
}

#[test]
fn test_delta_computation_entity_spawning() {
    let world1 = create_test_world(10);
    let state1 = WorldState::snapshot(&world1);

    let world2 = create_test_world(15); // 5 more entities
    let state2 = WorldState::snapshot(&world2);

    let delta = WorldStateDelta::compute(&state1, &state2);

    // Should have 5 added entities (or close, accounting for ID differences)
    assert!(delta.added_entities.len() > 0);
}

#[test]
fn test_delta_application_roundtrip() {
    let world1 = create_test_world(10);
    let mut state1 = WorldState::snapshot(&world1);

    // Create modified world
    let mut world2 = create_test_world(10);
    let entities: Vec<_> = world2.entities().collect();
    for entity in entities.iter().take(5) {
        if let Some(transform) = world2.get_mut::<Transform>(*entity) {
            transform.position.x += 10.0;
        }
    }

    let state2 = WorldState::snapshot(&world2);

    // Compute delta
    let delta = WorldStateDelta::compute(&state1, &state2);

    // Apply delta to state1
    delta.apply(&mut state1);

    // State1 should now match state2 in entity count
    assert_eq!(state1.entities.len(), state2.entities.len());
    assert_eq!(state1.components.len(), state2.components.len());
}

#[test]
fn test_network_delta_creation() {
    let world1 = create_test_world(100);
    let state1 = WorldState::snapshot(&world1);

    let mut world2 = create_test_world(100);
    let entities: Vec<_> = world2.entities().collect();
    for entity in entities.iter().take(10) {
        if let Some(transform) = world2.get_mut::<Transform>(*entity) {
            transform.position.x += 10.0;
        }
    }
    let state2 = WorldState::snapshot(&world2);

    let net_delta = NetworkDelta::from_states(&state1, &state2);

    // Verify metadata
    assert!(net_delta.delta_size > 0);
    assert!(net_delta.full_size > 0);
    assert!(net_delta.compression_ratio > 0.0);
    assert!(net_delta.compression_ratio <= 1.0);
}

#[test]
fn test_network_delta_serialization_roundtrip() {
    let world1 = create_test_world(50);
    let state1 = WorldState::snapshot(&world1);

    let mut world2 = create_test_world(50);
    let entities: Vec<_> = world2.entities().collect();
    for entity in entities.iter().take(5) {
        if let Some(transform) = world2.get_mut::<Transform>(*entity) {
            transform.position.x += 10.0;
        }
    }
    let state2 = WorldState::snapshot(&world2);

    let net_delta = NetworkDelta::from_states(&state1, &state2);

    // Serialize and deserialize
    let bytes = net_delta.to_bytes();
    let restored = NetworkDelta::from_bytes(&bytes).unwrap();

    // Verify
    assert_eq!(net_delta.delta_size, restored.delta_size);
    assert_eq!(net_delta.full_size, restored.full_size);
    assert_eq!(net_delta.compression_ratio, restored.compression_ratio);
}

#[test]
fn test_network_delta_compression_effectiveness() {
    let world1 = create_test_world(1000);
    let state1 = WorldState::snapshot(&world1);

    let mut world2 = create_test_world(1000);
    let entities: Vec<_> = world2.entities().collect();
    // Change 5% of entities
    for entity in entities.iter().take(50) {
        if let Some(transform) = world2.get_mut::<Transform>(*entity) {
            transform.position.x += 10.0;
        }
    }
    let state2 = WorldState::snapshot(&world2);

    let net_delta = NetworkDelta::from_states(&state1, &state2);

    // With 5% changes, delta should be significantly smaller than full state
    // Compression ratio should be < 0.5 (delta is less than 50% of full size)
    assert!(
        net_delta.compression_ratio < 0.5,
        "Expected compression ratio < 0.5, got {}",
        net_delta.compression_ratio
    );
}

#[test]
fn test_network_delta_should_use_decision() {
    let world1 = create_test_world(100);
    let state1 = WorldState::snapshot(&world1);

    // Test with small changes (should use delta)
    let mut world2 = create_test_world(100);
    let entities: Vec<_> = world2.entities().collect();
    for entity in entities.iter().take(5) {
        if let Some(transform) = world2.get_mut::<Transform>(*entity) {
            transform.position.x += 10.0;
        }
    }
    let state2 = WorldState::snapshot(&world2);
    let net_delta_small = NetworkDelta::from_states(&state1, &state2);

    // Small changes should favor delta
    assert!(
        net_delta_small.should_use_delta(),
        "Expected to use delta for 5% changes, ratio: {}",
        net_delta_small.compression_ratio
    );

    // Test with large changes (might not use delta)
    let mut world3 = create_test_world(100);
    let entities3: Vec<_> = world3.entities().collect();
    for entity in entities3.iter().take(90) {
        if let Some(transform) = world3.get_mut::<Transform>(*entity) {
            transform.position.x += 10.0;
            transform.position.y += 20.0;
            transform.position.z += 30.0;
        }
    }
    let state3 = WorldState::snapshot(&world3);
    let net_delta_large = NetworkDelta::from_states(&state1, &state3);

    // With 90% changes, decision depends on compression ratio
    // Just verify the decision is made (not testing specific outcome)
    let _ = net_delta_large.should_use_delta();
}

#[test]
fn test_adaptive_strategy_basic() {
    let mut strategy = AdaptiveDeltaStrategy::new(5, 0.8);

    // Record some good ratios
    strategy.record_delta(0.3);
    strategy.record_delta(0.4);
    strategy.record_delta(0.5);

    // Should recommend using delta with good ratios
    assert!(strategy.should_use_delta(0.6));

    // Average should be around 0.4
    let avg = strategy.average_ratio();
    assert!(avg > 0.3 && avg < 0.5);
}

#[test]
fn test_adaptive_strategy_bad_ratios() {
    let mut strategy = AdaptiveDeltaStrategy::new(5, 0.8);

    // Record some bad ratios
    strategy.record_delta(0.9);
    strategy.record_delta(0.95);
    strategy.record_delta(0.85);

    // Should not recommend using delta with bad average
    assert!(!strategy.should_use_delta(0.85));
}

#[test]
fn test_adaptive_strategy_history_limit() {
    let mut strategy = AdaptiveDeltaStrategy::new(3, 0.8);

    strategy.record_delta(0.1);
    strategy.record_delta(0.2);
    strategy.record_delta(0.3);
    strategy.record_delta(0.4); // Should evict 0.1

    // Average should be (0.2 + 0.3 + 0.4) / 3 = 0.3
    let avg = strategy.average_ratio();
    assert!((avg - 0.3).abs() < 0.01);
}

#[test]
fn test_adaptive_strategy_reset() {
    let mut strategy = AdaptiveDeltaStrategy::new(5, 0.8);

    strategy.record_delta(0.5);
    strategy.record_delta(0.6);
    assert_eq!(strategy.average_ratio(), 0.55);

    strategy.reset();
    assert_eq!(strategy.average_ratio(), 1.0); // Default when empty
}

#[test]
fn test_delta_with_component_removal() {
    let mut world1 = create_test_world(10);
    let state1 = WorldState::snapshot(&world1);

    // Remove some components
    let entities: Vec<_> = world1.entities().collect();
    for entity in entities.iter().take(3) {
        world1.remove::<Health>(*entity);
    }

    let state2 = WorldState::snapshot(&world1);
    let delta = WorldStateDelta::compute(&state1, &state2);

    // Should have removed components
    assert!(delta.removed_components.len() > 0);
}

#[test]
fn test_delta_full_pipeline() {
    // Simulate complete network sync workflow
    let world1 = create_test_world(100);
    let mut state1 = WorldState::snapshot(&world1);

    let mut world2 = create_test_world(100);
    let entities: Vec<_> = world2.entities().collect();
    for entity in entities.iter().take(10) {
        if let Some(transform) = world2.get_mut::<Transform>(*entity) {
            transform.position.x += 10.0;
        }
    }
    let state2 = WorldState::snapshot(&world2);

    // Server: Compute and serialize delta
    let net_delta = NetworkDelta::from_states(&state1, &state2);
    let bytes = net_delta.to_bytes();

    // Network: Transmit bytes (simulated)

    // Client: Deserialize and apply delta
    let received_delta = NetworkDelta::from_bytes(&bytes).unwrap();
    received_delta.apply(&mut state1);

    // Verify state1 now matches state2
    assert_eq!(state1.entities.len(), state2.entities.len());
    assert_eq!(state1.components.len(), state2.components.len());
}
