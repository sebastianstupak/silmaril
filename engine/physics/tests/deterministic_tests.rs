//! Deterministic physics tests
//!
//! Verifies that deterministic mode produces identical results for identical inputs.

use engine_math::{Quat, Vec3};
use engine_physics::{
    create_snapshot, hash_physics_state, restore_snapshot, Collider, PhysicsConfig, PhysicsInput,
    PhysicsWorld, ReplayPlayer, ReplayRecorder, RigidBody,
};

/// Test that same inputs produce identical results
#[test]
fn test_determinism_same_inputs_identical_results() {
    // Create two identical worlds in deterministic mode
    let config = PhysicsConfig::default().with_deterministic(true);
    let mut world1 = PhysicsWorld::new(config.clone());
    let mut world2 = PhysicsWorld::new(config);

    // Add identical entities
    let rb = RigidBody::dynamic(1.0);
    world1.add_rigidbody(1, &rb, Vec3::new(0.0, 10.0, 0.0), Quat::IDENTITY);
    world1.add_collider(1, &Collider::sphere(0.5));

    world2.add_rigidbody(1, &rb, Vec3::new(0.0, 10.0, 0.0), Quat::IDENTITY);
    world2.add_collider(1, &Collider::sphere(0.5));

    // Run simulation for 100 frames
    let dt = 1.0 / 60.0;
    for _ in 0..100 {
        world1.step(dt);
        world2.step(dt);
    }

    // Get final states
    let (pos1, rot1) = world1.get_transform(1).unwrap();
    let (pos2, rot2) = world2.get_transform(1).unwrap();

    // Positions should be identical (bit-for-bit)
    assert_eq!(pos1.x, pos2.x, "X positions differ");
    assert_eq!(pos1.y, pos2.y, "Y positions differ");
    assert_eq!(pos1.z, pos2.z, "Z positions differ");

    assert_eq!(rot1.x, rot2.x, "Rotation X differs");
    assert_eq!(rot1.y, rot2.y, "Rotation Y differs");
    assert_eq!(rot1.z, rot2.z, "Rotation Z differs");
    assert_eq!(rot1.w, rot2.w, "Rotation W differs");

    // State hashes should match
    let hash1 = hash_physics_state(&world1);
    let hash2 = hash_physics_state(&world2);
    assert_eq!(hash1, hash2, "State hashes differ");
}

/// Test that state hash detects differences
#[test]
fn test_state_hash_detects_differences() {
    let config = PhysicsConfig::default().with_deterministic(true);
    let mut world = PhysicsWorld::new(config);

    let rb = RigidBody::dynamic(1.0);
    world.add_rigidbody(1, &rb, Vec3::new(0.0, 10.0, 0.0), Quat::IDENTITY);

    let hash1 = hash_physics_state(&world);

    // Make tiny change
    world.set_transform(1, Vec3::new(0.0, 10.0001, 0.0), Quat::IDENTITY);

    let hash2 = hash_physics_state(&world);

    assert_ne!(hash1, hash2, "State hash should detect tiny differences");
}

/// Test replay from snapshot matches original
#[test]
fn test_replay_from_snapshot_matches_original() {
    let config = PhysicsConfig::default().with_deterministic(true);
    let mut world = PhysicsWorld::new(config);

    let rb = RigidBody::dynamic(1.0);
    world.add_rigidbody(1, &rb, Vec3::new(0.0, 10.0, 0.0), Quat::IDENTITY);
    world.add_collider(1, &Collider::sphere(0.5));

    // Record initial state
    let mut recorder = ReplayRecorder::new();
    recorder.record_initial_snapshot(&world);

    // Run simulation and record inputs
    let dt = 1.0 / 60.0;
    for i in 0..60 {
        // Apply some forces
        if i % 10 == 0 {
            recorder.record_input(PhysicsInput::ApplyForce {
                entity_id: 1,
                force: Vec3::new(0.0, 50.0, 0.0),
            });
            world.apply_force(1, Vec3::new(0.0, 50.0, 0.0));
        }

        world.step(dt);
        recorder.commit_frame(&world);
    }

    // Get final state from original run
    let (final_pos, final_rot) = world.get_transform(1).unwrap();
    let final_hash = hash_physics_state(&world);

    // Create new world and replay
    let config2 = PhysicsConfig::default().with_deterministic(true);
    let mut replay_world = PhysicsWorld::new(config2);

    // Restore initial snapshot
    restore_snapshot(&mut replay_world, recorder.initial_snapshot().unwrap()).unwrap();

    // Replay
    let frames = recorder.frames().to_vec();
    let mut player = ReplayPlayer::new(recorder.initial_snapshot().unwrap().clone(), frames, true);

    while let Some(inputs) = player.next_frame() {
        // Apply inputs
        for input in inputs {
            match input {
                PhysicsInput::ApplyForce { entity_id, force } => {
                    replay_world.apply_force(*entity_id, *force);
                }
                PhysicsInput::ApplyImpulse { entity_id, impulse } => {
                    replay_world.apply_impulse(*entity_id, *impulse);
                }
                PhysicsInput::SetVelocity { entity_id, linear, angular } => {
                    replay_world.set_velocity(*entity_id, *linear, *angular);
                }
                PhysicsInput::SetTransform { entity_id, position, rotation } => {
                    replay_world.set_transform(*entity_id, *position, *rotation);
                }
            }
        }

        replay_world.step(dt);
        player.verify_hash(&replay_world).unwrap();
    }

    // Verify final state matches
    let (replay_pos, replay_rot) = replay_world.get_transform(1).unwrap();
    let replay_hash = hash_physics_state(&replay_world);

    assert_eq!(final_pos.x, replay_pos.x, "Final X position differs");
    assert_eq!(final_pos.y, replay_pos.y, "Final Y position differs");
    assert_eq!(final_pos.z, replay_pos.z, "Final Z position differs");

    assert_eq!(final_rot.x, replay_rot.x, "Final rotation X differs");
    assert_eq!(final_rot.y, replay_rot.y, "Final rotation Y differs");
    assert_eq!(final_rot.z, replay_rot.z, "Final rotation Z differs");
    assert_eq!(final_rot.w, replay_rot.w, "Final rotation W differs");

    assert_eq!(final_hash, replay_hash, "Final state hash differs");
}

/// Test deterministic mode works with collisions
#[test]
fn test_determinism_with_collisions() {
    let config = PhysicsConfig::default().with_deterministic(true);
    let mut world1 = PhysicsWorld::new(config.clone());
    let mut world2 = PhysicsWorld::new(config);

    // Add ground
    let ground = RigidBody::static_body();
    world1.add_rigidbody(1, &ground, Vec3::new(0.0, -1.0, 0.0), Quat::IDENTITY);
    world1.add_collider(1, &Collider::box_collider(Vec3::new(10.0, 0.5, 10.0)));

    world2.add_rigidbody(1, &ground, Vec3::new(0.0, -1.0, 0.0), Quat::IDENTITY);
    world2.add_collider(1, &Collider::box_collider(Vec3::new(10.0, 0.5, 10.0)));

    // Add falling ball
    let ball = RigidBody::dynamic(1.0);
    world1.add_rigidbody(2, &ball, Vec3::new(0.0, 10.0, 0.0), Quat::IDENTITY);
    world1.add_collider(2, &Collider::sphere(0.5));

    world2.add_rigidbody(2, &ball, Vec3::new(0.0, 10.0, 0.0), Quat::IDENTITY);
    world2.add_collider(2, &Collider::sphere(0.5));

    // Simulate collision
    let dt = 1.0 / 60.0;
    for _ in 0..120 {
        world1.step(dt);
        world2.step(dt);
    }

    // Verify identical results after collision
    let (pos1, _) = world1.get_transform(2).unwrap();
    let (pos2, _) = world2.get_transform(2).unwrap();

    assert_eq!(pos1.x, pos2.x, "Post-collision X differs");
    assert_eq!(pos1.y, pos2.y, "Post-collision Y differs");
    assert_eq!(pos1.z, pos2.z, "Post-collision Z differs");

    let hash1 = hash_physics_state(&world1);
    let hash2 = hash_physics_state(&world2);
    assert_eq!(hash1, hash2, "Post-collision hash differs");
}

/// Test deterministic mode with multiple objects
#[test]
fn test_determinism_multiple_objects() {
    let config = PhysicsConfig::default().with_deterministic(true);
    let mut world1 = PhysicsWorld::new(config.clone());
    let mut world2 = PhysicsWorld::new(config);

    // Add multiple objects
    for i in 0..10 {
        let rb = RigidBody::dynamic(1.0);
        let pos = Vec3::new(i as f32, 10.0 + i as f32, 0.0);

        world1.add_rigidbody(i, &rb, pos, Quat::IDENTITY);
        world1.add_collider(i, &Collider::sphere(0.5));

        world2.add_rigidbody(i, &rb, pos, Quat::IDENTITY);
        world2.add_collider(i, &Collider::sphere(0.5));
    }

    // Simulate
    let dt = 1.0 / 60.0;
    for _ in 0..100 {
        world1.step(dt);
        world2.step(dt);
    }

    // Verify all objects match
    for i in 0..10 {
        let (pos1, _) = world1.get_transform(i).unwrap();
        let (pos2, _) = world2.get_transform(i).unwrap();

        assert_eq!(pos1.x, pos2.x, "Object {} X differs", i);
        assert_eq!(pos1.y, pos2.y, "Object {} Y differs", i);
        assert_eq!(pos1.z, pos2.z, "Object {} Z differs", i);
    }

    let hash1 = hash_physics_state(&world1);
    let hash2 = hash_physics_state(&world2);
    assert_eq!(hash1, hash2, "Multi-object hash differs");
}

/// Test snapshot and restore preserves state
#[test]
fn test_snapshot_restore_preserves_state() {
    let config = PhysicsConfig::default().with_deterministic(true);
    let mut world = PhysicsWorld::new(config);

    // Add objects
    let rb = RigidBody::dynamic(1.0);
    world.add_rigidbody(1, &rb, Vec3::new(1.0, 2.0, 3.0), Quat::IDENTITY);
    world.add_collider(1, &Collider::sphere(0.5));
    world.set_velocity(1, Vec3::new(1.0, 2.0, 3.0), Vec3::new(0.1, 0.2, 0.3));

    // Create snapshot
    let snapshot = create_snapshot(&world, 0);
    let original_hash = snapshot.state_hash;

    // Modify state
    world.step(1.0 / 60.0);
    let modified_hash = hash_physics_state(&world);
    assert_ne!(original_hash, modified_hash, "State should change after step");

    // Restore snapshot
    restore_snapshot(&mut world, &snapshot).unwrap();
    let restored_hash = hash_physics_state(&world);

    assert_eq!(original_hash, restored_hash, "Restore should match original hash");
}

/// Test replay recorder memory usage
#[test]
fn test_replay_recorder_memory_usage() {
    let config = PhysicsConfig::default().with_deterministic(true);
    let mut world = PhysicsWorld::new(config);

    let rb = RigidBody::dynamic(1.0);
    world.add_rigidbody(1, &rb, Vec3::ZERO, Quat::IDENTITY);

    let mut recorder = ReplayRecorder::new();
    recorder.record_initial_snapshot(&world);

    let initial_usage = recorder.memory_usage();

    // Record 1000 frames
    let dt = 1.0 / 60.0;
    for _ in 0..1000 {
        recorder.record_input(PhysicsInput::ApplyForce {
            entity_id: 1,
            force: Vec3::new(0.0, 10.0, 0.0),
        });
        world.step(dt);
        recorder.commit_frame(&world);
    }

    let final_usage = recorder.memory_usage();

    // Should be less than 1MB for 1000 frames (performance target)
    assert!(final_usage < 1024 * 1024, "Memory usage {} exceeds 1MB target", final_usage);
    assert!(final_usage > initial_usage, "Memory should increase with recorded frames");
}

/// Test deterministic config disables parallelism and SIMD
#[test]
fn test_deterministic_config_settings() {
    let config = PhysicsConfig::default().with_deterministic(true);

    assert!(config.deterministic, "Deterministic flag should be set");
    assert!(!config.enable_parallel, "Parallelism should be disabled");
    assert!(!config.enable_simd, "SIMD should be disabled");
}

/// Test hash consistency across multiple calls
#[test]
fn test_hash_consistency() {
    let config = PhysicsConfig::default().with_deterministic(true);
    let mut world = PhysicsWorld::new(config);

    let rb = RigidBody::dynamic(1.0);
    world.add_rigidbody(1, &rb, Vec3::new(1.0, 2.0, 3.0), Quat::IDENTITY);

    // Hash multiple times without changing state
    let hashes: Vec<_> = (0..10).map(|_| hash_physics_state(&world)).collect();

    // All hashes should be identical
    for i in 1..hashes.len() {
        assert_eq!(hashes[0], hashes[i], "Hash {} differs from first hash", i);
    }
}
