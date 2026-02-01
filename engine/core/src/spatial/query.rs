//! Spatial query API for the ECS World.

use crate::ecs::{Entity, World};
use crate::math::Vec3;
use crate::spatial::{Aabb, Bvh, SpatialGrid, SpatialGridConfig};

/// Ray cast query parameters.
#[derive(Debug, Clone, Copy)]
pub struct RayCast {
    /// Ray origin in world space.
    pub origin: Vec3,
    /// Ray direction (should be normalized).
    pub direction: Vec3,
    /// Maximum distance to check.
    pub max_distance: f32,
}

impl RayCast {
    /// Create a new ray cast query.
    pub fn new(origin: Vec3, direction: Vec3, max_distance: f32) -> Self {
        Self {
            origin,
            direction,
            max_distance,
        }
    }
}

/// Ray hit result.
#[derive(Debug, Clone, Copy)]
pub struct RayHit {
    /// Entity that was hit.
    pub entity: Entity,
    /// Distance along the ray to the hit point.
    pub distance: f32,
    /// Hit point in world space.
    pub point: Vec3,
}

impl RayHit {
    /// Create a new ray hit.
    pub fn new(entity: Entity, distance: f32, origin: Vec3, direction: Vec3) -> Self {
        Self {
            entity,
            distance,
            point: origin + direction * distance,
        }
    }
}

/// Spatial query interface for the World.
///
/// This trait extends the World with spatial query methods.
/// It's implemented as an extension trait to keep the core World API clean.
pub trait SpatialQuery {
    /// Find all entities within a radius of a point using linear search.
    ///
    /// This is the baseline implementation - use BVH or SpatialGrid for better performance.
    fn spatial_query_radius_linear(&self, center: Vec3, radius: f32) -> Vec<Entity>;

    /// Find all entities within a radius using BVH.
    ///
    /// Builds a BVH on-demand and queries it. For repeated queries,
    /// consider building the BVH once and reusing it.
    fn spatial_query_radius_bvh(&self, center: Vec3, radius: f32) -> Vec<Entity>;

    /// Find all entities within a radius using spatial grid.
    ///
    /// Builds a spatial grid on-demand and queries it.
    fn spatial_query_radius_grid(&self, center: Vec3, radius: f32, config: SpatialGridConfig) -> Vec<Entity>;

    /// Perform a ray cast using linear search.
    fn spatial_raycast_linear(&self, origin: Vec3, direction: Vec3, max_distance: f32) -> Vec<RayHit>;

    /// Perform a ray cast using BVH.
    ///
    /// Returns hits sorted by distance from origin.
    fn spatial_raycast_bvh(&self, origin: Vec3, direction: Vec3, max_distance: f32) -> Vec<RayHit>;

    /// Find all entities within an AABB using linear search.
    fn spatial_query_aabb_linear(&self, aabb: &Aabb) -> Vec<Entity>;

    /// Find all entities within an AABB using BVH.
    fn spatial_query_aabb_bvh(&self, aabb: &Aabb) -> Vec<Entity>;

    /// Find all entities within an AABB using spatial grid.
    fn spatial_query_aabb_grid(&self, aabb: &Aabb, config: SpatialGridConfig) -> Vec<Entity>;
}

impl SpatialQuery for World {
    fn spatial_query_radius_linear(&self, center: Vec3, radius: f32) -> Vec<Entity> {
        #[cfg(feature = "profiling")]
        agent_game_engine_profiling::profile_scope!("spatial_query_radius_linear", agent_game_engine_profiling::ProfileCategory::Physics);

        let storage = match self.get_storage::<Aabb>() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let radius_sq = radius * radius;
        let mut results = Vec::new();

        for (entity, aabb) in storage.iter() {
            if aabb.distance_squared_to_point(center) <= radius_sq {
                results.push(entity);
            }
        }

        results
    }

    fn spatial_query_radius_bvh(&self, center: Vec3, radius: f32) -> Vec<Entity> {
        #[cfg(feature = "profiling")]
        agent_game_engine_profiling::profile_scope!("spatial_query_radius_bvh", agent_game_engine_profiling::ProfileCategory::Physics);

        let bvh = Bvh::build(self);
        bvh.query_radius(center, radius)
    }

    fn spatial_query_radius_grid(&self, center: Vec3, radius: f32, config: SpatialGridConfig) -> Vec<Entity> {
        #[cfg(feature = "profiling")]
        agent_game_engine_profiling::profile_scope!("spatial_query_radius_grid", agent_game_engine_profiling::ProfileCategory::Physics);

        let grid = SpatialGrid::build(self, config);
        grid.query_radius(center, radius)
    }

    fn spatial_raycast_linear(&self, origin: Vec3, direction: Vec3, max_distance: f32) -> Vec<RayHit> {
        #[cfg(feature = "profiling")]
        agent_game_engine_profiling::profile_scope!("spatial_raycast_linear", agent_game_engine_profiling::ProfileCategory::Physics);

        let storage = match self.get_storage::<Aabb>() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let mut hits = Vec::new();

        for (entity, aabb) in storage.iter() {
            if let Some((distance, _)) = aabb.ray_intersection(origin, direction, max_distance) {
                hits.push(RayHit::new(entity, distance, origin, direction));
            }
        }

        // Sort by distance
        hits.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap_or(std::cmp::Ordering::Equal));

        hits
    }

    fn spatial_raycast_bvh(&self, origin: Vec3, direction: Vec3, max_distance: f32) -> Vec<RayHit> {
        #[cfg(feature = "profiling")]
        agent_game_engine_profiling::profile_scope!("spatial_raycast_bvh", agent_game_engine_profiling::ProfileCategory::Physics);

        let bvh = Bvh::build(self);
        let hits = bvh.ray_cast(origin, direction, max_distance);

        hits.into_iter()
            .map(|(entity, distance)| RayHit::new(entity, distance, origin, direction))
            .collect()
    }

    fn spatial_query_aabb_linear(&self, aabb: &Aabb) -> Vec<Entity> {
        #[cfg(feature = "profiling")]
        agent_game_engine_profiling::profile_scope!("spatial_query_aabb_linear", agent_game_engine_profiling::ProfileCategory::Physics);

        let storage = match self.get_storage::<Aabb>() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let mut results = Vec::new();

        for (entity, entity_aabb) in storage.iter() {
            if entity_aabb.intersects(aabb) {
                results.push(entity);
            }
        }

        results
    }

    fn spatial_query_aabb_bvh(&self, aabb: &Aabb) -> Vec<Entity> {
        #[cfg(feature = "profiling")]
        agent_game_engine_profiling::profile_scope!("spatial_query_aabb_bvh", agent_game_engine_profiling::ProfileCategory::Physics);

        let bvh = Bvh::build(self);
        bvh.query_aabb(aabb)
    }

    fn spatial_query_aabb_grid(&self, aabb: &Aabb, config: SpatialGridConfig) -> Vec<Entity> {
        #[cfg(feature = "profiling")]
        agent_game_engine_profiling::profile_scope!("spatial_query_aabb_grid", agent_game_engine_profiling::ProfileCategory::Physics);

        let grid = SpatialGrid::build(self, config);
        grid.query_aabb(aabb)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raycast_new() {
        let ray = RayCast::new(Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0), 100.0);
        assert_eq!(ray.origin, Vec3::ZERO);
        assert_eq!(ray.max_distance, 100.0);
    }

    #[test]
    fn test_spatial_query_radius_linear() {
        let mut world = World::new();
        world.register::<Aabb>();

        // Create entities
        for i in 0..10 {
            let entity = world.spawn();
            let pos = Vec3::new(i as f32, 0.0, 0.0);
            let aabb = Aabb::from_center_half_extents(pos, Vec3::new(0.5, 0.5, 0.5));
            world.add(entity, aabb);
        }

        let results = world.spatial_query_radius_linear(Vec3::ZERO, 2.0);
        assert!(results.len() > 0);
        assert!(results.len() < 10);
    }

    #[test]
    fn test_spatial_raycast_linear() {
        let mut world = World::new();
        world.register::<Aabb>();

        // Create entities along X axis
        for i in 0..10 {
            let entity = world.spawn();
            let pos = Vec3::new(i as f32 * 2.0, 0.0, 0.0);
            let aabb = Aabb::from_center_half_extents(pos, Vec3::new(0.5, 0.5, 0.5));
            world.add(entity, aabb);
        }

        let hits = world.spatial_raycast_linear(Vec3::new(-1.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0), 100.0);
        assert_eq!(hits.len(), 10);

        // Verify sorted by distance
        for i in 1..hits.len() {
            assert!(hits[i - 1].distance <= hits[i].distance);
        }
    }

    #[test]
    fn test_spatial_query_aabb_linear() {
        let mut world = World::new();
        world.register::<Aabb>();

        // Create entities in a grid
        for x in 0..5 {
            for z in 0..5 {
                let entity = world.spawn();
                let pos = Vec3::new(x as f32 * 2.0, 0.0, z as f32 * 2.0);
                let aabb = Aabb::from_center_half_extents(pos, Vec3::new(0.5, 0.5, 0.5));
                world.add(entity, aabb);
            }
        }

        let query_aabb = Aabb::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(5.0, 1.0, 5.0));
        let results = world.spatial_query_aabb_linear(&query_aabb);
        assert!(results.len() > 0);
        assert!(results.len() <= 25);
    }
}
