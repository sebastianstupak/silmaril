//! Network Integration Benchmarks
//!
//! Comprehensive end-to-end benchmarks testing:
//! - Full client-server game loop
//! - End-to-end latency (input → server → client)
//! - Bandwidth usage per client
//! - Multiple concurrent clients
//! - State sync frequencies
//! - Resilience under packet loss
//! - Realistic game scenarios (MMORPG, FPS, Battle Royale)

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_core::{Entity, Transform, Vec3, Velocity, World};
use engine_networking::{NetworkProfile, NetworkSimulator};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// Mock client message for benchmarking
#[derive(Debug, Clone, Serialize, Deserialize)]
enum MockClientMessage {
    Input { sequence: u32, movement: Vec3 },
    Ping { timestamp: u64 },
}

/// Mock server message for benchmarking
#[derive(Debug, Clone, Serialize, Deserialize)]
enum MockServerMessage {
    StateUpdate { tick: u32, entity_count: usize },
    Pong { timestamp: u64 },
}

/// Game scenario configuration
#[derive(Debug, Clone)]
struct GameScenario {
    name: &'static str,
    player_count: usize,
    entity_count: usize,
    update_radius: f32,
    movement_speed: f32,
    update_frequency_hz: u32,
}

impl GameScenario {
    /// MMORPG: Many players, mostly static, sparse updates
    fn mmorpg() -> Self {
        Self {
            name: "MMORPG",
            player_count: 100,
            entity_count: 1000,
            update_radius: 50.0,
            movement_speed: 5.0,
            update_frequency_hz: 30,
        }
    }

    /// FPS: Few players, high movement, frequent updates
    fn fps() -> Self {
        Self {
            name: "FPS",
            player_count: 16,
            entity_count: 100,
            update_radius: 100.0,
            movement_speed: 20.0,
            update_frequency_hz: 60,
        }
    }

    /// Battle Royale: Many players, distributed, varying density
    fn battle_royale() -> Self {
        Self {
            name: "BattleRoyale",
            player_count: 100,
            entity_count: 500,
            update_radius: 200.0,
            movement_speed: 10.0,
            update_frequency_hz: 60,
        }
    }
}

/// Simulated client
#[allow(dead_code)]
struct SimulatedClient {
    id: usize,
    entity: Entity,
    position: Vec3,
    input_sequence: u32,
    last_ack: u32,
    simulator: NetworkSimulator,
    latency_samples: Vec<Duration>,
    bytes_sent: usize,
    bytes_received: usize,
}

impl SimulatedClient {
    fn new(id: usize, entity: Entity, position: Vec3, profile: NetworkProfile) -> Self {
        Self {
            id,
            entity,
            position,
            input_sequence: 0,
            last_ack: 0,
            simulator: NetworkSimulator::new(profile),
            latency_samples: Vec::new(),
            bytes_sent: 0,
            bytes_received: 0,
        }
    }

    fn send_input(&mut self, input: Vec3) -> MockClientMessage {
        self.input_sequence += 1;
        MockClientMessage::Input {
            sequence: self.input_sequence,
            movement: input,
        }
    }

    fn record_latency(&mut self, sent_at: Instant) {
        let latency = sent_at.elapsed();
        self.latency_samples.push(latency);
    }

    fn average_latency(&self) -> Duration {
        if self.latency_samples.is_empty() {
            return Duration::ZERO;
        }
        let total: Duration = self.latency_samples.iter().sum();
        total / self.latency_samples.len() as u32
    }

    fn bandwidth_usage(&self, duration: Duration) -> f64 {
        let total_bytes = self.bytes_sent + self.bytes_received;
        (total_bytes as f64) / duration.as_secs_f64()
    }
}

/// Simulated server
struct SimulatedServer {
    world: World,
    clients: Vec<usize>,
    tick: u32,
}

impl SimulatedServer {
    fn new(scenario: &GameScenario) -> Self {
        let mut world = World::new();

        // Spawn entities for the scenario
        for i in 0..scenario.entity_count {
            let entity = world.spawn();
            let position = Vec3::new(
                (i as f32 % 100.0) * 10.0,
                0.0,
                (i as f32 / 100.0) * 10.0,
            );
            world.add(entity, Transform::new(position, engine_core::Quat::IDENTITY, Vec3::ONE));
            world.add(entity, Velocity::new(0.0, 0.0, 0.0));
        }

        Self {
            world,
            clients: Vec::new(),
            tick: 0,
        }
    }

    fn add_client(&mut self, client_id: usize) -> Entity {
        self.clients.push(client_id);
        let entity = self.world.spawn();
        let position = Vec3::new(
            (client_id as f32 % 10.0) * 20.0,
            0.0,
            (client_id as f32 / 10.0) * 20.0,
        );
        self.world.add(entity, Transform::new(position, engine_core::Quat::IDENTITY, Vec3::ONE));
        self.world.add(entity, Velocity::new(0.0, 0.0, 0.0));
        entity
    }

    fn process_input(&mut self, entity: Entity, movement: Vec3) {
        if let Some(velocity) = self.world.get_mut::<Velocity>(entity) {
            velocity.x = movement.x;
            velocity.z = movement.z;
        }
    }

    fn tick(&mut self, dt: f32) {
        self.tick += 1;

        // Update all entities with velocity
        // Collect entities and velocities first to avoid borrow conflicts
        let updates: Vec<_> = {
            let query = self.world.query::<(&Transform, &Velocity)>();
            query.map(|(e, (_, vel))| (e, vel.x, vel.z)).collect()
        };

        // Then apply updates
        for (entity, vel_x, vel_z) in updates {
            if let Some(transform) = self.world.get_mut::<Transform>(entity) {
                transform.position.x += vel_x * dt;
                transform.position.z += vel_z * dt;
            }
        }
    }

    fn create_state_update(&self, client_entity: Entity, radius: f32) -> MockServerMessage {
        let client_pos = self
            .world
            .get::<Transform>(client_entity)
            .map(|t| t.position)
            .unwrap_or(Vec3::ZERO);

        // Find entities within radius
        let query = self.world.query::<&Transform>();
        let nearby_entities: Vec<_> = query
            .filter(|(_, transform)| (transform.position - client_pos).length() <= radius)
            .map(|(e, _)| e)
            .collect();

        MockServerMessage::StateUpdate {
            tick: self.tick,
            entity_count: nearby_entities.len(),
        }
    }
}

/// Run a complete game loop simulation
fn simulate_game_loop(
    scenario: &GameScenario,
    profile: NetworkProfile,
    duration: Duration,
) -> SimulationResults {
    let mut server = SimulatedServer::new(scenario);
    let mut clients: Vec<SimulatedClient> = Vec::new();

    // Create clients
    for i in 0..scenario.player_count {
        let entity = server.add_client(i);
        let position = server
            .world
            .get::<Transform>(entity)
            .map(|t| t.position)
            .unwrap_or(Vec3::ZERO);
        clients.push(SimulatedClient::new(i, entity, position, profile));
    }

    let tick_duration = Duration::from_secs_f32(1.0 / scenario.update_frequency_hz as f32);
    let start = Instant::now();
    let mut ticks = 0;

    while start.elapsed() < duration {
        let tick_start = Instant::now();

        // Process client inputs
        for client in &mut clients {
            // Generate deterministic movement
            let movement = Vec3::new(
                ((client.id * 13 + ticks) % 3) as f32 - 1.0,
                0.0,
                ((client.id * 17 + ticks) % 3) as f32 - 1.0,
            ) * scenario.movement_speed;

            let msg = client.send_input(movement);

            // Simulate sending through network
            let serialized = bincode::serialize(&msg).unwrap();
            client.bytes_sent += serialized.len();
            client.simulator.send(serialized);
        }

        // Server receives inputs
        for client in &mut clients {
            let received = client.simulator.recv();
            for packet in received {
                if let Ok(msg) = bincode::deserialize::<MockClientMessage>(&packet) {
                    if let MockClientMessage::Input { movement, .. } = msg {
                        server.process_input(client.entity, movement);
                    }
                }
            }
        }

        // Server tick
        server.tick(tick_duration.as_secs_f32());

        // Server broadcasts state updates
        for client in &mut clients {
            let update = server.create_state_update(client.entity, scenario.update_radius);
            let serialized = bincode::serialize(&update).unwrap();

            // Simulate sending through network
            client.simulator.send(serialized.clone());
            // Server also tracks sent bytes (in real system)
        }

        // Clients receive updates
        for client in &mut clients {
            let received = client.simulator.recv();
            for packet in received {
                client.bytes_received += packet.len();
                client.record_latency(tick_start);
            }
        }

        ticks += 1;

        // Maintain tick rate (don't sleep in benchmarks)
        let _elapsed = tick_start.elapsed();
    }

    SimulationResults {
        total_ticks: ticks,
        clients,
        duration: start.elapsed(),
    }
}

/// Results from a game simulation
struct SimulationResults {
    total_ticks: usize,
    clients: Vec<SimulatedClient>,
    duration: Duration,
}

impl SimulationResults {
    #[allow(dead_code)]
    fn average_latency(&self) -> Duration {
        if self.clients.is_empty() {
            return Duration::ZERO;
        }
        let total: Duration = self.clients.iter().map(|c| c.average_latency()).sum();
        total / self.clients.len() as u32
    }

    #[allow(dead_code)]
    fn max_latency(&self) -> Duration {
        self.clients
            .iter()
            .map(|c| c.average_latency())
            .max()
            .unwrap_or(Duration::ZERO)
    }

    fn average_bandwidth(&self) -> f64 {
        if self.clients.is_empty() {
            return 0.0;
        }
        self.clients
            .iter()
            .map(|c| c.bandwidth_usage(self.duration))
            .sum::<f64>()
            / self.clients.len() as f64
    }

    #[allow(dead_code)]
    fn total_bandwidth(&self) -> f64 {
        self.clients
            .iter()
            .map(|c| c.bandwidth_usage(self.duration))
            .sum()
    }
}

/// Benchmark: Full game loop with different scenarios
fn bench_game_scenarios(c: &mut Criterion) {
    let mut group = c.benchmark_group("game_scenarios");

    let scenarios = vec![
        GameScenario::mmorpg(),
        GameScenario::fps(),
        GameScenario::battle_royale(),
    ];

    for scenario in scenarios {
        group.bench_with_input(
            BenchmarkId::new(scenario.name, "LAN"),
            &scenario,
            |b, scenario| {
                b.iter(|| {
                    simulate_game_loop(
                        black_box(scenario),
                        NetworkProfile::Lan,
                        Duration::from_millis(100),
                    )
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: End-to-end latency measurement
fn bench_end_to_end_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("end_to_end_latency");

    let profiles = vec![
        ("LAN", NetworkProfile::Lan),
        ("Cable", NetworkProfile::Cable),
        ("DSL", NetworkProfile::Dsl),
        ("4G", NetworkProfile::FourG),
    ];

    for (name, profile) in profiles {
        group.bench_function(name, |b| {
            b.iter(|| {
                let results = simulate_game_loop(
                    black_box(&GameScenario::fps()),
                    profile,
                    Duration::from_millis(100),
                );
                black_box(results.average_latency())
            });
        });
    }

    group.finish();
}

/// Benchmark: Bandwidth usage per client
fn bench_bandwidth_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("bandwidth_usage");

    let update_rates = vec![30, 60, 120];

    for rate in update_rates {
        let mut scenario = GameScenario::fps();
        scenario.update_frequency_hz = rate;

        group.bench_with_input(BenchmarkId::from_parameter(rate), &scenario, |b, scenario| {
            b.iter(|| {
                let results = simulate_game_loop(
                    black_box(scenario),
                    NetworkProfile::Cable,
                    Duration::from_millis(1000),
                );
                black_box(results.average_bandwidth())
            });
        });
    }

    group.finish();
}

/// Benchmark: Multiple concurrent clients
fn bench_concurrent_clients(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_clients");
    group.sample_size(10); // Reduce sample size for expensive benchmarks

    let client_counts = vec![1, 10, 50, 100];

    for count in client_counts {
        let mut scenario = GameScenario::fps();
        scenario.player_count = count;

        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(count),
            &scenario,
            |b, scenario| {
                b.iter(|| {
                    simulate_game_loop(
                        black_box(scenario),
                        NetworkProfile::Cable,
                        Duration::from_millis(100),
                    )
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Packet loss resilience
fn bench_packet_loss_resilience(c: &mut Criterion) {
    let mut group = c.benchmark_group("packet_loss_resilience");

    let loss_rates = vec![
        ("0%", NetworkProfile::Lan),
        ("0.5%", NetworkProfile::Dsl),
        ("1%", NetworkProfile::FourG),
        ("3%", NetworkProfile::ThreeG),
    ];

    for (name, profile) in loss_rates {
        group.bench_function(name, |b| {
            b.iter(|| {
                let results = simulate_game_loop(
                    black_box(&GameScenario::fps()),
                    profile,
                    Duration::from_millis(100),
                );
                black_box(results.total_ticks)
            });
        });
    }

    group.finish();
}

/// Benchmark: Scalability curve (latency vs player count)
fn bench_scalability(c: &mut Criterion) {
    let mut group = c.benchmark_group("scalability");
    group.sample_size(10); // Reduce sample size for expensive benchmarks

    let player_counts = vec![10, 25, 50, 100];

    for count in player_counts {
        let mut scenario = GameScenario::mmorpg();
        scenario.player_count = count;

        group.bench_with_input(
            BenchmarkId::from_parameter(count),
            &scenario,
            |b, scenario| {
                b.iter(|| {
                    let results = simulate_game_loop(
                        black_box(scenario),
                        NetworkProfile::Cable,
                        Duration::from_millis(100),
                    );
                    black_box(results.average_latency())
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Network simulation overhead
fn bench_simulator_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("simulator_overhead");

    let profiles = vec![
        ("LAN", NetworkProfile::Lan),
        ("Cable", NetworkProfile::Cable),
        ("4G", NetworkProfile::FourG),
    ];

    for (name, profile) in profiles {
        group.bench_function(name, |b| {
            let mut sim = NetworkSimulator::new(profile);
            let data = vec![0u8; 1024]; // 1KB packet

            b.iter(|| {
                sim.send(black_box(data.clone()));
                let received = sim.recv();
                black_box(received)
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_game_scenarios,
    bench_end_to_end_latency,
    bench_bandwidth_usage,
    bench_concurrent_clients,
    bench_packet_loss_resilience,
    bench_scalability,
    bench_simulator_overhead,
);
criterion_main!(benches);
