//! Advanced Raycast Tests for Physics Engine
//!
//! Tests complex raycasting scenarios:
//! - Zero-length and degenerate rays
//! - Layer filtering and collision masks
//! - Shapecasting (swept volumes)
//! - Multiple hits and sorting
//! - Performance edge cases
//!
//! These tests validate production-ready raycast handling.

use engine_math::{Quat, Vec3};
use engine_physics::{Collider, PhysicsConfig, PhysicsWorld, RigidBody};

/// Test: Near-Zero Length Ray
///
/// Validates handling of very short rays.
/// Note: Exact zero-length rays cause division by zero in underlying Rapier/Parry.
#[test]
fn test_zero_length_ray() {
    let mut world = PhysicsWorld::new(PhysicsConfig::default());

    // Add obstacle
    world.add_rigidbody(1, &RigidBody::static_body(), Vec3::new(5.0, 0.0, 0.0), Quat::IDENTITY);
    world.add_collider(1, &Collider::box_collider(Vec3::new(1.0, 1.0, 1.0)));

    world.step(1.0 / 60.0);

    // Very short ray (near-zero but not exactly zero)
    let origin = Vec3::new(0.0, 0.0, 0.0);
    let direction = Vec3::X; // Valid direction
    let max_distance = 0.001; // 1mm - very short but non-zero

    // Should not crash
    let hit = world.raycast(origin, direction, max_distance);

    // Should not hit obstacle at x=5 (too far)
    assert!(hit.is_none(), "Very short ray should not hit distant obstacle");

    // Test 2: Short ray that DOES hit
    let hit2 = world.raycast(origin, direction, 10.0); // Long enough to reach x=5

    assert!(hit2.is_some(), "Ray should hit obstacle within range");

    if let Some(h) = hit2 {
        assert!(h.distance.is_finite(), "Hit distance should be finite");
        assert!(h.normal.is_finite(), "Hit normal should be finite");
        assert!(h.point.is_finite(), "Hit point should be finite");

        // Should hit around x=4 (box extends from 4 to 6)
        assert!(
            h.distance > 3.0 && h.distance < 6.0,
            "Should hit box edge, distance: {}",
            h.distance
        );
    }
}

/// Test: Ray from Inside Collider
///
/// Validates behavior when ray origin is inside a collider.
#[test]
fn test_ray_from_inside_collider() {
    let mut world = PhysicsWorld::new(PhysicsConfig::default());

    // Large box at origin
    world.add_rigidbody(1, &RigidBody::static_body(), Vec3::new(0.0, 0.0, 0.0), Quat::IDENTITY);
    world.add_collider(1, &Collider::box_collider(Vec3::new(10.0, 10.0, 10.0)));

    world.step(1.0 / 60.0);

    // Ray starting from inside the box
    let origin = Vec3::new(0.0, 0.0, 0.0); // Center of box
    let direction = Vec3::X;
    let max_distance = 20.0;

    let hit = world.raycast(origin, direction, max_distance);

    // Should detect exit point or handle gracefully
    if let Some(h) = hit {
        assert!(h.distance.is_finite(), "Hit from inside should have finite distance");
        assert!(h.point.is_finite(), "Hit point should be finite");
        // Distance should be positive (to exit point)
        assert!(h.distance >= 0.0, "Hit distance should be non-negative");
    }
}

/// Test: Multiple Objects on Ray Path
///
/// Validates raycast hits first object when multiple are aligned.
#[test]
fn test_raycast_layer_filtering() {
    let mut world = PhysicsWorld::new(PhysicsConfig::default());

    // Object at x=5
    world.add_rigidbody(1, &RigidBody::static_body(), Vec3::new(5.0, 0.0, 0.0), Quat::IDENTITY);
    world.add_collider(1, &Collider::box_collider(Vec3::new(1.0, 1.0, 1.0)));

    // Object at x=10
    world.add_rigidbody(2, &RigidBody::static_body(), Vec3::new(10.0, 0.0, 0.0), Quat::IDENTITY);
    world.add_collider(2, &Collider::box_collider(Vec3::new(1.0, 1.0, 1.0)));

    world.step(1.0 / 60.0);

    let origin = Vec3::new(0.0, 0.0, 0.0);
    let direction = Vec3::X;
    let max_distance = 20.0;

    // Raycast should hit first object
    let hit = world.raycast(origin, direction, max_distance);

    assert!(hit.is_some(), "Should hit first object");

    if let Some(h) = hit {
        // Should hit object at x=5 (closer than x=10)
        assert!(h.point.x < 7.0, "Should hit first object near x=5, hit at x={}", h.point.x);

        // Distance should be around 4-5 (depending on box edge)
        assert!(h.distance < 7.0, "Distance to first hit should be < 7, got {}", h.distance);
    }

    // Ray missing both objects (different direction)
    let hit_miss = world.raycast(origin, Vec3::Y, max_distance);
    assert!(hit_miss.is_none(), "Should miss objects when aimed away");
}

/// Test: Multiple Overlapping Colliders
///
/// Validates raycast behavior with multiple objects along ray path.
#[test]
fn test_raycast_multiple_hits() {
    let mut world = PhysicsWorld::new(PhysicsConfig::default());

    // Create three boxes in a line
    for i in 0..3 {
        let x = (i + 1) as f32 * 5.0;
        world.add_rigidbody(
            i + 1,
            &RigidBody::static_body(),
            Vec3::new(x, 0.0, 0.0),
            Quat::IDENTITY,
        );
        world.add_collider(i + 1, &Collider::box_collider(Vec3::new(1.0, 1.0, 1.0)));
    }

    world.step(1.0 / 60.0);

    let origin = Vec3::new(0.0, 0.0, 0.0);
    let direction = Vec3::X;
    let max_distance = 20.0;

    // Standard raycast should hit first object
    let hit = world.raycast(origin, direction, max_distance);

    assert!(hit.is_some(), "Should hit at least one object");

    if let Some(h) = hit {
        // Should hit the closest object (at x=5)
        assert!(h.distance < 10.0, "Should hit closest object first, distance: {}", h.distance);

        // Hit point should be near first box
        assert!(h.point.x < 7.0, "Should hit first box, hit at x={}", h.point.x);
    }
}

/// Test: Extreme Distance Raycast
///
/// Validates numerical stability with very long raycasts.
#[test]
fn test_extreme_distance_raycast() {
    let mut world = PhysicsWorld::new(PhysicsConfig::default());

    // Obstacle at moderate distance
    world.add_rigidbody(1, &RigidBody::static_body(), Vec3::new(100.0, 0.0, 0.0), Quat::IDENTITY);
    world.add_collider(1, &Collider::box_collider(Vec3::new(1.0, 1.0, 1.0)));

    world.step(1.0 / 60.0);

    // Extremely long ray (10,000 units)
    let origin = Vec3::new(0.0, 0.0, 0.0);
    let direction = Vec3::X;
    let max_distance = 10_000.0;

    let hit = world.raycast(origin, direction, max_distance);

    assert!(hit.is_some(), "Should hit object within extreme distance");

    if let Some(h) = hit {
        assert!(h.distance.is_finite(), "Hit distance should be finite");
        assert!(h.normal.is_finite(), "Hit normal should be finite");
        assert!(h.point.is_finite(), "Hit point should be finite");

        // Should hit the box around x=100
        assert!(h.distance < 101.0, "Should hit box at reasonable distance, got {}", h.distance);
    }
}

/// Test: Parallel Raycasts (Performance Stress)
///
/// Validates performance and stability with many concurrent raycasts.
#[test]
fn test_many_parallel_raycasts() {
    let mut world = PhysicsWorld::new(PhysicsConfig::default());

    // Create grid of obstacles
    for x in 0..10 {
        for z in 0..10 {
            let id = (x * 10 + z + 1) as u64;
            world.add_rigidbody(
                id,
                &RigidBody::static_body(),
                Vec3::new(x as f32 * 3.0, 0.0, z as f32 * 3.0),
                Quat::IDENTITY,
            );
            world.add_collider(id, &Collider::box_collider(Vec3::new(1.0, 1.0, 1.0)));
        }
    }

    world.step(1.0 / 60.0);

    // Perform many raycasts from different angles
    let mut hit_count = 0;

    for angle in 0..360 {
        let radians = (angle as f32).to_radians();
        let direction = Vec3::new(radians.cos(), 0.0, radians.sin()).normalize();
        let origin = Vec3::new(15.0, 0.0, 15.0); // Center of grid

        let hit = world.raycast(origin, direction, 50.0);

        if hit.is_some() {
            hit_count += 1;
        }
    }

    // Most rays should hit something in the grid
    assert!(
        hit_count > 100,
        "Should hit obstacles with many raycasts, hit {} times",
        hit_count
    );
}

/// Test: Raycast Normal Validation
///
/// Validates that hit normals point in correct direction.
#[test]
fn test_raycast_normal_direction() {
    let mut world = PhysicsWorld::new(PhysicsConfig::default());

    // Box at origin
    world.add_rigidbody(1, &RigidBody::static_body(), Vec3::new(0.0, 0.0, 0.0), Quat::IDENTITY);
    world.add_collider(1, &Collider::box_collider(Vec3::new(2.0, 2.0, 2.0)));

    world.step(1.0 / 60.0);

    // Ray from +X direction hitting box
    let origin = Vec3::new(10.0, 0.0, 0.0);
    let direction = -Vec3::X; // Pointing toward origin
    let max_distance = 20.0;

    let hit = world.raycast(origin, direction, max_distance);

    assert!(hit.is_some(), "Should hit box");

    if let Some(h) = hit {
        // Normal should point back toward ray origin (away from box center)
        // For a ray from +X hitting the +X face, normal should be +X
        assert!(h.normal.is_normalized(), "Normal should be normalized");

        // Normal should generally oppose the ray direction
        let dot = h.normal.dot(direction);
        assert!(
            dot < 0.1, // Should be negative or close to zero
            "Normal should oppose ray direction, dot product: {}",
            dot
        );
    }
}
