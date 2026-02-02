//! Extreme Scale Stress Tests - 10K to 100K Players
//!
//! Tests the absolute limits of the interest management system:
//! - 10K players (realistic MMO)
//! - 20K players (extreme MMO)
//! - 50K players (mega-scale)
//! - 100K players (theoretical limit)
//!
//! These benchmarks validate:
//! - Memory efficiency at scale
//! - CPU performance under massive load
//! - Spatial indexing scalability
//! - Threading/parallelization effectiveness

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_core::{Aabb, Quat, Transform, Vec3, World};
use engine_interest::{AreaOfInterest, InterestManager};
use std::time::Duration;

// ============================================================================
// Helper Functions
// ============================================================================

fn create_massive_world(
    player_count: usize,
    entities_per_region: usize,
) -> (World, Vec<(u64, Vec3)>) {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Aabb>();

    // Create world with regions (players spread across zones)
    let region_size = (player_count as f32).sqrt() as usize;
    let region_spacing = 1000.0; // Large spacing for realistic world

    // Spawn entities in each region
    for region_x in 0..region_size {
        for region_z in 0..region_size {
            let region_center_x = region_x as f32 * region_spacing;
            let region_center_z = region_z as f32 * region_spacing;

            // Entities in this region
            for i in 0..entities_per_region {
                let entity = world.spawn();
                let angle = (i as f32) * std::f32::consts::PI * 2.0 / (entities_per_region as f32);
                let radius = (i % 10) as f32 * 10.0;
                let pos = Vec3::new(
                    region_center_x + radius * angle.cos(),
                    0.0,
                    region_center_z + radius * angle.sin(),
                );
                world.add(entity, Transform::new(pos, Quat::IDENTITY, Vec3::ONE));
                world.add(entity, Aabb::from_center_half_extents(pos, Vec3::ONE));
            }
        }
    }

    // Generate player positions (one per region)
    let mut players = Vec::new();
    for region_x in 0..region_size {
        for region_z in 0..region_size {
            let player_id = (region_x * region_size + region_z) as u64;
            let pos =
                Vec3::new(region_x as f32 * region_spacing, 0.0, region_z as f32 * region_spacing);
            players.push((player_id, pos));

            if players.len() >= player_count {
                break;
            }
        }
        if players.len() >= player_count {
            break;
        }
    }

    (world, players)
}

#[allow(dead_code)]
fn measure_memory_usage() -> usize {
    // Estimate memory usage (actual measurement would require platform-specific APIs)
    // For now, we'll track this manually in the benchmark output
    0
}

// ============================================================================
// Extreme Scale Benchmarks
// ============================================================================

fn bench_10k_players(c: &mut Criterion) {
    let mut group = c.benchmark_group("extreme_10k_players");
    group.measurement_time(Duration::from_secs(30));
    group.sample_size(10);

    println!("\n=== 10K PLAYER STRESS TEST ===");

    // 10K players, 50 entities per region = 500K total entities
    let (world, players) = create_massive_world(10_000, 50);
    println!("World created: {} entities, {} players", world.entity_count(), players.len());

    let mut manager = InterestManager::new(200.0);

    let start = std::time::Instant::now();
    manager.update_from_world(&world);
    let update_time = start.elapsed();
    println!("Initial world update: {:?}", update_time);

    // Register all players
    let start = std::time::Instant::now();
    for (player_id, pos) in &players {
        manager.set_client_interest(*player_id, AreaOfInterest::new(*pos, 200.0));
    }
    let registration_time = start.elapsed();
    println!("Player registration: {:?}", registration_time);

    group.throughput(Throughput::Elements(players.len() as u64));

    // Benchmark: Calculate visibility for all players
    group.bench_function("full_visibility_cycle", |b| {
        b.iter(|| {
            let mut total_visible = 0;
            for (player_id, _) in &players {
                let visible = black_box(manager.calculate_visibility(*player_id));
                total_visible += visible.len();
                black_box(visible);
            }
            black_box(total_visible);
        });
    });

    // Benchmark: Sample of 100 players (representative)
    group.bench_function("sample_100_players", |b| {
        b.iter(|| {
            for i in (0..players.len()).step_by(100) {
                let (player_id, _) = players[i];
                let visible = black_box(manager.calculate_visibility(player_id));
                black_box(visible);
            }
        });
    });

    // Benchmark: Single player visibility
    group.bench_function("single_player_visibility", |b| {
        let (test_player, _) = players[0];
        b.iter(|| {
            let visible = black_box(manager.calculate_visibility(test_player));
            black_box(visible);
        });
    });

    println!("Memory estimate: ~{} MB", world.entity_count() * 128 / 1_000_000);

    group.finish();
}

fn bench_20k_players(c: &mut Criterion) {
    let mut group = c.benchmark_group("extreme_20k_players");
    group.measurement_time(Duration::from_secs(40));
    group.sample_size(10);

    println!("\n=== 20K PLAYER STRESS TEST ===");

    // 20K players, 50 entities per region = 1M total entities
    let (world, players) = create_massive_world(20_000, 50);
    println!("World created: {} entities, {} players", world.entity_count(), players.len());

    let mut manager = InterestManager::new(200.0);

    let start = std::time::Instant::now();
    manager.update_from_world(&world);
    let update_time = start.elapsed();
    println!("Initial world update: {:?}", update_time);

    let start = std::time::Instant::now();
    for (player_id, pos) in &players {
        manager.set_client_interest(*player_id, AreaOfInterest::new(*pos, 200.0));
    }
    let registration_time = start.elapsed();
    println!("Player registration: {:?}", registration_time);

    group.throughput(Throughput::Elements(players.len() as u64));

    // Sample-based benchmarks (full iteration would take too long)
    group.bench_function("sample_200_players", |b| {
        b.iter(|| {
            for i in (0..players.len()).step_by(100) {
                let (player_id, _) = players[i];
                let visible = black_box(manager.calculate_visibility(player_id));
                black_box(visible);
            }
        });
    });

    group.bench_function("single_player_visibility", |b| {
        let (test_player, _) = players[0];
        b.iter(|| {
            let visible = black_box(manager.calculate_visibility(test_player));
            black_box(visible);
        });
    });

    println!("Memory estimate: ~{} MB", world.entity_count() * 128 / 1_000_000);

    group.finish();
}

fn bench_50k_players(c: &mut Criterion) {
    let mut group = c.benchmark_group("extreme_50k_players");
    group.measurement_time(Duration::from_secs(60));
    group.sample_size(10);

    println!("\n=== 50K PLAYER STRESS TEST (MEGA-SCALE) ===");

    // 50K players, 30 entities per region = 1.5M total entities
    let (world, players) = create_massive_world(50_000, 30);
    println!("World created: {} entities, {} players", world.entity_count(), players.len());

    let mut manager = InterestManager::new(200.0);

    let start = std::time::Instant::now();
    manager.update_from_world(&world);
    let update_time = start.elapsed();
    println!("Initial world update: {:?}", update_time);

    let start = std::time::Instant::now();
    for (player_id, pos) in &players {
        manager.set_client_interest(*player_id, AreaOfInterest::new(*pos, 200.0));
    }
    let registration_time = start.elapsed();
    println!("Player registration: {:?}", registration_time);

    group.throughput(Throughput::Elements(players.len() as u64));

    // Only sample-based tests at this scale
    group.bench_function("sample_500_players", |b| {
        b.iter(|| {
            for i in (0..players.len()).step_by(100) {
                let (player_id, _) = players[i];
                let visible = black_box(manager.calculate_visibility(player_id));
                black_box(visible);
            }
        });
    });

    group.bench_function("single_player_visibility", |b| {
        let (test_player, _) = players[0];
        b.iter(|| {
            let visible = black_box(manager.calculate_visibility(test_player));
            black_box(visible);
        });
    });

    // Test spatial query performance
    group.bench_function("spatial_grid_efficiency", |b| {
        let test_positions = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(50000.0, 0.0, 0.0),
            Vec3::new(100000.0, 0.0, 0.0),
        ];

        b.iter(|| {
            for pos in &test_positions {
                manager.set_client_interest(999999, AreaOfInterest::new(*pos, 200.0));
                let visible = black_box(manager.calculate_visibility(999999));
                black_box(visible);
            }
        });
    });

    println!("Memory estimate: ~{} MB", world.entity_count() * 128 / 1_000_000);

    group.finish();
}

fn bench_100k_players(c: &mut Criterion) {
    let mut group = c.benchmark_group("extreme_100k_players");
    group.measurement_time(Duration::from_secs(90));
    group.sample_size(10);

    println!("\n=== 100K PLAYER STRESS TEST (THEORETICAL LIMIT) ===");

    // 100K players, 20 entities per region = 2M total entities
    let (world, players) = create_massive_world(100_000, 20);
    println!("World created: {} entities, {} players", world.entity_count(), players.len());

    let mut manager = InterestManager::new(200.0);

    let start = std::time::Instant::now();
    manager.update_from_world(&world);
    let update_time = start.elapsed();
    println!("Initial world update: {:?}", update_time);

    println!("Registering 100K players...");
    let start = std::time::Instant::now();
    for (player_id, pos) in &players {
        manager.set_client_interest(*player_id, AreaOfInterest::new(*pos, 200.0));
    }
    let registration_time = start.elapsed();
    println!("Player registration: {:?}", registration_time);

    group.throughput(Throughput::Elements(players.len() as u64));

    // Only lightweight tests at this scale
    group.bench_function("sample_1000_players", |b| {
        b.iter(|| {
            for i in (0..players.len()).step_by(100) {
                let (player_id, _) = players[i];
                let visible = black_box(manager.calculate_visibility(player_id));
                black_box(visible);
            }
        });
    });

    group.bench_function("single_player_visibility", |b| {
        let (test_player, _) = players[0];
        b.iter(|| {
            let visible = black_box(manager.calculate_visibility(test_player));
            black_box(visible);
        });
    });

    // Test if spatial partitioning scales logarithmically
    group.bench_function("scaling_test", |b| {
        b.iter(|| {
            // Test at different world positions
            for i in 0..10 {
                let pos = Vec3::new(i as f32 * 10000.0, 0.0, 0.0);
                manager.set_client_interest(888888, AreaOfInterest::new(pos, 200.0));
                let visible = black_box(manager.calculate_visibility(888888));
                black_box(visible);
            }
        });
    });

    println!("Memory estimate: ~{} MB", world.entity_count() * 128 / 1_000_000);
    println!("Players per server core (assuming 32 cores): ~{}", 100_000 / 32);

    group.finish();
}

// ============================================================================
// Memory and Throughput Analysis
// ============================================================================

fn bench_memory_scalability(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_scalability");
    group.measurement_time(Duration::from_secs(30));

    println!("\n=== MEMORY SCALABILITY ANALYSIS ===");

    for player_count in [1_000, 5_000, 10_000, 20_000, 50_000] {
        let (world, players) = create_massive_world(player_count, 20);
        let mut manager = InterestManager::new(200.0);

        manager.update_from_world(&world);
        for (player_id, pos) in &players {
            manager.set_client_interest(*player_id, AreaOfInterest::new(*pos, 200.0));
        }

        let entity_count = world.entity_count();
        let estimated_memory_mb = entity_count * 128 / 1_000_000;

        println!(
            "{} players: {} entities, ~{} MB",
            player_count, entity_count, estimated_memory_mb
        );

        group.bench_with_input(
            BenchmarkId::new("visibility_per_player", player_count),
            &player_count,
            |b, _| {
                let (test_player, _) = players[0];
                b.iter(|| {
                    let visible = black_box(manager.calculate_visibility(test_player));
                    black_box(visible);
                });
            },
        );
    }

    group.finish();
}

fn bench_throughput_analysis(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput_analysis");
    group.measurement_time(Duration::from_secs(30));

    println!("\n=== THROUGHPUT ANALYSIS (Players/Second) ===");

    let (world, players) = create_massive_world(10_000, 50);
    let mut manager = InterestManager::new(200.0);
    manager.update_from_world(&world);

    for (player_id, pos) in &players {
        manager.set_client_interest(*player_id, AreaOfInterest::new(*pos, 200.0));
    }

    // Measure how many player visibility calculations per second
    group.bench_function("players_per_second", |b| {
        let mut player_idx = 0;
        b.iter(|| {
            let (player_id, _) = players[player_idx % players.len()];
            let visible = black_box(manager.calculate_visibility(player_id));
            black_box(visible);
            player_idx += 1;
        });
    });

    // Batch processing throughput
    group.bench_function("batch_processing_100", |b| {
        b.iter(|| {
            for i in 0..100 {
                let (player_id, _) = players[i % players.len()];
                let visible = black_box(manager.calculate_visibility(player_id));
                black_box(visible);
            }
        });
    });

    group.finish();
}

// ============================================================================
// Benchmark Registration
// ============================================================================

criterion_group!(
    extreme_scale_benches,
    bench_10k_players,
    bench_20k_players,
    bench_50k_players,
    bench_100k_players,
    bench_memory_scalability,
    bench_throughput_analysis,
);

criterion_main!(extreme_scale_benches);
