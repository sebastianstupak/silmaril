//! Real-world serialization scenario benchmarks
//!
//! Benchmarks based on actual game use cases:
//! - MMO player save/load
//! - Network state sync at 60 FPS
//! - Battle royale 100-player state
//! - RTS 10,000 unit state
//! - MOBA 5v5 match state

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_core::ecs::World;
use engine_core::math::Transform;
use engine_core::serialization::{
    ChecksumAlgorithm, CompressedData, CompressionAlgorithm, Format, OptimizedDelta, Serializable,
    ValidatedWorldState, VersionedWorldState, WorldState,
};

// ============================================================================
// Scenario Builders
// ============================================================================

/// Create MMO player world
///
/// Player entity with position, inventory (simulated with components)
fn create_mmo_player_world() -> World {
    let mut world = World::new();
    world.register::<Transform>();

    // Player entity
    let player = world.spawn();
    let mut transform = Transform::identity();
    transform.position = engine_core::math::Vec3::new(1234.56, 789.01, 2345.67);
    transform.rotation = engine_core::math::Quat::from_xyzw(0.0, 0.707, 0.0, 0.707);
    world.add(player, transform);

    world
}

/// Create network sync world (100 players)
///
/// Typical battle royale or MMO scenario
fn create_network_sync_world(player_count: usize) -> World {
    let mut world = World::new();
    world.register::<Transform>();

    for i in 0..player_count {
        let player = world.spawn();
        let mut transform = Transform::identity();

        // Spread players in grid
        let x = (i % 10) as f32 * 10.0;
        let z = (i / 10) as f32 * 10.0;
        transform.position = engine_core::math::Vec3::new(x, 0.0, z);

        world.add(player, transform);
    }

    world
}

/// Create RTS game world (10,000 units)
///
/// Real-time strategy game with many units
fn create_rts_world(unit_count: usize) -> World {
    let mut world = World::new();
    world.register::<Transform>();

    for i in 0..unit_count {
        let unit = world.spawn();
        let mut transform = Transform::identity();

        // Spread units across map
        let x = (i % 100) as f32;
        let z = (i / 100) as f32;
        transform.position = engine_core::math::Vec3::new(x, 0.0, z);

        world.add(unit, transform);
    }

    world
}

/// Create MOBA match world (5v5 + minions)
///
/// 10 players + 100 minions + 30 structures
fn create_moba_world() -> World {
    let mut world = World::new();
    world.register::<Transform>();

    // 10 heroes (5v5)
    for i in 0..10 {
        let hero = world.spawn();
        let mut transform = Transform::identity();
        let side = if i < 5 { -50.0 } else { 50.0 };
        transform.position = engine_core::math::Vec3::new(side, 0.0, (i % 5) as f32 * 10.0);
        world.add(hero, transform);
    }

    // 100 minions
    for i in 0..100 {
        let minion = world.spawn();
        let mut transform = Transform::identity();
        transform.position = engine_core::math::Vec3::new((i % 10) as f32, 0.0, (i / 10) as f32);
        world.add(minion, transform);
    }

    // 30 structures (towers, inhibitors, nexus)
    for i in 0..30 {
        let structure = world.spawn();
        let mut transform = Transform::identity();
        let side = if i < 15 { -100.0 } else { 100.0 };
        transform.position = engine_core::math::Vec3::new(side, 0.0, (i % 15) as f32 * 5.0);
        world.add(structure, transform);
    }

    world
}

// ============================================================================
// Scenario 1: MMO Player Save/Load
// ============================================================================

fn bench_scenario_mmo_player_save(c: &mut Criterion) {
    let mut group = c.benchmark_group("scenario_mmo_player_save");

    let world = create_mmo_player_world();
    let state = WorldState::snapshot(&world);

    // Full production pipeline
    group.bench_function("full_pipeline", |b| {
        b.iter(|| {
            // Version
            let versioned = VersionedWorldState::new(black_box(state.clone()));

            // Validate
            let validated = ValidatedWorldState::new(versioned.state, ChecksumAlgorithm::Xxh3);

            // Serialize
            let bytes = validated.save().unwrap();

            // Compress
            let compressed = CompressedData::compress(&bytes, CompressionAlgorithm::Zstd).unwrap();

            black_box(compressed);
        });
    });

    // Just serialize (baseline)
    group.bench_function("serialize_only", |b| {
        b.iter(|| {
            let bytes = state.serialize(black_box(Format::Bincode)).unwrap();
            black_box(bytes);
        });
    });

    group.finish();
}

fn bench_scenario_mmo_player_load(c: &mut Criterion) {
    let mut group = c.benchmark_group("scenario_mmo_player_load");

    let world = create_mmo_player_world();
    let state = WorldState::snapshot(&world);

    // Prepare saved data
    let versioned = VersionedWorldState::new(state);
    let validated = ValidatedWorldState::new(versioned.state, ChecksumAlgorithm::Xxh3);
    let bytes = validated.save().unwrap();
    let compressed = CompressedData::compress(&bytes, CompressionAlgorithm::Zstd).unwrap();

    // Full production pipeline
    group.bench_function("full_pipeline", |b| {
        b.iter(|| {
            // Decompress
            let decompressed = black_box(&compressed).decompress().unwrap();

            // Validate
            let loaded = ValidatedWorldState::load_validated(&decompressed).unwrap();

            // Restore
            let mut world2 = World::new();
            world2.register::<Transform>();
            loaded.state.restore(&mut world2);

            black_box(world2);
        });
    });

    group.finish();
}

// ============================================================================
// Scenario 2: Network State Sync (60 FPS)
// ============================================================================

fn bench_scenario_network_sync_60fps(c: &mut Criterion) {
    let mut group = c.benchmark_group("scenario_network_sync_60fps");

    for &player_count in &[10, 50, 100] {
        group.throughput(Throughput::Elements(player_count as u64));

        let mut world = create_network_sync_world(player_count);
        let state_t0 = WorldState::snapshot(&world);

        // Simulate 1 frame - 10% of players move
        let move_count = player_count / 10;
        let entities_to_move: Vec<_> = world.entities().take(move_count).collect();
        for entity in entities_to_move {
            if let Some(transform) = world.get_mut::<Transform>(entity) {
                transform.position.x += 1.0;
            }
        }

        let state_t1 = WorldState::snapshot(&world);

        // Benchmark delta + compress + send
        group.bench_with_input(
            BenchmarkId::new("delta_compress_send", player_count),
            &(state_t0.clone(), state_t1.clone()),
            |b, (s0, s1)| {
                b.iter(|| {
                    // Delta
                    let delta = OptimizedDelta::compute(black_box(s0), black_box(s1));

                    // Serialize
                    let delta_bytes = bincode::serialize(&delta).unwrap();

                    // Compress (LZ4 for network)
                    let compressed =
                        CompressedData::compress(&delta_bytes, CompressionAlgorithm::Lz4).unwrap();

                    black_box(compressed);
                });
            },
        );

        // Benchmark receive + decompress + apply
        let delta = OptimizedDelta::compute(&state_t0, &state_t1);
        let delta_bytes = bincode::serialize(&delta).unwrap();
        let compressed = CompressedData::compress(&delta_bytes, CompressionAlgorithm::Lz4).unwrap();

        group.bench_with_input(
            BenchmarkId::new("receive_decompress_apply", player_count),
            &(compressed, state_t0.clone()),
            |b, (comp, base)| {
                b.iter(|| {
                    // Decompress
                    let decompressed = black_box(comp).decompress().unwrap();

                    // Deserialize
                    let delta: OptimizedDelta = bincode::deserialize(&decompressed).unwrap();

                    // Apply
                    let mut state = base.clone();
                    delta.apply(&mut state);

                    black_box(state);
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Scenario 3: RTS Large World Persistence
// ============================================================================

fn bench_scenario_rts_persistence(c: &mut Criterion) {
    let mut group = c.benchmark_group("scenario_rts_persistence");
    group.sample_size(10); // Fewer samples for large data

    for &unit_count in &[1_000, 5_000, 10_000] {
        group.throughput(Throughput::Elements(unit_count as u64));

        let world = create_rts_world(unit_count);
        let state = WorldState::snapshot(&world);

        // Save (snapshot + compress)
        group.bench_with_input(BenchmarkId::new("save", unit_count), &state, |b, state| {
            b.iter(|| {
                // Snapshot already done, just serialize + compress
                let bytes = state.serialize(black_box(Format::Bincode)).unwrap();
                let compressed =
                    CompressedData::compress(&bytes, CompressionAlgorithm::Zstd).unwrap();
                black_box(compressed);
            });
        });

        // Load (decompress + deserialize + restore)
        let bytes = state.serialize(Format::Bincode).unwrap();
        let compressed = CompressedData::compress(&bytes, CompressionAlgorithm::Zstd).unwrap();

        group.bench_with_input(BenchmarkId::new("load", unit_count), &compressed, |b, comp| {
            b.iter(|| {
                let decompressed = black_box(comp).decompress().unwrap();
                let loaded = WorldState::deserialize(&decompressed, Format::Bincode).unwrap();

                let mut world2 = World::new();
                world2.register::<Transform>();
                loaded.restore(&mut world2);

                black_box(world2);
            });
        });
    }

    group.finish();
}

// ============================================================================
// Scenario 4: MOBA Match State
// ============================================================================

fn bench_scenario_moba_match(c: &mut Criterion) {
    let mut group = c.benchmark_group("scenario_moba_match");

    let world = create_moba_world();
    let state = WorldState::snapshot(&world);

    // Full state (for replay/reconnect)
    group.bench_function("full_state_serialize", |b| {
        b.iter(|| {
            let bytes = state.serialize(black_box(Format::Bincode)).unwrap();
            let compressed = CompressedData::compress(&bytes, CompressionAlgorithm::Lz4).unwrap();
            black_box(compressed);
        });
    });

    // Incremental update (combat happens)
    let mut world2 = create_moba_world();

    // Move 5 heroes + 20 minions (combat)
    let combat_entities: Vec<_> = world2.entities().take(25).collect();
    for entity in combat_entities {
        if let Some(transform) = world2.get_mut::<Transform>(entity) {
            transform.position.x += 2.0;
            transform.position.z += 1.5;
        }
    }

    let state2 = WorldState::snapshot(&world2);

    group.bench_function("incremental_update", |b| {
        b.iter(|| {
            let delta = OptimizedDelta::compute(black_box(&state), black_box(&state2));
            let delta_bytes = bincode::serialize(&delta).unwrap();
            let compressed =
                CompressedData::compress(&delta_bytes, CompressionAlgorithm::Lz4).unwrap();
            black_box(compressed);
        });
    });

    group.finish();
}

// ============================================================================
// Scenario 5: Bandwidth Analysis
// ============================================================================

fn bench_scenario_bandwidth_analysis(c: &mut Criterion) {
    let mut group = c.benchmark_group("scenario_bandwidth_analysis");

    // 100 player battle royale at 60 FPS
    let player_count = 100;
    let mut world = create_network_sync_world(player_count);
    let state_base = WorldState::snapshot(&world);

    // Test different change percentages
    for &change_percent in &[5, 10, 25, 50] {
        let move_count = (player_count * change_percent) / 100;

        // Apply changes
        let changing_entities: Vec<_> = world.entities().take(move_count).collect();
        for entity in changing_entities {
            if let Some(transform) = world.get_mut::<Transform>(entity) {
                transform.position.x += 1.0;
                transform.position.y += 0.5;
            }
        }

        let state_changed = WorldState::snapshot(&world);

        group.bench_with_input(
            BenchmarkId::new("bandwidth", format!("{}%_change", change_percent)),
            &(state_base.clone(), state_changed),
            |b, (base, changed)| {
                b.iter(|| {
                    // Full pipeline: delta + serialize + compress
                    let delta = OptimizedDelta::compute(black_box(base), black_box(changed));
                    let bytes = bincode::serialize(&delta).unwrap();
                    let compressed =
                        CompressedData::compress(&bytes, CompressionAlgorithm::Lz4).unwrap();

                    // Return bandwidth (bytes)
                    black_box(compressed.data.len());
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_scenario_mmo_player_save,
    bench_scenario_mmo_player_load,
    bench_scenario_network_sync_60fps,
    bench_scenario_rts_persistence,
    bench_scenario_moba_match,
    bench_scenario_bandwidth_analysis,
);
criterion_main!(benches);
