// Change Detection Integration Tests
//
// This test suite validates that change detection filtering works correctly
// across all query iterator implementations (single component, tuples, macro-generated).

use engine_core::ecs::{Component, World};

#[derive(Debug, Clone, Copy, PartialEq)]
struct Position {
    x: f32,
    y: f32,
}
impl Component for Position {}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Velocity {
    x: f32,
    y: f32,
}
impl Component for Velocity {}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Health {
    current: i32,
    max: i32,
}
impl Component for Health {}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Marker;
impl Component for Marker {}

// Helper function to create a world with all component types registered
fn create_world() -> World {
    let mut world = World::new();
    world.register::<Position>();
    world.register::<Velocity>();
    world.register::<Health>();
    world.register::<Marker>();
    world
}

/// Test basic change detection with single component mutable query
#[test]
fn test_single_component_change_detection() {
    let mut world = create_world();

    // Spawn entities with Position
    let e1 = world.spawn();
    world.add(e1, Position { x: 1.0, y: 1.0 });

    let e2 = world.spawn();
    world.add(e2, Position { x: 2.0, y: 2.0 });

    let e3 = world.spawn();
    world.add(e3, Position { x: 3.0, y: 3.0 });

    // Increment tick so the initial adds are in the past
    world.increment_tick();

    // Capture the tick before modifications
    let tick_before = world.current_tick();

    // Modify only e1 and e2
    {
        let mut query = world.query_mut::<&mut Position>();

        // Modify e1
        if let Some((entity, pos)) = query.next() {
            assert_eq!(entity, e1);
            pos.x = 10.0;
        }

        // Modify e2
        if let Some((entity, pos)) = query.next() {
            assert_eq!(entity, e2);
            pos.x = 20.0;
        }

        // Skip e3
    }

    // Increment tick to simulate next frame
    world.increment_tick();

    // Query only entities where Position has changed since tick_before
    let changed_entities: Vec<_> = world
        .query_mut::<&mut Position>()
        .since_tick(tick_before)
        .changed::<Position>()
        .map(|(e, _)| e)
        .collect();

    // Only e1 and e2 should be in the results
    assert_eq!(changed_entities.len(), 2);
    assert!(changed_entities.contains(&e1));
    assert!(changed_entities.contains(&e2));
    assert!(!changed_entities.contains(&e3));
}

/// Test change detection with two-component tuple query (&mut A, &mut B)
#[test]
fn test_two_component_mut_change_detection() {
    let mut world = create_world();

    // Spawn entities with Position and Velocity
    let e1 = world.spawn();
    world.add(e1, Position { x: 1.0, y: 1.0 });
    world.add(e1, Velocity { x: 0.1, y: 0.1 });

    let e2 = world.spawn();
    world.add(e2, Position { x: 2.0, y: 2.0 });
    world.add(e2, Velocity { x: 0.2, y: 0.2 });

    let e3 = world.spawn();
    world.add(e3, Position { x: 3.0, y: 3.0 });
    world.add(e3, Velocity { x: 0.3, y: 0.3 });

    let tick_before = world.current_tick();

    // Modify only Velocity of e1
    {
        let mut query = world.query_mut::<&mut Velocity>();
        if let Some((entity, vel)) = query.next() {
            assert_eq!(entity, e1);
            vel.x = 10.0;
        }
    }

    world.increment_tick();

    // Query entities where Velocity changed
    let changed_entities: Vec<_> = world
        .query_mut::<(&mut Position, &mut Velocity)>()
        .since_tick(tick_before)
        .changed::<Velocity>()
        .map(|(e, _)| e)
        .collect();

    // Only e1 should be in the results
    assert_eq!(changed_entities.len(), 1);
    assert_eq!(changed_entities[0], e1);
}

/// Test change detection with mixed mutability query (&A, &mut B)
#[test]
fn test_mixed_mutability_change_detection() {
    let mut world = create_world();

    let e1 = world.spawn();
    world.add(e1, Position { x: 1.0, y: 1.0 });
    world.add(e1, Health { current: 100, max: 100 });

    let e2 = world.spawn();
    world.add(e2, Position { x: 2.0, y: 2.0 });
    world.add(e2, Health { current: 50, max: 100 });

    let tick_before = world.current_tick();

    // Modify Position of e1
    {
        let mut query = world.query_mut::<&mut Position>();
        if let Some((entity, pos)) = query.next() {
            assert_eq!(entity, e1);
            pos.x = 10.0;
        }
    }

    world.increment_tick();

    // Query with mixed mutability where Position changed
    let changed_entities: Vec<_> = world
        .query_mut::<(&Position, &mut Health)>()
        .since_tick(tick_before)
        .changed::<Position>()
        .map(|(e, _)| e)
        .collect();

    assert_eq!(changed_entities.len(), 1);
    assert_eq!(changed_entities[0], e1);
}

/// Test change detection with opposite mixed mutability (&mut A, &B)
#[test]
fn test_mixed_mutability_reverse_change_detection() {
    let mut world = create_world();

    let e1 = world.spawn();
    world.add(e1, Position { x: 1.0, y: 1.0 });
    world.add(e1, Health { current: 100, max: 100 });

    let e2 = world.spawn();
    world.add(e2, Position { x: 2.0, y: 2.0 });
    world.add(e2, Health { current: 50, max: 100 });

    let tick_before = world.current_tick();

    // Modify Health of e2
    {
        let mut query = world.query_mut::<&mut Health>();
        query.next(); // Skip e1
        if let Some((entity, health)) = query.next() {
            assert_eq!(entity, e2);
            health.current = 75;
        }
    }

    world.increment_tick();

    // Query with reverse mixed mutability where Health changed
    let changed_entities: Vec<_> = world
        .query_mut::<(&mut Position, &Health)>()
        .since_tick(tick_before)
        .changed::<Health>()
        .map(|(e, _)| e)
        .collect();

    assert_eq!(changed_entities.len(), 1);
    assert_eq!(changed_entities[0], e2);
}

/// Test that unchanged entities are properly filtered out
#[test]
fn test_unchanged_entities_filtered() {
    let mut world = create_world();

    let e1 = world.spawn();
    world.add(e1, Position { x: 1.0, y: 1.0 });

    let e2 = world.spawn();
    world.add(e2, Position { x: 2.0, y: 2.0 });

    let e3 = world.spawn();
    world.add(e3, Position { x: 3.0, y: 3.0 });

    let tick_before = world.current_tick();

    // Don't modify anything
    world.increment_tick();

    // Query for changes - should be empty
    let changed_entities: Vec<_> = world
        .query_mut::<&mut Position>()
        .since_tick(tick_before)
        .changed::<Position>()
        .map(|(e, _)| e)
        .collect();

    assert_eq!(changed_entities.len(), 0);
}

/// Test edge case: all entities changed
#[test]
fn test_all_entities_changed() {
    let mut world = create_world();

    let e1 = world.spawn();
    world.add(e1, Position { x: 1.0, y: 1.0 });

    let e2 = world.spawn();
    world.add(e2, Position { x: 2.0, y: 2.0 });

    let e3 = world.spawn();
    world.add(e3, Position { x: 3.0, y: 3.0 });

    let tick_before = world.current_tick();

    // Modify all entities
    for (_, pos) in world.query_mut::<&mut Position>() {
        pos.x += 10.0;
    }

    world.increment_tick();

    // Query for changes - should get all
    let changed_entities: Vec<_> = world
        .query_mut::<&mut Position>()
        .since_tick(tick_before)
        .changed::<Position>()
        .map(|(e, _)| e)
        .collect();

    assert_eq!(changed_entities.len(), 3);
    assert!(changed_entities.contains(&e1));
    assert!(changed_entities.contains(&e2));
    assert!(changed_entities.contains(&e3));
}

/// Test edge case: no entities have the component
#[test]
fn test_no_entities_with_component() {
    let mut world = create_world();

    let tick_before = world.current_tick();

    // Spawn entities without Position
    let _e1 = world.spawn();
    let _e2 = world.spawn();

    world.increment_tick();

    // Query for Position changes - should be empty
    let changed_entities: Vec<_> = world
        .query_mut::<&mut Position>()
        .since_tick(tick_before)
        .changed::<Position>()
        .map(|(e, _)| e)
        .collect();

    assert_eq!(changed_entities.len(), 0);
}

/// Test combining change detection with other filters
#[test]
fn test_change_detection_with_other_filters() {
    let mut world = create_world();

    let e1 = world.spawn();
    world.add(e1, Position { x: 1.0, y: 1.0 });
    world.add(e1, Marker);

    let e2 = world.spawn();
    world.add(e2, Position { x: 2.0, y: 2.0 });
    // e2 doesn't have Marker

    let e3 = world.spawn();
    world.add(e3, Position { x: 3.0, y: 3.0 });
    world.add(e3, Marker);

    let tick_before = world.current_tick();

    // Modify e1 and e2
    {
        let mut query = world.query_mut::<&mut Position>();

        if let Some((_, pos)) = query.next() {
            pos.x = 10.0; // e1
        }
        if let Some((_, pos)) = query.next() {
            pos.x = 20.0; // e2
        }
    }

    world.increment_tick();

    // Query for changed Position WITH Marker filter
    let changed_entities: Vec<_> = world
        .query_mut::<&mut Position>()
        .since_tick(tick_before)
        .with::<Marker>()
        .changed::<Position>()
        .map(|(e, _)| e)
        .collect();

    // Only e1 should match (changed AND has Marker)
    assert_eq!(changed_entities.len(), 1);
    assert_eq!(changed_entities[0], e1);
}

/// Test macro-generated 3-component tuple with change detection
#[test]
fn test_three_component_change_detection() {
    let mut world = create_world();

    let e1 = world.spawn();
    world.add(e1, Position { x: 1.0, y: 1.0 });
    world.add(e1, Velocity { x: 0.1, y: 0.1 });
    world.add(e1, Health { current: 100, max: 100 });

    let e2 = world.spawn();
    world.add(e2, Position { x: 2.0, y: 2.0 });
    world.add(e2, Velocity { x: 0.2, y: 0.2 });
    world.add(e2, Health { current: 50, max: 100 });

    let tick_before = world.current_tick();

    // Modify Velocity of e1
    {
        let mut query = world.query_mut::<&mut Velocity>();
        if let Some((_, vel)) = query.next() {
            vel.x = 10.0;
        }
    }

    world.increment_tick();

    // Query 3-tuple where Velocity changed
    let changed_entities: Vec<_> = world
        .query_mut::<(&mut Position, &mut Velocity, &mut Health)>()
        .since_tick(tick_before)
        .changed::<Velocity>()
        .map(|(e, _)| e)
        .collect();

    assert_eq!(changed_entities.len(), 1);
    assert_eq!(changed_entities[0], e1);
}

/// Test multiple change filters (entity must have ANY of the specified changes)
#[test]
fn test_multiple_change_filters() {
    let mut world = create_world();

    let e1 = world.spawn();
    world.add(e1, Position { x: 1.0, y: 1.0 });
    world.add(e1, Velocity { x: 0.1, y: 0.1 });

    let e2 = world.spawn();
    world.add(e2, Position { x: 2.0, y: 2.0 });
    world.add(e2, Velocity { x: 0.2, y: 0.2 });

    let e3 = world.spawn();
    world.add(e3, Position { x: 3.0, y: 3.0 });
    world.add(e3, Velocity { x: 0.3, y: 0.3 });

    let tick_before = world.current_tick();

    // Modify Position of e1 and Velocity of e2
    {
        let mut query = world.query_mut::<&mut Position>();
        if let Some((_, pos)) = query.next() {
            pos.x = 10.0; // e1
        }
    }
    {
        let mut query = world.query_mut::<&mut Velocity>();
        query.next(); // Skip e1
        if let Some((_, vel)) = query.next() {
            vel.x = 20.0; // e2
        }
    }

    world.increment_tick();

    // Query where Position changed
    let pos_changed: Vec<_> = world
        .query_mut::<(&mut Position, &mut Velocity)>()
        .since_tick(tick_before)
        .changed::<Position>()
        .map(|(e, _)| e)
        .collect();

    // Only e1 should have Position changed
    assert_eq!(pos_changed.len(), 1);
    assert_eq!(pos_changed[0], e1);

    // Query where Velocity changed
    let vel_changed: Vec<_> = world
        .query_mut::<(&mut Position, &mut Velocity)>()
        .since_tick(tick_before)
        .changed::<Velocity>()
        .map(|(e, _)| e)
        .collect();

    // Only e2 should have Velocity changed
    assert_eq!(vel_changed.len(), 1);
    assert_eq!(vel_changed[0], e2);
}

/// Test that adding a component marks it as changed
#[test]
fn test_add_component_marks_changed() {
    let mut world = create_world();

    let e1 = world.spawn();

    let tick_before = world.current_tick();

    // Add Position component
    world.add(e1, Position { x: 1.0, y: 1.0 });

    world.increment_tick();

    // Query for changed Position
    let changed_entities: Vec<_> = world
        .query_mut::<&mut Position>()
        .since_tick(tick_before)
        .changed::<Position>()
        .map(|(e, _)| e)
        .collect();

    // e1 should be in the results because the component was just added
    assert_eq!(changed_entities.len(), 1);
    assert_eq!(changed_entities[0], e1);
}
