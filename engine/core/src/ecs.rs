//! Entity Component System (ECS)
//!
//! This module will contain the ECS implementation once Phase 1 is complete.
//! For now, it provides basic placeholder types.

/// Placeholder for World
pub struct World;

impl World {
    /// Creates a new World
    pub fn new() -> Self {
        Self
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

/// Placeholder for Entity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Entity(u64);

/// Placeholder for Component trait
pub trait Component: 'static {}
