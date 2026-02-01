//! Physics module for the agent game engine.
//!
//! Integrates Rapier physics engine with ECS and provides SIMD-optimized systems.
//!
//! # Architecture
//!
//! This module is **configuration-driven** - the same code runs on client/server/singleplayer,
//! with behavior controlled by runtime `PhysicsConfig` instead of compile-time features.

#![warn(missing_docs)]

pub mod components;
pub mod config;
pub mod events;
pub mod sync;
pub mod systems;
pub mod world;

// Re-exports
pub use components::{
    Collider, ColliderShape, CombineMode, PhysicsMaterial, RigidBody, RigidBodyType, Velocity,
};
pub use config::{PhysicsConfig, PhysicsMode};
pub use events::{
    BodySleepEvent, BodyWakeEvent, CollisionEndEvent, CollisionStartEvent, ContactForceEvent,
    TriggerEnterEvent, TriggerExitEvent,
};
pub use sync::{build_entity_mapping, PhysicsSyncConfig, PhysicsSyncSystem};
pub use systems::{physics_integration_system, physics_integration_system_simd};
pub use world::PhysicsWorld;
