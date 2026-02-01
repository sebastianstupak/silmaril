//! Snapshot benchmarks for network state synchronization
//!
//! Measures performance of snapshot generation, serialization, and application
//! at different scales to validate network synchronization performance.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_core::ecs::World;
use engine_core::gameplay::Health;
use engine_core::math::{Transform, Vec3, Quat};
use engine_core::physics_components::Velocity;
use engine_core::serialization::Format;
use engine_networking::snapshot::WorldSnapshot;

/// Create a world with the specified number of entities
///
/// Each entity has Transform, Velocity, and Health components
fn create_test_world(entity_count: usize) -> World {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Velocity>();
    world.register::<Health>();

    for i in 0..entity_count {
        let entity = world.spawn();

        let pos_x = (i as f32) * 10.0;
        let pos_y = (i as f32) * 5.0;
        let pos_z = (i as f32) * 2.0;

        world.add(
            entity,
            Transform::new(
                Vec3::new(pos_x, pos_y, pos_z),
                Quat::IDENTITY,
                Vec3::ONE,
            ),
        );

        world.add(entity, Velocity { x: 1.0, y: 0.0, z: 0.0 });

        world.add(entity, Health::new(100.0, 100.0));
    }

    world
}

/// Benchmark snapshot generation at different scales
fn bench_snapshot_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("snapshot_generation");

    for entity_count in [100, 1_000, 10_000, 100_000].iter() {
        let world = create_test_world(*entity_count);

        group.throughput(Throughput::Elements(*entity_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(entity_count),
            entity_count,
            |b, _| {
                b.iter(|| {
                    let snapshot = WorldSnapshot::from_world(black_box(&world));
                    black_box(snapshot);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark snapshot serialization to bytes (Bincode)
fn bench_snapshot_serialization_bincode(c: &mut Criterion) {
    let mut group = c.benchmark_group("snapshot_serialization_bincode");

    for entity_count in [100, 1_000, 10_000, 100_000].iter() {
        let world = create_test_world(*entity_count);
        let snapshot = WorldSnapshot::from_world(&world);

        group.throughput(Throughput::Elements(*entity_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(entity_count),
            entity_count,
            |b, _| {
                b.iter(|| {
                    let bytes = snapshot.to_bytes(black_box(Format::Bincode)).unwrap();
                    black_box(bytes);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark snapshot deserialization from bytes (Bincode)
fn bench_snapshot_deserialization_bincode(c: &mut Criterion) {
    let mut group = c.benchmark_group("snapshot_deserialization_bincode");

    for entity_count in [100, 1_000, 10_000, 100_000].iter() {
        let world = create_test_world(*entity_count);
        let snapshot = WorldSnapshot::from_world(&world);
        let bytes = snapshot.to_bytes(Format::Bincode).unwrap();

        group.throughput(Throughput::Elements(*entity_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(entity_count),
            entity_count,
            |b, _| {
                b.iter(|| {
                    let snapshot = WorldSnapshot::from_bytes(black_box(&bytes), Format::Bincode)
                        .unwrap();
                    black_box(snapshot);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark snapshot application to world
fn bench_snapshot_application(c: &mut Criterion) {
    let mut group = c.benchmark_group("snapshot_application");

    for entity_count in [100, 1_000, 10_000, 100_000].iter() {
        let world = create_test_world(*entity_count);
        let snapshot = WorldSnapshot::from_world(&world);

        group.throughput(Throughput::Elements(*entity_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(entity_count),
            entity_count,
            |b, _| {
                b.iter(|| {
                    let mut target_world = World::new();
                    target_world.register::<Transform>();
                    target_world.register::<Velocity>();
                    target_world.register::<Health>();

                    snapshot.apply_to_world(black_box(&mut target_world));
                    black_box(target_world);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark snapshot size (measure bytes per entity)
fn bench_snapshot_size(c: &mut Criterion) {
    let group = c.benchmark_group("snapshot_size");

    // Just measure, don't benchmark timing
    for entity_count in [100, 1_000, 10_000, 100_000].iter() {
        let world = create_test_world(*entity_count);
        let snapshot = WorldSnapshot::from_world(&world);

        // Bincode size
        let bincode_bytes = snapshot.to_bytes(Format::Bincode).unwrap();
        let bincode_size = bincode_bytes.len();
        let bincode_per_entity = bincode_size as f64 / *entity_count as f64;

        println!(
            "Snapshot size for {} entities (Bincode): {} bytes ({:.2} bytes/entity)",
            entity_count, bincode_size, bincode_per_entity
        );

        // YAML size (for comparison)
        let yaml_bytes = snapshot.to_bytes(Format::Yaml).unwrap();
        let yaml_size = yaml_bytes.len();
        let yaml_per_entity = yaml_size as f64 / *entity_count as f64;

        println!(
            "Snapshot size for {} entities (YAML): {} bytes ({:.2} bytes/entity)",
            entity_count, yaml_size, yaml_per_entity
        );

        println!(
            "Compression ratio (YAML/Bincode): {:.2}x\n",
            yaml_size as f64 / bincode_size as f64
        );
    }

    group.finish();
}

/// Benchmark full roundtrip: World -> Snapshot -> Bytes -> Snapshot -> World
fn bench_snapshot_full_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("snapshot_full_roundtrip");

    for entity_count in [100, 1_000, 10_000, 100_000].iter() {
        let world = create_test_world(*entity_count);

        group.throughput(Throughput::Elements(*entity_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(entity_count),
            entity_count,
            |b, _| {
                b.iter(|| {
                    // World -> Snapshot
                    let snapshot1 = WorldSnapshot::from_world(black_box(&world));

                    // Snapshot -> Bytes
                    let bytes = snapshot1.to_bytes(Format::Bincode).unwrap();

                    // Bytes -> Snapshot
                    let snapshot2 = WorldSnapshot::from_bytes(&bytes, Format::Bincode).unwrap();

                    // Snapshot -> World
                    let mut target_world = World::new();
                    target_world.register::<Transform>();
                    target_world.register::<Velocity>();
                    target_world.register::<Health>();
                    snapshot2.apply_to_world(&mut target_world);

                    black_box(target_world);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark serialization throughput (MB/sec)
fn bench_snapshot_serialization_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("snapshot_serialization_throughput");

    for entity_count in [100, 1_000, 10_000, 100_000].iter() {
        let world = create_test_world(*entity_count);
        let snapshot = WorldSnapshot::from_world(&world);

        // Get size for throughput calculation
        let bytes = snapshot.to_bytes(Format::Bincode).unwrap();
        let size_bytes = bytes.len();

        group.throughput(Throughput::Bytes(size_bytes as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(entity_count),
            entity_count,
            |b, _| {
                b.iter(|| {
                    let bytes = snapshot.to_bytes(black_box(Format::Bincode)).unwrap();
                    black_box(bytes);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark deserialization throughput (MB/sec)
fn bench_snapshot_deserialization_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("snapshot_deserialization_throughput");

    for entity_count in [100, 1_000, 10_000, 100_000].iter() {
        let world = create_test_world(*entity_count);
        let snapshot = WorldSnapshot::from_world(&world);
        let bytes = snapshot.to_bytes(Format::Bincode).unwrap();
        let size_bytes = bytes.len();

        group.throughput(Throughput::Bytes(size_bytes as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(entity_count),
            entity_count,
            |b, _| {
                b.iter(|| {
                    let snapshot = WorldSnapshot::from_bytes(black_box(&bytes), Format::Bincode)
                        .unwrap();
                    black_box(snapshot);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_snapshot_generation,
    bench_snapshot_serialization_bincode,
    bench_snapshot_deserialization_bincode,
    bench_snapshot_application,
    bench_snapshot_size,
    bench_snapshot_full_roundtrip,
    bench_snapshot_serialization_throughput,
    bench_snapshot_deserialization_throughput,
);

criterion_main!(benches);
