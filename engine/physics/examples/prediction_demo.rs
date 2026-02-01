//! Client-side prediction demonstration
//!
//! This example demonstrates:
//! 1. Input buffering for networked gameplay
//! 2. Local prediction (client simulates without waiting for server)
//! 3. State reconciliation (client corrects when server disagrees)
//! 4. Error smoothing (smooth visual correction)
//!
//! # Scenario
//!
//! Simulates a networked player with 100ms latency:
//! - Client sends inputs to "server" (simulated)
//! - Client predicts position locally
//! - Server sends back authoritative state (delayed)
//! - Client reconciles and smooths errors
//!
//! # Output
//!
//! Shows prediction accuracy, reconciliation count, and performance metrics.

use engine_core::ecs::{EntityAllocator, World};
use engine_math::{Quat, Transform, Vec3};
use engine_physics::{
    prediction::{PlayerInput, PredictionSystem},
    Collider, PhysicsConfig, PhysicsWorld, RigidBody,
};
use std::collections::VecDeque;
use std::time::Instant;

/// Simulated network packet
#[derive(Clone)]
struct NetworkPacket {
    /// Sequence number
    sequence: u32,
    /// Server position
    position: Vec3,
    /// Server rotation
    rotation: Quat,
    /// Server velocity
    velocity: Vec3,
}

/// Simulated server
struct SimulatedServer {
    physics: PhysicsWorld,
    physics_id: u64,
    /// Input queue (simulates network delay)
    input_queue: VecDeque<(PlayerInput, Instant)>,
    /// State update queue (simulates network delay)
    state_queue: VecDeque<(NetworkPacket, Instant)>,
    /// Network latency (one-way, in milliseconds)
    latency_ms: u64,
}

impl SimulatedServer {
    fn new(latency_ms: u64) -> Self {
        let config = PhysicsConfig::default();
        let mut physics = PhysicsWorld::new(config);

        // Create ground
        let ground_id = 0;
        physics.add_rigidbody(
            ground_id,
            &RigidBody::static_body(),
            Vec3::new(0.0, -1.0, 0.0),
            Quat::IDENTITY,
        );
        physics.add_collider(ground_id, &Collider::box_collider(Vec3::new(100.0, 0.5, 100.0)));

        // Create player
        let physics_id = 1;
        physics.add_rigidbody(
            physics_id,
            &RigidBody::dynamic(1.0),
            Vec3::new(0.0, 5.0, 0.0),
            Quat::IDENTITY,
        );
        physics.add_collider(physics_id, &Collider::capsule(0.5, 0.3));

        Self {
            physics,
            physics_id,
            input_queue: VecDeque::new(),
            state_queue: VecDeque::new(),
            latency_ms,
        }
    }

    /// Send input to server (simulates network send)
    fn send_input(&mut self, input: PlayerInput) {
        let arrival_time = Instant::now() + std::time::Duration::from_millis(self.latency_ms);
        self.input_queue.push_back((input, arrival_time));
    }

    /// Process inputs and send state updates
    fn update(&mut self) {
        let now = Instant::now();

        // Process arrived inputs
        while let Some((_input, arrival_time)) = self.input_queue.front() {
            if *arrival_time <= now {
                let input = self.input_queue.pop_front().unwrap().0;

                // Apply input to server physics
                if input.movement.length_squared() > 0.001 {
                    let force = input.movement.normalize() * 50.0;
                    self.physics.apply_force(self.physics_id, force);
                }

                if input.jump {
                    if let Some((linvel, _)) = self.physics.get_velocity(self.physics_id) {
                        if linvel.y.abs() < 0.1 {
                            self.physics.apply_impulse(self.physics_id, Vec3::new(0.0, 5.0, 0.0));
                        }
                    }
                }

                // Step physics
                self.physics.step(input.delta_time);

                // Send state update
                if let Some((pos, rot)) = self.physics.get_transform(self.physics_id) {
                    if let Some((vel, _)) = self.physics.get_velocity(self.physics_id) {
                        let packet =
                            NetworkPacket { sequence: input.sequence, position: pos, rotation: rot, velocity: vel };

                        let send_time =
                            Instant::now() + std::time::Duration::from_millis(self.latency_ms);
                        self.state_queue.push_back((packet, send_time));
                    }
                }
            } else {
                break;
            }
        }
    }

    /// Receive state updates from server
    fn receive_state(&mut self) -> Option<NetworkPacket> {
        let now = Instant::now();

        if let Some((_packet, arrival_time)) = self.state_queue.front() {
            if *arrival_time <= now {
                return Some(self.state_queue.pop_front().unwrap().0);
            }
        }

        None
    }
}

fn main() {
    println!("=== Client-Side Prediction Demo ===\n");

    // Setup
    let mut allocator = EntityAllocator::new();
    let entity = allocator.allocate();

    let mut world = World::new();
    world.register::<Transform>();
    world.add(entity, Transform::from_position(Vec3::new(0.0, 5.0, 0.0)));

    // Client physics
    let config = PhysicsConfig::default();
    let mut client_physics = PhysicsWorld::new(config);

    // Create ground (client-side)
    let ground_id = 0;
    client_physics.add_rigidbody(
        ground_id,
        &RigidBody::static_body(),
        Vec3::new(0.0, -1.0, 0.0),
        Quat::IDENTITY,
    );
    client_physics.add_collider(ground_id, &Collider::box_collider(Vec3::new(100.0, 0.5, 100.0)));

    // Create player (client-side)
    let physics_id = 1;
    client_physics.add_rigidbody(
        physics_id,
        &RigidBody::dynamic(1.0),
        Vec3::new(0.0, 5.0, 0.0),
        Quat::IDENTITY,
    );
    client_physics.add_collider(physics_id, &Collider::capsule(0.5, 0.3));

    // Start prediction
    let mut prediction = PredictionSystem::new();
    prediction.start_prediction(
        entity,
        physics_id,
        Vec3::new(0.0, 5.0, 0.0),
        Quat::IDENTITY,
        Vec3::ZERO,
    );

    // Simulated server (100ms latency = 200ms round-trip)
    let mut server = SimulatedServer::new(100);

    println!("Network latency: 100ms one-way (200ms RTT)");
    println!("Simulating 3 seconds of gameplay...\n");

    // Statistics
    let mut frame_count = 0;
    let mut reconciliation_count = 0;
    let mut total_prediction_error = 0.0;
    let mut max_prediction_error: f32 = 0.0;

    let start_time = Instant::now();
    let mut current_time = 0.0;
    let dt = 1.0 / 60.0; // 60 FPS

    // Simulate 3 seconds
    while current_time < 3.0 {
        frame_count += 1;

        // Generate input (move forward)
        let movement = Vec3::new(1.0, 0.0, 0.0);
        let jump = frame_count == 30; // Jump at frame 30

        // Add input and predict locally
        let timestamp = (current_time * 1000.0) as u64;
        prediction.add_input_and_predict(timestamp, movement, jump, dt, &mut client_physics);
        client_physics.step(dt);

        // Send input to server
        if let Some(state) = prediction.predicted_state() {
            let input = PlayerInput::new(
                prediction.current_sequence() - 1,
                timestamp,
                movement,
                jump,
                dt,
            );
            server.send_input(input);
        }

        // Update server
        server.update();

        // Receive state from server
        if let Some(packet) = server.receive_state() {
            // Calculate prediction error before reconciliation
            if let Some(state) = prediction.predicted_state() {
                let error = (state.predicted_position - packet.position).length();
                total_prediction_error += error;
                max_prediction_error = max_prediction_error.max(error);

                if error > 0.01 {
                    reconciliation_count += 1;
                }
            }

            // Reconcile
            prediction.reconcile(
                packet.sequence,
                packet.position,
                packet.rotation,
                packet.velocity,
                &mut client_physics,
            );
        }

        // Apply error smoothing
        prediction.apply_error_smoothing(&mut world, dt);

        // Print status every second
        if frame_count % 60 == 0 {
            if let Some(state) = prediction.predicted_state() {
                println!("Frame {}: Position = {:?}", frame_count, state.predicted_position);
                println!("  Buffered inputs: {}", prediction.buffered_input_count());
                if let Some((server_pos, _)) = server.physics.get_transform(server.physics_id) {
                    let error = (state.predicted_position - server_pos).length();
                    println!("  Current error: {:.4}m", error);
                }
            }
        }

        current_time += dt;
    }

    let elapsed = start_time.elapsed();

    // Print results
    println!("\n=== Results ===");
    println!("Frames simulated: {}", frame_count);
    println!("Reconciliations: {}", reconciliation_count);
    println!(
        "Average prediction error: {:.4}m",
        total_prediction_error / frame_count as f32
    );
    println!("Max prediction error: {:.4}m", max_prediction_error);
    println!("Simulation time: {:.2}ms", elapsed.as_secs_f64() * 1000.0);
    println!(
        "Average frame time: {:.2}µs",
        (elapsed.as_secs_f64() * 1_000_000.0) / frame_count as f64
    );

    // Final position comparison
    if let Some(state) = prediction.predicted_state() {
        if let Some((server_pos, _)) = server.physics.get_transform(server.physics_id) {
            println!("\n=== Final State ===");
            println!("Client position: {:?}", state.predicted_position);
            println!("Server position: {:?}", server_pos);
            println!("Final error: {:.4}m", (state.predicted_position - server_pos).length());
        }
    }

    // Performance analysis
    println!("\n=== Performance Analysis ===");
    let reconciliation_rate = (reconciliation_count as f32 / frame_count as f32) * 100.0;
    println!("Reconciliation rate: {:.1}%", reconciliation_rate);

    if reconciliation_rate < 10.0 {
        println!("✓ Excellent prediction accuracy!");
    } else if reconciliation_rate < 30.0 {
        println!("✓ Good prediction accuracy");
    } else {
        println!("⚠ High reconciliation rate (may indicate issues)");
    }

    if max_prediction_error < 0.5 {
        println!("✓ Low maximum error (smooth gameplay)");
    } else {
        println!("⚠ High maximum error (may cause visual artifacts)");
    }

    println!("\n=== Demo Complete ===");
    println!("\nKey takeaways:");
    println!("1. Client predicts locally without waiting for server");
    println!("2. Server confirms state with 200ms delay (100ms each way)");
    println!("3. Client reconciles when server state arrives");
    println!("4. Error smoothing prevents visual 'pops'");
    println!("\nThis allows responsive gameplay even with high latency!");
}
