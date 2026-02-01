# Spatial Queries Guide

Quick reference for using spatial data structures in the game engine.

## Overview

The engine provides three spatial query methods:

1. **Linear Search** - O(N) baseline, always correct
2. **Spatial Grid** - O(1) for nearby queries, best for uniform distributions
3. **BVH (Bounding Volume Hierarchy)** - O(log N), best for ray casts and non-uniform distributions

## Quick Start

### 1. Add AABB Component

```rust
use engine_core::{World, spatial::Aabb, math::Vec3};

let mut world = World::new();
world.register::<Aabb>();

let entity = world.spawn();
let aabb = Aabb::from_center_half_extents(
    Vec3::new(10.0, 0.0, 5.0),  // center
    Vec3::new(1.0, 2.0, 1.0)     // half-extents (width/2, height/2, depth/2)
);
world.add(entity, aabb);
```

### 2. Perform Queries

#### Radius Query (Find Nearby Entities)

```rust
use engine_core::spatial::{SpatialQuery, SpatialGridConfig};

// Option A: Linear search (baseline)
let nearby = world.spatial_query_radius_linear(
    Vec3::new(10.0, 0.0, 5.0),  // center
    15.0                          // radius
);

// Option B: Spatial grid (fastest for nearby)
let config = SpatialGridConfig::default();
let nearby = world.spatial_query_radius_grid(
    Vec3::new(10.0, 0.0, 5.0),
    15.0,
    config
);

// Option C: BVH (best for sparse/non-uniform)
let nearby = world.spatial_query_radius_bvh(
    Vec3::new(10.0, 0.0, 5.0),
    15.0
);
```

#### Ray Cast (Pick/Shoot)

```rust
// Ray from (0, 0, 0) pointing along +X axis
let origin = Vec3::ZERO;
let direction = Vec3::new(1.0, 0.0, 0.0);
let max_distance = 100.0;

let hits = world.spatial_raycast_bvh(origin, direction, max_distance);

// Hits are sorted by distance
for hit in hits {
    println!("Hit entity {:?} at distance {}", hit.entity, hit.distance);
    println!("Hit point: {:?}", hit.point);
}
```

#### AABB Query (Box Selection)

```rust
use engine_core::spatial::Aabb;

let selection_box = Aabb::new(
    Vec3::new(0.0, 0.0, 0.0),   // min corner
    Vec3::new(20.0, 10.0, 20.0) // max corner
);

let selected = world.spatial_query_aabb_bvh(&selection_box);
```

## When to Use Each Method

### Linear Search
**Use when:**
- Debugging (always correct)
- Few entities (< 100)
- Query everything

**Pros:**
- Simple, no overhead
- Always up-to-date

**Cons:**
- O(N) - slow for many entities

### Spatial Grid
**Use when:**
- Uniformly distributed entities
- Many nearby queries
- Entities rarely move
- Query radius ≈ cell size

**Pros:**
- O(1) for nearby queries
- Fast construction

**Cons:**
- Needs rebuild when entities move
- Poor for very sparse/clustered data

**Configuration:**
```rust
use engine_core::spatial::SpatialGridConfig;

let config = SpatialGridConfig {
    cell_size: 10.0,           // Larger = fewer cells, more entities per cell
    entities_per_cell: 16,     // Capacity hint (pre-allocation)
};
```

**Rules of thumb:**
- Set `cell_size` to 2-3x your typical query radius
- Increase `entities_per_cell` if cells are densely populated

### BVH (Bounding Volume Hierarchy)
**Use when:**
- Ray casting
- Frustum culling
- Non-uniform distributions
- Sparse entities

**Pros:**
- O(log N) queries
- Good for any distribution
- Excellent for ray casts

**Cons:**
- Slower construction than grid
- Needs rebuild when entities move

## Advanced Usage

### Reusing Structures

If entities don't move, build once and reuse:

```rust
use engine_core::spatial::{SpatialGrid, Bvh};

// Build once
let grid = SpatialGrid::build(&world, config);
let bvh = Bvh::build(&world);

// Query many times
for query_point in query_points {
    let nearby = grid.query_radius(query_point, radius);
    // Process results...
}
```

### Dynamic Updates

For moving entities, update the grid:

```rust
let mut grid = SpatialGrid::new(config);

// Insert entities
grid.insert(entity, aabb);

// Update when moved
let new_aabb = Aabb::from_center_half_extents(new_pos, half_extents);
grid.update(entity, old_aabb, new_aabb);

// Remove when deleted
grid.remove(entity, aabb);
```

### AABB from Transform

```rust
use engine_core::{math::Transform, spatial::Aabb};

fn compute_aabb(transform: &Transform, mesh_bounds: &Aabb) -> Aabb {
    // Transform mesh bounds to world space
    let min = transform.transform_point(mesh_bounds.min);
    let max = transform.transform_point(mesh_bounds.max);
    Aabb::new(min, max)
}
```

## Performance Tips

1. **Choose the right structure:**
   - Grid: Nearby queries, uniform distribution
   - BVH: Ray casts, non-uniform distribution
   - Linear: < 100 entities

2. **Tune cell size:**
   - Too small: Many cells, overhead
   - Too large: Many entities per cell, slow queries
   - Sweet spot: 2-3x query radius

3. **Batch queries:**
   - Build once, query many times
   - Amortize construction cost

4. **Profile:**
   ```bash
   cargo build --features profiling
   # Look for "spatial_grid_build", "spatial_query_radius", etc.
   ```

5. **Benchmark:**
   ```bash
   cargo bench --bench spatial_benches
   ```

## Examples

### Collision Detection System

```rust
use engine_core::{World, spatial::{SpatialGrid, SpatialGridConfig, SpatialQuery}};

fn collision_system(world: &World) {
    let grid = SpatialGrid::build(world, SpatialGridConfig::default());

    // For each entity, find nearby entities
    let storage = world.get_storage::<Aabb>().unwrap();
    for (entity, aabb) in storage.iter() {
        let nearby = grid.query_radius(aabb.center(), 5.0);

        for other in nearby {
            if entity != other {
                // Check actual collision
                if let Some(other_aabb) = world.get::<Aabb>(other) {
                    if aabb.intersects(other_aabb) {
                        // Handle collision
                    }
                }
            }
        }
    }
}
```

### Projectile Ray Cast

```rust
use engine_core::spatial::SpatialQuery;
use engine_core::math::Vec3;

fn shoot_raycast(world: &World, origin: Vec3, direction: Vec3) -> Option<Entity> {
    let hits = world.spatial_raycast_bvh(origin, direction, 1000.0);

    // Return first hit (sorted by distance)
    hits.first().map(|hit| {
        println!("Hit at distance: {}", hit.distance);
        hit.entity
    })
}
```

### Interest Management (MMO)

```rust
use engine_core::spatial::{SpatialGrid, SpatialGridConfig};

fn get_visible_entities(
    world: &World,
    player_pos: Vec3,
    view_distance: f32
) -> Vec<Entity> {
    let grid = SpatialGrid::build(world, SpatialGridConfig {
        cell_size: view_distance / 2.0,
        entities_per_cell: 50,
    });

    grid.query_radius(player_pos, view_distance)
}
```

## Testing

All spatial methods are tested for consistency:

```rust
#[test]
fn verify_consistency() {
    // Grid, BVH, and linear should find same entities
    let grid_results = world.spatial_query_radius_grid(center, radius, config);
    let bvh_results = world.spatial_query_radius_bvh(center, radius);
    let linear_results = world.spatial_query_radius_linear(center, radius);

    assert_eq!(grid_results.len(), linear_results.len());
    assert_eq!(bvh_results.len(), linear_results.len());
}
```

Run tests:
```bash
cargo test --package engine-core spatial
cargo test --package engine-core --test spatial_integration_test
```

Run benchmarks:
```bash
cargo bench --bench spatial_benches
```

## API Reference

See rustdoc for full API documentation:
```bash
cargo doc --package engine-core --open
```

Navigate to `engine_core::spatial` module.

## Related

- [ECS Documentation](ecs.md)
- [Performance Targets](performance-targets.md)
- [Profiling Guide](profiling.md)
