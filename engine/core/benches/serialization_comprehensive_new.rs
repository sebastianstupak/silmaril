//! Comprehensive serialization benchmarks
//!
//! Measures:
//! - Throughput (MB/s) for serialize/deserialize
//! - Large dataset performance (100K+ entities)
//! - Memory efficiency
//! - Compression ratios
//!
//! Run with: cargo bench --bench serialization_comprehensive_new

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_core::ecs::EntityAllocator;
use engine_core::gameplay::Health;
use engine_core::math::{Transform, Vec3};
use engine_core::physics_components::Velocity;
use engine_core::rendering::MeshRenderer;
use engine_core::serialization::{ComponentData, Format, Serializable, WorldState};

/// Create a realistic WorldState with mixed components
fn create_realistic_world_state(entity_count: usize) -> WorldState {
    let mut state = WorldState::new();
    let mut allocator = EntityAllocator::new();

    for i in 0..entity_count {
        let entity = allocator.allocate();
        let mut components = Vec::new();

        // All entities have Transform
        components.push(ComponentData::Transform(Transform::identity().with_position(Vec3::new(
            (i as f32) * 0.1,
            (i as f32) * 0.2,
            (i as f32) * 0.3,
        ))));

        // 80% have Health
        if i % 5 != 0 {
            components.push(ComponentData::Health(Health::new(100.0 - (i as f32 % 100.0), 100.0)));
        }

        // 60% have Velocity
        if i % 10 < 6 {
            components.push(ComponentData::Velocity(Velocity::new(
                (i as f32) * 0.1,
                (i as f32) * 0.2,
                (i as f32) * 0.3,
            )));
        }

        // 30% have MeshRenderer
        if i % 10 < 3 {
            components
                .push(ComponentData::MeshRenderer(MeshRenderer::new(i as u64, i as u64 + 1000)));
        }

        state.entities.push(engine_core::serialization::EntityMetadata {
            entity,
            generation: entity.generation(),
            alive: true,
        });
        state.components.insert(entity, components);
    }

    state.metadata.entity_count = entity_count;
    state.metadata.component_count = state.components.values().map(|v| v.len()).sum();
    state.metadata.version = 1;
    state.metadata.timestamp = 0;

    state
}

/// Benchmark serialization throughput (MB/s)
fn bench_serialization_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialization_throughput");

    // Test with various dataset sizes
    for &size in &[100, 1000, 10_000, 100_000] {
        let state = create_realistic_world_state(size);

        // Measure bincode serialization
        let serialized = state.serialize(Format::Bincode).unwrap();
        let bytes_size = serialized.len() as u64;

        group.throughput(Throughput::Bytes(bytes_size));
        group.bench_with_input(BenchmarkId::new("bincode_serialize", size), &size, |b, _| {
            b.iter(|| {
                let bytes = black_box(&state).serialize(Format::Bincode).unwrap();
                black_box(bytes);
            });
        });

        // Measure bincode deserialization
        group.throughput(Throughput::Bytes(bytes_size));
        group.bench_with_input(BenchmarkId::new("bincode_deserialize", size), &size, |b, _| {
            b.iter(|| {
                let state =
                    WorldState::deserialize(black_box(&serialized), Format::Bincode).unwrap();
                black_box(state);
            });
        });
    }

    group.finish();
}

/// Benchmark component-dense vs sparse worlds
fn bench_component_density(c: &mut Criterion) {
    let mut group = c.benchmark_group("component_density");

    let entity_count = 10_000;

    // Dense: All entities have all components
    let mut dense_state = WorldState::new();
    let mut allocator = EntityAllocator::new();

    for i in 0..entity_count {
        let entity = allocator.allocate();
        let components = vec![
            ComponentData::Transform(Transform::default()),
            ComponentData::Health(Health::new(100.0, 100.0)),
            ComponentData::Velocity(Velocity::default()),
            ComponentData::MeshRenderer(MeshRenderer::new(i as u64, i as u64)),
        ];

        dense_state.entities.push(engine_core::serialization::EntityMetadata {
            entity,
            generation: entity.generation(),
            alive: true,
        });
        dense_state.components.insert(entity, components);
    }

    dense_state.metadata.entity_count = entity_count;
    dense_state.metadata.component_count = dense_state.components.values().map(|v| v.len()).sum();

    // Sparse: Entities have 1-2 components each
    let mut sparse_state = WorldState::new();
    let mut allocator2 = EntityAllocator::new();

    for i in 0..entity_count {
        let entity = allocator2.allocate();
        let mut components = vec![ComponentData::Transform(Transform::default())];

        if i % 3 == 0 {
            components.push(ComponentData::Health(Health::new(100.0, 100.0)));
        }

        sparse_state.entities.push(engine_core::serialization::EntityMetadata {
            entity,
            generation: entity.generation(),
            alive: true,
        });
        sparse_state.components.insert(entity, components);
    }

    sparse_state.metadata.entity_count = entity_count;
    sparse_state.metadata.component_count = sparse_state.components.values().map(|v| v.len()).sum();

    // Benchmark dense
    let dense_bytes = dense_state.serialize(Format::Bincode).unwrap();
    group.throughput(Throughput::Bytes(dense_bytes.len() as u64));
    group.bench_function("dense_serialize", |b| {
        b.iter(|| {
            let bytes = black_box(&dense_state).serialize(Format::Bincode).unwrap();
            black_box(bytes);
        });
    });

    group.throughput(Throughput::Bytes(dense_bytes.len() as u64));
    group.bench_function("dense_deserialize", |b| {
        b.iter(|| {
            let state = WorldState::deserialize(black_box(&dense_bytes), Format::Bincode).unwrap();
            black_box(state);
        });
    });

    // Benchmark sparse
    let sparse_bytes = sparse_state.serialize(Format::Bincode).unwrap();
    group.throughput(Throughput::Bytes(sparse_bytes.len() as u64));
    group.bench_function("sparse_serialize", |b| {
        b.iter(|| {
            let bytes = black_box(&sparse_state).serialize(Format::Bincode).unwrap();
            black_box(bytes);
        });
    });

    group.throughput(Throughput::Bytes(sparse_bytes.len() as u64));
    group.bench_function("sparse_deserialize", |b| {
        b.iter(|| {
            let state = WorldState::deserialize(black_box(&sparse_bytes), Format::Bincode).unwrap();
            black_box(state);
        });
    });

    group.finish();
}

/// Benchmark serialization with varying entity counts
fn bench_scalability(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialization_scalability");

    for &size in &[10, 100, 1_000, 10_000, 50_000] {
        let state = create_realistic_world_state(size);
        let bytes = state.serialize(Format::Bincode).unwrap();

        // Entities per second
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::new("entities_per_sec", size), &size, |b, _| {
            b.iter(|| {
                let bytes = black_box(&state).serialize(Format::Bincode).unwrap();
                black_box(bytes);
            });
        });

        // Bytes per second
        group.throughput(Throughput::Bytes(bytes.len() as u64));
        group.bench_with_input(BenchmarkId::new("bytes_per_sec", size), &size, |b, _| {
            b.iter(|| {
                let bytes = black_box(&state).serialize(Format::Bincode).unwrap();
                black_box(bytes);
            });
        });
    }

    group.finish();
}

/// Benchmark memory efficiency
fn bench_memory_efficiency(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_efficiency");

    for &size in &[1_000, 10_000, 100_000] {
        let state = create_realistic_world_state(size);
        let bytes = state.serialize(Format::Bincode).unwrap();

        let bytes_per_entity = bytes.len() as f64 / size as f64;
        let components_per_entity = state.metadata.component_count as f64 / size as f64;

        println!(
            "\n{} entities: {} bytes total, {:.2} bytes/entity, {:.2} components/entity",
            size,
            bytes.len(),
            bytes_per_entity,
            components_per_entity
        );

        group.throughput(Throughput::Bytes(bytes.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| {
                let bytes = black_box(&state).serialize(Format::Bincode).unwrap();
                black_box(bytes);
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_serialization_throughput,
    bench_component_density,
    bench_scalability,
    bench_memory_efficiency,
);

criterion_main!(benches);
