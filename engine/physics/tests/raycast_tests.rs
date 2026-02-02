//! Raycast tests for physics engine
//!
//! Tests raycasting functionality including:
//! - Single raycast (first hit)
//! - Multiple raycasts (all hits)
//! - Raycast misses
//! - Normal calculation
//! - Sensor collider interaction with raycasts

use engine_math::{Quat, Vec3};
use engine_physics::{Collider, ColliderShape, PhysicsConfig, PhysicsWorld, RigidBody};

#[test]
fn test_raycast_hits_ground() {
    let config = PhysicsConfig::default();
    let mut world = PhysicsWorld::new(config);

    // Create ground plane (static box)
    let ground = 1u64;
    let rb = RigidBody::static_body();
    world.add_rigidbody(ground, &rb, Vec3::new(0.0, -0.5, 0.0), Quat::IDENTITY);

    let ground_collider = Collider::box_collider(Vec3::new(50.0, 0.5, 50.0));
    world.add_collider(ground, &ground_collider);

    // Update query pipeline
    world.step(0.0);

    // Raycast downward from above ground
    let origin = Vec3::new(0.0, 10.0, 0.0);
    let direction = Vec3::new(0.0, -1.0, 0.0);
    let max_distance = 20.0;

    let hit = world.raycast(origin, direction, max_distance);

    assert!(hit.is_some(), "Raycast should hit ground");

    let hit = hit.unwrap();
    assert_eq!(hit.entity, ground);
    assert!(
        (hit.distance - 10.0).abs() < 0.1,
        "Distance should be ~10 (hit.distance={})",
        hit.distance
    );
    assert!(
        (hit.point.y - 0.0).abs() < 0.1,
        "Hit point Y should be near 0 (hit.point.y={})",
        hit.point.y
    );

    // Normal should point upward
    assert!(hit.normal.y > 0.9, "Normal should point up (hit.normal.y={})", hit.normal.y);
}

#[test]
fn test_raycast_misses() {
    let config = PhysicsConfig::default();
    let mut world = PhysicsWorld::new(config);

    // Create ground plane
    let ground = 1u64;
    let rb = RigidBody::static_body();
    world.add_rigidbody(ground, &rb, Vec3::new(0.0, -0.5, 0.0), Quat::IDENTITY);

    let ground_collider = Collider::box_collider(Vec3::new(50.0, 0.5, 50.0));
    world.add_collider(ground, &ground_collider);

    world.step(0.0);

    // Raycast upward (should miss)
    let origin = Vec3::new(0.0, 10.0, 0.0);
    let direction = Vec3::new(0.0, 1.0, 0.0);
    let max_distance = 20.0;

    let hit = world.raycast(origin, direction, max_distance);

    assert!(hit.is_none(), "Raycast upward should miss");
}

#[test]
fn test_raycast_max_distance() {
    let config = PhysicsConfig::default();
    let mut world = PhysicsWorld::new(config);

    // Create ground plane
    let ground = 1u64;
    let rb = RigidBody::static_body();
    world.add_rigidbody(ground, &rb, Vec3::new(0.0, -0.5, 0.0), Quat::IDENTITY);

    let ground_collider = Collider::box_collider(Vec3::new(50.0, 0.5, 50.0));
    world.add_collider(ground, &ground_collider);

    world.step(0.0);

    // Raycast with max_distance too short to reach ground
    let origin = Vec3::new(0.0, 10.0, 0.0);
    let direction = Vec3::new(0.0, -1.0, 0.0);
    let max_distance = 5.0; // Only 5 units, ground is 10 units away

    let hit = world.raycast(origin, direction, max_distance);

    assert!(hit.is_none(), "Raycast should not reach ground with short max_distance");
}

#[test]
fn test_raycast_all_multiple_hits() {
    let config = PhysicsConfig::default();
    let mut world = PhysicsWorld::new(config);

    // Create three platforms at different heights
    for i in 0..3 {
        let entity_id = (i + 1) as u64;
        let height = (i as f32) * 5.0; // Heights: 0, 5, 10

        let rb = RigidBody::static_body();
        world.add_rigidbody(entity_id, &rb, Vec3::new(0.0, height, 0.0), Quat::IDENTITY);

        let collider = Collider::box_collider(Vec3::new(2.0, 0.5, 2.0));
        world.add_collider(entity_id, &collider);
    }

    world.step(0.0);

    // Raycast downward from above all platforms
    let origin = Vec3::new(0.0, 15.0, 0.0);
    let direction = Vec3::new(0.0, -1.0, 0.0);
    let max_distance = 20.0;

    let hits = world.raycast_all(origin, direction, max_distance);

    assert_eq!(hits.len(), 3, "Should hit all three platforms");

    // Hits should be sorted by distance (nearest first)
    // Platform at height 10 should be hit first
    assert_eq!(hits[0].entity, 3, "First hit should be entity 3 (highest platform)");
    assert!(
        (hits[0].distance - 4.5).abs() < 0.6,
        "First hit distance should be ~5 (hits[0].distance={})",
        hits[0].distance
    );

    assert_eq!(hits[1].entity, 2, "Second hit should be entity 2");
    assert_eq!(hits[2].entity, 1, "Third hit should be entity 1 (lowest platform)");

    // Verify distances are sorted
    assert!(
        hits[0].distance < hits[1].distance,
        "Hits should be sorted by distance (ascending)"
    );
    assert!(hits[1].distance < hits[2].distance);
}

#[test]
fn test_raycast_all_empty() {
    let config = PhysicsConfig::default();
    let world = PhysicsWorld::new(config);

    // Raycast in empty world
    let origin = Vec3::new(0.0, 10.0, 0.0);
    let direction = Vec3::new(0.0, -1.0, 0.0);
    let max_distance = 20.0;

    let hits = world.raycast_all(origin, direction, max_distance);

    assert!(hits.is_empty(), "Raycast in empty world should return no hits");
}

#[test]
fn test_raycast_sphere_collider() {
    let config = PhysicsConfig::default();
    let mut world = PhysicsWorld::new(config);

    // Create sphere
    let sphere = 1u64;
    let rb = RigidBody::static_body();
    world.add_rigidbody(sphere, &rb, Vec3::new(0.0, 0.0, 0.0), Quat::IDENTITY);

    let sphere_collider = Collider::sphere(1.0);
    world.add_collider(sphere, &sphere_collider);

    world.step(0.0);

    // Raycast toward sphere center
    let origin = Vec3::new(-5.0, 0.0, 0.0);
    let direction = Vec3::new(1.0, 0.0, 0.0);
    let max_distance = 10.0;

    let hit = world.raycast(origin, direction, max_distance);

    assert!(hit.is_some(), "Should hit sphere");

    let hit = hit.unwrap();
    assert_eq!(hit.entity, sphere);

    // Should hit at sphere surface (radius = 1.0, centered at origin)
    // Ray from (-5, 0, 0) toward (1, 0, 0) should hit at (-1, 0, 0)
    // Distance = 4.0
    assert!(
        (hit.distance - 4.0).abs() < 0.1,
        "Distance should be ~4.0 (hit.distance={})",
        hit.distance
    );

    // Normal should point toward ray origin (outward from sphere)
    assert!(
        hit.normal.x < -0.9,
        "Normal should point left (toward ray origin) (hit.normal.x={})",
        hit.normal.x
    );
}

#[test]
fn test_raycast_capsule_collider() {
    let config = PhysicsConfig::default();
    let mut world = PhysicsWorld::new(config);

    // Create capsule (upright character-like)
    let capsule = 1u64;
    let rb = RigidBody::static_body();
    world.add_rigidbody(capsule, &rb, Vec3::new(0.0, 1.0, 0.0), Quat::IDENTITY);

    let capsule_collider = Collider::capsule(1.0, 0.5); // half_height=1.0, radius=0.5
    world.add_collider(capsule, &capsule_collider);

    world.step(0.0);

    // Raycast from side
    let origin = Vec3::new(-5.0, 1.0, 0.0);
    let direction = Vec3::new(1.0, 0.0, 0.0);
    let max_distance = 10.0;

    let hit = world.raycast(origin, direction, max_distance);

    assert!(hit.is_some(), "Should hit capsule");

    let hit = hit.unwrap();
    assert_eq!(hit.entity, capsule);
}

#[test]
fn test_raycast_ignores_sensor_colliders() {
    let config = PhysicsConfig::default();
    let mut world = PhysicsWorld::new(config);

    // Create sensor collider (trigger)
    let trigger = 1u64;
    let rb = RigidBody::static_body();
    world.add_rigidbody(trigger, &rb, Vec3::new(0.0, 0.0, 0.0), Quat::IDENTITY);

    let sensor_collider = Collider::sensor(ColliderShape::Box { half_extents: Vec3::ONE });
    world.add_collider(trigger, &sensor_collider);

    world.step(0.0);

    // Raycast through sensor
    let origin = Vec3::new(-5.0, 0.0, 0.0);
    let direction = Vec3::new(1.0, 0.0, 0.0);
    let max_distance = 10.0;

    let hit = world.raycast(origin, direction, max_distance);

    // Sensors should NOT block raycasts by default
    // (Rapier's default QueryFilter excludes sensors)
    assert!(hit.is_none(), "Raycast should pass through sensor colliders by default");
}

#[test]
fn test_raycast_horizontal() {
    let config = PhysicsConfig::default();
    let mut world = PhysicsWorld::new(config);

    // Create wall
    let wall = 1u64;
    let rb = RigidBody::static_body();
    world.add_rigidbody(wall, &rb, Vec3::new(10.0, 0.0, 0.0), Quat::IDENTITY);

    let wall_collider = Collider::box_collider(Vec3::new(1.0, 5.0, 5.0));
    world.add_collider(wall, &wall_collider);

    world.step(0.0);

    // Raycast horizontally toward wall
    let origin = Vec3::new(0.0, 0.0, 0.0);
    let direction = Vec3::new(1.0, 0.0, 0.0);
    let max_distance = 20.0;

    let hit = world.raycast(origin, direction, max_distance);

    assert!(hit.is_some(), "Should hit wall");

    let hit = hit.unwrap();
    assert_eq!(hit.entity, wall);

    // Distance should be ~9.0 (wall at x=10, width=1, so surface at x=9)
    assert!(
        (hit.distance - 9.0).abs() < 0.1,
        "Distance should be ~9.0 (hit.distance={})",
        hit.distance
    );

    // Normal should point back toward origin
    assert!(
        hit.normal.x < -0.9,
        "Normal should point left (toward origin) (hit.normal.x={})",
        hit.normal.x
    );
}

#[test]
fn test_raycast_diagonal() {
    let config = PhysicsConfig::default();
    let mut world = PhysicsWorld::new(config);

    // Create ground
    let ground = 1u64;
    let rb = RigidBody::static_body();
    world.add_rigidbody(ground, &rb, Vec3::new(0.0, -0.5, 0.0), Quat::IDENTITY);

    let ground_collider = Collider::box_collider(Vec3::new(50.0, 0.5, 50.0));
    world.add_collider(ground, &ground_collider);

    world.step(0.0);

    // Raycast diagonally downward
    let origin = Vec3::new(0.0, 10.0, 0.0);
    let direction = Vec3::new(0.707, -0.707, 0.0).normalize(); // 45 degrees down+right
    let max_distance = 20.0;

    let hit = world.raycast(origin, direction, max_distance);

    assert!(hit.is_some(), "Should hit ground on diagonal ray");

    let hit = hit.unwrap();
    assert_eq!(hit.entity, ground);

    // Normal should still point upward (ground normal)
    assert!(hit.normal.y > 0.9, "Ground normal should point up");
}
