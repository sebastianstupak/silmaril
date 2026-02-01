# Phase 2.7: Client-Side Prediction

**Status:** ⚪ Not Started
**Estimated Time:** 4-5 days
**Priority:** Critical (responsive gameplay)

---

## 🎯 **Objective**

Implement client-side prediction system that makes multiplayer feel responsive despite network latency. Predict local player movement immediately, buffer inputs, reconcile with authoritative server state, and compensate for lag on remote entities.

**Key Techniques:**
- Client-side movement prediction
- Input buffering and replay
- Server reconciliation
- Lag compensation for remote entities
- Smooth error correction

---

## 📋 **Detailed Tasks**

### **1. Input Buffer** (Day 1)

**File:** `engine/networking/src/prediction/input_buffer.rs`

```rust
use std::collections::VecDeque;

/// Buffered player input with sequence number
#[derive(Debug, Clone)]
pub struct BufferedInput {
    pub sequence: u32,
    pub timestamp: u64,
    pub movement: glam::Vec3,
    pub look_delta: glam::Vec3,
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
    pub fn new(max_size: usize) -> Self {
        Self {
            pending_inputs: VecDeque::with_capacity(max_size),
            max_size,
            current_sequence: 0,
        }
    }

    /// Add input to buffer
    pub fn push_input(
        &mut self,
        timestamp: u64,
        movement: glam::Vec3,
        look_delta: glam::Vec3,
        buttons: u32,
    ) -> u32 {
        let sequence = self.current_sequence;
        self.current_sequence += 1;

        let input = BufferedInput {
            sequence,
            timestamp,
            movement,
            look_delta,
            buttons,
        };

        self.pending_inputs.push_back(input);

        // Trim buffer if full
        if self.pending_inputs.len() > self.max_size {
            self.pending_inputs.pop_front();
            tracing::warn!("Input buffer overflow, dropping oldest input");
        }

        sequence
    }

    /// Acknowledge inputs up to sequence number
    pub fn acknowledge(&mut self, sequence: u32) {
        // Remove all inputs up to and including sequence
        self.pending_inputs
            .retain(|input| input.sequence > sequence);

        tracing::trace!("Acknowledged inputs up to sequence {}", sequence);
    }

    /// Get all pending inputs
    pub fn pending_inputs(&self) -> &VecDeque<BufferedInput> {
        &self.pending_inputs
    }

    /// Get inputs from sequence onwards
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_buffer() {
        let mut buffer = InputBuffer::new(100);

        // Add inputs
        let seq1 = buffer.push_input(100, glam::Vec3::X, glam::Vec3::ZERO, 0);
        let seq2 = buffer.push_input(110, glam::Vec3::Y, glam::Vec3::ZERO, 0);
        let seq3 = buffer.push_input(120, glam::Vec3::Z, glam::Vec3::ZERO, 0);

        assert_eq!(buffer.len(), 3);

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
            buffer.push_input(i, glam::Vec3::ZERO, glam::Vec3::ZERO, 0);
        }

        // Should only keep last 10
        assert_eq!(buffer.len(), 10);
    }
}
```

---

### **2. Client Prediction** (Day 1-2)

**File:** `engine/networking/src/prediction/client_predictor.rs`

```rust
/// Client-side prediction for local player
pub struct ClientPredictor {
    /// Input buffer
    input_buffer: InputBuffer,

    /// Predicted state (local simulation)
    predicted_state: PredictedState,

    /// Last acknowledged server state
    last_server_state: Option<(u32, Transform, Velocity)>,

    /// Configuration
    config: PredictionConfig,
}

#[derive(Debug, Clone)]
pub struct PredictionConfig {
    /// Enable prediction
    pub enabled: bool,

    /// Movement speed
    pub movement_speed: f32,

    /// Max prediction time (ms)
    pub max_prediction_time_ms: u64,
}

impl Default for PredictionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            movement_speed: 5.0,
            max_prediction_time_ms: 500,
        }
    }
}

#[derive(Debug, Clone)]
struct PredictedState {
    position: glam::Vec3,
    velocity: glam::Vec3,
    rotation: glam::Quat,
}

impl ClientPredictor {
    pub fn new(config: PredictionConfig) -> Self {
        Self {
            input_buffer: InputBuffer::new(60), // 1 second at 60fps
            predicted_state: PredictedState {
                position: glam::Vec3::ZERO,
                velocity: glam::Vec3::ZERO,
                rotation: glam::Quat::IDENTITY,
            },
            last_server_state: None,
            config,
        }
    }

    /// Process local input and predict movement
    pub fn process_input(
        &mut self,
        timestamp: u64,
        movement: glam::Vec3,
        look_delta: glam::Vec3,
        buttons: u32,
        dt: f32,
    ) -> u32 {
        if !self.config.enabled {
            return 0;
        }

        // Add to input buffer
        let sequence = self.input_buffer.push_input(
            timestamp,
            movement,
            look_delta,
            buttons,
        );

        // Apply input to predicted state
        self.apply_input(movement, look_delta, dt);

        sequence
    }

    /// Apply input to predicted state
    fn apply_input(&mut self, movement: glam::Vec3, look_delta: glam::Vec3, dt: f32) {
        // Update rotation from look delta
        let yaw = look_delta.x * 0.002; // Mouse sensitivity
        let pitch = look_delta.y * 0.002;

        let yaw_quat = glam::Quat::from_rotation_y(yaw);
        self.predicted_state.rotation = yaw_quat * self.predicted_state.rotation;

        // Update velocity from movement input
        let forward = self.predicted_state.rotation * glam::Vec3::Z;
        let right = self.predicted_state.rotation * glam::Vec3::X;

        let move_dir = forward * movement.z + right * movement.x;
        self.predicted_state.velocity = move_dir * self.config.movement_speed;

        // Update position
        self.predicted_state.position += self.predicted_state.velocity * dt;
    }

    /// Reconcile with authoritative server state
    pub fn reconcile(
        &mut self,
        server_sequence: u32,
        server_position: glam::Vec3,
        server_velocity: glam::Vec3,
        server_rotation: glam::Quat,
        current_time: u64,
    ) {
        // Store server state
        let server_transform = Transform {
            position: server_position,
            rotation: server_rotation,
            scale: glam::Vec3::ONE,
        };

        self.last_server_state = Some((
            server_sequence,
            server_transform,
            Velocity(server_velocity),
        ));

        // Acknowledge inputs up to this sequence
        self.input_buffer.acknowledge(server_sequence);

        // Check if prediction matches server
        let position_error = (self.predicted_state.position - server_position).length();

        if position_error > 0.1 {
            // Prediction error detected, reconcile
            tracing::debug!(
                "Prediction error: {:.2} units, reconciling",
                position_error
            );

            // Reset to server state
            self.predicted_state.position = server_position;
            self.predicted_state.velocity = server_velocity;
            self.predicted_state.rotation = server_rotation;

            // Replay pending inputs
            let pending_inputs = self.input_buffer.inputs_from_sequence(server_sequence + 1);

            for input in pending_inputs {
                // Estimate dt (assume 60fps)
                let dt = 1.0 / 60.0;

                self.apply_input(input.movement, input.look_delta, dt);
            }

            tracing::debug!("Replayed {} inputs after reconciliation", self.input_buffer.len());
        } else {
            tracing::trace!("Prediction accurate (error: {:.3})", position_error);
        }
    }

    /// Get predicted state
    pub fn predicted_position(&self) -> glam::Vec3 {
        self.predicted_state.position
    }

    pub fn predicted_rotation(&self) -> glam::Quat {
        self.predicted_state.rotation
    }

    pub fn predicted_velocity(&self) -> glam::Vec3 {
        self.predicted_state.velocity
    }

    /// Get input buffer
    pub fn input_buffer(&self) -> &InputBuffer {
        &self.input_buffer
    }
}
```

---

### **3. Lag Compensation** (Day 2-3)

**File:** `engine/networking/src/prediction/lag_compensation.rs`

```rust
use std::collections::{HashMap, VecDeque};

/// Lag compensation for remote entities
pub struct LagCompensator {
    /// Entity state history (entity_id -> history)
    entity_histories: HashMap<u32, EntityHistory>,

    /// Configuration
    config: LagCompensationConfig,
}

#[derive(Debug, Clone)]
pub struct LagCompensationConfig {
    /// History duration (milliseconds)
    pub history_duration_ms: u64,

    /// Interpolation delay (milliseconds)
    pub interpolation_delay_ms: u64,
}

impl Default for LagCompensationConfig {
    fn default() -> Self {
        Self {
            history_duration_ms: 1000,
            interpolation_delay_ms: 100,
        }
    }
}

/// Entity state snapshot at a point in time
#[derive(Debug, Clone)]
struct EntitySnapshot {
    timestamp: u64,
    position: glam::Vec3,
    rotation: glam::Quat,
    velocity: glam::Vec3,
}

/// Entity state history
struct EntityHistory {
    snapshots: VecDeque<EntitySnapshot>,
    max_snapshots: usize,
}

impl EntityHistory {
    fn new(max_snapshots: usize) -> Self {
        Self {
            snapshots: VecDeque::with_capacity(max_snapshots),
            max_snapshots,
        }
    }

    fn add_snapshot(&mut self, snapshot: EntitySnapshot) {
        self.snapshots.push_back(snapshot);

        // Remove old snapshots
        if self.snapshots.len() > self.max_snapshots {
            self.snapshots.pop_front();
        }
    }

    fn get_at_time(&self, timestamp: u64) -> Option<EntitySnapshot> {
        if self.snapshots.is_empty() {
            return None;
        }

        // Find snapshots before and after target time
        let mut before = None;
        let mut after = None;

        for snapshot in &self.snapshots {
            if snapshot.timestamp <= timestamp {
                before = Some(snapshot);
            } else if after.is_none() {
                after = Some(snapshot);
                break;
            }
        }

        // Interpolate between snapshots
        match (before, after) {
            (Some(b), Some(a)) => {
                let t = if a.timestamp > b.timestamp {
                    (timestamp - b.timestamp) as f32 / (a.timestamp - b.timestamp) as f32
                } else {
                    0.0
                };

                Some(EntitySnapshot {
                    timestamp,
                    position: b.position.lerp(a.position, t),
                    rotation: b.rotation.slerp(a.rotation, t),
                    velocity: b.velocity.lerp(a.velocity, t),
                })
            }
            (Some(b), None) => Some(b.clone()),
            (None, Some(a)) => Some(a.clone()),
            (None, None) => None,
        }
    }
}

impl LagCompensator {
    pub fn new(config: LagCompensationConfig) -> Self {
        Self {
            entity_histories: HashMap::new(),
            config,
        }
    }

    /// Record entity state
    pub fn record_state(
        &mut self,
        entity_id: u32,
        timestamp: u64,
        position: glam::Vec3,
        rotation: glam::Quat,
        velocity: glam::Vec3,
    ) {
        let snapshot = EntitySnapshot {
            timestamp,
            position,
            rotation,
            velocity,
        };

        let max_snapshots = (self.config.history_duration_ms / 16) as usize; // Assume 60fps

        self.entity_histories
            .entry(entity_id)
            .or_insert_with(|| EntityHistory::new(max_snapshots))
            .add_snapshot(snapshot);
    }

    /// Get interpolated state for rendering
    pub fn get_interpolated_state(
        &self,
        entity_id: u32,
        current_time: u64,
    ) -> Option<(glam::Vec3, glam::Quat)> {
        // Render at time - interpolation_delay for smoothness
        let render_time = current_time.saturating_sub(self.config.interpolation_delay_ms);

        let history = self.entity_histories.get(&entity_id)?;
        let snapshot = history.get_at_time(render_time)?;

        Some((snapshot.position, snapshot.rotation))
    }

    /// Rewind to specific time (for server-side hit detection)
    pub fn rewind_to_time(
        &self,
        timestamp: u64,
    ) -> HashMap<u32, (glam::Vec3, glam::Quat)> {
        let mut rewound_states = HashMap::new();

        for (entity_id, history) in &self.entity_histories {
            if let Some(snapshot) = history.get_at_time(timestamp) {
                rewound_states.insert(*entity_id, (snapshot.position, snapshot.rotation));
            }
        }

        rewound_states
    }

    /// Remove entity from tracking
    pub fn remove_entity(&mut self, entity_id: u32) {
        self.entity_histories.remove(&entity_id);
    }

    /// Clear all history
    pub fn clear(&mut self) {
        self.entity_histories.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lag_compensation() {
        let mut compensator = LagCompensator::new(LagCompensationConfig::default());

        // Record states at different times
        compensator.record_state(
            1,
            0,
            glam::Vec3::ZERO,
            glam::Quat::IDENTITY,
            glam::Vec3::ZERO,
        );

        compensator.record_state(
            1,
            100,
            glam::Vec3::new(10.0, 0.0, 0.0),
            glam::Quat::IDENTITY,
            glam::Vec3::ZERO,
        );

        // Get interpolated state at t=50 (midpoint)
        if let Some((pos, _)) = compensator.get_interpolated_state(1, 150) {
            // Should be interpolated between 0 and 100ms (accounting for delay)
            assert!((pos.x - 5.0).abs() < 1.0);
        } else {
            panic!("Expected interpolated state");
        }
    }

    #[test]
    fn test_rewind() {
        let mut compensator = LagCompensator::new(LagCompensationConfig::default());

        // Record entity moving
        for i in 0..10 {
            compensator.record_state(
                1,
                i * 16,
                glam::Vec3::new(i as f32, 0.0, 0.0),
                glam::Quat::IDENTITY,
                glam::Vec3::ZERO,
            );
        }

        // Rewind to t=80 (should be position ~5)
        let rewound = compensator.rewind_to_time(80);
        let (pos, _) = rewound.get(&1).unwrap();

        assert!((pos.x - 5.0).abs() < 1.0);
    }
}
```

---

### **4. Smooth Error Correction** (Day 3-4)

**File:** `engine/networking/src/prediction/error_correction.rs`

```rust
/// Smooth error correction to avoid visual snapping
pub struct ErrorCorrector {
    /// Current error offset
    error_offset: glam::Vec3,

    /// Error correction rate (units per second)
    correction_rate: f32,
}

impl ErrorCorrector {
    pub fn new(correction_rate: f32) -> Self {
        Self {
            error_offset: glam::Vec3::ZERO,
            correction_rate,
        }
    }

    /// Set error to correct
    pub fn set_error(&mut self, error: glam::Vec3) {
        self.error_offset = error;
        tracing::debug!("Error correction: {:.2} units", error.length());
    }

    /// Update correction (call each frame)
    pub fn update(&mut self, dt: f32) -> glam::Vec3 {
        if self.error_offset.length() < 0.001 {
            // Error corrected
            self.error_offset = glam::Vec3::ZERO;
            return glam::Vec3::ZERO;
        }

        // Calculate correction amount for this frame
        let correction_distance = self.correction_rate * dt;
        let error_length = self.error_offset.length();

        if correction_distance >= error_length {
            // Fully correct this frame
            let correction = self.error_offset;
            self.error_offset = glam::Vec3::ZERO;
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
    pub fn current_error(&self) -> glam::Vec3 {
        self.error_offset
    }

    /// Check if correction is complete
    pub fn is_corrected(&self) -> bool {
        self.error_offset.length() < 0.001
    }

    /// Reset correction
    pub fn reset(&mut self) {
        self.error_offset = glam::Vec3::ZERO;
    }
}

/// Adaptive error correction (faster for large errors)
pub struct AdaptiveErrorCorrector {
    /// Current error offset
    error_offset: glam::Vec3,

    /// Base correction rate
    base_rate: f32,

    /// Max correction rate
    max_rate: f32,
}

impl AdaptiveErrorCorrector {
    pub fn new(base_rate: f32, max_rate: f32) -> Self {
        Self {
            error_offset: glam::Vec3::ZERO,
            base_rate,
            max_rate,
        }
    }

    pub fn set_error(&mut self, error: glam::Vec3) {
        self.error_offset = error;
    }

    pub fn update(&mut self, dt: f32) -> glam::Vec3 {
        let error_length = self.error_offset.length();

        if error_length < 0.001 {
            self.error_offset = glam::Vec3::ZERO;
            return glam::Vec3::ZERO;
        }

        // Adaptive rate: faster for larger errors
        let rate = if error_length > 5.0 {
            // Large error: snap immediately
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
            self.error_offset = glam::Vec3::ZERO;
            correction
        } else {
            let correction_dir = self.error_offset.normalize();
            let correction = correction_dir * correction_distance;
            self.error_offset -= correction;
            correction
        }
    }

    pub fn current_error(&self) -> glam::Vec3 {
        self.error_offset
    }

    pub fn is_corrected(&self) -> bool {
        self.error_offset.length() < 0.001
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_correction() {
        let mut corrector = ErrorCorrector::new(10.0); // 10 units/sec

        // Set 5 unit error
        corrector.set_error(glam::Vec3::new(5.0, 0.0, 0.0));

        // Correct over 0.5 seconds (should correct fully)
        let correction = corrector.update(0.5);

        assert!(correction.length() > 4.9);
        assert!(corrector.is_corrected());
    }

    #[test]
    fn test_adaptive_correction() {
        let mut corrector = AdaptiveErrorCorrector::new(5.0, 20.0);

        // Large error (should snap)
        corrector.set_error(glam::Vec3::new(10.0, 0.0, 0.0));
        let correction = corrector.update(0.016); // One frame at 60fps

        assert!(correction.length() > 9.0); // Should correct most of it
    }
}
```

---

### **5. Integration** (Day 4-5)

**File:** `engine/networking/src/prediction/mod.rs`

```rust
pub mod input_buffer;
pub mod client_predictor;
pub mod lag_compensation;
pub mod error_correction;

pub use input_buffer::{InputBuffer, BufferedInput};
pub use client_predictor::{ClientPredictor, PredictionConfig};
pub use lag_compensation::{LagCompensator, LagCompensationConfig};
pub use error_correction::{ErrorCorrector, AdaptiveErrorCorrector};

/// Complete prediction system for client
pub struct PredictionSystem {
    /// Predictor for local player
    predictor: ClientPredictor,

    /// Lag compensation for remote entities
    lag_compensator: LagCompensator,

    /// Error corrector
    error_corrector: AdaptiveErrorCorrector,
}

impl PredictionSystem {
    pub fn new() -> Self {
        Self {
            predictor: ClientPredictor::new(PredictionConfig::default()),
            lag_compensator: LagCompensator::new(LagCompensationConfig::default()),
            error_corrector: AdaptiveErrorCorrector::new(5.0, 20.0),
        }
    }

    /// Process local input
    pub fn process_input(
        &mut self,
        timestamp: u64,
        movement: glam::Vec3,
        look_delta: glam::Vec3,
        buttons: u32,
        dt: f32,
    ) -> u32 {
        self.predictor.process_input(timestamp, movement, look_delta, buttons, dt)
    }

    /// Reconcile with server state
    pub fn reconcile_server_state(
        &mut self,
        sequence: u32,
        position: glam::Vec3,
        velocity: glam::Vec3,
        rotation: glam::Quat,
        timestamp: u64,
    ) {
        self.predictor.reconcile(sequence, position, velocity, rotation, timestamp);
    }

    /// Update remote entity
    pub fn update_remote_entity(
        &mut self,
        entity_id: u32,
        timestamp: u64,
        position: glam::Vec3,
        rotation: glam::Quat,
        velocity: glam::Vec3,
    ) {
        self.lag_compensator.record_state(entity_id, timestamp, position, rotation, velocity);
    }

    /// Get predicted local position
    pub fn local_position(&self) -> glam::Vec3 {
        self.predictor.predicted_position()
    }

    /// Get predicted local rotation
    pub fn local_rotation(&self) -> glam::Quat {
        self.predictor.predicted_rotation()
    }

    /// Get interpolated remote entity state
    pub fn remote_entity_state(
        &self,
        entity_id: u32,
        current_time: u64,
    ) -> Option<(glam::Vec3, glam::Quat)> {
        self.lag_compensator.get_interpolated_state(entity_id, current_time)
    }

    /// Update error correction
    pub fn update_error_correction(&mut self, dt: f32) -> glam::Vec3 {
        self.error_corrector.update(dt)
    }
}
```

---

## ✅ **Acceptance Criteria**

- [ ] Local player input feels instant (< 1ms overhead)
- [ ] Input buffer stores pending inputs
- [ ] Client prediction matches server within tolerance
- [ ] Server reconciliation replays inputs correctly
- [ ] Prediction errors corrected smoothly
- [ ] Remote entities interpolated smoothly
- [ ] Lag compensation rewinds state accurately
- [ ] No visible snapping on reconciliation
- [ ] Works with 100ms+ latency
- [ ] Prediction overhead < 1ms per frame

---

## 🎯 **Performance Targets**

| Operation | Target | Critical |
|-----------|--------|----------|
| Process input | < 0.5ms | < 1ms |
| Apply prediction | < 0.3ms | < 1ms |
| Reconciliation | < 2ms | < 5ms |
| Input replay (10 inputs) | < 1ms | < 3ms |
| Lag compensation update | < 0.5ms | < 1ms |
| Error correction update | < 0.1ms | < 0.5ms |
| Total prediction overhead | < 1ms | < 2ms |

**Responsiveness:**
- Input to visual feedback: < 16ms (1 frame)
- Server roundtrip: 50-200ms (acceptable with prediction)
- Error correction time: < 200ms for small errors

---

## 🧪 **Tests**

```rust
#[test]
fn test_input_buffering() {
    let mut predictor = ClientPredictor::new(PredictionConfig::default());

    // Process inputs
    let seq1 = predictor.process_input(
        0,
        glam::Vec3::X,
        glam::Vec3::ZERO,
        0,
        0.016,
    );

    let seq2 = predictor.process_input(
        16,
        glam::Vec3::Y,
        glam::Vec3::ZERO,
        0,
        0.016,
    );

    // Buffer should contain 2 inputs
    assert_eq!(predictor.input_buffer().len(), 2);

    // Acknowledge first input
    predictor.reconcile(
        seq1,
        glam::Vec3::ZERO,
        glam::Vec3::ZERO,
        glam::Quat::IDENTITY,
        16,
    );

    // Buffer should contain 1 input
    assert_eq!(predictor.input_buffer().len(), 1);
}

#[test]
fn test_prediction_accuracy() {
    let mut predictor = ClientPredictor::new(PredictionConfig::default());

    // Process forward movement
    for _ in 0..60 {
        predictor.process_input(
            0,
            glam::Vec3::new(0.0, 0.0, 1.0),
            glam::Vec3::ZERO,
            0,
            1.0 / 60.0,
        );
    }

    let predicted_pos = predictor.predicted_position();

    // Should have moved forward (speed 5.0 * 1 second = 5.0 units)
    assert!((predicted_pos.z - 5.0).abs() < 0.1);
}

#[test]
fn test_reconciliation() {
    let mut predictor = ClientPredictor::new(PredictionConfig::default());

    // Predict movement
    let seq = predictor.process_input(
        0,
        glam::Vec3::new(1.0, 0.0, 0.0),
        glam::Vec3::ZERO,
        0,
        1.0,
    );

    let predicted_before = predictor.predicted_position();

    // Server says we're slightly off
    let server_pos = predicted_before + glam::Vec3::new(0.0, 0.0, 0.5);

    predictor.reconcile(
        seq,
        server_pos,
        glam::Vec3::ZERO,
        glam::Quat::IDENTITY,
        0,
    );

    let predicted_after = predictor.predicted_position();

    // Should correct to match server
    assert!((predicted_after - server_pos).length() < 0.1);
}

#[test]
fn test_lag_compensation() {
    let mut compensator = LagCompensator::new(LagCompensationConfig::default());

    // Record entity moving
    for i in 0..10 {
        compensator.record_state(
            1,
            i * 100,
            glam::Vec3::new(i as f32 * 10.0, 0.0, 0.0),
            glam::Quat::IDENTITY,
            glam::Vec3::X * 10.0,
        );
    }

    // Get state at t=250 (should interpolate)
    if let Some((pos, _)) = compensator.get_interpolated_state(1, 350) {
        // 350 - 100 (delay) = 250ms = 2.5 position
        assert!((pos.x - 25.0).abs() < 5.0);
    } else {
        panic!("Expected interpolated state");
    }
}
```

---

## 💡 **How It Works**

### **Client-Side Prediction Flow:**

1. **Input**: Player presses W
2. **Immediate**: Client applies movement locally (prediction)
3. **Send**: Client sends input to server via UDP
4. **Server**: Server processes input authoritatively
5. **Response**: Server sends back confirmed position
6. **Reconcile**: Client checks if prediction matched
7. **Correct**: If mismatch, client smoothly corrects error

### **Lag Compensation Flow:**

1. **Server sends**: Remote player position updates
2. **Client records**: Stores position history
3. **Client renders**: Interpolates between past states
4. **Smooth motion**: Entities appear smooth despite packet loss

### **Benefits:**

- Local player feels instant (no latency)
- Remote players appear smooth (interpolation)
- Server is authoritative (prevents cheating)
- Works with high latency (100-200ms)

---

**Dependencies:** [phase2-network-protocol.md](phase2-network-protocol.md), [phase2-udp-packets.md](phase2-udp-packets.md), [phase2-state-sync.md](phase2-state-sync.md)
**Next:** [phase2-interest-basic.md](phase2-interest-basic.md)
