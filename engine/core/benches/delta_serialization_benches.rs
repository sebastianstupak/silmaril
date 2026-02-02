//! Delta serialization benchmarks
//!
//! Compares basic WorldStateDelta vs OptimizedDelta under realistic scenarios:
//! - 10% entity changes (typical frame)
//! - Position-only changes (common in games)
//! - Full state changes (worst case)
//! - Large worlds (10K+ entities)

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_core::ecs::World;
use engine_core::math::Transform;
use engine_core::serialization::{OptimizedDelta, WorldState, WorldStateDelta};

/// Create a world with N entities, each with Transform
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

/// Modify N% of entities (position only)
fn modify_entities_position(world: &mut World, percent: f32) {
    let entity_count = world.entity_count();
    let modify_count = ((entity_count as f32) * (percent / 100.0)) as usize;

    let entities: Vec<_> = world.entities().take(modify_count).collect();

    for entity in entities {
        if let Some(transform) = world.get_mut::<Transform>(entity) {
            let pos = transform.position;
            transform.position =
                engine_core::math::Vec3::new(pos.x + 1.0, pos.y + 1.0, pos.z + 1.0);
        }
    }
}

/// Benchmark basic delta computation
fn bench_basic_delta_compute(c: &mut Criterion) {
    let mut group = c.benchmark_group("delta_basic_compute");

    for &size in &[100, 1000, 5000, 10000] {
        for &change_percent in &[1.0, 10.0, 50.0] {
            let mut world1 = create_world(size);
            let state1 = WorldState::snapshot(&world1);

            modify_entities_position(&mut world1, change_percent);
            let state2 = WorldState::snapshot(&world1);

            group.throughput(Throughput::Elements(size as u64));

            group.bench_with_input(
                BenchmarkId::from_parameter(format!("{}ent_{}%", size, change_percent)),
                &(state1, state2),
                |b, (s1, s2)| {
                    b.iter(|| {
                        let delta = WorldStateDelta::compute(black_box(s1), black_box(s2));
                        black_box(delta);
                    });
                },
            );
        }
    }

    group.finish();
}

/// Benchmark optimized delta computation
fn bench_optimized_delta_compute(c: &mut Criterion) {
    let mut group = c.benchmark_group("delta_optimized_compute");

    for &size in &[100, 1000, 5000, 10000] {
        for &change_percent in &[1.0, 10.0, 50.0] {
            let mut world1 = create_world(size);
            let state1 = WorldState::snapshot(&world1);

            modify_entities_position(&mut world1, change_percent);
            let state2 = WorldState::snapshot(&world1);

            group.throughput(Throughput::Elements(size as u64));

            group.bench_with_input(
                BenchmarkId::from_parameter(format!("{}ent_{}%", size, change_percent)),
                &(state1, state2),
                |b, (s1, s2)| {
                    b.iter(|| {
                        let delta = OptimizedDelta::compute(black_box(s1), black_box(s2));
                        black_box(delta);
                    });
                },
            );
        }
    }

    group.finish();
}

/// Benchmark delta serialization size
fn bench_delta_size_comparison(c: &mut Criterion) {
    let group = c.benchmark_group("delta_size_comparison");

    for &size in &[1000, 10000] {
        for &change_percent in &[1.0, 10.0, 50.0] {
            let mut world1 = create_world(size);
            let state1 = WorldState::snapshot(&world1);

            modify_entities_position(&mut world1, change_percent);
            let state2 = WorldState::snapshot(&world1);

            // Compute deltas
            let basic_delta = WorldStateDelta::compute(&state1, &state2);
            let optimized_delta = OptimizedDelta::compute(&state1, &state2);

            // Serialize
            let full_size = bincode::serialize(&state2).unwrap().len();
            let basic_size = bincode::serialize(&basic_delta).unwrap().len();
            let optimized_size = bincode::serialize(&optimized_delta).unwrap().len();

            // Calculate ratios
            let basic_ratio = basic_size as f32 / full_size as f32;
            let optimized_ratio = optimized_size as f32 / full_size as f32;
            let improvement = (1.0 - (optimized_ratio / basic_ratio)) * 100.0;

            println!("\n{} entities, {}% changes:", size, change_percent);
            println!("  Full state:       {:>8} bytes", full_size);
            println!(
                "  Basic delta:      {:>8} bytes ({:.1}% of full)",
                basic_size,
                basic_ratio * 100.0
            );
            println!(
                "  Optimized delta:  {:>8} bytes ({:.1}% of full)",
                optimized_size,
                optimized_ratio * 100.0
            );
            println!("  Improvement:      {:.1}% smaller than basic", improvement);

            // Get stats
            let stats = optimized_delta.stats();
            println!(
                "  Changed entities: {}/{} ({:.1}%)",
                stats.changed_entities,
                stats.changed_entities + stats.unchanged_entities,
                stats.change_percentage()
            );
        }
    }

    group.finish();
}

/// Benchmark delta apply
fn bench_delta_apply(c: &mut Criterion) {
    let mut group = c.benchmark_group("delta_apply");

    for &size in &[100, 1000, 10000] {
        for &change_percent in &[10.0, 50.0] {
            let mut world1 = create_world(size);
            let state1 = WorldState::snapshot(&world1);

            modify_entities_position(&mut world1, change_percent);
            let state2 = WorldState::snapshot(&world1);

            // Basic delta
            let basic_delta = WorldStateDelta::compute(&state1, &state2);
            let state_copy = state1.clone();

            group.throughput(Throughput::Elements(size as u64));

            group.bench_with_input(
                BenchmarkId::from_parameter(format!("basic_{}ent_{}%", size, change_percent)),
                &(basic_delta, state_copy.clone()),
                |b, (delta, state)| {
                    b.iter(|| {
                        let mut s = state.clone();
                        delta.apply(black_box(&mut s));
                        black_box(s);
                    });
                },
            );

            // Optimized delta
            let optimized_delta = OptimizedDelta::compute(&state1, &state2);

            group.bench_with_input(
                BenchmarkId::from_parameter(format!("optimized_{}ent_{}%", size, change_percent)),
                &(optimized_delta, state_copy),
                |b, (delta, state)| {
                    b.iter(|| {
                        let mut s = state.clone();
                        delta.apply(black_box(&mut s));
                        black_box(s);
                    });
                },
            );
        }
    }

    group.finish();
}

/// Benchmark full serialization vs delta
fn bench_full_vs_delta(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_vs_delta_serialization");

    for &size in &[1000, 10000] {
        let mut world1 = create_world(size);
        let state1 = WorldState::snapshot(&world1);

        // 10% changes (typical frame)
        modify_entities_position(&mut world1, 10.0);
        let state2 = WorldState::snapshot(&world1);

        let optimized_delta = OptimizedDelta::compute(&state1, &state2);

        group.throughput(Throughput::Elements(size as u64));

        // Full serialization
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("full_{}ent", size)),
            &state2,
            |b, state| {
                b.iter(|| {
                    let bytes = bincode::serialize(black_box(state)).unwrap();
                    black_box(bytes);
                });
            },
        );

        // Delta serialization
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("delta_{}ent", size)),
            &optimized_delta,
            |b, delta| {
                b.iter(|| {
                    let bytes = bincode::serialize(black_box(delta)).unwrap();
                    black_box(bytes);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_basic_delta_compute,
    bench_optimized_delta_compute,
    bench_delta_size_comparison,
    bench_delta_apply,
    bench_full_vs_delta,
);
criterion_main!(benches);
