//! Spatial data structures for efficient spatial queries.
//!
//! This module provides optimized data structures for spatial queries:
//! - BVH (Bounding Volume Hierarchy) for ray casts and frustum culling
//! - Spatial Grid for uniform distributions and nearby queries
//!
//! # Performance
//!
//! - Spatial Grid: O(1) average case for nearby queries
//! - BVH: O(log N) for ray casts and frustum culling
//! - Target: 10-100x speedup vs linear search on 100K entities

pub mod aabb;
pub mod bvh;
pub mod grid;
pub mod query;

pub use aabb::{Aabb, BoundingBox};
pub use bvh::{Bvh, BvhNode};
pub use grid::{SpatialGrid, SpatialGridConfig};
pub use query::{RayCast, RayHit, SpatialQuery};
