//! Debug test to understand physics behavior

use engine_math::{Quat, Vec3};
use engine_physics::{Collider, PhysicsConfig, PhysicsWorld, RigidBody};

#[test]
fn debug_falling_box() {
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

    println!("=== Initial State ===");
    let (pos, _) = world.get_transform(box_id).unwrap();
    let (vel, _) = world.get_velocity(box_id).unwrap();
    println!("Position: {:?}", pos);
    println!("Velocity: {:?}", vel);
    println!("Body count: {}", world.body_count());
    println!("Collider count: {}", world.collider_count());

    // Simulate 200 frames to see full collision and settling
    for frame in 0..200 {
        world.step(1.0 / 60.0);

        let (pos, _) = world.get_transform(box_id).unwrap();
        let (vel, _) = world.get_velocity(box_id).unwrap();
        let collisions = world.collision_events().len();

        // Print every frame near collision
        if frame % 10 == 0 || collisions > 0 || vel.length() < 1.0 {
            println!("\n=== Frame {} ===", frame);
            println!("Position: y={:.3}", pos.y);
            println!("Velocity: {:.3}", vel.length());
            println!("Collisions: {}", collisions);
        }

        // Stop if settled
        if frame > 100 && vel.length() < 0.1 {
            println!("\n>>> Box settled at frame {}", frame);
            break;
        }
    }

    println!("\n=== Final State ===");
    let (pos, _) = world.get_transform(box_id).unwrap();
    let (vel, _) = world.get_velocity(box_id).unwrap();
    println!("Position: {:?}", pos);
    println!("Velocity length: {:.3}", vel.length());
    println!("Total collisions in last frame: {}", world.collision_events().len());
}
