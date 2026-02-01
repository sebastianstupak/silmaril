//! Character Controller Demo
//!
//! Demonstrates character controller features:
//! 1. Basic movement (WASD simulation)
//! 2. Jumping and landing
//! 3. Ground detection
//! 4. Falling from platforms
//! 5. Multiple characters interacting
//!
//! Run with: cargo run --example character_demo

use engine_math::{Quat, Vec3};
use engine_physics::{CharacterController, Collider, PhysicsConfig, PhysicsWorld, RigidBody};

fn main() {
    println!("=== Character Controller Demo ===\n");

    // Create physics world
    let mut world = PhysicsWorld::new(PhysicsConfig::default());

    // Create ground plane
    let ground_id = 0;
    world.add_rigidbody(
        ground_id,
        &RigidBody::static_body(),
        Vec3::new(0.0, -0.5, 0.0),
        Quat::IDENTITY,
    );
    world.add_collider(ground_id, &Collider::box_collider(Vec3::new(20.0, 0.5, 20.0)));
    println!("Created ground plane at y=0");

    // Create elevated platform
    let platform_id = 1;
    world.add_rigidbody(
        platform_id,
        &RigidBody::static_body(),
        Vec3::new(10.0, 2.0, 0.0),
        Quat::IDENTITY,
    );
    world.add_collider(platform_id, &Collider::box_collider(Vec3::new(3.0, 0.5, 3.0)));
    println!("Created platform at x=10, y=2");

    // Create ramp/slope
    let ramp_id = 2;
    world.add_rigidbody(ramp_id, &RigidBody::static_body(), Vec3::new(5.0, 1.0, 0.0), Quat::IDENTITY);
    world.add_collider(ramp_id, &Collider::box_collider(Vec3::new(2.0, 0.2, 2.0)));
    println!("Created ramp at x=5, y=1");

    // Create main character
    let char_id = 10;
    world.add_rigidbody(char_id, &RigidBody::kinematic(), Vec3::new(0.0, 1.0, 0.0), Quat::IDENTITY);
    world.add_collider(char_id, &Collider::capsule(0.9, 0.4)); // 1.8m tall
    let mut character = CharacterController::new(5.0, 10.0);
    println!("Created character at origin (0, 1, 0)");

    // Create a second character (NPC)
    let npc_id = 11;
    world.add_rigidbody(npc_id, &RigidBody::kinematic(), Vec3::new(3.0, 1.0, 3.0), Quat::IDENTITY);
    world.add_collider(npc_id, &Collider::capsule(0.9, 0.4));
    let mut npc = CharacterController::new(3.0, 8.0); // Slower, lower jump
    println!("Created NPC at (3, 1, 3)");

    // Initialize physics
    world.step(1.0 / 60.0);

    println!("\n=== Simulation Start ===\n");

    // Scenario 1: Character walks forward and jumps
    println!("--- Scenario 1: Walking and Jumping ---");
    character.set_movement_input(Vec3::new(0.0, 0.0, 1.0));

    for frame in 0..60 {
        character.update(&mut world, char_id, 1.0 / 60.0);
        world.step(1.0 / 60.0);

        if frame == 30 {
            println!("Frame {}: Character attempts jump", frame);
            if character.jump() {
                println!("  ✓ Jump successful!");
            } else {
                println!("  ✗ Jump failed (not grounded)");
            }
        }

        if frame % 15 == 0 {
            let (pos, _) = world.get_transform(char_id).unwrap();
            println!(
                "Frame {}: pos=({:.2}, {:.2}, {:.2}), grounded={}, vel_y={:.2}",
                frame,
                pos.x,
                pos.y,
                pos.z,
                character.is_grounded(),
                character.vertical_velocity()
            );
        }
    }

    // Scenario 2: Character moves to platform
    println!("\n--- Scenario 2: Moving to Platform ---");
    character.set_movement_input(Vec3::new(1.0, 0.0, 0.0)); // Move right toward platform

    for frame in 0..120 {
        character.update(&mut world, char_id, 1.0 / 60.0);
        world.step(1.0 / 60.0);

        // Try to jump periodically
        if frame % 40 == 0 && character.is_grounded() {
            println!("Frame {}: Attempting jump to reach platform", frame);
            character.jump();
        }

        if frame % 30 == 0 {
            let (pos, _) = world.get_transform(char_id).unwrap();
            println!(
                "Frame {}: pos=({:.2}, {:.2}, {:.2}), grounded={}",
                frame, pos.x, pos.y, pos.z, character.is_grounded()
            );
        }
    }

    // Scenario 3: NPC walks in a circle
    println!("\n--- Scenario 3: NPC Circular Movement ---");

    for frame in 0..120 {
        // Calculate circular movement
        let angle = (frame as f32) * 0.05;
        let direction = Vec3::new(angle.cos(), 0.0, angle.sin());
        npc.set_movement_input(direction);

        npc.update(&mut world, npc_id, 1.0 / 60.0);
        world.step(1.0 / 60.0);

        if frame % 30 == 0 {
            let (pos, _) = world.get_transform(npc_id).unwrap();
            println!(
                "Frame {}: NPC pos=({:.2}, {:.2}, {:.2}), grounded={}",
                frame, pos.x, pos.y, pos.z, npc.is_grounded()
            );
        }
    }

    // Scenario 4: Character falls off platform
    println!("\n--- Scenario 4: Falling Test ---");

    // Teleport character to high position
    world.set_transform(char_id, Vec3::new(0.0, 10.0, 0.0), Quat::IDENTITY);
    world.step(1.0 / 60.0);

    character.set_movement_input(Vec3::ZERO); // No horizontal movement

    println!("Character teleported to height y=10");

    for frame in 0..180 {
        character.update(&mut world, char_id, 1.0 / 60.0);
        world.step(1.0 / 60.0);

        // Detect landing
        if !character.was_grounded() && character.is_grounded() {
            let (pos, _) = world.get_transform(char_id).unwrap();
            println!(
                "Frame {}: *** LANDED *** at y={:.2}, final velocity={:.2} m/s",
                frame,
                pos.y,
                character.vertical_velocity()
            );
        }

        if frame % 30 == 0 {
            let (pos, _) = world.get_transform(char_id).unwrap();
            println!(
                "Frame {}: y={:.2}, vel_y={:.2}, grounded={}",
                frame,
                pos.y,
                character.vertical_velocity(),
                character.is_grounded()
            );
        }
    }

    // Scenario 5: Rapid direction changes
    println!("\n--- Scenario 5: Responsive Controls ---");

    let directions = [
        Vec3::new(1.0, 0.0, 0.0),   // Right
        Vec3::new(0.0, 0.0, 1.0),   // Forward
        Vec3::new(-1.0, 0.0, 0.0),  // Left
        Vec3::new(0.0, 0.0, -1.0),  // Back
    ];

    for (i, direction) in directions.iter().enumerate() {
        println!("Direction {}: {:?}", i, direction);
        character.set_movement_input(*direction);

        for _ in 0..15 {
            character.update(&mut world, char_id, 1.0 / 60.0);
            world.step(1.0 / 60.0);
        }

        let (vel, _) = world.get_velocity(char_id).unwrap();
        println!("  Velocity: ({:.2}, {:.2}, {:.2})", vel.x, vel.y, vel.z);
    }

    // Final statistics
    println!("\n=== Simulation Complete ===");
    println!("\nFinal State:");

    let (char_pos, _) = world.get_transform(char_id).unwrap();
    let (npc_pos, _) = world.get_transform(npc_id).unwrap();

    println!("Character: pos=({:.2}, {:.2}, {:.2}), grounded={}",
             char_pos.x, char_pos.y, char_pos.z, character.is_grounded());
    println!("NPC:       pos=({:.2}, {:.2}, {:.2}), grounded={}",
             npc_pos.x, npc_pos.y, npc_pos.z, npc.is_grounded());

    println!("\nPhysics Stats:");
    println!("  Bodies: {}", world.body_count());
    println!("  Colliders: {}", world.collider_count());
    println!("  Frames: {}", world.frame_count());

    println!("\n✅ All scenarios completed successfully!");
}
