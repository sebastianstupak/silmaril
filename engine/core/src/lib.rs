//! Engine Core
//!
//! Provides the foundational systems for the game engine:
//! - ECS (Entity Component System)
//! - Serialization
//! - Platform abstraction
//! - Math types
//! - Core components

#![warn(missing_docs)]

pub mod ecs;
pub mod error;
pub mod gameplay;
pub mod math;
pub mod physics_components;
pub mod platform;
pub mod rendering;
pub mod serialization;

// Re-export commonly used types
pub use ecs::{Component, ComponentDescriptor, Entity, EntityAllocator, SparseSet, World};
pub use error::{EngineError, ErrorCode, ErrorSeverity};
pub use gameplay::Health;
pub use math::{Quat, Transform, Vec3};
pub use physics_components::Velocity;
pub use rendering::MeshRenderer;
