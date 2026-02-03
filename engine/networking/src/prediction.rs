//! Client-side prediction and server reconciliation
//!
//! Implements client-side prediction to make multiplayer feel responsive despite network latency.
//! Key features:
//! - Input buffering with sequence numbers
//! - Local state prediction
//! - Server reconciliation with rollback/replay
//! - Smooth error correction

use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use tracing::{debug, trace, warn};

/// Buffered player input with sequence number
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferedInput {
    /// Sequence number (monotonically increasing)
    pub sequence: u32,
    /// Timestamp in milliseconds
    pub timestamp: u64,
    /// Movement input (normalized direction vector)
    pub movement: Vec3,
    /// Look delta (mouse/stick movement)
    pub look_delta: Vec3,
    /// Button states (bitfield)
    pub buttons: u32,
}

/// Input buffer for client prediction
pub struct InputBuffer {
    /// Buffered inputs (not yet acknowledged by server)
    pending_inputs: VecDeque<BufferedInput>,
    /// Maximum buffer size
    max_size: usize,
    /// Current sequence number
    current_sequence: u32,
}

impl InputBuffer {
    /// Create a new input buffer
    pub fn new(max_size: usize) -> Self {
        Self { pending_inputs: VecDeque::with_capacity(max_size), max_size, current_sequence: 0 }
    }

    /// Add input to buffer
    pub fn push_input(
        &mut self,
        timestamp: u64,
        movement: Vec3,
        look_delta: Vec3,
        buttons: u32,
    ) -> u32 {
        let sequence = self.current_sequence;
        self.current_sequence = self.current_sequence.wrapping_add(1);

        let input = BufferedInput { sequence, timestamp, movement, look_delta, buttons };

        self.pending_inputs.push_back(input);

        // Trim buffer if full
        if self.pending_inputs.len() > self.max_size {
            self.pending_inputs.pop_front();
            warn!(
                sequence = sequence,
                max_size = self.max_size,
                "Input buffer overflow, dropping oldest input"
            );
        }

        sequence
    }

    /// Acknowledge inputs up to sequence number (inclusive)
    pub fn acknowledge(&mut self, sequence: u32) {
        let before_len = self.pending_inputs.len();

        // Remove all inputs up to and including sequence
        self.pending_inputs.retain(|input| input.sequence > sequence);

        let removed = before_len - self.pending_inputs.len();

        if removed > 0 {
            trace!(
                acknowledged_sequence = sequence,
                removed_inputs = removed,
                remaining = self.pending_inputs.len(),
                "Acknowledged inputs"
            );
        }
    }

    /// Get all pending inputs
    pub fn pending_inputs(&self) -> &VecDeque<BufferedInput> {
        &self.pending_inputs
    }

    /// Get inputs from sequence onwards (for replay)
    pub fn inputs_from_sequence(&self, sequence: u32) -> Vec<BufferedInput> {
        self.pending_inputs
            .iter()
            .filter(|input| input.sequence >= sequence)
            .cloned()
            .collect()
    }

    /// Clear all inputs
    pub fn clear(&mut self) {
        self.pending_inputs.clear();
        trace!("Input buffer cleared");
    }

    /// Get buffer size
    pub fn len(&self) -> usize {
        self.pending_inputs.len()
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.pending_inputs.is_empty()
    }
}

/// Configuration for client prediction
#[derive(Debug, Clone)]
pub struct PredictionConfig {
    /// Enable prediction
    pub enabled: bool,
    /// Movement speed (units per second)
    pub movement_speed: f32,
    /// Reconciliation error threshold (units)
    pub error_threshold: f32,
    /// Maximum prediction time (milliseconds)
    pub max_prediction_time_ms: u64,
}

impl Default for PredictionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            movement_speed: 5.0,
            error_threshold: 0.1,
            max_prediction_time_ms: 500,
        }
    }
}

/// Predicted state for local player
#[derive(Debug, Clone)]
struct PredictedState {
    position: Vec3,
    velocity: Vec3,
    rotation: Quat,
}

impl Default for PredictedState {
    fn default() -> Self {
        Self { position: Vec3::ZERO, velocity: Vec3::ZERO, rotation: Quat::IDENTITY }
    }
}

/// Client-side predictor for local player
pub struct ClientPredictor {
    /// Input buffer
    input_buffer: InputBuffer,
    /// Predicted state (local simulation)
    predicted_state: PredictedState,
    /// Last acknowledged server state
    last_server_state: Option<ServerState>,
    /// Configuration
    config: PredictionConfig,
}

/// Server state received from network
#[derive(Debug, Clone)]
struct ServerState {
    sequence: u32,
    position: Vec3,
    velocity: Vec3,
    rotation: Quat,
}

impl ClientPredictor {
    /// Create a new client predictor
    pub fn new(config: PredictionConfig) -> Self {
        Self {
            input_buffer: InputBuffer::new(60), // 1 second at 60fps
            predicted_state: PredictedState::default(),
            last_server_state: None,
            config,
        }
    }

    /// Process local input and predict movement
    pub fn process_input(
        &mut self,
        timestamp: u64,
        movement: Vec3,
        look_delta: Vec3,
        buttons: u32,
        dt: f32,
    ) -> u32 {
        if !self.config.enabled {
            return 0;
        }

        // Add to input buffer
        let sequence = self.input_buffer.push_input(timestamp, movement, look_delta, buttons);

        // Apply input to predicted state
        self.apply_input(movement, look_delta, dt);

        sequence
    }

    /// Apply input to predicted state
    fn apply_input(&mut self, movement: Vec3, look_delta: Vec3, dt: f32) {
        // Update rotation from look delta (mouse/stick input)
        let yaw = look_delta.x * 0.002; // Mouse sensitivity
        let _pitch = look_delta.y * 0.002; // TODO: Implement pitch rotation

        let yaw_quat = Quat::from_rotation_y(yaw);
        self.predicted_state.rotation = yaw_quat * self.predicted_state.rotation;

        // Update velocity from movement input
        let forward = self.predicted_state.rotation * Vec3::NEG_Z; // Forward in -Z
        let right = self.predicted_state.rotation * Vec3::X;

        let move_dir = forward * movement.z + right * movement.x;
        let move_dir_normalized =
            if move_dir.length_squared() > 0.0 { move_dir.normalize() } else { Vec3::ZERO };

        self.predicted_state.velocity = move_dir_normalized * self.config.movement_speed;

        // Update position
        self.predicted_state.position += self.predicted_state.velocity * dt;
    }

    /// Reconcile with authoritative server state
    pub fn reconcile(
        &mut self,
        server_sequence: u32,
        server_position: Vec3,
        server_velocity: Vec3,
        server_rotation: Quat,
        _current_time: u64,
    ) {
        // Store server state
        let server_state = ServerState {
            sequence: server_sequence,
            position: server_position,
            velocity: server_velocity,
            rotation: server_rotation,
        };

        self.last_server_state = Some(server_state);

        // Acknowledge inputs up to this sequence
        self.input_buffer.acknowledge(server_sequence);

        // Check if prediction matches server
        let position_error = (self.predicted_state.position - server_position).length();

        if position_error > self.config.error_threshold {
            // Prediction error detected, reconcile
            debug!(
                error = %format!("{:.3}", position_error),
                threshold = %format!("{:.3}", self.config.error_threshold),
                sequence = server_sequence,
                "Prediction error detected, reconciling"
            );

            // Reset to server state
            self.predicted_state.position = server_position;
            self.predicted_state.velocity = server_velocity;
            self.predicted_state.rotation = server_rotation;

            // Replay pending inputs
            let pending_inputs = self.input_buffer.inputs_from_sequence(server_sequence + 1);
            let replay_count = pending_inputs.len();

            for input in pending_inputs {
                // Estimate dt (assume 60fps)
                let dt = 1.0 / 60.0;
                self.apply_input(input.movement, input.look_delta, dt);
            }

            debug!(
                replayed_inputs = replay_count,
                final_error = %format!("{:.3}", (self.predicted_state.position - server_position).length()),
                "Input replay complete"
            );
        } else {
            trace!(
                error = %format!("{:.4}", position_error),
                threshold = %format!("{:.3}", self.config.error_threshold),
                "Prediction accurate"
            );
        }
    }

    /// Get predicted position
    pub fn predicted_position(&self) -> Vec3 {
        self.predicted_state.position
    }

    /// Get predicted rotation
    pub fn predicted_rotation(&self) -> Quat {
        self.predicted_state.rotation
    }

    /// Get predicted velocity
    pub fn predicted_velocity(&self) -> Vec3 {
        self.predicted_state.velocity
    }

    /// Get input buffer (for inspection)
    pub fn input_buffer(&self) -> &InputBuffer {
        &self.input_buffer
    }

    /// Set predicted position (for initialization)
    pub fn set_position(&mut self, position: Vec3) {
        self.predicted_state.position = position;
    }

    /// Set predicted rotation (for initialization)
    pub fn set_rotation(&mut self, rotation: Quat) {
        self.predicted_state.rotation = rotation;
    }

    /// Get last server state (if any)
    pub fn last_server_state(&self) -> Option<(u32, Vec3, Vec3, Quat)> {
        self.last_server_state
            .as_ref()
            .map(|s| (s.sequence, s.position, s.velocity, s.rotation))
    }
}

/// Smooth error correction to avoid visual snapping
pub struct ErrorCorrector {
    /// Current error offset
    error_offset: Vec3,
    /// Error correction rate (units per second)
    correction_rate: f32,
}

impl ErrorCorrector {
    /// Create a new error corrector
    pub fn new(correction_rate: f32) -> Self {
        Self { error_offset: Vec3::ZERO, correction_rate }
    }

    /// Set error to correct
    pub fn set_error(&mut self, error: Vec3) {
        let error_magnitude = error.length();
        self.error_offset = error;

        if error_magnitude > 0.001 {
            debug!(
                error_magnitude = %format!("{:.3}", error_magnitude),
                "Error correction started"
            );
        }
    }

    /// Update correction (call each frame)
    pub fn update(&mut self, dt: f32) -> Vec3 {
        let error_length = self.error_offset.length();

        if error_length < 0.001 {
            // Error corrected
            self.error_offset = Vec3::ZERO;
            return Vec3::ZERO;
        }

        // Calculate correction amount for this frame
        let correction_distance = self.correction_rate * dt;

        if correction_distance >= error_length {
            // Fully correct this frame
            let correction = self.error_offset;
            self.error_offset = Vec3::ZERO;
            trace!("Error correction complete");
            correction
        } else {
            // Partial correction
            let correction_dir = self.error_offset.normalize();
            let correction = correction_dir * correction_distance;
            self.error_offset -= correction;
            correction
        }
    }

    /// Get current error
    pub fn current_error(&self) -> Vec3 {
        self.error_offset
    }

    /// Check if correction is complete
    pub fn is_corrected(&self) -> bool {
        self.error_offset.length() < 0.001
    }

    /// Reset correction
    pub fn reset(&mut self) {
        self.error_offset = Vec3::ZERO;
    }
}

/// Adaptive error correction (faster for large errors)
pub struct AdaptiveErrorCorrector {
    /// Current error offset
    error_offset: Vec3,
    /// Base correction rate (units per second)
    base_rate: f32,
    /// Max correction rate (units per second)
    max_rate: f32,
    /// Snap threshold (units) - errors larger than this snap immediately
    snap_threshold: f32,
}

impl AdaptiveErrorCorrector {
    /// Create a new adaptive error corrector
    pub fn new(base_rate: f32, max_rate: f32) -> Self {
        Self { error_offset: Vec3::ZERO, base_rate, max_rate, snap_threshold: 5.0 }
    }

    /// Set error to correct
    pub fn set_error(&mut self, error: Vec3) {
        let error_magnitude = error.length();
        self.error_offset = error;

        if error_magnitude > 0.001 {
            debug!(
                error_magnitude = %format!("{:.3}", error_magnitude),
                "Adaptive error correction started"
            );
        }
    }

    /// Update correction (call each frame)
    pub fn update(&mut self, dt: f32) -> Vec3 {
        let error_length = self.error_offset.length();

        if error_length < 0.001 {
            self.error_offset = Vec3::ZERO;
            return Vec3::ZERO;
        }

        // Adaptive rate: faster for larger errors
        let rate = if error_length > self.snap_threshold {
            // Large error: snap immediately
            debug!(
                error_length = %format!("{:.3}", error_length),
                threshold = self.snap_threshold,
                "Snapping to correct position"
            );
            error_length / dt
        } else if error_length > 1.0 {
            // Medium error: use max rate
            self.max_rate
        } else {
            // Small error: use base rate
            self.base_rate
        };

        let correction_distance = rate * dt;

        if correction_distance >= error_length {
            let correction = self.error_offset;
            self.error_offset = Vec3::ZERO;
            trace!("Adaptive error correction complete");
            correction
        } else {
            let correction_dir = self.error_offset.normalize();
            let correction = correction_dir * correction_distance;
            self.error_offset -= correction;
            correction
        }
    }

    /// Get current error
    pub fn current_error(&self) -> Vec3 {
        self.error_offset
    }

    /// Check if correction is complete
    pub fn is_corrected(&self) -> bool {
        self.error_offset.length() < 0.001
    }

    /// Reset correction
    pub fn reset(&mut self) {
        self.error_offset = Vec3::ZERO;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_buffer() {
        let mut buffer = InputBuffer::new(100);

        // Add inputs
        let seq1 = buffer.push_input(100, Vec3::X, Vec3::ZERO, 0);
        let seq2 = buffer.push_input(110, Vec3::Y, Vec3::ZERO, 0);
        let seq3 = buffer.push_input(120, Vec3::Z, Vec3::ZERO, 0);

        assert_eq!(buffer.len(), 3);
        assert_eq!(seq1, 0);
        assert_eq!(seq2, 1);
        assert_eq!(seq3, 2);

        // Acknowledge first two
        buffer.acknowledge(seq2);

        assert_eq!(buffer.len(), 1);
        assert_eq!(buffer.pending_inputs()[0].sequence, seq3);
    }

    #[test]
    fn test_buffer_overflow() {
        let mut buffer = InputBuffer::new(10);

        // Add more than capacity
        for i in 0..20 {
            buffer.push_input(i, Vec3::ZERO, Vec3::ZERO, 0);
        }

        // Should only keep last 10
        assert_eq!(buffer.len(), 10);
        assert_eq!(buffer.pending_inputs()[0].sequence, 10);
    }

    #[test]
    fn test_inputs_from_sequence() {
        let mut buffer = InputBuffer::new(100);

        for i in 0..10 {
            buffer.push_input(i * 10, Vec3::X * i as f32, Vec3::ZERO, 0);
        }

        let inputs = buffer.inputs_from_sequence(5);
        assert_eq!(inputs.len(), 5);
        assert_eq!(inputs[0].sequence, 5);
    }

    #[test]
    fn test_client_predictor_basic() {
        let mut predictor = ClientPredictor::new(PredictionConfig::default());

        // Set initial position
        predictor.set_position(Vec3::ZERO);

        // Process forward movement
        let seq = predictor.process_input(
            0,
            Vec3::new(0.0, 0.0, 1.0), // Forward
            Vec3::ZERO,
            0,
            1.0 / 60.0,
        );

        assert_eq!(seq, 0);
        assert!(predictor.predicted_position().z < 0.0); // Should move forward (-Z)
    }

    #[test]
    fn test_prediction_movement() {
        let mut predictor = ClientPredictor::new(PredictionConfig::default());
        predictor.set_position(Vec3::ZERO);

        // Process forward movement for 1 second at 60fps
        for _ in 0..60 {
            predictor.process_input(0, Vec3::new(0.0, 0.0, 1.0), Vec3::ZERO, 0, 1.0 / 60.0);
        }

        let predicted_pos = predictor.predicted_position();

        // Should have moved ~5 units forward (speed 5.0 * 1 second)
        assert!((predicted_pos.z.abs() - 5.0).abs() < 0.1);
    }

    #[test]
    fn test_reconciliation_no_error() {
        let mut predictor = ClientPredictor::new(PredictionConfig::default());
        predictor.set_position(Vec3::ZERO);

        // Predict movement
        let seq = predictor.process_input(0, Vec3::new(1.0, 0.0, 0.0), Vec3::ZERO, 0, 1.0);

        let predicted_before = predictor.predicted_position();

        // Server confirms our prediction (within threshold)
        predictor.reconcile(seq, predicted_before, Vec3::ZERO, Quat::IDENTITY, 0);

        // Should remain the same
        let predicted_after = predictor.predicted_position();
        assert!((predicted_after - predicted_before).length() < 0.01);
    }

    #[test]
    fn test_reconciliation_with_error() {
        let mut predictor = ClientPredictor::new(PredictionConfig::default());
        predictor.set_position(Vec3::ZERO);

        // Predict movement
        let seq = predictor.process_input(0, Vec3::new(1.0, 0.0, 0.0), Vec3::ZERO, 0, 1.0);

        let predicted_before = predictor.predicted_position();

        // Server says we're slightly off (beyond threshold)
        let server_pos = predicted_before + Vec3::new(0.0, 0.0, 0.5);

        predictor.reconcile(seq, server_pos, Vec3::ZERO, Quat::IDENTITY, 0);

        // Should correct to match server
        let predicted_after = predictor.predicted_position();
        assert!((predicted_after - server_pos).length() < 0.1);
    }

    #[test]
    fn test_input_replay() {
        let mut predictor = ClientPredictor::new(PredictionConfig::default());
        predictor.set_position(Vec3::ZERO);

        // Send multiple inputs
        let seq1 = predictor.process_input(0, Vec3::X, Vec3::ZERO, 0, 0.016);
        let _seq2 = predictor.process_input(16, Vec3::X, Vec3::ZERO, 0, 0.016);
        let _seq3 = predictor.process_input(32, Vec3::X, Vec3::ZERO, 0, 0.016);

        // Server acknowledges first input but with different position
        predictor.reconcile(seq1, Vec3::new(0.1, 0.0, 0.0), Vec3::ZERO, Quat::IDENTITY, 32);

        // Should replay seq2 and seq3
        assert_eq!(predictor.input_buffer().len(), 2);
    }

    #[test]
    fn test_error_correction() {
        let mut corrector = ErrorCorrector::new(10.0); // 10 units/sec

        // Set 5 unit error
        corrector.set_error(Vec3::new(5.0, 0.0, 0.0));

        // Correct over 0.5 seconds (should correct fully)
        let correction = corrector.update(0.5);

        assert!(correction.length() >= 4.9);
        assert!(corrector.is_corrected());
    }

    #[test]
    fn test_error_correction_partial() {
        let mut corrector = ErrorCorrector::new(10.0);

        corrector.set_error(Vec3::new(10.0, 0.0, 0.0));

        // Correct for 0.1 seconds (should only correct 1 unit)
        let correction = corrector.update(0.1);

        assert!((correction.length() - 1.0).abs() < 0.1);
        assert!(!corrector.is_corrected());
        assert!((corrector.current_error().length() - 9.0).abs() < 0.1);
    }

    #[test]
    fn test_adaptive_correction_small_error() {
        let mut corrector = AdaptiveErrorCorrector::new(5.0, 20.0);

        // Small error (should use base rate)
        corrector.set_error(Vec3::new(0.5, 0.0, 0.0));
        let correction = corrector.update(0.1);

        // Base rate 5.0 * 0.1s = 0.5 units
        assert!((correction.length() - 0.5).abs() < 0.1);
    }

    #[test]
    fn test_adaptive_correction_large_error() {
        let mut corrector = AdaptiveErrorCorrector::new(5.0, 20.0);

        // Large error (should snap)
        corrector.set_error(Vec3::new(10.0, 0.0, 0.0));
        let correction = corrector.update(0.016); // One frame at 60fps

        // Should correct most of it (snap behavior)
        assert!(correction.length() >= 9.0);
    }

    #[test]
    fn test_sequence_wrapping() {
        let mut buffer = InputBuffer::new(10);

        // Test that sequence numbers wrap correctly
        buffer.current_sequence = u32::MAX - 5;

        for _ in 0..10 {
            buffer.push_input(0, Vec3::ZERO, Vec3::ZERO, 0);
        }

        // Should have wrapped around
        assert!(buffer.pending_inputs().back().unwrap().sequence < 10);
    }
}
