//! Integration test for agentic debugging infrastructure
//!
//! Tests complete workflow:
//! 1. Run physics simulation with event recording
//! 2. Capture snapshots and export to JSONL/SQLite/CSV
//! 3. Use query API to analyze data
//! 4. Detect divergence between simulations
//! 5. Verify AI agent can debug issues autonomously

use engine_math::{Quat, Vec3};
use engine_physics::agentic_debug::WakeReason;
use engine_physics::*;
use tempfile::{NamedTempFile, TempDir};

/// Test: Complete agentic debugging workflow
#[test]
fn test_complete_workflow() {
    let temp_dir = TempDir::new().unwrap();

    // === Phase 1: Run simulation with recording ===

    let mut world = PhysicsWorld::new(PhysicsConfig::default());

    // Create scene: ground + falling box
    world.add_rigidbody(0, &RigidBody::static_body(), Vec3::new(0.0, 0.0, 0.0), Quat::IDENTITY);
    world.add_collider(0, &Collider::box_collider(Vec3::new(10.0, 0.5, 10.0)));

    world.add_rigidbody(1, &RigidBody::dynamic(1.0), Vec3::new(0.0, 10.0, 0.0), Quat::IDENTITY);
    world.add_collider(1, &Collider::box_collider(Vec3::new(0.5, 0.5, 0.5)));

    // Export paths
    let jsonl_path = temp_dir.path().join("physics_debug.jsonl");
    let sqlite_path = temp_dir.path().join("physics_debug.db");
    let csv_path = temp_dir.path().join("physics_metrics.csv");

    let mut jsonl_exporter = JsonlExporter::create(&jsonl_path).unwrap();
    let mut sqlite_exporter = SqliteExporter::create(&sqlite_path).unwrap();
    let mut csv_exporter = CsvExporter::create(&csv_path).unwrap();

    // Run simulation for 60 frames (1 second @ 60fps)
    let dt = 1.0 / 60.0;
    for frame in 0..60 {
        world.step(dt);

        // Create snapshot using PhysicsWorld's built-in method
        let snapshot = world.create_debug_snapshot(frame);

        // Export to all formats
        jsonl_exporter.write_snapshot(&snapshot).unwrap();
        sqlite_exporter.write_snapshot(&snapshot).unwrap();
        csv_exporter.write_snapshot(&snapshot).unwrap();
    }

    // Finish exports
    let jsonl_count = jsonl_exporter.finish().unwrap();
    sqlite_exporter.optimize().unwrap();
    let (sqlite_snapshots, _) = sqlite_exporter.statistics();
    let csv_rows = csv_exporter.finish().unwrap();

    assert_eq!(jsonl_count, 60);
    assert_eq!(sqlite_snapshots, 60);
    assert_eq!(csv_rows, 60 * 2); // 2 entities per snapshot

    // === Phase 2: Query and analyze data ===

    let query_api = PhysicsQueryAPI::open(&sqlite_path).unwrap();

    // Query: Get entity 1 (falling box) history
    let history = query_api.entity_history(1, 0, 59).unwrap();
    assert_eq!(history.len(), 60);

    // Verify physics: entity should have fallen
    assert!(history[0].position.y > 9.0); // Started at y=10
    assert!(history[59].position.y < 6.0); // Should have fallen significantly (still falling at frame 59)

    // Verify it's moving downward
    assert!(history[59].linear_velocity.y < 0.0); // Falling down

    // Verify velocity increased during free fall
    let initial_speed = history[0].linear_velocity.length();
    let mid_speed = history[30].linear_velocity.length();
    assert!(mid_speed > initial_speed); // Accelerating due to gravity

    // Query: Find high-velocity frames
    let high_vel = query_api.find_high_velocity(1, 5.0).unwrap();
    assert!(high_vel.len() > 0); // Should have high velocity during fall

    // Query: Get database statistics
    let stats = query_api.statistics().unwrap();
    assert_eq!(stats.total_frames, 60);
    assert_eq!(stats.total_entities, 2);

    // === Phase 3: Test divergence detection ===

    // Create two fresh identical simulations to test determinism
    let mut world_a = PhysicsWorld::new(PhysicsConfig::default());
    world_a.add_rigidbody(0, &RigidBody::static_body(), Vec3::new(0.0, 0.0, 0.0), Quat::IDENTITY);
    world_a.add_collider(0, &Collider::box_collider(Vec3::new(10.0, 0.5, 10.0)));
    world_a.add_rigidbody(1, &RigidBody::dynamic(1.0), Vec3::new(0.0, 10.0, 0.0), Quat::IDENTITY);
    world_a.add_collider(1, &Collider::box_collider(Vec3::new(0.5, 0.5, 0.5)));

    let mut world_b = PhysicsWorld::new(PhysicsConfig::default());
    world_b.add_rigidbody(0, &RigidBody::static_body(), Vec3::new(0.0, 0.0, 0.0), Quat::IDENTITY);
    world_b.add_collider(0, &Collider::box_collider(Vec3::new(10.0, 0.5, 10.0)));
    world_b.add_rigidbody(1, &RigidBody::dynamic(1.0), Vec3::new(0.0, 10.0, 0.0), Quat::IDENTITY);
    world_b.add_collider(1, &Collider::box_collider(Vec3::new(0.5, 0.5, 0.5)));

    let mut detector = DivergenceDetector::new();

    // Run both simulations and compare (should be identical - deterministic physics)
    for frame in 0..60 {
        world_a.step(dt);
        world_b.step(dt);

        let snapshot_a = world_a.create_debug_snapshot(frame);
        let snapshot_b = world_b.create_debug_snapshot(frame);

        // Should have no divergence (deterministic)
        let divergence = detector.check_divergence(&snapshot_a, &snapshot_b);
        assert!(divergence.is_none(), "Deterministic simulation should not diverge");
    }

    assert_eq!(detector.total_divergences(), 0);
}

/// Test: Divergence detection with intentional mismatch
#[test]
fn test_divergence_detection() {
    let mut detector = DivergenceDetector::new();

    // Create reference snapshot
    let mut reference = PhysicsDebugSnapshot::new(1, 0.016);
    reference.entities.push(EntityState {
        id: 1,
        position: Vec3::new(1.0, 2.0, 3.0),
        rotation: Quat::IDENTITY,
        linear_velocity: Vec3::new(5.0, 0.0, 0.0),
        angular_velocity: Vec3::ZERO,
        forces: Vec3::ZERO,
        torques: Vec3::ZERO,
        mass: 1.0,
        linear_damping: 0.0,
        angular_damping: 0.0,
        gravity_scale: 1.0,
        sleeping: false,
        is_static: false,
        is_kinematic: false,
        can_sleep: true,
        ccd_enabled: false,
    });

    // Create diverged snapshot (position off by 50cm)
    let mut diverged = reference.clone();
    diverged.entities[0].position.x += 0.5;

    let report = detector.check_divergence(&reference, &diverged);
    assert!(report.is_some());

    let report = report.unwrap();
    assert_eq!(report.diverged_entities.len(), 1);
    assert_eq!(report.diverged_entities[0].entity_id, 1);
    assert!((report.diverged_entities[0].position_delta - 0.5).abs() < 0.01);
}

/// Test: Event recording and export
#[test]
fn test_event_recording() {
    let temp_file = NamedTempFile::new().unwrap();
    let mut exporter = JsonlExporter::create(temp_file.path()).unwrap();

    let mut recorder = EventRecorder::new();
    recorder.enable();

    // Record some events
    recorder.record(PhysicsEvent::CollisionStart {
        frame: 1,
        timestamp: 0.016,
        entity_a: 1,
        entity_b: 2,
        contact_point: Vec3::ZERO,
        normal: Vec3::Y,
        impulse: 10.0,
        relative_velocity: 5.0,
    });

    recorder.record(PhysicsEvent::EntityWake {
        frame: 2,
        timestamp: 0.032,
        entity_id: 3,
        reason: WakeReason::Collision,
    });

    let events = recorder.drain_events();
    assert_eq!(events.len(), 2);

    // Export events
    exporter.write_events(&events).unwrap();
    exporter.flush().unwrap();

    assert_eq!(exporter.objects_written(), 2);
}

/// Test: CSV export format
#[test]
fn test_csv_export_format() {
    let temp_file = NamedTempFile::new().unwrap();
    let mut exporter = CsvExporter::create(temp_file.path()).unwrap();

    let mut snapshot = PhysicsDebugSnapshot::new(0, 0.0);
    snapshot.entities.push(EntityState {
        id: 42,
        position: Vec3::new(1.0, 2.0, 3.0),
        rotation: Quat::IDENTITY,
        linear_velocity: Vec3::new(5.0, 0.0, 0.0),
        angular_velocity: Vec3::ZERO,
        forces: Vec3::ZERO,
        torques: Vec3::ZERO,
        mass: 1.5,
        linear_damping: 0.0,
        angular_damping: 0.0,
        gravity_scale: 1.0,
        sleeping: false,
        is_static: false,
        is_kinematic: false,
        can_sleep: true,
        ccd_enabled: false,
    });

    exporter.write_snapshot(&snapshot).unwrap();
    exporter.flush().unwrap();

    // Read CSV and verify format
    let content = std::fs::read_to_string(temp_file.path()).unwrap();
    let lines: Vec<&str> = content.lines().collect();

    assert_eq!(lines.len(), 2); // Header + 1 data row

    // Header should contain expected columns
    assert!(lines[0].contains("frame"));
    assert!(lines[0].contains("entity_id"));
    assert!(lines[0].contains("pos_x"));
    assert!(lines[0].contains("vel_x"));

    // Data row should contain correct values
    assert!(lines[1].contains("42")); // entity_id
    let parts: Vec<&str> = lines[1].split(',').collect();
    assert!(parts.len() > 10); // Should have many columns with data
}

/// Test: Hash consistency
#[test]
fn test_hash_consistency() {
    let mut snapshot1 = PhysicsDebugSnapshot::new(100, 1.5);
    snapshot1.entities.push(EntityState {
        id: 1,
        position: Vec3::new(1.234, 2.345, 3.456),
        rotation: Quat::IDENTITY,
        linear_velocity: Vec3::new(5.0, 0.0, 0.0),
        angular_velocity: Vec3::ZERO,
        forces: Vec3::ZERO,
        torques: Vec3::ZERO,
        mass: 1.0,
        linear_damping: 0.0,
        angular_damping: 0.0,
        gravity_scale: 1.0,
        sleeping: false,
        is_static: false,
        is_kinematic: false,
        can_sleep: true,
        ccd_enabled: false,
    });

    let snapshot2 = snapshot1.clone();

    // Same state should produce same hash
    let hash1 = snapshot1.compute_hash();
    let hash2 = snapshot2.compute_hash();
    assert_eq!(hash1, hash2);

    // Tiny change should produce different hash (after fixed-point rounding)
    let mut snapshot3 = snapshot1.clone();
    snapshot3.entities[0].position.x += 0.001; // 1mm change

    let hash3 = snapshot3.compute_hash();
    assert_ne!(hash1, hash3);
}

/// Test: Large-scale export (performance)
#[test]
fn test_large_scale_export() {
    let temp_dir = TempDir::new().unwrap();
    let sqlite_path = temp_dir.path().join("large_physics.db");

    let mut exporter = SqliteExporter::create(&sqlite_path).unwrap();

    // Export 1000 frames with 100 entities each
    for frame in 0..1000 {
        let mut snapshot = PhysicsDebugSnapshot::new(frame, frame as f64 * 0.016);

        for entity_id in 1..=100 {
            snapshot.entities.push(EntityState {
                id: entity_id,
                position: Vec3::new(entity_id as f32, frame as f32 * 0.01, 0.0),
                rotation: Quat::IDENTITY,
                linear_velocity: Vec3::ZERO,
                angular_velocity: Vec3::ZERO,
                forces: Vec3::ZERO,
                torques: Vec3::ZERO,
                mass: 1.0,
                linear_damping: 0.0,
                angular_damping: 0.0,
                gravity_scale: 1.0,
                sleeping: false,
                is_static: false,
                is_kinematic: false,
                can_sleep: true,
                ccd_enabled: false,
            });
        }

        exporter.write_snapshot(&snapshot).unwrap();
    }

    let (snapshots, _) = exporter.statistics();
    assert_eq!(snapshots, 1000);

    // Query the database
    let query_api = PhysicsQueryAPI::open(&sqlite_path).unwrap();
    let stats = query_api.statistics().unwrap();

    assert_eq!(stats.total_frames, 1000);
    assert_eq!(stats.total_entities, 100);

    // Query entity history (should be fast)
    let history = query_api.entity_history(50, 0, 999).unwrap();
    assert_eq!(history.len(), 1000);
}
