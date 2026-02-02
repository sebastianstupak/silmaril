//! Entity interpolation and extrapolation benchmarks
//!
//! Benchmarks for:
//! - Single entity interpolation (<0.5ms target)
//! - 100 entities interpolation (<5ms target)
//! - 1000 entities interpolation (<50ms target)
//! - Single entity extrapolation with velocity (<0.5ms)
//! - Dead reckoning accuracy measurement
//! - Jitter buffer operations (<100µs)

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use engine_networking::protocol::EntityState;
use std::collections::VecDeque;

// ============================================================================
// Interpolation Data Structures
// ============================================================================

/// Interpolation buffer for smooth entity movement
#[derive(Debug, Clone)]
struct InterpolationBuffer {
    states: VecDeque<TimestampedState>,
    buffer_duration_ms: u64,
}

/// Entity state with timestamp
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

        // Remove old states
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

        // Find two states to interpolate between
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
                    qx: slerp_quat(
                        (prev.state.qx, prev.state.qy, prev.state.qz, prev.state.qw),
                        (next.state.qx, next.state.qy, next.state.qz, next.state.qw),
                        t,
                    )
                    .0,
                    qy: slerp_quat(
                        (prev.state.qx, prev.state.qy, prev.state.qz, prev.state.qw),
                        (next.state.qx, next.state.qy, next.state.qz, next.state.qw),
                        t,
                    )
                    .1,
                    qz: slerp_quat(
                        (prev.state.qx, prev.state.qy, prev.state.qz, prev.state.qw),
                        (next.state.qx, next.state.qy, next.state.qz, next.state.qw),
                        t,
                    )
                    .2,
                    qw: slerp_quat(
                        (prev.state.qx, prev.state.qy, prev.state.qz, prev.state.qw),
                        (next.state.qx, next.state.qy, next.state.qz, next.state.qw),
                        t,
                    )
                    .3,
                    health: prev.state.health,
                    max_health: prev.state.max_health,
                })
            }
            (Some(prev), None) => Some(prev.state.clone()),
            _ => None,
        }
    }
}

// ============================================================================
// Extrapolation Data Structures
// ============================================================================

/// Entity state with velocity for extrapolation
#[derive(Debug, Clone)]
struct EntityStateWithVelocity {
    state: EntityState,
    velocity: (f32, f32, f32),
    #[allow(dead_code)]
    angular_velocity: (f32, f32, f32),
}

impl EntityStateWithVelocity {
    fn extrapolate(&self, dt: f32) -> EntityState {
        EntityState {
            entity: self.state.entity,
            x: self.state.x + self.velocity.0 * dt,
            y: self.state.y + self.velocity.1 * dt,
            z: self.state.z + self.velocity.2 * dt,
            // Simplified rotation extrapolation (should use proper quaternion integration)
            qx: self.state.qx,
            qy: self.state.qy,
            qz: self.state.qz,
            qw: self.state.qw,
            health: self.state.health,
            max_health: self.state.max_health,
        }
    }

    fn dead_reckoning(&self, dt: f32, drag: f32) -> EntityState {
        let drag_factor = (1.0 - drag * dt).max(0.0);
        EntityState {
            entity: self.state.entity,
            x: self.state.x + self.velocity.0 * dt * drag_factor,
            y: self.state.y + self.velocity.1 * dt * drag_factor,
            z: self.state.z + self.velocity.2 * dt * drag_factor,
            qx: self.state.qx,
            qy: self.state.qy,
            qz: self.state.qz,
            qw: self.state.qw,
            health: self.state.health,
            max_health: self.state.max_health,
        }
    }
}

// ============================================================================
// Jitter Buffer
// ============================================================================

/// Adaptive jitter buffer for packet reordering
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

        // Insert in timestamp order
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

    fn adaptive_adjust(&mut self, latency_ms: u64, jitter_ms: u64) {
        // Simple adaptive algorithm: target_delay = latency + 2*jitter
        self.target_delay_ms = latency_ms + 2 * jitter_ms;
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn slerp_quat(q1: (f32, f32, f32, f32), q2: (f32, f32, f32, f32), t: f32) -> (f32, f32, f32, f32) {
    // Simplified slerp (should use proper quaternion math)
    let (x1, y1, z1, w1) = q1;
    let (x2, y2, z2, w2) = q2;

    let dot = x1 * x2 + y1 * y2 + z1 * z2 + w1 * w2;
    let dot = dot.clamp(-1.0, 1.0);

    if (1.0 - dot.abs()) < 0.001 {
        // Quaternions very close, use linear interpolation
        return (lerp(x1, x2, t), lerp(y1, y2, t), lerp(z1, z2, t), lerp(w1, w2, t));
    }

    let theta = dot.acos();
    let sin_theta = theta.sin();

    let a = ((1.0 - t) * theta).sin() / sin_theta;
    let b = (t * theta).sin() / sin_theta;

    (a * x1 + b * x2, a * y1 + b * y2, a * z1 + b * z2, a * w1 + b * w2)
}

fn create_test_state(index: usize) -> EntityState {
    use engine_core::ecs::Entity;

    EntityState {
        entity: Entity::new(index as u32, 0),
        x: (index as f32) * 10.0,
        y: (index as f32) * 5.0,
        z: (index as f32) * 2.0,
        qx: 0.0,
        qy: 0.0,
        qz: 0.0,
        qw: 1.0,
        health: Some(100.0),
        max_health: Some(100.0),
    }
}

fn create_test_state_with_velocity(index: usize) -> EntityStateWithVelocity {
    EntityStateWithVelocity {
        state: create_test_state(index),
        velocity: (1.0, 0.5, 0.2),
        angular_velocity: (0.1, 0.05, 0.02),
    }
}

// ============================================================================
// Benchmarks
// ============================================================================

fn bench_single_entity_interpolation(c: &mut Criterion) {
    c.bench_function("interpolation/single_entity", |b| {
        let mut buffer = InterpolationBuffer::new(100);

        // Populate buffer with states
        for i in 0..10 {
            let state = create_test_state(0);
            buffer.insert(state, i * 10);
        }

        b.iter(|| {
            let result = buffer.interpolate(black_box(45));
            black_box(result);
        });
    });
}

fn bench_batch_interpolation(c: &mut Criterion) {
    let mut group = c.benchmark_group("interpolation/batch");

    for entity_count in [100, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(entity_count),
            entity_count,
            |b, &count| {
                let mut buffers: Vec<InterpolationBuffer> =
                    (0..count).map(|_| InterpolationBuffer::new(100)).collect();

                // Populate all buffers
                for buffer in buffers.iter_mut() {
                    for i in 0..10 {
                        let state = create_test_state(0);
                        buffer.insert(state, i * 10);
                    }
                }

                b.iter(|| {
                    for buffer in buffers.iter() {
                        let result = buffer.interpolate(black_box(45));
                        black_box(result);
                    }
                });
            },
        );
    }

    group.finish();
}

fn bench_buffer_management(c: &mut Criterion) {
    let mut group = c.benchmark_group("interpolation/buffer_management");

    group.bench_function("insert", |b| {
        let mut buffer = InterpolationBuffer::new(100);
        let state = create_test_state(0);
        let mut timestamp = 0u64;

        b.iter(|| {
            buffer.insert(black_box(state.clone()), timestamp);
            timestamp += 10;
        });
    });

    group.bench_function("insert_with_cleanup", |b| {
        let state = create_test_state(0);
        let mut timestamp = 0u64;

        b.iter(|| {
            let mut buffer = InterpolationBuffer::new(100);
            // Insert many states to trigger cleanup
            for i in 0..20 {
                buffer.insert(state.clone(), timestamp + i * 10);
            }
            timestamp += 200;
            black_box(buffer);
        });
    });

    group.finish();
}

fn bench_single_entity_extrapolation(c: &mut Criterion) {
    c.bench_function("extrapolation/single_entity", |b| {
        let state = create_test_state_with_velocity(0);

        b.iter(|| {
            let result = state.extrapolate(black_box(0.016)); // 16ms
            black_box(result);
        });
    });
}

fn bench_dead_reckoning(c: &mut Criterion) {
    let mut group = c.benchmark_group("extrapolation/dead_reckoning");

    group.bench_function("single_entity", |b| {
        let state = create_test_state_with_velocity(0);

        b.iter(|| {
            let result = state.dead_reckoning(black_box(0.016), black_box(0.1));
            black_box(result);
        });
    });

    group.bench_function("batch_100", |b| {
        let states: Vec<EntityStateWithVelocity> =
            (0..100).map(create_test_state_with_velocity).collect();

        b.iter(|| {
            for state in states.iter() {
                let result = state.dead_reckoning(black_box(0.016), black_box(0.1));
                black_box(result);
            }
        });
    });

    group.bench_function("batch_1000", |b| {
        let states: Vec<EntityStateWithVelocity> =
            (0..1000).map(create_test_state_with_velocity).collect();

        b.iter(|| {
            for state in states.iter() {
                let result = state.dead_reckoning(black_box(0.016), black_box(0.1));
                black_box(result);
            }
        });
    });

    group.finish();
}

fn bench_extrapolation_error_accumulation(c: &mut Criterion) {
    c.bench_function("extrapolation/error_accumulation", |b| {
        let initial_state = create_test_state_with_velocity(0);

        b.iter(|| {
            let mut state = initial_state.clone();
            let mut accumulated_error = 0.0f32;

            // Simulate 1 second of extrapolation at 60Hz
            for _ in 0..60 {
                let extrapolated = state.extrapolate(0.016);
                // Calculate error (simplified)
                let error = ((extrapolated.x - state.state.x).powi(2)
                    + (extrapolated.y - state.state.y).powi(2)
                    + (extrapolated.z - state.state.z).powi(2))
                .sqrt();
                accumulated_error += error;
                state.state = extrapolated;
            }

            black_box(accumulated_error);
        });
    });
}

fn bench_jitter_buffer_insertion(c: &mut Criterion) {
    c.bench_function("jitter_buffer/insert", |b| {
        let mut buffer = JitterBuffer::new(100, 50);
        let data = vec![0u8; 100];
        let mut timestamp = 0u64;

        b.iter(|| {
            let _ = buffer.insert(timestamp, black_box(data.clone()));
            timestamp += 10;
            if timestamp > 1000 {
                timestamp = 0;
                buffer = JitterBuffer::new(100, 50);
            }
        });
    });
}

fn bench_jitter_buffer_retrieval(c: &mut Criterion) {
    c.bench_function("jitter_buffer/retrieve", |b| {
        let data = vec![0u8; 100];

        b.iter(|| {
            let mut buffer = JitterBuffer::new(100, 50);

            // Insert packets
            for i in 0..10 {
                let _ = buffer.insert(i * 10, data.clone());
            }

            // Retrieve packets
            let mut current_time = 100u64;
            while let Some(retrieved) = buffer.retrieve(current_time) {
                black_box(retrieved);
                current_time += 10;
            }
        });
    });
}

fn bench_jitter_buffer_out_of_order(c: &mut Criterion) {
    c.bench_function("jitter_buffer/out_of_order", |b| {
        let data = vec![0u8; 100];

        b.iter(|| {
            let mut buffer = JitterBuffer::new(100, 50);

            // Insert packets out of order
            let timestamps = vec![40, 10, 60, 20, 50, 30, 0, 70, 80, 90];
            for ts in timestamps {
                let _ = buffer.insert(ts, data.clone());
            }

            black_box(buffer);
        });
    });
}

fn bench_adaptive_jitter_buffer(c: &mut Criterion) {
    c.bench_function("jitter_buffer/adaptive_sizing", |b| {
        let mut buffer = JitterBuffer::new(100, 50);

        b.iter(|| {
            // Measure latency and jitter
            let latency = black_box(30);
            let jitter = black_box(10);

            buffer.adaptive_adjust(latency, jitter);
            black_box(buffer.target_delay_ms);
        });
    });
}

fn bench_interpolation_vs_extrapolation(c: &mut Criterion) {
    let mut group = c.benchmark_group("comparison/interpolation_vs_extrapolation");

    group.bench_function("interpolation", |b| {
        let mut buffer = InterpolationBuffer::new(100);
        for i in 0..10 {
            buffer.insert(create_test_state(0), i * 10);
        }

        b.iter(|| {
            let result = buffer.interpolate(black_box(45));
            black_box(result);
        });
    });

    group.bench_function("extrapolation", |b| {
        let state = create_test_state_with_velocity(0);

        b.iter(|| {
            let result = state.extrapolate(black_box(0.016));
            black_box(result);
        });
    });

    group.finish();
}

criterion_group!(
    interpolation_benches,
    bench_single_entity_interpolation,
    bench_batch_interpolation,
    bench_buffer_management,
    bench_single_entity_extrapolation,
    bench_dead_reckoning,
    bench_extrapolation_error_accumulation,
    bench_jitter_buffer_insertion,
    bench_jitter_buffer_retrieval,
    bench_jitter_buffer_out_of_order,
    bench_adaptive_jitter_buffer,
    bench_interpolation_vs_extrapolation,
);

criterion_main!(interpolation_benches);
