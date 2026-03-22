//! Engine Core
//!
//! Provides the foundational systems for the game engine:
//! - ECS (Entity Component System)
//! - Serialization
//! - Platform abstraction
//! - Math types
//! - Core components

#![warn(missing_docs)]

// Allow `engine_core::` paths to resolve inside this crate (needed by derive macros)
extern crate self as engine_core;

pub mod allocators;
pub mod ecs;
pub mod error;
pub mod gameplay;
pub mod math;
pub mod physics_components;
pub mod platform;
pub mod rendering;
pub mod serialization;
pub mod spatial;

// Re-export commonly used types
// Note: The `Component` derive macro is re-exported from `ecs` module
pub use allocators::{Arena, FrameAllocator, PoolAllocator};
pub use ecs::{Component, ComponentDescriptor, Entity, EntityAllocator, SparseSet, World};
pub use error::{EngineError, ErrorCode, ErrorSeverity};
pub use gameplay::{Health, Parent};
pub use math::{Quat, Transform, Vec3};
pub use physics_components::Velocity;
pub use platform::PlatformError;
pub use rendering::{Camera, MeshRenderer};
pub use spatial::{
    Aabb, BoundingBox, Bvh, RayCast, RayHit, SpatialGrid, SpatialGridConfig, SpatialQuery,
};
