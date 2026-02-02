//! Deterministic physics mode for replay and multiplayer
//!
//! Provides state hashing, replay recording, and deterministic simulation guarantees.

use crate::world::PhysicsWorld;
use engine_core::{EngineError, ErrorCode, ErrorSeverity};
use engine_macros::define_error;
use engine_math::{Quat, Vec3};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

#[cfg(feature = "profiling")]
use silmaril_profiling::{profile_scope, ProfileCategory};

define_error! {
    pub enum DeterministicError {
        ReplayFailed { reason: String } = ErrorCode::PhysicsInitFailed, ErrorSeverity::Error,
        HashMismatch { expected: u64, actual: u64 } = ErrorCode::PhysicsInitFailed, ErrorSeverity::Error,
        InvalidSnapshot { reason: String } = ErrorCode::PhysicsInitFailed, ErrorSeverity::Error,
    }
}

/// Input action for a single frame
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PhysicsInput {
    /// Apply force to entity
    ApplyForce {
        /// Entity ID to apply force to
        entity_id: u64,
        /// Force vector to apply
        force: Vec3,
    },
    /// Apply impulse to entity
    ApplyImpulse {
        /// Entity ID to apply impulse to
        entity_id: u64,
        /// Impulse vector to apply
        impulse: Vec3,
    },
    /// Set velocity of entity
    SetVelocity {
        /// Entity ID to set velocity for
        entity_id: u64,
        /// Linear velocity
        linear: Vec3,
        /// Angular velocity
        angular: Vec3,
    },
    /// Set transform of entity
    SetTransform {
        /// Entity ID to set transform for
        entity_id: u64,
        /// Position vector
        position: Vec3,
        /// Rotation quaternion
        rotation: Quat,
    },
}

/// Recorded frame with inputs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedFrame {
    /// Frame number
    pub frame: u64,
    /// Inputs applied this frame
    pub inputs: Vec<PhysicsInput>,
    /// State hash after applying inputs (for verification)
    pub state_hash: u64,
}

/// Physics state snapshot for replay
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicsSnapshot {
    /// Frame number when snapshot was taken
    pub frame: u64,
    /// Entity transforms (position + rotation)
    pub transforms: HashMap<u64, (Vec3, Quat)>,
    /// Entity velocities (linear + angular)
    pub velocities: HashMap<u64, (Vec3, Vec3)>,
    /// State hash for verification
    pub state_hash: u64,
}

/// Replay recorder for deterministic physics
///
/// Records inputs and state hashes for each frame, allowing perfect replay.
pub struct ReplayRecorder {
    /// Recorded frames
    frames: Vec<RecordedFrame>,
    /// Current frame number
    current_frame: u64,
    /// Inputs for current frame (not yet committed)
    pending_inputs: Vec<PhysicsInput>,
    /// Initial snapshot (frame 0)
    initial_snapshot: Option<PhysicsSnapshot>,
}

impl ReplayRecorder {
    /// Create a new replay recorder
    pub fn new() -> Self {
        Self {
            frames: Vec::new(),
            current_frame: 0,
            pending_inputs: Vec::new(),
            initial_snapshot: None,
        }
    }

    /// Record the initial snapshot
    pub fn record_initial_snapshot(&mut self, world: &PhysicsWorld) {
        self.initial_snapshot = Some(create_snapshot(world, 0));

        tracing::info!(
            frame = 0,
            state_hash = self.initial_snapshot.as_ref().unwrap().state_hash,
            "Recorded initial physics snapshot"
        );
    }

    /// Record an input for the current frame
    pub fn record_input(&mut self, input: PhysicsInput) {
        self.pending_inputs.push(input);
    }

    /// Commit the current frame (call after physics step)
    pub fn commit_frame(&mut self, world: &PhysicsWorld) {
        #[cfg(feature = "profiling")]
        profile_scope!("commit_replay_frame", ProfileCategory::Physics);

        let state_hash = hash_physics_state(world);

        let frame = RecordedFrame {
            frame: self.current_frame,
            inputs: std::mem::take(&mut self.pending_inputs),
            state_hash,
        };

        tracing::trace!(
            frame = self.current_frame,
            inputs = frame.inputs.len(),
            state_hash = state_hash,
            "Committed replay frame"
        );

        self.frames.push(frame);
        self.current_frame += 1;
    }

    /// Get all recorded frames
    pub fn frames(&self) -> &[RecordedFrame] {
        &self.frames
    }

    /// Get initial snapshot
    pub fn initial_snapshot(&self) -> Option<&PhysicsSnapshot> {
        self.initial_snapshot.as_ref()
    }

    /// Get current frame number
    pub fn current_frame(&self) -> u64 {
        self.current_frame
    }

    /// Clear all recorded data
    pub fn clear(&mut self) {
        self.frames.clear();
        self.pending_inputs.clear();
        self.current_frame = 0;
        self.initial_snapshot = None;
    }

    /// Get memory usage in bytes (approximate)
    pub fn memory_usage(&self) -> usize {
        let frames_size = self.frames.len() * std::mem::size_of::<RecordedFrame>();
        let inputs_size = self.pending_inputs.len() * std::mem::size_of::<PhysicsInput>();
        let snapshot_size = std::mem::size_of::<PhysicsSnapshot>();
        frames_size + inputs_size + snapshot_size
    }
}

impl Default for ReplayRecorder {
    fn default() -> Self {
        Self::new()
    }
}

/// Replay player for deterministic physics
///
/// Replays recorded inputs from a snapshot, verifying state hashes.
pub struct ReplayPlayer {
    /// Frames to replay
    frames: Vec<RecordedFrame>,
    /// Current frame index
    current_index: usize,
    /// Initial snapshot
    initial_snapshot: PhysicsSnapshot,
    /// Whether to verify state hashes
    verify_hashes: bool,
}

impl ReplayPlayer {
    /// Create a new replay player
    pub fn new(
        initial_snapshot: PhysicsSnapshot,
        frames: Vec<RecordedFrame>,
        verify_hashes: bool,
    ) -> Self {
        Self { frames, current_index: 0, initial_snapshot, verify_hashes }
    }

    /// Get the initial snapshot
    pub fn initial_snapshot(&self) -> &PhysicsSnapshot {
        &self.initial_snapshot
    }

    /// Get inputs for the next frame
    ///
    /// Returns None if replay is complete.
    pub fn next_frame(&mut self) -> Option<&[PhysicsInput]> {
        if self.current_index >= self.frames.len() {
            return None;
        }

        let frame = &self.frames[self.current_index];
        self.current_index += 1;

        Some(&frame.inputs)
    }

    /// Verify state hash for current frame
    pub fn verify_hash(&self, world: &PhysicsWorld) -> Result<(), DeterministicError> {
        if !self.verify_hashes || self.current_index == 0 {
            return Ok(());
        }

        let expected_hash = self.frames[self.current_index - 1].state_hash;
        let actual_hash = hash_physics_state(world);

        if expected_hash != actual_hash {
            tracing::error!(
                frame = self.current_index - 1,
                expected = expected_hash,
                actual = actual_hash,
                "State hash mismatch during replay"
            );

            return Err(DeterministicError::HashMismatch {
                expected: expected_hash,
                actual: actual_hash,
            });
        }

        Ok(())
    }

    /// Check if replay is complete
    pub fn is_complete(&self) -> bool {
        self.current_index >= self.frames.len()
    }

    /// Get current frame index
    pub fn current_frame(&self) -> usize {
        self.current_index
    }

    /// Get total number of frames
    pub fn total_frames(&self) -> usize {
        self.frames.len()
    }
}

/// Create a snapshot of physics state
pub fn create_snapshot(world: &PhysicsWorld, frame: u64) -> PhysicsSnapshot {
    #[cfg(feature = "profiling")]
    profile_scope!("create_physics_snapshot", ProfileCategory::Physics);

    let mut transforms = HashMap::new();
    let mut velocities = HashMap::new();

    // Collect all entity states
    for entity_id in world.entity_ids() {
        if let Some(transform) = world.get_transform(entity_id) {
            transforms.insert(entity_id, transform);
        }
        if let Some(velocity) = world.get_velocity(entity_id) {
            velocities.insert(entity_id, velocity);
        }
    }

    let state_hash = hash_physics_state(world);

    PhysicsSnapshot { frame, transforms, velocities, state_hash }
}

/// Restore physics state from snapshot
pub fn restore_snapshot(
    world: &mut PhysicsWorld,
    snapshot: &PhysicsSnapshot,
) -> Result<(), DeterministicError> {
    #[cfg(feature = "profiling")]
    profile_scope!("restore_physics_snapshot", ProfileCategory::Physics);

    // Restore transforms
    for (entity_id, (position, rotation)) in &snapshot.transforms {
        world.set_transform(*entity_id, *position, *rotation);
    }

    // Restore velocities
    for (entity_id, (linear, angular)) in &snapshot.velocities {
        world.set_velocity(*entity_id, *linear, *angular);
    }

    tracing::info!(
        frame = snapshot.frame,
        entities = snapshot.transforms.len(),
        state_hash = snapshot.state_hash,
        "Restored physics snapshot"
    );

    Ok(())
}

/// Hash the current physics state for verification
///
/// This creates a deterministic hash of all entity transforms and velocities.
/// Used to verify that replay produces identical results.
pub fn hash_physics_state(world: &PhysicsWorld) -> u64 {
    #[cfg(feature = "profiling")]
    profile_scope!("hash_physics_state", ProfileCategory::Physics);

    use std::collections::hash_map::DefaultHasher;

    let mut hasher = DefaultHasher::new();

    // Sort entity IDs for deterministic ordering
    let mut entity_ids: Vec<_> = world.entity_ids().collect();
    entity_ids.sort_unstable();

    // Hash each entity's state in order
    for entity_id in entity_ids {
        entity_id.hash(&mut hasher);

        if let Some((pos, rot)) = world.get_transform(entity_id) {
            // Hash position (convert to bits for exact comparison)
            hash_vec3(&pos, &mut hasher);
            hash_quat(&rot, &mut hasher);
        }

        if let Some((linvel, angvel)) = world.get_velocity(entity_id) {
            hash_vec3(&linvel, &mut hasher);
            hash_vec3(&angvel, &mut hasher);
        }
    }

    hasher.finish()
}

/// Hash a Vec3 deterministically
#[inline]
fn hash_vec3(v: &Vec3, hasher: &mut impl Hasher) {
    v.x.to_bits().hash(hasher);
    v.y.to_bits().hash(hasher);
    v.z.to_bits().hash(hasher);
}

/// Hash a Quat deterministically
#[inline]
fn hash_quat(q: &Quat, hasher: &mut impl Hasher) {
    q.x.to_bits().hash(hasher);
    q.y.to_bits().hash(hasher);
    q.z.to_bits().hash(hasher);
    q.w.to_bits().hash(hasher);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::RigidBody;
    use crate::config::PhysicsConfig;

    #[test]
    fn test_replay_recorder_creation() {
        let recorder = ReplayRecorder::new();
        assert_eq!(recorder.current_frame(), 0);
        assert_eq!(recorder.frames().len(), 0);
    }

    #[test]
    fn test_record_input() {
        let mut recorder = ReplayRecorder::new();

        recorder.record_input(PhysicsInput::ApplyForce {
            entity_id: 1,
            force: Vec3::new(0.0, 10.0, 0.0),
        });

        assert_eq!(recorder.pending_inputs.len(), 1);
    }

    #[test]
    fn test_snapshot_creation() {
        let config = PhysicsConfig::default().with_deterministic(true);
        let mut world = PhysicsWorld::new(config);

        let rb = RigidBody::dynamic(1.0);
        world.add_rigidbody(1, &rb, Vec3::ZERO, Quat::IDENTITY);

        let snapshot = create_snapshot(&world, 0);
        assert_eq!(snapshot.frame, 0);
        assert_eq!(snapshot.transforms.len(), 1);
    }

    #[test]
    fn test_snapshot_restore() {
        let config = PhysicsConfig::default().with_deterministic(true);
        let mut world = PhysicsWorld::new(config);

        let rb = RigidBody::dynamic(1.0);
        world.add_rigidbody(1, &rb, Vec3::new(1.0, 2.0, 3.0), Quat::IDENTITY);

        let snapshot = create_snapshot(&world, 0);

        // Modify state
        world.set_transform(1, Vec3::new(5.0, 6.0, 7.0), Quat::IDENTITY);

        // Restore
        restore_snapshot(&mut world, &snapshot).unwrap();

        let (pos, _) = world.get_transform(1).unwrap();
        assert!((pos.x - 1.0).abs() < 0.01);
        assert!((pos.y - 2.0).abs() < 0.01);
        assert!((pos.z - 3.0).abs() < 0.01);
    }

    #[test]
    fn test_state_hashing_deterministic() {
        let config = PhysicsConfig::default().with_deterministic(true);
        let mut world = PhysicsWorld::new(config);

        let rb = RigidBody::dynamic(1.0);
        world.add_rigidbody(1, &rb, Vec3::new(1.0, 2.0, 3.0), Quat::IDENTITY);

        let hash1 = hash_physics_state(&world);
        let hash2 = hash_physics_state(&world);

        // Same state should produce same hash
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_state_hashing_detects_changes() {
        let config = PhysicsConfig::default().with_deterministic(true);
        let mut world = PhysicsWorld::new(config);

        let rb = RigidBody::dynamic(1.0);
        world.add_rigidbody(1, &rb, Vec3::new(1.0, 2.0, 3.0), Quat::IDENTITY);

        let hash1 = hash_physics_state(&world);

        // Change state
        world.set_transform(1, Vec3::new(1.0, 2.0, 3.001), Quat::IDENTITY);

        let hash2 = hash_physics_state(&world);

        // Different state should produce different hash
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_replay_player_creation() {
        let snapshot = PhysicsSnapshot {
            frame: 0,
            transforms: HashMap::new(),
            velocities: HashMap::new(),
            state_hash: 0,
        };

        let player = ReplayPlayer::new(snapshot, Vec::new(), true);
        assert_eq!(player.current_frame(), 0);
        assert_eq!(player.total_frames(), 0);
        assert!(player.is_complete());
    }

    #[test]
    fn test_replay_player_next_frame() {
        let snapshot = PhysicsSnapshot {
            frame: 0,
            transforms: HashMap::new(),
            velocities: HashMap::new(),
            state_hash: 0,
        };

        let frames = vec![RecordedFrame {
            frame: 0,
            inputs: vec![PhysicsInput::ApplyForce {
                entity_id: 1,
                force: Vec3::new(0.0, 10.0, 0.0),
            }],
            state_hash: 12345,
        }];

        let mut player = ReplayPlayer::new(snapshot, frames, false);

        let inputs = player.next_frame().unwrap();
        assert_eq!(inputs.len(), 1);

        assert!(player.next_frame().is_none());
        assert!(player.is_complete());
    }

    #[test]
    fn test_memory_usage() {
        let mut recorder = ReplayRecorder::new();
        let initial_usage = recorder.memory_usage();

        recorder.record_input(PhysicsInput::ApplyForce {
            entity_id: 1,
            force: Vec3::new(0.0, 10.0, 0.0),
        });

        let after_usage = recorder.memory_usage();
        assert!(after_usage > initial_usage);
    }
}
