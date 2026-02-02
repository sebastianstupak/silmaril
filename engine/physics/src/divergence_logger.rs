//! Automatic divergence logging for multiplayer physics debugging (Phase A.3.2)
//!
//! Automatically detects and logs physics state divergences between client and server,
//! or between recorded and replayed simulations.
//!
//! # Features
//!
//! - Configurable divergence thresholds
//! - Automatic logging when threshold exceeded
//! - Frame-by-frame divergence tracking
//! - Integration with tracing for structured logging
//!
//! # Usage
//!
//! ```no_run
//! use engine_physics::{PhysicsWorld, DivergenceLogger, DivergenceThresholds};
//!
//! let mut logger = DivergenceLogger::new(DivergenceThresholds::default());
//!
//! // Each frame, compare client and server
//! let client_world: PhysicsWorld = todo!();
//! let server_world: PhysicsWorld = todo!();
//!
//! logger.check_and_log(&client_world, &server_world, 100);
//! ```

use crate::world::PhysicsWorld;
use engine_math::Vec3;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Divergence detection thresholds
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct DivergenceThresholds {
    /// Position delta threshold (meters)
    pub position: f32,

    /// Velocity delta threshold (m/s)
    pub velocity: f32,

    /// Rotation delta threshold (radians)
    pub rotation: f32,

    /// Angular velocity delta threshold (rad/s)
    pub angular_velocity: f32,
}

impl Default for DivergenceThresholds {
    fn default() -> Self {
        Self {
            position: 0.01,        // 1cm
            velocity: 0.1,         // 10 cm/s
            rotation: 0.01,        // ~0.57 degrees
            angular_velocity: 0.1, // ~5.7 degrees/s
        }
    }
}

impl DivergenceThresholds {
    /// Strict thresholds for deterministic simulations
    pub fn strict() -> Self {
        Self {
            position: 0.001,        // 1mm
            velocity: 0.01,         // 1 cm/s
            rotation: 0.001,        // ~0.057 degrees
            angular_velocity: 0.01, // ~0.57 degrees/s
        }
    }

    /// Relaxed thresholds for networked games with latency
    pub fn relaxed() -> Self {
        Self {
            position: 0.1,         // 10cm
            velocity: 1.0,         // 1 m/s
            rotation: 0.1,         // ~5.7 degrees
            angular_velocity: 1.0, // ~57 degrees/s
        }
    }
}

/// Single entity divergence record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityDivergenceRecord {
    /// Entity ID
    pub entity_id: u64,

    /// Frame number when divergence detected
    pub frame: u64,

    /// Position delta (meters)
    pub position_delta: f32,

    /// Velocity delta (m/s)
    pub velocity_delta: f32,

    /// Reference (server/original) position
    pub reference_position: Vec3,

    /// Actual (client/replay) position
    pub actual_position: Vec3,

    /// Reference velocity
    pub reference_velocity: Vec3,

    /// Actual velocity
    pub actual_velocity: Vec3,
}

/// Divergence logger for automatic detection and logging
pub struct DivergenceLogger {
    /// Detection thresholds
    thresholds: DivergenceThresholds,

    /// Total divergences detected
    total_divergences: usize,

    /// Divergences by entity ID
    divergences_by_entity: HashMap<u64, Vec<EntityDivergenceRecord>>,

    /// Log to tracing (vs silent)
    log_enabled: bool,

    /// Last frame checked
    last_frame: u64,
}

impl DivergenceLogger {
    /// Create a new divergence logger
    pub fn new(thresholds: DivergenceThresholds) -> Self {
        Self {
            thresholds,
            total_divergences: 0,
            divergences_by_entity: HashMap::new(),
            log_enabled: true,
            last_frame: 0,
        }
    }

    /// Enable/disable automatic logging
    pub fn set_logging_enabled(&mut self, enabled: bool) {
        self.log_enabled = enabled;
    }

    /// Check for divergences and log if threshold exceeded
    ///
    /// Compares reference world (server/original) against actual world (client/replay).
    /// Returns number of divergent entities detected this frame.
    pub fn check_and_log(
        &mut self,
        reference: &PhysicsWorld,
        actual: &PhysicsWorld,
        frame: u64,
    ) -> usize {
        self.last_frame = frame;
        let mut divergent_count = 0;

        // Get all entity IDs from reference world
        let reference_ids: Vec<u64> = reference.entity_ids().collect();

        for entity_id in reference_ids {
            // Check if entity exists in both worlds
            let ref_transform = reference.get_transform(entity_id);
            let act_transform = actual.get_transform(entity_id);

            if ref_transform.is_none() || act_transform.is_none() {
                if self.log_enabled {
                    tracing::warn!(
                        frame = frame,
                        entity_id = entity_id,
                        "Entity exists in one world but not the other"
                    );
                }
                continue;
            }

            let (ref_pos, _ref_rot) = ref_transform.unwrap();
            let (act_pos, _act_rot) = act_transform.unwrap();

            // Check position divergence
            let position_delta = (ref_pos - act_pos).length();

            // Check velocity divergence
            let ref_vel = reference.get_velocity(entity_id);
            let act_vel = actual.get_velocity(entity_id);

            let velocity_delta =
                if let (Some((ref_linvel, _)), Some((act_linvel, _))) = (ref_vel, act_vel) {
                    (ref_linvel - act_linvel).length()
                } else {
                    0.0
                };

            // Check if any threshold exceeded
            let diverged = position_delta > self.thresholds.position
                || velocity_delta > self.thresholds.velocity;

            if diverged {
                divergent_count += 1;
                self.total_divergences += 1;

                let record = EntityDivergenceRecord {
                    entity_id,
                    frame,
                    position_delta,
                    velocity_delta,
                    reference_position: ref_pos,
                    actual_position: act_pos,
                    reference_velocity: ref_vel.map(|(v, _)| v).unwrap_or(Vec3::ZERO),
                    actual_velocity: act_vel.map(|(v, _)| v).unwrap_or(Vec3::ZERO),
                };

                // Store divergence
                self.divergences_by_entity
                    .entry(entity_id)
                    .or_default()
                    .push(record.clone());

                // Log if enabled
                if self.log_enabled {
                    tracing::warn!(
                        frame = frame,
                        entity_id = entity_id,
                        position_delta_m = %format!("{:.4}", position_delta),
                        velocity_delta_ms = %format!("{:.4}", velocity_delta),
                        ref_pos = ?ref_pos,
                        act_pos = ?act_pos,
                        "Physics divergence detected"
                    );
                }
            }
        }

        divergent_count
    }

    /// Get total number of divergences detected
    pub fn total_divergences(&self) -> usize {
        self.total_divergences
    }

    /// Get divergences for a specific entity
    pub fn entity_divergences(&self, entity_id: u64) -> Option<&Vec<EntityDivergenceRecord>> {
        self.divergences_by_entity.get(&entity_id)
    }

    /// Get all divergence records
    pub fn all_divergences(&self) -> &HashMap<u64, Vec<EntityDivergenceRecord>> {
        &self.divergences_by_entity
    }

    /// Clear all recorded divergences
    pub fn clear(&mut self) {
        self.total_divergences = 0;
        self.divergences_by_entity.clear();
    }

    /// Export divergences to JSON
    pub fn export_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.divergences_by_entity)
    }

    /// Get statistics summary
    pub fn statistics(&self) -> DivergenceStatistics {
        let entity_count = self.divergences_by_entity.len();
        let max_divergences_per_entity =
            self.divergences_by_entity.values().map(|v| v.len()).max().unwrap_or(0);

        let avg_divergences_per_entity = if entity_count > 0 {
            self.total_divergences as f32 / entity_count as f32
        } else {
            0.0
        };

        DivergenceStatistics {
            total_divergences: self.total_divergences,
            divergent_entity_count: entity_count,
            max_divergences_per_entity,
            avg_divergences_per_entity,
            last_frame: self.last_frame,
        }
    }
}

/// Summary statistics for divergences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DivergenceStatistics {
    /// Total divergences across all entities
    pub total_divergences: usize,

    /// Number of entities that diverged at least once
    pub divergent_entity_count: usize,

    /// Maximum divergences for any single entity
    pub max_divergences_per_entity: usize,

    /// Average divergences per divergent entity
    pub avg_divergences_per_entity: f32,

    /// Last frame checked
    pub last_frame: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Collider, PhysicsConfig, RigidBody};
    use engine_math::Quat;

    #[test]
    fn test_no_divergence() {
        let mut logger = DivergenceLogger::new(DivergenceThresholds::default());

        let mut world1 = PhysicsWorld::new(PhysicsConfig::default());
        let mut world2 = PhysicsWorld::new(PhysicsConfig::default());

        // Add identical entities
        let rb = RigidBody::dynamic(1.0);
        world1.add_rigidbody(1, &rb, Vec3::new(0.0, 10.0, 0.0), Quat::IDENTITY);
        world1.add_collider(1, &Collider::sphere(0.5));

        world2.add_rigidbody(1, &rb, Vec3::new(0.0, 10.0, 0.0), Quat::IDENTITY);
        world2.add_collider(1, &Collider::sphere(0.5));

        // Run identical simulations
        for frame in 0..10 {
            world1.step(1.0 / 60.0);
            world2.step(1.0 / 60.0);

            let divergent = logger.check_and_log(&world1, &world2, frame);
            assert_eq!(divergent, 0, "No divergence expected for identical simulations");
        }

        assert_eq!(logger.total_divergences(), 0);
    }

    #[test]
    fn test_position_divergence_detected() {
        let mut logger = DivergenceLogger::new(DivergenceThresholds::default());

        let mut world1 = PhysicsWorld::new(PhysicsConfig::default());
        let mut world2 = PhysicsWorld::new(PhysicsConfig::default());

        let rb = RigidBody::dynamic(1.0);
        world1.add_rigidbody(1, &rb, Vec3::new(0.0, 10.0, 0.0), Quat::IDENTITY);
        world1.add_collider(1, &Collider::sphere(0.5));

        // World 2 has different position (exceeds threshold)
        world2.add_rigidbody(1, &rb, Vec3::new(0.0, 10.1, 0.0), Quat::IDENTITY);
        world2.add_collider(1, &Collider::sphere(0.5));

        let divergent = logger.check_and_log(&world1, &world2, 0);
        assert_eq!(divergent, 1, "Should detect position divergence");
        assert_eq!(logger.total_divergences(), 1);
    }

    #[test]
    fn test_threshold_strictness() {
        // Strict thresholds
        let mut strict_logger = DivergenceLogger::new(DivergenceThresholds::strict());

        let mut world1 = PhysicsWorld::new(PhysicsConfig::default());
        let mut world2 = PhysicsWorld::new(PhysicsConfig::default());

        let rb = RigidBody::dynamic(1.0);
        world1.add_rigidbody(1, &rb, Vec3::new(0.0, 10.0, 0.0), Quat::IDENTITY);
        world2.add_rigidbody(1, &rb, Vec3::new(0.0, 10.005, 0.0), Quat::IDENTITY); // 5mm diff

        let divergent = strict_logger.check_and_log(&world1, &world2, 0);
        assert_eq!(divergent, 1, "Strict thresholds should detect 5mm difference");

        // Relaxed thresholds
        let mut relaxed_logger = DivergenceLogger::new(DivergenceThresholds::relaxed());
        let divergent = relaxed_logger.check_and_log(&world1, &world2, 0);
        assert_eq!(divergent, 0, "Relaxed thresholds should not detect 5mm difference");
    }

    #[test]
    fn test_statistics() {
        let mut logger = DivergenceLogger::new(DivergenceThresholds::default());
        logger.set_logging_enabled(false); // Disable log spam

        let mut world1 = PhysicsWorld::new(PhysicsConfig::default());
        let mut world2 = PhysicsWorld::new(PhysicsConfig::default());

        // Add 3 entities with varying divergences
        for i in 1..=3 {
            let rb = RigidBody::dynamic(1.0);
            world1.add_rigidbody(i, &rb, Vec3::new(0.0, 10.0, 0.0), Quat::IDENTITY);
            // World 2 has divergent positions
            world2.add_rigidbody(i, &rb, Vec3::new(0.0, 10.1, 0.0), Quat::IDENTITY);
        }

        for frame in 0..10 {
            logger.check_and_log(&world1, &world2, frame);
        }

        let stats = logger.statistics();
        assert_eq!(stats.divergent_entity_count, 3);
        assert_eq!(stats.total_divergences, 30); // 3 entities * 10 frames
        assert_eq!(stats.max_divergences_per_entity, 10);
    }

    #[test]
    fn test_export_json() {
        let mut logger = DivergenceLogger::new(DivergenceThresholds::default());
        logger.set_logging_enabled(false);

        let mut world1 = PhysicsWorld::new(PhysicsConfig::default());
        let mut world2 = PhysicsWorld::new(PhysicsConfig::default());

        let rb = RigidBody::dynamic(1.0);
        world1.add_rigidbody(1, &rb, Vec3::new(0.0, 10.0, 0.0), Quat::IDENTITY);
        world2.add_rigidbody(1, &rb, Vec3::new(0.0, 10.1, 0.0), Quat::IDENTITY);

        logger.check_and_log(&world1, &world2, 0);

        let json = logger.export_json().unwrap();
        assert!(json.contains("entity_id"));
        assert!(json.contains("position_delta"));
    }
}
