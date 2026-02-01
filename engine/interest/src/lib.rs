//! Engine Interest Management
//!
//! Provides interest management for multiplayer:
//! - Spatial partitioning (grid/octree)
//! - Interest area calculation
//! - Priority-based updates
//! - Bandwidth optimization

#![warn(missing_docs)]

pub mod grid;
pub mod priority;
pub mod manager;

// Re-export commonly used types
pub use manager::InterestManager;
pub use grid::Grid;
