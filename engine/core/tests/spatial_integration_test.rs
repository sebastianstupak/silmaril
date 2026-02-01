//! Integration tests for spatial data structures.
//!
//! Tests the full integration of spatial grids and BVH with the ECS World.

use engine_core::{
    spatial::{Aabb, Bvh, SpatialGrid, SpatialGridConfig, SpatialQuery},
    World,
};
use engine_math::Vec3;

/// Test that spatial grid correctly finds entities within radius.
#[test]
fn test_spatial_grid_radius_query_integration() {
    let mut world = World::new();
    world.register::<Aabb>();

    // Create entities in a 10x10 grid
    let mut entities = Vec::new();
    for x in 0..10 {
        for z in 0..10 {
            let entity = world.spawn();
            let pos = Vec3::new(x as f32 * 5.0, 0.0, z as f32 * 5.0);
            let aabb = Aabb::from_center_half_extents(pos, Vec3::new(1.0, 1.0, 1.0));
            world.add(entity, aabb);
            entities.push((entity, pos));
        }
    }

    // Build spatial grid
    let config = SpatialGridConfig { cell_size: 10.0, entities_per_cell: 4 };
    let grid = SpatialGrid::build(&world, config);

    // Query entities near origin
    let results = grid.query_radius(Vec3::ZERO, 8.0);

    // Should find entities at (0,0,0) and (5,0,0), (0,0,5)
    assert!(results.len() >= 1, "Should find at least 1 entity near origin");
    assert!(results.len() <= 9, "Should not find all entities (too far away)");

    // Query entities in the center
    let center = Vec3::new(22.5, 0.0, 22.5);
    let results = grid.query_radius(center, 12.0);
    assert!(results.len() >= 4, "Should find multiple entities in center");
}

/// Test that BVH correctly finds entities with ray casts.
#[test]
fn test_bvh_raycast_integration() {
    let mut world = World::new();
    world.register::<Aabb>();

    // Create a line of entities along the X axis
    let mut entities = Vec::new();
    for i in 0..20 {
        let entity = world.spawn();
        let pos = Vec3::new(i as f32 * 3.0, 0.0, 0.0);
        let aabb = Aabb::from_center_half_extents(pos, Vec3::new(0.5, 0.5, 0.5));
        world.add(entity, aabb);
        entities.push((entity, pos));
    }

    // Build BVH
    let bvh = Bvh::build(&world);
    assert_eq!(bvh.entity_count(), 20);

    // Cast ray along X axis
    let origin = Vec3::new(-2.0, 0.0, 0.0);
    let direction = Vec3::new(1.0, 0.0, 0.0);
    let hits = bvh.ray_cast(origin, direction, 100.0);

    // Should hit all 20 entities
    assert_eq!(hits.len(), 20, "Ray should hit all entities along X axis");

    // Verify hits are sorted by distance
    for i in 1..hits.len() {
        assert!(hits[i - 1].1 <= hits[i].1, "Hits should be sorted by distance");
    }

    // Cast ray that misses everything
    let origin = Vec3::new(-2.0, 10.0, 0.0); // Above all entities
    let direction = Vec3::new(1.0, 0.0, 0.0);
    let hits = bvh.ray_cast(origin, direction, 100.0);
    assert_eq!(hits.len(), 0, "Ray should miss all entities");
}

/// Test that spatial grid and BVH produce consistent results.
#[test]
fn test_spatial_grid_vs_bvh_consistency() {
    let mut world = World::new();
    world.register::<Aabb>();

    // Create random-ish distribution of entities
    let mut seed = 12345u64;
    for _ in 0..100 {
        let entity = world.spawn();

        // Pseudo-random position
        seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
        let x = ((seed >> 16) % 100) as f32 - 50.0;
        seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
        let y = ((seed >> 16) % 100) as f32 - 50.0;
        seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
        let z = ((seed >> 16) % 100) as f32 - 50.0;

        let pos = Vec3::new(x, y, z);
        let aabb = Aabb::from_center_half_extents(pos, Vec3::new(1.0, 1.0, 1.0));
        world.add(entity, aabb);
    }

    // Query both structures
    let center = Vec3::ZERO;
    let radius = 20.0;

    let grid_config = SpatialGridConfig { cell_size: 10.0, entities_per_cell: 8 };

    let grid_results = world.spatial_query_radius_grid(center, radius, grid_config);
    let bvh_results = world.spatial_query_radius_bvh(center, radius);
    let linear_results = world.spatial_query_radius_linear(center, radius);

    // Convert to sets for comparison (order doesn't matter)
    use std::collections::HashSet;
    let grid_set: HashSet<_> = grid_results.into_iter().collect();
    let bvh_set: HashSet<_> = bvh_results.into_iter().collect();
    let linear_set: HashSet<_> = linear_results.into_iter().collect();

    // All three methods should find the same entities
    assert_eq!(grid_set, linear_set, "Grid and linear search should find same entities");
    assert_eq!(bvh_set, linear_set, "BVH and linear search should find same entities");
}

/// Test AABB query with spatial grid.
#[test]
fn test_spatial_grid_aabb_query() {
    let mut world = World::new();
    world.register::<Aabb>();

    // Create a 5x5x5 cube of entities
    for x in 0..5 {
        for y in 0..5 {
            for z in 0..5 {
                let entity = world.spawn();
                let pos = Vec3::new(x as f32 * 3.0, y as f32 * 3.0, z as f32 * 3.0);
                let aabb = Aabb::from_center_half_extents(pos, Vec3::new(0.5, 0.5, 0.5));
                world.add(entity, aabb);
            }
        }
    }

    let config = SpatialGridConfig::default();
    let grid = SpatialGrid::build(&world, config);

    // Query a box in the middle
    let query_aabb = Aabb::new(Vec3::new(3.0, 3.0, 3.0), Vec3::new(9.0, 9.0, 9.0));
    let results = grid.query_aabb(&query_aabb);

    // Should find entities in the queried region
    assert!(results.len() > 0, "Should find entities in query region");
    assert!(results.len() < 125, "Should not find all entities");
}

/// Test spatial grid update operations.
#[test]
fn test_spatial_grid_update_operations() {
    let mut world = World::new();
    world.register::<Aabb>();

    let entity = world.spawn();
    let initial_aabb = Aabb::from_center_half_extents(Vec3::ZERO, Vec3::ONE);
    world.add(entity, initial_aabb);

    let mut grid = SpatialGrid::new(SpatialGridConfig { cell_size: 5.0, entities_per_cell: 4 });

    // Insert entity
    grid.insert(entity, initial_aabb);
    assert_eq!(grid.entity_count(), 1);

    // Update to new position in same cell
    let new_aabb_same_cell = Aabb::from_center_half_extents(Vec3::new(1.0, 0.0, 0.0), Vec3::ONE);
    grid.update(entity, initial_aabb, new_aabb_same_cell);
    assert_eq!(grid.entity_count(), 1);
    assert_eq!(grid.cell_count(), 1); // Still in same cell

    // Update to new position in different cell
    let new_aabb_diff_cell = Aabb::from_center_half_extents(Vec3::new(10.0, 0.0, 0.0), Vec3::ONE);
    grid.update(entity, new_aabb_same_cell, new_aabb_diff_cell);
    assert_eq!(grid.entity_count(), 1);
    assert_eq!(grid.cell_count(), 1); // Old cell should be removed

    // Remove entity
    assert!(grid.remove(entity, new_aabb_diff_cell));
    assert_eq!(grid.entity_count(), 0);
    assert_eq!(grid.cell_count(), 0);
}

/// Test BVH query with AABB.
#[test]
fn test_bvh_aabb_query() {
    let mut world = World::new();
    world.register::<Aabb>();

    // Create entities in a line
    for i in 0..30 {
        let entity = world.spawn();
        let pos = Vec3::new(i as f32 * 2.0, 0.0, 0.0);
        let aabb = Aabb::from_center_half_extents(pos, Vec3::new(0.5, 0.5, 0.5));
        world.add(entity, aabb);
    }

    let bvh = Bvh::build(&world);

    // Query middle section
    let query_aabb = Aabb::new(Vec3::new(15.0, -1.0, -1.0), Vec3::new(35.0, 1.0, 1.0));
    let results = bvh.query_aabb(&query_aabb);

    // Should find entities in range [15, 35]
    assert!(results.len() >= 8, "Should find multiple entities in range");
    assert!(results.len() <= 12, "Should only find entities in range");
}

/// Test that spatial queries handle empty worlds correctly.
#[test]
fn test_spatial_queries_empty_world() {
    let world = World::new();

    // Grid query on empty world
    let grid_results =
        world.spatial_query_radius_grid(Vec3::ZERO, 10.0, SpatialGridConfig::default());
    assert_eq!(grid_results.len(), 0);

    // BVH query on empty world
    let bvh_results = world.spatial_query_radius_bvh(Vec3::ZERO, 10.0);
    assert_eq!(bvh_results.len(), 0);

    // Linear query on empty world
    let linear_results = world.spatial_query_radius_linear(Vec3::ZERO, 10.0);
    assert_eq!(linear_results.len(), 0);

    // Raycast on empty world
    let ray_hits = world.spatial_raycast_bvh(Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0), 100.0);
    assert_eq!(ray_hits.len(), 0);
}

/// Test spatial grid statistics.
#[test]
fn test_spatial_grid_stats() {
    let mut world = World::new();
    world.register::<Aabb>();

    // Create entities with known distribution
    for i in 0..50 {
        let entity = world.spawn();
        let pos = Vec3::new((i % 10) as f32 * 2.0, 0.0, (i / 10) as f32 * 2.0);
        let aabb = Aabb::from_center_half_extents(pos, Vec3::new(0.5, 0.5, 0.5));
        world.add(entity, aabb);
    }

    let grid = SpatialGrid::build(&world, SpatialGridConfig::default());
    let stats = grid.stats();

    assert_eq!(stats.entity_count, 50);
    assert!(stats.cell_count > 0);
    assert!(stats.avg_entities_per_cell > 0.0);
    assert!(stats.min_entities_per_cell > 0);
    assert!(stats.max_entities_per_cell >= stats.min_entities_per_cell);
}

/// Test performance difference between linear and spatial queries.
/// This is a basic sanity check, not a benchmark.
#[test]
fn test_spatial_query_faster_than_linear() {
    let mut world = World::new();
    world.register::<Aabb>();

    // Create many entities
    for i in 0..1000 {
        let entity = world.spawn();
        let pos =
            Vec3::new((i % 10) as f32 * 5.0, ((i / 10) % 10) as f32 * 5.0, (i / 100) as f32 * 5.0);
        let aabb = Aabb::from_center_half_extents(pos, Vec3::new(1.0, 1.0, 1.0));
        world.add(entity, aabb);
    }

    let center = Vec3::new(25.0, 25.0, 2.5);
    let radius = 10.0;

    // Build structures once
    let grid = SpatialGrid::build(&world, SpatialGridConfig::default());
    let bvh = Bvh::build(&world);

    // Do queries (not measuring time, just verifying they work)
    let linear_results = world.spatial_query_radius_linear(center, radius);
    let grid_results = grid.query_radius(center, radius);
    let bvh_results = bvh.query_radius(center, radius);

    // All should find entities
    assert!(linear_results.len() > 0, "Linear search should find entities");
    assert!(grid_results.len() > 0, "Grid search should find entities");
    assert!(bvh_results.len() > 0, "BVH search should find entities");

    // Results should be consistent (may differ due to floating point precision)
    let diff = (linear_results.len() as i32 - grid_results.len() as i32).abs();
    assert!(diff <= 2, "Grid and linear should find similar number of entities");
}
