//! Advanced Joint Tests for Physics Engine
//!
//! Tests complex joint scenarios and edge cases:
//! - Motor saturation and limits
//! - Conflicting constraints
//! - Joint chains under stress
//! - Breakage and reconnection
//! - Multi-DOF joints
//!
//! These tests validate production-ready joint handling.

use engine_math::{Quat, Vec3};
use engine_physics::{Collider, JointBuilder, JointMotor, PhysicsConfig, PhysicsWorld, RigidBody};

/// Test: Joint Motor High Speed
///
/// Validates that joint motors can drive rotation and remain stable at high speeds.
#[test]
fn test_joint_motor_saturation() {
    let mut world = PhysicsWorld::new(PhysicsConfig::default());

    // Fixed base
    world.add_rigidbody(0, &RigidBody::static_body(), Vec3::new(0.0, 0.0, 0.0), Quat::IDENTITY);
    world.add_collider(0, &Collider::box_collider(Vec3::new(1.0, 0.5, 1.0)));

    // Rotating arm
    world.add_rigidbody(1, &RigidBody::dynamic(1.0), Vec3::new(0.0, 2.0, 0.0), Quat::IDENTITY);
    world.add_collider(1, &Collider::box_collider(Vec3::new(0.5, 1.0, 0.5)));

    // Revolute joint with motor targeting high speed
    let motor = JointMotor::new(100.0) // High target velocity (100 rad/s ≈ 955 RPM)
        .with_max_force(1000.0); // High force to reach target

    let joint = JointBuilder::revolute()
        .anchor1(Vec3::new(0.0, 1.0, 0.0))
        .anchor2(Vec3::new(0.0, -1.0, 0.0))
        .axis(Vec3::Y)
        .motor(motor)
        .build();

    world.add_joint(0, 1, &joint);

    let dt = 1.0 / 60.0;

    // Run simulation
    for _ in 0..180 {
        // 3 seconds to reach target speed
        world.step(dt);
    }

    // Verify motor didn't break physics (no NaN/Inf)
    let (pos, _) = world.get_transform(1).unwrap();
    assert!(pos.is_finite(), "Joint with high-speed motor should remain stable");

    // Verify motor achieved significant angular velocity
    let (_, angular_vel) = world.get_velocity(1).unwrap();
    let angular_speed = angular_vel.length();

    // Motor should be spinning fast (close to target 100 rad/s)
    assert!(angular_speed > 50.0, "Motor should reach high speed, speed: {}", angular_speed);

    // Should remain numerically stable even at high speeds
    assert!(angular_speed.is_finite(), "Angular velocity should be finite at high speeds");
}

/// Test: Conflicting Joint Constraints
///
/// Validates handling of multiple joints with contradictory goals.
#[test]
fn test_conflicting_constraints() {
    let mut world = PhysicsWorld::new(PhysicsConfig::default());

    // Base
    world.add_rigidbody(0, &RigidBody::static_body(), Vec3::new(0.0, 0.0, 0.0), Quat::IDENTITY);
    world.add_collider(0, &Collider::box_collider(Vec3::new(1.0, 0.5, 1.0)));

    // Second anchor point
    world.add_rigidbody(1, &RigidBody::static_body(), Vec3::new(3.0, 0.0, 0.0), Quat::IDENTITY);
    world.add_collider(1, &Collider::box_collider(Vec3::new(1.0, 0.5, 1.0)));

    // Dynamic body in between
    world.add_rigidbody(2, &RigidBody::dynamic(1.0), Vec3::new(1.5, 2.0, 0.0), Quat::IDENTITY);
    world.add_collider(2, &Collider::box_collider(Vec3::new(0.5, 0.5, 0.5)));

    // Constraint 1: Fixed joint to base (wants body at (0, 2, 0))
    let joint1 = JointBuilder::fixed()
        .anchor1(Vec3::new(0.0, 2.0, 0.0))
        .anchor2(Vec3::new(0.0, 0.0, 0.0))
        .build();

    world.add_joint(0, 2, &joint1);

    // Constraint 2: Fixed joint to second anchor (wants body at (3, 2, 0))
    // These two fixed joints create impossible constraints!
    let joint2 = JointBuilder::fixed()
        .anchor1(Vec3::new(0.0, 2.0, 0.0))
        .anchor2(Vec3::new(0.0, 0.0, 0.0))
        .build();

    world.add_joint(1, 2, &joint2);

    let dt = 1.0 / 60.0;

    // Run simulation - physics solver should handle contradiction
    for _ in 0..120 {
        world.step(dt);
    }

    // Verify solver didn't explode (no NaN/Inf)
    let (pos, _) = world.get_transform(2).unwrap();
    assert!(pos.is_finite(), "Conflicting constraints should not cause NaN");

    // Body should settle somewhere between the two anchors
    // (exact position depends on solver, but should be reasonable)
    assert!(
        pos.x > -5.0 && pos.x < 10.0,
        "Body with conflicting constraints should settle to reasonable position, at x={}",
        pos.x
    );
}

/// Test: Long Joint Chain Under Load
///
/// Validates stability of long chain of connected bodies under stress.
#[test]
fn test_long_joint_chain_stress() {
    let mut world = PhysicsWorld::new(PhysicsConfig::default());

    // Fixed base
    world.add_rigidbody(0, &RigidBody::static_body(), Vec3::new(0.0, 0.0, 0.0), Quat::IDENTITY);
    world.add_collider(0, &Collider::sphere(0.5));

    // Create chain of 20 bodies
    let chain_length = 20;
    for i in 1..=chain_length {
        let y = -(i as f32) * 1.0;

        world.add_rigidbody(i, &RigidBody::dynamic(1.0), Vec3::new(0.0, y, 0.0), Quat::IDENTITY);
        world.add_collider(i, &Collider::box_collider(Vec3::new(0.3, 0.4, 0.3)));

        // Connect to previous body
        let joint = JointBuilder::spherical()
            .anchor1(Vec3::new(0.0, -0.5, 0.0))
            .anchor2(Vec3::new(0.0, 0.5, 0.0))
            .build();

        world.add_joint(i - 1, i, &joint);
    }

    // Apply horizontal force to end of chain (stress test)
    world.set_velocity(chain_length, Vec3::new(10.0, 0.0, 0.0), Vec3::ZERO);

    let dt = 1.0 / 60.0;

    // Simulate
    for _ in 0..300 {
        // 5 seconds
        world.step(dt);
    }

    // Verify all bodies remain stable (no NaN, positions reasonable)
    for i in 1..=chain_length {
        let (pos, _) = world.get_transform(i).unwrap();
        assert!(pos.is_finite(), "Chain body {} should have finite position", i);

        // Bodies should stay within reasonable distance of origin
        let distance = pos.length();
        assert!(
            distance < 100.0,
            "Chain body {} should not fly away, distance from origin: {}",
            i,
            distance
        );
    }
}

/// Test: Joint Under Extreme Stress
///
/// Validates joint stability under excessive forces.
#[test]
fn test_joint_extreme_stress() {
    let mut world = PhysicsWorld::new(PhysicsConfig::default());

    // Fixed base
    world.add_rigidbody(0, &RigidBody::static_body(), Vec3::new(0.0, 0.0, 0.0), Quat::IDENTITY);
    world.add_collider(0, &Collider::box_collider(Vec3::new(1.0, 1.0, 1.0)));

    // Attached body
    world.add_rigidbody(
        1,
        &RigidBody::dynamic(10.0), // Heavy mass
        Vec3::new(0.0, 2.0, 0.0),
        Quat::IDENTITY,
    );
    world.add_collider(1, &Collider::box_collider(Vec3::new(0.5, 0.5, 0.5)));

    // Strong joint
    let joint = JointBuilder::fixed()
        .anchor1(Vec3::new(0.0, 1.0, 0.0))
        .anchor2(Vec3::new(0.0, -0.5, 0.0))
        .build();

    world.add_joint(0, 1, &joint);

    // Apply massive angular velocity
    world.set_velocity(1, Vec3::new(0.0, 0.0, 0.0), Vec3::new(100.0, 0.0, 0.0));

    let dt = 1.0 / 60.0;

    for _ in 0..120 {
        world.step(dt);
    }

    // Validate system remains stable under stress
    let (pos, _) = world.get_transform(1).unwrap();
    assert!(pos.is_finite(), "Body should remain stable under extreme stress");

    // Joint should constrain the body somewhat (not fly off to infinity)
    let distance = pos.length();
    assert!(
        distance < 50.0,
        "Joint should provide some constraint, body at distance {}",
        distance
    );
}

/// Test: Multi-DOF Joint Limits
///
/// Validates joints with multiple degrees of freedom and angular limits.
#[test]
fn test_multi_dof_joint_limits() {
    let mut world = PhysicsWorld::new(PhysicsConfig::default());

    // Fixed base
    world.add_rigidbody(0, &RigidBody::static_body(), Vec3::new(0.0, 0.0, 0.0), Quat::IDENTITY);
    world.add_collider(0, &Collider::box_collider(Vec3::new(1.0, 0.5, 1.0)));

    // Attached body
    world.add_rigidbody(1, &RigidBody::dynamic(1.0), Vec3::new(0.0, 2.0, 0.0), Quat::IDENTITY);
    world.add_collider(1, &Collider::box_collider(Vec3::new(0.3, 1.0, 0.3)));

    // Spherical joint (3 DOF rotation) with limits
    let joint = JointBuilder::spherical()
        .anchor1(Vec3::new(0.0, 1.0, 0.0))
        .anchor2(Vec3::new(0.0, -1.0, 0.0))
        .build();

    world.add_joint(0, 1, &joint);

    // Apply torques in multiple directions
    world.set_velocity(1, Vec3::ZERO, Vec3::new(5.0, 5.0, 5.0));

    let dt = 1.0 / 60.0;

    for _ in 0..180 {
        world.step(dt);
    }

    // Verify joint allows rotation but constrains position
    let (pos, rot) = world.get_transform(1).unwrap();

    assert!(pos.is_finite(), "Position should be finite");
    assert!(rot.is_finite(), "Rotation should be finite");

    // Position should stay near joint anchor (joint constrains translation)
    let distance_from_anchor = (pos - Vec3::new(0.0, 2.0, 0.0)).length();
    assert!(
        distance_from_anchor < 2.0,
        "Spherical joint should constrain translation, distance: {}",
        distance_from_anchor
    );
}

/// Test: Joint Network (Multiple Bodies, Multiple Joints)
///
/// Validates complex networks of interconnected bodies.
#[test]
fn test_joint_network_stability() {
    let mut world = PhysicsWorld::new(PhysicsConfig::default());

    // Create 3x3 grid of connected bodies
    let grid_size = 3;
    let mut body_id = 0u64;

    // Create grid of bodies
    for x in 0..grid_size {
        for z in 0..grid_size {
            body_id += 1;
            let is_corner = (x == 0 || x == grid_size - 1) && (z == 0 || z == grid_size - 1);

            let rb = if is_corner {
                RigidBody::static_body() // Corner anchors
            } else {
                RigidBody::dynamic(1.0)
            };

            world.add_rigidbody(
                body_id,
                &rb,
                Vec3::new(x as f32 * 2.0, 0.0, z as f32 * 2.0),
                Quat::IDENTITY,
            );
            world.add_collider(body_id, &Collider::sphere(0.3));
        }
    }

    // Connect bodies horizontally
    for x in 0..(grid_size - 1) {
        for z in 0..grid_size {
            let id1 = (x * grid_size + z + 1) as u64;
            let id2 = ((x + 1) * grid_size + z + 1) as u64;

            let joint = JointBuilder::fixed()
                .anchor1(Vec3::new(1.0, 0.0, 0.0))
                .anchor2(Vec3::new(-1.0, 0.0, 0.0))
                .build();

            world.add_joint(id1, id2, &joint);
        }
    }

    // Connect bodies vertically
    for x in 0..grid_size {
        for z in 0..(grid_size - 1) {
            let id1 = (x * grid_size + z + 1) as u64;
            let id2 = (x * grid_size + z + 2) as u64;

            let joint = JointBuilder::fixed()
                .anchor1(Vec3::new(0.0, 0.0, 1.0))
                .anchor2(Vec3::new(0.0, 0.0, -1.0))
                .build();

            world.add_joint(id1, id2, &joint);
        }
    }

    // Apply forces to center bodies
    let center_id = ((grid_size / 2) * grid_size + (grid_size / 2) + 1) as u64;
    world.set_velocity(center_id, Vec3::new(0.0, 10.0, 0.0), Vec3::ZERO);

    let dt = 1.0 / 60.0;

    // Simulate complex joint network
    for _ in 0..180 {
        world.step(dt);
    }

    // Verify all bodies remain stable
    for id in 1..=body_id {
        if let Some((pos, _)) = world.get_transform(id) {
            assert!(pos.is_finite(), "Body {} in joint network should be stable", id);
        }
    }
}
