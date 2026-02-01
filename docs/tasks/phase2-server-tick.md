# Phase 2.6: Server Tick Loop

**Status:** ⚪ Not Started
**Estimated Time:** 3-4 days
**Priority:** Critical (server game loop)

---

## 🎯 **Objective**

Implement authoritative server tick loop that runs game simulation at fixed rate (60 TPS), processes player input, updates game state, and broadcasts updates to clients.

**Server Responsibilities:**
- Run at fixed 60 TPS
- Process player input
- Update game logic
- Run physics simulation (server-authoritative)
- Broadcast state to clients
- Handle disconnections

---

## 📋 **Detailed Tasks**

### **1. Server Structure** (Day 1)

**File:** `engine/binaries/server/src/server.rs`

```rust
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

/// Game server
pub struct GameServer {
    /// ECS world (authoritative state)
    world: World,

    /// Current tick number
    current_tick: u64,

    /// Target ticks per second
    ticks_per_second: u32,

    /// Tick duration
    tick_duration: Duration,

    /// State synchronizer
    state_sync: ServerStateSynchronizer,

    /// Network (TCP/UDP)
    tcp_server: TcpServer,
    udp_server: UdpServer,

    /// Connected clients
    clients: HashMap<u64, ClientState>,

    /// Input queue (client_id, tick, input)
    input_queue: Vec<(u64, u64, PlayerInput)>,
}

#[derive(Debug)]
struct ClientState {
    client_id: u64,
    player_entity: Option<Entity>,
    last_input_tick: u64,
    connected_at: Instant,
}

impl GameServer {
    /// Create server
    pub async fn new(config: ServerConfig) -> Result<Self, ServerError> {
        // Create world
        let mut world = World::new();
        Self::register_components(&mut world);

        // Create network
        let tcp_server = TcpServer::new(&config.tcp_bind_addr).await?;
        let udp_server = UdpServer::new(&config.udp_bind_addr).await?;

        // Create state synchronizer
        let state_sync = ServerStateSynchronizer::new(
            world.clone(),
            SyncConfig::default(),
        );

        tracing::info!(
            "Server created: {}:{} (TCP), {}:{} (UDP)",
            config.tcp_bind_addr,
            config.udp_bind_addr,
            config.tcp_bind_addr,
            config.udp_bind_addr
        );

        Ok(Self {
            world,
            current_tick: 0,
            ticks_per_second: config.ticks_per_second,
            tick_duration: Duration::from_secs_f64(1.0 / config.ticks_per_second as f64),
            state_sync,
            tcp_server,
            udp_server,
            clients: HashMap::new(),
            input_queue: Vec::new(),
        })
    }

    /// Register all component types
    fn register_components(world: &mut World) {
        world.register::<Transform>();
        world.register::<Velocity>();
        world.register::<Health>();
        world.register::<PlayerConnection>();
        world.register::<ServerAuthority>();
        // ... register all components
    }

    /// Run server
    pub async fn run(&mut self) -> Result<(), ServerError> {
        tracing::info!("Server starting at {} TPS", self.ticks_per_second);

        let mut last_tick = Instant::now();

        loop {
            let tick_start = Instant::now();

            // Process network events
            self.process_network_events().await?;

            // Process input queue
            self.process_input();

            // Run game tick
            self.tick();

            // Send state updates
            self.send_state_updates().await?;

            // Sleep until next tick
            let elapsed = tick_start.elapsed();
            if elapsed < self.tick_duration {
                tokio::time::sleep(self.tick_duration - elapsed).await;
            } else {
                tracing::warn!(
                    "Tick took {:.2}ms (budget: {:.2}ms)",
                    elapsed.as_secs_f64() * 1000.0,
                    self.tick_duration.as_secs_f64() * 1000.0
                );
            }

            // Advance tick
            self.current_tick += 1;

            // Log tick rate
            if self.current_tick % 60 == 0 {
                let actual_tick_duration = last_tick.elapsed();
                let actual_tps = 60.0 / actual_tick_duration.as_secs_f64();
                tracing::debug!("Actual TPS: {:.2}", actual_tps);
                last_tick = Instant::now();
            }
        }
    }

    /// Process network events (connections, messages)
    async fn process_network_events(&mut self) -> Result<(), ServerError> {
        // Process TCP events
        while let Ok(event) = self.tcp_server.try_recv_event() {
            match event {
                ServerEvent::ClientConnected { client_id, addr } => {
                    self.handle_client_connected(client_id, addr);
                }
                ServerEvent::ClientDisconnected { client_id } => {
                    self.handle_client_disconnected(client_id);
                }
                ServerEvent::MessageReceived { client_id, data } => {
                    self.handle_client_message(client_id, data)?;
                }
            }
        }

        // Process UDP packets
        while let Ok((client_id, data)) = self.udp_server.try_recv() {
            self.handle_udp_packet(client_id, data)?;
        }

        Ok(())
    }

    /// Handle client connected
    fn handle_client_connected(&mut self, client_id: u64, addr: SocketAddr) {
        tracing::info!("Client {} connected from {}", client_id, addr);

        let client = ClientState {
            client_id,
            player_entity: None,
            last_input_tick: 0,
            connected_at: Instant::now(),
        };

        self.clients.insert(client_id, client);
        self.state_sync.add_client(client_id);
    }

    /// Handle client disconnected
    fn handle_client_disconnected(&mut self, client_id: u64) {
        tracing::info!("Client {} disconnected", client_id);

        // Remove player entity
        if let Some(client) = self.clients.get(&client_id) {
            if let Some(player_entity) = client.player_entity {
                self.world.despawn(player_entity);
            }
        }

        self.clients.remove(&client_id);
        self.state_sync.remove_client(client_id);
    }

    /// Handle client message
    fn handle_client_message(&mut self, client_id: u64, data: Vec<u8>) -> Result<(), ServerError> {
        let packet = Protocol::decode_client_packet(&data)?;

        match packet.message_type() {
            ClientMessage::JoinRequest => {
                let request = packet.message_as_join_request().unwrap();
                self.handle_join_request(client_id, request)?;
            }
            ClientMessage::PlayerInput => {
                let input = packet.message_as_player_input().unwrap();
                self.handle_player_input(client_id, input)?;
            }
            _ => {
                tracing::warn!("Unknown client message type from {}", client_id);
            }
        }

        Ok(())
    }

    /// Handle join request
    fn handle_join_request(
        &mut self,
        client_id: u64,
        request: JoinRequest,
    ) -> Result<(), ServerError> {
        tracing::info!("Client {} joining as '{}'", client_id, request.player_name());

        // Spawn player entity
        let player_entity = self.world.spawn();
        self.world.add(player_entity, Transform::default());
        self.world.add(player_entity, Velocity(Vec3::ZERO));
        self.world.add(player_entity, Health {
            current: 100.0,
            max: 100.0,
        });
        self.world.add(player_entity, PlayerConnection { connection_id: client_id });

        // Update client state
        if let Some(client) = self.clients.get_mut(&client_id) {
            client.player_entity = Some(player_entity);
        }

        // Send join response
        let response = JoinResponseBuilder::build(true, client_id, "");
        self.tcp_server.send_to_client(client_id, response).await?;

        // Send initial world snapshot
        let snapshot = WorldSnapshotBuilder::build(self.current_tick, &self.world);
        self.tcp_server.send_to_client(client_id, snapshot).await?;

        tracing::info!("Client {} joined successfully", client_id);

        Ok(())
    }

    /// Handle player input
    fn handle_player_input(
        &mut self,
        client_id: u64,
        input: PlayerInput,
    ) -> Result<(), ServerError> {
        // Queue input for processing
        self.input_queue.push((client_id, input.sequence() as u64, input));
        Ok(())
    }

    /// Handle UDP packet
    fn handle_udp_packet(&mut self, client_id: u64, data: Vec<u8>) -> Result<(), ServerError> {
        // UDP packets are typically PlayerInput (unreliable)
        let packet = Protocol::decode_client_packet(&data)?;

        if let ClientMessage::PlayerInput = packet.message_type() {
            let input = packet.message_as_player_input().unwrap();
            self.handle_player_input(client_id, input)?;
        }

        Ok(())
    }

    /// Process input queue
    fn process_input(&mut self) {
        // Sort by tick (oldest first)
        self.input_queue.sort_by_key(|(_, tick, _)| *tick);

        for (client_id, tick, input) in self.input_queue.drain(..) {
            if let Some(client) = self.clients.get_mut(&client_id) {
                if let Some(player_entity) = client.player_entity {
                    // Apply input to player entity
                    if let Some(velocity) = self.world.get_mut::<Velocity>(player_entity) {
                        // Convert input to velocity
                        let movement = Vec3::new(
                            input.movement().x(),
                            input.movement().y(),
                            input.movement().z(),
                        );
                        velocity.0 = movement * 5.0; // Movement speed
                    }
                }

                client.last_input_tick = tick;
            }
        }
    }

    /// Run game tick
    fn tick(&mut self) {
        // Run systems
        movement_system(&mut self.world, self.tick_duration.as_secs_f32());
        physics_system(&mut self.world);
        // ... other systems

        // Update state synchronizer
        let _ = self.state_sync.tick(self.current_tick);
    }

    /// Send state updates to clients
    async fn send_state_updates(&mut self) -> Result<(), ServerError> {
        let updates = self.state_sync.tick(self.current_tick);

        for (client_id, data) in updates {
            // Send via TCP (reliable)
            self.tcp_server.send_to_client(client_id, data).await?;
        }

        Ok(())
    }
}

/// Server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub tcp_bind_addr: String,
    pub udp_bind_addr: String,
    pub ticks_per_second: u32,
    pub max_clients: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            tcp_bind_addr: "0.0.0.0:7777".to_string(),
            udp_bind_addr: "0.0.0.0:7778".to_string(),
            ticks_per_second: 60,
            max_clients: 1000,
        }
    }
}
```

---

### **2. Server Systems** (Day 2)

**File:** `engine/core/src/systems/server.rs`

```rust
use agent_game_engine_macros::server_only;

/// Physics system (server-only, authoritative)
#[server_only]
pub fn physics_system(world: &mut World) {
    // Apply gravity
    for (entity, (transform, velocity)) in world.query::<(&mut Transform, &mut Velocity)>() {
        velocity.0.y -= 9.81 * 0.016; // Gravity
    }

    // Ground collision (simple)
    for (entity, (transform, velocity)) in world.query::<(&mut Transform, &mut Velocity)>() {
        if transform.position.y < 0.0 {
            transform.position.y = 0.0;
            velocity.0.y = 0.0;
        }
    }
}

/// Health regeneration system (server-only)
#[server_only]
pub fn health_regen_system(world: &mut World, dt: f32) {
    for (entity, health) in world.query::<&mut Health>() {
        if health.current < health.max {
            health.current += 5.0 * dt; // Regen 5 HP/s
            health.current = health.current.min(health.max);
        }
    }
}

/// Timeout disconnected clients (server-only)
#[server_only]
pub fn timeout_system(world: &mut World, current_time: Instant) {
    const TIMEOUT_DURATION: Duration = Duration::from_secs(30);

    let mut to_remove = Vec::new();

    for (entity, connection) in world.query::<&PlayerConnection>() {
        // Check if client timed out
        // (Would need to track last activity time)
        // if current_time - connection.last_activity > TIMEOUT_DURATION {
        //     to_remove.push(entity);
        // }
    }

    for entity in to_remove {
        world.despawn(entity);
        tracing::info!("Entity {:?} timed out and removed", entity);
    }
}
```

---

### **3. Server Main** (Day 2-3)

**File:** `engine/binaries/server/src/main.rs`

```rust
use tokio::signal;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,agent_game_engine=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Agent Game Engine Server starting...");

    // Load config
    let config = ServerConfig::default();

    // Create server
    let mut server = GameServer::new(config).await?;

    // Spawn server task
    let server_task = tokio::spawn(async move {
        server.run().await
    });

    // Wait for shutdown signal
    signal::ctrl_c().await?;
    tracing::info!("Shutdown signal received");

    // Cleanup
    server_task.abort();

    tracing::info!("Server shutdown complete");

    Ok(())
}
```

---

### **4. Performance Monitoring** (Day 3-4)

**File:** `engine/networking/src/server/metrics.rs`

```rust
use std::time::{Duration, Instant};

/// Server performance metrics
#[derive(Debug, Clone)]
pub struct ServerMetrics {
    pub tick: u64,
    pub tick_duration_ms: f32,
    pub client_count: usize,
    pub entity_count: usize,
    pub input_queue_size: usize,
    pub bandwidth_out_kbps: f32,
}

/// Metrics collector
pub struct MetricsCollector {
    metrics_history: Vec<ServerMetrics>,
    last_metrics_time: Instant,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            metrics_history: Vec::new(),
            last_metrics_time: Instant::now(),
        }
    }

    /// Record metrics for current tick
    pub fn record(&mut self, metrics: ServerMetrics) {
        self.metrics_history.push(metrics.clone());

        // Log every second
        if self.last_metrics_time.elapsed() >= Duration::from_secs(1) {
            self.log_metrics(&metrics);
            self.last_metrics_time = Instant::now();
        }

        // Keep last 60 seconds
        if self.metrics_history.len() > 60 * 60 {
            self.metrics_history.remove(0);
        }
    }

    /// Log current metrics
    fn log_metrics(&self, metrics: &ServerMetrics) {
        tracing::info!(
            "Tick {}: {:.2}ms, {} clients, {} entities, {:.2} KB/s out",
            metrics.tick,
            metrics.tick_duration_ms,
            metrics.client_count,
            metrics.entity_count,
            metrics.bandwidth_out_kbps
        );
    }

    /// Get average tick time (last N ticks)
    pub fn average_tick_time(&self, n: usize) -> f32 {
        let recent: Vec<_> = self
            .metrics_history
            .iter()
            .rev()
            .take(n)
            .collect();

        if recent.is_empty() {
            return 0.0;
        }

        let sum: f32 = recent.iter().map(|m| m.tick_duration_ms).sum();
        sum / recent.len() as f32
    }
}
```

---

## ✅ **Acceptance Criteria**

- [ ] Server runs at stable 60 TPS
- [ ] Clients can connect via TCP
- [ ] Player input processed correctly
- [ ] Game state updated each tick
- [ ] State synchronized to all clients
- [ ] Disconnections handled gracefully
- [ ] Tick budget maintained (< 16.67ms)
- [ ] 100+ concurrent clients supported
- [ ] Metrics logged every second
- [ ] Graceful shutdown on Ctrl+C

---

## 🎯 **Performance Targets**

| Metric | Target | Critical |
|--------|--------|----------|
| Tick rate | 60 TPS ± 1 | 55-65 TPS |
| Tick duration | < 10ms | < 16ms |
| Input latency | < 50ms | < 100ms |
| State sync latency | < 100ms | < 200ms |
| Max clients (1000 entities) | > 100 | > 50 |
| Memory usage | < 500 MB | < 1 GB |
| CPU usage (idle) | < 10% | < 20% |
| CPU usage (full load) | < 80% | < 95% |

---

## 🧪 **Tests**

```rust
#[tokio::test]
async fn test_server_starts() {
    let config = ServerConfig::default();
    let mut server = GameServer::new(config).await.unwrap();

    // Should start successfully
}

#[tokio::test]
async fn test_server_tick_rate() {
    let config = ServerConfig {
        ticks_per_second: 10,
        ..Default::default()
    };

    let mut server = GameServer::new(config).await.unwrap();

    let start = Instant::now();

    // Run 10 ticks
    for _ in 0..10 {
        server.tick();
    }

    let elapsed = start.elapsed();

    // Should take ~1 second (10 ticks at 10 TPS)
    assert!(elapsed >= Duration::from_millis(900));
    assert!(elapsed <= Duration::from_millis(1100));
}

#[tokio::test]
async fn test_client_join() {
    let mut server = GameServer::new(ServerConfig::default()).await.unwrap();

    // Simulate client connection
    let client_id = 1;
    server.handle_client_connected(client_id, "127.0.0.1:1234".parse().unwrap());

    // Client should be registered
    assert!(server.clients.contains_key(&client_id));

    // Simulate join request
    let request = JoinRequest { player_name: "TestPlayer", version: 1 };
    server.handle_join_request(client_id, request).await.unwrap();

    // Player entity should be spawned
    let client = server.clients.get(&client_id).unwrap();
    assert!(client.player_entity.is_some());
}
```

---

## 💡 **Usage**

```bash
# Start server
cargo run --bin server --features server --release

# With custom config
RUST_LOG=debug cargo run --bin server --features server

# Production (with systemd)
[Unit]
Description=Agent Game Engine Server
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/server
Restart=always

[Install]
WantedBy=multi-user.target
```

---

**Dependencies:** [phase2-state-sync.md](phase2-state-sync.md)
**Next:** Phase 3 (Physics, Audio, LOD)
