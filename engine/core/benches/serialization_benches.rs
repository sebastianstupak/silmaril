//! Serialization performance benchmarks
//!
//! Benchmarks for Phase 1.3 serialization functionality including:
//! - WorldState serialization (YAML, Bincode)
//! - Delta compression
//! - Component serialization
//!
//! Run with: cargo bench --bench serialization_benches

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_core::ecs::{Entity, EntityAllocator};
use engine_core::gameplay::Health;
use engine_core::math::Transform;
use engine_core::physics_components::Velocity;
use engine_core::rendering::MeshRenderer;
use engine_core::serialization::{
    ComponentData, Format, Serializable, WorldState, WorldStateDelta,
};

// Helper to create a WorldState with N entities
fn create_world_state_with_entities(count: usize) -> WorldState {
    let mut state = WorldState::new();
    let mut allocator = EntityAllocator::new();

    for i in 0..count {
        let entity = allocator.allocate();
        let mut components = Vec::new();

        // Add Transform to all entities
        components.push(ComponentData::Transform(Transform::default()));

        // Add Health to all entities
        components.push(ComponentData::Health(Health::new(100.0 - (i as f32 % 100.0), 100.0)));

        // Add Velocity to half the entities
        if i % 2 == 0 {
            components.push(ComponentData::Velocity(Velocity::new(
                (i as f32) * 0.1,
                (i as f32) * 0.2,
                (i as f32) * 0.3,
            )));
        }

        // Add MeshRenderer to 1/3 of entities
        if i % 3 == 0 {
            components.push(ComponentData::MeshRenderer(MeshRenderer::new(i as u64)));
        }

        state.entities.push(engine_core::serialization::EntityMetadata {
            entity,
            generation: entity.generation(),
            alive: true,
        });
        state.components.insert(entity, components);
    }

    state.metadata.entity_count = count;
    state.metadata.component_count = state.components.values().map(|v| v.len()).sum();
    state
}

fn bench_yaml_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("yaml_serialization");

    for size in [10, 100, 1000].iter() {
        let state = create_world_state_with_entities(*size);
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let bytes = Serializable::serialize(black_box(&state), Format::Yaml).unwrap();
                black_box(bytes);
            });
        });
    }

    group.finish();
}

fn bench_yaml_deserialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("yaml_deserialization");

    for size in [10, 100, 1000].iter() {
        let state = create_world_state_with_entities(*size);
        let bytes = Serializable::serialize(&state, Format::Yaml).unwrap();
        group.throughput(Throughput::Bytes(bytes.len() as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let restored =
                    <WorldState as Serializable>::deserialize(black_box(&bytes), Format::Yaml)
                        .unwrap();
                black_box(restored);
            });
        });
    }

    group.finish();
}

fn bench_bincode_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("bincode_serialization");

    for size in [10, 100, 1000, 10000].iter() {
        let state = create_world_state_with_entities(*size);
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let bytes = Serializable::serialize(black_box(&state), Format::Bincode).unwrap();
                black_box(bytes);
            });
        });
    }

    group.finish();
}

fn bench_bincode_deserialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("bincode_deserialization");

    for size in [10, 100, 1000, 10000].iter() {
        let state = create_world_state_with_entities(*size);
        let bytes = Serializable::serialize(&state, Format::Bincode).unwrap();
        group.throughput(Throughput::Bytes(bytes.len() as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let restored =
                    <WorldState as Serializable>::deserialize(black_box(&bytes), Format::Bincode)
                        .unwrap();
                black_box(restored);
            });
        });
    }

    group.finish();
}

fn bench_delta_compute(c: &mut Criterion) {
    let mut group = c.benchmark_group("delta_compute");

    for size in [10, 100, 1000, 10000].iter() {
        let old_state = create_world_state_with_entities(*size);
        let mut new_state = old_state.clone();

        // Modify 10% of entities
        let modifications = (size / 10).max(1);
        let entities_to_modify: Vec<Entity> =
            new_state.components.keys().take(modifications).copied().collect();
        for entity in entities_to_modify {
            if let Some(components) = new_state.components.get_mut(&entity) {
                for comp in components.iter_mut() {
                    if let ComponentData::Health(ref mut health) = comp {
                        health.current -= 10.0;
                    }
                }
            }
        }

        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let delta = WorldStateDelta::compute(black_box(&old_state), black_box(&new_state));
                black_box(delta);
            });
        });
    }

    group.finish();
}

fn bench_delta_apply(c: &mut Criterion) {
    let mut group = c.benchmark_group("delta_apply");

    for size in [10, 100, 1000, 10000].iter() {
        let old_state = create_world_state_with_entities(*size);
        let mut new_state = old_state.clone();

        // Modify 10% of entities
        let modifications = (size / 10).max(1);
        let entities_to_modify: Vec<Entity> =
            new_state.components.keys().take(modifications).copied().collect();
        for entity in entities_to_modify {
            if let Some(components) = new_state.components.get_mut(&entity) {
                for comp in components.iter_mut() {
                    if let ComponentData::Health(ref mut health) = comp {
                        health.current -= 10.0;
                    }
                }
            }
        }

        let delta = WorldStateDelta::compute(&old_state, &new_state);
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let mut base = old_state.clone();
                black_box(&delta).apply(&mut base);
                black_box(base);
            });
        });
    }

    group.finish();
}

fn bench_delta_vs_full_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("delta_vs_full_comparison");

    for modification_percent in [1, 10, 50, 100].iter() {
        let size = 1000;
        let old_state = create_world_state_with_entities(size);
        let mut new_state = old_state.clone();

        // Modify specified percentage of entities
        let modifications = ((size * modification_percent) / 100).max(1);
        let entities_to_modify: Vec<Entity> =
            new_state.components.keys().take(modifications).copied().collect();
        for entity in entities_to_modify {
            if let Some(components) = new_state.components.get_mut(&entity) {
                for comp in components.iter_mut() {
                    if let ComponentData::Health(ref mut health) = comp {
                        health.current -= 10.0;
                    }
                }
            }
        }

        let delta = WorldStateDelta::compute(&old_state, &new_state);
        let delta_size = bincode::serialize(&delta).unwrap().len();
        let full_size = bincode::serialize(&new_state).unwrap().len();

        group.bench_with_input(
            BenchmarkId::new("delta_serialization", modification_percent),
            modification_percent,
            |b, _| {
                b.iter(|| {
                    let bytes = bincode::serialize(black_box(&delta)).unwrap();
                    black_box(bytes);
                });
            },
        );

        // Print size comparison
        println!(
            "{}% modified - Delta: {} bytes, Full: {} bytes, Ratio: {:.2}%",
            modification_percent,
            delta_size,
            full_size,
            (delta_size as f64 / full_size as f64) * 100.0
        );
    }

    group.finish();
}

fn bench_component_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("component_serialization");

    let transform = ComponentData::Transform(Transform::default());
    let health = ComponentData::Health(Health::new(75.0, 100.0));
    let velocity = ComponentData::Velocity(Velocity::new(1.0, 2.0, 3.0));
    let mesh_renderer = ComponentData::MeshRenderer(MeshRenderer::new(123));

    group.bench_function("transform", |b| {
        b.iter(|| {
            let bytes = bincode::serialize(black_box(&transform)).unwrap();
            black_box(bytes);
        });
    });

    group.bench_function("health", |b| {
        b.iter(|| {
            let bytes = bincode::serialize(black_box(&health)).unwrap();
            black_box(bytes);
        });
    });

    group.bench_function("velocity", |b| {
        b.iter(|| {
            let bytes = bincode::serialize(black_box(&velocity)).unwrap();
            black_box(bytes);
        });
    });

    group.bench_function("mesh_renderer", |b| {
        b.iter(|| {
            let bytes = bincode::serialize(black_box(&mesh_renderer)).unwrap();
            black_box(bytes);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_yaml_serialization,
    bench_yaml_deserialization,
    bench_bincode_serialization,
    bench_bincode_deserialization,
    bench_delta_compute,
    bench_delta_apply,
    bench_delta_vs_full_size,
    bench_component_serialization,
);

criterion_main!(benches);
