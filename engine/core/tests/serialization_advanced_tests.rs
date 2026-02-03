//! Advanced serialization tests for AAA-quality validation
//!
//! This test suite covers edge cases, error handling, and production scenarios
//! that go beyond basic functionality testing.

use engine_core::ecs::World;
use engine_core::gameplay::Health;
use engine_core::math::Transform;
use engine_core::physics_components::Velocity;
use engine_core::rendering::MeshRenderer;
use engine_core::serialization::{Format, Serializable, WorldState, WorldStateDelta};
use std::sync::Arc;
use std::thread;
use std::time::Instant;

// ====================
// CORRUPT DATA HANDLING
// ====================

#[test]
fn test_bincode_deserialize_empty_data() {
    let empty: Vec<u8> = Vec::new();
    let result = <WorldState as Serializable>::deserialize(&empty, Format::Bincode);
    assert!(result.is_err(), "Should fail to deserialize empty data");
}

#[test]
fn test_bincode_deserialize_invalid_data() {
    let garbage: Vec<u8> = vec![0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
    let result = <WorldState as Serializable>::deserialize(&garbage, Format::Bincode);
    assert!(result.is_err(), "Should fail to deserialize garbage data");
}

#[test]
fn test_yaml_deserialize_malformed() {
    let malformed = b"{ this is not valid yaml: [[[";
    let result = <WorldState as Serializable>::deserialize(malformed, Format::Yaml);
    assert!(result.is_err(), "Should fail to deserialize malformed YAML");
}

#[test]
fn test_yaml_deserialize_wrong_structure() {
    let wrong_structure = b"just_a_string: value\nanother: thing";
    let result = <WorldState as Serializable>::deserialize(wrong_structure, Format::Yaml);
    // Should fail or produce minimal valid state
    assert!(result.is_err() || result.unwrap().entities.is_empty());
}

#[test]
fn test_partial_bincode_data() {
    // Create valid state and truncate its serialization
    let mut world = World::new();
    world.register::<Health>();
    let e = world.spawn();
    world.add(e, Health::new(100.0, 100.0));

    let state = WorldState::snapshot(&world);
    let full_bytes = state.serialize(Format::Bincode).unwrap();

    // Truncate to half size
    let partial = &full_bytes[..full_bytes.len() / 2];
    let result = <WorldState as Serializable>::deserialize(partial, Format::Bincode);
    assert!(result.is_err(), "Should fail to deserialize partial data");
}

// ====================
// CONCURRENT ACCESS
// ====================

#[test]
fn test_concurrent_delta_computation() {
    let mut world = World::new();
    world.register::<Health>();

    for i in 0..500 {
        let e = world.spawn();
        world.add(e, Health::new(i as f32, 100.0));
    }

    let state1 = WorldState::snapshot(&world);

    // Modify half the entities
    let entities_to_modify: Vec<_> = world.entities().take(250).collect();
    for entity in entities_to_modify {
        if let Some(health) = world.get_mut::<Health>(entity) {
            health.current = 50.0;
        }
    }

    let state2 = WorldState::snapshot(&world);

    let state1 = Arc::new(state1);
    let state2 = Arc::new(state2);
    let mut handles = vec![];

    // Compute delta from multiple threads
    for _ in 0..4 {
        let s1 = Arc::clone(&state1);
        let s2 = Arc::clone(&state2);
        let handle = thread::spawn(move || WorldStateDelta::compute(&*s1, &*s2));
        handles.push(handle);
    }

    let deltas: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    // All deltas should be identical
    for delta in &deltas[1..] {
        assert_eq!(deltas[0].added_entities.len(), delta.added_entities.len());
        assert_eq!(deltas[0].removed_entities.len(), delta.removed_entities.len());
        assert_eq!(deltas[0].modified_components.len(), delta.modified_components.len());
    }
}

// ====================
// LARGE SCALE TESTS
// ====================

#[test]
fn test_serialize_100k_entities() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Health>();

    println!("Creating 100k entities...");
    let start = Instant::now();
    for i in 0..100_000 {
        let e = world.spawn();
        world.add(e, Transform::default());
        world.add(e, Health::new((i % 100) as f32, 100.0));
    }
    println!("Creation took: {:?}", start.elapsed());

    println!("Taking snapshot...");
    let start = Instant::now();
    let snapshot = WorldState::snapshot(&world);
    println!("Snapshot took: {:?}", start.elapsed());

    assert_eq!(snapshot.entities.len(), 100_000);

    println!("Serializing to bincode...");
    let start = Instant::now();
    let bytes = snapshot.serialize(Format::Bincode).unwrap();
    println!(
        "Serialization took: {:?}, size: {} MB",
        start.elapsed(),
        bytes.len() / 1_000_000
    );

    // Verify size is reasonable (< 20MB for 100k simple entities)
    assert!(bytes.len() < 20_000_000, "Bincode size: {} bytes", bytes.len());
}

#[test]
fn test_component_churn_stress() {
    // Simulates rapidly adding/removing components (common in game loops)
    let mut world = World::new();
    world.register::<Health>();
    world.register::<Velocity>();

    let mut entities = Vec::new();
    for _ in 0..1000 {
        entities.push(world.spawn());
    }

    // 100 iterations of add/remove churn
    for iteration in 0..100 {
        // Add components to half
        for (i, &entity) in entities.iter().enumerate() {
            if i % 2 == iteration % 2 {
                world.add(entity, Health::new(iteration as f32, 100.0));
                world.add(entity, Velocity::new(iteration as f32, 0.0, 0.0));
            }
        }

        // Snapshot
        let snapshot = WorldState::snapshot(&world);
        assert!(snapshot.entities.len() <= 1000);

        // Remove components from half
        for (i, &entity) in entities.iter().enumerate() {
            if i % 2 != iteration % 2 {
                world.remove::<Health>(entity);
                world.remove::<Velocity>(entity);
            }
        }
    }

    // Final snapshot should work
    let final_snapshot = WorldState::snapshot(&world);
    assert_eq!(final_snapshot.entities.len(), 1000);
}

// ====================
// ENTITY LIFECYCLE STRESS
// ====================

#[test]
fn test_spawn_despawn_churn() {
    // Tests that serialization handles rapid entity creation/destruction
    let mut world = World::new();
    world.register::<Health>();

    for _cycle in 0..100 {
        // Spawn 100 entities
        let mut entities = Vec::new();
        for i in 0..100 {
            let e = world.spawn();
            world.add(e, Health::new(i as f32, 100.0));
            entities.push(e);
        }

        // Take snapshot
        let snapshot = WorldState::snapshot(&world);
        assert_eq!(snapshot.entities.len(), 100);

        // Despawn all
        for entity in entities {
            world.despawn(entity);
        }

        // Snapshot should be empty
        let empty_snapshot = WorldState::snapshot(&world);
        assert_eq!(empty_snapshot.entities.len(), 0);
    }
}

#[test]
fn test_generation_wraparound_safety() {
    // Ensures serialization works even with high generation numbers
    let mut world = World::new();
    world.register::<Health>();

    // Create and destroy same entity slot many times
    for generation in 0..1000 {
        let e = world.spawn();
        world.add(e, Health::new(generation as f32, 100.0));

        let snapshot = WorldState::snapshot(&world);
        assert_eq!(snapshot.entities.len(), 1);

        // Verify entity in snapshot has correct generation
        assert!(snapshot.entities[0].entity.generation() >= generation);

        world.despawn(e);
    }
}

// ====================
// DELTA COMPRESSION EDGE CASES
// ====================

#[test]
fn test_delta_with_duplicate_changes() {
    // Tests that applying the same delta multiple times is idempotent
    let mut world = World::new();
    world.register::<Health>();

    for _ in 0..50 {
        let e = world.spawn();
        world.add(e, Health::new(100.0, 100.0));
    }

    let state1 = WorldState::snapshot(&world);

    // Modify entities
    let entities_to_modify: Vec<_> = world.entities().take(25).collect();
    for entity in entities_to_modify {
        if let Some(health) = world.get_mut::<Health>(entity) {
            health.current = 50.0;
        }
    }

    let state2 = WorldState::snapshot(&world);
    let delta = WorldStateDelta::compute(&state1, &state2);

    // Apply delta multiple times
    let mut result = state1.clone();
    delta.apply(&mut result);
    let first_apply = result.clone();

    delta.apply(&mut result);
    let second_apply = result.clone();

    delta.apply(&mut result);
    let third_apply = result.clone();

    // All should be identical
    assert_eq!(first_apply.entities.len(), second_apply.entities.len());
    assert_eq!(second_apply.entities.len(), third_apply.entities.len());
    assert_eq!(first_apply.metadata.component_count, third_apply.metadata.component_count);
}

#[test]
fn test_delta_chain_correctness() {
    // Tests that chaining deltas produces correct result
    let mut world = World::new();
    world.register::<Health>();

    for _ in 0..100 {
        let e = world.spawn();
        world.add(e, Health::new(100.0, 100.0));
    }

    let state1 = WorldState::snapshot(&world);

    // First change
    let entities: Vec<_> = world.entities().take(30).collect();
    for entity in entities {
        if let Some(health) = world.get_mut::<Health>(entity) {
            health.current = 75.0;
        }
    }
    let state2 = WorldState::snapshot(&world);

    // Second change
    let entities: Vec<_> = world.entities().skip(30).take(30).collect();
    for entity in entities {
        if let Some(health) = world.get_mut::<Health>(entity) {
            health.current = 50.0;
        }
    }
    let state3 = WorldState::snapshot(&world);

    // Compute deltas
    let delta1_2 = WorldStateDelta::compute(&state1, &state2);
    let delta2_3 = WorldStateDelta::compute(&state2, &state3);

    // Apply chain
    let mut result = state1.clone();
    delta1_2.apply(&mut result);
    delta2_3.apply(&mut result);

    // Should match state3
    assert_eq!(result.entities.len(), state3.entities.len());
    assert_eq!(result.metadata.component_count, state3.metadata.component_count);
}

// ====================
// PERFORMANCE REGRESSION DETECTION
// ====================

#[test]
fn test_serialization_performance_regression() {
    // Ensures serialization doesn't get slower over time
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Health>();
    world.register::<Velocity>();
    world.register::<MeshRenderer>();

    for i in 0..5000 {
        let e = world.spawn();
        world.add(e, Transform::default());
        world.add(e, Health::new(i as f32, 100.0));
        world.add(e, Velocity::new(i as f32, 0.0, 0.0));
        world.add(e, MeshRenderer::new(i as u64));
    }

    let snapshot = WorldState::snapshot(&world);

    // Measure serialization time (debug build)
    let start = Instant::now();
    let _ = snapshot.serialize(Format::Bincode).unwrap();
    let serialize_time = start.elapsed();

    // Should complete in reasonable time even in debug build
    // 5000 entities with 4 components = 20k components
    println!("Serialization time (5k entities, 4 components each): {:?}", serialize_time);
    assert!(serialize_time.as_millis() < 500, "Serialization too slow: {:?}", serialize_time);
}

#[test]
fn test_delta_computation_performance() {
    let mut world = World::new();
    world.register::<Health>();

    for _ in 0..5000 {
        let e = world.spawn();
        world.add(e, Health::new(100.0, 100.0));
    }

    let state1 = WorldState::snapshot(&world);

    // Modify 10% of entities
    let entities_to_modify: Vec<_> = world.entities().take(500).collect();
    for entity in entities_to_modify {
        if let Some(health) = world.get_mut::<Health>(entity) {
            health.current = 50.0;
        }
    }

    let state2 = WorldState::snapshot(&world);

    let start = Instant::now();
    let _ = WorldStateDelta::compute(&state1, &state2);
    let delta_time = start.elapsed();

    println!("Delta computation time (5k entities, 10% modified): {:?}", delta_time);
    assert!(delta_time.as_millis() < 100, "Delta computation too slow: {:?}", delta_time);
}

// ====================
// COMPONENT TYPE SAFETY
// ====================

#[test]
fn test_mixed_component_types_serialization() {
    // Ensures all component types serialize/deserialize correctly
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Health>();
    world.register::<Velocity>();
    world.register::<MeshRenderer>();

    // Create entities with different component combinations
    let e1 = world.spawn();
    world.add(e1, Transform::default());

    let e2 = world.spawn();
    world.add(e2, Health::new(100.0, 100.0));

    let e3 = world.spawn();
    world.add(e3, Velocity::new(1.0, 2.0, 3.0));

    let e4 = world.spawn();
    world.add(e4, MeshRenderer::new(1));

    let e5 = world.spawn();
    world.add(e5, Transform::default());
    world.add(e5, Health::new(75.0, 100.0));
    world.add(e5, Velocity::new(5.0, 6.0, 7.0));
    world.add(e5, MeshRenderer::new(3));

    let snapshot = WorldState::snapshot(&world);
    let bytes = snapshot.serialize(Format::Bincode).unwrap();
    let restored = <WorldState as Serializable>::deserialize(&bytes, Format::Bincode).unwrap();

    let mut world2 = World::new();
    world2.register::<Transform>();
    world2.register::<Health>();
    world2.register::<Velocity>();
    world2.register::<MeshRenderer>();
    restored.restore(&mut world2);

    // Verify all entities and components restored
    assert!(world2.is_alive(e1));
    assert!(world2.has::<Transform>(e1));
    assert!(!world2.has::<Health>(e1));

    assert!(world2.is_alive(e5));
    assert!(world2.has::<Transform>(e5));
    assert!(world2.has::<Health>(e5));
    assert!(world2.has::<Velocity>(e5));
    assert!(world2.has::<MeshRenderer>(e5));
}

// ====================
// DETERMINISM VALIDATION
// ====================

#[test]
fn test_serialization_determinism() {
    // Multiple serializations of same state should produce identical bytes
    let mut world = World::new();
    world.register::<Health>();
    world.register::<Transform>();

    for i in 0..100 {
        let e = world.spawn();
        world.add(e, Health::new(i as f32, 100.0));
        world.add(e, Transform::default());
    }

    let snapshot = WorldState::snapshot(&world);

    // Serialize 10 times
    let serializations: Vec<_> =
        (0..10).map(|_| snapshot.serialize(Format::Bincode).unwrap()).collect();

    // All should be identical
    let first = &serializations[0];
    for bytes in &serializations[1..] {
        assert_eq!(first, bytes, "Serialization is not deterministic!");
    }
}

#[test]
fn test_delta_computation_determinism() {
    let mut world = World::new();
    world.register::<Health>();

    for i in 0..50 {
        let e = world.spawn();
        world.add(e, Health::new(i as f32, 100.0));
    }

    let state1 = WorldState::snapshot(&world);

    let entities_to_modify: Vec<_> = world.entities().take(25).collect();
    for entity in entities_to_modify {
        if let Some(health) = world.get_mut::<Health>(entity) {
            health.current = 50.0;
        }
    }

    let state2 = WorldState::snapshot(&world);

    // Compute delta 10 times
    let deltas: Vec<_> = (0..10).map(|_| WorldStateDelta::compute(&state1, &state2)).collect();

    // All should produce same results
    for delta in &deltas[1..] {
        assert_eq!(deltas[0].added_entities.len(), delta.added_entities.len());
        assert_eq!(deltas[0].removed_entities.len(), delta.removed_entities.len());
        assert_eq!(deltas[0].modified_components.len(), delta.modified_components.len());
    }
}

// ====================
// ENTITY ID REUSE PATTERNS
// ====================

#[test]
fn test_complex_entity_reuse_pattern() {
    // Tests serialization with complex patterns of entity creation/destruction
    let mut world = World::new();
    world.register::<Health>();

    let mut all_entities = Vec::new();

    // Create initial batch
    for i in 0..50 {
        let e = world.spawn();
        world.add(e, Health::new(i as f32, 100.0));
        all_entities.push(e);
    }

    // Remove every other entity
    for i in (0..50).step_by(2) {
        world.despawn(all_entities[i]);
    }

    let state1 = WorldState::snapshot(&world);
    assert_eq!(state1.entities.len(), 25);

    // Create new entities (will reuse freed IDs)
    for i in 0..30 {
        let e = world.spawn();
        world.add(e, Health::new((i + 100) as f32, 100.0));
    }

    let state2 = WorldState::snapshot(&world);
    assert_eq!(state2.entities.len(), 55); // 25 remaining + 30 new

    // Compute delta
    let delta = WorldStateDelta::compute(&state1, &state2);

    // Should have 30 added entities (new ones)
    assert_eq!(delta.added_entities.len(), 30);
    assert_eq!(delta.removed_entities.len(), 0);

    // Apply delta
    let mut state1_copy = state1.clone();
    delta.apply(&mut state1_copy);

    assert_eq!(state1_copy.entities.len(), 55);
}
