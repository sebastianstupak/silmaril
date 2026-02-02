//! Physics module for the agent game engine.
//!
//! Integrates Rapier physics engine with ECS and provides SIMD-optimized systems.
//!
//! # Architecture
//!
//! This module is **configuration-driven** - the same code runs on client/server/singleplayer,
//! with behavior controlled by runtime `PhysicsConfig` instead of compile-time features.

#![warn(missing_docs)]

pub mod agentic_debug;
pub mod character_controller;
pub mod components;
pub mod config;
pub mod deterministic;
pub mod events;
pub mod joints;
pub mod prediction;
pub mod sync;
pub mod systems;
pub mod world;

// Re-exports
pub use agentic_debug::{
    CsvExporter, DivergenceDetector, DivergenceReport, EntityDivergence, EntityState,
    EventRecorder, ExportError, JsonlExporter, PhysicsDebugSnapshot, PhysicsEvent, PhysicsQueryAPI,
    QueryError, SqliteExporter,
};
pub use character_controller::CharacterController;
pub use components::{
    Collider, ColliderShape, CombineMode, PhysicsMaterial, RigidBody, RigidBodyType, Velocity,
};
pub use config::{PhysicsConfig, PhysicsMode};
pub use deterministic::{
    create_snapshot, hash_physics_state, restore_snapshot, DeterministicError, PhysicsInput,
    PhysicsSnapshot, RecordedFrame, ReplayPlayer, ReplayRecorder,
};
pub use events::{
    BodySleepEvent, BodyWakeEvent, CollisionEndEvent, CollisionStartEvent, ContactForceEvent,
    TriggerEnterEvent, TriggerExitEvent,
};
pub use joints::{
    FixedJointConfig, Joint, JointBuilder, JointHandle, JointMotor, PrismaticJointConfig,
    RevoluteJointConfig, SphericalJointConfig,
};
pub use prediction::{InputBuffer, PlayerInput, PredictedState, PredictionSystem};
pub use sync::{build_entity_mapping, PhysicsSyncConfig, PhysicsSyncSystem};
pub use systems::{physics_integration_system, physics_integration_system_simd};
pub use world::{PhysicsWorld, RaycastHit};
