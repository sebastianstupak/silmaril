//! Divergence detection for determinism validation
//!
//! Compares physics states between:
//! - Client and server (multiplayer)
//! - Recorded and replayed simulations
//! - Reference and current builds (regression testing)
//!
//! Uses hash-based validation and per-entity delta analysis.

use crate::agentic_debug::PhysicsDebugSnapshot;
use engine_math::Vec3;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Divergence detector for comparing physics states
pub struct DivergenceDetector {
    /// Position delta threshold (meters)
    position_threshold: f32,

    /// Velocity delta threshold (m/s)
    velocity_threshold: f32,

    /// Total divergences detected
    total_divergences: usize,
}

impl DivergenceDetector {
    /// Create a new divergence detector with default thresholds
    pub fn new() -> Self {
        Self {
            position_threshold: 0.01, // 1cm
            velocity_threshold: 0.1,  // 10 cm/s
            total_divergences: 0,
        }
    }

    /// Create with custom thresholds
    pub fn with_thresholds(position: f32, velocity: f32) -> Self {
        Self { position_threshold: position, velocity_threshold: velocity, total_divergences: 0 }
    }

    /// Check for divergence between two snapshots
    ///
    /// Compares entity states and returns detailed divergence report if found.
    pub fn check_divergence(
        &mut self,
        reference: &PhysicsDebugSnapshot,
        actual: &PhysicsDebugSnapshot,
    ) -> Option<DivergenceReport> {
        // Quick check: frame numbers should match
        if reference.frame != actual.frame {
            tracing::warn!(
                reference_frame = reference.frame,
                actual_frame = actual.frame,
                "Frame number mismatch"
            );
        }

        // Quick check: entity counts should match
        if reference.entities.len() != actual.entities.len() {
            tracing::warn!(
                reference_count = reference.entities.len(),
                actual_count = actual.entities.len(),
                "Entity count mismatch"
            );
        }

        let mut diverged_entities = Vec::new();

        // Compare each entity
        for ref_entity in &reference.entities {
            if let Some(act_entity) = actual.get_entity(ref_entity.id) {
                let position_delta = (ref_entity.position - act_entity.position).length();
                let velocity_delta =
                    (ref_entity.linear_velocity - act_entity.linear_velocity).length();

                if position_delta > self.position_threshold
                    || velocity_delta > self.velocity_threshold
                {
                    diverged_entities.push(EntityDivergence {
                        entity_id: ref_entity.id,
                        reference_position: ref_entity.position,
                        actual_position: act_entity.position,
                        position_delta,
                        reference_velocity: ref_entity.linear_velocity,
                        actual_velocity: act_entity.linear_velocity,
                        velocity_delta,
                        reference_sleeping: ref_entity.sleeping,
                        actual_sleeping: act_entity.sleeping,
                    });
                }
            } else {
                // Entity missing in actual snapshot
                diverged_entities.push(EntityDivergence {
                    entity_id: ref_entity.id,
                    reference_position: ref_entity.position,
                    actual_position: Vec3::ZERO,
                    position_delta: f32::MAX,
                    reference_velocity: ref_entity.linear_velocity,
                    actual_velocity: Vec3::ZERO,
                    velocity_delta: f32::MAX,
                    reference_sleeping: ref_entity.sleeping,
                    actual_sleeping: false,
                });
            }
        }

        if !diverged_entities.is_empty() {
            self.total_divergences += 1;

            tracing::warn!(
                frame = reference.frame,
                diverged_count = diverged_entities.len(),
                "Physics divergence detected"
            );

            Some(DivergenceReport {
                frame: reference.frame,
                reference_hash: compute_snapshot_hash(reference),
                actual_hash: compute_snapshot_hash(actual),
                diverged_entities,
                position_threshold: self.position_threshold,
                velocity_threshold: self.velocity_threshold,
            })
        } else {
            None
        }
    }

    /// Get total divergences detected
    pub fn total_divergences(&self) -> usize {
        self.total_divergences
    }

    /// Reset counter
    pub fn reset(&mut self) {
        self.total_divergences = 0;
    }
}

impl Default for DivergenceDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Divergence report showing differences between reference and actual state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DivergenceReport {
    /// Frame where divergence occurred
    pub frame: u64,

    /// Reference state hash (SHA256)
    pub reference_hash: String,

    /// Actual state hash (SHA256)
    pub actual_hash: String,

    /// Entities that diverged
    pub diverged_entities: Vec<EntityDivergence>,

    /// Position threshold used (meters)
    pub position_threshold: f32,

    /// Velocity threshold used (m/s)
    pub velocity_threshold: f32,
}

impl DivergenceReport {
    /// Get most diverged entity (largest position delta)
    pub fn most_diverged(&self) -> Option<&EntityDivergence> {
        self.diverged_entities
            .iter()
            .max_by(|a, b| a.position_delta.partial_cmp(&b.position_delta).unwrap())
    }

    /// Get average position delta
    pub fn average_position_delta(&self) -> f32 {
        if self.diverged_entities.is_empty() {
            return 0.0;
        }

        let sum: f32 = self.diverged_entities.iter().map(|e| e.position_delta).sum();
        sum / self.diverged_entities.len() as f32
    }

    /// Get maximum position delta
    pub fn max_position_delta(&self) -> f32 {
        self.diverged_entities.iter().map(|e| e.position_delta).fold(0.0, f32::max)
    }

    /// Count entities with critical divergence (position > 1.0m)
    pub fn critical_count(&self) -> usize {
        self.diverged_entities.iter().filter(|e| e.position_delta > 1.0).count()
    }
}

/// Single entity divergence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityDivergence {
    /// Entity ID
    pub entity_id: u64,

    /// Reference (expected) position
    pub reference_position: Vec3,

    /// Actual position
    pub actual_position: Vec3,

    /// Position delta magnitude
    pub position_delta: f32,

    /// Reference velocity
    pub reference_velocity: Vec3,

    /// Actual velocity
    pub actual_velocity: Vec3,

    /// Velocity delta magnitude
    pub velocity_delta: f32,

    /// Was entity sleeping in reference?
    pub reference_sleeping: bool,

    /// Is entity sleeping in actual?
    pub actual_sleeping: bool,
}

impl EntityDivergence {
    /// Is this a critical divergence? (position > 1.0m)
    pub fn is_critical(&self) -> bool {
        self.position_delta > 1.0
    }

    /// Did sleep state change?
    pub fn sleep_state_changed(&self) -> bool {
        self.reference_sleeping != self.actual_sleeping
    }
}

/// Compute deterministic hash of physics snapshot
///
/// Uses SHA256 for cryptographic-quality hashing suitable for
/// detecting even tiny differences between states.
pub fn compute_snapshot_hash(snapshot: &PhysicsDebugSnapshot) -> String {
    let mut hasher = Sha256::new();

    // Hash frame
    hasher.update(snapshot.frame.to_le_bytes());

    // Hash entity count
    hasher.update((snapshot.entities.len() as u64).to_le_bytes());

    // Hash each entity's state (sorted by ID for determinism)
    let mut sorted_entities = snapshot.entities.clone();
    sorted_entities.sort_by_key(|e| e.id);

    for entity in sorted_entities {
        hasher.update(entity.id.to_le_bytes());

        // Position (fixed-point to avoid float precision issues)
        hash_vec3_fixed(&mut hasher, entity.position);

        // Rotation
        hash_quat_fixed(&mut hasher, entity.rotation);

        // Velocity
        hash_vec3_fixed(&mut hasher, entity.linear_velocity);
        hash_vec3_fixed(&mut hasher, entity.angular_velocity);

        // Boolean flags
        hasher.update([entity.sleeping as u8]);
        hasher.update([entity.is_static as u8]);
    }

    format!("{:x}", hasher.finalize())
}

// Helper: Hash Vec3 as fixed-point (4 decimal places)
fn hash_vec3_fixed(hasher: &mut Sha256, v: Vec3) {
    let x = (v.x * 10000.0).round() as i64;
    let y = (v.y * 10000.0).round() as i64;
    let z = (v.z * 10000.0).round() as i64;

    hasher.update(x.to_le_bytes());
    hasher.update(y.to_le_bytes());
    hasher.update(z.to_le_bytes());
}

// Helper: Hash Quat as fixed-point
fn hash_quat_fixed(hasher: &mut Sha256, q: engine_math::Quat) {
    let x = (q.x * 10000.0).round() as i64;
    let y = (q.y * 10000.0).round() as i64;
    let z = (q.z * 10000.0).round() as i64;
    let w = (q.w * 10000.0).round() as i64;

    hasher.update(x.to_le_bytes());
    hasher.update(y.to_le_bytes());
    hasher.update(z.to_le_bytes());
    hasher.update(w.to_le_bytes());
}

/// Compare two snapshot hashes
pub fn hashes_match(hash1: &str, hash2: &str) -> bool {
    hash1 == hash2
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agentic_debug::{EntityState, PhysicsDebugSnapshot};
    use engine_math::Quat;

    fn create_test_snapshot(frame: u64, position: Vec3) -> PhysicsDebugSnapshot {
        let mut snapshot = PhysicsDebugSnapshot::new(frame, frame as f64 * 0.016);

        snapshot.entities.push(EntityState {
            id: 1,
            position,
            rotation: Quat::IDENTITY,
            linear_velocity: Vec3::ZERO,
            angular_velocity: Vec3::ZERO,
            forces: Vec3::ZERO,
            torques: Vec3::ZERO,
            mass: 1.0,
            linear_damping: 0.0,
            angular_damping: 0.0,
            gravity_scale: 1.0,
            sleeping: false,
            is_static: false,
            is_kinematic: false,
            can_sleep: true,
            ccd_enabled: false,
        });

        snapshot
    }

    #[test]
    fn test_no_divergence() {
        let mut detector = DivergenceDetector::new();

        let reference = create_test_snapshot(1, Vec3::new(1.0, 2.0, 3.0));
        let actual = create_test_snapshot(1, Vec3::new(1.0, 2.0, 3.0));

        let result = detector.check_divergence(&reference, &actual);
        assert!(result.is_none());
        assert_eq!(detector.total_divergences(), 0);
    }

    #[test]
    fn test_small_divergence() {
        let mut detector = DivergenceDetector::new();

        let reference = create_test_snapshot(1, Vec3::new(1.0, 2.0, 3.0));
        let actual = create_test_snapshot(1, Vec3::new(1.005, 2.0, 3.0)); // 5mm delta

        // Should not detect (below 1cm threshold)
        let result = detector.check_divergence(&reference, &actual);
        assert!(result.is_none());
    }

    #[test]
    fn test_large_divergence() {
        let mut detector = DivergenceDetector::new();

        let reference = create_test_snapshot(1, Vec3::new(1.0, 2.0, 3.0));
        let actual = create_test_snapshot(1, Vec3::new(1.5, 2.0, 3.0)); // 50cm delta

        let result = detector.check_divergence(&reference, &actual);
        assert!(result.is_some());

        let report = result.unwrap();
        assert_eq!(report.diverged_entities.len(), 1);
        assert_eq!(report.diverged_entities[0].entity_id, 1);
        assert!((report.diverged_entities[0].position_delta - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_critical_divergence() {
        let mut detector = DivergenceDetector::new();

        let reference = create_test_snapshot(1, Vec3::new(0.0, 0.0, 0.0));
        let actual = create_test_snapshot(1, Vec3::new(5.0, 0.0, 0.0)); // 5m delta (critical)

        let result = detector.check_divergence(&reference, &actual);
        assert!(result.is_some());

        let report = result.unwrap();
        assert_eq!(report.critical_count(), 1);
        assert_eq!(report.max_position_delta(), 5.0);

        let most_diverged = report.most_diverged().unwrap();
        assert!(most_diverged.is_critical());
    }

    #[test]
    fn test_custom_thresholds() {
        // Very strict thresholds
        let mut detector = DivergenceDetector::with_thresholds(0.001, 0.01);

        let reference = create_test_snapshot(1, Vec3::new(1.0, 2.0, 3.0));
        let actual = create_test_snapshot(1, Vec3::new(1.005, 2.0, 3.0)); // 5mm

        // Should detect with strict threshold
        let result = detector.check_divergence(&reference, &actual);
        assert!(result.is_some());
    }

    #[test]
    fn test_hash_computation() {
        let snapshot1 = create_test_snapshot(1, Vec3::new(1.0, 2.0, 3.0));
        let snapshot2 = create_test_snapshot(1, Vec3::new(1.0, 2.0, 3.0));

        let hash1 = compute_snapshot_hash(&snapshot1);
        let hash2 = compute_snapshot_hash(&snapshot2);

        // Same state should produce same hash
        assert_eq!(hash1, hash2);

        // Different state should produce different hash
        let snapshot3 = create_test_snapshot(1, Vec3::new(1.001, 2.0, 3.0));
        let hash3 = compute_snapshot_hash(&snapshot3);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_hash_format() {
        let snapshot = create_test_snapshot(1, Vec3::new(1.0, 2.0, 3.0));
        let hash = compute_snapshot_hash(&snapshot);

        // SHA256 produces 64 hex characters
        assert_eq!(hash.len(), 64);

        // Should be valid hex
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_multiple_divergences() {
        let mut detector = DivergenceDetector::new();

        for i in 0..5 {
            let reference = create_test_snapshot(i, Vec3::new(1.0, 2.0, 3.0));
            let actual = create_test_snapshot(i, Vec3::new(1.5, 2.0, 3.0));

            let result = detector.check_divergence(&reference, &actual);
            assert!(result.is_some());
        }

        assert_eq!(detector.total_divergences(), 5);
    }
}
