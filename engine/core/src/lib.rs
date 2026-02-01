//! Engine Core
//!
//! Provides the foundational systems for the game engine:
//! - ECS (Entity Component System)
//! - Serialization
//! - Platform abstraction

#![warn(missing_docs)]

pub mod ecs;
pub mod serialization;
pub mod platform;

// Re-export commonly used types
pub use ecs::{World, Entity, Component};
