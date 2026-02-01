//! Example integration test demonstrating test infrastructure usage
//!
//! This file shows how to use the common test utilities and integration helpers.

// Import common test utilities
mod common;
mod integration;

use common::{
    MockPosition, MockVelocity, MockHealth, MockName, MockPlayer,
    TestEntityBuilder, TestIdGenerator, create_test_entities,
    assert_approx_eq, assert_position_eq,
};

use integration::{
    init_test_environment, IntegrationTestConfig,
    MultiFrameTest, PerformanceMeasurement,
};

#[test]
fn test_basic_component_usage() {
    // Test mock components
    let pos = MockPosition::new(1.0, 2.0, 3.0);
    assert_approx_eq!(pos.x, 1.0);
    assert_approx_eq!(pos.y, 2.0);
    assert_approx_eq!(pos.z, 3.0);

    let vel = MockVelocity::new(0.5, 0.0, -0.5);
    assert_approx_eq!(vel.magnitude(), 0.707, 0.001);

    let mut health = MockHealth::full(100);
    assert!(health.is_alive());
    assert!(health.is_full());

    health.damage(30);
    assert_eq!(health.current, 70);
    assert!(!health.is_full());
    assert!(health.is_alive());
}

#[test]
fn test_entity_builder() {
    let entity = TestEntityBuilder::new()
        .with_position(1.0, 2.0, 3.0)
        .with_velocity(0.5, 0.0, -0.5)
        .with_health(50, 100)
        .with_name("TestEntity")
        .as_player();

    assert!(entity.position().is_some());
    assert!(entity.velocity().is_some());
    assert!(entity.health().is_some());
    assert!(entity.name().is_some());
    assert!(entity.is_player());
    assert!(!entity.is_enemy());

    // Check values
    let pos = entity.position().unwrap();
    assert_position_eq!(*pos, MockPosition::new(1.0, 2.0, 3.0));

    let health = entity.health().unwrap();
    assert_eq!(health.current, 50);
    assert_eq!(health.max, 100);
}

#[test]
fn test_id_generator() {
    let gen = TestIdGenerator::new();

    // Generate sequential IDs
    assert_eq!(gen.next(), 1);
    assert_eq!(gen.next(), 2);
    assert_eq!(gen.next(), 3);

    // Reset and verify
    gen.reset();
    assert_eq!(gen.next(), 1);
}

#[test]
fn test_batch_entity_creation() {
    let entities = create_test_entities(10);
    assert_eq!(entities.len(), 10);

    for (i, entity) in entities.iter().enumerate() {
        let pos = entity.position().unwrap();
        assert_approx_eq!(pos.x, i as f32);
        assert_approx_eq!(pos.y, i as f32);
        assert_approx_eq!(pos.z, i as f32);

        let name = entity.name().unwrap();
        assert_eq!(name.value, format!("Entity_{}", i));
    }
}

#[test]
fn test_position_distance() {
    let pos1 = MockPosition::new(0.0, 0.0, 0.0);
    let pos2 = MockPosition::new(3.0, 4.0, 0.0);

    let distance = pos1.distance_to(&pos2);
    assert_approx_eq!(distance, 5.0);
}

#[test]
fn test_multi_frame_simulation() {
    init_test_environment();

    let mut positions = vec![
        MockPosition::new(0.0, 0.0, 0.0),
        MockPosition::new(10.0, 0.0, 0.0),
    ];

    let velocities = vec![
        MockVelocity::new(1.0, 0.0, 0.0),
        MockVelocity::new(-1.0, 0.0, 0.0),
    ];

    let mut test = MultiFrameTest::new(10);

    test.run(|_frame| {
        // Update positions
        for (pos, vel) in positions.iter_mut().zip(velocities.iter()) {
            pos.x += vel.x;
            pos.y += vel.y;
            pos.z += vel.z;
        }
    });

    // After 10 frames, objects should have moved
    assert_approx_eq!(positions[0].x, 10.0);
    assert_approx_eq!(positions[1].x, 0.0);
}

#[test]
fn test_performance_measurement() {
    let mut perf = PerformanceMeasurement::new("test_operation");

    for _ in 0..10 {
        perf.measure(|| {
            // Simulate some work
            let mut sum = 0;
            for i in 0..1000 {
                sum += i;
            }
            sum
        });
    }

    assert_eq!(perf.samples.len(), 10);
    assert!(perf.average() > std::time::Duration::ZERO);
    assert!(perf.min() <= perf.average());
    assert!(perf.max() >= perf.average());
}

#[test]
fn test_custom_assertions() {
    let pos1 = MockPosition::new(1.0, 2.0, 3.0);
    let pos2 = MockPosition::new(1.0001, 2.0001, 3.0001);

    // Should pass with epsilon
    assert_position_eq!(pos1, pos2, 0.001);

    // Test velocity assertion
    let vel1 = MockVelocity::new(1.0, 2.0, 3.0);
    let vel2 = MockVelocity::new(1.0001, 2.0001, 3.0001);
    assert_velocity_eq!(vel1, vel2, 0.001);
}

#[test]
fn test_integration_config() {
    let config = IntegrationTestConfig::new()
        .with_logging()
        .with_timeout(10000);

    assert!(config.enable_logging);
    assert_eq!(config.timeout_ms, 10000);
}

#[test]
#[ignore] // Ignored by default, run with: cargo test -- --ignored
fn test_stress_simulation() {
    use common::stress_test;

    let mut counter = 0;

    stress_test(10000, |_| {
        counter += 1;
    });

    assert_eq!(counter, 10000);
}

#[test]
#[ignore] // Ignored by default, run with: cargo test -- --ignored
fn test_parallel_stress() {
    use common::parallel_stress_test;
    use std::sync::{Arc, atomic::{AtomicU32, Ordering}};

    let counter = Arc::new(AtomicU32::new(0));
    let counter_clone = Arc::clone(&counter);

    parallel_stress_test(4, 1000, move |_| {
        counter_clone.fetch_add(1, Ordering::SeqCst);
    });

    assert_eq!(counter.load(Ordering::SeqCst), 4000);
}
