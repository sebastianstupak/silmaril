//! Demonstration of Phase 1.3 serialization features
//!
//! Run with: cargo run --example serialization_demo

use engine_core::serialization::{Format, Serializable, WorldState, WorldStateDelta};
use engine_core::{Health, MeshRenderer, Transform, Velocity, World};

fn main() {
    println!("=== Phase 1.3 Serialization Demo ===\n");

    // Create a world with components
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Health>();
    world.register::<Velocity>();
    world.register::<MeshRenderer>();

    // Spawn some entities
    for i in 0..5 {
        let entity = world.spawn();
        world.add(entity, Transform::default());
        world.add(entity, Health::new(100.0 - (i as f32 * 10.0), 100.0));

        if i % 2 == 0 {
            world.add(entity, Velocity::new(1.0, 0.0, 0.0));
        }

        if i % 3 == 0 {
            world.add(entity, MeshRenderer::new(i as u64, i as u64 + 100));
        }
    }

    println!("Created world with {} entities", world.entity_count());

    // Demo 1: YAML Serialization (human-readable)
    println!("\n--- Demo 1: YAML Serialization ---");
    let state1 = WorldState::new();
    match Serializable::serialize(&state1, Format::Yaml) {
        Ok(yaml_bytes) => {
            let yaml_string = String::from_utf8(yaml_bytes).unwrap();
            println!("YAML output:\n{}", yaml_string);
            println!("YAML size: {} bytes", yaml_string.len());
        }
        Err(e) => println!("Error: {}", e),
    }

    // Demo 2: Bincode Serialization (fast binary)
    println!("\n--- Demo 2: Bincode Serialization ---");
    let state2 = WorldState::new();
    match Serializable::serialize(&state2, Format::Bincode) {
        Ok(bytes) => {
            println!("Bincode size: {} bytes", bytes.len());

            // Roundtrip test
            match <WorldState as Serializable>::deserialize(&bytes, Format::Bincode) {
                Ok(restored) => {
                    println!("✓ Roundtrip successful!");
                    println!(
                        "  Original version: {}, Restored version: {}",
                        state2.metadata.version, restored.metadata.version
                    );
                }
                Err(e) => println!("✗ Deserialization failed: {}", e),
            }
        }
        Err(e) => println!("Error: {}", e),
    }

    // Demo 3: Delta Compression
    println!("\n--- Demo 3: Delta Compression ---");
    let old_state = WorldState::new();
    let mut new_state = WorldState::new();
    new_state.metadata.version = 2;

    let delta = WorldStateDelta::compute(&old_state, &new_state);
    println!("Delta computed:");
    println!("  Base version: {}", delta.base_version);
    println!("  Target version: {}", delta.target_version);
    println!("  Added entities: {}", delta.added_entities.len());
    println!("  Removed entities: {}", delta.removed_entities.len());

    if delta.is_smaller_than(&new_state) {
        println!("✓ Delta is smaller than full state (use delta)");
    } else {
        println!("✗ Delta is larger (use full state)");
    }

    // Apply delta
    let mut base = old_state.clone();
    delta.apply(&mut base);
    println!(
        "✓ Delta applied: version {} -> {}",
        old_state.metadata.version, base.metadata.version
    );

    // Demo 4: Component Tests
    println!("\n--- Demo 4: Component Features ---");

    // Health component
    let mut health = Health::new(100.0, 100.0);
    println!("Health: {}/{}", health.current, health.max);
    health.damage(30.0);
    println!("After damage(30): {}/{}", health.current, health.max);
    println!("Is alive? {}", health.is_alive());

    health.heal(50.0);
    println!("After heal(50): {}/{}", health.current, health.max);
    println!("Is full? {}", health.is_full());

    println!("\n=== Demo Complete ===");
}
