//! Example: Physics-ECS Event Integration
//!
//! Demonstrates how to use the physics event system with ECS.
//!
//! This example shows:
//! 1. Setting up physics with ECS synchronization
//! 2. Sending physics events to ECS
//! 3. Multiple systems reacting to the same events
//! 4. Batch-optimized transform synchronization

use engine_core::ecs::{Entity, EntityAllocator, World};
use engine_math::{Quat, Vec3};
use engine_physics::events::*;
use engine_physics::sync::*;
use engine_physics::{Collider, PhysicsConfig, PhysicsWorld, RigidBody};

fn main() {
    println!("=== Physics-ECS Event Integration Example ===\n");

    // Setup
    let mut ecs_world = World::new();
    let mut physics = PhysicsWorld::new(PhysicsConfig::default());
    let mut sync = PhysicsSyncSystem::default();

    // Create entities with physics
    println!("Creating entities with physics...");

    // Ground (static)
    let ground_id = 0u64;
    physics.add_rigidbody(
        ground_id,
        &RigidBody::static_body(),
        Vec3::new(0.0, -1.0, 0.0),
        Quat::IDENTITY,
    );
    physics.add_collider(ground_id, &Collider::box_collider(Vec3::new(10.0, 0.5, 10.0)));

    // Falling box (dynamic)
    let box_id = 1u64;
    let mut allocator = EntityAllocator::new();
    let box_entity = allocator.allocate();
    physics.add_rigidbody(
        box_id,
        &RigidBody::dynamic(1.0),
        Vec3::new(0.0, 10.0, 0.0),
        Quat::IDENTITY,
    );
    physics.add_collider(box_id, &Collider::box_collider(Vec3::ONE));

    // Register entity mapping for sync
    sync.register_entity(box_id, box_entity);

    println!("  Ground: entity_id={}", ground_id);
    println!("  Box:    entity_id={}, ecs_entity={:?}\n", box_id, box_entity);

    // Create event readers for different systems
    let mut collision_reader = ecs_world.get_event_reader::<CollisionStartEvent>();
    let mut force_reader = ecs_world.get_event_reader::<ContactForceEvent>();

    println!("Simulating physics...\n");

    // Simulate for 120 frames (2 seconds at 60 FPS)
    for frame in 0..120 {
        // Step physics
        physics.step(1.0 / 60.0);

        // Sync to ECS (sends events, updates transforms)
        sync.sync_to_ecs(&physics, &mut ecs_world);

        // Audio System: React to collision events
        for event in ecs_world.read_events(&mut collision_reader) {
            println!(
                "[Frame {}] 🔊 Audio System: Collision between {} and {}",
                frame, event.entity_a, event.entity_b
            );
            println!("  → Playing impact sound at {:?}", event.contact_point);
        }

        // Damage System: React to high-force collisions
        for event in ecs_world.read_events(&mut force_reader) {
            if event.force_magnitude > 10.0 {
                println!(
                    "[Frame {}] 💥 Damage System: High-force contact! Force: {:.1}N",
                    frame, event.force_magnitude
                );
                println!(
                    "  → Applying damage to entities {} and {}",
                    event.entity_a, event.entity_b
                );
            }
        }

        // Print box state every 30 frames
        if frame % 30 == 0 {
            if let Some((pos, _)) = physics.get_transform(box_id) {
                println!("[Frame {}] Box position: y={:.2}", frame, pos.y);
            }
        }
    }

    println!("\n=== Simulation Complete ===");
    println!("\nKey Takeaways:");
    println!("✅ Physics events automatically sent to ECS");
    println!("✅ Multiple systems reacted to same events");
    println!("✅ Batch-optimized transform synchronization");
    println!("✅ Zero tight coupling between physics and gameplay");
}
