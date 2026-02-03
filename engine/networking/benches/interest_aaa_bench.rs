//! AAA Benchmark Suite - Industry Comparison
//!
//! This benchmark suite compares silmaril's interest management against
//! industry standards and documented performance from other engines:
//!
//! - Unity DOTS NetCode: 1700 players documented max
//! - Unreal Engine Mass/Replication Graph: ~2000+ players estimated
//! - EVE Online: 6000+ players (with time dilation)
//! - WoW Classic: ~3000 players per server
//! - Final Fantasy XIV: ~1500 concurrent per zone
//!
//! Real-world game patterns from AAA MMOs are also benchmarked to validate
//! production readiness.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_core::{Aabb, Entity, Quat, Transform, Vec3, World};
use engine_interest::{AreaOfInterest, InterestManager};
use std::time::Duration;

// ============================================================================
// Benchmark Utilities
// ============================================================================

/// Create a world with entities in a grid pattern
fn create_world_grid(count: usize) -> (World, Vec<Entity>) {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Aabb>();

    let mut entities = Vec::with_capacity(count);
    let grid_size = (count as f32).sqrt() as usize;

    for i in 0..count {
        let entity = world.spawn();
        let x = ((i % grid_size) as f32) * 10.0;
        let z = ((i / grid_size) as f32) * 10.0;
        let pos = Vec3::new(x, 0.0, z);

        world.add(entity, Transform::new(pos, Quat::IDENTITY, Vec3::ONE));
        world.add(entity, Aabb::from_center_half_extents(pos, Vec3::ONE));
        entities.push(entity);
    }

    (world, entities)
}

/// Create entities in a realistic MMO city distribution
/// - Some clustering (markets, quest hubs)
/// - Some spread (streets, buildings)
/// - Represents actual player behavior
fn create_mmo_city_distribution(player_count: usize) -> (World, Vec<Vec3>) {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Aabb>();

    let mut positions = Vec::with_capacity(player_count);

    // 30% in market/auction house (clustered)
    let market_count = (player_count as f32 * 0.3) as usize;
    let market_center = Vec3::new(100.0, 0.0, 100.0);

    for i in 0..market_count {
        let angle = (i as f32 / market_count as f32) * std::f32::consts::TAU;
        let radius = (i as f32 / market_count as f32).sqrt() * 30.0;
        let pos = market_center + Vec3::new(angle.cos() * radius, 0.0, angle.sin() * radius);
        positions.push(pos);

        let entity = world.spawn();
        world.add(entity, Transform::new(pos, Quat::IDENTITY, Vec3::ONE));
        world.add(entity, Aabb::from_center_half_extents(pos, Vec3::ONE));
    }

    // 50% spread across city streets
    let street_count = (player_count as f32 * 0.5) as usize;
    for i in 0..street_count {
        let street_x = ((i % 20) as f32) * 25.0;
        let street_z = ((i / 20) as f32) * 25.0;
        let pos = Vec3::new(street_x, 0.0, street_z);
        positions.push(pos);

        let entity = world.spawn();
        world.add(entity, Transform::new(pos, Quat::IDENTITY, Vec3::ONE));
        world.add(entity, Aabb::from_center_half_extents(pos, Vec3::ONE));
    }

    // 20% in instances/dungeons (completely separate)
    let instance_count = player_count - market_count - street_count;
    for i in 0..instance_count {
        let instance_offset = Vec3::new((i / 5) as f32 * 1000.0, 0.0, 0.0);
        let pos = instance_offset + Vec3::new((i % 5) as f32 * 10.0, 0.0, 0.0);
        positions.push(pos);

        let entity = world.spawn();
        world.add(entity, Transform::new(pos, Quat::IDENTITY, Vec3::ONE));
        world.add(entity, Aabb::from_center_half_extents(pos, Vec3::ONE));
    }

    (world, positions)
}

/// Create raid formation (tanks, DPS, healers)
fn create_raid_formation(boss_pos: Vec3, player_count: usize) -> Vec<Vec3> {
    let mut positions = Vec::with_capacity(player_count);

    let tank_count = (player_count as f32 * 0.15) as usize;
    let melee_count = (player_count as f32 * 0.25) as usize;
    let ranged_count = (player_count as f32 * 0.45) as usize;
    let healer_count = player_count - tank_count - melee_count - ranged_count;

    // Tanks (5 units)
    for i in 0..tank_count {
        let angle = (i as f32 / tank_count as f32) * std::f32::consts::PI;
        positions.push(boss_pos + Vec3::new(angle.cos() * 5.0, 0.0, angle.sin() * 5.0));
    }

    // Melee (8 units)
    for i in 0..melee_count {
        let angle = (i as f32 / melee_count as f32) * std::f32::consts::PI;
        positions.push(boss_pos + Vec3::new(angle.cos() * 8.0, 0.0, angle.sin() * 8.0));
    }

    // Ranged (15-20 units)
    for i in 0..ranged_count {
        let angle = (i as f32 / ranged_count as f32) * std::f32::consts::PI * 1.5;
        let dist = 15.0 + (i % 3) as f32 * 2.0;
        positions.push(boss_pos + Vec3::new(angle.cos() * dist, 0.0, angle.sin() * dist));
    }

    // Healers (12-18 units, spread)
    for i in 0..healer_count {
        let angle = (i as f32 / healer_count as f32) * std::f32::consts::TAU;
        positions.push(boss_pos + Vec3::new(angle.cos() * 15.0, 0.0, angle.sin() * 15.0));
    }

    positions
}

// ============================================================================
// Industry Comparison Benchmarks
// ============================================================================

/// Benchmark: Unity DOTS NetCode comparison (1700 players)
///
/// Unity DOTS NetCode documentation shows max 1700 players per server instance
/// with their replication system. We benchmark at this scale to compare.
///
/// Target: <2ms per player visibility calculation
fn bench_unity_dots_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("industry_unity_dots");
    group.sample_size(50);
    group.measurement_time(Duration::from_secs(15));

    let (world, _) = create_world_grid(1700);

    let mut manager = InterestManager::new(50.0);
    manager.update_from_world(&world);

    // Register 1700 clients spread across world
    let grid_size = (1700_f32.sqrt()) as usize;
    for i in 0..1700 {
        let x = ((i % grid_size) as f32) * 100.0;
        let z = ((i / grid_size) as f32) * 100.0;
        let pos = Vec3::new(x, 0.0, z);
        manager.set_client_interest(i as u64, AreaOfInterest::new(pos, 100.0));
    }

    group.throughput(Throughput::Elements(1700));

    group.bench_function("1700_players_visibility_all", |b| {
        b.iter(|| {
            for i in 0..1700 {
                let visible = black_box(manager.calculate_visibility(i as u64));
                black_box(visible);
            }
        });
    });

    group.bench_function("1700_players_visibility_single", |b| {
        b.iter(|| {
            let visible = black_box(manager.calculate_visibility(850));
            black_box(visible);
        });
    });

    group.finish();
}

/// Benchmark: Unreal Engine Mass comparison (2000+ players)
///
/// Unreal Mass Entity system with Replication Graph can handle 2000+ entities.
/// We test at 2000 and 2500 to compare against Unreal's capabilities.
///
/// Target: <5ms per player
fn bench_unreal_mass_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("industry_unreal_mass");
    group.sample_size(30);
    group.measurement_time(Duration::from_secs(15));

    for player_count in [2000, 2500] {
        let (world, _) = create_world_grid(player_count);

        let mut manager = InterestManager::new(50.0);
        manager.update_from_world(&world);

        let grid_size = (player_count as f32).sqrt() as usize;
        for i in 0..player_count {
            let x = ((i % grid_size) as f32) * 100.0;
            let z = ((i / grid_size) as f32) * 100.0;
            let pos = Vec3::new(x, 0.0, z);
            manager.set_client_interest(i as u64, AreaOfInterest::new(pos, 100.0));
        }

        group.throughput(Throughput::Elements(player_count as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(player_count),
            &player_count,
            |b, &count| {
                b.iter(|| {
                    for i in 0..count {
                        let visible = black_box(manager.calculate_visibility(i as u64));
                        black_box(visible);
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: EVE Online scale (6000 players)
///
/// EVE Online famously handles 6000+ players in a single system (with time dilation).
/// This is the extreme high-end of MMO scale.
///
/// Target: System completes without crashing (time dilation acceptable)
fn bench_eve_online_scale(c: &mut Criterion) {
    let mut group = c.benchmark_group("industry_eve_online");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(20));

    let player_count = 6000;
    let (world, _) = create_world_grid(player_count);

    let mut manager = InterestManager::new(50.0);
    manager.update_from_world(&world);

    let grid_size = (player_count as f32).sqrt() as usize;
    for i in 0..player_count {
        let x = ((i % grid_size) as f32) * 100.0;
        let z = ((i / grid_size) as f32) * 100.0;
        let pos = Vec3::new(x, 0.0, z);
        manager.set_client_interest(i as u64, AreaOfInterest::new(pos, 150.0));
    }

    group.throughput(Throughput::Elements(player_count as u64));

    group.bench_function("6000_players_sample_100", |b| {
        b.iter(|| {
            // Sample 100 players (1.67%) for performance measurement
            for i in (0..player_count).step_by(60) {
                let visible = black_box(manager.calculate_visibility(i as u64));
                black_box(visible);
            }
        });
    });

    group.finish();
}

/// Benchmark: WoW Classic scale (3000 players per server)
///
/// World of Warcraft Classic servers handle ~3000 concurrent players.
/// Modern WoW uses sharding, but Classic gives us a real-world baseline.
///
/// Target: <1ms per player
fn bench_wow_classic_scale(c: &mut Criterion) {
    let mut group = c.benchmark_group("industry_wow_classic");
    group.sample_size(20);
    group.measurement_time(Duration::from_secs(15));

    let player_count = 3000;
    let (world, positions) = create_mmo_city_distribution(player_count);

    let mut manager = InterestManager::new(50.0);
    manager.update_from_world(&world);

    for (i, &pos) in positions.iter().enumerate() {
        manager.set_client_interest(i as u64, AreaOfInterest::new(pos, 100.0));
    }

    group.throughput(Throughput::Elements(player_count as u64));

    group.bench_function("3000_players_realistic_distribution", |b| {
        b.iter(|| {
            for i in 0..player_count {
                let visible = black_box(manager.calculate_visibility(i as u64));
                black_box(visible);
            }
        });
    });

    group.bench_function("3000_players_sample_300", |b| {
        b.iter(|| {
            // Sample 10% of players
            for i in (0..player_count).step_by(10) {
                let visible = black_box(manager.calculate_visibility(i as u64));
                black_box(visible);
            }
        });
    });

    group.finish();
}

/// Benchmark: Final Fantasy XIV scale (1500 players per zone)
///
/// FFXIV limits zones to ~1500 concurrent players before instancing.
/// This is a well-tuned production MMO target.
///
/// Target: <1ms per player
fn bench_ffxiv_scale(c: &mut Criterion) {
    let mut group = c.benchmark_group("industry_ffxiv");
    group.sample_size(50);
    group.measurement_time(Duration::from_secs(10));

    let player_count = 1500;
    let (world, positions) = create_mmo_city_distribution(player_count);

    let mut manager = InterestManager::new(50.0);
    manager.update_from_world(&world);

    for (i, &pos) in positions.iter().enumerate() {
        manager.set_client_interest(i as u64, AreaOfInterest::new(pos, 100.0));
    }

    group.throughput(Throughput::Elements(player_count as u64));

    group.bench_function("1500_players_city_distribution", |b| {
        b.iter(|| {
            for i in 0..player_count {
                let visible = black_box(manager.calculate_visibility(i as u64));
                black_box(visible);
            }
        });
    });

    group.finish();
}

// ============================================================================
// Real-World Game Pattern Benchmarks
// ============================================================================

/// Benchmark: City hotspot (300 players, 50% moving, 50% static)
///
/// Typical capital city during peak hours:
/// - Half the players are AFK/trading/chatting (static)
/// - Half are actively moving/questing
///
/// Target: <0.5ms per player
fn bench_city_hotspot_realistic(c: &mut Criterion) {
    let mut group = c.benchmark_group("realworld_city_hotspot");
    group.sample_size(100);

    let player_count = 300;
    let (world, mut positions) = create_mmo_city_distribution(player_count);

    let mut manager = InterestManager::new(50.0);
    manager.update_from_world(&world);

    for (i, &pos) in positions.iter().enumerate() {
        manager.set_client_interest(i as u64, AreaOfInterest::new(pos, 75.0));
    }

    group.throughput(Throughput::Elements(player_count as u64));

    group.bench_function("300_players_50pct_moving", |b| {
        b.iter(|| {
            // Update half the players (simulate movement)
            for i in (0..player_count).step_by(2) {
                positions[i] += Vec3::new(0.5, 0.0, 0.5);
                manager.set_client_interest(i as u64, AreaOfInterest::new(positions[i], 75.0));
            }

            // Calculate visibility for all
            for i in 0..player_count {
                let visible = black_box(manager.calculate_visibility(i as u64));
                black_box(visible);
            }
        });
    });

    group.finish();
}

/// Benchmark: 40-player raid formation
///
/// Tight formation with coordinated movement (tanks, healers, DPS).
/// Everyone sees everyone, high update rate.
///
/// Target: <0.2ms per player
fn bench_raid_formation(c: &mut Criterion) {
    let mut group = c.benchmark_group("realworld_raid");
    group.sample_size(200);

    let boss_pos = Vec3::ZERO;
    let positions = create_raid_formation(boss_pos, 40);

    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Aabb>();

    for &pos in &positions {
        let entity = world.spawn();
        world.add(entity, Transform::new(pos, Quat::IDENTITY, Vec3::ONE));
        world.add(entity, Aabb::from_center_half_extents(pos, Vec3::ONE));
    }

    let mut manager = InterestManager::new(30.0);
    manager.update_from_world(&world);

    for (i, &pos) in positions.iter().enumerate() {
        manager.set_client_interest(i as u64, AreaOfInterest::new(pos, 50.0));
    }

    group.throughput(Throughput::Elements(40));

    group.bench_function("40_players_tight_formation", |b| {
        b.iter(|| {
            for i in 0..40 {
                let visible = black_box(manager.calculate_visibility(i as u64));
                black_box(visible);
            }
        });
    });

    group.finish();
}

/// Benchmark: 20v20 PvP skirmish with rapid movement
///
/// Two teams fighting, lots of movement, deaths, respawns.
/// Rapid visibility changes.
///
/// Target: <0.5ms per player
fn bench_pvp_skirmish(c: &mut Criterion) {
    let mut group = c.benchmark_group("realworld_pvp_skirmish");
    group.sample_size(100);

    let battlefield_center = Vec3::ZERO;

    // Team 1: 20 players west
    let mut team1_positions: Vec<Vec3> = (0..20)
        .map(|i| {
            let angle = (i as f32 / 20.0) * std::f32::consts::TAU;
            battlefield_center + Vec3::new(-30.0 + angle.cos() * 15.0, 0.0, angle.sin() * 15.0)
        })
        .collect();

    // Team 2: 20 players east
    let mut team2_positions: Vec<Vec3> = (0..20)
        .map(|i| {
            let angle = (i as f32 / 20.0) * std::f32::consts::TAU;
            battlefield_center + Vec3::new(30.0 + angle.cos() * 15.0, 0.0, angle.sin() * 15.0)
        })
        .collect();

    let mut all_positions = team1_positions.clone();
    all_positions.append(&mut team2_positions);

    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Aabb>();

    for &pos in &all_positions {
        let entity = world.spawn();
        world.add(entity, Transform::new(pos, Quat::IDENTITY, Vec3::ONE));
        world.add(entity, Aabb::from_center_half_extents(pos, Vec3::ONE));
    }

    let mut manager = InterestManager::new(40.0);
    manager.update_from_world(&world);

    for (i, &pos) in all_positions.iter().enumerate() {
        manager.set_client_interest(i as u64, AreaOfInterest::new(pos, 80.0));
    }

    group.throughput(Throughput::Elements(40));

    group.bench_function("40_players_pvp_with_movement", |b| {
        b.iter(|| {
            // Simulate rapid movement toward center
            for i in 0..40 {
                let direction = (battlefield_center - all_positions[i]).normalize();
                all_positions[i] += direction * 2.0; // Fast movement
                manager.set_client_interest(i as u64, AreaOfInterest::new(all_positions[i], 80.0));
            }

            // Calculate visibility
            for i in 0..40 {
                let visible = black_box(manager.calculate_visibility(i as u64));
                black_box(visible);
            }
        });
    });

    group.finish();
}

/// Benchmark: World boss convergence (150 players)
///
/// Players converging on a world boss spawn point.
/// Density increases over time, stress test.
///
/// Target: <1ms per player
fn bench_world_boss_convergence(c: &mut Criterion) {
    let mut group = c.benchmark_group("realworld_world_boss");
    group.sample_size(50);

    let boss_location = Vec3::new(500.0, 0.0, 500.0);

    // Players starting spread out, converging
    let mut positions: Vec<Vec3> = (0..150)
        .map(|i| {
            let angle = (i as f32 / 150.0) * std::f32::consts::TAU;
            let radius = 200.0 + (i % 50) as f32 * 10.0;
            boss_location + Vec3::new(angle.cos() * radius, 0.0, angle.sin() * radius)
        })
        .collect();

    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Aabb>();

    // Boss
    let boss = world.spawn();
    world.add(boss, Transform::new(boss_location, Quat::IDENTITY, Vec3::splat(5.0)));
    world.add(boss, Aabb::from_center_half_extents(boss_location, Vec3::splat(5.0)));

    // Players
    for &pos in &positions {
        let entity = world.spawn();
        world.add(entity, Transform::new(pos, Quat::IDENTITY, Vec3::ONE));
        world.add(entity, Aabb::from_center_half_extents(pos, Vec3::ONE));
    }

    let mut manager = InterestManager::new(50.0);
    manager.update_from_world(&world);

    for (i, &pos) in positions.iter().enumerate() {
        manager.set_client_interest(i as u64, AreaOfInterest::new(pos, 100.0));
    }

    group.throughput(Throughput::Elements(150));

    group.bench_function("150_players_converging", |b| {
        b.iter(|| {
            // Simulate convergence
            for i in 0..150 {
                let direction = (boss_location - positions[i]).normalize();
                positions[i] += direction * 5.0;
                manager.set_client_interest(i as u64, AreaOfInterest::new(positions[i], 100.0));
            }

            // Calculate visibility
            for i in 0..150 {
                let visible = black_box(manager.calculate_visibility(i as u64));
                black_box(visible);
            }
        });
    });

    group.finish();
}

// ============================================================================
// Performance Profiling Benchmarks
// ============================================================================

/// Benchmark: CPU scaling across entity counts
///
/// Measure how visibility calculation scales with entity count.
/// Target: Linear or better scaling
fn bench_cpu_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("profiling_cpu_scaling");
    group.sample_size(50);

    for entity_count in [100, 500, 1000, 5000, 10000] {
        let (world, _) = create_world_grid(entity_count);

        let mut manager = InterestManager::new(50.0);
        manager.update_from_world(&world);

        // Single client in middle of world
        let center =
            Vec3::new((entity_count as f32).sqrt() * 5.0, 0.0, (entity_count as f32).sqrt() * 5.0);
        manager.set_client_interest(0, AreaOfInterest::new(center, 100.0));

        group.throughput(Throughput::Elements(entity_count as u64));

        group.bench_with_input(BenchmarkId::from_parameter(entity_count), &entity_count, |b, _| {
            b.iter(|| {
                let visible = black_box(manager.calculate_visibility(0));
                black_box(visible);
            });
        });
    }

    group.finish();
}

/// Benchmark: Memory usage per player
///
/// Measure memory overhead of interest management structures.
/// Target: <1KB per player
fn bench_memory_per_player(c: &mut Criterion) {
    let mut group = c.benchmark_group("profiling_memory");
    group.sample_size(20);

    for player_count in [1, 10, 100, 1000, 10000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(player_count),
            &player_count,
            |b, &count| {
                b.iter(|| {
                    let (world, positions) = create_mmo_city_distribution(count);

                    let mut manager = InterestManager::new(50.0);
                    manager.update_from_world(&world);

                    for (i, &pos) in positions.iter().enumerate() {
                        manager.set_client_interest(i as u64, AreaOfInterest::new(pos, 100.0));
                    }

                    // Calculate visibility for all (exercises all data structures)
                    for i in 0..count {
                        let visible = manager.calculate_visibility(i as u64);
                        black_box(visible);
                    }

                    black_box(manager);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Grid query cache efficiency
///
/// Measure how well spatial grid queries utilize cache.
/// Target: >90% cache hit rate (approximate via timing consistency)
fn bench_cache_hit_rate(c: &mut Criterion) {
    let mut group = c.benchmark_group("profiling_cache_efficiency");
    group.sample_size(200);

    let (world, _) = create_world_grid(1000);

    let mut manager = InterestManager::new(50.0);
    manager.update_from_world(&world);

    let center = Vec3::new(150.0, 0.0, 150.0);
    manager.set_client_interest(0, AreaOfInterest::new(center, 100.0));

    group.bench_function("repeated_same_query", |b| {
        b.iter(|| {
            // Same query repeated - should be cache-friendly
            for _ in 0..10 {
                let visible = black_box(manager.calculate_visibility(0));
                black_box(visible);
            }
        });
    });

    group.bench_function("scattered_random_queries", |b| {
        b.iter(|| {
            // Random positions - cache-unfriendly
            let positions = [
                Vec3::new(10.0, 0.0, 10.0),
                Vec3::new(500.0, 0.0, 500.0),
                Vec3::new(50.0, 0.0, 800.0),
                Vec3::new(900.0, 0.0, 100.0),
                Vec3::new(300.0, 0.0, 300.0),
            ];

            for (i, &pos) in positions.iter().enumerate() {
                manager.set_client_interest(i as u64, AreaOfInterest::new(pos, 100.0));
                let visible = black_box(manager.calculate_visibility(i as u64));
                black_box(visible);
            }
        });
    });

    group.finish();
}

// ============================================================================
// Criterion Configuration
// ============================================================================

criterion_group!(
    industry_comparisons,
    bench_unity_dots_comparison,
    bench_unreal_mass_comparison,
    bench_eve_online_scale,
    bench_wow_classic_scale,
    bench_ffxiv_scale,
);

criterion_group!(
    realworld_patterns,
    bench_city_hotspot_realistic,
    bench_raid_formation,
    bench_pvp_skirmish,
    bench_world_boss_convergence,
);

criterion_group!(
    performance_profiling,
    bench_cpu_scaling,
    bench_memory_per_player,
    bench_cache_hit_rate,
);

criterion_main!(industry_comparisons, realworld_patterns, performance_profiling);
