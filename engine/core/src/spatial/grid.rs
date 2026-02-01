//! Spatial grid for uniform entity distributions.
//!
//! Provides O(1) average-case performance for nearby queries
//! when entities are uniformly distributed.

use crate::ecs::Entity;
use crate::math::Vec3;
use crate::spatial::Aabb;
use std::collections::HashMap;

/// Configuration for spatial grid.
#[derive(Debug, Clone, Copy)]
pub struct SpatialGridConfig {
    /// Size of each grid cell.
    pub cell_size: f32,
    /// Expected number of entities per cell (for capacity hints).
    pub entities_per_cell: usize,
}

impl Default for SpatialGridConfig {
    fn default() -> Self {
        Self {
            cell_size: 10.0,
            entities_per_cell: 16,
        }
    }
}

/// 3D grid coordinate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct GridCell {
    x: i32,
    y: i32,
    z: i32,
}

impl GridCell {
    #[inline]
    fn from_position(pos: Vec3, cell_size: f32) -> Self {
        let inv_size = 1.0 / cell_size;
        Self {
            x: (pos.x * inv_size).floor() as i32,
            y: (pos.y * inv_size).floor() as i32,
            z: (pos.z * inv_size).floor() as i32,
        }
    }

    #[inline]
    #[allow(dead_code)]
    fn neighbors(&self) -> [GridCell; 27] {
        let mut cells = [*self; 27];
        let mut idx = 0;
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    cells[idx] = GridCell {
                        x: self.x + dx,
                        y: self.y + dy,
                        z: self.z + dz,
                    };
                    idx += 1;
                }
            }
        }
        cells
    }
}

/// Spatial grid for efficient nearby queries.
///
/// Best for uniformly distributed entities. Provides O(1) average-case
/// performance for radius queries when radius <= cell_size.
///
/// # Examples
///
/// ```
/// # use engine_core::spatial::{SpatialGrid, SpatialGridConfig, Aabb};
/// # use engine_core::ecs::World;
/// # use engine_core::math::Vec3;
/// let mut world = World::new();
/// world.register::<Aabb>();
///
/// // Spawn entities
/// for i in 0..100 {
///     let entity = world.spawn();
///     let pos = Vec3::new((i % 10) as f32 * 2.0, 0.0, (i / 10) as f32 * 2.0);
///     let aabb = Aabb::from_center_half_extents(pos, Vec3::new(0.5, 0.5, 0.5));
///     world.add(entity, aabb);
/// }
///
/// // Build spatial grid
/// let config = SpatialGridConfig::default();
/// let grid = SpatialGrid::build(&world, config);
///
/// // Query nearby entities
/// let nearby = grid.query_radius(Vec3::ZERO, 5.0);
/// assert!(!nearby.is_empty());
/// ```
pub struct SpatialGrid {
    /// Grid cells mapping to entities.
    cells: HashMap<GridCell, Vec<(Entity, Aabb)>>,
    /// Configuration.
    config: SpatialGridConfig,
    /// Total entity count.
    entity_count: usize,
}

impl SpatialGrid {
    /// Create a new empty spatial grid.
    pub fn new(config: SpatialGridConfig) -> Self {
        Self {
            cells: HashMap::new(),
            config,
            entity_count: 0,
        }
    }

    /// Build a spatial grid from all entities with Aabb components.
    pub fn build(world: &crate::ecs::World, config: SpatialGridConfig) -> Self {
        #[cfg(feature = "profiling")]
        agent_game_engine_profiling::profile_scope!("spatial_grid_build", agent_game_engine_profiling::ProfileCategory::Physics);

        let storage = match world.get_storage::<Aabb>() {
            Some(s) => s,
            None => {
                return Self {
                    cells: HashMap::new(),
                    config,
                    entity_count: 0,
                }
            }
        };

        let mut grid = Self::new(config);

        for (entity, aabb) in storage.iter() {
            grid.insert(entity, *aabb);
        }

        grid
    }

    /// Insert an entity into the grid.
    pub fn insert(&mut self, entity: Entity, aabb: Aabb) {
        let center = aabb.center();
        let cell = GridCell::from_position(center, self.config.cell_size);

        self.cells
            .entry(cell)
            .or_insert_with(|| Vec::with_capacity(self.config.entities_per_cell))
            .push((entity, aabb));

        self.entity_count += 1;
    }

    /// Remove an entity from the grid.
    pub fn remove(&mut self, entity: Entity, aabb: Aabb) -> bool {
        let center = aabb.center();
        let cell = GridCell::from_position(center, self.config.cell_size);

        if let Some(entities) = self.cells.get_mut(&cell) {
            if let Some(pos) = entities.iter().position(|(e, _)| *e == entity) {
                entities.swap_remove(pos);
                self.entity_count -= 1;

                // Remove empty cells
                if entities.is_empty() {
                    self.cells.remove(&cell);
                }

                return true;
            }
        }

        false
    }

    /// Update an entity's position in the grid.
    ///
    /// This is more efficient than remove + insert if the entity
    /// stays in the same cell.
    pub fn update(&mut self, entity: Entity, old_aabb: Aabb, new_aabb: Aabb) {
        let old_cell = GridCell::from_position(old_aabb.center(), self.config.cell_size);
        let new_cell = GridCell::from_position(new_aabb.center(), self.config.cell_size);

        if old_cell == new_cell {
            // Entity stayed in same cell - just update AABB
            if let Some(entities) = self.cells.get_mut(&old_cell) {
                if let Some(entry) = entities.iter_mut().find(|(e, _)| *e == entity) {
                    entry.1 = new_aabb;
                }
            }
        } else {
            // Entity moved to different cell
            self.remove(entity, old_aabb);
            self.insert(entity, new_aabb);
        }
    }

    /// Find all entities within a radius of a point.
    ///
    /// O(1) average case when radius <= cell_size.
    /// O(k) where k is the number of cells intersected for larger radii.
    pub fn query_radius(&self, center: Vec3, radius: f32) -> Vec<Entity> {
        #[cfg(feature = "profiling")]
        agent_game_engine_profiling::profile_scope!("spatial_grid_query_radius", agent_game_engine_profiling::ProfileCategory::Physics);

        let mut results = Vec::new();
        let radius_sq = radius * radius;

        // Determine which cells to check
        let center_cell = GridCell::from_position(center, self.config.cell_size);
        let cell_radius = (radius / self.config.cell_size).ceil() as i32;

        for dx in -cell_radius..=cell_radius {
            for dy in -cell_radius..=cell_radius {
                for dz in -cell_radius..=cell_radius {
                    let cell = GridCell {
                        x: center_cell.x + dx,
                        y: center_cell.y + dy,
                        z: center_cell.z + dz,
                    };

                    if let Some(entities) = self.cells.get(&cell) {
                        for (entity, aabb) in entities {
                            // Check actual distance to AABB
                            if aabb.distance_squared_to_point(center) <= radius_sq {
                                results.push(*entity);
                            }
                        }
                    }
                }
            }
        }

        results
    }

    /// Find all entities within an AABB.
    pub fn query_aabb(&self, aabb: &Aabb) -> Vec<Entity> {
        #[cfg(feature = "profiling")]
        agent_game_engine_profiling::profile_scope!("spatial_grid_query_aabb", agent_game_engine_profiling::ProfileCategory::Physics);

        let mut results = Vec::new();

        // Determine cells that intersect the AABB
        let min_cell = GridCell::from_position(aabb.min, self.config.cell_size);
        let max_cell = GridCell::from_position(aabb.max, self.config.cell_size);

        for x in min_cell.x..=max_cell.x {
            for y in min_cell.y..=max_cell.y {
                for z in min_cell.z..=max_cell.z {
                    let cell = GridCell { x, y, z };

                    if let Some(entities) = self.cells.get(&cell) {
                        for (entity, entity_aabb) in entities {
                            if entity_aabb.intersects(aabb) {
                                results.push(*entity);
                            }
                        }
                    }
                }
            }
        }

        results
    }

    /// Get all entities in a specific cell (for debugging).
    pub fn get_cell(&self, position: Vec3) -> Option<&Vec<(Entity, Aabb)>> {
        let cell = GridCell::from_position(position, self.config.cell_size);
        self.cells.get(&cell)
    }

    /// Get the total number of entities in the grid.
    #[inline]
    pub fn entity_count(&self) -> usize {
        self.entity_count
    }

    /// Get the number of occupied cells.
    #[inline]
    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }

    /// Clear all entities from the grid.
    pub fn clear(&mut self) {
        self.cells.clear();
        self.entity_count = 0;
    }

    /// Check if the grid is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entity_count == 0
    }

    /// Get grid statistics (for debugging and optimization).
    pub fn stats(&self) -> SpatialGridStats {
        let mut min_entities = usize::MAX;
        let mut max_entities = 0;
        let mut total_entities = 0;

        for entities in self.cells.values() {
            let count = entities.len();
            min_entities = min_entities.min(count);
            max_entities = max_entities.max(count);
            total_entities += count;
        }

        let avg_entities = if !self.cells.is_empty() {
            total_entities as f32 / self.cells.len() as f32
        } else {
            0.0
        };

        SpatialGridStats {
            cell_count: self.cells.len(),
            entity_count: self.entity_count,
            min_entities_per_cell: if self.cells.is_empty() { 0 } else { min_entities },
            max_entities_per_cell: max_entities,
            avg_entities_per_cell: avg_entities,
            cell_size: self.config.cell_size,
        }
    }
}

/// Statistics about spatial grid distribution.
#[derive(Debug, Clone)]
pub struct SpatialGridStats {
    /// Number of occupied cells.
    pub cell_count: usize,
    /// Total number of entities.
    pub entity_count: usize,
    /// Minimum entities in any cell.
    pub min_entities_per_cell: usize,
    /// Maximum entities in any cell.
    pub max_entities_per_cell: usize,
    /// Average entities per cell.
    pub avg_entities_per_cell: f32,
    /// Cell size.
    pub cell_size: f32,
}

impl Default for SpatialGrid {
    fn default() -> Self {
        Self::new(SpatialGridConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::World;

    #[test]
    fn test_spatial_grid_insert() {
        let mut grid = SpatialGrid::new(SpatialGridConfig::default());
        let mut world = World::new();

        let entity = world.spawn();
        let aabb = Aabb::from_center_half_extents(Vec3::ZERO, Vec3::ONE);

        grid.insert(entity, aabb);
        assert_eq!(grid.entity_count(), 1);
    }

    #[test]
    fn test_spatial_grid_query_radius() {
        let mut world = World::new();
        world.register::<Aabb>();

        // Create entities in a grid
        for x in 0..10 {
            for z in 0..10 {
                let entity = world.spawn();
                let pos = Vec3::new(x as f32 * 2.0, 0.0, z as f32 * 2.0);
                let aabb = Aabb::from_center_half_extents(pos, Vec3::new(0.5, 0.5, 0.5));
                world.add(entity, aabb);
            }
        }

        let config = SpatialGridConfig {
            cell_size: 5.0,
            entities_per_cell: 4,
        };
        let grid = SpatialGrid::build(&world, config);

        // Query small radius
        let results = grid.query_radius(Vec3::ZERO, 3.0);
        assert!(results.len() > 0);
        assert!(results.len() < 100);

        // Query large radius
        let results = grid.query_radius(Vec3::new(10.0, 0.0, 10.0), 30.0);
        assert!(results.len() > 50);
    }

    #[test]
    fn test_spatial_grid_remove() {
        let mut grid = SpatialGrid::new(SpatialGridConfig::default());
        let mut world = World::new();

        let entity = world.spawn();
        let aabb = Aabb::from_center_half_extents(Vec3::ZERO, Vec3::ONE);

        grid.insert(entity, aabb);
        assert_eq!(grid.entity_count(), 1);

        assert!(grid.remove(entity, aabb));
        assert_eq!(grid.entity_count(), 0);
    }

    #[test]
    fn test_spatial_grid_update_same_cell() {
        let mut grid = SpatialGrid::new(SpatialGridConfig { cell_size: 10.0, entities_per_cell: 4 });
        let mut world = World::new();

        let entity = world.spawn();
        let old_aabb = Aabb::from_center_half_extents(Vec3::ZERO, Vec3::ONE);
        let new_aabb = Aabb::from_center_half_extents(Vec3::new(1.0, 0.0, 0.0), Vec3::ONE);

        grid.insert(entity, old_aabb);
        grid.update(entity, old_aabb, new_aabb);

        assert_eq!(grid.entity_count(), 1);
        assert_eq!(grid.cell_count(), 1);
    }

    #[test]
    fn test_spatial_grid_stats() {
        let mut world = World::new();
        world.register::<Aabb>();

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
    }
}
