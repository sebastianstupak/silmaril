//! Integration tests for physics system
//!
//! These tests verify complete physics scenarios:
//! - Falling boxes (gravity works)
//! - Collisions (bodies interact correctly)
//! - Raycasting (queries work)
//! - Performance (meets AAA targets)

use engine_math::{Quat, Vec3};
use engine_physics::{Collider, PhysicsConfig, PhysicsWorld, RigidBody};

#[test]
fn test_falling_box() {
    let config = PhysicsConfig::default();
    let mut world = PhysicsWorld::new(config);

    // Create ground plane
    let ground_id = 0;
    world.add_rigidbody(
        ground_id,
        &RigidBody::static_body(),
        Vec3::new(0.0, -1.0, 0.0),
        Quat::IDENTITY,
    );
    world.add_collider(ground_id, &Collider::box_collider(Vec3::new(10.0, 0.5, 10.0)));

    // Create falling box
    let box_id = 1;
    world.add_rigidbody(
        box_id,
        &RigidBody::dynamic(1.0),
        Vec3::new(0.0, 10.0, 0.0),
        Quat::IDENTITY,
    );
    world.add_collider(box_id, &Collider::box_collider(Vec3::ONE));

    // Simulate 2 seconds (120 frames) to allow box to fall and settle
    for _ in 0..120 {
        world.step(1.0 / 60.0);
    }

    // Box should have fallen
    let (pos, _) = world.get_transform(box_id).unwrap();
    assert!(pos.y < 10.0, "Box should have fallen due to gravity");
    assert!(
        pos.y > -1.0 && pos.y < 2.0,
        "Box should be resting on ground near y=0.5, got y={}",
        pos.y
    );

    // Box should have stopped (velocity near zero)
    let (linvel, _) = world.get_velocity(box_id).unwrap();
    assert!(linvel.length() < 0.5, "Box should have stopped, velocity: {}", linvel.length());
}

#[test]
fn test_collision_detection() {
    let config = PhysicsConfig::default();
    let mut world = PhysicsWorld::new(config);

    // Ground
    let ground_id = 0;
    world.add_rigidbody(ground_id, &RigidBody::static_body(), Vec3::ZERO, Quat::IDENTITY);
    world.add_collider(ground_id, &Collider::box_collider(Vec3::new(10.0, 0.5, 10.0)));

    // Falling box
    let box_id = 1;
    world.add_rigidbody(box_id, &RigidBody::dynamic(1.0), Vec3::new(0.0, 5.0, 0.0), Quat::IDENTITY);
    world.add_collider(box_id, &Collider::box_collider(Vec3::ONE));

    // Simulate until collision
    let mut collision_detected = false;
    for _ in 0..120 {
        world.step(1.0 / 60.0);

        if !world.collision_events().is_empty() {
            collision_detected = true;
            break;
        }
    }

    assert!(collision_detected, "Box should collide with ground");
}

#[test]
fn test_raycast() {
    let config = PhysicsConfig::default();
    let mut world = PhysicsWorld::new(config);

    // Create box at origin
    let box_id = 1;
    world.add_rigidbody(box_id, &RigidBody::static_body(), Vec3::ZERO, Quat::IDENTITY);
    world.add_collider(box_id, &Collider::box_collider(Vec3::ONE));

    // Step once to update query pipeline
    world.step(1.0 / 60.0);

    // Raycast downward from above
    let hit = world.raycast(
        Vec3::new(0.0, 5.0, 0.0),  // Origin
        Vec3::new(0.0, -1.0, 0.0), // Direction (downward)
        10.0,                      // Max distance
    );

    assert!(hit.is_some(), "Raycast should hit box");

    let hit = hit.unwrap();
    assert_eq!(hit.entity, box_id);
    assert!(
        hit.distance > 3.0 && hit.distance < 5.0,
        "Distance should be ~4 units (5 - 1 for box half-extent), got {}",
        hit.distance
    );
}

#[test]
fn test_stacked_boxes() {
    let config = PhysicsConfig::default();
    let mut world = PhysicsWorld::new(config);

    // Ground
    world.add_rigidbody(0, &RigidBody::static_body(), Vec3::new(0.0, -0.5, 0.0), Quat::IDENTITY);
    world.add_collider(0, &Collider::box_collider(Vec3::new(10.0, 0.5, 10.0)));

    // Stack 5 boxes
    for i in 0..5 {
        let entity_id = i + 1;
        let y = 1.0 + i as f32 * 2.1; // Slightly spaced

        world.add_rigidbody(
            entity_id,
            &RigidBody::dynamic(1.0),
            Vec3::new(0.0, y, 0.0),
            Quat::IDENTITY,
        );
        world.add_collider(entity_id, &Collider::box_collider(Vec3::ONE));
    }

    // Simulate for 2 seconds
    for _ in 0..120 {
        world.step(1.0 / 60.0);
    }

    // All boxes should have settled on top of each other
    for i in 0..5 {
        let entity_id = i + 1;
        let (pos, _) = world.get_transform(entity_id).unwrap();

        // Each box should be roughly at height = 1 + i * 2
        let expected_y = 1.0 + i as f32 * 2.0;
        assert!(
            (pos.y - expected_y).abs() < 1.0,
            "Box {} should be at y ~{}, got {}",
            i,
            expected_y,
            pos.y
        );
    }
}

#[test]
fn test_bouncing_ball() {
    let config = PhysicsConfig::default();
    let mut world = PhysicsWorld::new(config);

    // Ground (bouncy material)
    world.add_rigidbody(0, &RigidBody::static_body(), Vec3::ZERO, Quat::IDENTITY);

    let mut bouncy_collider = Collider::box_collider(Vec3::new(10.0, 0.5, 10.0));
    bouncy_collider.material.restitution = 0.9; // Very bouncy
    world.add_collider(0, &bouncy_collider);

    // Bouncing ball
    let ball_id = 1;
    world.add_rigidbody(
        ball_id,
        &RigidBody::dynamic(1.0),
        Vec3::new(0.0, 5.0, 0.0),
        Quat::IDENTITY,
    );

    let mut ball_collider = Collider::sphere(0.5);
    ball_collider.material.restitution = 0.9;
    world.add_collider(ball_id, &ball_collider);

    // Track maximum height after first bounce
    let mut max_height_after_bounce = 0.0;
    let mut bounced = false;

    for _ in 0..300 {
        world.step(1.0 / 60.0);

        let (pos, _) = world.get_transform(ball_id).unwrap();

        // Detect bounce (ball going back up)
        if pos.y < 1.0 {
            bounced = true;
        }

        if bounced && pos.y > max_height_after_bounce {
            max_height_after_bounce = pos.y;
        }
    }

    // Ball should bounce to at least 70% of original height (accounting for energy loss)
    assert!(
        max_height_after_bounce > 3.0,
        "Ball should bounce to >3 units with 0.9 restitution, got {}",
        max_height_after_bounce
    );
}

#[test]
fn test_impulse_application() {
    let config = PhysicsConfig::default();
    let mut world = PhysicsWorld::new(config);

    let box_id = 1;
    world.add_rigidbody(box_id, &RigidBody::dynamic(1.0), Vec3::ZERO, Quat::IDENTITY);
    world.add_collider(box_id, &Collider::box_collider(Vec3::ONE));

    // Apply impulse
    world.apply_impulse(box_id, Vec3::new(10.0, 0.0, 0.0));

    // Step once
    world.step(1.0 / 60.0);

    // Check velocity changed
    let (linvel, _) = world.get_velocity(box_id).unwrap();
    assert!(linvel.x > 5.0, "Impulse should have changed velocity, got {}", linvel.x);

    // Step for 1 second
    for _ in 0..59 {
        world.step(1.0 / 60.0);
    }

    // Check box moved in X direction
    let (pos, _) = world.get_transform(box_id).unwrap();
    assert!(pos.x > 5.0, "Box should have moved from impulse, got x={}", pos.x);
}

#[test]
fn test_multiple_physics_modes() {
    // Test that different modes don't crash
    let modes = vec![
        PhysicsConfig::server_authoritative(),
        PhysicsConfig::client_prediction(0.1),
        PhysicsConfig::deterministic(false),
        PhysicsConfig::default(), // LocalOnly
    ];

    for config in modes {
        let mut world = PhysicsWorld::new(config);

        world.add_rigidbody(1, &RigidBody::dynamic(1.0), Vec3::ZERO, Quat::IDENTITY);
        world.add_collider(1, &Collider::sphere(1.0));

        // Just verify it doesn't crash
        for _ in 0..10 {
            world.step(1.0 / 60.0);
        }

        assert!(world.body_count() > 0);
    }
}

#[test]
fn test_performance_1000_bodies() {
    use std::time::Instant;

    let config = PhysicsConfig::default();
    let mut world = PhysicsWorld::new(config);

    // Ground
    world.add_rigidbody(0, &RigidBody::static_body(), Vec3::new(0.0, -1.0, 0.0), Quat::IDENTITY);
    world.add_collider(0, &Collider::box_collider(Vec3::new(50.0, 0.5, 50.0)));

    // Add 1000 dynamic boxes in grid
    let grid_size = 32; // 32x32 = 1024
    for x in 0..grid_size {
        for z in 0..grid_size {
            let entity_id = x * grid_size + z + 1;
            let pos = Vec3::new(x as f32 * 2.0, 10.0, z as f32 * 2.0);

            world.add_rigidbody(entity_id, &RigidBody::dynamic(1.0), pos, Quat::IDENTITY);
            world.add_collider(entity_id, &Collider::box_collider(Vec3::ONE));
        }
    }

    assert_eq!(world.body_count(), 1025); // 1024 + ground

    // Warmup
    for _ in 0..10 {
        world.step(1.0 / 60.0);
    }

    // Measure 60 frames
    let start = Instant::now();
    for _ in 0..60 {
        world.step(1.0 / 60.0);
    }
    let elapsed = start.elapsed();

    let avg_frame_time = elapsed.as_secs_f32() / 60.0;
    let avg_frame_time_ms = avg_frame_time * 1000.0;

    println!("Average frame time for 1000 bodies: {:.2}ms", avg_frame_time_ms);

    // Performance targets (based on AAA game engines):
    // Unity PhysX: ~15-20ms for 1000 active bodies
    // Unreal Chaos: ~12-18ms for 1000 active bodies
    // Our target: < 20ms (AAA standard), < 10ms (excellent)
    assert!(
        avg_frame_time_ms < 20.0,
        "Physics step should be < 20ms for 1000 bodies (AAA standard), got {:.2}ms",
        avg_frame_time_ms
    );

    if avg_frame_time_ms < 10.0 {
        println!("✅ EXCELLENT: Beating AAA target! ({:.2}ms < 10ms)", avg_frame_time_ms);
    } else if avg_frame_time_ms < 15.0 {
        println!("✅ GREAT: Matching Unity/Unreal performance ({:.2}ms)", avg_frame_time_ms);
    } else {
        println!("✓ GOOD: Within AAA acceptable range ({:.2}ms < 20ms)", avg_frame_time_ms);
    }
}
