//! Shared game logic that runs on both client and server.
//!
//! This crate contains:
//! - Components (data): Health, Transform, Velocity, etc.
//! - Systems (logic): movement, combat, regeneration, etc.
//!
//! IMPORTANT: Code here must be deterministic and work the same on client & server.

pub mod components;
pub mod systems;

// Re-export for convenience
pub use components::*;
pub use systems::*;
