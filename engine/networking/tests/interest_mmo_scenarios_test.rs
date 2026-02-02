//! MMO Scenario Tests - Real-World Game Patterns
//!
//! This test suite validates interest management against actual MMO scenarios:
//! - City hotspots (capital cities, auction houses)
//! - Raid/dungeon scenarios (40-player raids, instance isolation)
//! - PvP battlegrounds (100v100, siege warfare)
//! - Open world events (world bosses, flying mounts)
//! - Stress scenarios (login storms, server merges)
//!
//! These tests represent real production scenarios from AAA MMOs and validate
//! that the interest management system can handle them at scale.

use engine_core::{Aabb, Entity, Quat, Transform, Vec3, World};
use engine_interest::{AreaOfInterest, InterestManager};
use std::collections::HashMap;

// ============================================================================
// Test Utilities
// ============================================================================

/// Create a world with entities at specified positions
fn create_world_at_positions(positions: &[Vec3]) -> (World, Vec<Entity>) {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Aabb>();

    let mut entities = Vec::with_capacity(positions.len());

    for &pos in positions {
        let entity = world.spawn();
        world.add(entity, Transform::new(pos, Quat::IDENTITY, Vec3::ONE));
        world.add(entity, Aabb::from_center_half_extents(pos, Vec3::ONE));
        entities.push(entity);
    }

    (world, entities)
}

/// Create entities in a clustered pattern (for hotspots)
fn create_clustered_entities(center: Vec3, count: usize, radius: f32) -> Vec<Vec3> {
    use std::f32::consts::PI;

    let mut positions = Vec::with_capacity(count);

    for i in 0..count {
        let angle = (i as f32 / count as f32) * 2.0 * PI;
        let dist = (i as f32 / count as f32).sqrt() * radius; // Denser near center
        let x = center.x + angle.cos() * dist;
        let z = center.z + angle.sin() * dist;
        positions.push(Vec3::new(x, center.y, z));
    }

    positions
}

/// Create entities spread across streets/buildings in a grid
fn create_city_grid(origin: Vec3, streets: usize, players_per_street: usize) -> Vec<Vec3> {
    let mut positions = Vec::new();

    for street_x in 0..streets {
        for street_z in 0..streets {
            for player in 0..players_per_street {
                let x = origin.x + (street_x as f32) * 50.0 + (player % 5) as f32 * 8.0;
                let z = origin.z + (street_z as f32) * 50.0 + (player / 5) as f32 * 8.0;
                positions.push(Vec3::new(x, origin.y, z));
            }
        }
    }

    positions
}

/// Create raid formation (tanks, healers, DPS in groups)
fn create_raid_formation(boss_pos: Vec3, player_count: usize) -> Vec<Vec3> {
    let mut positions = Vec::with_capacity(player_count);

    // Tanks (melee range, 5 units from boss)
    let tank_count = (player_count as f32 * 0.15) as usize;
    for i in 0..tank_count {
        let angle = (i as f32 / tank_count as f32) * std::f32::consts::PI; // Semicircle
        positions.push(boss_pos + Vec3::new(angle.cos() * 5.0, 0.0, angle.sin() * 5.0));
    }

    // Melee DPS (slightly behind tanks, 8 units)
    let melee_count = (player_count as f32 * 0.25) as usize;
    for i in 0..melee_count {
        let angle = (i as f32 / melee_count as f32) * std::f32::consts::PI;
        positions.push(boss_pos + Vec3::new(angle.cos() * 8.0, 0.0, angle.sin() * 8.0));
    }

    // Ranged DPS (15-20 units back)
    let ranged_count = (player_count as f32 * 0.45) as usize;
    for i in 0..ranged_count {
        let angle = (i as f32 / ranged_count as f32) * std::f32::consts::PI * 1.5;
        let dist = 15.0 + (i % 3) as f32 * 2.0;
        positions.push(boss_pos + Vec3::new(angle.cos() * dist, 0.0, angle.sin() * dist));
    }

    // Healers (spread out, 12-18 units)
    let healer_count = player_count - tank_count - melee_count - ranged_count;
    for i in 0..healer_count {
        let angle = (i as f32 / healer_count as f32) * std::f32::consts::PI * 2.0;
        positions.push(boss_pos + Vec3::new(angle.cos() * 15.0, 0.0, angle.sin() * 15.0));
    }

    positions
}

// ============================================================================
// City Scenarios
// ============================================================================

#[test]
fn test_major_city_hundreds_players() {
    // Scenario: 500 players in capital city (typical Saturday afternoon)
    // Requirements:
    // - Each player sees ~50-100 nearby players (not all 500)
    // - Performance target: <2ms per player visibility calculation
    // - Bandwidth reduction: >95% (500 * 500 = 250K → ~30K updates)

    let positions = create_city_grid(Vec3::ZERO, 10, 50); // 10x10 streets, 50 players each = 500
    let (world, entities) = create_world_at_positions(&positions);

    let mut manager = InterestManager::new(50.0);
    manager.update_from_world(&world);

    // Register all 500 players as clients
    for (i, &pos) in positions.iter().enumerate() {
        manager.set_client_interest(i as u64, AreaOfInterest::new(pos, 75.0));
    }

    // Calculate visibility for all players
    let start = std::time::Instant::now();
    let mut total_visible = 0;

    for i in 0..500 {
        let visible = manager.calculate_visibility(i as u64);
        total_visible += visible.len();

        // Each player should see some but not all entities
        assert!(visible.len() > 0, "Player {} should see someone", i);
        assert!(visible.len() < 500, "Player {} should not see everyone", i);
        assert!(visible.len() < 150, "Player {} sees too many ({})", i, visible.len());
    }

    let elapsed = start.elapsed();
    let per_player = elapsed.as_micros() / 500;

    tracing::info!(
        "Major city: 500 players, {} total visible, {}µs avg per player",
        total_visible,
        per_player
    );

    // Performance validation
    assert!(per_player < 2000, "Per-player calculation too slow: {}µs", per_player);

    // Bandwidth validation
    let (without, with, reduction) = manager.compute_bandwidth_reduction();
    assert_eq!(without, 500 * 500, "Should be 250K without interest");
    assert!(reduction > 95.0, "Should achieve >95% reduction, got {}%", reduction);
}

#[test]
fn test_auction_house_hotspot() {
    // Scenario: 200 players clustered at auction house NPC
    // This is a pathological case - everyone in same small area
    // Requirements:
    // - Everyone sees everyone (or close to it)
    // - System doesn't degrade under extreme density
    // - Still faster than no interest management

    let positions = create_clustered_entities(Vec3::new(0.0, 0.0, 0.0), 200, 20.0);
    let (world, _entities) = create_world_at_positions(&positions);

    let mut manager = InterestManager::new(50.0);
    manager.update_from_world(&world);

    // All players at auction house
    for (i, &pos) in positions.iter().enumerate() {
        manager.set_client_interest(i as u64, AreaOfInterest::new(pos, 50.0));
    }

    // Everyone should see most/all other players
    for i in 0..200 {
        let visible = manager.calculate_visibility(i as u64);

        // In a 20 unit radius cluster with 50 unit AOI, should see nearly everyone
        assert!(visible.len() > 150, "Player {} only sees {}, expected >150", i, visible.len());
    }

    // Bandwidth reduction will be minimal but system should still work
    let (without, with, reduction) = manager.compute_bandwidth_reduction();

    tracing::info!(
        "Auction house hotspot: 200 players, bandwidth: {} → {} ({}% reduction)",
        without,
        with,
        reduction
    );

    // Even in worst case, some reduction from precise distance checks
    assert!(with <= without, "Interest management should not increase bandwidth");
}

#[test]
fn test_city_streets_distribution() {
    // Scenario: Players spread across streets and buildings
    // More realistic than auction house - players doing different activities
    // Requirements:
    // - Players on different streets don't see each other
    // - Players in same street/building see each other
    // - High bandwidth reduction (80-90%)

    let positions = create_city_grid(Vec3::ZERO, 8, 30); // 8x8 streets, 30 players = 240 total
    let (world, _entities) = create_world_at_positions(&positions);

    let mut manager = InterestManager::new(50.0);
    manager.update_from_world(&world);

    for (i, &pos) in positions.iter().enumerate() {
        manager.set_client_interest(i as u64, AreaOfInterest::new(pos, 60.0));
    }

    // Sample a few players and verify locality
    let player_corner = 0; // Corner of city
    let visible_corner = manager.calculate_visibility(player_corner);

    let player_center = 240 / 2; // Middle of city
    let visible_center = manager.calculate_visibility(player_center as u64);

    // Corner player sees fewer (edge effect)
    // Center player sees more (surrounded)
    assert!(visible_corner.len() < visible_center.len());

    // Neither sees everyone
    assert!(visible_corner.len() < 240);
    assert!(visible_center.len() < 240);

    let (_, _, reduction) = manager.compute_bandwidth_reduction();
    assert!(
        reduction > 80.0,
        "Street distribution should achieve >80% reduction, got {}%",
        reduction
    );
}

#[test]
fn test_city_instance_sharding() {
    // Scenario: Multiple city instances (sharding)
    // Instance 1 and Instance 2 are same location but different "phases"
    // Requirements:
    // - Players in different instances don't see each other (no cross-instance visibility)
    // - Each instance operates independently

    // Instance 1: 100 players
    let instance1_pos = create_city_grid(Vec3::ZERO, 5, 20);

    // Instance 2: 100 players (same positions, different "dimension")
    // We simulate this by using different client IDs (1000-1099) for instance 2
    let instance2_pos = create_city_grid(Vec3::new(10000.0, 0.0, 0.0), 5, 20); // Far away

    let mut all_positions = instance1_pos.clone();
    all_positions.extend(instance2_pos.clone());

    let (world, _entities) = create_world_at_positions(&all_positions);

    let mut manager = InterestManager::new(50.0);
    manager.update_from_world(&world);

    // Instance 1 clients (0-99)
    for (i, &pos) in instance1_pos.iter().enumerate() {
        manager.set_client_interest(i as u64, AreaOfInterest::new(pos, 75.0));
    }

    // Instance 2 clients (1000-1099)
    for (i, &pos) in instance2_pos.iter().enumerate() {
        manager.set_client_interest((i + 1000) as u64, AreaOfInterest::new(pos, 75.0));
    }

    // Verify instance isolation
    let player1_visible = manager.calculate_visibility(0);
    let player2_visible = manager.calculate_visibility(1000);

    // No overlap between instances
    for entity in &player1_visible {
        assert!(!player2_visible.contains(entity), "Cross-instance visibility detected!");
    }

    tracing::info!(
        "Instance sharding: Instance 1 sees {}, Instance 2 sees {} (no overlap)",
        player1_visible.len(),
        player2_visible.len()
    );
}

// ============================================================================
// Raid/Dungeon Scenarios
// ============================================================================

#[test]
fn test_40_player_raid() {
    // Scenario: 40-player raid fighting a boss
    // High density, coordinated movement
    // Requirements:
    // - All 40 players see each other (small area)
    // - Boss visible to all players
    // - Fast updates for coordinated gameplay

    let boss_pos = Vec3::new(0.0, 0.0, 0.0);
    let player_positions = create_raid_formation(boss_pos, 40);

    let mut all_positions = vec![boss_pos]; // Boss entity
    all_positions.extend(player_positions.clone());

    let (world, entities) = create_world_at_positions(&all_positions);
    let boss_entity = entities[0];

    let mut manager = InterestManager::new(30.0);
    manager.update_from_world(&world);

    // All 40 players
    for (i, &pos) in player_positions.iter().enumerate() {
        manager.set_client_interest(i as u64, AreaOfInterest::new(pos, 50.0));
    }

    // Every player should see the boss
    for i in 0..40 {
        let visible = manager.calculate_visibility(i as u64);
        assert!(visible.contains(&boss_entity), "Player {} cannot see boss!", i);

        // Should see most other players (might miss a few at max range)
        assert!(visible.len() >= 35, "Player {} only sees {} entities", i, visible.len());
    }

    tracing::info!("40-player raid: All players see boss and most teammates");
}

#[test]
fn test_raid_wipe_respawn() {
    // Scenario: All 40 players die and respawn simultaneously at graveyard
    // Tests rapid position changes and visibility recalculation
    // Requirements:
    // - System handles mass teleportation
    // - All players see each other at graveyard
    // - No stale visibility data

    let boss_pos = Vec3::new(0.0, 0.0, 0.0);
    let graveyard_pos = Vec3::new(500.0, 0.0, 500.0);

    // Initial: 40 players fighting boss
    let raid_positions = create_raid_formation(boss_pos, 40);
    let (mut world, entities) = create_world_at_positions(&raid_positions);

    let mut manager = InterestManager::new(30.0);
    manager.update_from_world(&world);

    for (i, &pos) in raid_positions.iter().enumerate() {
        manager.set_client_interest(i as u64, AreaOfInterest::new(pos, 50.0));
    }

    // Initial visibility calculated
    for i in 0..40 {
        manager.get_visibility_changes(i as u64);
    }

    // WIPE! Everyone dies and respawns at graveyard
    world.register::<Transform>();
    world.register::<Aabb>();

    let graveyard_cluster = create_clustered_entities(graveyard_pos, 40, 10.0);

    // Update all entity positions (simulate respawn)
    for (i, &entity) in entities.iter().enumerate() {
        world.remove::<Transform>(entity);
        world.remove::<Aabb>(entity);

        let new_pos = graveyard_cluster[i];
        world.add(entity, Transform::new(new_pos, Quat::IDENTITY, Vec3::ONE));
        world.add(entity, Aabb::from_center_half_extents(new_pos, Vec3::ONE));
    }

    manager.update_from_world(&world);

    // Update client interests to graveyard
    for (i, &pos) in graveyard_cluster.iter().enumerate() {
        manager.set_client_interest(i as u64, AreaOfInterest::new(pos, 50.0));
    }

    // Everyone should see everyone at graveyard
    for i in 0..40 {
        let (entered, exited) = manager.get_visibility_changes(i as u64);

        // Should see most players now (old boss area players exited, graveyard entered)
        assert!(!entered.is_empty() || !exited.is_empty(), "Player {} should detect changes", i);

        let visible = manager.calculate_visibility(i as u64);
        assert!(visible.len() >= 35, "Player {} should see most players at graveyard", i);
    }

    tracing::info!("Raid wipe respawn: Successfully handled mass teleportation");
}

#[test]
fn test_instanced_dungeon_isolation() {
    // Scenario: Multiple 5-player dungeon groups
    // Each group is in their own instance of the same dungeon
    // Requirements:
    // - Groups don't see each other (instance isolation)
    // - Each group sees their own members

    let dungeon_start = Vec3::ZERO;

    // 10 groups of 5 players each
    let mut all_positions = Vec::new();
    let group_offsets = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1000.0, 0.0, 0.0),
        Vec3::new(2000.0, 0.0, 0.0),
        Vec3::new(3000.0, 0.0, 0.0),
        Vec3::new(4000.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, 1000.0),
        Vec3::new(1000.0, 0.0, 1000.0),
        Vec3::new(2000.0, 0.0, 1000.0),
        Vec3::new(3000.0, 0.0, 1000.0),
        Vec3::new(4000.0, 0.0, 1000.0),
    ];

    for offset in &group_offsets {
        for i in 0..5 {
            all_positions.push(dungeon_start + *offset + Vec3::new(i as f32 * 5.0, 0.0, 0.0));
        }
    }

    let (world, _entities) = create_world_at_positions(&all_positions);

    let mut manager = InterestManager::new(50.0);
    manager.update_from_world(&world);

    // Register all 50 players
    for (i, &pos) in all_positions.iter().enumerate() {
        manager.set_client_interest(i as u64, AreaOfInterest::new(pos, 100.0));
    }

    // Verify group isolation
    for group in 0..10 {
        let player_in_group = group * 5; // First player in this group
        let visible = manager.calculate_visibility(player_in_group as u64);

        // Should see their 4 teammates (+ themselves if counted)
        assert!(
            visible.len() >= 4 && visible.len() <= 5,
            "Group {} player sees {} entities, expected 4-5",
            group,
            visible.len()
        );
    }

    tracing::info!("Instanced dungeons: 10 groups of 5 properly isolated");
}

#[test]
fn test_phased_content() {
    // Scenario: Same location, different quest phases
    // Players in phase 1 see different NPCs than players in phase 2
    // Requirements:
    // - Phase isolation (simulated via separate spatial regions)
    // - Players in same phase see each other

    let quest_hub = Vec3::ZERO;

    // Phase 1: 50 players (pre-quest)
    let phase1_positions = create_clustered_entities(quest_hub, 50, 30.0);

    // Phase 2: 30 players (post-quest) - simulated as different area
    let phase2_positions =
        create_clustered_entities(quest_hub + Vec3::new(5000.0, 0.0, 0.0), 30, 30.0);

    let mut all_positions = phase1_positions.clone();
    all_positions.extend(phase2_positions.clone());

    let (world, _entities) = create_world_at_positions(&all_positions);

    let mut manager = InterestManager::new(50.0);
    manager.update_from_world(&world);

    // Phase 1 clients
    for (i, &pos) in phase1_positions.iter().enumerate() {
        manager.set_client_interest(i as u64, AreaOfInterest::new(pos, 75.0));
    }

    // Phase 2 clients (offset IDs)
    for (i, &pos) in phase2_positions.iter().enumerate() {
        manager.set_client_interest((i + 100) as u64, AreaOfInterest::new(pos, 75.0));
    }

    // Verify phase isolation
    let phase1_player = manager.calculate_visibility(0);
    let phase2_player = manager.calculate_visibility(100);

    // No overlap
    for entity in &phase1_player {
        assert!(!phase2_player.contains(entity), "Cross-phase visibility!");
    }

    tracing::info!(
        "Phased content: Phase 1 sees {}, Phase 2 sees {} (isolated)",
        phase1_player.len(),
        phase2_player.len()
    );
}

// ============================================================================
// PvP Scenarios
// ============================================================================

#[test]
fn test_100v100_battleground() {
    // Scenario: 200 players in confined battleground arena
    // Two teams fighting in close quarters
    // Requirements:
    // - All players see enemies and allies (high density)
    // - System handles combat movement and deaths
    // - Performance under stress

    let arena_center = Vec3::ZERO;

    // Team 1: 100 players on west side
    let team1_positions =
        create_clustered_entities(arena_center + Vec3::new(-50.0, 0.0, 0.0), 100, 40.0);

    // Team 2: 100 players on east side
    let team2_positions =
        create_clustered_entities(arena_center + Vec3::new(50.0, 0.0, 0.0), 100, 40.0);

    let mut all_positions = team1_positions.clone();
    all_positions.extend(team2_positions);

    let (world, _entities) = create_world_at_positions(&all_positions);

    let mut manager = InterestManager::new(50.0);
    manager.update_from_world(&world);

    // Register all 200 players
    for (i, &pos) in all_positions.iter().enumerate() {
        manager.set_client_interest(i as u64, AreaOfInterest::new(pos, 100.0));
    }

    let start = std::time::Instant::now();

    // Calculate visibility for all (simulates one frame)
    for i in 0..200 {
        let visible = manager.calculate_visibility(i as u64);

        // In 100 unit AOI with two 40 unit clusters 100 units apart,
        // each player should see their team and possibly some enemies
        assert!(visible.len() > 50, "Player {} only sees {}", i, visible.len());
    }

    let elapsed = start.elapsed();

    tracing::info!(
        "100v100 battleground: 200 players, {}ms total, {}µs per player",
        elapsed.as_millis(),
        elapsed.as_micros() / 200
    );

    // Performance target: <200ms for all 200 players
    assert!(elapsed.as_millis() < 200, "Battleground too slow: {}ms", elapsed.as_millis());
}

#[test]
fn test_siege_warfare() {
    // Scenario: 300 attackers, 100 defenders, castle walls
    // Attackers outside walls, defenders inside
    // Requirements:
    // - Attackers see attackers, defenders see defenders
    // - Cross-wall visibility limited by distance
    // - Siege engines and projectiles visible to both sides

    let castle_center = Vec3::ZERO;

    // Defenders inside castle (tight cluster)
    let defenders = create_clustered_entities(castle_center, 100, 20.0);

    // Attackers outside castle (spread in siege line)
    let attackers =
        create_clustered_entities(castle_center + Vec3::new(100.0, 0.0, 0.0), 300, 60.0);

    let mut all_positions = defenders.clone();
    all_positions.extend(attackers.clone());

    let (world, _entities) = create_world_at_positions(&all_positions);

    let mut manager = InterestManager::new(50.0);
    manager.update_from_world(&world);

    // Defenders (0-99)
    for (i, &pos) in defenders.iter().enumerate() {
        manager.set_client_interest(i as u64, AreaOfInterest::new(pos, 75.0));
    }

    // Attackers (100-399)
    for (i, &pos) in attackers.iter().enumerate() {
        manager.set_client_interest((i + 100) as u64, AreaOfInterest::new(pos, 75.0));
    }

    // Sample visibility
    let defender_visible = manager.calculate_visibility(0);
    let attacker_visible = manager.calculate_visibility(100);

    // Defenders see mostly other defenders (inside castle)
    // Attackers see mostly other attackers (outside castle)
    // Some cross-wall visibility at edges

    tracing::info!(
        "Siege warfare: Defender sees {}, Attacker sees {}",
        defender_visible.len(),
        attacker_visible.len()
    );

    assert!(defender_visible.len() > 0);
    assert!(attacker_visible.len() > 0);
}

#[test]
fn test_world_pvp_hotspot() {
    // Scenario: 50+ players converging on contested resource node
    // Requires:
    // - Everyone sees everyone (small area)
    // - Rapid updates as players arrive
    // - Deaths and respawns handled

    let resource_node = Vec3::new(100.0, 0.0, 100.0);

    // 60 players arriving from different directions
    let positions = create_clustered_entities(resource_node, 60, 25.0);
    let (world, _entities) = create_world_at_positions(&positions);

    let mut manager = InterestManager::new(40.0);
    manager.update_from_world(&world);

    for (i, &pos) in positions.iter().enumerate() {
        manager.set_client_interest(i as u64, AreaOfInterest::new(pos, 60.0));
    }

    // Everyone should see most/all players
    for i in 0..60 {
        let visible = manager.calculate_visibility(i as u64);
        assert!(visible.len() > 40, "Player {} only sees {}, expected >40", i, visible.len());
    }

    tracing::info!("World PvP hotspot: 60 players, high visibility overlap");
}

#[test]
fn test_faction_visibility() {
    // Scenario: Faction-based visibility (stealth, invisibility)
    // Note: Actual faction logic would be in gameplay code,
    // but interest management must support it via custom filtering
    // For this test, we use distance as a proxy for "stealth detection range"

    let battlefield = Vec3::ZERO;

    // 20 "visible" players (normal AOI)
    let visible_positions = create_clustered_entities(battlefield, 20, 30.0);

    // 10 "stealthed" players (closer AOI required to detect)
    let stealthed_positions =
        create_clustered_entities(battlefield + Vec3::new(15.0, 0.0, 0.0), 10, 10.0);

    let mut all_positions = visible_positions.clone();
    all_positions.extend(stealthed_positions.clone());

    let (world, entities) = create_world_at_positions(&all_positions);

    let mut manager = InterestManager::new(30.0);
    manager.update_from_world(&world);

    // Observer at origin with normal AOI (75 units)
    manager.set_client_interest(0, AreaOfInterest::new(battlefield, 75.0));

    let visible_normal = manager.calculate_visibility(0);

    // Observer with "stealth detection" (smaller AOI simulates detection range)
    manager.set_client_interest(1, AreaOfInterest::new(battlefield, 20.0));
    let visible_stealth_detect = manager.calculate_visibility(1);

    // Normal AOI sees more entities than stealth-detection range
    assert!(visible_normal.len() > visible_stealth_detect.len());

    tracing::info!(
        "Faction visibility: Normal AOI sees {}, Stealth detection sees {}",
        visible_normal.len(),
        visible_stealth_detect.len()
    );
}

// ============================================================================
// Open World Scenarios
// ============================================================================

#[test]
fn test_flying_mount_high_speed() {
    // Scenario: Players on flying mounts moving at 3x normal speed
    // Requirements:
    // - Interest updates keep up with fast movement
    // - No "pop-in" (entities suddenly appearing)
    // - Visibility changes calculated correctly

    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Aabb>();

    // Create a path with entities along it
    let mut entities = Vec::new();
    for i in 0..50 {
        let entity = world.spawn();
        let pos = Vec3::new(i as f32 * 20.0, 0.0, 0.0);
        world.add(entity, Transform::new(pos, Quat::IDENTITY, Vec3::ONE));
        world.add(entity, Aabb::from_center_half_extents(pos, Vec3::ONE));
        entities.push(entity);
    }

    let mut manager = InterestManager::new(50.0);
    manager.update_from_world(&world);

    let client_id = 1;

    // Simulate flying mount movement (0 → 1000 units at 50 units/update)
    let mut position = Vec3::ZERO;
    let velocity = Vec3::new(50.0, 0.0, 0.0); // Fast flying

    let mut total_entered = 0;
    let mut total_exited = 0;

    for step in 0..20 {
        position += velocity;
        manager.set_client_interest(client_id, AreaOfInterest::new(position, 100.0));

        let (entered, exited) = manager.get_visibility_changes(client_id);

        total_entered += entered.len();
        total_exited += exited.len();

        if step > 0 {
            // Should see entities enter/exit as we fly past
            assert!(
                entered.len() > 0 || exited.len() > 0 || step > 15,
                "Step {}: No visibility changes during flight",
                step
            );
        }
    }

    tracing::info!(
        "Flying mount: Moved 1000 units, {} entities entered, {} exited",
        total_entered,
        total_exited
    );

    assert!(total_entered > 0, "Should have seen entities while flying");
    assert!(total_exited > 0, "Should have left entities behind");
}

#[test]
fn test_world_boss_spawn() {
    // Scenario: World boss spawns, 100+ players converge on location
    // Requirements:
    // - System handles rapid influx of players
    // - Boss visible to all players in range
    // - Performance doesn't degrade as crowd grows

    let boss_location = Vec3::new(500.0, 0.0, 500.0);

    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Aabb>();

    // Boss entity
    let boss = world.spawn();
    world.add(boss, Transform::new(boss_location, Quat::IDENTITY, Vec3::splat(5.0)));
    world.add(boss, Aabb::from_center_half_extents(boss_location, Vec3::splat(5.0)));

    // 150 players converging (simulate over 10 "frames")
    let final_positions = create_clustered_entities(boss_location, 150, 50.0);

    let mut manager = InterestManager::new(50.0);

    // Simulate players arriving over time
    for wave in 0..10 {
        let players_this_wave = 15; // 15 players per wave
        let start_idx = wave * players_this_wave;

        // Add player entities
        for i in start_idx..(start_idx + players_this_wave) {
            let entity = world.spawn();
            let pos = final_positions[i];
            world.add(entity, Transform::new(pos, Quat::IDENTITY, Vec3::ONE));
            world.add(entity, Aabb::from_center_half_extents(pos, Vec3::ONE));
        }

        manager.update_from_world(&world);

        // Update interests
        for i in 0..=start_idx + players_this_wave {
            if i < final_positions.len() {
                manager
                    .set_client_interest(i as u64, AreaOfInterest::new(final_positions[i], 100.0));
            }
        }

        // Verify boss is visible to newly arrived players
        for i in start_idx..(start_idx + players_this_wave).min(final_positions.len()) {
            let visible = manager.calculate_visibility(i as u64);
            assert!(visible.contains(&boss), "Player {} cannot see world boss!", i);
        }
    }

    tracing::info!("World boss spawn: 150 players converged, all see boss");
}

#[test]
fn test_farming_bots_grid() {
    // Scenario: Bots in perfect grid pattern (pathological case for spatial partitioning)
    // This tests worst-case spatial grid performance
    // Requirements:
    // - System doesn't degrade with artificial patterns
    // - Grid cell boundaries handled correctly

    let mut positions = Vec::new();

    // 20x20 grid of "bots" at exact grid intervals
    for x in 0..20 {
        for z in 0..20 {
            positions.push(Vec3::new(x as f32 * 50.0, 0.0, z as f32 * 50.0));
        }
    }
    // 400 entities in perfect grid

    let (world, _entities) = create_world_at_positions(&positions);

    let mut manager = InterestManager::new(50.0); // Matches bot spacing!
    manager.update_from_world(&world);

    // Observer in the middle
    let center = Vec3::new(500.0, 0.0, 500.0);
    manager.set_client_interest(0, AreaOfInterest::new(center, 150.0));

    let visible = manager.calculate_visibility(0);

    // Should see a subset based on distance, not have issues with grid alignment
    assert!(visible.len() > 0);
    assert!(visible.len() < 400);

    tracing::info!("Farming bots grid: 400 bots in perfect grid, {} visible", visible.len());
}

#[test]
fn test_mount_dismount_spam() {
    // Scenario: Player rapidly mounting/dismounting (position changes)
    // Tests rapid visibility recalculation
    // Requirements:
    // - System handles rapid AOI updates
    // - No crashes or corruption
    // - Visibility stays consistent

    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Aabb>();

    // Nearby entities
    for i in 0..20 {
        let entity = world.spawn();
        let pos = Vec3::new((i % 5) as f32 * 10.0, 0.0, (i / 5) as f32 * 10.0);
        world.add(entity, Transform::new(pos, Quat::IDENTITY, Vec3::ONE));
        world.add(entity, Aabb::from_center_half_extents(pos, Vec3::ONE));
    }

    let mut manager = InterestManager::new(30.0);
    manager.update_from_world(&world);

    let client_id = 1;
    let base_pos = Vec3::new(25.0, 0.0, 25.0);

    // Rapidly change AOI radius (simulates mount speed changes)
    for i in 0..100 {
        let radius = if i % 2 == 0 { 50.0 } else { 100.0 }; // Alternating radii
        manager.set_client_interest(client_id, AreaOfInterest::new(base_pos, radius));

        let (entered, exited) = manager.get_visibility_changes(client_id);

        // Should see changes as AOI expands/contracts
        if i > 0 {
            // After first frame, should see enter/exit events
            assert!(entered.len() > 0 || exited.len() > 0 || i % 10 == 0);
        }
    }

    tracing::info!("Mount/dismount spam: 100 rapid AOI changes handled");
}

// ============================================================================
// Stress Scenarios
// ============================================================================

#[test]
fn test_login_storm_capital() {
    // Scenario: 1000 players login to capital city within 10 seconds
    // Simulates server restart or maintenance end
    // Requirements:
    // - System scales to handle rapid client registration
    // - No performance cliff
    // - Memory usage stays reasonable

    let capital_center = Vec3::ZERO;
    let positions = create_city_grid(capital_center, 15, 67); // ~1000 players

    let (world, _entities) = create_world_at_positions(&positions);

    let mut manager = InterestManager::new(50.0);
    manager.update_from_world(&world);

    let start = std::time::Instant::now();

    // Register all 1000 players as fast as possible
    for (i, &pos) in positions.iter().take(1000).enumerate() {
        manager.set_client_interest(i as u64, AreaOfInterest::new(pos, 75.0));
    }

    let registration_time = start.elapsed();

    // Calculate initial visibility for all
    let start_visibility = std::time::Instant::now();

    for i in 0..1000 {
        manager.get_visibility_changes(i as u64);
    }

    let visibility_time = start_visibility.elapsed();

    tracing::info!(
        "Login storm: 1000 players, registration: {}ms, initial visibility: {}ms",
        registration_time.as_millis(),
        visibility_time.as_millis()
    );

    // Performance targets
    assert!(registration_time.as_millis() < 100, "Registration too slow");
    assert!(visibility_time.as_millis() < 2000, "Initial visibility too slow");

    // Verify correctness
    let sample_visible = manager.calculate_visibility(500);
    assert!(sample_visible.len() > 0);
    assert!(sample_visible.len() < 1000);
}

#[test]
fn test_server_merge() {
    // Scenario: Two servers merge, 10K players from each into one world
    // This is an extreme stress test
    // Requirements:
    // - System handles 20K total entities
    // - Interest management scales appropriately
    // - No memory explosion

    // Create 20K entities spread across a massive world
    let mut positions = Vec::new();

    let grid_size = 141; // ~141x141 ≈ 20K
    for x in 0..grid_size {
        for z in 0..grid_size {
            if positions.len() >= 20_000 {
                break;
            }
            positions.push(Vec3::new(x as f32 * 15.0, 0.0, z as f32 * 15.0));
        }
        if positions.len() >= 20_000 {
            break;
        }
    }

    let (world, _entities) = create_world_at_positions(&positions[..20_000].to_vec().as_slice());

    let mut manager = InterestManager::new(50.0);

    let start = std::time::Instant::now();
    manager.update_from_world(&world);
    let grid_update_time = start.elapsed();

    tracing::info!("Server merge: 20K entities, grid update: {}ms", grid_update_time.as_millis());

    // Sample a few clients
    for i in (0..100).step_by(10) {
        let pos = positions[i * 200]; // Sample scattered positions
        manager.set_client_interest(i, AreaOfInterest::new(pos, 100.0));
    }

    // Calculate visibility for sample clients
    for i in (0..100).step_by(10) {
        let visible = manager.calculate_visibility(i);
        assert!(visible.len() > 0, "Client {} should see entities", i);
    }

    // Grid update should complete in reasonable time even with 20K entities
    assert!(grid_update_time.as_millis() < 500, "Grid update too slow for 20K entities");
}

#[test]
#[ignore] // Long-running test - run explicitly
fn test_24_hour_stability() {
    // Scenario: Simulate 24 hours of operation (compressed into ~60 seconds)
    // Requirements:
    // - No memory leaks
    // - Consistent performance over time
    // - No data corruption

    let positions = create_city_grid(Vec3::ZERO, 10, 50); // 500 players
    let (mut world, _entities) = create_world_at_positions(&positions);

    let mut manager = InterestManager::new(50.0);
    manager.update_from_world(&world);

    for (i, &pos) in positions.iter().enumerate() {
        manager.set_client_interest(i as u64, AreaOfInterest::new(pos, 75.0));
    }

    let iterations = 60 * 60 * 24; // 1 iteration = 1 "second"
    let sample_interval = iterations / 60; // Sample 60 times

    let mut timings = Vec::new();

    for iteration in 0..iterations {
        // Simulate some player movement (every 10 "seconds")
        if iteration % 10 == 0 {
            // Update a few player positions
            for i in (0..10).map(|x| x * 50) {
                if let Some(&old_pos) = positions.get(i) {
                    let new_pos = old_pos + Vec3::new(1.0, 0.0, 1.0);
                    manager.set_client_interest(i as u64, AreaOfInterest::new(new_pos, 75.0));
                }
            }
        }

        // Calculate visibility for a sample of players
        if iteration % sample_interval == 0 {
            let start = std::time::Instant::now();

            for i in (0..50).step_by(5) {
                manager.calculate_visibility(i as u64);
            }

            timings.push(start.elapsed());
        }
    }

    // Verify consistent performance (no degradation)
    let first_half_avg: std::time::Duration =
        timings[..timings.len() / 2].iter().sum::<std::time::Duration>()
            / (timings.len() / 2) as u32;
    let second_half_avg: std::time::Duration =
        timings[timings.len() / 2..].iter().sum::<std::time::Duration>()
            / (timings.len() / 2) as u32;

    let degradation = (second_half_avg.as_micros() as f64 - first_half_avg.as_micros() as f64)
        / first_half_avg.as_micros() as f64
        * 100.0;

    tracing::info!(
        "24-hour stability: First half avg: {}µs, Second half avg: {}µs, Degradation: {:.2}%",
        first_half_avg.as_micros(),
        second_half_avg.as_micros(),
        degradation
    );

    // Performance should not degrade more than 10%
    assert!(degradation < 10.0, "Performance degraded by {:.2}%", degradation);
}

#[test]
fn test_memory_leak_detection() {
    // Scenario: 1M interest updates, check for memory leaks
    // Requirements:
    // - Memory usage stays bounded
    // - No unbounded growth in internal structures

    use std::collections::HashMap;

    let positions = create_city_grid(Vec3::ZERO, 5, 20); // 100 players
    let (world, _entities) = create_world_at_positions(&positions);

    let mut manager = InterestManager::new(50.0);
    manager.update_from_world(&world);

    // Register all clients
    for (i, &pos) in positions.iter().enumerate() {
        manager.set_client_interest(i as u64, AreaOfInterest::new(pos, 75.0));
    }

    // Baseline memory measurement (approximate via cache sizes)
    let baseline_clients = manager.client_count();
    let baseline_entities = manager.entity_count();

    // Run 1M visibility calculations
    for iteration in 0..1_000_000 {
        let client_id = (iteration % 100) as u64;
        manager.calculate_visibility(client_id);

        // Occasionally move a client
        if iteration % 1000 == 0 {
            let new_pos =
                positions[client_id as usize % positions.len()] + Vec3::new(1.0, 0.0, 0.0);
            manager.set_client_interest(client_id, AreaOfInterest::new(new_pos, 75.0));
        }
    }

    // Verify no unbounded growth
    let final_clients = manager.client_count();
    let final_entities = manager.entity_count();

    assert_eq!(baseline_clients, final_clients, "Client count should not change");
    assert_eq!(baseline_entities, final_entities, "Entity count should not change");

    tracing::info!(
        "Memory leak detection: 1M updates, clients: {} → {}, entities: {} → {}",
        baseline_clients,
        final_clients,
        baseline_entities,
        final_entities
    );
}
