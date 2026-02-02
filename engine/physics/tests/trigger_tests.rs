//! Trigger/sensor collider tests
//!
//! Tests trigger detection including:
//! - Trigger enter events
//! - Trigger exit events
//! - Sensor colliders don't cause collision response
//! - Multiple simultaneous triggers

use engine_math::{Quat, Vec3};
use engine_physics::{Collider, ColliderShape, PhysicsConfig, PhysicsWorld, RigidBody};

#[test]
fn test_trigger_enter_event() {
    let config = PhysicsConfig::default();
    let mut world = PhysicsWorld::new(config);

    // Create trigger zone (sensor)
    let trigger = 1u64;
    let rb_trigger = RigidBody::static_body();
    world.add_rigidbody(trigger, &rb_trigger, Vec3::new(0.0, 0.0, 0.0), Quat::IDENTITY);

    let trigger_collider =
        Collider::sensor(ColliderShape::Box { half_extents: Vec3::new(2.0, 2.0, 2.0) });
    world.add_collider(trigger, &trigger_collider);

    // Create dynamic object outside trigger
    let object = 2u64;
    let rb_object = RigidBody::dynamic(1.0);
    world.add_rigidbody(object, &rb_object, Vec3::new(5.0, 0.0, 0.0), Quat::IDENTITY);

    let object_collider = Collider::sphere(0.5);
    world.add_collider(object, &object_collider);

    // Step once to initialize
    world.step(1.0 / 60.0);

    assert_eq!(world.trigger_enter_events().len(), 0, "No trigger events initially");

    // Move object into trigger zone
    world.set_transform(object, Vec3::new(1.0, 0.0, 0.0), Quat::IDENTITY);

    // Step to detect collision
    world.step(1.0 / 60.0);

    let enter_events = world.trigger_enter_events();

    assert_eq!(enter_events.len(), 1, "Should have one trigger enter event");

    let (trigger_entity, other_entity) = enter_events[0];
    assert_eq!(trigger_entity, trigger, "Trigger entity should be the sensor");
    assert_eq!(other_entity, object, "Other entity should be the dynamic object");
}

#[test]
fn test_trigger_exit_event() {
    let config = PhysicsConfig::default();
    let mut world = PhysicsWorld::new(config);

    // Create trigger zone
    let trigger = 1u64;
    let rb_trigger = RigidBody::static_body();
    world.add_rigidbody(trigger, &rb_trigger, Vec3::new(0.0, 0.0, 0.0), Quat::IDENTITY);

    let trigger_collider =
        Collider::sensor(ColliderShape::Box { half_extents: Vec3::new(2.0, 2.0, 2.0) });
    world.add_collider(trigger, &trigger_collider);

    // Create object inside trigger
    let object = 2u64;
    let rb_object = RigidBody::dynamic(1.0);
    world.add_rigidbody(object, &rb_object, Vec3::new(0.0, 0.0, 0.0), Quat::IDENTITY);

    let object_collider = Collider::sphere(0.5);
    world.add_collider(object, &object_collider);

    // Step to detect initial collision
    world.step(1.0 / 60.0);

    // Should get enter event
    assert_eq!(
        world.trigger_enter_events().len(),
        1,
        "Should have trigger enter event when starting inside"
    );

    // Step again to clear events
    world.step(1.0 / 60.0);

    // Now move object outside trigger
    world.set_transform(object, Vec3::new(5.0, 0.0, 0.0), Quat::IDENTITY);

    // Step to detect exit
    world.step(1.0 / 60.0);

    let exit_events = world.trigger_exit_events();

    assert_eq!(exit_events.len(), 1, "Should have one trigger exit event");

    let (trigger_entity, other_entity) = exit_events[0];
    assert_eq!(trigger_entity, trigger);
    assert_eq!(other_entity, object);
}

#[test]
fn test_trigger_enter_exit_cycle() {
    let config = PhysicsConfig::default();
    let mut world = PhysicsWorld::new(config);

    // Create trigger
    let trigger = 1u64;
    let rb_trigger = RigidBody::static_body();
    world.add_rigidbody(trigger, &rb_trigger, Vec3::new(0.0, 0.0, 0.0), Quat::IDENTITY);

    let trigger_collider =
        Collider::sensor(ColliderShape::Box { half_extents: Vec3::new(2.0, 2.0, 2.0) });
    world.add_collider(trigger, &trigger_collider);

    // Create object
    let object = 2u64;
    let rb_object = RigidBody::dynamic(1.0);
    world.add_rigidbody(object, &rb_object, Vec3::new(5.0, 0.0, 0.0), Quat::IDENTITY);

    let object_collider = Collider::sphere(0.5);
    world.add_collider(object, &object_collider);

    world.step(1.0 / 60.0);

    // Move in
    world.set_transform(object, Vec3::new(0.0, 0.0, 0.0), Quat::IDENTITY);
    world.step(1.0 / 60.0);
    assert_eq!(world.trigger_enter_events().len(), 1, "Enter event");
    assert_eq!(world.trigger_exit_events().len(), 0, "No exit event");

    // Stay inside (no new events)
    world.step(1.0 / 60.0);
    assert_eq!(world.trigger_enter_events().len(), 0, "No new enter event");
    assert_eq!(world.trigger_exit_events().len(), 0, "No exit event");

    // Move out
    world.set_transform(object, Vec3::new(5.0, 0.0, 0.0), Quat::IDENTITY);
    world.step(1.0 / 60.0);
    assert_eq!(world.trigger_enter_events().len(), 0, "No enter event");
    assert_eq!(world.trigger_exit_events().len(), 1, "Exit event");

    // Stay outside (no new events)
    world.step(1.0 / 60.0);
    assert_eq!(world.trigger_enter_events().len(), 0, "No new enter event");
    assert_eq!(world.trigger_exit_events().len(), 0, "No new exit event");

    // Move in again
    world.set_transform(object, Vec3::new(0.0, 0.0, 0.0), Quat::IDENTITY);
    world.step(1.0 / 60.0);
    assert_eq!(world.trigger_enter_events().len(), 1, "Enter event again");
    assert_eq!(world.trigger_exit_events().len(), 0, "No exit event");
}

#[test]
fn test_sensor_no_collision_response() {
    let config = PhysicsConfig::default();
    let mut world = PhysicsWorld::new(config);

    // Create sensor (should not block movement)
    let sensor = 1u64;
    let rb_sensor = RigidBody::static_body();
    world.add_rigidbody(sensor, &rb_sensor, Vec3::new(0.0, 0.0, 0.0), Quat::IDENTITY);

    let sensor_collider =
        Collider::sensor(ColliderShape::Box { half_extents: Vec3::new(2.0, 2.0, 2.0) });
    world.add_collider(sensor, &sensor_collider);

    // Create dynamic object with velocity toward sensor
    let object = 2u64;
    let rb_object = RigidBody::dynamic(1.0);
    world.add_rigidbody(object, &rb_object, Vec3::new(-5.0, 0.0, 0.0), Quat::IDENTITY);

    let object_collider = Collider::sphere(0.5);
    world.add_collider(object, &object_collider);

    // Set velocity to move through sensor
    world.set_velocity(object, Vec3::new(10.0, 0.0, 0.0), Vec3::ZERO);

    // Step several times
    for _ in 0..10 {
        world.step(1.0 / 60.0);
    }

    // Object should have passed through sensor (not stopped by it)
    let (pos, _) = world.get_transform(object).unwrap();
    assert!(pos.x > 2.0, "Object should pass through sensor (pos.x={})", pos.x);
}

#[test]
fn test_multiple_triggers() {
    let config = PhysicsConfig::default();
    let mut world = PhysicsWorld::new(config);

    // Create two trigger zones side by side
    for i in 0..2 {
        let trigger = (i + 1) as u64;
        let rb = RigidBody::static_body();
        world.add_rigidbody(trigger, &rb, Vec3::new(i as f32 * 5.0, 0.0, 0.0), Quat::IDENTITY);

        let collider =
            Collider::sensor(ColliderShape::Box { half_extents: Vec3::new(2.0, 2.0, 2.0) });
        world.add_collider(trigger, &collider);
    }

    // Create object
    let object = 3u64;
    let rb_object = RigidBody::dynamic(1.0);
    world.add_rigidbody(object, &rb_object, Vec3::new(0.0, 0.0, 0.0), Quat::IDENTITY);

    let object_collider = Collider::sphere(0.5);
    world.add_collider(object, &object_collider);

    // Step to detect collision with first trigger
    world.step(1.0 / 60.0);

    assert_eq!(world.trigger_enter_events().len(), 1, "Should enter first trigger");
    assert_eq!(world.trigger_enter_events()[0].0, 1, "Should be trigger 1");

    // Clear events
    world.step(1.0 / 60.0);

    // Move to second trigger
    world.set_transform(object, Vec3::new(5.0, 0.0, 0.0), Quat::IDENTITY);
    world.step(1.0 / 60.0);

    // Should exit first and enter second
    assert_eq!(world.trigger_exit_events().len(), 1, "Should exit first trigger");
    assert_eq!(world.trigger_exit_events()[0].0, 1, "Should exit trigger 1");

    assert_eq!(world.trigger_enter_events().len(), 1, "Should enter second trigger");
    assert_eq!(world.trigger_enter_events()[0].0, 2, "Should be trigger 2");
}

#[test]
fn test_overlapping_triggers() {
    let config = PhysicsConfig::default();
    let mut world = PhysicsWorld::new(config);

    // Create two overlapping trigger zones
    for i in 0..2 {
        let trigger = (i + 1) as u64;
        let rb = RigidBody::static_body();
        world.add_rigidbody(trigger, &rb, Vec3::ZERO, Quat::IDENTITY);

        let collider =
            Collider::sensor(ColliderShape::Box { half_extents: Vec3::new(2.0, 2.0, 2.0) });
        world.add_collider(trigger, &collider);
    }

    // Create object outside both triggers
    let object = 3u64;
    let rb_object = RigidBody::dynamic(1.0);
    world.add_rigidbody(object, &rb_object, Vec3::new(5.0, 0.0, 0.0), Quat::IDENTITY);

    let object_collider = Collider::sphere(0.5);
    world.add_collider(object, &object_collider);

    world.step(1.0 / 60.0);

    // Move into overlapping region
    world.set_transform(object, Vec3::ZERO, Quat::IDENTITY);
    world.step(1.0 / 60.0);

    // Should enter both triggers
    let enter_events = world.trigger_enter_events();
    assert_eq!(enter_events.len(), 2, "Should enter both overlapping triggers");

    // Clear events
    world.step(1.0 / 60.0);

    // Move out
    world.set_transform(object, Vec3::new(5.0, 0.0, 0.0), Quat::IDENTITY);
    world.step(1.0 / 60.0);

    // Should exit both triggers
    let exit_events = world.trigger_exit_events();
    assert_eq!(exit_events.len(), 2, "Should exit both overlapping triggers");
}

#[test]
fn test_trigger_with_dynamic_object() {
    let config = PhysicsConfig::default();
    let mut world = PhysicsWorld::new(config);

    // Create trigger
    let trigger = 1u64;
    let rb_trigger = RigidBody::static_body();
    world.add_rigidbody(trigger, &rb_trigger, Vec3::new(0.0, -5.0, 0.0), Quat::IDENTITY);

    let trigger_collider =
        Collider::sensor(ColliderShape::Box { half_extents: Vec3::new(5.0, 1.0, 5.0) });
    world.add_collider(trigger, &trigger_collider);

    // Create falling object
    let object = 2u64;
    let rb_object = RigidBody::dynamic(1.0);
    world.add_rigidbody(object, &rb_object, Vec3::new(0.0, 10.0, 0.0), Quat::IDENTITY);

    let object_collider = Collider::sphere(0.5);
    world.add_collider(object, &object_collider);

    // Let object fall
    let mut entered = false;
    for _ in 0..200 {
        world.step(1.0 / 60.0);

        if !world.trigger_enter_events().is_empty() {
            entered = true;
            break;
        }
    }

    assert!(entered, "Falling object should trigger enter event");
}

#[test]
fn test_sensor_collider_constructor() {
    let sensor = Collider::sensor(ColliderShape::Box { half_extents: Vec3::ONE });
    assert!(sensor.is_sensor, "Sensor constructor should set is_sensor to true");
}

#[test]
fn test_non_sensor_collider() {
    let config = PhysicsConfig::default();
    let mut world = PhysicsWorld::new(config);

    // Create regular (non-sensor) collider
    let wall = 1u64;
    let rb_wall = RigidBody::static_body();
    world.add_rigidbody(wall, &rb_wall, Vec3::new(0.0, 0.0, 0.0), Quat::IDENTITY);

    let wall_collider = Collider::box_collider(Vec3::new(2.0, 2.0, 2.0));
    world.add_collider(wall, &wall_collider);

    // Create dynamic object
    let object = 2u64;
    let rb_object = RigidBody::dynamic(1.0);
    world.add_rigidbody(object, &rb_object, Vec3::new(5.0, 0.0, 0.0), Quat::IDENTITY);

    let object_collider = Collider::sphere(0.5);
    world.add_collider(object, &object_collider);

    // Set velocity toward wall
    world.set_velocity(object, Vec3::new(-10.0, 0.0, 0.0), Vec3::ZERO);

    // Step several times
    for _ in 0..30 {
        world.step(1.0 / 60.0);
    }

    // Object should be stopped by wall (not pass through)
    let (pos, _) = world.get_transform(object).unwrap();
    assert!(pos.x > 2.0, "Object should be stopped by non-sensor collider (pos.x={})", pos.x);

    // Should NOT generate trigger events
    assert_eq!(
        world.trigger_enter_events().len(),
        0,
        "Non-sensor collider should not generate trigger events"
    );
}
