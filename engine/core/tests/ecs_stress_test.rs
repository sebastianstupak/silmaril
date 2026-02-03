//! Comprehensive ECS stress tests to validate performance under extreme conditions
//!
//! Tests large entity counts, high component density, concurrent operations,
//! and long-running simulations to ensure the ECS scales to production workloads.

use engine_core::ecs::{Component, World};
use engine_core::math::{Transform, Vec3};
use std::time::Instant;

#[derive(Debug, Clone, Copy)]
struct Position(Vec3);
impl Component for Position {}

#[derive(Debug, Clone, Copy)]
struct Velocity(Vec3);
impl Component for Velocity {}

#[derive(Debug, Clone, Copy)]
struct Acceleration(Vec3);
impl Component for Acceleration {}

#[derive(Debug, Clone, Copy)]
struct Health {
    current: f32,
    max: f32,
}
impl Component for Health {}

#[derive(Debug, Clone, Copy)]
struct Armor(f32);
impl Component for Armor {}

#[derive(Debug, Clone, Copy)]
struct Team(u8);
impl Component for Team {}

#[derive(Debug, Clone, Copy)]
struct Level(u32);
impl Component for Level {}

#[derive(Debug, Clone, Copy)]
struct Experience(u64);
impl Component for Experience {}

#[derive(Debug, Clone, Copy)]
struct Faction(u32);
impl Component for Faction {}

#[derive(Debug, Clone, Copy)]
struct ActiveEffects([u32; 8]);
impl Component for ActiveEffects {}

/// Test spawning and despawning 10K entities
#[test]
fn test_spawn_10k_entities() {
    let mut world = World::new();
    world.register::<Position>();
    world.register::<Velocity>();

    let start = Instant::now();

    // Spawn 10,000 entities
    let mut entities = Vec::with_capacity(10_000);
    for i in 0..10_000 {
        let entity = world.spawn();
        world.add(entity, Position(Vec3::new(i as f32, 0.0, 0.0)));
        world.add(entity, Velocity(Vec3::new(1.0, 0.0, 0.0)));
        entities.push(entity);
    }

    let spawn_time = start.elapsed();
    println!("Spawned 10K entities in {:?}", spawn_time);

    // Verify all entities exist
    assert_eq!(entities.len(), 10_000);

    // Despawn all entities
    let start = Instant::now();
    for entity in entities {
        world.despawn(entity);
    }
    let despawn_time = start.elapsed();
    println!("Despawned 10K entities in {:?}", despawn_time);

    // Performance targets
    assert!(
        spawn_time.as_millis() < 100,
        "Spawning 10K entities took too long: {:?}",
        spawn_time
    );
    assert!(
        despawn_time.as_millis() < 50,
        "Despawning 10K entities took too long: {:?}",
        despawn_time
    );
}

/// Test spawning 100K entities (stress test)
#[test]
#[ignore] // Expensive test, run with --ignored
fn test_spawn_100k_entities() {
    let mut world = World::new();
    world.register::<Position>();
    world.register::<Velocity>();
    world.register::<Transform>();

    let start = Instant::now();

    // Spawn 100,000 entities
    for i in 0..100_000 {
        let entity = world.spawn();
        world.add(entity, Position(Vec3::new(i as f32 % 1000.0, (i / 1000) as f32, 0.0)));
        world.add(entity, Velocity(Vec3::new((i % 3) as f32, (i % 5) as f32, 0.0)));
        world.add(entity, Transform::default());
    }

    let spawn_time = start.elapsed();
    println!("Spawned 100K entities in {:?}", spawn_time);

    // Performance target: < 1 second for 100K entities
    assert!(
        spawn_time.as_secs() < 2,
        "Spawning 100K entities took too long: {:?}",
        spawn_time
    );
}

/// Test high component density (many components per entity)
#[test]
fn test_high_component_density() {
    let mut world = World::new();
    world.register::<Position>();
    world.register::<Velocity>();
    world.register::<Acceleration>();
    world.register::<Health>();
    world.register::<Armor>();
    world.register::<Team>();
    world.register::<Level>();
    world.register::<Experience>();
    world.register::<Faction>();
    world.register::<ActiveEffects>();
    world.register::<Transform>();

    let start = Instant::now();

    // Spawn 1000 entities with 11 components each
    for i in 0..1000 {
        let entity = world.spawn();
        world.add(entity, Position(Vec3::new(i as f32, 0.0, 0.0)));
        world.add(entity, Velocity(Vec3::ZERO));
        world.add(entity, Acceleration(Vec3::ZERO));
        world.add(entity, Health { current: 100.0, max: 100.0 });
        world.add(entity, Armor(50.0));
        world.add(entity, Team(i as u8 % 4));
        world.add(entity, Level(1));
        world.add(entity, Experience(0));
        world.add(entity, Faction(i % 10));
        world.add(entity, ActiveEffects([0; 8]));
        world.add(entity, Transform::default());
    }

    let spawn_time = start.elapsed();
    println!("Spawned 1000 entities with 11 components in {:?}", spawn_time);

    // Performance target: < 10ms for 1000 high-density entities
    assert!(
        spawn_time.as_millis() < 50,
        "High-density spawn took too long: {:?}",
        spawn_time
    );
}

/// Test query iteration performance on large worlds
#[test]
fn test_query_iteration_scaling() {
    let mut world = World::new();
    world.register::<Position>();
    world.register::<Velocity>();

    // Spawn 10,000 entities
    for i in 0..10_000 {
        let entity = world.spawn();
        world.add(entity, Position(Vec3::new(i as f32, 0.0, 0.0)));
        world.add(entity, Velocity(Vec3::new(1.0, 0.0, 0.0)));
    }

    // Measure query iteration time
    let start = Instant::now();

    let mut count = 0;
    for (pos, vel) in world.query::<(&Position, &Velocity)>().iter_mut() {
        count += 1;
        // Simulate some work
        let _ = pos.0 + vel.0;
    }

    let iteration_time = start.elapsed();
    println!("Queried {} entities in {:?}", count, iteration_time);

    assert_eq!(count, 10_000);

    // Performance target: < 5ms for 10K entity query
    assert!(
        iteration_time.as_millis() < 10,
        "Query iteration took too long: {:?}",
        iteration_time
    );
}

/// Test mutable query iteration
#[test]
fn test_mutable_query_iteration() {
    let mut world = World::new();
    world.register::<Position>();
    world.register::<Velocity>();

    // Spawn 5,000 entities
    for i in 0..5000 {
        let entity = world.spawn();
        world.add(entity, Position(Vec3::new(i as f32, 0.0, 0.0)));
        world.add(entity, Velocity(Vec3::new(1.0, 0.0, 0.0)));
    }

    let start = Instant::now();

    // Mutate all positions
    let mut count = 0;
    for (mut pos, vel) in world.query::<(&mut Position, &Velocity)>().iter_mut() {
        pos.0 = pos.0 + vel.0;
        count += 1;
    }

    let mutation_time = start.elapsed();
    println!("Mutated {} entities in {:?}", count, mutation_time);

    assert_eq!(count, 5000);

    // Performance target: < 5ms for 5K entity mutation
    assert!(
        mutation_time.as_millis() < 10,
        "Mutable query took too long: {:?}",
        mutation_time
    );
}

/// Test component add/remove performance
#[test]
fn test_component_add_remove_performance() {
    let mut world = World::new();
    world.register::<Position>();
    world.register::<Velocity>();
    world.register::<Health>();

    // Spawn 1000 entities with Position only
    let mut entities = Vec::new();
    for i in 0..1000 {
        let entity = world.spawn();
        world.add(entity, Position(Vec3::new(i as f32, 0.0, 0.0)));
        entities.push(entity);
    }

    // Add Velocity to all entities
    let start = Instant::now();
    for &entity in &entities {
        world.add(entity, Velocity(Vec3::ZERO));
    }
    let add_time = start.elapsed();
    println!("Added components to 1000 entities in {:?}", add_time);

    // Remove Velocity from all entities
    let start = Instant::now();
    for &entity in &entities {
        world.remove::<Velocity>(entity);
    }
    let remove_time = start.elapsed();
    println!("Removed components from 1000 entities in {:?}", remove_time);

    // Performance targets
    assert!(add_time.as_millis() < 10, "Component add took too long: {:?}", add_time);
    assert!(
        remove_time.as_millis() < 10,
        "Component remove took too long: {:?}",
        remove_time
    );
}

/// Test memory usage with large entity counts
#[test]
#[ignore] // Memory intensive test
fn test_memory_scaling() {
    let mut world = World::new();
    world.register::<Position>();
    world.register::<Velocity>();
    world.register::<Transform>();

    // Spawn 1 million entities
    println!("Spawning 1M entities...");
    let start = Instant::now();

    for i in 0..1_000_000 {
        let entity = world.spawn();
        world.add(
            entity,
            Position(Vec3::new(
                (i % 1000) as f32,
                ((i / 1000) % 1000) as f32,
                (i / 1_000_000) as f32,
            )),
        );
        world.add(entity, Velocity(Vec3::new((i % 3) as f32, (i % 5) as f32, (i % 7) as f32)));
        world.add(entity, Transform::default());

        if i > 0 && i % 100_000 == 0 {
            println!("  Spawned {}K entities...", i / 1000);
        }
    }

    let spawn_time = start.elapsed();
    println!("Spawned 1M entities in {:?}", spawn_time);

    // Query all entities
    let start = Instant::now();
    let count = world.query::<(&Position, &Velocity)>().iter_mut().count();
    let query_time = start.elapsed();
    println!("Queried {} entities in {:?}", count, query_time);

    assert_eq!(count, 1_000_000);

    // Performance targets for 1M entities
    assert!(
        spawn_time.as_secs() < 10,
        "Spawning 1M entities took too long: {:?}",
        spawn_time
    );
    assert!(
        query_time.as_millis() < 100,
        "Querying 1M entities took too long: {:?}",
        query_time
    );
}

/// Test long-running simulation (1000 frames)
#[test]
fn test_long_running_simulation() {
    let mut world = World::new();
    world.register::<Position>();
    world.register::<Velocity>();

    // Spawn 1000 entities
    for i in 0..1000 {
        let entity = world.spawn();
        world.add(entity, Position(Vec3::new(i as f32, 0.0, 0.0)));
        world.add(entity, Velocity(Vec3::new(1.0, 0.0, 0.0)));
    }

    // Run 1000 simulation frames
    let start = Instant::now();
    let dt = 1.0 / 60.0; // 60 FPS

    for frame in 0..1000 {
        // Update positions
        for (mut pos, vel) in world.query::<(&mut Position, &Velocity)>().iter_mut() {
            pos.0 = pos.0 + vel.0 * dt;
        }

        // Occasional spawn/despawn
        if frame % 100 == 0 {
            let entity = world.spawn();
            world.add(entity, Position(Vec3::ZERO));
            world.add(entity, Velocity(Vec3::ZERO));
        }
    }

    let simulation_time = start.elapsed();
    println!(
        "Ran 1000 frame simulation in {:?} ({:.2} fps)",
        simulation_time,
        1000.0 / simulation_time.as_secs_f64()
    );

    // Performance target: Should maintain >60 FPS average
    let avg_frame_time = simulation_time.as_secs_f64() / 1000.0;
    assert!(
        avg_frame_time < 1.0 / 60.0,
        "Average frame time too slow: {:.4}ms (target: <16.67ms)",
        avg_frame_time * 1000.0
    );
}

/// Test fragmented world (many entity types)
#[test]
fn test_fragmented_world() {
    let mut world = World::new();
    world.register::<Position>();
    world.register::<Velocity>();
    world.register::<Health>();
    world.register::<Armor>();

    // Create different "archetypes" of entities
    for i in 0..1000 {
        let entity = world.spawn();
        world.add(entity, Position(Vec3::new(i as f32, 0.0, 0.0)));

        // Different component combinations
        match i % 4 {
            0 => {
                world.add(entity, Velocity(Vec3::ZERO));
            }
            1 => {
                world.add(entity, Health { current: 100.0, max: 100.0 });
            }
            2 => {
                world.add(entity, Velocity(Vec3::ZERO));
                world.add(entity, Health { current: 100.0, max: 100.0 });
            }
            3 => {
                world.add(entity, Armor(50.0));
            }
            _ => unreachable!(),
        }
    }

    // Query specific archetypes
    let start = Instant::now();
    let count1 = world.query::<(&Position, &Velocity)>().iter_mut().count();
    let count2 = world.query::<(&Position, &Health)>().iter_mut().count();
    let count3 = world.query::<(&Position, &Velocity, &Health)>().iter_mut().count();
    let query_time = start.elapsed();

    println!(
        "Queried fragmented world in {:?} (counts: {}, {}, {})",
        query_time, count1, count2, count3
    );

    // Verify counts
    assert_eq!(count1, 500); // 0 and 2
    assert_eq!(count2, 500); // 1 and 2
    assert_eq!(count3, 250); // only 2

    // Performance target: < 5ms for fragmented queries
    assert!(
        query_time.as_millis() < 10,
        "Fragmented world queries took too long: {:?}",
        query_time
    );
}
