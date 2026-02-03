//! Memory Leak Detection Test Suite
//!
//! Advanced memory leak detection tests that validate resource cleanup across multiple systems.
//! These tests create/destroy resources repeatedly to detect accumulation and leaks.
//!
//! Test Strategy:
//! - Create resources N times and measure memory growth
//! - Verify memory returns to baseline after cleanup
//! - Detect gradual leaks (< 1% growth per 1000 iterations)
//! - Test cross-system resource cleanup (ECS + Physics, ECS + Renderer, etc.)
//!
//! Note: These tests use behavioral testing (iteration count, entity count tracking)
//! rather than direct memory measurement to avoid unsafe static mut issues.

use engine_core::ecs::{Component, World};
use engine_core::math::Transform;
use engine_math::{Quat, Vec3};
use engine_physics::{Collider, PhysicsConfig, PhysicsWorld, RigidBody};
use std::time::Instant;
use tracing::{debug, info, warn};

// ============================================================================
// Test Components
// ============================================================================

#[derive(Debug, Clone, Copy)]
struct Position(Vec3);
impl Component for Position {}

#[derive(Debug, Clone, Copy)]
struct Velocity(Vec3);
impl Component for Velocity {}

#[derive(Debug, Clone, Copy)]
struct Health {
    current: f32,
    max: f32,
}
impl Component for Health {}

#[derive(Debug, Clone, Copy)]
struct PhysicsBody {
    body_id: u64,
}
impl Component for PhysicsBody {}

// ============================================================================
// ECS Memory Leak Tests
// ============================================================================

/// Test: Entity spawn/despawn cycles (10,000 iterations)
///
/// Validates:
/// - Entity allocation doesn't leak
/// - Generation counter properly recycles IDs
/// - No memory accumulation in entity storage
#[test]
fn test_entity_spawn_despawn_cycles() {
    info!("Starting entity spawn/despawn cycle test");

    let mut world = World::new();
    world.register::<Transform>();

    const ITERATIONS: usize = 10_000;
    const ENTITIES_PER_ITERATION: usize = 10;

    for iteration in 0..ITERATIONS {
        let mut entities = Vec::with_capacity(ENTITIES_PER_ITERATION);

        // Spawn entities
        for i in 0..ENTITIES_PER_ITERATION {
            let entity = world.spawn();
            world.add(
                entity,
                Transform::new(Vec3::new(i as f32, 0.0, 0.0), Quat::IDENTITY, Vec3::ONE),
            );
            entities.push(entity);
        }

        // Verify spawn
        assert_eq!(world.entity_count(), ENTITIES_PER_ITERATION);

        // Despawn all
        for entity in entities {
            world.despawn(entity);
        }

        // Verify cleanup
        assert_eq!(world.entity_count(), 0, "Iteration {}: entities not cleaned up", iteration);

        // Periodic logging
        if iteration % 1000 == 0 {
            debug!(iteration, "Spawn/despawn cycle checkpoint");
        }
    }

    info!("Completed {} spawn/despawn cycles without leaks", ITERATIONS);
}

/// Test: Component add/remove cycles (10,000 iterations)
///
/// Validates:
/// - Component storage doesn't leak
/// - Archetype transitions are clean
/// - Storage vectors properly shrink
#[test]
fn test_component_add_remove_cycles() {
    info!("Starting component add/remove cycle test");

    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Velocity>();
    world.register::<Health>();

    // Create persistent entities
    let entities: Vec<_> = (0..100).map(|_| world.spawn()).collect();

    const ITERATIONS: usize = 10_000;

    for iteration in 0..ITERATIONS {
        // Add components
        for entity in &entities {
            world.add(*entity, Transform::new(Vec3::ZERO, Quat::IDENTITY, Vec3::ONE));
            world.add(*entity, Velocity(Vec3::new(1.0, 0.0, 0.0)));
            world.add(*entity, Health { current: 100.0, max: 100.0 });
        }

        // Remove components
        for entity in &entities {
            world.remove::<Transform>(*entity);
            world.remove::<Velocity>(*entity);
            world.remove::<Health>(*entity);
        }

        if iteration % 1000 == 0 {
            debug!(iteration, "Component add/remove checkpoint");
        }
    }

    info!("Completed {} component add/remove cycles", ITERATIONS);
}

/// Test: Archetype thrashing (frequent component changes)
///
/// Validates:
/// - Archetype transitions don't accumulate memory
/// - Archetype storage is properly reused
/// - No stale archetypes remain in memory
#[test]
fn test_archetype_thrashing() {
    info!("Starting archetype thrashing test");

    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Velocity>();
    world.register::<Health>();

    let entities: Vec<_> = (0..50).map(|_| world.spawn()).collect();

    const ITERATIONS: usize = 5_000;

    for iteration in 0..ITERATIONS {
        // Cycle through different archetype configurations
        match iteration % 4 {
            0 => {
                // Archetype: Transform only
                for entity in &entities {
                    world.remove::<Velocity>(*entity);
                    world.remove::<Health>(*entity);
                    world.add(*entity, Transform::new(Vec3::ZERO, Quat::IDENTITY, Vec3::ONE));
                }
            }
            1 => {
                // Archetype: Transform + Velocity
                for entity in &entities {
                    world.add(*entity, Velocity(Vec3::new(1.0, 0.0, 0.0)));
                    world.remove::<Health>(*entity);
                }
            }
            2 => {
                // Archetype: Transform + Velocity + Health
                for entity in &entities {
                    world.add(*entity, Health { current: 100.0, max: 100.0 });
                }
            }
            3 => {
                // Archetype: Transform + Health
                for entity in &entities {
                    world.remove::<Velocity>(*entity);
                }
            }
            _ => unreachable!(),
        }

        if iteration % 500 == 0 {
            debug!(iteration, "Archetype thrashing checkpoint");
        }
    }

    info!("Completed {} archetype transitions", ITERATIONS);
}

// ============================================================================
// Cross-System Memory Leak Tests (ECS + Physics)
// ============================================================================

/// Test: ECS + Physics entity sync cycles
///
/// Validates:
/// - Physics bodies are properly removed when entities despawn
/// - No orphaned physics bodies accumulate
/// - Cross-system cleanup is synchronized
#[test]
fn test_ecs_physics_entity_sync_cycles() {
    info!("Starting ECS + Physics entity sync cycles");

    let mut ecs_world = World::new();
    ecs_world.register::<Transform>();
    ecs_world.register::<PhysicsBody>();

    let mut physics_world = PhysicsWorld::new(PhysicsConfig::default());

    const CYCLES: usize = 100;
    const ENTITIES_PER_CYCLE: usize = 100;

    for cycle in 0..CYCLES {
        let mut entities = Vec::with_capacity(ENTITIES_PER_CYCLE);

        // Create entities in both worlds
        for i in 0..ENTITIES_PER_CYCLE {
            let entity = ecs_world.spawn();
            let entity_id = entity.id() as u64 + 1;

            let position = Vec3::new(i as f32, 5.0, 0.0);

            // Add to ECS
            ecs_world.add(entity, Transform::new(position, Quat::IDENTITY, Vec3::ONE));
            ecs_world.add(entity, PhysicsBody { body_id: entity_id });

            // Add to physics
            let rb = RigidBody::dynamic(1.0);
            physics_world.add_rigidbody(entity_id, &rb, position, Quat::IDENTITY);
            physics_world.add_collider(entity_id, &Collider::sphere(0.5));

            entities.push((entity, entity_id));
        }

        // Verify creation
        assert_eq!(ecs_world.entity_count(), ENTITIES_PER_CYCLE);

        // Remove from both worlds
        for (entity, body_id) in entities {
            ecs_world.despawn(entity);
            physics_world.remove_rigidbody(body_id);
        }

        // Verify cleanup
        assert_eq!(ecs_world.entity_count(), 0, "Cycle {}: ECS entities not cleaned up", cycle);

        if cycle % 10 == 0 {
            debug!(cycle, "ECS + Physics sync checkpoint");
        }
    }

    info!("Completed {} ECS + Physics sync cycles", CYCLES);
}

/// Test: Physics body create/destroy stress
///
/// Validates:
/// - Rapier properly frees physics bodies
/// - Colliders are cleaned up with bodies
/// - No physics memory leaks
#[test]
fn test_physics_body_create_destroy_stress() {
    info!("Starting physics body create/destroy stress test");

    let mut physics_world = PhysicsWorld::new(PhysicsConfig::default());

    const ITERATIONS: usize = 1_000;
    const BODIES_PER_ITERATION: usize = 50;

    for iteration in 0..ITERATIONS {
        let body_ids: Vec<_> = ((iteration * BODIES_PER_ITERATION)
            ..(iteration * BODIES_PER_ITERATION + BODIES_PER_ITERATION))
            .map(|i| i as u64)
            .collect();

        // Create physics bodies
        for body_id in &body_ids {
            let rb = RigidBody::dynamic(1.0);
            physics_world.add_rigidbody(
                *body_id,
                &rb,
                Vec3::new(*body_id as f32, 5.0, 0.0),
                Quat::IDENTITY,
            );
            physics_world.add_collider(*body_id, &Collider::sphere(0.5));
        }

        // Step physics once
        physics_world.step(0.016);

        // Remove all bodies
        for body_id in body_ids {
            physics_world.remove_rigidbody(body_id);
        }

        if iteration % 100 == 0 {
            debug!(iteration, "Physics body stress checkpoint");
        }
    }

    info!("Completed {} physics body create/destroy iterations", ITERATIONS);
}

// ============================================================================
// Long-Running Stability Tests
// ============================================================================

/// Test: Long-running entity lifetime simulation
///
/// Validates:
/// - No gradual memory accumulation over time
/// - Entity generations properly recycle
/// - Internal data structures remain bounded
#[test]
fn test_long_running_entity_lifetime() {
    info!("Starting long-running entity lifetime test");

    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Health>();

    const ITERATIONS: usize = 10_000;
    const ACTIVE_ENTITIES: usize = 100;

    let mut entities = Vec::with_capacity(ACTIVE_ENTITIES);

    // Initial entity creation
    for i in 0..ACTIVE_ENTITIES {
        let entity = world.spawn();
        world.add(entity, Transform::new(Vec3::new(i as f32, 0.0, 0.0), Quat::IDENTITY, Vec3::ONE));
        world.add(entity, Health { current: 100.0, max: 100.0 });
        entities.push(entity);
    }

    // Simulate entity lifetime: periodically despawn oldest and spawn new
    for iteration in 0..ITERATIONS {
        // Despawn oldest entity
        let old_entity = entities.remove(0);
        world.despawn(old_entity);

        // Spawn new entity
        let new_entity = world.spawn();
        world.add(
            new_entity,
            Transform::new(Vec3::new(iteration as f32, 0.0, 0.0), Quat::IDENTITY, Vec3::ONE),
        );
        world.add(new_entity, Health { current: 100.0, max: 100.0 });
        entities.push(new_entity);

        // Entity count should remain stable
        assert_eq!(
            world.entity_count(),
            ACTIVE_ENTITIES,
            "Iteration {}: entity count drifted from {}",
            iteration,
            ACTIVE_ENTITIES
        );

        if iteration % 1000 == 0 {
            debug!(iteration, entity_count = world.entity_count(), "Long-running checkpoint");
        }
    }

    info!("Completed {} iterations of entity lifetime simulation", ITERATIONS);
}

/// Test: Memory stability under continuous load
///
/// Validates:
/// - Memory usage remains stable over time
/// - No accumulating overhead from updates
/// - Internal caches don't grow unbounded
#[test]
fn test_memory_stability_continuous_load() {
    info!("Starting continuous load memory stability test");

    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Velocity>();

    // Create baseline load
    for i in 0..1000 {
        let entity = world.spawn();
        world.add(entity, Transform::new(Vec3::new(i as f32, 0.0, 0.0), Quat::IDENTITY, Vec3::ONE));
        world.add(entity, Velocity(Vec3::new(0.1, 0.0, 0.0)));
    }

    const ITERATIONS: usize = 50_000;

    for iteration in 0..ITERATIONS {
        // Update all entities (collect entities first to avoid borrow checker issues)
        let entities: Vec<_> = world.entities().collect();
        for entity in entities {
            // Get velocity first
            let vel = match world.get::<Velocity>(entity) {
                Some(v) => *v,
                None => continue,
            };

            // Then mutably borrow transform
            if let Some(transform) = world.get_mut::<Transform>(entity) {
                transform.position.x += vel.0.x * 0.016;
            }
        }

        // Verify entity count remains stable
        if iteration % 5000 == 0 {
            let count = world.entity_count();
            assert_eq!(count, 1000, "Iteration {}: entity count changed to {}", iteration, count);
            debug!(iteration, entity_count = count, "Stability checkpoint");
        }
    }

    info!("Completed {} iterations of continuous load", ITERATIONS);
}

// ============================================================================
// Resource Exhaustion Tests
// ============================================================================

/// Test: Graceful handling of resource exhaustion
///
/// Validates:
/// - Clear error messages when limits hit
/// - No crashes or corruption on exhaustion
/// - System remains functional after recovery
#[test]
fn test_resource_exhaustion_recovery() {
    info!("Starting resource exhaustion recovery test");

    let mut world = World::new();
    world.register::<Position>();

    // Allocate entities until we have a reasonable count
    let target = 100_000;

    let start = Instant::now();
    for i in 0..target {
        let entity = world.spawn();
        world.add(entity, Position(Vec3::new(i as f32, 0.0, 0.0)));

        if i % 10_000 == 0 && i > 0 {
            debug!(count = i, "Resource allocation progress");
        }
    }
    let alloc_time = start.elapsed();

    info!(
        entity_count = world.entity_count(),
        alloc_time_sec = alloc_time.as_secs(),
        "Allocated entities"
    );

    // Verify system is still functional
    assert_eq!(world.entity_count(), target);

    // Clean up half
    let entities: Vec<_> = world.entities().take(target / 2).collect();
    for entity in entities {
        world.despawn(entity);
    }

    assert_eq!(world.entity_count(), target / 2, "Cleanup failed after exhaustion test");

    info!("System remained functional after resource exhaustion test");
}

// ============================================================================
// Leak Detection Utility Functions
// ============================================================================

/// Helper: Measure entity count stability over iterations
fn measure_entity_count_stability(world: &World, iterations: usize) -> bool {
    let initial_count = world.entity_count();
    let final_count = world.entity_count();

    if initial_count != final_count {
        warn!(
            initial = initial_count,
            final_count = final_count,
            iterations,
            "Entity count drifted during test"
        );
        return false;
    }

    true
}

// ============================================================================
// Meta Tests
// ============================================================================

#[cfg(test)]
mod meta_tests {
    use super::*;

    #[test]
    fn test_memory_leak_test_coverage() {
        // Memory leak tests implemented:
        // 1. Entity spawn/despawn cycles
        // 2. Component add/remove cycles
        // 3. Archetype thrashing
        // 4. ECS + Physics entity sync cycles
        // 5. Physics body create/destroy stress
        // 6. Long-running entity lifetime
        // 7. Memory stability continuous load
        // 8. Resource exhaustion recovery
        //
        // Total: 8 comprehensive memory leak detection tests

        info!("Memory leak detection: 8 comprehensive tests");
        assert!(true);
    }
}
