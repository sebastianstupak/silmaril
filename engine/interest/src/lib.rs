//! Engine Interest Management
//!
//! Provides interest management for multiplayer bandwidth optimization:
//! - Spatial partitioning (using core::spatial::SpatialGrid)
//! - Area of Interest (AOI) calculation per client
//! - Visibility tracking and change detection
//! - Bandwidth optimization through relevance filtering
//!
//! # Architecture
//!
//! This crate wraps the core spatial grid with client-specific AOI tracking:
//! - SpatialGrid (from engine-core): Entity spatial partitioning
//! - InterestManager: Per-client visibility tracking
//! - Bandwidth reduction: 80-95% typical savings
//!
//! # Performance Targets
//!
//! - Visibility calculation: <1ms for 1K entities per client
//! - 100 clients: <100ms total
//! - Bandwidth reduction: 80-95%
//!
//! # Example
//!
//! ```
//! use engine_interest::{InterestManager, AreaOfInterest};
//! use engine_core::{World, Transform, Vec3, Aabb};
//!
//! let mut world = World::new();
//! world.register::<Transform>();
//! world.register::<Aabb>();
//!
//! // Spawn entities
//! for i in 0..100 {
//!     let entity = world.spawn();
//!     let pos = Vec3::new((i % 10) as f32 * 10.0, 0.0, (i / 10) as f32 * 10.0);
//!     world.add(entity, Transform::from_translation(pos));
//!     world.add(entity, Aabb::from_center_half_extents(pos, Vec3::ONE));
//! }
//!
//! // Create interest manager
//! let mut manager = InterestManager::new(50.0); // 50 unit cell size
//!
//! // Update from world
//! manager.update_from_world(&world);
//!
//! // Set client interest
//! let client_id = 1;
//! let aoi = AreaOfInterest::new(Vec3::ZERO, 100.0);
//! manager.set_client_interest(client_id, aoi);
//!
//! // Calculate visibility
//! let visible = manager.calculate_visibility(client_id);
//! println!("Client sees {} entities", visible.len());
//!
//! // Get changes since last update
//! let (entered, exited) = manager.get_visibility_changes(client_id);
//! println!("Entered: {}, Exited: {}", entered.len(), exited.len());
//! ```

#![warn(missing_docs)]

pub mod adaptive;
pub mod fog_of_war;
pub mod manager;
pub mod telemetry;

pub use adaptive::{AdaptiveInterestManager, PerformanceMonitor, TuningParams};
pub use fog_of_war::{
    EntityType, FogConfig, FogOfWar, FogResult, FogState, StealthState, TeamId, VisionRange,
};
pub use manager::{AreaOfInterest, InterestManager, VisibilityChange};
pub use telemetry::{AlertSeverity, Counter, Gauge, Histogram, InterestMetrics, PerformanceAlert};

// Re-export commonly used types from engine-core
pub use engine_core::spatial::{SpatialGrid, SpatialGridConfig};
pub use engine_core::{Entity, Vec3};
