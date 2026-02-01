//! Comprehensive tests for client-side prediction
//!
//! Tests cover:
//! - Input buffering correctness
//! - State reconciliation
//! - Input replay
//! - Error smoothing
//! - Edge cases (large errors, network lag)

use engine_core::ecs::{EntityAllocator, World};
use engine_math::{Quat, Transform, Vec3};
use engine_physics::{
    prediction::{InputBuffer, PlayerInput, PredictedState, PredictionSystem},
    Collider, PhysicsConfig, PhysicsWorld, RigidBody,
};

#[test]
fn test_input_buffer_stores_correctly() {
    let mut buffer = InputBuffer::new();

    // Add several inputs
    buffer.add_input(1000, Vec3::new(1.0, 0.0, 0.0), false, 0.016);
    buffer.add_input(1016, Vec3::new(0.0, 0.0, 1.0), false, 0.016);
    buffer.add_input(1032, Vec3::new(-1.0, 0.0, 0.0), true, 0.016);

    assert_eq!(buffer.len(), 3);
    assert_eq!(buffer.current_sequence(), 3);

    // Verify retrieval
    let inputs = buffer.get_inputs_from(0);
    assert_eq!(inputs.len(), 3);
    assert_eq!(inputs[0].movement, Vec3::new(1.0, 0.0, 0.0));
    assert!(!inputs[0].jump);
    assert_eq!(inputs[2].movement, Vec3::new(-1.0, 0.0, 0.0));
    assert!(inputs[2].jump);
}

#[test]
fn test_input_buffer_circular_behavior() {
    let mut buffer = InputBuffer::new();

    // Fill beyond capacity
    for i in 0..150 {
        buffer.add_input(i as u64 * 16, Vec3::ZERO, false, 0.016);
    }

    // Should cap at max size (120)
    assert_eq!(buffer.len(), 120);

    // Oldest inputs should be removed (sequences 0-29 gone)
    let inputs = buffer.get_inputs_from(0);
    assert_eq!(inputs[0].sequence, 30); // First available is sequence 30
}

#[test]
fn test_input_buffer_remove_before() {
    let mut buffer = InputBuffer::new();

    for i in 0..20 {
        buffer.add_input(i * 16, Vec3::ZERO, false, 0.016);
    }

    // Remove inputs before sequence 10
    buffer.remove_before(10);
    assert_eq!(buffer.len(), 10);

    // Remaining should be sequences 10-19
    let inputs = buffer.get_inputs_from(0);
    assert_eq!(inputs[0].sequence, 10);
    assert_eq!(inputs[9].sequence, 19);
}

#[test]
fn test_input_buffer_get_from_filters_correctly() {
    let mut buffer = InputBuffer::new();

    for i in 0..10 {
        buffer.add_input(i * 16, Vec3::ZERO, false, 0.016);
    }

    // Get inputs from sequence 5
    let inputs = buffer.get_inputs_from(5);
    assert_eq!(inputs.len(), 5); // Sequences 5-9
    assert_eq!(inputs[0].sequence, 5);

    // Get inputs from sequence 100 (none exist)
    let inputs = buffer.get_inputs_from(100);
    assert_eq!(inputs.len(), 0);
}

#[test]
fn test_predicted_state_update_server_state() {
    let mut allocator = EntityAllocator::new();
    let entity = allocator.allocate();

    let mut state = PredictedState::new(entity, Vec3::ZERO, Quat::IDENTITY, Vec3::ZERO);

    state.update_server_state(
        10,
        Vec3::new(5.0, 1.0, 3.0),
        Quat::from_axis_angle(Vec3::Y, 1.57),
        Vec3::new(1.0, 0.0, 0.0),
    );

    assert_eq!(state.confirmed_sequence, 10);
    assert_eq!(state.server_position, Vec3::new(5.0, 1.0, 3.0));
    assert_eq!(state.server_velocity, Vec3::new(1.0, 0.0, 0.0));
}

#[test]
fn test_reconciliation_detects_mismatch() {
    let mut allocator = EntityAllocator::new();
    let entity = allocator.allocate();

    let mut state = PredictedState::new(entity, Vec3::ZERO, Quat::IDENTITY, Vec3::ZERO);

    // Simulate prediction drift
    state.predicted_position = Vec3::new(1.0, 0.0, 0.0);
    state.server_position = Vec3::ZERO;

    let error = state.calculate_error();
    assert!((error - 1.0).abs() < 0.01);
}

#[test]
fn test_reconciliation_triggers_correction() {
    let mut allocator = EntityAllocator::new();
    let entity = allocator.allocate();

    let mut state = PredictedState::new(entity, Vec3::ZERO, Quat::IDENTITY, Vec3::ZERO);

    // Small error - should not trigger immediate correction
    state.predicted_position = Vec3::new(0.5, 0.0, 0.0);
    assert!(!state.needs_immediate_correction());

    // Large error - should trigger immediate correction
    state.predicted_position = Vec3::new(10.0, 0.0, 0.0);
    assert!(state.needs_immediate_correction());
}

#[test]
fn test_prediction_system_lifecycle() {
    let mut allocator = EntityAllocator::new();
    let entity = allocator.allocate();

    let mut system = PredictionSystem::new();

    // Start prediction
    system.start_prediction(entity, 1, Vec3::ZERO, Quat::IDENTITY, Vec3::ZERO);
    assert!(system.predicted_state().is_some());
    assert_eq!(system.buffered_input_count(), 0);

    // Stop prediction
    system.stop_prediction();
    assert!(system.predicted_state().is_none());
}

#[test]
fn test_prediction_with_physics_integration() {
    let mut allocator = EntityAllocator::new();
    let entity = allocator.allocate();

    let config = PhysicsConfig::default();
    let mut physics = PhysicsWorld::new(config);

    // Create physics body
    let physics_id = 1;
    physics.add_rigidbody(
        physics_id,
        &RigidBody::dynamic(1.0),
        Vec3::new(0.0, 1.0, 0.0),
        Quat::IDENTITY,
    );
    physics.add_collider(physics_id, &Collider::capsule(0.5, 0.3));

    // Start prediction
    let mut system = PredictionSystem::new();
    system.start_prediction(entity, physics_id, Vec3::new(0.0, 1.0, 0.0), Quat::IDENTITY, Vec3::ZERO);

    // Add input and predict
    system.add_input_and_predict(
        1000,
        Vec3::new(1.0, 0.0, 0.0),
        false,
        0.016,
        &mut physics,
    );

    assert_eq!(system.buffered_input_count(), 1);
    assert_eq!(system.current_sequence(), 1);
}

#[test]
fn test_reconciliation_with_small_error() {
    let mut allocator = EntityAllocator::new();
    let entity = allocator.allocate();

    let config = PhysicsConfig::default();
    let mut physics = PhysicsWorld::new(config);

    let physics_id = 1;
    physics.add_rigidbody(physics_id, &RigidBody::dynamic(1.0), Vec3::ZERO, Quat::IDENTITY);
    physics.add_collider(physics_id, &Collider::sphere(0.5));

    let mut system = PredictionSystem::new();
    system.start_prediction(entity, physics_id, Vec3::ZERO, Quat::IDENTITY, Vec3::ZERO);

    // Add some inputs
    for i in 0..5 {
        system.add_input_and_predict(
            i * 16,
            Vec3::new(1.0, 0.0, 0.0),
            false,
            0.016,
            &mut physics,
        );
        physics.step(0.016);
    }

    // Server state slightly different (small error)
    system.reconcile(
        2,
        Vec3::new(0.1, 0.0, 0.0),
        Quat::IDENTITY,
        Vec3::ZERO,
        &mut physics,
    );

    // Should have reconciled
    assert!(system.predicted_state().is_some());
}

#[test]
fn test_reconciliation_with_large_error() {
    let mut allocator = EntityAllocator::new();
    let entity = allocator.allocate();

    let config = PhysicsConfig::default();
    let mut physics = PhysicsWorld::new(config);

    let physics_id = 1;
    physics.add_rigidbody(physics_id, &RigidBody::dynamic(1.0), Vec3::ZERO, Quat::IDENTITY);
    physics.add_collider(physics_id, &Collider::sphere(0.5));

    let mut system = PredictionSystem::new();
    system.start_prediction(entity, physics_id, Vec3::ZERO, Quat::IDENTITY, Vec3::ZERO);

    // Predicted state
    system.add_input_and_predict(1000, Vec3::X, false, 0.016, &mut physics);

    // Server sends very different position (large error - should teleport)
    system.reconcile(
        0,
        Vec3::new(10.0, 0.0, 0.0), // Far away
        Quat::IDENTITY,
        Vec3::ZERO,
        &mut physics,
    );

    // Should have been corrected
    if let Some(state) = system.predicted_state() {
        assert!((state.predicted_position - Vec3::new(10.0, 0.0, 0.0)).length() < 0.1);
    }
}

#[test]
fn test_input_replay_determinism() {
    let config = PhysicsConfig::default();
    let mut physics1 = PhysicsWorld::new(config.clone());
    let mut physics2 = PhysicsWorld::new(config);

    // Create identical physics setups
    let id = 1;
    for physics in [&mut physics1, &mut physics2] {
        physics.add_rigidbody(id, &RigidBody::dynamic(1.0), Vec3::ZERO, Quat::IDENTITY);
        physics.add_collider(id, &Collider::sphere(0.5));
    }

    // Apply same inputs to both
    let inputs = vec![
        PlayerInput::new(0, 0, Vec3::new(1.0, 0.0, 0.0), false, 0.016),
        PlayerInput::new(1, 16, Vec3::new(0.0, 0.0, 1.0), false, 0.016),
        PlayerInput::new(2, 32, Vec3::new(-1.0, 0.0, 0.0), true, 0.016),
    ];

    for input in &inputs {
        // Apply to physics1
        physics1.apply_force(id, input.movement * 50.0);
        physics1.step(input.delta_time);

        // Apply to physics2
        physics2.apply_force(id, input.movement * 50.0);
        physics2.step(input.delta_time);
    }

    // Positions should be identical (deterministic physics)
    let (pos1, _) = physics1.get_transform(id).unwrap();
    let (pos2, _) = physics2.get_transform(id).unwrap();

    assert!((pos1 - pos2).length() < 0.001, "Physics should be deterministic");
}

#[test]
fn test_error_smoothing() {
    let mut allocator = EntityAllocator::new();
    let entity = allocator.allocate();

    let mut world = World::new();
    world.register::<Transform>();

    // Add entity with transform
    world.add(entity, Transform::from_position(Vec3::ZERO));

    let mut system = PredictionSystem::new();
    system.start_prediction(entity, 1, Vec3::ZERO, Quat::IDENTITY, Vec3::ZERO);

    // Create prediction error
    if let Some(state) = system.predicted_state.as_mut() {
        state.predicted_position = Vec3::new(1.0, 0.0, 0.0);
        state.server_position = Vec3::ZERO;
    }

    // Apply smoothing multiple times
    for _ in 0..10 {
        system.apply_error_smoothing(&mut world, 0.016);
    }

    // Error should be reduced (smoothed toward server position)
    if let Some(state) = system.predicted_state() {
        let error = state.calculate_error();
        assert!(error < 1.0, "Error should be smoothed, got: {}", error);
    }
}

#[test]
fn test_high_latency_scenario() {
    let mut allocator = EntityAllocator::new();
    let entity = allocator.allocate();

    let config = PhysicsConfig::default();
    let mut physics = PhysicsWorld::new(config);

    let physics_id = 1;
    physics.add_rigidbody(physics_id, &RigidBody::dynamic(1.0), Vec3::ZERO, Quat::IDENTITY);
    physics.add_collider(physics_id, &Collider::sphere(0.5));

    let mut system = PredictionSystem::new();
    system.start_prediction(entity, physics_id, Vec3::ZERO, Quat::IDENTITY, Vec3::ZERO);

    // Simulate 100ms of latency (6 frames at 60 FPS)
    for i in 0..6 {
        system.add_input_and_predict(
            i * 16,
            Vec3::new(1.0, 0.0, 0.0),
            false,
            0.016,
            &mut physics,
        );
        physics.step(0.016);
    }

    // Server confirms state from 3 frames ago
    system.reconcile(3, Vec3::new(0.05, 0.0, 0.0), Quat::IDENTITY, Vec3::ZERO, &mut physics);

    // Should have buffered inputs and handled reconciliation
    assert!(system.buffered_input_count() > 0);
}

#[test]
fn test_prediction_sequence_monotonic() {
    let system = PredictionSystem::new();
    let seq1 = system.current_sequence();

    let mut system = system;
    let config = PhysicsConfig::default();
    let mut physics = PhysicsWorld::new(config);

    let mut allocator = EntityAllocator::new();
    let entity = allocator.allocate();

    physics.add_rigidbody(1, &RigidBody::dynamic(1.0), Vec3::ZERO, Quat::IDENTITY);
    physics.add_collider(1, &Collider::sphere(0.5));

    system.start_prediction(entity, 1, Vec3::ZERO, Quat::IDENTITY, Vec3::ZERO);

    // Add inputs
    for i in 0..10 {
        let seq_before = system.current_sequence();
        system.add_input_and_predict(i * 16, Vec3::X, false, 0.016, &mut physics);
        let seq_after = system.current_sequence();

        assert_eq!(seq_after, seq_before + 1, "Sequence should increment monotonically");
    }
}

#[test]
fn test_buffer_clears_on_stop_prediction() {
    let mut allocator = EntityAllocator::new();
    let entity = allocator.allocate();

    let mut system = PredictionSystem::new();
    system.start_prediction(entity, 1, Vec3::ZERO, Quat::IDENTITY, Vec3::ZERO);

    let config = PhysicsConfig::default();
    let mut physics = PhysicsWorld::new(config);
    physics.add_rigidbody(1, &RigidBody::dynamic(1.0), Vec3::ZERO, Quat::IDENTITY);
    physics.add_collider(1, &Collider::sphere(0.5));

    // Add some inputs
    for i in 0..5 {
        system.add_input_and_predict(i * 16, Vec3::X, false, 0.016, &mut physics);
    }

    assert_eq!(system.buffered_input_count(), 5);

    // Stop prediction
    system.stop_prediction();

    // Buffer should be cleared
    assert_eq!(system.buffered_input_count(), 0);
    assert_eq!(system.current_sequence(), 0);
}

#[test]
fn test_reconciliation_removes_old_inputs() {
    let mut allocator = EntityAllocator::new();
    let entity = allocator.allocate();

    let config = PhysicsConfig::default();
    let mut physics = PhysicsWorld::new(config);

    let physics_id = 1;
    physics.add_rigidbody(physics_id, &RigidBody::dynamic(1.0), Vec3::ZERO, Quat::IDENTITY);
    physics.add_collider(physics_id, &Collider::sphere(0.5));

    let mut system = PredictionSystem::new();
    system.start_prediction(entity, physics_id, Vec3::ZERO, Quat::IDENTITY, Vec3::ZERO);

    // Add 10 inputs
    for i in 0..10 {
        system.add_input_and_predict(i * 16, Vec3::X, false, 0.016, &mut physics);
        physics.step(0.016);
    }

    assert_eq!(system.buffered_input_count(), 10);

    // Server confirms sequence 7 (should remove 0-6)
    system.reconcile(7, Vec3::new(0.1, 0.0, 0.0), Quat::IDENTITY, Vec3::ZERO, &mut physics);

    // Only inputs 8, 9 should remain
    assert!(system.buffered_input_count() <= 3, "Old inputs should be removed");
}
