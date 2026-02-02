//! Integration test for Phase A.2 - Enhanced Profiling Metrics
//!
//! Tests complete metrics collection workflow:
//! 1. Enable metrics collection
//! 2. Run physics simulation
//! 3. Extract frame metrics
//! 4. Verify all statistics are populated correctly
//! 5. Test metrics serialization

use engine_math::{Quat, Vec3};
use engine_physics::*;

#[test]
fn test_metrics_disabled_by_default() {
    let world = PhysicsWorld::new(PhysicsConfig::default());
    assert!(!world.metrics_enabled());
}

#[test]
fn test_metrics_enable_disable() {
    let mut world = PhysicsWorld::new(PhysicsConfig::default());

    // Initially disabled
    assert!(!world.metrics_enabled());

    // Enable
    world.enable_metrics();
    assert!(world.metrics_enabled());

    // Disable
    world.disable_metrics();
    assert!(!world.metrics_enabled());
}

#[test]
fn test_metrics_collection_basic() {
    let mut world = PhysicsWorld::new(PhysicsConfig::default());

    // Add some entities
    world.add_rigidbody(0, &RigidBody::static_body(), Vec3::ZERO, Quat::IDENTITY);
    world.add_collider(0, &Collider::box_collider(Vec3::new(10.0, 0.5, 10.0)));

    world.add_rigidbody(1, &RigidBody::dynamic(1.0), Vec3::new(0.0, 10.0, 0.0), Quat::IDENTITY);
    world.add_collider(1, &Collider::box_collider(Vec3::ONE));

    // Enable metrics
    world.enable_metrics();

    // Step simulation
    world.step(1.0 / 60.0);

    // Get metrics
    let metrics = world.last_frame_metrics().expect("Metrics should be available");

    // Verify basic statistics
    assert_eq!(metrics.frame, 1);
    assert!(metrics.frame_time_us > 0);
    assert_eq!(metrics.active_body_count, 2); // Static + dynamic
    assert_eq!(metrics.sleeping_body_count, 0);
    assert_eq!(metrics.total_collider_count, 2);

    // Verify performance tracking
    assert!(metrics.solver_time_us > 0);
}

#[test]
fn test_metrics_collision_tracking() {
    let mut world = PhysicsWorld::new(PhysicsConfig::default());

    // Ground
    world.add_rigidbody(0, &RigidBody::static_body(), Vec3::ZERO, Quat::IDENTITY);
    world.add_collider(0, &Collider::box_collider(Vec3::new(10.0, 0.5, 10.0)));

    // Falling box
    world.add_rigidbody(1, &RigidBody::dynamic(1.0), Vec3::new(0.0, 2.0, 0.0), Quat::IDENTITY);
    world.add_collider(1, &Collider::box_collider(Vec3::ONE));

    world.enable_metrics();

    // Step until collision
    for _ in 0..60 {
        world.step(1.0 / 60.0);
    }

    let metrics = world.last_frame_metrics().unwrap();

    // Should have detected collision pair
    assert!(metrics.collision_pair_count > 0, "Should detect collision pairs");
    assert!(metrics.active_contact_count > 0, "Should have active contacts");
}

#[test]
fn test_metrics_island_tracking() {
    let mut world = PhysicsWorld::new(PhysicsConfig::default());

    // Create multiple dynamic bodies
    for i in 0..5 {
        world.add_rigidbody(
            i,
            &RigidBody::dynamic(1.0),
            Vec3::new(i as f32 * 5.0, 10.0, 0.0),
            Quat::IDENTITY,
        );
        world.add_collider(i, &Collider::box_collider(Vec3::ONE));
    }

    world.enable_metrics();
    world.step(1.0 / 60.0);

    let metrics = world.last_frame_metrics().unwrap();

    // Should have at least 1 island
    assert!(metrics.island_count > 0);
    assert!(metrics.avg_bodies_per_island > 0.0);
    assert!(metrics.max_bodies_in_island > 0);
}

#[test]
fn test_metrics_joint_tracking() {
    let mut world = PhysicsWorld::new(PhysicsConfig::default());

    // Two bodies connected by a joint
    world.add_rigidbody(1, &RigidBody::dynamic(1.0), Vec3::new(-1.0, 0.0, 0.0), Quat::IDENTITY);
    world.add_collider(1, &Collider::box_collider(Vec3::ONE));

    world.add_rigidbody(2, &RigidBody::dynamic(1.0), Vec3::new(1.0, 0.0, 0.0), Quat::IDENTITY);
    world.add_collider(2, &Collider::box_collider(Vec3::ONE));

    // Add fixed joint between them
    let joint = FixedJointConfig {
        anchor1: Vec3::new(0.5, 0.0, 0.0),
        anchor2: Vec3::new(-0.5, 0.0, 0.0),
        rotation1: Quat::IDENTITY,
        rotation2: Quat::IDENTITY,
    };
    world.add_joint(1, 2, Joint::Fixed(joint));

    world.enable_metrics();
    world.step(1.0 / 60.0);

    let metrics = world.last_frame_metrics().unwrap();

    // Should track the joint
    assert_eq!(metrics.total_joint_count, 1);
    assert_eq!(metrics.constraint_count, 1);
}

#[test]
fn test_metrics_performance_warning() {
    let metrics = FrameMetrics {
        frame: 1,
        frame_time_us: 20_000, // 20ms - over 60 FPS budget
        broadphase_time_us: 5_000,
        narrowphase_time_us: 5_000,
        solver_time_us: 8_000,
        island_build_time_us: 1_000,
        ccd_time_us: 1_000,
        island_count: 2,
        avg_bodies_per_island: 50.0,
        max_bodies_in_island: 75,
        active_body_count: 100,
        sleeping_body_count: 50,
        collision_pair_count: 200,
        active_contact_count: 150,
        solver_iterations: 4,
        solver_residual: 0.001,
        constraint_count: 20,
        total_collider_count: 150,
        total_joint_count: 25,
    };

    assert!(metrics.has_performance_warning());
}

#[test]
fn test_metrics_overhead_calculation() {
    let metrics = FrameMetrics {
        frame: 1,
        frame_time_us: 10_000,      // 10ms total
        broadphase_time_us: 2_000,  // 2ms
        narrowphase_time_us: 3_000, // 3ms
        solver_time_us: 4_000,      // 4ms
        island_build_time_us: 500,
        ccd_time_us: 500,
        island_count: 1,
        avg_bodies_per_island: 100.0,
        max_bodies_in_island: 100,
        active_body_count: 100,
        sleeping_body_count: 0,
        collision_pair_count: 150,
        active_contact_count: 120,
        solver_iterations: 4,
        solver_residual: 0.0,
        constraint_count: 10,
        total_collider_count: 100,
        total_joint_count: 12,
    };

    // Verify percentage calculations
    assert!((metrics.broadphase_overhead_percent() - 20.0).abs() < 0.1);
    assert!((metrics.narrowphase_overhead_percent() - 30.0).abs() < 0.1);
    assert!((metrics.solver_overhead_percent() - 40.0).abs() < 0.1);
}

#[test]
fn test_metrics_serialization() {
    let mut world = PhysicsWorld::new(PhysicsConfig::default());

    world.add_rigidbody(0, &RigidBody::dynamic(1.0), Vec3::ZERO, Quat::IDENTITY);
    world.add_collider(0, &Collider::box_collider(Vec3::ONE));

    world.enable_metrics();
    world.step(1.0 / 60.0);

    let metrics = world.last_frame_metrics().unwrap();

    // Test JSON serialization
    let json = serde_json::to_string(&metrics).expect("Should serialize to JSON");
    assert!(json.contains("frame"));
    assert!(json.contains("frame_time_us"));
    assert!(json.contains("active_body_count"));

    // Test roundtrip
    let deserialized: FrameMetrics = serde_json::from_str(&json).expect("Should deserialize");
    assert_eq!(metrics, deserialized);
}

#[test]
fn test_metrics_summary_format() {
    let metrics = FrameMetrics {
        frame: 42,
        frame_time_us: 8_500, // 8.5ms
        broadphase_time_us: 1_000,
        narrowphase_time_us: 2_000,
        solver_time_us: 5_000,
        island_build_time_us: 250,
        ccd_time_us: 250,
        island_count: 3,
        avg_bodies_per_island: 15.5,
        max_bodies_in_island: 20,
        active_body_count: 45,
        sleeping_body_count: 10,
        collision_pair_count: 80,
        active_contact_count: 60,
        solver_iterations: 4,
        solver_residual: 0.0,
        constraint_count: 8,
        total_collider_count: 55,
        total_joint_count: 10,
    };

    let summary = metrics.summary();

    // Verify summary contains key information
    assert!(summary.contains("Frame 42"));
    assert!(summary.contains("8.50ms"));
    assert!(summary.contains("45 active"));
    assert!(summary.contains("10 sleeping"));
    assert!(summary.contains("Islands: 3"));
    assert!(summary.contains("Pairs: 80"));
    assert!(summary.contains("Contacts: 60"));
}

#[test]
fn test_metrics_continuous_collection() {
    let mut world = PhysicsWorld::new(PhysicsConfig::default());

    world.add_rigidbody(0, &RigidBody::static_body(), Vec3::ZERO, Quat::IDENTITY);
    world.add_collider(0, &Collider::box_collider(Vec3::new(10.0, 0.5, 10.0)));

    world.add_rigidbody(1, &RigidBody::dynamic(1.0), Vec3::new(0.0, 10.0, 0.0), Quat::IDENTITY);
    world.add_collider(1, &Collider::box_collider(Vec3::ONE));

    world.enable_metrics();

    // Collect metrics over multiple frames
    let mut all_metrics = Vec::new();
    for _ in 0..10 {
        world.step(1.0 / 60.0);
        if let Some(metrics) = world.last_frame_metrics() {
            all_metrics.push(metrics);
        }
    }

    assert_eq!(all_metrics.len(), 10);

    // Verify frame numbers are sequential
    for (i, metrics) in all_metrics.iter().enumerate() {
        assert_eq!(metrics.frame, (i + 1) as u64);
    }

    // Verify all have timing data
    for metrics in &all_metrics {
        assert!(metrics.frame_time_us > 0);
        assert!(metrics.solver_time_us > 0);
    }
}
