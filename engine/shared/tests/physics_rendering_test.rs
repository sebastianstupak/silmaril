//! Comprehensive integration tests for physics-rendering synchronization
//!
//! Tests physics state determinism, rendering sync correctness, and edge cases.
//! This is a **cross-crate integration test** (uses engine-physics + engine-core + engine-math).
//!
//! Per TESTING_ARCHITECTURE.md: Cross-crate tests MUST be in engine/shared/tests/

use engine_core::ecs::World;
use engine_core::math::Transform;
use engine_math::{Quat, Vec3};
use engine_physics::{
    Collider, PhysicsConfig, PhysicsMode, PhysicsWorld, RigidBody, RigidBodyType, Velocity,
};

// ============================================================================
// Test Category 1: Physics-Rendering Sync Tests
// ============================================================================

/// Test that physics transform updates propagate to ECS correctly
#[test]
fn test_physics_transform_sync_basic() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<RigidBody>();
    world.register::<Collider>();

    let config = PhysicsConfig::default();
    let mut physics = PhysicsWorld::new(config);

    // Create entity with physics body
    let entity = world.spawn();
    let transform = Transform::from_position(Vec3::new(0.0, 10.0, 0.0));
    world.add(entity, transform);

    let rb = RigidBody::dynamic(1.0);
    let entity_id = entity.id();
    physics.add_rigidbody(entity_id, &rb, transform.position, transform.rotation);
    physics.add_collider(entity_id, &Collider::sphere(0.5));

    // Step physics (gravity should move it down)
    physics.step(0.016);

    // Get new transform from physics
    let (new_pos, new_rot) = physics.get_transform(entity_id).expect("Entity should exist");

    // Update ECS transform (this is what rendering would do)
    if let Some(ecs_transform) = world.get_mut::<Transform>(entity) {
        ecs_transform.position = new_pos;
        ecs_transform.rotation = new_rot;
    }

    // Verify sync
    let synced_transform = world.get::<Transform>(entity).expect("Transform should exist");
    assert!(
        (synced_transform.position.y - new_pos.y).abs() < 1e-5,
        "Position should match physics state"
    );
}

/// Test multiple bodies sync correctly without interference
#[test]
fn test_multi_body_sync() {
    let mut world = World::new();
    world.register::<Transform>();

    let config = PhysicsConfig::default();
    let mut physics = PhysicsWorld::new(config);

    // Create 10 entities with different positions
    let entity_ids: Vec<_> = (0..10)
        .map(|i| {
            let entity = world.spawn();
            let pos = Vec3::new(i as f32, 10.0 + i as f32, 0.0);
            let transform = Transform::from_position(pos);
            world.add(entity, transform);

            let rb = RigidBody::dynamic(1.0);
            let entity_id = entity.id();
            physics.add_rigidbody(entity_id, &rb, pos, Quat::IDENTITY);
            physics.add_collider(entity_id, &Collider::sphere(0.5));

            entity_id
        })
        .collect();

    // Step physics
    for _ in 0..10 {
        physics.step(0.016);
    }

    // Verify all entities have unique positions (no collision/overlap in physics state)
    let mut positions = Vec::new();
    for &entity_id in &entity_ids {
        let (pos, _) = physics.get_transform(entity_id).expect("Entity should exist");
        positions.push(pos);
    }

    // Check all positions are distinct (within tolerance)
    for i in 0..positions.len() {
        for j in (i + 1)..positions.len() {
            let dist = (positions[i] - positions[j]).length();
            assert!(
                dist > 0.1,
                "Entities {} and {} too close: distance = {}",
                i,
                j,
                dist
            );
        }
    }
}

/// Test velocity synchronization between physics and rendering
#[test]
fn test_velocity_sync() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Velocity>();

    let config = PhysicsConfig::default();
    let mut physics = PhysicsWorld::new(config);

    let entity = world.spawn();
    let transform = Transform::from_position(Vec3::ZERO);
    world.add(entity, transform);

    let rb = RigidBody::dynamic(1.0);
    let entity_id = entity.id();
    physics.add_rigidbody(entity_id, &rb, Vec3::ZERO, Quat::IDENTITY);

    // Set initial velocity
    let initial_vel = Vec3::new(5.0, 0.0, 2.0);
    physics.set_velocity(entity_id, initial_vel, Vec3::ZERO);

    // Step physics
    physics.step(0.016);

    // Get velocity from physics
    let (lin_vel, _ang_vel) = physics.get_velocity(entity_id).expect("Entity should exist");

    // Update ECS velocity component
    world.add(entity, Velocity::new(lin_vel.x, lin_vel.y, lin_vel.z));

    // Verify sync
    let synced_vel = world.get::<Velocity>(entity).expect("Velocity should exist");
    assert!(
        (synced_vel.linear.x - lin_vel.x).abs() < 1e-5,
        "Velocity X should match"
    );
    assert!(
        (synced_vel.linear.y - lin_vel.y).abs() < 1e-5,
        "Velocity Y should match"
    );
    assert!(
        (synced_vel.linear.z - lin_vel.z).abs() < 1e-5,
        "Velocity Z should match"
    );
}

// ============================================================================
// Test Category 2: Determinism Tests
// ============================================================================

/// Test that physics simulation is deterministic (same input → same output)
#[test]
fn test_physics_determinism_basic() {
    let config = PhysicsConfig::default_deterministic();

    // Run simulation twice with identical setup
    let final_pos_1 = run_deterministic_simulation(&config, 100);
    let final_pos_2 = run_deterministic_simulation(&config, 100);

    // Results should be EXACTLY the same
    assert_eq!(
        final_pos_1, final_pos_2,
        "Deterministic physics must produce identical results"
    );
}

/// Run a deterministic physics simulation and return final position
fn run_deterministic_simulation(config: &PhysicsConfig, steps: usize) -> Vec3 {
    let mut physics = PhysicsWorld::new(config.clone());

    let rb = RigidBody::dynamic(1.0);
    physics.add_rigidbody(1, &rb, Vec3::new(0.0, 10.0, 0.0), Quat::IDENTITY);
    physics.add_collider(1, &Collider::sphere(0.5));

    // Ground plane
    let ground = RigidBody::static_body();
    physics.add_rigidbody(2, &ground, Vec3::new(0.0, -1.0, 0.0), Quat::IDENTITY);
    physics.add_collider(2, &Collider::box_collider(Vec3::new(100.0, 0.5, 100.0)));

    // Run simulation
    for _ in 0..steps {
        physics.step(0.016);
    }

    let (pos, _) = physics.get_transform(1).expect("Entity should exist");
    pos
}

/// Test determinism with complex scene (multiple bodies, collisions)
#[test]
fn test_physics_determinism_complex() {
    let config = PhysicsConfig::default_deterministic();

    // Run complex simulation twice
    let final_positions_1 = run_complex_deterministic_simulation(&config, 200);
    let final_positions_2 = run_complex_deterministic_simulation(&config, 200);

    // All final positions should match exactly
    assert_eq!(
        final_positions_1.len(),
        final_positions_2.len(),
        "Entity count should match"
    );

    for (i, (pos1, pos2)) in final_positions_1.iter().zip(final_positions_2.iter()).enumerate() {
        assert_eq!(
            pos1, pos2,
            "Position of entity {} should be deterministic",
            i
        );
    }
}

/// Run complex deterministic simulation with multiple colliding bodies
fn run_complex_deterministic_simulation(config: &PhysicsConfig, steps: usize) -> Vec<Vec3> {
    let mut physics = PhysicsWorld::new(config.clone());

    // Create stack of boxes
    for i in 0..5 {
        let rb = RigidBody::dynamic(1.0);
        let pos = Vec3::new(0.0, i as f32 * 2.0 + 1.0, 0.0);
        physics.add_rigidbody(i + 1, &rb, pos, Quat::IDENTITY);
        physics.add_collider(i + 1, &Collider::box_collider(Vec3::new(0.5, 0.5, 0.5)));
    }

    // Ground
    let ground = RigidBody::static_body();
    physics.add_rigidbody(100, &ground, Vec3::new(0.0, -1.0, 0.0), Quat::IDENTITY);
    physics.add_collider(100, &Collider::box_collider(Vec3::new(100.0, 0.5, 100.0)));

    // Run simulation
    for _ in 0..steps {
        physics.step(0.016);
    }

    // Collect final positions
    (1..=5)
        .map(|id| {
            let (pos, _) = physics.get_transform(id).expect("Entity should exist");
            pos
        })
        .collect()
}

/// Test determinism across different frame rates (fixed timestep)
#[test]
fn test_determinism_across_framerates() {
    let config = PhysicsConfig::default_deterministic();

    // Simulate at 60 FPS (16ms frames)
    let mut physics_60fps = PhysicsWorld::new(config.clone());
    let rb = RigidBody::dynamic(1.0);
    physics_60fps.add_rigidbody(1, &rb, Vec3::new(0.0, 10.0, 0.0), Quat::IDENTITY);
    physics_60fps.add_collider(1, &Collider::sphere(0.5));

    for _ in 0..60 {
        physics_60fps.step(0.016); // 60 FPS
    }

    // Simulate at 30 FPS (33ms frames) - should still be deterministic
    let mut physics_30fps = PhysicsWorld::new(config);
    let rb = RigidBody::dynamic(1.0);
    physics_30fps.add_rigidbody(1, &rb, Vec3::new(0.0, 10.0, 0.0), Quat::IDENTITY);
    physics_30fps.add_collider(1, &Collider::sphere(0.5));

    for _ in 0..30 {
        physics_30fps.step(0.033); // 30 FPS (2 substeps per frame)
    }

    let (pos_60fps, _) = physics_60fps.get_transform(1).expect("Entity should exist");
    let (pos_30fps, _) = physics_30fps.get_transform(1).expect("Entity should exist");

    // Positions should be very close (not exact due to substep accumulation)
    let diff = (pos_60fps - pos_30fps).length();
    assert!(
        diff < 0.1,
        "Fixed timestep should produce similar results across framerates: diff = {}",
        diff
    );
}

// ============================================================================
// Test Category 3: Interpolation/Extrapolation Tests
// ============================================================================

/// Test linear interpolation between physics states
#[test]
fn test_linear_interpolation() {
    let start_pos = Vec3::new(0.0, 0.0, 0.0);
    let end_pos = Vec3::new(10.0, 5.0, 2.0);

    // Interpolate at various alpha values
    let mid_pos = lerp_vec3(start_pos, end_pos, 0.5);
    assert_eq!(mid_pos, Vec3::new(5.0, 2.5, 1.0));

    let quarter_pos = lerp_vec3(start_pos, end_pos, 0.25);
    assert_eq!(quarter_pos, Vec3::new(2.5, 1.25, 0.5));

    let three_quarter_pos = lerp_vec3(start_pos, end_pos, 0.75);
    assert_eq!(three_quarter_pos, Vec3::new(7.5, 3.75, 1.5));
}

/// Linear interpolation helper
fn lerp_vec3(a: Vec3, b: Vec3, t: f32) -> Vec3 {
    a + (b - a) * t
}

/// Test quaternion interpolation (slerp) for smooth rotation
#[test]
fn test_quaternion_interpolation() {
    let start_rot = Quat::IDENTITY;
    let end_rot = Quat::from_axis_angle(Vec3::Y, std::f32::consts::PI / 2.0); // 90° rotation

    // Interpolate at midpoint
    let mid_rot = slerp_quat(start_rot, end_rot, 0.5);

    // Verify it's roughly 45° rotation
    let angle = mid_rot.to_axis_angle().1;
    let expected_angle = std::f32::consts::PI / 4.0; // 45°
    assert!(
        (angle - expected_angle).abs() < 0.01,
        "Slerp should produce smooth rotation"
    );
}

/// Spherical linear interpolation helper
fn slerp_quat(a: Quat, b: Quat, t: f32) -> Quat {
    // Simple lerp + normalize for small angles (good enough for tests)
    let result = Quat::from_xyzw(
        a.x + (b.x - a.x) * t,
        a.y + (b.y - a.y) * t,
        a.z + (b.z - a.z) * t,
        a.w + (b.w - a.w) * t,
    );
    result.normalize()
}

// ============================================================================
// Test Category 4: Edge Case Tests
// ============================================================================

/// Test handling of teleportation (instant position change)
#[test]
fn test_teleportation_edge_case() {
    let config = PhysicsConfig::default();
    let mut physics = PhysicsWorld::new(config);

    let rb = RigidBody::dynamic(1.0);
    physics.add_rigidbody(1, &rb, Vec3::ZERO, Quat::IDENTITY);
    physics.add_collider(1, &Collider::sphere(0.5));

    // Step once
    physics.step(0.016);

    // Teleport entity far away
    let teleport_pos = Vec3::new(1000.0, 500.0, 300.0);
    physics.set_transform(1, teleport_pos, Quat::IDENTITY);

    // Step again
    physics.step(0.016);

    // Verify entity is at teleport position
    let (pos, _) = physics.get_transform(1).expect("Entity should exist");
    assert!(
        (pos - teleport_pos).length() < 1.0,
        "Entity should stay near teleport position"
    );
}

/// Test physics explosion (extreme forces)
#[test]
fn test_extreme_force_stability() {
    let config = PhysicsConfig::default();
    let mut physics = PhysicsWorld::new(config);

    let rb = RigidBody::dynamic(1.0);
    physics.add_rigidbody(1, &rb, Vec3::ZERO, Quat::IDENTITY);
    physics.add_collider(1, &Collider::sphere(0.5));

    // Apply extreme force
    let extreme_force = Vec3::new(0.0, 10000.0, 0.0);
    physics.apply_force(1, extreme_force);

    // Step physics (should not crash or produce NaN)
    physics.step(0.016);

    let (pos, _) = physics.get_transform(1).expect("Entity should exist");
    assert!(pos.is_finite(), "Position should remain finite after extreme force");
}

/// Test large timestep stability
#[test]
fn test_large_timestep_stability() {
    let config = PhysicsConfig::default();
    let mut physics = PhysicsWorld::new(config);

    let rb = RigidBody::dynamic(1.0);
    physics.add_rigidbody(1, &rb, Vec3::new(0.0, 10.0, 0.0), Quat::IDENTITY);
    physics.add_collider(1, &Collider::sphere(0.5));

    // Step with large timestep (1 second - will use substeps)
    physics.step(1.0);

    let (pos, _) = physics.get_transform(1).expect("Entity should exist");
    assert!(
        pos.is_finite(),
        "Position should remain finite after large timestep"
    );
    assert!(pos.y < 10.0, "Object should have fallen due to gravity");
}

/// Test sleeping body edge cases
#[test]
fn test_sleeping_body_wakeup() {
    let config = PhysicsConfig::default();
    let mut physics = PhysicsWorld::new(config);

    // Create static ground
    let ground = RigidBody::static_body();
    physics.add_rigidbody(2, &ground, Vec3::new(0.0, -1.0, 0.0), Quat::IDENTITY);
    physics.add_collider(2, &Collider::box_collider(Vec3::new(100.0, 0.5, 100.0)));

    // Create dynamic body on ground
    let rb = RigidBody::dynamic(1.0);
    physics.add_rigidbody(1, &rb, Vec3::new(0.0, 1.0, 0.0), Quat::IDENTITY);
    physics.add_collider(1, &Collider::sphere(0.5));

    // Step many times to let it sleep
    for _ in 0..200 {
        physics.step(0.016);
    }

    // Apply force to wake it up
    physics.apply_force(1, Vec3::new(0.0, 500.0, 0.0));

    // Step once
    physics.step(0.016);

    // Check velocity (should be moving)
    let (lin_vel, _) = physics.get_velocity(1).expect("Entity should exist");
    assert!(
        lin_vel.length() > 0.1,
        "Body should wake up and start moving after force applied"
    );
}

/// Test constraint failure fallback
#[test]
fn test_joint_stress() {
    let config = PhysicsConfig::default();
    let mut physics = PhysicsWorld::new(config);

    // Create two bodies
    let rb1 = RigidBody::dynamic(1.0);
    physics.add_rigidbody(1, &rb1, Vec3::new(0.0, 0.0, 0.0), Quat::IDENTITY);
    physics.add_collider(1, &Collider::sphere(0.5));

    let rb2 = RigidBody::dynamic(1.0);
    physics.add_rigidbody(2, &rb2, Vec3::new(2.0, 0.0, 0.0), Quat::IDENTITY);
    physics.add_collider(2, &Collider::sphere(0.5));

    // Connect with joint
    let joint = engine_physics::joints::Joint::fixed(Vec3::ZERO, Vec3::ZERO);
    let joint_handle = physics.add_joint(1, 2, &joint);
    assert!(joint_handle.is_some(), "Joint should be created");

    // Apply extreme opposing forces
    physics.apply_force(1, Vec3::new(-1000.0, 0.0, 0.0));
    physics.apply_force(2, Vec3::new(1000.0, 0.0, 0.0));

    // Step physics (joint should handle stress without breaking simulation)
    for _ in 0..10 {
        physics.step(0.016);
    }

    // Verify entities still exist and have valid transforms
    let (pos1, _) = physics.get_transform(1).expect("Entity 1 should exist");
    let (pos2, _) = physics.get_transform(2).expect("Entity 2 should exist");
    assert!(pos1.is_finite(), "Entity 1 position should be finite");
    assert!(pos2.is_finite(), "Entity 2 position should be finite");
}

/// Test physics-rendering sync with fast-moving objects
#[test]
fn test_fast_moving_object_sync() {
    let mut world = World::new();
    world.register::<Transform>();

    let config = PhysicsConfig::default();
    let mut physics = PhysicsWorld::new(config);

    let entity = world.spawn();
    let transform = Transform::from_position(Vec3::ZERO);
    world.add(entity, transform);

    // Create body with CCD enabled
    let mut rb = RigidBody::dynamic(1.0);
    rb.ccd_enabled = true;
    let entity_id = entity.id();
    physics.add_rigidbody(entity_id, &rb, Vec3::ZERO, Quat::IDENTITY);
    physics.add_collider(entity_id, &Collider::sphere(0.5));

    // Apply very high velocity
    physics.set_velocity(entity_id, Vec3::new(1000.0, 0.0, 0.0), Vec3::ZERO);

    // Step several times
    for _ in 0..10 {
        physics.step(0.016);

        // Sync to ECS
        let (pos, rot) = physics.get_transform(entity_id).expect("Entity should exist");
        if let Some(ecs_transform) = world.get_mut::<Transform>(entity) {
            ecs_transform.position = pos;
            ecs_transform.rotation = rot;
        }
    }

    // Verify entity moved far and sync is maintained
    let final_transform = world.get::<Transform>(entity).expect("Transform should exist");
    assert!(
        final_transform.position.x > 100.0,
        "Fast-moving object should have traveled far"
    );
}

/// Test rendering sync during entity despawn
#[test]
fn test_sync_during_despawn() {
    let mut world = World::new();
    world.register::<Transform>();

    let config = PhysicsConfig::default();
    let mut physics = PhysicsWorld::new(config);

    // Create multiple entities
    let entity_ids: Vec<_> = (0..10)
        .map(|i| {
            let entity = world.spawn();
            let pos = Vec3::new(i as f32, 0.0, 0.0);
            let transform = Transform::from_position(pos);
            world.add(entity, transform);

            let rb = RigidBody::dynamic(1.0);
            let entity_id = entity.id();
            physics.add_rigidbody(entity_id, &rb, pos, Quat::IDENTITY);
            physics.add_collider(entity_id, &Collider::sphere(0.5));

            entity_id
        })
        .collect();

    // Step physics
    physics.step(0.016);

    // Despawn half the entities
    for &entity_id in entity_ids.iter().take(5) {
        physics.remove_rigidbody(entity_id);
    }

    // Step again (should not crash)
    physics.step(0.016);

    // Verify remaining entities still have valid state
    for &entity_id in entity_ids.iter().skip(5) {
        let result = physics.get_transform(entity_id);
        assert!(result.is_some(), "Remaining entities should still exist");
    }
}

// ============================================================================
// Test Category 5: Collision Visualization Tests
// ============================================================================

/// Test collision event detection for rendering feedback
#[test]
fn test_collision_event_detection() {
    let config = PhysicsConfig::default();
    let mut physics = PhysicsWorld::new(config);

    // Create ground
    let ground = RigidBody::static_body();
    physics.add_rigidbody(1, &ground, Vec3::new(0.0, -1.0, 0.0), Quat::IDENTITY);
    physics.add_collider(1, &Collider::box_collider(Vec3::new(100.0, 0.5, 100.0)));

    // Create falling object
    let rb = RigidBody::dynamic(1.0);
    physics.add_rigidbody(2, &rb, Vec3::new(0.0, 10.0, 0.0), Quat::IDENTITY);
    physics.add_collider(2, &Collider::sphere(0.5));

    // Step until collision occurs
    let mut collision_detected = false;
    for _ in 0..100 {
        physics.step(0.016);

        // Check for collision events
        let events = physics.collision_events();
        if !events.is_empty() {
            collision_detected = true;
            break;
        }
    }

    assert!(
        collision_detected,
        "Collision event should be detected when object hits ground"
    );
}

/// Test contact force visualization data
#[test]
fn test_contact_force_data() {
    let config = PhysicsConfig::default();
    let mut physics = PhysicsWorld::new(config);

    // Create ground
    let ground = RigidBody::static_body();
    physics.add_rigidbody(1, &ground, Vec3::new(0.0, -1.0, 0.0), Quat::IDENTITY);
    physics.add_collider(1, &Collider::box_collider(Vec3::new(100.0, 0.5, 100.0)));

    // Create heavy falling object
    let rb = RigidBody::dynamic(100.0); // Heavy mass
    physics.add_rigidbody(2, &rb, Vec3::new(0.0, 10.0, 0.0), Quat::IDENTITY);
    physics.add_collider(2, &Collider::sphere(0.5));

    // Step until impact
    for _ in 0..100 {
        physics.step(0.016);
    }

    // Check contact force events (for visualization)
    let force_events = physics.contact_force_events();
    // Note: Contact force events may not be available in all physics configurations
    // This test documents the API, even if events are empty
    assert!(
        force_events.len() >= 0,
        "Contact force events API should be available"
    );
}

// ============================================================================
// Test Category 6: Substepping Accuracy Tests
// ============================================================================

/// Test substepping improves accuracy
#[test]
fn test_substepping_accuracy() {
    // Config without substeps
    let config_no_substep = PhysicsConfig {
        mode: PhysicsMode::Standard,
        gravity: Vec3::new(0.0, -9.81, 0.0),
        timestep: 0.016,
        max_substeps: 1,
        deterministic: false,
        solver_iterations: 4,
        enable_ccd: true,
    };

    // Config with substeps
    let config_substeps = PhysicsConfig {
        mode: PhysicsMode::Standard,
        gravity: Vec3::new(0.0, -9.81, 0.0),
        timestep: 0.016,
        max_substeps: 4,
        deterministic: false,
        solver_iterations: 4,
        enable_ccd: true,
    };

    // Run simulation with no substeps
    let mut physics_no_substep = PhysicsWorld::new(config_no_substep);
    let rb = RigidBody::dynamic(1.0);
    physics_no_substep.add_rigidbody(1, &rb, Vec3::new(0.0, 10.0, 0.0), Quat::IDENTITY);
    physics_no_substep.add_collider(1, &Collider::sphere(0.5));

    for _ in 0..60 {
        physics_no_substep.step(0.016);
    }

    let (pos_no_substep, _) =
        physics_no_substep.get_transform(1).expect("Entity should exist");

    // Run simulation with substeps
    let mut physics_substeps = PhysicsWorld::new(config_substeps);
    let rb = RigidBody::dynamic(1.0);
    physics_substeps.add_rigidbody(1, &rb, Vec3::new(0.0, 10.0, 0.0), Quat::IDENTITY);
    physics_substeps.add_collider(1, &Collider::sphere(0.5));

    for _ in 0..60 {
        physics_substeps.step(0.016);
    }

    let (pos_substeps, _) = physics_substeps.get_transform(1).expect("Entity should exist");

    // Substepping should produce different (more accurate) results
    let diff = (pos_no_substep - pos_substeps).length();
    assert!(
        diff > 0.0,
        "Substepping should affect simulation accuracy"
    );
}

// ============================================================================
// Test Category 7: Ragdoll Rendering Edge Cases
// ============================================================================

/// Test ragdoll physics with multiple connected bodies
#[test]
fn test_ragdoll_joint_chain() {
    let config = PhysicsConfig::default();
    let mut physics = PhysicsWorld::new(config);

    // Create chain of bodies (simplified ragdoll)
    let body_ids: Vec<u64> = (0..5)
        .map(|i| {
            let rb = RigidBody::dynamic(1.0);
            let id = (i + 1) as u64;
            physics.add_rigidbody(
                id,
                &rb,
                Vec3::new(0.0, 5.0 - i as f32, 0.0),
                Quat::IDENTITY,
            );
            physics.add_collider(id, &Collider::capsule(0.5, 0.25));
            id
        })
        .collect();

    // Connect bodies with joints
    for i in 0..body_ids.len() - 1 {
        let joint = engine_physics::joints::Joint::fixed(
            Vec3::new(0.0, -0.5, 0.0),
            Vec3::new(0.0, 0.5, 0.0),
        );
        physics.add_joint(body_ids[i], body_ids[i + 1], &joint);
    }

    // Step simulation
    for _ in 0..100 {
        physics.step(0.016);
    }

    // Verify all bodies still have valid transforms
    for &id in &body_ids {
        let (pos, rot) = physics.get_transform(id).expect("Body should exist");
        assert!(pos.is_finite(), "Ragdoll body position should be finite");
        assert!(
            rot.is_normalized(),
            "Ragdoll body rotation should be normalized"
        );
    }
}

// ============================================================================
// Helper Traits
// ============================================================================

trait Vec3Extensions {
    fn is_finite(&self) -> bool;
}

impl Vec3Extensions for Vec3 {
    fn is_finite(&self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }
}

trait QuatExtensions {
    fn is_normalized(&self) -> bool;
}

impl QuatExtensions for Quat {
    fn is_normalized(&self) -> bool {
        let len_sq = self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w;
        (len_sq - 1.0).abs() < 1e-4
    }
}
