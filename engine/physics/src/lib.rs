//! Physics module for the Silmaril.
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
pub mod divergence_logger;
pub mod events;
pub mod joints;
pub mod metrics;
pub mod prediction;
pub mod sync;
pub mod systems;
pub mod world;

// Debug rendering (Phase A.1 - Visual Debugging)
#[cfg(feature = "debug-render")]
pub mod debug_render;

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
    PhysicsSnapshot, RecordedFrame, ReplayFile, ReplayMetadata, ReplayPlayer, ReplayRecorder,
};
pub use divergence_logger::{
    DivergenceLogger, DivergenceStatistics, DivergenceThresholds, EntityDivergenceRecord,
};
pub use events::{
    BodySleepEvent, BodyWakeEvent, CollisionEndEvent, CollisionStartEvent, ContactForceEvent,
    TriggerEnterEvent, TriggerExitEvent,
};
pub use joints::{
    FixedJointConfig, Joint, JointBuilder, JointHandle, JointMotor, PrismaticJointConfig,
    RevoluteJointConfig, SphericalJointConfig,
};
pub use metrics::{FrameMetrics, FrameStats, MetricsCollector};
pub use prediction::{InputBuffer, PlayerInput, PredictedState, PredictionSystem};
pub use sync::{build_entity_mapping, PhysicsSyncConfig, PhysicsSyncSystem};
pub use systems::{physics_integration_system, physics_integration_system_simd};
pub use world::{PhysicsWorld, RaycastHit};
