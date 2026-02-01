//! Axis-Aligned Bounding Box (AABB) for spatial queries.

use crate::ecs::Component;
use crate::math::Vec3;
use serde::{Deserialize, Serialize};

/// Axis-Aligned Bounding Box component.
///
/// Represents the spatial extent of an entity in world space.
/// Used for broad-phase collision detection, frustum culling, and spatial queries.
///
/// # Examples
///
/// ```
/// # use engine_core::spatial::Aabb;
/// # use engine_core::math::Vec3;
/// let aabb = Aabb::new(
///     Vec3::new(-1.0, -1.0, -1.0),
///     Vec3::new(1.0, 1.0, 1.0)
/// );
/// assert!(aabb.contains_point(Vec3::ZERO));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Aabb {
    /// Minimum corner of the bounding box
    pub min: Vec3,
    /// Maximum corner of the bounding box
    pub max: Vec3,
}

impl Aabb {
    /// Create a new AABB from min and max corners.
    ///
    /// # Panics
    ///
    /// Panics in debug builds if min > max on any axis.
    #[inline]
    pub fn new(min: Vec3, max: Vec3) -> Self {
        debug_assert!(
            min.x <= max.x && min.y <= max.y && min.z <= max.z,
            "AABB min must be <= max on all axes"
        );
        Self { min, max }
    }

    /// Create an AABB from a center point and half-extents.
    ///
    /// # Examples
    ///
    /// ```
    /// # use engine_core::spatial::Aabb;
    /// # use engine_core::math::Vec3;
    /// let aabb = Aabb::from_center_half_extents(
    ///     Vec3::ZERO,
    ///     Vec3::new(1.0, 1.0, 1.0)
    /// );
    /// assert_eq!(aabb.min, Vec3::new(-1.0, -1.0, -1.0));
    /// assert_eq!(aabb.max, Vec3::new(1.0, 1.0, 1.0));
    /// ```
    #[inline]
    pub fn from_center_half_extents(center: Vec3, half_extents: Vec3) -> Self {
        Self {
            min: center - half_extents,
            max: center + half_extents,
        }
    }

    /// Create an AABB that encompasses a single point.
    #[inline]
    pub fn from_point(point: Vec3) -> Self {
        Self { min: point, max: point }
    }

    /// Get the center of the AABB.
    #[inline]
    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    /// Get the half-extents (half the size on each axis).
    #[inline]
    pub fn half_extents(&self) -> Vec3 {
        (self.max - self.min) * 0.5
    }

    /// Get the full size on each axis.
    #[inline]
    pub fn size(&self) -> Vec3 {
        self.max - self.min
    }

    /// Get the surface area of the AABB (used for BVH SAH heuristic).
    #[inline]
    pub fn surface_area(&self) -> f32 {
        let size = self.size();
        2.0 * (size.x * size.y + size.y * size.z + size.z * size.x)
    }

    /// Get the volume of the AABB.
    #[inline]
    pub fn volume(&self) -> f32 {
        let size = self.size();
        size.x * size.y * size.z
    }

    /// Check if this AABB contains a point.
    #[inline]
    pub fn contains_point(&self, point: Vec3) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
            && point.z >= self.min.z
            && point.z <= self.max.z
    }

    /// Check if this AABB intersects another AABB.
    #[inline]
    pub fn intersects(&self, other: &Aabb) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
            && self.min.z <= other.max.z
            && self.max.z >= other.min.z
    }

    /// Check if this AABB fully contains another AABB.
    #[inline]
    pub fn contains(&self, other: &Aabb) -> bool {
        self.min.x <= other.min.x
            && self.max.x >= other.max.x
            && self.min.y <= other.min.y
            && self.max.y >= other.max.y
            && self.min.z <= other.min.z
            && self.max.z >= other.max.z
    }

    /// Merge this AABB with another, returning a new AABB that contains both.
    #[inline]
    pub fn merge(&self, other: &Aabb) -> Aabb {
        Aabb {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    /// Expand this AABB to include a point.
    #[inline]
    pub fn expand_to_include(&mut self, point: Vec3) {
        self.min = self.min.min(point);
        self.max = self.max.max(point);
    }

    /// Expand this AABB by a margin on all sides.
    #[inline]
    pub fn expand(&self, margin: f32) -> Aabb {
        let margin_vec = Vec3::new(margin, margin, margin);
        Aabb {
            min: self.min - margin_vec,
            max: self.max + margin_vec,
        }
    }

    /// Test ray intersection with this AABB.
    ///
    /// Returns Some((t_min, t_max)) if the ray intersects,
    /// where t_min and t_max are the ray parameters at entry and exit.
    ///
    /// # Examples
    ///
    /// ```
    /// # use engine_core::spatial::Aabb;
    /// # use engine_core::math::Vec3;
    /// let aabb = Aabb::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
    /// let origin = Vec3::new(-2.0, 0.0, 0.0);
    /// let direction = Vec3::new(1.0, 0.0, 0.0);
    ///
    /// let hit = aabb.ray_intersection(origin, direction, 10.0);
    /// assert!(hit.is_some());
    /// ```
    pub fn ray_intersection(&self, origin: Vec3, direction: Vec3, max_distance: f32) -> Option<(f32, f32)> {
        // Optimized slab method
        let inv_dir = Vec3::new(
            if direction.x != 0.0 { 1.0 / direction.x } else { f32::INFINITY },
            if direction.y != 0.0 { 1.0 / direction.y } else { f32::INFINITY },
            if direction.z != 0.0 { 1.0 / direction.z } else { f32::INFINITY },
        );

        let t1 = (self.min - origin) * inv_dir;
        let t2 = (self.max - origin) * inv_dir;

        let t_min = t1.min(t2);
        let t_max = t1.max(t2);

        let t_enter = t_min.x.max(t_min.y).max(t_min.z);
        let t_exit = t_max.x.min(t_max.y).min(t_max.z);

        if t_enter <= t_exit && t_exit >= 0.0 && t_enter <= max_distance {
            Some((t_enter.max(0.0), t_exit))
        } else {
            None
        }
    }

    /// Get the closest point on or inside the AABB to a given point.
    #[inline]
    pub fn closest_point(&self, point: Vec3) -> Vec3 {
        Vec3::new(
            point.x.clamp(self.min.x, self.max.x),
            point.y.clamp(self.min.y, self.max.y),
            point.z.clamp(self.min.z, self.max.z),
        )
    }

    /// Get the squared distance from a point to the nearest point on the AABB.
    #[inline]
    pub fn distance_squared_to_point(&self, point: Vec3) -> f32 {
        let closest = self.closest_point(point);
        (point - closest).length_squared()
    }
}

/// Type alias for the bounding box component.
///
/// This is the component that should be added to entities for spatial queries.
pub type BoundingBox = Aabb;

impl Component for Aabb {}

impl Default for Aabb {
    fn default() -> Self {
        Self {
            min: Vec3::ZERO,
            max: Vec3::ZERO,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aabb_creation() {
        let aabb = Aabb::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
        assert_eq!(aabb.min, Vec3::new(-1.0, -1.0, -1.0));
        assert_eq!(aabb.max, Vec3::new(1.0, 1.0, 1.0));
    }

    #[test]
    fn test_aabb_center() {
        let aabb = Aabb::new(Vec3::new(-2.0, -2.0, -2.0), Vec3::new(2.0, 2.0, 2.0));
        assert_eq!(aabb.center(), Vec3::ZERO);
    }

    #[test]
    fn test_aabb_contains_point() {
        let aabb = Aabb::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
        assert!(aabb.contains_point(Vec3::ZERO));
        assert!(aabb.contains_point(Vec3::new(0.5, 0.5, 0.5)));
        assert!(!aabb.contains_point(Vec3::new(2.0, 0.0, 0.0)));
    }

    #[test]
    fn test_aabb_intersects() {
        let aabb1 = Aabb::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
        let aabb2 = Aabb::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(2.0, 2.0, 2.0));
        let aabb3 = Aabb::new(Vec3::new(5.0, 5.0, 5.0), Vec3::new(6.0, 6.0, 6.0));

        assert!(aabb1.intersects(&aabb2));
        assert!(!aabb1.intersects(&aabb3));
    }

    #[test]
    fn test_aabb_merge() {
        let aabb1 = Aabb::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
        let aabb2 = Aabb::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(2.0, 2.0, 2.0));
        let merged = aabb1.merge(&aabb2);

        assert_eq!(merged.min, Vec3::new(-1.0, -1.0, -1.0));
        assert_eq!(merged.max, Vec3::new(2.0, 2.0, 2.0));
    }

    #[test]
    fn test_aabb_ray_intersection() {
        let aabb = Aabb::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
        let origin = Vec3::new(-2.0, 0.0, 0.0);
        let direction = Vec3::new(1.0, 0.0, 0.0);

        let hit = aabb.ray_intersection(origin, direction, 10.0);
        assert!(hit.is_some());
        let (t_min, t_max) = hit.unwrap();
        assert!(t_min < t_max);
        assert!(t_min >= 0.0);
    }

    #[test]
    fn test_aabb_surface_area() {
        let aabb = Aabb::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(2.0, 2.0, 2.0));
        // Each face is 2x2 = 4, 6 faces = 24
        assert_eq!(aabb.surface_area(), 24.0);
    }

    #[test]
    fn test_aabb_from_center_half_extents() {
        let aabb = Aabb::from_center_half_extents(Vec3::ZERO, Vec3::new(1.0, 1.0, 1.0));
        assert_eq!(aabb.min, Vec3::new(-1.0, -1.0, -1.0));
        assert_eq!(aabb.max, Vec3::new(1.0, 1.0, 1.0));
    }
}
