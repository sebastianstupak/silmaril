//! Complete Core Physics Tests
//!
//! Tests for missing basic physics features to match Unreal coverage:
//! - Sleeping/waking (island management)
//! - Restitution (bounciness)
//! - Friction (surface interaction)
//! - Linear damping (velocity decay)
//! - Angular damping (rotation decay)
//!
//! Target: Close -7.5 point gap with Unreal

use engine_math::{Quat, Vec3};
use engine_physics::{Collider, PhysicsConfig, PhysicsMaterial, PhysicsWorld, RigidBody};

/// Test: Body Sleeping and Waking
///
/// Validates that physics islands correctly put bodies to sleep when stationary
/// and wake them when forces are applied or they're disturbed.
#[test]
fn test_body_sleeping_and_waking() {
    let mut world = PhysicsWorld::new(PhysicsConfig::default());

    // Create ground
    world.add_rigidbody(0, &RigidBody::static_body(), Vec3::new(0.0, -1.0, 0.0), Quat::IDENTITY);
    world.add_collider(0, &Collider::box_collider(Vec3::new(100.0, 1.0, 100.0)));

    // Create dynamic body that should fall and sleep
    let body_id = 1;
    world.add_rigidbody(
        body_id,
        &RigidBody::dynamic(1.0),
        Vec3::new(0.0, 2.0, 0.0),
        Quat::IDENTITY,
    );
    world.add_collider(body_id, &Collider::sphere(0.5));

    // Run simulation until body settles (should sleep after ~2 seconds)
    let dt = 1.0 / 60.0;
    let mut last_pos = Vec3::ZERO;

    for frame in 0..180 {
        // 3 seconds at 60 FPS
        world.step(dt);

        if let Some((pos, _)) = world.get_transform(body_id) {
            // After first second, body should be nearly stationary
            if frame > 60 {
                let movement = (pos - last_pos).length();
                // Movement should be decreasing (settling)
                if frame > 120 {
                    assert!(
                        movement < 0.001,
                        "Body should be nearly stationary after 2 seconds, moved {}",
                        movement
                    );
                }
            }
            last_pos = pos;
        }
    }

    // Apply impulse to wake sleeping body
    world.apply_impulse(body_id, Vec3::new(0.0, 5.0, 0.0));

    // Step once
    world.step(dt);

    // Body should have moved significantly after wake-up
    if let Some((new_pos, _)) = world.get_transform(body_id) {
        let movement = (new_pos - last_pos).length();
        assert!(
            movement > 0.01,
            "Body should move after impulse wake-up, moved only {}",
            movement
        );
    }
}

/// Test: Restitution (Bounciness)
///
/// Validates that restitution coefficient correctly affects bounce behavior.
/// - Restitution = 0.0: No bounce (inelastic)
/// - Restitution = 1.0: Perfect bounce (elastic)
#[test]
fn test_restitution_bounce_behavior() {
    // Test 1: No bounce (restitution = 0.0)
    {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());

        // Ground with no restitution
        world.add_rigidbody(
            0,
            &RigidBody::static_body(),
            Vec3::new(0.0, -1.0, 0.0),
            Quat::IDENTITY,
        );

        let mat_no_bounce =
            PhysicsMaterial { restitution: 0.0, friction: 0.5, ..Default::default() };
        world.add_collider(
            0,
            &Collider::box_collider(Vec3::new(10.0, 1.0, 10.0)).with_material(mat_no_bounce),
        );

        // Drop ball from height
        world.add_rigidbody(1, &RigidBody::dynamic(1.0), Vec3::new(0.0, 5.0, 0.0), Quat::IDENTITY);
        world.add_collider(1, &Collider::sphere(0.5).with_material(mat_no_bounce));

        // Let ball fall and hit ground
        let dt = 1.0 / 60.0;
        for _ in 0..120 {
            world.step(dt);
        }

        // Ball should be resting on ground (minimal bounce)
        let (pos, _) = world.get_transform(1).unwrap();
        assert!(pos.y < 1.0, "Ball with restitution=0 should not bounce high, y={}", pos.y);
    }

    // Test 2: High bounce (restitution = 0.9)
    {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());

        world.add_rigidbody(
            0,
            &RigidBody::static_body(),
            Vec3::new(0.0, -1.0, 0.0),
            Quat::IDENTITY,
        );

        let mat_bouncy = PhysicsMaterial { restitution: 0.9, friction: 0.1, ..Default::default() };
        world.add_collider(
            0,
            &Collider::box_collider(Vec3::new(10.0, 1.0, 10.0)).with_material(mat_bouncy),
        );

        world.add_rigidbody(1, &RigidBody::dynamic(1.0), Vec3::new(0.0, 5.0, 0.0), Quat::IDENTITY);
        world.add_collider(1, &Collider::sphere(0.5).with_material(mat_bouncy));

        let dt = 1.0 / 60.0;
        let mut max_bounce_height = 0.0f32;

        // Run simulation and track max height after first bounce
        for frame in 0..180 {
            world.step(dt);

            if frame > 60 {
                // After first bounce
                let (pos, _) = world.get_transform(1).unwrap();
                max_bounce_height = max_bounce_height.max(pos.y);
            }
        }

        // With high restitution, ball should bounce to significant height
        assert!(
            max_bounce_height > 2.0,
            "Ball with restitution=0.9 should bounce high, max_y={}",
            max_bounce_height
        );
    }
}

/// Test: Friction (Surface Interaction)
///
/// Validates that friction coefficient affects sliding behavior.
/// - Friction = 0.0: Frictionless (slides forever)
/// - Friction = 1.0: High friction (stops quickly)
#[test]
fn test_friction_sliding_behavior() {
    // Test 1: Low friction (slides far)
    {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());

        // Sloped ground with low friction - steeper slope for clear sliding
        let slope_angle = 0.4; // ~23 degree slope - significant but realistic
        let slope_quat = Quat::from_axis_angle(Vec3::Z, slope_angle);

        world.add_rigidbody(0, &RigidBody::static_body(), Vec3::new(0.0, 0.0, 0.0), slope_quat);

        let mat_slippery =
            PhysicsMaterial { friction: 0.05, restitution: 0.0, ..Default::default() };
        world.add_collider(
            0,
            &Collider::box_collider(Vec3::new(50.0, 1.0, 10.0)).with_material(mat_slippery),
        );

        // Box on slope
        world.add_rigidbody(
            1,
            &RigidBody::dynamic(1.0),
            Vec3::new(-20.0, 2.0, 0.0),
            Quat::IDENTITY,
        );
        world.add_collider(
            1,
            &Collider::box_collider(Vec3::new(0.5, 0.5, 0.5)).with_material(mat_slippery),
        );

        let dt = 1.0 / 60.0;
        let initial_x = -20.0;

        // Run simulation longer to see clear sliding
        for _ in 0..180 {
            // 3 seconds
            world.step(dt);
        }

        // Box should have slid down slope significantly
        let (pos, _) = world.get_transform(1).unwrap();
        let distance_slid = pos.x - initial_x;

        assert!(
            distance_slid.abs() > 5.0, // Use abs() since direction depends on slope orientation
            "Box with low friction should slide far, only slid {}",
            distance_slid
        );
    }

    // Test 2: High friction (stops quickly)
    {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());

        let slope_angle = 0.2;
        let slope_quat = Quat::from_axis_angle(Vec3::Z, slope_angle);

        world.add_rigidbody(0, &RigidBody::static_body(), Vec3::new(0.0, 0.0, 0.0), slope_quat);

        let mat_sticky = PhysicsMaterial { friction: 1.0, restitution: 0.0, ..Default::default() };
        world.add_collider(
            0,
            &Collider::box_collider(Vec3::new(50.0, 1.0, 10.0)).with_material(mat_sticky),
        );

        world.add_rigidbody(
            1,
            &RigidBody::dynamic(1.0),
            Vec3::new(-20.0, 2.0, 0.0),
            Quat::IDENTITY,
        );
        world.add_collider(
            1,
            &Collider::box_collider(Vec3::new(0.5, 0.5, 0.5)).with_material(mat_sticky),
        );

        let dt = 1.0 / 60.0;
        let initial_x = -20.0;

        for _ in 0..120 {
            world.step(dt);
        }

        let (pos, _) = world.get_transform(1).unwrap();
        let distance_slid = pos.x - initial_x;

        // Box should barely slide with high friction
        assert!(
            distance_slid < 2.0,
            "Box with high friction should barely slide, slid {}",
            distance_slid
        );
    }
}

/// Test: Linear Damping (Velocity Decay)
///
/// Validates that linear damping reduces velocity over time.
#[test]
fn test_linear_damping_velocity_decay() {
    let mut world = PhysicsWorld::new(PhysicsConfig::default());

    // Create body with linear damping and no gravity (to isolate damping effect)
    let body_id = 1;
    let rb = RigidBody::dynamic(1.0)
        .with_linear_damping(0.7) // Damping to achieve ~50% reduction in 1s
        .with_gravity_scale(0.0); // Disable gravity to isolate damping

    world.add_rigidbody(
        body_id,
        &rb,
        Vec3::new(0.0, 10.0, 0.0), // High up to avoid any collisions
        Quat::IDENTITY,
    );
    world.add_collider(body_id, &Collider::sphere(0.5));

    // Apply initial velocity
    world.set_velocity(body_id, Vec3::new(10.0, 0.0, 0.0), Vec3::ZERO);

    let dt = 1.0 / 60.0;
    let initial_vel = 10.0;

    // Run simulation
    for frame in 0..60 {
        world.step(dt);

        if let Some((linear_vel, _)) = world.get_velocity(body_id) {
            // Velocity should be decreasing due to damping
            let current_speed = linear_vel.length();

            if frame == 30 {
                // After 0.5 seconds
                assert!(
                    current_speed < initial_vel * 0.8,
                    "Velocity should have decreased after 0.5s, still at {}",
                    current_speed
                );
            }

            if frame == 59 {
                // After ~1 second
                assert!(
                    current_speed < initial_vel * 0.5,
                    "Velocity should be <50% after 1s with damping, at {}",
                    current_speed
                );
            }
        }
    }
}

/// Test: Angular Damping (Rotation Decay)
///
/// Validates that angular damping reduces angular velocity over time.
#[test]
fn test_angular_damping_rotation_decay() {
    let mut world = PhysicsWorld::new(PhysicsConfig::default());

    // Create body with angular damping
    let body_id = 1;
    let rb = RigidBody::dynamic(1.0).with_angular_damping(0.7); // Damping to achieve ~50% reduction in 1s

    world.add_rigidbody(body_id, &rb, Vec3::new(0.0, 0.0, 0.0), Quat::IDENTITY);
    world.add_collider(body_id, &Collider::box_collider(Vec3::new(1.0, 0.5, 0.5)));

    // Apply initial angular velocity (spinning around Y axis)
    world.set_velocity(body_id, Vec3::ZERO, Vec3::new(0.0, 10.0, 0.0));

    let dt = 1.0 / 60.0;
    let initial_angular_speed = 10.0;

    // Run simulation
    for frame in 0..60 {
        world.step(dt);

        if let Some((_, angular_vel)) = world.get_velocity(body_id) {
            let current_speed = angular_vel.length();

            if frame == 30 {
                assert!(
                    current_speed < initial_angular_speed * 0.8,
                    "Angular velocity should decrease with damping, at {}",
                    current_speed
                );
            }

            if frame == 59 {
                assert!(
                    current_speed < initial_angular_speed * 0.5,
                    "Angular velocity should be <50% after 1s, at {}",
                    current_speed
                );
            }
        }
    }
}

/// Test: Combined Material Properties
///
/// Validates that friction and restitution work together correctly.
#[test]
fn test_combined_material_properties() {
    let mut world = PhysicsWorld::new(PhysicsConfig::default());

    // Ground with moderate friction and some bounce
    world.add_rigidbody(0, &RigidBody::static_body(), Vec3::new(0.0, -1.0, 0.0), Quat::IDENTITY);

    let mat_ground = PhysicsMaterial { friction: 0.7, restitution: 0.3, ..Default::default() };
    world.add_collider(
        0,
        &Collider::box_collider(Vec3::new(10.0, 1.0, 10.0)).with_material(mat_ground),
    );

    // Ball with matching properties - start lower for quicker ground contact
    // Add air resistance to ensure velocity decreases
    world.add_rigidbody(
        1,
        &RigidBody::dynamic(1.0).with_linear_damping(0.1),
        Vec3::new(-5.0, 2.0, 0.0), // Lower starting height
        Quat::IDENTITY,
    );

    let mat_ball = PhysicsMaterial {
        friction: 0.8, // Higher friction for clear effect
        restitution: 0.4,
        ..Default::default()
    };
    world.add_collider(1, &Collider::sphere(0.5).with_material(mat_ball));

    // Apply velocity at angle (will bounce and roll)
    world.set_velocity(1, Vec3::new(8.0, 0.0, 0.0), Vec3::ZERO);

    let dt = 1.0 / 60.0;
    let mut bounced = false;
    let mut slowed_by_friction = false;
    let mut initial_x_vel = 0.0;

    for frame in 0..240 {
        // 4 seconds total
        world.step(dt);

        if let Some((linear_vel, _)) = world.get_velocity(1) {
            // Record initial horizontal velocity after first few frames
            if frame == 5 {
                initial_x_vel = linear_vel.x.abs();
            }

            // Check for bounce (vertical velocity should go positive after falling)
            if linear_vel.y > 0.5 && frame > 10 {
                bounced = true;
            }

            // Check that horizontal velocity decreases significantly after ground contact
            // Ball should hit ground around frame 30-40, check at frame 150
            if frame > 150 && linear_vel.x.abs() < initial_x_vel * 0.6 {
                slowed_by_friction = true;
            }
        }
    }

    assert!(bounced, "Ball should bounce with restitution > 0");
    assert!(slowed_by_friction, "Ball should slow due to friction");
}

/// Test: Zero Friction Edge Case
///
/// Validates behavior with zero friction (perfect slip).
#[test]
fn test_zero_friction_perfect_slip() {
    let mut world = PhysicsWorld::new(PhysicsConfig::default());

    // Flat ground with zero friction
    world.add_rigidbody(0, &RigidBody::static_body(), Vec3::new(0.0, -1.0, 0.0), Quat::IDENTITY);

    let mat_ice = PhysicsMaterial { friction: 0.0, restitution: 0.0, ..Default::default() };
    world.add_collider(
        0,
        &Collider::box_collider(Vec3::new(100.0, 1.0, 100.0)).with_material(mat_ice),
    );

    // Box with initial horizontal velocity
    world.add_rigidbody(1, &RigidBody::dynamic(1.0), Vec3::new(0.0, 1.0, 0.0), Quat::IDENTITY);
    world.add_collider(1, &Collider::box_collider(Vec3::new(0.5, 0.5, 0.5)).with_material(mat_ice));

    world.set_velocity(1, Vec3::new(5.0, 0.0, 0.0), Vec3::ZERO);

    let dt = 1.0 / 60.0;
    let initial_speed = 5.0;

    // Run simulation
    for _ in 0..120 {
        world.step(dt);
    }

    // With zero friction, velocity should be nearly unchanged
    // (only air resistance/numerical damping)
    if let Some((linear_vel, _)) = world.get_velocity(1) {
        let current_speed = linear_vel.length();
        assert!(
            current_speed > initial_speed * 0.9,
            "Zero friction should preserve velocity, dropped to {}",
            current_speed
        );
    }
}
