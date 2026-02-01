//! Bounding Volume Hierarchy (BVH) for efficient spatial queries.
//!
//! BVH is a tree structure that organizes spatial objects in a hierarchy
//! of bounding volumes. It provides O(log N) performance for:
//! - Ray casts
//! - Frustum culling
//! - Nearest neighbor queries
//!
//! # Algorithm
//!
//! Uses Surface Area Heuristic (SAH) for optimal tree construction.
//! Dynamic entities trigger incremental rebuilds.

use crate::ecs::Entity;
use crate::math::Vec3;
use crate::spatial::Aabb;
use std::cmp::Ordering;

/// Maximum entities per leaf node before splitting.
const MAX_LEAF_SIZE: usize = 4;

/// Minimum entities for SAH split consideration.
const MIN_SAH_PRIMITIVES: usize = 4;

/// BVH node in the hierarchy.
#[derive(Debug, Clone)]
pub enum BvhNode {
    /// Leaf node containing entities.
    Leaf {
        /// Bounding box of all entities in this leaf.
        bounds: Aabb,
        /// Entities in this leaf.
        entities: Vec<(Entity, Aabb)>,
    },
    /// Internal node with two children.
    Internal {
        /// Bounding box of all children.
        bounds: Aabb,
        /// Left child node.
        left: Box<BvhNode>,
        /// Right child node.
        right: Box<BvhNode>,
    },
}

impl BvhNode {
    /// Get the bounding box of this node.
    #[inline]
    pub fn bounds(&self) -> &Aabb {
        match self {
            BvhNode::Leaf { bounds, .. } => bounds,
            BvhNode::Internal { bounds, .. } => bounds,
        }
    }

    /// Check if this is a leaf node.
    #[inline]
    pub fn is_leaf(&self) -> bool {
        matches!(self, BvhNode::Leaf { .. })
    }
}

/// Bounding Volume Hierarchy for efficient spatial queries.
///
/// # Examples
///
/// ```
/// # use engine_core::spatial::{Bvh, Aabb};
/// # use engine_core::ecs::World;
/// # use engine_core::math::Vec3;
/// let mut world = World::new();
/// world.register::<Aabb>();
///
/// // Spawn entities with bounding boxes
/// for i in 0..100 {
///     let entity = world.spawn();
///     let pos = Vec3::new(i as f32, 0.0, 0.0);
///     let aabb = Aabb::from_center_half_extents(pos, Vec3::new(0.5, 0.5, 0.5));
///     world.add(entity, aabb);
/// }
///
/// // Build BVH
/// let bvh = Bvh::build(&world);
///
/// // Ray cast
/// let hits = bvh.ray_cast(Vec3::new(-1.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0), 50.0);
/// assert!(!hits.is_empty());
/// ```
pub struct Bvh {
    /// Root node of the BVH tree.
    root: Option<BvhNode>,
    /// Total number of entities in the BVH.
    entity_count: usize,
}

impl Bvh {
    /// Create a new empty BVH.
    pub fn new() -> Self {
        Self { root: None, entity_count: 0 }
    }

    /// Build a BVH from all entities with Aabb components.
    ///
    /// This performs a full rebuild of the BVH using SAH (Surface Area Heuristic).
    pub fn build(world: &crate::ecs::World) -> Self {
        #[cfg(feature = "profiling")]
        agent_game_engine_profiling::profile_scope!(
            "bvh_build",
            agent_game_engine_profiling::ProfileCategory::Physics
        );

        // Collect all entities with AABB components
        let storage = match world.get_storage::<Aabb>() {
            Some(s) => s,
            None => return Self { root: None, entity_count: 0 },
        };

        let mut primitives: Vec<(Entity, Aabb)> = Vec::new();
        for (entity, aabb) in storage.iter() {
            primitives.push((entity, *aabb));
        }

        let entity_count = primitives.len();
        let root = if !primitives.is_empty() {
            let len = primitives.len();
            Some(Self::build_recursive(&mut primitives, 0, len))
        } else {
            None
        };

        Self { root, entity_count }
    }

    /// Recursively build BVH using SAH.
    fn build_recursive(primitives: &mut [(Entity, Aabb)], start: usize, end: usize) -> BvhNode {
        debug_assert!(start < end, "Invalid range for BVH build");

        let count = end - start;

        // Compute bounds for this node
        let mut bounds = primitives[start].1;
        for i in (start + 1)..end {
            bounds = bounds.merge(&primitives[i].1);
        }

        // Create leaf if small enough
        if count <= MAX_LEAF_SIZE {
            return BvhNode::Leaf { bounds, entities: primitives[start..end].to_vec() };
        }

        // Find best split using SAH
        let (axis, split_pos) = Self::find_best_split(primitives, start, end, &bounds);

        // Partition primitives
        let mid = Self::partition(primitives, start, end, axis, split_pos);

        // Ensure we don't create degenerate splits
        if mid == start || mid == end {
            // Fall back to median split
            let mid = start + count / 2;
            Self::partition_median(primitives, start, end, axis);

            let left = Self::build_recursive(primitives, start, mid);
            let right = Self::build_recursive(primitives, mid, end);

            return BvhNode::Internal { bounds, left: Box::new(left), right: Box::new(right) };
        }

        // Recursively build children
        let left = Self::build_recursive(primitives, start, mid);
        let right = Self::build_recursive(primitives, mid, end);

        BvhNode::Internal { bounds, left: Box::new(left), right: Box::new(right) }
    }

    /// Find the best split using Surface Area Heuristic.
    fn find_best_split(
        primitives: &[(Entity, Aabb)],
        start: usize,
        end: usize,
        bounds: &Aabb,
    ) -> (usize, f32) {
        let count = end - start;

        // For small counts, use simple median split
        if count < MIN_SAH_PRIMITIVES {
            let axis = Self::largest_axis(bounds);
            let split_pos = bounds.center()[axis];
            return (axis, split_pos);
        }

        let mut best_cost = f32::INFINITY;
        let mut best_axis = 0;
        let mut best_pos = 0.0;

        // Try each axis
        for axis in 0..3 {
            // Sample split positions
            let num_samples = (count / 2).min(16);
            for i in 1..num_samples {
                let t = i as f32 / num_samples as f32;
                let split_pos = bounds.min[axis] + t * (bounds.max[axis] - bounds.min[axis]);

                let cost = Self::evaluate_sah(primitives, start, end, axis, split_pos);
                if cost < best_cost {
                    best_cost = cost;
                    best_axis = axis;
                    best_pos = split_pos;
                }
            }
        }

        (best_axis, best_pos)
    }

    /// Evaluate SAH cost for a split.
    fn evaluate_sah(
        primitives: &[(Entity, Aabb)],
        start: usize,
        end: usize,
        axis: usize,
        pos: f32,
    ) -> f32 {
        let mut left_box = None;
        let mut right_box = None;
        let mut left_count = 0;
        let mut right_count = 0;

        for i in start..end {
            let center = primitives[i].1.center();
            if center[axis] < pos {
                left_count += 1;
                left_box = Some(match left_box {
                    Some(b) => primitives[i].1.merge(&b),
                    None => primitives[i].1,
                });
            } else {
                right_count += 1;
                right_box = Some(match right_box {
                    Some(b) => primitives[i].1.merge(&b),
                    None => primitives[i].1,
                });
            }
        }

        // SAH cost = surface_area(left) * count(left) + surface_area(right) * count(right)
        let left_cost = left_box.map_or(0.0, |b| b.surface_area() * left_count as f32);
        let right_cost = right_box.map_or(0.0, |b| b.surface_area() * right_count as f32);

        left_cost + right_cost
    }

    /// Get the largest axis of a bounding box.
    fn largest_axis(bounds: &Aabb) -> usize {
        let size = bounds.size();
        if size.x >= size.y && size.x >= size.z {
            0
        } else if size.y >= size.z {
            1
        } else {
            2
        }
    }

    /// Partition primitives around a split position.
    fn partition(
        primitives: &mut [(Entity, Aabb)],
        start: usize,
        end: usize,
        axis: usize,
        pos: f32,
    ) -> usize {
        let mut i = start;
        let mut j = end - 1;

        while i <= j {
            let center = primitives[i].1.center();
            if center[axis] < pos {
                i += 1;
            } else {
                primitives.swap(i, j);
                if j == 0 {
                    break;
                }
                j -= 1;
            }
        }

        i
    }

    /// Partition primitives using median split.
    fn partition_median(primitives: &mut [(Entity, Aabb)], start: usize, end: usize, axis: usize) {
        primitives[start..end].sort_by(|a, b| {
            let a_center = a.1.center()[axis];
            let b_center = b.1.center()[axis];
            a_center.partial_cmp(&b_center).unwrap_or(Ordering::Equal)
        });
    }

    /// Perform a ray cast against the BVH.
    ///
    /// Returns all entities whose bounding boxes intersect the ray,
    /// sorted by distance from the ray origin.
    pub fn ray_cast(&self, origin: Vec3, direction: Vec3, max_distance: f32) -> Vec<(Entity, f32)> {
        let mut hits = Vec::new();

        if let Some(ref root) = self.root {
            Self::ray_cast_recursive(root, origin, direction, max_distance, &mut hits);
        }

        // Sort by distance
        hits.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));

        hits
    }

    /// Recursive ray cast traversal.
    fn ray_cast_recursive(
        node: &BvhNode,
        origin: Vec3,
        direction: Vec3,
        max_distance: f32,
        hits: &mut Vec<(Entity, f32)>,
    ) {
        // Test ray against node bounds
        if node.bounds().ray_intersection(origin, direction, max_distance).is_none() {
            return;
        }

        match node {
            BvhNode::Leaf { entities, .. } => {
                // Test all entities in leaf
                for (entity, aabb) in entities {
                    if let Some((t_min, _)) = aabb.ray_intersection(origin, direction, max_distance)
                    {
                        hits.push((*entity, t_min));
                    }
                }
            }
            BvhNode::Internal { left, right, .. } => {
                // Recursively test children
                Self::ray_cast_recursive(left, origin, direction, max_distance, hits);
                Self::ray_cast_recursive(right, origin, direction, max_distance, hits);
            }
        }
    }

    /// Find all entities within a radius of a point.
    pub fn query_radius(&self, center: Vec3, radius: f32) -> Vec<Entity> {
        let mut results = Vec::new();
        let radius_sq = radius * radius;

        if let Some(ref root) = self.root {
            Self::query_radius_recursive(root, center, radius_sq, &mut results);
        }

        results
    }

    /// Recursive radius query traversal.
    fn query_radius_recursive(
        node: &BvhNode,
        center: Vec3,
        radius_sq: f32,
        results: &mut Vec<Entity>,
    ) {
        // Test if sphere intersects node bounds
        if node.bounds().distance_squared_to_point(center) > radius_sq {
            return;
        }

        match node {
            BvhNode::Leaf { entities, .. } => {
                for (entity, aabb) in entities {
                    if aabb.distance_squared_to_point(center) <= radius_sq {
                        results.push(*entity);
                    }
                }
            }
            BvhNode::Internal { left, right, .. } => {
                Self::query_radius_recursive(left, center, radius_sq, results);
                Self::query_radius_recursive(right, center, radius_sq, results);
            }
        }
    }

    /// Find all entities within an AABB.
    pub fn query_aabb(&self, aabb: &Aabb) -> Vec<Entity> {
        let mut results = Vec::new();

        if let Some(ref root) = self.root {
            Self::query_aabb_recursive(root, aabb, &mut results);
        }

        results
    }

    /// Recursive AABB query traversal.
    fn query_aabb_recursive(node: &BvhNode, query_aabb: &Aabb, results: &mut Vec<Entity>) {
        if !node.bounds().intersects(query_aabb) {
            return;
        }

        match node {
            BvhNode::Leaf { entities, .. } => {
                for (entity, aabb) in entities {
                    if aabb.intersects(query_aabb) {
                        results.push(*entity);
                    }
                }
            }
            BvhNode::Internal { left, right, .. } => {
                Self::query_aabb_recursive(left, query_aabb, results);
                Self::query_aabb_recursive(right, query_aabb, results);
            }
        }
    }

    /// Get the number of entities in the BVH.
    #[inline]
    pub fn entity_count(&self) -> usize {
        self.entity_count
    }

    /// Check if the BVH is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entity_count == 0
    }
}

impl Default for Bvh {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::World;

    #[test]
    fn test_bvh_build_empty() {
        let world = World::new();
        let bvh = Bvh::build(&world);
        assert!(bvh.is_empty());
    }

    #[test]
    fn test_bvh_build_single_entity() {
        let mut world = World::new();
        world.register::<Aabb>();

        let entity = world.spawn();
        world.add(entity, Aabb::from_center_half_extents(Vec3::ZERO, Vec3::ONE));

        let bvh = Bvh::build(&world);
        assert_eq!(bvh.entity_count(), 1);
    }

    #[test]
    fn test_bvh_ray_cast() {
        let mut world = World::new();
        world.register::<Aabb>();

        // Create a line of entities along X axis
        for i in 0..10 {
            let entity = world.spawn();
            let pos = Vec3::new(i as f32 * 2.0, 0.0, 0.0);
            let aabb = Aabb::from_center_half_extents(pos, Vec3::new(0.5, 0.5, 0.5));
            world.add(entity, aabb);
        }

        let bvh = Bvh::build(&world);

        // Cast ray along X axis
        let origin = Vec3::new(-1.0, 0.0, 0.0);
        let direction = Vec3::new(1.0, 0.0, 0.0);
        let hits = bvh.ray_cast(origin, direction, 100.0);

        assert_eq!(hits.len(), 10);
    }

    #[test]
    fn test_bvh_query_radius() {
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

        let bvh = Bvh::build(&world);

        // Query small radius - should find few entities
        let results = bvh.query_radius(Vec3::ZERO, 1.0);
        assert!(results.len() > 0 && results.len() < 25);

        // Query large radius - should find all entities
        let results = bvh.query_radius(Vec3::new(4.0, 0.0, 4.0), 20.0);
        assert_eq!(results.len(), 25);
    }
}
