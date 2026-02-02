//! Client-side prediction system for networked physics
//!
//! This module implements client-side prediction to minimize perceived latency in networked games:
//! - **Input Buffering**: Store player inputs with timestamps
//! - **Local Prediction**: Simulate physics locally without waiting for server
//! - **State Reconciliation**: Compare server state, replay inputs if mismatch
//! - **Error Smoothing**: Interpolate to correct position when reconciliation occurs
//!
//! # Architecture
//!
//! 1. Client stores inputs in circular buffer
//! 2. Client simulates physics locally (prediction)
//! 3. Server sends authoritative state updates
//! 4. Client reconciles: if mismatch, rewind and replay inputs
//! 5. Client smoothly interpolates any position errors
//!
//! # Performance Targets
//!
//! - Input buffering: < 1µs per input
//! - State reconciliation: < 100µs
//! - Input replay: < 1ms for 60 inputs
//! - Prediction overhead: < 5% of normal physics step

use crate::world::PhysicsWorld;
use engine_core::ecs::{Entity, World};
use engine_math::{Quat, Vec3};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[cfg(feature = "profiling")]
use agent_game_engine_profiling::{profile_scope, ProfileCategory};

/// Maximum number of inputs to buffer (2 seconds at 60 FPS = 120 inputs)
const MAX_INPUT_BUFFER_SIZE: usize = 120;

/// Maximum position error before triggering immediate correction (teleport)
const MAX_ERROR_THRESHOLD: f32 = 5.0;

/// Smoothing factor for error correction (higher = faster correction)
const ERROR_SMOOTHING_FACTOR: f32 = 0.1;

/// Player input for physics simulation
///
/// Stores all input data needed to deterministically simulate physics.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PlayerInput {
    /// Sequence number (increments each frame)
    pub sequence: u32,
    /// Timestamp when input was created (milliseconds)
    pub timestamp: u64,
    /// Movement direction (normalized)
    pub movement: Vec3,
    /// Jump pressed this frame
    pub jump: bool,
    /// Delta time for this input (seconds)
    pub delta_time: f32,
}

impl PlayerInput {
    /// Create a new input
    pub fn new(sequence: u32, timestamp: u64, movement: Vec3, jump: bool, delta_time: f32) -> Self {
        Self { sequence, timestamp, movement, jump, delta_time }
    }
}

/// Predicted entity state
///
/// Stores the predicted position/rotation for an entity, along with
/// error correction data for smooth reconciliation.
#[derive(Debug, Clone, Copy)]
pub struct PredictedState {
    /// Entity being predicted
    pub entity: Entity,
    /// Last server-confirmed position
    pub server_position: Vec3,
    /// Last server-confirmed rotation
    pub server_rotation: Quat,
    /// Last server-confirmed velocity
    pub server_velocity: Vec3,
    /// Sequence number of last confirmed state
    pub confirmed_sequence: u32,
    /// Current predicted position
    pub predicted_position: Vec3,
    /// Current predicted rotation
    pub predicted_rotation: Quat,
    /// Position error (for smoothing)
    pub position_error: Vec3,
    /// Rotation error (for smoothing)
    pub rotation_error: Quat,
}

impl PredictedState {
    /// Create a new predicted state
    pub fn new(entity: Entity, position: Vec3, rotation: Quat, velocity: Vec3) -> Self {
        Self {
            entity,
            server_position: position,
            server_rotation: rotation,
            server_velocity: velocity,
            confirmed_sequence: 0,
            predicted_position: position,
            predicted_rotation: rotation,
            position_error: Vec3::ZERO,
            rotation_error: Quat::IDENTITY,
        }
    }

    /// Update server-confirmed state
    pub fn update_server_state(
        &mut self,
        sequence: u32,
        position: Vec3,
        rotation: Quat,
        velocity: Vec3,
    ) {
        self.confirmed_sequence = sequence;
        self.server_position = position;
        self.server_rotation = rotation;
        self.server_velocity = velocity;
    }

    /// Calculate position error between predicted and server state
    pub fn calculate_error(&self) -> f32 {
        (self.predicted_position - self.server_position).length()
    }

    /// Check if error exceeds threshold (needs immediate correction)
    pub fn needs_immediate_correction(&self) -> bool {
        self.calculate_error() > MAX_ERROR_THRESHOLD
    }
}

/// Input buffer with circular queue
///
/// Efficiently stores recent inputs for reconciliation.
/// Uses VecDeque for O(1) push/pop on both ends.
pub struct InputBuffer {
    /// Buffered inputs (oldest to newest)
    inputs: VecDeque<PlayerInput>,
    /// Next sequence number to assign
    next_sequence: u32,
}

impl InputBuffer {
    /// Create a new input buffer
    pub fn new() -> Self {
        Self { inputs: VecDeque::with_capacity(MAX_INPUT_BUFFER_SIZE), next_sequence: 0 }
    }

    /// Add a new input to the buffer
    ///
    /// # Performance
    ///
    /// - Target: < 1µs
    /// - O(1) push to back of deque
    pub fn add_input(&mut self, timestamp: u64, movement: Vec3, jump: bool, delta_time: f32) {
        #[cfg(feature = "profiling")]
        profile_scope!("input_buffer_add", ProfileCategory::Physics);

        let input = PlayerInput::new(self.next_sequence, timestamp, movement, jump, delta_time);
        self.next_sequence += 1;

        self.inputs.push_back(input);

        // Remove oldest if buffer is full
        if self.inputs.len() > MAX_INPUT_BUFFER_SIZE {
            self.inputs.pop_front();
        }
    }

    /// Get inputs starting from a sequence number
    ///
    /// Returns all inputs with sequence >= start_sequence.
    /// Used for input replay during reconciliation.
    pub fn get_inputs_from(&self, start_sequence: u32) -> Vec<PlayerInput> {
        #[cfg(feature = "profiling")]
        profile_scope!("input_buffer_get_from", ProfileCategory::Physics);

        self.inputs
            .iter()
            .filter(|input| input.sequence >= start_sequence)
            .copied()
            .collect()
    }

    /// Remove inputs older than a sequence number
    ///
    /// Called after server confirms state to free memory.
    pub fn remove_before(&mut self, sequence: u32) {
        #[cfg(feature = "profiling")]
        profile_scope!("input_buffer_remove_before", ProfileCategory::Physics);

        while let Some(input) = self.inputs.front() {
            if input.sequence < sequence {
                self.inputs.pop_front();
            } else {
                break;
            }
        }
    }

    /// Get the current sequence number
    pub fn current_sequence(&self) -> u32 {
        self.next_sequence
    }

    /// Get the number of buffered inputs
    pub fn len(&self) -> usize {
        self.inputs.len()
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.inputs.is_empty()
    }

    /// Clear all buffered inputs
    pub fn clear(&mut self) {
        self.inputs.clear();
        self.next_sequence = 0;
    }
}

impl Default for InputBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// Client-side prediction system
///
/// Manages input buffering, prediction, and reconciliation for client entities.
pub struct PredictionSystem {
    /// Input buffer for local player
    input_buffer: InputBuffer,
    /// Predicted state for local player
    predicted_state: Option<PredictedState>,
    /// Entity ID in physics world (u64)
    physics_entity_id: Option<u64>,
}

impl PredictionSystem {
    /// Create a new prediction system
    pub fn new() -> Self {
        Self { input_buffer: InputBuffer::new(), predicted_state: None, physics_entity_id: None }
    }

    /// Start predicting an entity
    ///
    /// Call this when spawning the local player entity.
    pub fn start_prediction(
        &mut self,
        entity: Entity,
        physics_entity_id: u64,
        position: Vec3,
        rotation: Quat,
        velocity: Vec3,
    ) {
        tracing::info!(
            entity_id = ?entity,
            physics_id = physics_entity_id,
            position = ?position,
            "Started client-side prediction"
        );

        self.predicted_state = Some(PredictedState::new(entity, position, rotation, velocity));
        self.physics_entity_id = Some(physics_entity_id);
        self.input_buffer.clear();
    }

    /// Stop predicting (when disconnecting or switching entities)
    pub fn stop_prediction(&mut self) {
        tracing::debug!("Stopped client-side prediction");
        self.predicted_state = None;
        self.physics_entity_id = None;
        self.input_buffer.clear();
    }

    /// Add a new input and predict locally
    ///
    /// Called each frame with player input. Stores the input and
    /// simulates physics locally without waiting for server.
    ///
    /// # Performance
    ///
    /// - Target: < 50µs (excluding physics step)
    pub fn add_input_and_predict(
        &mut self,
        timestamp: u64,
        movement: Vec3,
        jump: bool,
        delta_time: f32,
        physics: &mut PhysicsWorld,
    ) {
        #[cfg(feature = "profiling")]
        profile_scope!("prediction_add_input", ProfileCategory::Physics);

        // Buffer the input
        self.input_buffer.add_input(timestamp, movement, jump, delta_time);

        // Apply input to physics simulation
        if let Some(physics_id) = self.physics_entity_id {
            self.apply_input_to_physics(physics_id, movement, jump, physics);

            // Update predicted state with new position after physics step
            if let Some(state) = &mut self.predicted_state {
                if let Some((pos, rot)) = physics.get_transform(physics_id) {
                    state.predicted_position = pos;
                    state.predicted_rotation = rot;
                }
            }
        }
    }

    /// Reconcile with server state
    ///
    /// Called when receiving authoritative state from server.
    /// Compares predicted state with server state, and replays inputs if mismatch.
    ///
    /// # Performance
    ///
    /// - Target: < 100µs without replay
    /// - Replay target: < 1ms for 60 inputs
    pub fn reconcile(
        &mut self,
        server_sequence: u32,
        server_position: Vec3,
        server_rotation: Quat,
        server_velocity: Vec3,
        physics: &mut PhysicsWorld,
    ) {
        #[cfg(feature = "profiling")]
        profile_scope!("prediction_reconcile", ProfileCategory::Physics);

        let Some(state) = &mut self.predicted_state else {
            return;
        };

        let Some(physics_id) = self.physics_entity_id else {
            return;
        };

        // Update server-confirmed state
        state.update_server_state(
            server_sequence,
            server_position,
            server_rotation,
            server_velocity,
        );

        // Calculate position error
        let error = (state.predicted_position - server_position).length();

        tracing::trace!(
            sequence = server_sequence,
            error = error,
            predicted_pos = ?state.predicted_position,
            server_pos = ?server_position,
            "Reconciling client prediction"
        );

        // If error is large, need to reconcile
        if error > 0.01 {
            // Immediate teleport if error is huge (e.g., server correction)
            if state.needs_immediate_correction() {
                tracing::warn!(
                    error = error,
                    "Large prediction error, teleporting to server position"
                );

                state.predicted_position = server_position;
                state.predicted_rotation = server_rotation;
                state.position_error = Vec3::ZERO;

                // Set physics state directly
                if let Some((_, _)) = physics.get_transform(physics_id) {
                    physics.set_transform(physics_id, server_position, server_rotation);
                    physics.set_velocity(physics_id, server_velocity, Vec3::ZERO);
                }
            } else {
                // Replay inputs from confirmed sequence
                self.replay_inputs(
                    server_sequence,
                    server_position,
                    server_rotation,
                    server_velocity,
                    physics,
                );
            }

            // Remove old inputs
            self.input_buffer.remove_before(server_sequence);
        }
    }

    /// Replay inputs from a starting state
    ///
    /// Used during reconciliation to re-simulate physics with buffered inputs.
    fn replay_inputs(
        &mut self,
        start_sequence: u32,
        start_position: Vec3,
        start_rotation: Quat,
        start_velocity: Vec3,
        physics: &mut PhysicsWorld,
    ) {
        #[cfg(feature = "profiling")]
        profile_scope!("prediction_replay_inputs", ProfileCategory::Physics);

        let Some(physics_id) = self.physics_entity_id else {
            return;
        };

        // Reset physics state to server state
        physics.set_transform(physics_id, start_position, start_rotation);
        physics.set_velocity(physics_id, start_velocity, Vec3::ZERO);

        // Get inputs to replay
        let inputs = self.input_buffer.get_inputs_from(start_sequence + 1);

        tracing::trace!(
            input_count = inputs.len(),
            start_sequence = start_sequence,
            "Replaying inputs"
        );

        // Re-apply each input
        for input in inputs {
            self.apply_input_to_physics(physics_id, input.movement, input.jump, physics);

            // Step physics with the input's delta time
            physics.step(input.delta_time);
        }

        // Update predicted state with replayed result
        if let Some(state) = &mut self.predicted_state {
            if let Some((pos, rot)) = physics.get_transform(physics_id) {
                state.predicted_position = pos;
                state.predicted_rotation = rot;
            }
        }
    }

    /// Apply input to physics simulation
    ///
    /// Helper function to apply player input to physics body.
    fn apply_input_to_physics(
        &self,
        physics_id: u64,
        movement: Vec3,
        jump: bool,
        physics: &mut PhysicsWorld,
    ) {
        // Apply movement force
        if movement.length_squared() > 0.001 {
            let force = movement.normalize() * 50.0; // Movement force magnitude
            physics.apply_force(physics_id, force);
        }

        // Apply jump impulse
        if jump {
            if let Some((linvel, _)) = physics.get_velocity(physics_id) {
                // Only jump if grounded (velocity.y near zero)
                if linvel.y.abs() < 0.1 {
                    physics.apply_impulse(physics_id, Vec3::new(0.0, 5.0, 0.0));
                }
            }
        }
    }

    /// Apply error smoothing to ECS transform
    ///
    /// Call this each frame to smooth out prediction errors.
    /// Uses exponential smoothing for natural-looking correction.
    pub fn apply_error_smoothing(&mut self, world: &mut World, dt: f32) {
        #[cfg(feature = "profiling")]
        profile_scope!("prediction_error_smoothing", ProfileCategory::Physics);

        let Some(state) = &mut self.predicted_state else {
            return;
        };

        // Calculate error
        state.position_error = state.server_position - state.predicted_position;

        // Apply exponential smoothing
        let smoothing = 1.0 - (-ERROR_SMOOTHING_FACTOR * dt).exp();
        let correction = state.position_error * smoothing;

        state.predicted_position += correction;

        // Update ECS transform with smoothed position
        if let Some(transform) = world.get_mut::<engine_math::Transform>(state.entity) {
            transform.position = state.predicted_position;
            transform.rotation = state.predicted_rotation;
        }
    }

    /// Get current input sequence number
    pub fn current_sequence(&self) -> u32 {
        self.input_buffer.current_sequence()
    }

    /// Get number of buffered inputs
    pub fn buffered_input_count(&self) -> usize {
        self.input_buffer.len()
    }

    /// Get predicted state (if any)
    pub fn predicted_state(&self) -> Option<&PredictedState> {
        self.predicted_state.as_ref()
    }
}

impl Default for PredictionSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_player_input_creation() {
        let input = PlayerInput::new(0, 1000, Vec3::new(1.0, 0.0, 0.0), false, 0.016);

        assert_eq!(input.sequence, 0);
        assert_eq!(input.timestamp, 1000);
        assert_eq!(input.movement, Vec3::new(1.0, 0.0, 0.0));
        assert!(!input.jump);
        assert!((input.delta_time - 0.016).abs() < 0.001);
    }

    #[test]
    fn test_input_buffer_add() {
        let mut buffer = InputBuffer::new();

        buffer.add_input(1000, Vec3::X, false, 0.016);
        buffer.add_input(1016, Vec3::Y, true, 0.016);

        assert_eq!(buffer.len(), 2);
        assert_eq!(buffer.current_sequence(), 2);
    }

    #[test]
    fn test_input_buffer_overflow() {
        let mut buffer = InputBuffer::new();

        // Add more than max capacity
        for i in 0..MAX_INPUT_BUFFER_SIZE + 10 {
            buffer.add_input(i as u64, Vec3::ZERO, false, 0.016);
        }

        // Should cap at max size
        assert_eq!(buffer.len(), MAX_INPUT_BUFFER_SIZE);
    }

    #[test]
    fn test_input_buffer_get_from() {
        let mut buffer = InputBuffer::new();

        for i in 0..10 {
            buffer.add_input(i * 16, Vec3::ZERO, false, 0.016);
        }

        let inputs = buffer.get_inputs_from(5);
        assert_eq!(inputs.len(), 5); // Sequences 5, 6, 7, 8, 9
        assert_eq!(inputs[0].sequence, 5);
        assert_eq!(inputs[4].sequence, 9);
    }

    #[test]
    fn test_input_buffer_remove_before() {
        let mut buffer = InputBuffer::new();

        for i in 0..10 {
            buffer.add_input(i * 16, Vec3::ZERO, false, 0.016);
        }

        buffer.remove_before(5);
        assert_eq!(buffer.len(), 5); // Sequences 5, 6, 7, 8, 9 remain
    }

    #[test]
    fn test_predicted_state_creation() {
        use engine_core::ecs::EntityAllocator;

        let mut allocator = EntityAllocator::new();
        let entity = allocator.allocate();

        let state =
            PredictedState::new(entity, Vec3::new(1.0, 2.0, 3.0), Quat::IDENTITY, Vec3::ZERO);

        assert_eq!(state.entity, entity);
        assert_eq!(state.server_position, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(state.predicted_position, Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn test_predicted_state_error_calculation() {
        use engine_core::ecs::EntityAllocator;

        let mut allocator = EntityAllocator::new();
        let entity = allocator.allocate();

        let mut state = PredictedState::new(entity, Vec3::ZERO, Quat::IDENTITY, Vec3::ZERO);

        state.predicted_position = Vec3::new(3.0, 4.0, 0.0);
        state.server_position = Vec3::ZERO;

        // Error = sqrt(3^2 + 4^2) = 5.0
        assert!((state.calculate_error() - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_predicted_state_needs_correction() {
        use engine_core::ecs::EntityAllocator;

        let mut allocator = EntityAllocator::new();
        let entity = allocator.allocate();

        let mut state = PredictedState::new(entity, Vec3::ZERO, Quat::IDENTITY, Vec3::ZERO);

        // Small error - no immediate correction
        state.predicted_position = Vec3::new(1.0, 0.0, 0.0);
        assert!(!state.needs_immediate_correction());

        // Large error - needs immediate correction
        state.predicted_position = Vec3::new(10.0, 0.0, 0.0);
        assert!(state.needs_immediate_correction());
    }

    #[test]
    fn test_prediction_system_start_stop() {
        use engine_core::ecs::EntityAllocator;

        let mut allocator = EntityAllocator::new();
        let entity = allocator.allocate();

        let mut system = PredictionSystem::new();
        assert!(system.predicted_state.is_none());

        system.start_prediction(entity, 1, Vec3::ZERO, Quat::IDENTITY, Vec3::ZERO);
        assert!(system.predicted_state.is_some());

        system.stop_prediction();
        assert!(system.predicted_state.is_none());
    }

    #[test]
    fn test_prediction_system_sequence() {
        let system = PredictionSystem::new();
        assert_eq!(system.current_sequence(), 0);
        assert_eq!(system.buffered_input_count(), 0);
    }
}
