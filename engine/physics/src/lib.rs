//! Physics module for the agent game engine.
//!
//! Integrates Rapier physics engine with ECS and provides SIMD-optimized systems.

#![warn(missing_docs)]

pub mod components;
pub mod systems;

// Re-exports
pub use components::Velocity;
pub use systems::{physics_integration_system, physics_integration_system_simd};
