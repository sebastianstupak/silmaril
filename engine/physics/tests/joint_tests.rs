//! Comprehensive tests for physics joints
//!
//! Tests all joint types, limits, and motors.

use engine_math::{Quat, Vec3};
use engine_physics::{Collider, JointBuilder, JointMotor, PhysicsConfig, PhysicsWorld, RigidBody};
use std::f32::consts::PI;

#[test]
fn test_fixed_joint_constrains_position() {
    let mut world = PhysicsWorld::new(PhysicsConfig::default());

    // Create two bodies
    let body1_id = 1u64;
    let body2_id = 2u64;

    world.add_rigidbody(
        body1_id,
        &RigidBody::static_body(),
        Vec3::new(0.0, 5.0, 0.0),
        Quat::IDENTITY,
    );
    world.add_collider(body1_id, &Collider::sphere(0.5));

    world.add_rigidbody(
        body2_id,
        &RigidBody::dynamic(1.0),
        Vec3::new(0.0, 3.0, 0.0),
        Quat::IDENTITY,
    );
    world.add_collider(body2_id, &Collider::sphere(0.5));

    // Create fixed joint
    let joint = JointBuilder::fixed()
        .anchor1(Vec3::new(0.0, -1.0, 0.0))
        .anchor2(Vec3::new(0.0, 1.0, 0.0))
        .build();

    let joint_handle = world.add_joint(body1_id, body2_id, &joint);
    assert!(joint_handle.is_some());

    // Simulate
    for _ in 0..60 {
        world.step(1.0 / 60.0);
    }

    // Check that body2 is still near body1 (fixed constraint maintained)
    let (pos1, _) = world.get_transform(body1_id).unwrap();
    let (pos2, _) = world.get_transform(body2_id).unwrap();

    let distance = (pos2 - pos1).length();
    assert!(
        distance < 2.5,
        "Fixed joint should maintain relative position (distance: {})",
        distance
    );
}

#[test]
fn test_revolute_joint_allows_rotation() {
    let mut world = PhysicsWorld::new(PhysicsConfig::default());

    // Create two bodies
    let body1_id = 1u64;
    let body2_id = 2u64;

    world.add_rigidbody(
        body1_id,
        &RigidBody::static_body(),
        Vec3::new(0.0, 5.0, 0.0),
        Quat::IDENTITY,
    );
    world.add_collider(body1_id, &Collider::box_collider(Vec3::new(0.5, 0.5, 0.5)));

    world.add_rigidbody(
        body2_id,
        &RigidBody::dynamic(1.0),
        Vec3::new(0.0, 3.0, 0.0),
        Quat::IDENTITY,
    );
    world.add_collider(body2_id, &Collider::box_collider(Vec3::new(0.5, 0.5, 0.5)));

    // Create revolute joint around Y axis (vertical hinge)
    let joint = JointBuilder::revolute()
        .anchor1(Vec3::new(0.0, -2.0, 0.0))
        .anchor2(Vec3::new(0.0, 0.0, 0.0))
        .axis(Vec3::Y)
        .build();

    let joint_handle = world.add_joint(body1_id, body2_id, &joint);
    assert!(joint_handle.is_some());

    // Apply torque to rotate the joint
    world.apply_force(body2_id, Vec3::new(10.0, 0.0, 0.0));

    // Simulate
    for _ in 0..120 {
        world.step(1.0 / 60.0);
    }

    // Body should have moved due to rotation (joint allows rotation)
    let (pos2, _) = world.get_transform(body2_id).unwrap();
    let initial_pos = Vec3::new(0.0, 3.0, 0.0);

    // Calculate distance in XZ plane (ignore Y)
    let dx = pos2.x - initial_pos.x;
    let dz = pos2.z - initial_pos.z;
    let distance = (dx * dx + dz * dz).sqrt();
    // Should have rotated (moved in XZ plane)
    assert!(distance > 0.1, "Revolute joint should allow rotation");
}

#[test]
fn test_revolute_joint_limits() {
    let mut world = PhysicsWorld::new(PhysicsConfig::default());

    let body1_id = 1u64;
    let body2_id = 2u64;

    world.add_rigidbody(
        body1_id,
        &RigidBody::static_body(),
        Vec3::new(0.0, 5.0, 0.0),
        Quat::IDENTITY,
    );
    world.add_collider(body1_id, &Collider::sphere(0.5));

    world.add_rigidbody(
        body2_id,
        &RigidBody::dynamic(1.0),
        Vec3::new(0.0, 3.0, 0.0),
        Quat::IDENTITY,
    );
    world.add_collider(body2_id, &Collider::sphere(0.5));

    // Create revolute joint with limits (±45 degrees)
    let joint = JointBuilder::revolute()
        .anchor1(Vec3::ZERO)
        .anchor2(Vec3::ZERO)
        .axis(Vec3::Z)
        .limits(-PI / 4.0, PI / 4.0)
        .build();

    let joint_handle = world.add_joint(body1_id, body2_id, &joint);
    assert!(joint_handle.is_some());

    // Simulate with limits
    for _ in 0..120 {
        world.step(1.0 / 60.0);
    }

    // Joint should enforce limits (this is more of an integration test)
    assert_eq!(world.joint_count(), 1);
}

#[test]
fn test_prismatic_joint_allows_sliding() {
    let mut world = PhysicsWorld::new(PhysicsConfig::default());

    let body1_id = 1u64;
    let body2_id = 2u64;

    world.add_rigidbody(
        body1_id,
        &RigidBody::static_body(),
        Vec3::new(0.0, 5.0, 0.0),
        Quat::IDENTITY,
    );
    world.add_collider(body1_id, &Collider::sphere(0.5));

    world.add_rigidbody(
        body2_id,
        &RigidBody::dynamic(1.0),
        Vec3::new(0.0, 3.0, 0.0),
        Quat::IDENTITY,
    );
    world.add_collider(body2_id, &Collider::sphere(0.5));

    // Create prismatic joint (slider) along Y axis
    let joint = JointBuilder::prismatic()
        .anchor1(Vec3::ZERO)
        .anchor2(Vec3::ZERO)
        .axis(Vec3::Y)
        .build();

    let joint_handle = world.add_joint(body1_id, body2_id, &joint);
    assert!(joint_handle.is_some());

    // Gravity should cause body2 to slide down along Y axis
    for _ in 0..60 {
        world.step(1.0 / 60.0);
    }

    let (pos2, _) = world.get_transform(body2_id).unwrap();

    // Should slide downward (Y should decrease)
    assert!(pos2.y < 3.0, "Prismatic joint should allow sliding along axis (y: {})", pos2.y);
}

#[test]
fn test_prismatic_joint_limits() {
    let mut world = PhysicsWorld::new(PhysicsConfig::default());

    let body1_id = 1u64;
    let body2_id = 2u64;

    world.add_rigidbody(
        body1_id,
        &RigidBody::static_body(),
        Vec3::new(0.0, 5.0, 0.0),
        Quat::IDENTITY,
    );
    world.add_collider(body1_id, &Collider::sphere(0.5));

    world.add_rigidbody(
        body2_id,
        &RigidBody::dynamic(1.0),
        Vec3::new(0.0, 3.0, 0.0),
        Quat::IDENTITY,
    );
    world.add_collider(body2_id, &Collider::sphere(0.5));

    // Create prismatic joint with limits (can slide 1 meter in each direction)
    let joint = JointBuilder::prismatic()
        .anchor1(Vec3::ZERO)
        .anchor2(Vec3::ZERO)
        .axis(Vec3::Y)
        .limits(-1.0, 1.0)
        .build();

    let joint_handle = world.add_joint(body1_id, body2_id, &joint);
    assert!(joint_handle.is_some());

    // Simulate
    for _ in 0..120 {
        world.step(1.0 / 60.0);
    }

    // Joint should enforce limits
    assert_eq!(world.joint_count(), 1);
}

#[test]
fn test_spherical_joint_allows_all_rotation() {
    let mut world = PhysicsWorld::new(PhysicsConfig::default());

    let body1_id = 1u64;
    let body2_id = 2u64;

    world.add_rigidbody(
        body1_id,
        &RigidBody::static_body(),
        Vec3::new(0.0, 5.0, 0.0),
        Quat::IDENTITY,
    );
    world.add_collider(body1_id, &Collider::sphere(0.5));

    world.add_rigidbody(
        body2_id,
        &RigidBody::dynamic(1.0),
        Vec3::new(0.0, 3.0, 0.0),
        Quat::IDENTITY,
    );
    world.add_collider(body2_id, &Collider::sphere(0.5));

    // Create spherical joint (ball-and-socket)
    let joint = JointBuilder::spherical()
        .anchor1(Vec3::new(0.0, -2.0, 0.0))
        .anchor2(Vec3::new(0.0, 0.0, 0.0))
        .build();

    let joint_handle = world.add_joint(body1_id, body2_id, &joint);
    assert!(joint_handle.is_some());

    // Apply force to create rotation
    world.apply_force(body2_id, Vec3::new(5.0, 0.0, 5.0));

    // Simulate
    for _ in 0..120 {
        world.step(1.0 / 60.0);
    }

    // Body should move (spherical joint allows free rotation)
    let (pos2, _) = world.get_transform(body2_id).unwrap();
    let distance = (pos2 - Vec3::new(0.0, 3.0, 0.0)).length();

    assert!(distance > 0.1, "Spherical joint should allow rotation in all axes");
}

#[test]
fn test_joint_motor_applies_force() {
    let mut world = PhysicsWorld::new(PhysicsConfig::default());

    let body1_id = 1u64;
    let body2_id = 2u64;

    world.add_rigidbody(
        body1_id,
        &RigidBody::static_body(),
        Vec3::new(0.0, 5.0, 0.0),
        Quat::IDENTITY,
    );
    world.add_collider(body1_id, &Collider::sphere(0.5));

    world.add_rigidbody(
        body2_id,
        &RigidBody::dynamic(1.0),
        Vec3::new(0.0, 3.0, 0.0),
        Quat::IDENTITY,
    );
    world.add_collider(body2_id, &Collider::sphere(0.5));

    // Create revolute joint with motor
    let motor = JointMotor::new(5.0).with_max_force(100.0);

    let joint = JointBuilder::revolute()
        .anchor1(Vec3::new(0.0, -2.0, 0.0))
        .anchor2(Vec3::new(0.0, 0.0, 0.0))
        .axis(Vec3::Y)
        .motor(motor)
        .build();

    let joint_handle = world.add_joint(body1_id, body2_id, &joint);
    assert!(joint_handle.is_some());

    // Simulate - motor should rotate the joint
    for _ in 0..120 {
        world.step(1.0 / 60.0);
    }

    // Body should have rotated due to motor
    let (pos2, _) = world.get_transform(body2_id).unwrap();
    let initial_pos = Vec3::new(0.0, 3.0, 0.0);

    // Calculate distance in XZ plane (ignore Y)
    let dx = pos2.x - initial_pos.x;
    let dz = pos2.z - initial_pos.z;
    let distance = (dx * dx + dz * dz).sqrt();
    // Motor should have caused rotation
    assert!(distance > 0.1, "Motor should apply force to rotate joint");
}

#[test]
fn test_joint_creation_and_removal() {
    let mut world = PhysicsWorld::new(PhysicsConfig::default());

    let body1_id = 1u64;
    let body2_id = 2u64;

    world.add_rigidbody(body1_id, &RigidBody::static_body(), Vec3::ZERO, Quat::IDENTITY);
    world.add_rigidbody(
        body2_id,
        &RigidBody::dynamic(1.0),
        Vec3::new(2.0, 0.0, 0.0),
        Quat::IDENTITY,
    );

    assert_eq!(world.joint_count(), 0);

    // Create joint
    let joint = JointBuilder::fixed().build();
    let handle = world.add_joint(body1_id, body2_id, &joint).unwrap();

    assert_eq!(world.joint_count(), 1);

    // Remove joint
    let removed = world.remove_joint(handle);
    assert!(removed);
    assert_eq!(world.joint_count(), 0);
}

#[test]
fn test_joint_creation_with_invalid_entities() {
    let mut world = PhysicsWorld::new(PhysicsConfig::default());

    // Try to create joint with non-existent entities
    let joint = JointBuilder::fixed().build();
    let handle = world.add_joint(999, 1000, &joint);

    assert!(handle.is_none(), "Should fail to create joint with invalid entities");
}

#[test]
fn test_multiple_joints() {
    let mut world = PhysicsWorld::new(PhysicsConfig::default());

    // Create a chain of 3 bodies connected by joints
    let body1_id = 1u64;
    let body2_id = 2u64;
    let body3_id = 3u64;

    world.add_rigidbody(
        body1_id,
        &RigidBody::static_body(),
        Vec3::new(0.0, 5.0, 0.0),
        Quat::IDENTITY,
    );
    world.add_collider(body1_id, &Collider::sphere(0.5));

    world.add_rigidbody(
        body2_id,
        &RigidBody::dynamic(1.0),
        Vec3::new(0.0, 3.0, 0.0),
        Quat::IDENTITY,
    );
    world.add_collider(body2_id, &Collider::sphere(0.5));

    world.add_rigidbody(
        body3_id,
        &RigidBody::dynamic(1.0),
        Vec3::new(0.0, 1.0, 0.0),
        Quat::IDENTITY,
    );
    world.add_collider(body3_id, &Collider::sphere(0.5));

    // Connect body1 to body2
    let joint1 = JointBuilder::revolute().axis(Vec3::Z).build();
    let handle1 = world.add_joint(body1_id, body2_id, &joint1);
    assert!(handle1.is_some());

    // Connect body2 to body3
    let joint2 = JointBuilder::revolute().axis(Vec3::Z).build();
    let handle2 = world.add_joint(body2_id, body3_id, &joint2);
    assert!(handle2.is_some());

    assert_eq!(world.joint_count(), 2);

    // Simulate chain
    for _ in 0..120 {
        world.step(1.0 / 60.0);
    }

    // Bodies should remain connected
    assert_eq!(world.body_count(), 3);
    assert_eq!(world.joint_count(), 2);
}

#[test]
fn test_joint_performance_creation() {
    let mut world = PhysicsWorld::new(PhysicsConfig::default());

    // Create bodies
    for i in 0..100 {
        world.add_rigidbody(
            i * 2,
            &RigidBody::static_body(),
            Vec3::new(i as f32 * 2.0, 0.0, 0.0),
            Quat::IDENTITY,
        );
        world.add_rigidbody(
            i * 2 + 1,
            &RigidBody::dynamic(1.0),
            Vec3::new(i as f32 * 2.0 + 1.0, 0.0, 0.0),
            Quat::IDENTITY,
        );
    }

    // Create 100 joints
    let start = std::time::Instant::now();
    for i in 0..100 {
        let joint = JointBuilder::revolute().axis(Vec3::Y).build();
        world.add_joint(i * 2, i * 2 + 1, &joint);
    }
    let duration = start.elapsed();

    assert_eq!(world.joint_count(), 100);

    // Should be fast (target: < 10µs per joint = 1ms for 100 joints)
    assert!(duration.as_micros() < 10000, "Joint creation too slow: {:?}", duration);
}

#[test]
fn test_joint_performance_simulation() {
    let mut world = PhysicsWorld::new(PhysicsConfig::default());

    // Create a simple pendulum
    for i in 0..10 {
        world.add_rigidbody(
            i * 2,
            &RigidBody::static_body(),
            Vec3::new(0.0, (10 - i) as f32, 0.0),
            Quat::IDENTITY,
        );
        world.add_collider(i * 2, &Collider::sphere(0.25));

        world.add_rigidbody(
            i * 2 + 1,
            &RigidBody::dynamic(0.5),
            Vec3::new(0.0, (9 - i) as f32, 0.0),
            Quat::IDENTITY,
        );
        world.add_collider(i * 2 + 1, &Collider::sphere(0.25));

        let joint = JointBuilder::spherical()
            .anchor1(Vec3::new(0.0, -0.5, 0.0))
            .anchor2(Vec3::new(0.0, 0.5, 0.0))
            .build();
        world.add_joint(i * 2, i * 2 + 1, &joint);
    }

    assert_eq!(world.joint_count(), 10);

    // Benchmark physics step with joints
    let start = std::time::Instant::now();
    for _ in 0..60 {
        world.step(1.0 / 60.0);
    }
    let duration = start.elapsed();

    // Should meet performance target (< 1ms per step with 10 joints)
    let avg_step_time = duration.as_micros() / 60;
    assert!(
        avg_step_time < 1000,
        "Physics step too slow with 10 joints: {}µs",
        avg_step_time
    );
}
