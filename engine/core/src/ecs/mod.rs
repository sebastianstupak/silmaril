//! Entity Component System (ECS) core functionality
//!
//! This module provides the foundational ECS implementation including:
//! - Entity management with generational indices
//! - Component storage using sparse sets
//! - World container for managing all ECS data
//! - Type-safe component queries
//! - Change detection for efficient component tracking
//! - Parallel query iteration for multi-core performance

pub mod change_detection;
pub mod component;
pub mod dependency_graph;
pub mod entity;
// TODO: Fix parallel module compilation errors
// pub mod parallel;
pub mod query;
pub mod schedule;
pub mod storage;
pub mod world;

// Re-export commonly used types
pub use change_detection::{Changed, ComponentTicks, SystemTicks, Tick};
pub use component::{Component, ComponentDescriptor};
pub use dependency_graph::{DependencyGraph, SystemNode};
pub use entity::{Entity, EntityAllocator};
// pub use parallel::ParallelWorld;
pub use query::{Query, QueryIter, QueryIterMut};
pub use schedule::{Schedule, System, SystemAccess};
pub use storage::SparseSet;
pub use world::World;
