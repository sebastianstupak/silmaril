//! Integration tests for entity interpolation and extrapolation
//!
//! Tests cover:
//! - Interpolation correctness
//! - Extrapolation accuracy
//! - Buffer management
//! - Jitter buffer operations
//! - Edge cases

use engine_core::ecs::Entity;
use engine_networking::protocol::EntityState;
use std::collections::VecDeque;

// ============================================================================
// Test Data Structures (duplicated from bench for testing)
// ============================================================================

#[derive(Debug, Clone)]
struct InterpolationBuffer {
    states: VecDeque<TimestampedState>,
    buffer_duration_ms: u64,
}

#[derive(Debug, Clone)]
struct TimestampedState {
    state: EntityState,
    timestamp: u64,
}

impl InterpolationBuffer {
    fn new(buffer_duration_ms: u64) -> Self {
        Self { states: VecDeque::new(), buffer_duration_ms }
    }

    fn insert(&mut self, state: EntityState, timestamp: u64) {
        self.states.push_back(TimestampedState { state, timestamp });

        let cutoff_time = timestamp.saturating_sub(self.buffer_duration_ms);
        while let Some(front) = self.states.front() {
            if front.timestamp < cutoff_time {
                self.states.pop_front();
            } else {
                break;
            }
        }
    }

    fn interpolate(&self, render_time: u64) -> Option<EntityState> {
        if self.states.len() < 2 {
            return self.states.back().map(|s| s.state.clone());
        }

        let mut prev_state = None;
        let mut next_state = None;

        for state in self.states.iter() {
            if state.timestamp <= render_time {
                prev_state = Some(state);
            } else {
                next_state = Some(state);
                break;
            }
        }

        match (prev_state, next_state) {
            (Some(prev), Some(next)) => {
                let t = (render_time - prev.timestamp) as f32
                    / (next.timestamp - prev.timestamp) as f32;
                let t = t.clamp(0.0, 1.0);

                Some(EntityState {
                    entity: prev.state.entity,
                    x: lerp(prev.state.x, next.state.x, t),
                    y: lerp(prev.state.y, next.state.y, t),
                    z: lerp(prev.state.z, next.state.z, t),
                    qx: prev.state.qx,
                    qy: prev.state.qy,
                    qz: prev.state.qz,
                    qw: prev.state.qw,
                    health: prev.state.health,
                    max_health: prev.state.max_health,
                })
            }
            (Some(prev), None) => Some(prev.state.clone()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
struct EntityStateWithVelocity {
    state: EntityState,
    velocity: (f32, f32, f32),
}

impl EntityStateWithVelocity {
    fn extrapolate(&self, dt: f32) -> EntityState {
        EntityState {
            entity: self.state.entity,
            x: self.state.x + self.velocity.0 * dt,
            y: self.state.y + self.velocity.1 * dt,
            z: self.state.z + self.velocity.2 * dt,
            qx: self.state.qx,
            qy: self.state.qy,
            qz: self.state.qz,
            qw: self.state.qw,
            health: self.state.health,
            max_health: self.state.max_health,
        }
    }
}

#[derive(Debug)]
struct JitterBuffer {
    buffer: VecDeque<(u64, Vec<u8>)>,
    max_size: usize,
    target_delay_ms: u64,
}

impl JitterBuffer {
    fn new(max_size: usize, target_delay_ms: u64) -> Self {
        Self { buffer: VecDeque::with_capacity(max_size), max_size, target_delay_ms }
    }

    fn insert(&mut self, timestamp: u64, data: Vec<u8>) -> Result<(), &'static str> {
        if self.buffer.len() >= self.max_size {
            return Err("Buffer full");
        }

        let pos = self
            .buffer
            .iter()
            .position(|(t, _)| *t > timestamp)
            .unwrap_or(self.buffer.len());

        self.buffer.insert(pos, (timestamp, data));
        Ok(())
    }

    fn retrieve(&mut self, current_time: u64) -> Option<Vec<u8>> {
        if self.buffer.is_empty() {
            return None;
        }

        let playback_time = current_time.saturating_sub(self.target_delay_ms);

        if let Some((timestamp, _)) = self.buffer.front() {
            if *timestamp <= playback_time {
                return self.buffer.pop_front().map(|(_, data)| data);
            }
        }

        None
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn create_test_state(x: f32, y: f32, z: f32) -> EntityState {
    EntityState {
        entity: Entity::new(1, 0),
        x,
        y,
        z,
        qx: 0.0,
        qy: 0.0,
        qz: 0.0,
        qw: 1.0,
        health: Some(100.0),
        max_health: Some(100.0),
    }
}

// ============================================================================
// Interpolation Tests
// ============================================================================

#[test]
fn test_interpolation_exact_timestamp() {
    let mut buffer = InterpolationBuffer::new(1000);

    buffer.insert(create_test_state(0.0, 0.0, 0.0), 0);
    buffer.insert(create_test_state(10.0, 10.0, 10.0), 100);

    // Query at exact timestamp
    let result = buffer.interpolate(0).unwrap();
    assert_eq!(result.x, 0.0);
    assert_eq!(result.y, 0.0);
    assert_eq!(result.z, 0.0);

    let result = buffer.interpolate(100).unwrap();
    assert_eq!(result.x, 10.0);
    assert_eq!(result.y, 10.0);
    assert_eq!(result.z, 10.0);
}

#[test]
fn test_interpolation_midpoint() {
    let mut buffer = InterpolationBuffer::new(1000);

    buffer.insert(create_test_state(0.0, 0.0, 0.0), 0);
    buffer.insert(create_test_state(10.0, 10.0, 10.0), 100);

    // Query at midpoint
    let result = buffer.interpolate(50).unwrap();
    assert!((result.x - 5.0).abs() < 0.001);
    assert!((result.y - 5.0).abs() < 0.001);
    assert!((result.z - 5.0).abs() < 0.001);
}

#[test]
fn test_interpolation_quarter_point() {
    let mut buffer = InterpolationBuffer::new(1000);

    buffer.insert(create_test_state(0.0, 0.0, 0.0), 0);
    buffer.insert(create_test_state(10.0, 10.0, 10.0), 100);

    // Query at 25%
    let result = buffer.interpolate(25).unwrap();
    assert!((result.x - 2.5).abs() < 0.001);
    assert!((result.y - 2.5).abs() < 0.001);
    assert!((result.z - 2.5).abs() < 0.001);
}

#[test]
fn test_interpolation_empty_buffer() {
    let buffer = InterpolationBuffer::new(1000);
    let result = buffer.interpolate(50);
    assert!(result.is_none());
}

#[test]
fn test_interpolation_single_state() {
    let mut buffer = InterpolationBuffer::new(1000);
    buffer.insert(create_test_state(5.0, 5.0, 5.0), 50);

    let result = buffer.interpolate(100).unwrap();
    assert_eq!(result.x, 5.0);
    assert_eq!(result.y, 5.0);
    assert_eq!(result.z, 5.0);
}

#[test]
fn test_interpolation_buffer_cleanup() {
    let mut buffer = InterpolationBuffer::new(100);

    // Insert states over time
    for i in 0..20 {
        buffer.insert(create_test_state(i as f32, 0.0, 0.0), i * 10);
    }

    // Buffer should only keep recent states
    assert!(buffer.states.len() <= 11); // 100ms buffer / 10ms per state + 1
}

#[test]
fn test_interpolation_future_query() {
    let mut buffer = InterpolationBuffer::new(1000);

    buffer.insert(create_test_state(0.0, 0.0, 0.0), 0);
    buffer.insert(create_test_state(10.0, 10.0, 10.0), 100);

    // Query beyond last timestamp - should return last state
    let result = buffer.interpolate(200).unwrap();
    assert_eq!(result.x, 10.0);
}

// ============================================================================
// Extrapolation Tests
// ============================================================================

#[test]
fn test_extrapolation_linear() {
    let state = EntityStateWithVelocity {
        state: create_test_state(0.0, 0.0, 0.0),
        velocity: (10.0, 5.0, 2.0),
    };

    let extrapolated = state.extrapolate(1.0); // 1 second

    assert!((extrapolated.x - 10.0).abs() < 0.001);
    assert!((extrapolated.y - 5.0).abs() < 0.001);
    assert!((extrapolated.z - 2.0).abs() < 0.001);
}

#[test]
fn test_extrapolation_zero_velocity() {
    let state = EntityStateWithVelocity {
        state: create_test_state(5.0, 5.0, 5.0),
        velocity: (0.0, 0.0, 0.0),
    };

    let extrapolated = state.extrapolate(1.0);

    assert_eq!(extrapolated.x, 5.0);
    assert_eq!(extrapolated.y, 5.0);
    assert_eq!(extrapolated.z, 5.0);
}

#[test]
fn test_extrapolation_negative_velocity() {
    let state = EntityStateWithVelocity {
        state: create_test_state(10.0, 10.0, 10.0),
        velocity: (-5.0, -2.0, -1.0),
    };

    let extrapolated = state.extrapolate(1.0);

    assert!((extrapolated.x - 5.0).abs() < 0.001);
    assert!((extrapolated.y - 8.0).abs() < 0.001);
    assert!((extrapolated.z - 9.0).abs() < 0.001);
}

#[test]
fn test_extrapolation_small_timestep() {
    let state = EntityStateWithVelocity {
        state: create_test_state(0.0, 0.0, 0.0),
        velocity: (100.0, 50.0, 25.0),
    };

    let extrapolated = state.extrapolate(0.016); // 16ms

    assert!((extrapolated.x - 1.6).abs() < 0.001);
    assert!((extrapolated.y - 0.8).abs() < 0.001);
    assert!((extrapolated.z - 0.4).abs() < 0.001);
}

// ============================================================================
// Jitter Buffer Tests
// ============================================================================

#[test]
fn test_jitter_buffer_insert_in_order() {
    let mut buffer = JitterBuffer::new(10, 50);

    for i in 0..5 {
        let result = buffer.insert(i * 10, vec![i as u8; 10]);
        assert!(result.is_ok());
    }

    assert_eq!(buffer.buffer.len(), 5);
}

#[test]
fn test_jitter_buffer_insert_out_of_order() {
    let mut buffer = JitterBuffer::new(10, 50);

    // Insert out of order
    buffer.insert(20, vec![2; 10]).unwrap();
    buffer.insert(0, vec![0; 10]).unwrap();
    buffer.insert(10, vec![1; 10]).unwrap();

    // Should be sorted by timestamp
    assert_eq!(buffer.buffer[0].0, 0);
    assert_eq!(buffer.buffer[1].0, 10);
    assert_eq!(buffer.buffer[2].0, 20);
}

#[test]
fn test_jitter_buffer_full() {
    let mut buffer = JitterBuffer::new(3, 50);

    buffer.insert(0, vec![0; 10]).unwrap();
    buffer.insert(10, vec![1; 10]).unwrap();
    buffer.insert(20, vec![2; 10]).unwrap();

    // Buffer full
    let result = buffer.insert(30, vec![3; 10]);
    assert!(result.is_err());
}

#[test]
fn test_jitter_buffer_retrieve_ready() {
    let mut buffer = JitterBuffer::new(10, 50);

    buffer.insert(0, vec![0; 10]).unwrap();
    buffer.insert(10, vec![1; 10]).unwrap();
    buffer.insert(20, vec![2; 10]).unwrap();

    // Retrieve with sufficient delay
    let data = buffer.retrieve(60);
    assert!(data.is_some());
    assert_eq!(data.unwrap()[0], 0);
}

#[test]
fn test_jitter_buffer_retrieve_not_ready() {
    let mut buffer = JitterBuffer::new(10, 50);

    buffer.insert(100, vec![0; 10]).unwrap();

    // Retrieve too early (need to wait until time 150 = 100 + 50)
    let data = buffer.retrieve(140);
    assert!(data.is_none());
}

#[test]
fn test_jitter_buffer_retrieve_sequential() {
    let mut buffer = JitterBuffer::new(10, 50);

    for i in 0..5 {
        buffer.insert(i * 10, vec![i as u8; 10]).unwrap();
    }

    // Retrieve all in order
    for i in 0..5 {
        let current_time = (i * 10 + 50) as u64;
        let data = buffer.retrieve(current_time);
        assert!(data.is_some());
        assert_eq!(data.unwrap()[0], i as u8);
    }

    assert!(buffer.buffer.is_empty());
}

#[test]
fn test_jitter_buffer_duplicate_timestamp() {
    let mut buffer = JitterBuffer::new(10, 50);

    buffer.insert(10, vec![1; 10]).unwrap();
    buffer.insert(10, vec![2; 10]).unwrap(); // Same timestamp

    assert_eq!(buffer.buffer.len(), 2); // Both stored
}
