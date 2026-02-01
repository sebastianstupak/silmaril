//! Entity Component System (ECS) core functionality
//!
//! This module provides the foundational ECS implementation including:
//! - Entity management with generational indices
//! - Component storage using sparse sets
//! - World container for managing all ECS data
//! - Type-safe component queries

pub mod entity;
pub mod component;
pub mod storage;
pub mod world;
pub mod query;

// Re-export commonly used types
pub use entity::{Entity, EntityAllocator};
pub use component::{Component, ComponentDescriptor};
pub use storage::SparseSet;
pub use world::World;
pub use query::{Query, QueryIter, QueryIterMut};
