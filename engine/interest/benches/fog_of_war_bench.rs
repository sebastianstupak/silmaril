//! Fog of War AAA-Quality Benchmarks
//!
//! Comprehensive performance benchmarks for Fog of War system:
//! - Core fog performance (visibility, LoS, team vision)
//! - Game-specific scenarios (RTS, Battle Royale, Stealth, MMO)
//! - Stress tests (worst case, rapid updates, massive scale)
//!
//! Performance Targets:
//! - <5ms fog update @ 1000 entities
//! - <10ms LoS checks for 1000 rays
//! - >95% cache hit rate
//! - <10 bytes per entity memory footprint

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_core::{Aabb, Entity, Vec3};
use engine_interest::fog_of_war::{EntityType, FogConfig, FogOfWar, StealthState, VisionRange};
use std::time::Duration;

// ============================================================================
// Core Fog Performance (6 benchmarks)
// ============================================================================

fn bench_fog_visibility_calculation(c: &mut Criterion) {
    let mut group = c.benchmark_group("fog_visibility_calculation");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(100);

    for entity_count in [100, 500, 1000, 5000, 10000].iter() {
        group.throughput(Throughput::Elements(*entity_count as u64));

        let mut fog = FogOfWar::new(FogConfig::default());

        // Register entities in a grid
        for i in 0..*entity_count {
            let entity = Entity::from_raw(i as u32);
            let x = ((i % 100) as f32) * 10.0;
            let z = ((i / 100) as f32) * 10.0;
            fog.register_entity(entity, Vec3::new(x, 0.0, z), i % 2, EntityType::Normal);
        }

        group.bench_with_input(BenchmarkId::from_parameter(entity_count), entity_count, |b, _| {
            b.iter(|| {
                let result = black_box(fog.calculate_fog_for_player(1, 0));
                black_box(result);
            });
        });
    }

    group.finish();
}

fn bench_fog_los_raycasting(c: &mut Criterion) {
    let mut group = c.benchmark_group("fog_los_raycasting");
    group.measurement_time(Duration::from_secs(10));

    for ray_count in [100, 500, 1000].iter() {
        group.throughput(Throughput::Elements(*ray_count as u64));

        let mut fog = FogOfWar::new(FogConfig::default());

        // Add obstacles
        for i in 0..20 {
            let x = (i as f32) * 50.0;
            fog.add_obstacle(Aabb::from_min_max(
                Vec3::new(x, -10.0, -10.0),
                Vec3::new(x + 10.0, 10.0, 10.0),
            ));
        }

        group.bench_with_input(BenchmarkId::from_parameter(ray_count), ray_count, |b, &count| {
            b.iter(|| {
                for i in 0..count {
                    let from = Vec3::ZERO;
                    let to = Vec3::new(
                        (i as f32) % 500.0,
                        ((i / 500) as f32) * 10.0,
                        ((i / 1000) as f32) * 10.0,
                    );
                    black_box(fog.check_line_of_sight(from, to));
                }
            });
        });
    }

    group.finish();
}

fn bench_fog_team_shared_vision(c: &mut Criterion) {
    let mut group = c.benchmark_group("fog_team_shared_vision");
    group.measurement_time(Duration::from_secs(10));

    for team_size in [2, 4, 8].iter() {
        let mut fog = FogOfWar::new(FogConfig::default());

        const TEAM_ID: u64 = 0;
        const ENTITIES_PER_MEMBER: usize = 100;

        // Register team members with entities
        for member_id in 0..*team_size {
            for entity_id in 0..ENTITIES_PER_MEMBER {
                let entity = Entity::from_raw((member_id * ENTITIES_PER_MEMBER + entity_id) as u32);
                let x = (member_id as f32) * 100.0 + ((entity_id % 10) as f32) * 5.0;
                let z = ((entity_id / 10) as f32) * 5.0;
                fog.register_entity(entity, Vec3::new(x, 0.0, z), TEAM_ID, EntityType::Normal);
            }
        }

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_members", team_size)),
            team_size,
            |b, _| {
                b.iter(|| {
                    let result = black_box(fog.calculate_fog_for_player(1, TEAM_ID));
                    black_box(result);
                });
            },
        );
    }

    group.finish();
}

fn bench_fog_update_moving_entities(c: &mut Criterion) {
    let mut group = c.benchmark_group("fog_update_moving_entities");
    group.measurement_time(Duration::from_secs(10));

    for moving_count in [10, 50, 100].iter() {
        group.throughput(Throughput::Elements(*moving_count as u64));

        let mut fog = FogOfWar::new(FogConfig::default());

        // Register entities
        for i in 0..*moving_count {
            let entity = Entity::from_raw(i as u32);
            fog.register_entity(
                entity,
                Vec3::new((i as f32) * 10.0, 0.0, 0.0),
                0,
                EntityType::Normal,
            );
        }

        group.bench_with_input(
            BenchmarkId::from_parameter(moving_count),
            moving_count,
            |b, &count| {
                b.iter(|| {
                    for i in 0..count {
                        let entity = Entity::from_raw(i as u32);
                        let old_pos = Vec3::new((i as f32) * 10.0, 0.0, 0.0);
                        let new_pos = Vec3::new((i as f32) * 10.0 + 5.0, 0.0, 0.0);
                        fog.update_entity_position(entity, old_pos, new_pos);
                    }
                });
            },
        );
    }

    group.finish();
}

fn bench_fog_cache_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("fog_cache_performance");
    group.measurement_time(Duration::from_secs(10));

    let mut fog = FogOfWar::new(FogConfig::default());

    // Add entities
    for i in 0..1000 {
        let entity = Entity::from_raw(i);
        fog.register_entity(
            entity,
            Vec3::new((i as f32) % 100.0, 0.0, (i as f32) / 100.0),
            i % 2,
            EntityType::Normal,
        );
    }

    // Add obstacles
    for i in 0..10 {
        fog.add_obstacle(Aabb::from_min_max(
            Vec3::new((i as f32) * 50.0, -5.0, -5.0),
            Vec3::new((i as f32) * 50.0 + 5.0, 5.0, 5.0),
        ));
    }

    group.bench_function("cache_hit_rate", |b| {
        b.iter(|| {
            // Repeated LoS checks should hit cache
            for _ in 0..100 {
                black_box(fog.check_line_of_sight(Vec3::ZERO, Vec3::new(100.0, 0.0, 0.0)));
            }
        });
    });

    group.finish();
}

fn bench_fog_memory_footprint(c: &mut Criterion) {
    let mut group = c.benchmark_group("fog_memory_footprint");
    group.measurement_time(Duration::from_secs(10));

    for entity_count in [1000, 10000, 100000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(entity_count),
            entity_count,
            |b, &count| {
                b.iter(|| {
                    let mut fog = FogOfWar::new(FogConfig::default());

                    for i in 0..count {
                        let entity = Entity::from_raw(i as u32);
                        let x = ((i % 100) as f32) * 10.0;
                        let z = ((i / 100) as f32) * 10.0;
                        fog.register_entity(
                            entity,
                            Vec3::new(x, 0.0, z),
                            i % 4,
                            EntityType::Normal,
                        );
                    }

                    black_box(fog);
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Game-Specific Benchmarks (8 benchmarks)
// ============================================================================

fn bench_rts_fog_100_units_per_player(c: &mut Criterion) {
    let mut group = c.benchmark_group("rts_fog_100_units_per_player");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(50);

    let mut fog = FogOfWar::new(FogConfig::default());

    const PLAYER_COUNT: usize = 4;
    const UNITS_PER_PLAYER: usize = 100;
    const TEAM_ID: u64 = 0;

    // Register units for 4 players
    for player_id in 0..PLAYER_COUNT {
        for unit_id in 0..UNITS_PER_PLAYER {
            let entity = Entity::from_raw((player_id * UNITS_PER_PLAYER + unit_id) as u32);
            let x = (player_id as f32) * 100.0 + ((unit_id % 10) as f32) * 5.0;
            let z = ((unit_id / 10) as f32) * 5.0;
            fog.register_entity(entity, Vec3::new(x, 0.0, z), TEAM_ID, EntityType::Normal);
        }
    }

    group.throughput(Throughput::Elements((PLAYER_COUNT * UNITS_PER_PLAYER) as u64));

    group.bench_function("4_players_100_units_each", |b| {
        b.iter(|| {
            let result = black_box(fog.calculate_fog_for_player(1, TEAM_ID));
            black_box(result);
        });
    });

    group.finish();
}

fn bench_battle_royale_100_players(c: &mut Criterion) {
    let mut group = c.benchmark_group("battle_royale_100_players");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(50);

    let mut fog = FogOfWar::new(FogConfig::default());

    const PLAYER_COUNT: usize = 100;

    // Spawn 100 players randomly
    for i in 0..PLAYER_COUNT {
        let player = Entity::from_raw(i as u32);
        let x = ((i % 10) as f32) * 100.0;
        let z = ((i / 10) as f32) * 100.0;
        fog.register_entity(player, Vec3::new(x, 0.0, z), i as u64, EntityType::Normal);
    }

    group.throughput(Throughput::Elements(PLAYER_COUNT as u64));

    group.bench_function("100_players_visibility", |b| {
        b.iter(|| {
            // Calculate fog for one player
            let result = black_box(fog.calculate_fog_for_player(1, 0));
            black_box(result);
        });
    });

    group.finish();
}

fn bench_stealth_game_guard_vision_cones(c: &mut Criterion) {
    let mut group = c.benchmark_group("stealth_game_guard_vision_cones");
    group.measurement_time(Duration::from_secs(10));

    for guard_count in [10, 25, 50].iter() {
        let mut fog = FogOfWar::new(FogConfig::default());

        // Spawn guards with vision cones
        for i in 0..*guard_count {
            let guard = Entity::from_raw(i as u32);
            let x = ((i % 10) as f32) * 20.0;
            let z = ((i / 10) as f32) * 20.0;
            fog.register_entity(guard, Vec3::new(x, 0.0, z), 0, EntityType::Normal);

            // Set directional vision
            let vision = VisionRange {
                base_range: 50.0,
                is_omnidirectional: false,
                cone_angle: std::f32::consts::PI / 2.0,
                facing: Vec3::new(1.0, 0.0, 0.0),
                ..Default::default()
            };
            fog.set_vision_range(guard, vision);
        }

        // Add player
        let player = Entity::new(1000, 0);
        fog.register_entity(player, Vec3::new(50.0, 0.0, 50.0), 1, EntityType::Stealth);

        group.throughput(Throughput::Elements(*guard_count as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_guards", guard_count)),
            guard_count,
            |b, _| {
                b.iter(|| {
                    let result = black_box(fog.calculate_fog_for_player(1000, 1));
                    black_box(result);
                });
            },
        );
    }

    group.finish();
}

fn bench_mmo_fog_1000_concurrent_players(c: &mut Criterion) {
    let mut group = c.benchmark_group("mmo_fog_1000_concurrent_players");
    group.measurement_time(Duration::from_secs(15));
    group.sample_size(30);

    let mut fog = FogOfWar::new(FogConfig::default());

    const PLAYER_COUNT: usize = 1000;

    // Spawn 1000 players across the world
    for i in 0..PLAYER_COUNT {
        let player = Entity::from_raw(i as u32);
        let x = ((i % 50) as f32) * 20.0;
        let z = ((i / 50) as f32) * 20.0;
        fog.register_entity(player, Vec3::new(x, 0.0, z), i % 10, EntityType::Normal);
    }

    group.throughput(Throughput::Elements(PLAYER_COUNT as u64));

    group.bench_function("1000_players_single_query", |b| {
        b.iter(|| {
            let result = black_box(fog.calculate_fog_for_player(1, 0));
            black_box(result);
        });
    });

    group.finish();
}

fn bench_fog_terrain_occlusion(c: &mut Criterion) {
    let mut group = c.benchmark_group("fog_terrain_occlusion");
    group.measurement_time(Duration::from_secs(10));

    for obstacle_count in [10, 50, 100].iter() {
        let mut fog = FogOfWar::new(FogConfig::default());

        // Add terrain obstacles (hills, buildings)
        for i in 0..*obstacle_count {
            let x = ((i % 10) as f32) * 50.0;
            let z = ((i / 10) as f32) * 50.0;
            fog.add_obstacle(Aabb::from_min_max(
                Vec3::new(x, 0.0, z),
                Vec3::new(x + 20.0, 15.0, z + 20.0),
            ));
        }

        // Add entities
        for i in 0..1000 {
            let entity = Entity::from_raw(i);
            fog.register_entity(
                entity,
                Vec3::new((i as f32) % 500.0, 0.0, (i as f32) / 500.0 * 50.0),
                i % 2,
                EntityType::Normal,
            );
        }

        group.throughput(Throughput::Elements(*obstacle_count as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_obstacles", obstacle_count)),
            obstacle_count,
            |b, _| {
                b.iter(|| {
                    let result = black_box(fog.calculate_fog_for_player(1, 0));
                    black_box(result);
                });
            },
        );
    }

    group.finish();
}

fn bench_fog_network_delta_compression(c: &mut Criterion) {
    let mut group = c.benchmark_group("fog_network_delta_compression");
    group.measurement_time(Duration::from_secs(10));

    let mut fog = FogOfWar::new(FogConfig::default());

    // Register entities
    for i in 0..1000 {
        let entity = Entity::from_raw(i);
        fog.register_entity(
            entity,
            Vec3::new((i as f32) % 100.0, 0.0, (i as f32) / 100.0 * 10.0),
            i % 2,
            EntityType::Normal,
        );
    }

    fog.set_time(0.0);

    group.bench_function("delta_calculation", |b| {
        b.iter(|| {
            // First calculation
            let result1 = fog.calculate_fog_for_player(1, 0);

            // Move some entities
            for i in 0..10 {
                let entity = Entity::from_raw(i);
                let old_pos = Vec3::new((i as f32) % 100.0, 0.0, (i as f32) / 100.0 * 10.0);
                let new_pos = old_pos + Vec3::new(5.0, 0.0, 0.0);
                fog.update_entity_position(entity, old_pos, new_pos);
            }

            // Second calculation (delta)
            let result2 = fog.calculate_fog_for_player(1, 0);

            // Delta is entered + exited
            let delta_size = result2.entered.len() + result2.exited.len();
            black_box(delta_size);
        });
    });

    group.finish();
}

fn bench_fog_prediction_client_side(c: &mut Criterion) {
    let mut group = c.benchmark_group("fog_prediction_client_side");
    group.measurement_time(Duration::from_secs(10));

    let mut fog = FogOfWar::new(FogConfig::default());

    // Add entities
    for i in 0..500 {
        let entity = Entity::from_raw(i);
        fog.register_entity(
            entity,
            Vec3::new((i as f32) % 50.0, 0.0, (i as f32) / 50.0 * 10.0),
            i % 2,
            EntityType::Normal,
        );
    }

    group.bench_function("client_prediction", |b| {
        b.iter(|| {
            // Client predicts entity movement
            for i in 0..10 {
                let entity = Entity::from_raw(i);
                let old_pos = Vec3::new((i as f32) % 50.0, 0.0, (i as f32) / 50.0 * 10.0);
                let predicted_pos = old_pos + Vec3::new(1.0, 0.0, 0.0);
                fog.update_entity_position(entity, old_pos, predicted_pos);
            }

            let result = black_box(fog.calculate_fog_for_player(1, 0));
            black_box(result);
        });
    });

    group.finish();
}

fn bench_fog_spatial_query_optimization(c: &mut Criterion) {
    let mut group = c.benchmark_group("fog_spatial_query_optimization");
    group.measurement_time(Duration::from_secs(10));

    // Compare different spatial partitioning strategies
    // For now, benchmark the current grid-based approach

    let mut fog = FogOfWar::new(FogConfig::default());

    for i in 0..10000 {
        let entity = Entity::from_raw(i);
        fog.register_entity(
            entity,
            Vec3::new((i as f32) % 100.0 * 10.0, 0.0, (i as f32) / 100.0 * 10.0),
            i % 2,
            EntityType::Normal,
        );
    }

    group.throughput(Throughput::Elements(10000));

    group.bench_function("grid_query_10k_entities", |b| {
        b.iter(|| {
            let result = black_box(fog.calculate_fog_for_player(1, 0));
            black_box(result);
        });
    });

    group.finish();
}

// ============================================================================
// Stress Tests (6 benchmarks)
// ============================================================================

fn bench_fog_worst_case_all_entities_visible(c: &mut Criterion) {
    let mut group = c.benchmark_group("fog_worst_case_all_visible");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(50);

    let mut fog = FogOfWar::new(FogConfig::default());

    const ENTITY_COUNT: usize = 1000;

    // All entities clustered together (worst case for bandwidth)
    for i in 0..ENTITY_COUNT {
        let entity = Entity::new(i as u32, 0);
        let offset = (i as f32) * 0.1; // Very close together
        fog.register_entity(entity, Vec3::new(offset, 0.0, 0.0), i % 2, EntityType::Normal);
    }

    // Player with huge vision range
    let player = Entity::new(10000, 0);
    fog.register_entity(player, Vec3::ZERO, 0, EntityType::Normal);
    let vision = VisionRange { base_range: 1000.0, ..Default::default() };
    fog.set_vision_range(player, vision);

    group.throughput(Throughput::Elements(ENTITY_COUNT as u64));

    group.bench_function("1000_entities_all_visible", |b| {
        b.iter(|| {
            let result = black_box(fog.calculate_fog_for_player(10000, 0));
            black_box(result);
        });
    });

    group.finish();
}

fn bench_fog_rapid_teleportation(c: &mut Criterion) {
    let mut group = c.benchmark_group("fog_rapid_teleportation");
    group.measurement_time(Duration::from_secs(10));

    let mut fog = FogOfWar::new(FogConfig::default());

    // Add entities
    for i in 0..1000 {
        let entity = Entity::from_raw(i);
        fog.register_entity(
            entity,
            Vec3::new((i as f32) % 100.0 * 10.0, 0.0, (i as f32) / 100.0 * 10.0),
            i % 2,
            EntityType::Normal,
        );
    }

    let player = Entity::new(10000, 0);
    fog.register_entity(player, Vec3::ZERO, 0, EntityType::Normal);

    group.bench_function("100_teleports_per_second", |b| {
        b.iter(|| {
            for i in 0..100 {
                let new_pos = Vec3::new((i as f32) * 10.0, 0.0, (i as f32) * 10.0);
                fog.update_entity_position(player, Vec3::ZERO, new_pos);
                black_box(fog.calculate_fog_for_player(10000, 0));
            }
        });
    });

    group.finish();
}

fn bench_fog_massive_entity_spawn(c: &mut Criterion) {
    let mut group = c.benchmark_group("fog_massive_entity_spawn");
    group.measurement_time(Duration::from_secs(10));

    for spawn_count in [100, 500, 1000].iter() {
        group.throughput(Throughput::Elements(*spawn_count as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("spawn_{}", spawn_count)),
            spawn_count,
            |b, &count| {
                b.iter(|| {
                    let mut fog = FogOfWar::new(FogConfig::default());

                    // Rapid entity spawning
                    for i in 0..count {
                        let entity = Entity::from_raw(i as u32);
                        fog.register_entity(
                            entity,
                            Vec3::new((i as f32) % 50.0, 0.0, (i as f32) / 50.0),
                            i % 2,
                            EntityType::Normal,
                        );
                    }

                    black_box(fog);
                });
            },
        );
    }

    group.finish();
}

fn bench_fog_entity_death_cleanup(c: &mut Criterion) {
    let mut group = c.benchmark_group("fog_entity_death_cleanup");
    group.measurement_time(Duration::from_secs(10));

    // Note: Current implementation doesn't have explicit despawn
    // This would measure the overhead of despawning logic when added
    let mut fog = FogOfWar::new(FogConfig::default());

    for i in 0..1000 {
        let entity = Entity::from_raw(i);
        fog.register_entity(entity, Vec3::new((i as f32) * 10.0, 0.0, 0.0), 0, EntityType::Normal);
    }

    group.bench_function("visibility_after_deaths", |b| {
        b.iter(|| {
            // In production, would despawn entities here
            // For now, measure fog calculation performance
            let result = black_box(fog.calculate_fog_for_player(1, 0));
            black_box(result);
        });
    });

    group.finish();
}

fn bench_fog_concurrent_updates(c: &mut Criterion) {
    let mut group = c.benchmark_group("fog_concurrent_updates");
    group.measurement_time(Duration::from_secs(10));

    // Note: FogOfWar is not currently thread-safe
    // This benchmark measures single-threaded performance
    // In production, would use Arc<Mutex<FogOfWar>> or similar

    let mut fog = FogOfWar::new(FogConfig::default());

    for i in 0..1000 {
        let entity = Entity::from_raw(i);
        fog.register_entity(
            entity,
            Vec3::new((i as f32) * 10.0, 0.0, 0.0),
            i % 4,
            EntityType::Normal,
        );
    }

    group.bench_function("sequential_updates", |b| {
        b.iter(|| {
            // Simulate multiple updates
            for i in 0..10 {
                let entity = Entity::from_raw(i);
                let old_pos = Vec3::new((i as f32) * 10.0, 0.0, 0.0);
                let new_pos = old_pos + Vec3::new(1.0, 0.0, 0.0);
                fog.update_entity_position(entity, old_pos, new_pos);
            }

            let result = black_box(fog.calculate_fog_for_player(1, 0));
            black_box(result);
        });
    });

    group.finish();
}

fn bench_fog_serialization_deserialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("fog_serialization");
    group.measurement_time(Duration::from_secs(10));

    // Note: FogOfWar doesn't currently implement serialization
    // This would measure serialization performance when implemented
    // For now, measure creation and population time

    for entity_count in [1000, 10000].iter() {
        group.throughput(Throughput::Elements(*entity_count as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_entities", entity_count)),
            entity_count,
            |b, &count| {
                b.iter(|| {
                    let mut fog = FogOfWar::new(FogConfig::default());

                    for i in 0..count {
                        let entity = Entity::from_raw(i as u32);
                        fog.register_entity(
                            entity,
                            Vec3::new((i as f32) % 100.0, 0.0, (i as f32) / 100.0),
                            i % 4,
                            EntityType::Normal,
                        );
                    }

                    black_box(fog);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    core_benches,
    bench_fog_visibility_calculation,
    bench_fog_los_raycasting,
    bench_fog_team_shared_vision,
    bench_fog_update_moving_entities,
    bench_fog_cache_performance,
    bench_fog_memory_footprint,
);

criterion_group!(
    game_benches,
    bench_rts_fog_100_units_per_player,
    bench_battle_royale_100_players,
    bench_stealth_game_guard_vision_cones,
    bench_mmo_fog_1000_concurrent_players,
    bench_fog_terrain_occlusion,
    bench_fog_network_delta_compression,
    bench_fog_prediction_client_side,
    bench_fog_spatial_query_optimization,
);

criterion_group!(
    stress_benches,
    bench_fog_worst_case_all_entities_visible,
    bench_fog_rapid_teleportation,
    bench_fog_massive_entity_spawn,
    bench_fog_entity_death_cleanup,
    bench_fog_concurrent_updates,
    bench_fog_serialization_deserialization,
);

criterion_main!(core_benches, game_benches, stress_benches);
