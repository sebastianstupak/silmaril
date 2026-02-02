//! Agentic Debugging System Benchmarks
//!
//! Measures performance overhead of the agentic debugging infrastructure:
//! - Snapshot creation overhead (1-10K entities)
//! - Export throughput (JSONL, SQLite, CSV)
//! - Query latency (entity_history, find_high_velocity, events_by_type)
//! - Total physics step overhead with debugging enabled
//!
//! ## Performance Targets
//!
//! - Snapshot creation: < 1ms for 1000 entities
//! - Export (JSONL): > 10 MB/sec throughput
//! - Export (SQLite): > 100 frames/sec throughput
//! - Export (CSV): > 5 MB/sec throughput
//! - Query latency: < 10ms for entity_history, find_high_velocity, events_by_type
//! - Total overhead: < 5% compared to normal physics step

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_math::{Quat, Vec3};
use engine_physics::agentic_debug::{CsvExporter, JsonlExporter, PhysicsQueryAPI, SqliteExporter};
use engine_physics::{Collider, PhysicsConfig, PhysicsWorld, RigidBody};
use std::time::Instant;
use tempfile::NamedTempFile;

/// Benchmark: Snapshot Creation Overhead
///
/// Measures time to create debug snapshot at various entity counts.
/// Target: < 1ms for 1000 entities
fn bench_snapshot_creation_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("snapshot_creation_overhead");

    for &entity_count in &[1, 100, 1000, 10000] {
        group.throughput(Throughput::Elements(entity_count));

        group.bench_with_input(
            BenchmarkId::from_parameter(entity_count),
            &entity_count,
            |bench, &entity_count| {
                // Create world with entities
                let mut world = PhysicsWorld::new(PhysicsConfig::default());

                // Ground
                world.add_rigidbody(
                    0,
                    &RigidBody::static_body(),
                    Vec3::new(0.0, -1.0, 0.0),
                    Quat::IDENTITY,
                );
                world.add_collider(0, &Collider::box_collider(Vec3::new(100.0, 1.0, 100.0)));

                // Dynamic entities
                for i in 1..=entity_count {
                    let rb = RigidBody::dynamic(1.0);
                    world.add_rigidbody(
                        i,
                        &rb,
                        Vec3::new(
                            (i % 32) as f32 * 2.0,
                            5.0 + (i / 32) as f32,
                            ((i / 32) % 32) as f32 * 2.0,
                        ),
                        Quat::IDENTITY,
                    );
                    world.add_collider(i, &Collider::sphere(0.5));
                }

                // Step once to initialize physics
                world.step(1.0 / 60.0);

                let mut frame = 0u64;

                bench.iter(|| {
                    let snapshot = world.create_debug_snapshot(black_box(frame));
                    frame += 1;
                    black_box(snapshot);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: JSONL Export Throughput
///
/// Measures MB/sec throughput for JSONL export.
/// Target: > 10 MB/sec
fn bench_jsonl_export_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("jsonl_export_throughput");

    for &snapshot_count in &[100, 1000, 10000] {
        group.throughput(Throughput::Elements(snapshot_count));

        group.bench_with_input(
            BenchmarkId::from_parameter(snapshot_count),
            &snapshot_count,
            |bench, &snapshot_count| {
                // Create snapshots to export
                let mut world = PhysicsWorld::new(PhysicsConfig::default());

                // Add 100 entities
                for i in 0..100 {
                    let rb = RigidBody::dynamic(1.0);
                    world.add_rigidbody(i, &rb, Vec3::new(i as f32, 5.0, 0.0), Quat::IDENTITY);
                    world.add_collider(i, &Collider::sphere(0.5));
                }

                world.step(1.0 / 60.0);

                let snapshots: Vec<_> =
                    (0..snapshot_count).map(|i| world.create_debug_snapshot(i)).collect();

                bench.iter(|| {
                    let temp_file = NamedTempFile::new().unwrap();
                    let mut exporter = JsonlExporter::create(temp_file.path()).unwrap();

                    for snapshot in &snapshots {
                        exporter.write_snapshot(black_box(snapshot)).unwrap();
                    }

                    exporter.flush().unwrap();
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: SQLite Export Throughput
///
/// Measures frames/sec throughput for SQLite export.
/// Target: > 100 frames/sec
fn bench_sqlite_export_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("sqlite_export_throughput");

    for &snapshot_count in &[100, 1000, 10000] {
        group.throughput(Throughput::Elements(snapshot_count));

        group.bench_with_input(
            BenchmarkId::from_parameter(snapshot_count),
            &snapshot_count,
            |bench, &snapshot_count| {
                // Create snapshots to export
                let mut world = PhysicsWorld::new(PhysicsConfig::default());

                // Add 100 entities
                for i in 0..100 {
                    let rb = RigidBody::dynamic(1.0);
                    world.add_rigidbody(i, &rb, Vec3::new(i as f32, 5.0, 0.0), Quat::IDENTITY);
                    world.add_collider(i, &Collider::sphere(0.5));
                }

                world.step(1.0 / 60.0);

                let snapshots: Vec<_> =
                    (0..snapshot_count).map(|i| world.create_debug_snapshot(i)).collect();

                bench.iter(|| {
                    let temp_file = NamedTempFile::new().unwrap();
                    let mut exporter = SqliteExporter::create(temp_file.path()).unwrap();

                    for snapshot in &snapshots {
                        exporter.write_snapshot(black_box(snapshot)).unwrap();
                    }

                    // Note: Explicit commit via drop
                    drop(exporter);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: CSV Export Throughput
///
/// Measures MB/sec throughput for CSV export.
/// Target: > 5 MB/sec
fn bench_csv_export_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("csv_export_throughput");

    for &snapshot_count in &[100, 1000, 10000] {
        group.throughput(Throughput::Elements(snapshot_count));

        group.bench_with_input(
            BenchmarkId::from_parameter(snapshot_count),
            &snapshot_count,
            |bench, &snapshot_count| {
                // Create snapshots to export
                let mut world = PhysicsWorld::new(PhysicsConfig::default());

                // Add 100 entities
                for i in 0..100 {
                    let rb = RigidBody::dynamic(1.0);
                    world.add_rigidbody(i, &rb, Vec3::new(i as f32, 5.0, 0.0), Quat::IDENTITY);
                    world.add_collider(i, &Collider::sphere(0.5));
                }

                world.step(1.0 / 60.0);

                let snapshots: Vec<_> =
                    (0..snapshot_count).map(|i| world.create_debug_snapshot(i)).collect();

                bench.iter(|| {
                    let temp_file = NamedTempFile::new().unwrap();
                    let mut exporter = CsvExporter::create(temp_file.path()).unwrap();

                    for snapshot in &snapshots {
                        exporter.write_snapshot(black_box(snapshot)).unwrap();
                    }

                    exporter.flush().unwrap();
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: entity_history() Query Latency
///
/// Measures query performance for retrieving entity history from SQLite.
/// Target: < 10ms
fn bench_entity_history_query_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_history_query_latency");

    for &db_size in &[1000, 10000, 100000] {
        group.throughput(Throughput::Elements(db_size));

        group.bench_with_input(
            BenchmarkId::from_parameter(db_size),
            &db_size,
            |bench, &db_size| {
                // Create database with entity history
                let temp_file = NamedTempFile::new().unwrap();
                let path = temp_file.path();

                {
                    let mut exporter = SqliteExporter::create(path).unwrap();

                    let mut world = PhysicsWorld::new(PhysicsConfig::default());

                    // Add 10 entities
                    for i in 0..10 {
                        let rb = RigidBody::dynamic(1.0);
                        world.add_rigidbody(i, &rb, Vec3::new(i as f32, 5.0, 0.0), Quat::IDENTITY);
                        world.add_collider(i, &Collider::sphere(0.5));
                    }

                    // Simulate many frames
                    for frame in 0..db_size {
                        world.step(1.0 / 60.0);
                        let snapshot = world.create_debug_snapshot(frame);
                        exporter.write_snapshot(&snapshot).unwrap();
                    }
                }

                let api = PhysicsQueryAPI::open(path).unwrap();

                bench.iter(|| {
                    let history = api
                        .entity_history(black_box(5), black_box(0), black_box(db_size - 1))
                        .unwrap();
                    black_box(history);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: find_high_velocity() Query Latency
///
/// Measures query performance for finding high-velocity frames.
/// Target: < 10ms
fn bench_find_high_velocity_query_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("find_high_velocity_query_latency");

    for &db_size in &[1000, 10000, 100000] {
        group.throughput(Throughput::Elements(db_size));

        group.bench_with_input(
            BenchmarkId::from_parameter(db_size),
            &db_size,
            |bench, &db_size| {
                // Create database with velocity data
                let temp_file = NamedTempFile::new().unwrap();
                let path = temp_file.path();

                {
                    let mut exporter = SqliteExporter::create(path).unwrap();

                    let mut world = PhysicsWorld::new(PhysicsConfig::default());

                    // Add entity that will have varying velocity
                    let rb = RigidBody::dynamic(1.0);
                    world.add_rigidbody(1, &rb, Vec3::new(0.0, 10.0, 0.0), Quat::IDENTITY);
                    world.add_collider(1, &Collider::sphere(0.5));

                    // Simulate with occasional high-velocity frames
                    for frame in 0..db_size {
                        if frame % 100 == 0 {
                            world.apply_force(1, Vec3::new(0.0, 1000.0, 0.0));
                        }
                        world.step(1.0 / 60.0);
                        let snapshot = world.create_debug_snapshot(frame);
                        exporter.write_snapshot(&snapshot).unwrap();
                    }
                }

                let api = PhysicsQueryAPI::open(path).unwrap();

                bench.iter(|| {
                    let high_vel = api.find_high_velocity(black_box(1), black_box(50.0)).unwrap();
                    black_box(high_vel);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: events_by_type() Query Latency
///
/// Measures query performance for retrieving events by type.
/// Target: < 10ms
fn bench_events_by_type_query_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("events_by_type_query_latency");

    for &db_size in &[1000, 10000, 100000] {
        group.throughput(Throughput::Elements(db_size));

        group.bench_with_input(
            BenchmarkId::from_parameter(db_size),
            &db_size,
            |bench, &db_size| {
                // Create database with events
                let temp_file = NamedTempFile::new().unwrap();
                let path = temp_file.path();

                {
                    let mut exporter = SqliteExporter::create(path).unwrap();

                    let mut world = PhysicsWorld::new(PhysicsConfig::default());
                    world.enable_agentic_debug();

                    // Add entities that will collide
                    let rb = RigidBody::dynamic(1.0);
                    world.add_rigidbody(1, &rb, Vec3::new(0.0, 10.0, 0.0), Quat::IDENTITY);
                    world.add_collider(1, &Collider::sphere(0.5));

                    world.add_rigidbody(
                        2,
                        &RigidBody::static_body(),
                        Vec3::new(0.0, 0.0, 0.0),
                        Quat::IDENTITY,
                    );
                    world.add_collider(2, &Collider::box_collider(Vec3::new(10.0, 1.0, 10.0)));

                    // Simulate frames
                    for frame in 0..db_size {
                        world.step(1.0 / 60.0);
                        let snapshot = world.create_debug_snapshot(frame);
                        exporter.write_snapshot(&snapshot).unwrap();

                        // Export events
                        let events = world.event_recorder_mut().drain_events();
                        exporter.write_events(&events).unwrap();
                    }
                }

                let api = PhysicsQueryAPI::open(path).unwrap();

                bench.iter(|| {
                    let events = api
                        .events_by_type(
                            black_box("CollisionStart"),
                            black_box(0),
                            black_box(db_size - 1),
                        )
                        .unwrap_or_default();
                    black_box(events);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Total Overhead - Physics Step with Debugging
///
/// Compares physics step time with and without snapshot creation.
/// Target: < 5% overhead
fn bench_total_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("total_overhead");

    // Create representative world (1000 entities)
    fn create_world() -> PhysicsWorld {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());

        // Ground
        world.add_rigidbody(
            0,
            &RigidBody::static_body(),
            Vec3::new(0.0, -1.0, 0.0),
            Quat::IDENTITY,
        );
        world.add_collider(0, &Collider::box_collider(Vec3::new(100.0, 1.0, 100.0)));

        // 999 dynamic bodies
        for i in 1..1000 {
            let rb = RigidBody::dynamic(1.0);
            world.add_rigidbody(
                i,
                &rb,
                Vec3::new(
                    (i % 32) as f32 * 2.0,
                    5.0 + (i / 32) as f32,
                    ((i / 32) % 32) as f32 * 2.0,
                ),
                Quat::IDENTITY,
            );
            world.add_collider(i, &Collider::sphere(0.5));
        }

        world
    }

    // Baseline: Normal physics step without debugging
    group.bench_function("normal_physics_step", |bench| {
        let mut world = create_world();
        let dt = 1.0 / 60.0;

        bench.iter(|| {
            world.step(black_box(dt));
        });
    });

    // With debugging: Physics step + snapshot creation
    group.bench_function("physics_step_with_snapshot", |bench| {
        let mut world = create_world();
        let dt = 1.0 / 60.0;
        let mut frame = 0u64;

        bench.iter(|| {
            world.step(black_box(dt));
            let snapshot = world.create_debug_snapshot(frame);
            frame += 1;
            black_box(snapshot);
        });
    });

    // Calculate and display overhead percentage
    group.finish();

    // Additional benchmark to isolate just the overhead
    let mut overhead_group = c.benchmark_group("snapshot_overhead_only");

    overhead_group.bench_function("overhead_measurement", |bench| {
        let mut world = create_world();
        let dt = 1.0 / 60.0;

        // Warmup: run a few steps
        for _ in 0..10 {
            world.step(dt);
        }

        bench.iter(|| {
            let start = Instant::now();
            world.step(dt);
            let step_time = start.elapsed();

            let start = Instant::now();
            let snapshot = world.create_debug_snapshot(0);
            let snapshot_time = start.elapsed();

            black_box((step_time, snapshot_time, snapshot));
        });
    });

    overhead_group.finish();
}

/// Benchmark: Snapshot Creation with Realistic Physics Scenarios
///
/// Tests snapshot overhead in realistic scenarios:
/// - Falling objects (many active entities)
/// - Collisions (event recording)
/// - Sleeping entities (mixed active/sleeping)
fn bench_realistic_physics_scenarios(c: &mut Criterion) {
    let mut group = c.benchmark_group("realistic_scenarios");

    // Scenario 1: Falling objects (all entities active)
    group.bench_function("falling_objects_1000", |bench| {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());

        // Ground
        world.add_rigidbody(
            0,
            &RigidBody::static_body(),
            Vec3::new(0.0, -1.0, 0.0),
            Quat::IDENTITY,
        );
        world.add_collider(0, &Collider::box_collider(Vec3::new(100.0, 1.0, 100.0)));

        // Falling objects
        for i in 1..=1000 {
            let rb = RigidBody::dynamic(1.0);
            world.add_rigidbody(
                i,
                &rb,
                Vec3::new(
                    (i % 32) as f32 * 2.0,
                    10.0 + (i / 32) as f32 * 2.0,
                    ((i / 32) % 32) as f32 * 2.0,
                ),
                Quat::IDENTITY,
            );
            world.add_collider(i, &Collider::sphere(0.5));
        }

        let dt = 1.0 / 60.0;
        let mut frame = 0u64;

        bench.iter(|| {
            world.step(dt);
            let snapshot = world.create_debug_snapshot(frame);
            frame += 1;
            black_box(snapshot);
        });
    });

    // Scenario 2: Mostly sleeping entities (realistic MMO)
    group.bench_function("mostly_sleeping_10000", |bench| {
        let mut world = PhysicsWorld::new(PhysicsConfig::default());

        // Ground
        world.add_rigidbody(
            0,
            &RigidBody::static_body(),
            Vec3::new(0.0, -1.0, 0.0),
            Quat::IDENTITY,
        );
        world.add_collider(0, &Collider::box_collider(Vec3::new(200.0, 1.0, 200.0)));

        // Many entities on ground (will sleep)
        for i in 1..=10000 {
            let rb = RigidBody::dynamic(1.0);
            world.add_rigidbody(
                i,
                &rb,
                Vec3::new(
                    (i % 100) as f32 * 2.0 - 100.0,
                    0.6,
                    ((i / 100) % 100) as f32 * 2.0 - 100.0,
                ),
                Quat::IDENTITY,
            );
            world.add_collider(i, &Collider::sphere(0.5));
        }

        let dt = 1.0 / 60.0;

        // Let entities settle and sleep
        for _ in 0..120 {
            world.step(dt);
        }

        let mut frame = 0u64;

        bench.iter(|| {
            world.step(dt);
            let snapshot = world.create_debug_snapshot(frame);
            frame += 1;
            black_box(snapshot);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_snapshot_creation_overhead,
    bench_jsonl_export_throughput,
    bench_sqlite_export_throughput,
    bench_csv_export_throughput,
    bench_entity_history_query_latency,
    bench_find_high_velocity_query_latency,
    bench_events_by_type_query_latency,
    bench_total_overhead,
    bench_realistic_physics_scenarios,
);
criterion_main!(benches);
