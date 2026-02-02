//! Agentic Debugging Infrastructure
//!
//! AI-first debugging tools that export complete physics state in machine-readable formats.
//! Enables AI agents to debug physics issues autonomously by querying exported data.
//!
//! # Architecture
//!
//! - **Snapshot System**: Capture complete physics state per frame
//! - **Event Stream**: Record temporal events (collisions, constraints, solver)
//! - **Exporters**: JSONL (streaming), SQLite (queryable), CSV (simple metrics)
//! - **Query API**: High-level interface for AI agents to analyze data
//! - **Divergence Detection**: Hash-based determinism validation
//!
//! # Usage
//!
//! ```rust,no_run
//! use engine_physics::{PhysicsWorld, PhysicsConfig};
//! use engine_physics::agentic_debug::*;
//!
//! let mut world = PhysicsWorld::new(PhysicsConfig::default());
//!
//! // Enable event recording
//! world.enable_agentic_debug();
//!
//! // Run simulation
//! for frame in 0..1000 {
//!     world.step(1.0 / 60.0);
//!
//!     // Capture snapshot
//!     let snapshot = world.create_debug_snapshot(frame);
//!
//!     // Export to JSONL (streaming)
//!     snapshot.export_jsonl("physics_debug.jsonl")?;
//! }
//!
//! // Later: AI agent queries exported data
//! let db = PhysicsQueryAPI::open("physics_debug.db")?;
//! let high_vel_frames = db.find_high_velocity(entity_id, 100.0)?;
//! ```

#![allow(missing_docs)]

pub mod divergence;
pub mod events;
pub mod exporters;
pub mod query;
pub mod snapshot;

pub use divergence::{DivergenceDetector, DivergenceReport, EntityDivergence};
pub use events::{EventRecorder, EventStatistics, PhysicsEvent, WakeReason};
pub use exporters::{CsvExporter, ExportError, JsonlExporter, SqliteExporter};
pub use query::{PhysicsQueryAPI, QueryError, QueryResult};
pub use snapshot::{
    BroadphasePairState, ColliderState, ConstraintState, ConstraintType, ContactManifoldState,
    ContactPointState, EntityState, IslandState, MaterialState, PhysicsConfigSnapshot,
    PhysicsDebugSnapshot, ShapeParams, ShapeType, AABB,
};
