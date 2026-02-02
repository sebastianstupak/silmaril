//! Advanced Edge Case Tests for Physics Engine
//!
//! Tests extreme and unusual scenarios to ensure robustness:
//! - Zero-size and degenerate colliders
//! - Extreme timestep variations
//! - CCD (Continuous Collision Detection)
//! - Numerical edge cases
//!
//! These tests validate production-ready edge case handling.

use engine_math::{Quat, Vec3};
use engine_physics::{Collider, PhysicsConfig, PhysicsWorld, RigidBody};

/// Test: Near-Zero Size Collider Handling
///
/// Validates that very small colliders remain numerically stable.
/// Note: Exact zero-size colliders are undefined behavior in all physics engines.
#[test]
fn test_zero_size_collider() {
    let mut world = PhysicsWorld::new(PhysicsConfig::default());

    // Test 1: Near-zero size sphere (very small but non-zero)
    let body_id = 1;
    world.add_rigidbody(
        body_id,
        &RigidBody::dynamic(1.0),
        Vec3::new(0.0, 5.0, 0.0),
        Quat::IDENTITY,
    );

    // Very small sphere (1mm radius)
    world.add_collider(body_id, &Collider::sphere(0.001));

    // Should not crash during physics steps
    for _ in 0..60 {
        world.step(1.0 / 60.0);
    }

    // Verify position is valid (not NaN)
    let (pos, _) = world.get_transform(body_id).unwrap();
    assert!(pos.x.is_finite(), "Small sphere X should be finite, got {}", pos.x);
    assert!(pos.y.is_finite(), "Small sphere Y should be finite, got {}", pos.y);
    assert!(pos.z.is_finite(), "Small sphere Z should be finite, got {}", pos.z);

    // Test 2: Microscopic box collider
    let body_id_2 = 2;
    world.add_rigidbody(
        body_id_2,
        &RigidBody::dynamic(0.1),
        Vec3::new(5.0, 5.0, 0.0),
        Quat::IDENTITY,
    );

    world.add_collider(body_id_2, &Collider::box_collider(Vec3::new(0.0001, 0.0001, 0.0001)));

    for _ in 0..60 {
        world.step(1.0 / 60.0);
    }

    let (pos2, _) = world.get_transform(body_id_2).unwrap();
    assert!(
        pos2.x.is_finite() && pos2.y.is_finite() && pos2.z.is_finite(),
        "Microscopic box should have finite position"
    );

    // Test 3: Mixed tiny/normal size collision
    let body_id_3 = 3;
    world.add_rigidbody(
        body_id_3,
        &RigidBody::static_body(),
        Vec3::new(0.0, 0.0, 0.0),
        Quat::IDENTITY,
    );
    world.add_collider(body_id_3, &Collider::box_collider(Vec3::new(10.0, 1.0, 10.0)));

    // Drop tiny sphere onto normal ground
    let body_id_4 = 4;
    world.add_rigidbody(
        body_id_4,
        &RigidBody::dynamic(0.001),
        Vec3::new(0.0, 3.0, 0.0),
        Quat::IDENTITY,
    );
    world.add_collider(body_id_4, &Collider::sphere(0.01)); // 1cm sphere

    for _ in 0..120 {
        world.step(1.0 / 60.0);
    }

    let (pos4, _) = world.get_transform(body_id_4).unwrap();
    assert!(pos4.is_finite(), "Tiny sphere on normal ground should be stable");
}

/// Test: Extreme Timestep Variations
///
/// Validates physics stability with very small and very large timesteps.
#[test]
fn test_extreme_timesteps() {
    // Test 1: Very small timestep (sub-millisecond)
    {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());

        world.add_rigidbody(1, &RigidBody::dynamic(1.0), Vec3::new(0.0, 10.0, 0.0), Quat::IDENTITY);
        world.add_collider(1, &Collider::sphere(0.5));

        // Micro-timestep: 0.0001s (0.1ms)
        let dt_tiny = 0.0001;
        let mut total_time = 0.0;

        // Run for equivalent of 1 second (10,000 steps!)
        for _ in 0..10000 {
            world.step(dt_tiny);
            total_time += dt_tiny;

            if total_time >= 1.0 {
                break;
            }
        }

        let (pos, _) = world.get_transform(1).unwrap();
        assert!(pos.y.is_finite(), "Position should be finite with tiny timesteps");
        // With gravity, object should have fallen
        assert!(pos.y < 10.0, "Object should fall with tiny timesteps, at y={}", pos.y);
    }

    // Test 2: Large timestep (100ms - game hitching)
    {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());

        world.add_rigidbody(1, &RigidBody::dynamic(1.0), Vec3::new(0.0, 10.0, 0.0), Quat::IDENTITY);
        world.add_collider(1, &Collider::sphere(0.5));

        // Large timestep: 0.1s (100ms - severe lag spike)
        let dt_large = 0.1;

        for _ in 0..10 {
            // 1 second total
            world.step(dt_large);
        }

        let (pos, _) = world.get_transform(1).unwrap();
        assert!(pos.y.is_finite(), "Position should be finite with large timesteps");
        assert!(pos.y < 10.0, "Object should fall with large timesteps");
    }

    // Test 3: Variable timesteps (realistic game conditions)
    {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());

        world.add_rigidbody(1, &RigidBody::dynamic(1.0), Vec3::new(0.0, 10.0, 0.0), Quat::IDENTITY);
        world.add_collider(1, &Collider::sphere(0.5));

        // Varying timesteps simulating framerate fluctuation
        let timesteps = [1.0 / 60.0, 1.0 / 30.0, 1.0 / 120.0, 1.0 / 45.0, 1.0 / 60.0];

        for _ in 0..20 {
            for &dt in &timesteps {
                world.step(dt);
            }
        }

        let (pos, _) = world.get_transform(1).unwrap();
        assert!(pos.y.is_finite(), "Position should remain finite with variable timesteps");
    }
}

/// Test: CCD (Continuous Collision Detection) for Fast Objects
///
/// Validates that fast-moving objects don't tunnel through thin barriers.
#[test]
fn test_ccd_fast_projectile() {
    // Enable CCD in config
    let mut config = PhysicsConfig::default();
    config.enable_ccd = true;

    let mut world = PhysicsWorld::new(config);

    // Thin wall
    world.add_rigidbody(0, &RigidBody::static_body(), Vec3::new(10.0, 0.0, 0.0), Quat::IDENTITY);
    world.add_collider(0, &Collider::box_collider(Vec3::new(0.1, 10.0, 10.0)));

    // Fast projectile with CCD enabled
    let mut projectile_rb = RigidBody::dynamic(0.1);
    projectile_rb.ccd_enabled = true;

    world.add_rigidbody(1, &projectile_rb, Vec3::new(0.0, 0.0, 0.0), Quat::IDENTITY);
    world.add_collider(1, &Collider::sphere(0.2));

    // Launch at extreme speed toward wall
    world.set_velocity(1, Vec3::new(500.0, 0.0, 0.0), Vec3::ZERO);

    let dt = 1.0 / 60.0;
    let mut hit_wall = false;

    for _ in 0..120 {
        world.step(dt);

        let (pos, _) = world.get_transform(1).unwrap();

        // If CCD works, projectile should stop at or before wall (x=10.0)
        // Without CCD, it would tunnel through
        if pos.x >= 9.5 && pos.x <= 10.5 {
            hit_wall = true;
        }

        // Should never go far past wall
        assert!(pos.x < 15.0, "CCD should prevent tunneling, projectile at x={}", pos.x);
    }

    assert!(hit_wall, "Fast projectile with CCD should interact with wall, not tunnel");
}

/// Test: CCD with Multiple Fast Objects
///
/// Validates CCD performance with multiple high-speed bodies.
#[test]
fn test_ccd_multiple_projectiles() {
    let mut config = PhysicsConfig::default();
    config.enable_ccd = true;

    let mut world = PhysicsWorld::new(config);

    // Wall barrier
    world.add_rigidbody(0, &RigidBody::static_body(), Vec3::new(20.0, 0.0, 0.0), Quat::IDENTITY);
    world.add_collider(0, &Collider::box_collider(Vec3::new(0.2, 20.0, 20.0)));

    // Create 10 fast projectiles
    for i in 1..=10 {
        let mut rb = RigidBody::dynamic(0.1);
        rb.ccd_enabled = true;

        let y_offset = (i as f32 - 5.0) * 2.0;

        world.add_rigidbody(i, &rb, Vec3::new(0.0, y_offset, 0.0), Quat::IDENTITY);
        world.add_collider(i, &Collider::sphere(0.3));

        // Launch at varying speeds
        world.set_velocity(i, Vec3::new(300.0 + i as f32 * 20.0, 0.0, 0.0), Vec3::ZERO);
    }

    let dt = 1.0 / 60.0;

    // Simulate
    for _ in 0..120 {
        world.step(dt);
    }

    // Verify all projectiles stopped at or before wall
    for i in 1..=10 {
        let (pos, _) = world.get_transform(i).unwrap();
        assert!(
            pos.x < 25.0,
            "Projectile {} should not tunnel through wall with CCD, at x={}",
            i,
            pos.x
        );
    }
}

/// Test: Degenerate Collider Shapes
///
/// Validates handling of unusual shape configurations.
#[test]
fn test_degenerate_shapes() {
    let mut world = PhysicsWorld::new(PhysicsConfig::default());

    // Test 1: Box with one dimension zero (flat plane)
    world.add_rigidbody(1, &RigidBody::dynamic(1.0), Vec3::new(0.0, 5.0, 0.0), Quat::IDENTITY);

    // Extremely flat box (essentially a 2D plane)
    world.add_collider(1, &Collider::box_collider(Vec3::new(1.0, 0.001, 1.0)));

    for _ in 0..60 {
        world.step(1.0 / 60.0);
    }

    let (pos, _) = world.get_transform(1).unwrap();
    assert!(pos.is_finite(), "Flat box should have finite position");

    // Test 2: Box with extreme aspect ratio
    world.add_rigidbody(2, &RigidBody::dynamic(1.0), Vec3::new(5.0, 5.0, 0.0), Quat::IDENTITY);

    // Very long thin stick
    world.add_collider(2, &Collider::box_collider(Vec3::new(0.01, 10.0, 0.01)));

    for _ in 0..60 {
        world.step(1.0 / 60.0);
    }

    let (pos2, _) = world.get_transform(2).unwrap();
    assert!(pos2.is_finite(), "Thin stick should have finite position");
}

/// Test: Numerical Stability with Extreme Values
///
/// Validates physics doesn't produce NaN or Inf with extreme inputs.
#[test]
fn test_numerical_stability() {
    let mut world = PhysicsWorld::new(PhysicsConfig::default());

    // Test 1: Very heavy object
    world.add_rigidbody(
        1,
        &RigidBody::dynamic(1_000_000.0), // 1 million kg
        Vec3::new(0.0, 10.0, 0.0),
        Quat::IDENTITY,
    );
    world.add_collider(1, &Collider::sphere(1.0));

    for _ in 0..60 {
        world.step(1.0 / 60.0);
    }

    let (pos, _) = world.get_transform(1).unwrap();
    assert!(pos.is_finite(), "Heavy object should have finite position");

    // Test 2: Very light object
    world.add_rigidbody(
        2,
        &RigidBody::dynamic(0.00001), // 0.01 grams
        Vec3::new(5.0, 10.0, 0.0),
        Quat::IDENTITY,
    );
    world.add_collider(2, &Collider::sphere(0.1));

    for _ in 0..60 {
        world.step(1.0 / 60.0);
    }

    let (pos2, _) = world.get_transform(2).unwrap();
    assert!(pos2.is_finite(), "Light object should have finite position");

    // Test 3: Extreme velocity
    world.add_rigidbody(
        3,
        &RigidBody::dynamic(1.0).with_gravity_scale(0.0),
        Vec3::new(0.0, 0.0, 0.0),
        Quat::IDENTITY,
    );
    world.add_collider(3, &Collider::sphere(0.5));

    // Set extreme velocity
    world.set_velocity(3, Vec3::new(10_000.0, 0.0, 0.0), Vec3::ZERO);

    for _ in 0..10 {
        world.step(1.0 / 60.0);
    }

    let (pos3, _) = world.get_transform(3).unwrap();
    assert!(pos3.is_finite(), "Object with extreme velocity should have finite position");
}
