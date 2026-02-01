//! Entity Component System (ECS) core functionality
//!
//! This module provides the foundational ECS implementation including:
//! - Entity management with generational indices
//! - Component storage using sparse sets
//! - World container for managing all ECS data
//! - Type-safe component queries

pub mod component;
pub mod entity;
pub mod query;
pub mod storage;
pub mod world;

// Re-export commonly used types
pub use component::{Component, ComponentDescriptor};
pub use entity::{Entity, EntityAllocator};
pub use query::{Query, QueryIter, QueryIterMut};
pub use storage::SparseSet;
pub use world::World;
