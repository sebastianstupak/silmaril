//! FlatBuffers serialization benchmarks
//!
//! Compares FlatBuffers vs Bincode performance for WorldState serialization.
//! Target: Match or exceed bincode performance for zero-copy deserialization.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_core::ecs::World;
use engine_core::math::Transform;
use engine_core::serialization::{Format, Serializable, WorldState};

/// Create a world with N entities
fn create_world(entity_count: usize) -> World {
    let mut world = World::new();
    world.register::<Transform>();

    for i in 0..entity_count {
        let entity = world.spawn();
        let mut transform = Transform::identity();
        transform.position = engine_core::math::Vec3::new(i as f32, i as f32, i as f32);
        world.add(entity, transform);
    }

    world
}

/// Benchmark FlatBuffers serialization
fn bench_flatbuffers_serialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("flatbuffers_serialize");

    for &size in &[100, 1000, 10000] {
        let world = create_world(size);
        let state = WorldState::snapshot(&world);

        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), &state, |b, state| {
            b.iter(|| {
                let bytes = state.serialize(black_box(Format::FlatBuffers)).unwrap();
                black_box(bytes);
            });
        });
    }

    group.finish();
}

/// Benchmark FlatBuffers deserialization
fn bench_flatbuffers_deserialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("flatbuffers_deserialize");

    for &size in &[100, 1000, 10000] {
        let world = create_world(size);
        let state = WorldState::snapshot(&world);
        let bytes = state.serialize(Format::FlatBuffers).unwrap();

        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), &bytes, |b, bytes| {
            b.iter(|| {
                let state = WorldState::deserialize(black_box(bytes), Format::FlatBuffers).unwrap();
                black_box(state);
            });
        });
    }

    group.finish();
}

/// Benchmark Bincode vs FlatBuffers
fn bench_format_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("format_comparison");

    let size = 1000;
    let world = create_world(size);
    let state = WorldState::snapshot(&world);

    group.throughput(Throughput::Elements(size as u64));

    // Bincode serialize
    group.bench_function("bincode_serialize", |b| {
        b.iter(|| {
            let bytes = state.serialize(black_box(Format::Bincode)).unwrap();
            black_box(bytes);
        });
    });

    // FlatBuffers serialize
    group.bench_function("flatbuffers_serialize", |b| {
        b.iter(|| {
            let bytes = state.serialize(black_box(Format::FlatBuffers)).unwrap();
            black_box(bytes);
        });
    });

    // Bincode deserialize
    let bincode_bytes = state.serialize(Format::Bincode).unwrap();
    group.bench_function("bincode_deserialize", |b| {
        b.iter(|| {
            let state =
                WorldState::deserialize(black_box(&bincode_bytes), Format::Bincode).unwrap();
            black_box(state);
        });
    });

    // FlatBuffers deserialize
    let flatbuffers_bytes = state.serialize(Format::FlatBuffers).unwrap();
    group.bench_function("flatbuffers_deserialize", |b| {
        b.iter(|| {
            let state = WorldState::deserialize(black_box(&flatbuffers_bytes), Format::FlatBuffers)
                .unwrap();
            black_box(state);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_flatbuffers_serialize,
    bench_flatbuffers_deserialize,
    bench_format_comparison,
);
criterion_main!(benches);
