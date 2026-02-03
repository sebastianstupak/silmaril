//! Server tick loop implementation
//!
//! Provides a production-quality fixed timestep server loop with:
//! - 60 TPS (16.67ms per tick) fixed timestep
//! - Client connection management
//! - State broadcasting
//! - Input processing
//! - Performance monitoring

use crate::protocol::{
    deserialize_client_message, serialize_server_message, ClientMessage, EntityState,
    ProtocolError, SerializationFormat, ServerMessage, PROTOCOL_VERSION,
};
use crate::snapshot::WorldSnapshot;
use crate::tcp::{TcpConnection, TcpError, TcpServer};
use engine_core::ecs::{Entity, World};
use std::collections::HashMap;
use std::fmt;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

/// Target ticks per second
pub const TARGET_TPS: u32 = 60;

/// Target tick duration (16.67ms)
pub const TARGET_TICK_DURATION: Duration = Duration::from_micros(16_667);

/// Maximum tick time before warning (33ms - allows for 30 FPS fallback)
pub const MAX_TICK_DURATION: Duration = Duration::from_millis(33);

/// Maximum accumulated time before skipping ticks
pub const MAX_ACCUMULATOR: Duration = Duration::from_millis(100);

/// Client timeout duration (30 seconds of inactivity)
pub const CLIENT_TIMEOUT: Duration = Duration::from_secs(30);

/// Server tick loop errors
#[derive(Debug)]
pub enum ServerLoopError {
    /// TCP error
    TcpError(TcpError),
    /// Protocol error
    ProtocolError(ProtocolError),
    /// Serialization error
    SerializationError(String),
    /// Server shutdown
    Shutdown,
}

impl std::fmt::Display for ServerLoopError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServerLoopError::TcpError(e) => write!(f, "TCP error: {}", e),
            ServerLoopError::ProtocolError(e) => write!(f, "Protocol error: {}", e),
            ServerLoopError::SerializationError(e) => write!(f, "Serialization error: {}", e),
            ServerLoopError::Shutdown => write!(f, "Server shutdown"),
        }
    }
}

impl std::error::Error for ServerLoopError {}

impl From<TcpError> for ServerLoopError {
    fn from(e: TcpError) -> Self {
        ServerLoopError::TcpError(e)
    }
}

impl From<ProtocolError> for ServerLoopError {
    fn from(e: ProtocolError) -> Self {
        ServerLoopError::ProtocolError(e)
    }
}

/// Result type for server loop operations
pub type ServerLoopResult<T> = Result<T, ServerLoopError>;

/// Client connection state
struct ClientState {
    /// TCP connection (used by message sender task)
    #[allow(dead_code)]
    connection: Arc<TcpConnection>,
    /// Client's player entity
    player_entity: Option<Entity>,
    /// Client address
    address: SocketAddr,
    /// Client name
    name: String,
    /// Connected timestamp
    connected_at: Instant,
    /// Last message received timestamp
    last_message_at: Instant,
    /// Message sender for this client
    message_tx: UnboundedSender<ServerMessage>,
}

impl fmt::Debug for ClientState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClientState")
            .field("player_entity", &self.player_entity)
            .field("address", &self.address)
            .field("name", &self.name)
            .field("connected_at", &self.connected_at)
            .field("last_message_at", &self.last_message_at)
            .finish()
    }
}

/// Network event from clients
#[allow(clippy::enum_variant_names)]
enum NetworkEvent {
    /// New client connected
    ClientConnected {
        /// Client ID
        client_id: u64,
        /// Connection
        connection: Arc<TcpConnection>,
        /// Client address
        address: SocketAddr,
    },
    /// Client disconnected
    ClientDisconnected {
        /// Client ID
        client_id: u64,
    },
    /// Client message received
    ClientMessage {
        /// Client ID
        client_id: u64,
        /// Message
        message: ClientMessage,
    },
}

/// Server tick loop state
pub struct ServerLoop {
    /// Game world
    world: Arc<Mutex<World>>,
    /// Connected clients
    clients: Arc<Mutex<HashMap<u64, ClientState>>>,
    /// Network event receiver
    event_rx: UnboundedReceiver<NetworkEvent>,
    /// Network event sender (for spawning connection tasks)
    event_tx: UnboundedSender<NetworkEvent>,
    /// Tick counter
    tick: u64,
    /// Performance stats
    stats: PerformanceStats,
}

/// Performance statistics
#[derive(Debug, Clone, Default)]
pub struct PerformanceStats {
    /// Current tick
    pub tick: u64,
    /// Average tick duration (ms)
    pub avg_tick_ms: f64,
    /// Min tick duration (ms)
    pub min_tick_ms: f64,
    /// Max tick duration (ms)
    pub max_tick_ms: f64,
    /// Current TPS
    pub current_tps: f64,
    /// Connected clients
    pub client_count: usize,
    /// Messages processed this second
    pub messages_per_second: u64,
}

impl ServerLoop {
    /// Create a new server loop
    pub fn new(world: World) -> Self {
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        Self {
            world: Arc::new(Mutex::new(world)),
            clients: Arc::new(Mutex::new(HashMap::new())),
            event_rx,
            event_tx,
            tick: 0,
            stats: PerformanceStats::default(),
        }
    }

    /// Get current performance statistics
    pub fn stats(&self) -> PerformanceStats {
        self.stats.clone()
    }

    /// Start accepting client connections
    pub async fn start_accepting(&self, tcp_server: TcpServer) {
        let event_tx = self.event_tx.clone();

        tokio::spawn(async move {
            loop {
                match tcp_server.accept().await {
                    Ok(connection) => {
                        let address = connection.peer_addr();
                        let client_id = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs();

                        info!(
                            client_id = client_id,
                            address = %address,
                            "Client connected"
                        );

                        let connection = Arc::new(connection);

                        if let Err(e) = event_tx.send(NetworkEvent::ClientConnected {
                            client_id,
                            connection: connection.clone(),
                            address,
                        }) {
                            error!(error = ?e, "Failed to send client connected event");
                            break;
                        }

                        // Spawn task to receive messages from this client
                        let event_tx = event_tx.clone();
                        tokio::spawn(async move {
                            Self::handle_client_messages(client_id, connection, event_tx).await;
                        });
                    }
                    Err(e) => {
                        error!(error = ?e, "Failed to accept client connection");
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                }
            }
        });
    }

    /// Handle incoming messages from a client
    async fn handle_client_messages(
        client_id: u64,
        connection: Arc<TcpConnection>,
        event_tx: UnboundedSender<NetworkEvent>,
    ) {
        loop {
            match connection.recv().await {
                Ok(data) => {
                    // Deserialize message
                    match deserialize_client_message(
                        &crate::protocol::FramedMessage::new(data).unwrap(),
                        SerializationFormat::Bincode,
                    ) {
                        Ok(message) => {
                            if let Err(e) =
                                event_tx.send(NetworkEvent::ClientMessage { client_id, message })
                            {
                                error!(
                                    client_id = client_id,
                                    error = ?e,
                                    "Failed to send client message event"
                                );
                                break;
                            }
                        }
                        Err(e) => {
                            warn!(
                                client_id = client_id,
                                error = ?e,
                                "Failed to deserialize client message"
                            );
                        }
                    }
                }
                Err(TcpError::ConnectionClosed) => {
                    info!(client_id = client_id, "Client disconnected");
                    let _ = event_tx.send(NetworkEvent::ClientDisconnected { client_id });
                    break;
                }
                Err(e) => {
                    error!(
                        client_id = client_id,
                        error = ?e,
                        "Error receiving message from client"
                    );
                    let _ = event_tx.send(NetworkEvent::ClientDisconnected { client_id });
                    break;
                }
            }
        }
    }

    /// Run the server loop
    ///
    /// This is the main server loop that runs at 60 TPS using a fixed timestep
    /// accumulator pattern. It processes network events, runs game logic,
    /// and broadcasts state updates.
    pub async fn run(
        &mut self,
        mut tick_callback: impl FnMut(&mut World, f32),
    ) -> ServerLoopResult<()> {
        info!("Server loop starting at {} TPS", TARGET_TPS);

        let mut accumulator = Duration::ZERO;
        let mut last_tick = Instant::now();
        let mut tick_times = Vec::with_capacity(60);
        let mut last_stats_update = Instant::now();
        let mut last_timeout_check = Instant::now();
        let mut messages_this_second = 0u64;

        loop {
            let frame_start = Instant::now();
            let delta = frame_start - last_tick;
            last_tick = frame_start;

            accumulator += delta;

            // Prevent spiral of death - if we're too far behind, skip ticks
            if accumulator > MAX_ACCUMULATOR {
                warn!(
                    accumulated_ms = accumulator.as_millis(),
                    "Server is running behind, skipping accumulated time"
                );
                accumulator = TARGET_TICK_DURATION;
            }

            // Process network events (non-blocking)
            while let Ok(event) = self.event_rx.try_recv() {
                self.process_network_event(event).await;
                messages_this_second += 1;
            }

            // Fixed timestep tick loop
            while accumulator >= TARGET_TICK_DURATION {
                let tick_start = Instant::now();

                // Process game tick
                self.process_tick(&mut tick_callback).await?;

                let tick_duration = tick_start.elapsed();
                tick_times.push(tick_duration.as_secs_f64() * 1000.0);

                // Warn if tick took too long
                if tick_duration > MAX_TICK_DURATION {
                    warn!(
                        tick = self.tick,
                        duration_ms = tick_duration.as_millis(),
                        "Tick exceeded maximum duration"
                    );
                }

                accumulator -= TARGET_TICK_DURATION;
                self.tick += 1;
            }

            // Update stats every second
            if last_stats_update.elapsed() >= Duration::from_secs(1) {
                self.update_stats(&tick_times, messages_this_second).await;
                tick_times.clear();
                messages_this_second = 0;
                last_stats_update = Instant::now();
            }

            // Check for timed-out clients every second
            if last_timeout_check.elapsed() >= Duration::from_secs(1) {
                self.check_timeouts().await;
                last_timeout_check = Instant::now();
            }

            // Sleep to maintain target tick rate
            let frame_time = frame_start.elapsed();
            if frame_time < TARGET_TICK_DURATION {
                tokio::time::sleep(TARGET_TICK_DURATION - frame_time).await;
            }
        }
    }

    /// Process a single game tick
    async fn process_tick(
        &mut self,
        tick_callback: &mut impl FnMut(&mut World, f32),
    ) -> ServerLoopResult<()> {
        let dt = TARGET_TICK_DURATION.as_secs_f32();

        // Run game logic
        {
            let mut world = self.world.lock().await;
            tick_callback(&mut world, dt);
        }

        // Broadcast state to all clients
        self.broadcast_state().await?;

        Ok(())
    }

    /// Process a network event
    async fn process_network_event(&mut self, event: NetworkEvent) {
        match event {
            NetworkEvent::ClientConnected { client_id, connection, address } => {
                let (message_tx, mut message_rx) = mpsc::unbounded_channel();

                let state = ClientState {
                    connection: connection.clone(),
                    player_entity: None,
                    address,
                    name: format!("Client{}", client_id),
                    connected_at: Instant::now(),
                    last_message_at: Instant::now(),
                    message_tx,
                };

                self.clients.lock().await.insert(client_id, state);

                // Spawn task to send messages to this client
                tokio::spawn(async move {
                    while let Some(message) = message_rx.recv().await {
                        if let Ok(framed) =
                            serialize_server_message(&message, SerializationFormat::Bincode)
                        {
                            if let Err(e) = connection.send(&framed.payload).await {
                                error!(
                                    client_id = client_id,
                                    error = ?e,
                                    "Failed to send message to client"
                                );
                                break;
                            }
                        }
                    }
                });

                info!(
                    client_id = client_id,
                    address = %address,
                    client_count = self.clients.lock().await.len(),
                    "Client added to server state"
                );
            }

            NetworkEvent::ClientDisconnected { client_id } => {
                if let Some(client) = self.clients.lock().await.remove(&client_id) {
                    info!(
                        client_id = client_id,
                        address = %client.address,
                        duration_secs = client.connected_at.elapsed().as_secs(),
                        "Client disconnected"
                    );

                    // Despawn player entity if it exists
                    if let Some(entity) = client.player_entity {
                        let mut world = self.world.lock().await;
                        world.despawn(entity);
                    }
                }
            }

            NetworkEvent::ClientMessage { client_id, message } => {
                self.process_client_message(client_id, message).await;
            }
        }
    }

    /// Process a message from a client
    async fn process_client_message(&mut self, client_id: u64, message: ClientMessage) {
        debug!(
            client_id = client_id,
            message = ?message,
            "Processing client message"
        );

        // Update last message time
        if let Some(client) = self.clients.lock().await.get_mut(&client_id) {
            client.last_message_at = Instant::now();
        }

        match message {
            ClientMessage::Handshake { version, client_name } => {
                if version != PROTOCOL_VERSION {
                    error!(
                        client_id = client_id,
                        client_version = version,
                        server_version = PROTOCOL_VERSION,
                        "Protocol version mismatch - disconnecting client"
                    );

                    // Send error response before disconnecting
                    if let Some(client) = self.clients.lock().await.get(&client_id) {
                        let error_msg = ServerMessage::ChatBroadcast {
                            sender: "Server".to_string(),
                            message: format!(
                                "Protocol version mismatch: client={}, server={}. Please update your client.",
                                version, PROTOCOL_VERSION
                            ),
                            channel: 0,
                        };
                        let _ = client.message_tx.send(error_msg);
                    }

                    // Disconnect the client
                    self.clients.lock().await.remove(&client_id);
                    return;
                }

                // Spawn player entity
                let player_entity = {
                    let mut world = self.world.lock().await;
                    world.spawn()
                };

                // Update client state
                if let Some(client) = self.clients.lock().await.get_mut(&client_id) {
                    client.player_entity = Some(player_entity);
                    client.name = client_name.clone();

                    // Send handshake response
                    let response = ServerMessage::HandshakeResponse {
                        version: PROTOCOL_VERSION,
                        server_name: "Silmaril Server".to_string(),
                        player_entity,
                    };

                    if let Err(e) = client.message_tx.send(response) {
                        error!(
                            client_id = client_id,
                            error = ?e,
                            "Failed to send handshake response"
                        );
                    }
                }

                info!(
                    client_id = client_id,
                    client_name = %client_name,
                    player_entity = ?player_entity,
                    "Client handshake complete"
                );
            }

            ClientMessage::Ping { client_time } => {
                if let Some(client) = self.clients.lock().await.get(&client_id) {
                    let server_time = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64;

                    let pong = ServerMessage::Pong { client_time, server_time };

                    if let Err(e) = client.message_tx.send(pong) {
                        error!(
                            client_id = client_id,
                            error = ?e,
                            "Failed to send pong"
                        );
                    }
                }
            }

            ClientMessage::PlayerMove { x, y, z, timestamp: _ } => {
                debug!(client_id = client_id, x = x, y = y, z = z, "Player move");
                // TODO: Apply movement to player entity
            }

            ClientMessage::PlayerAction { action_id, target, timestamp: _ } => {
                debug!(
                    client_id = client_id,
                    action_id = action_id,
                    target = ?target,
                    "Player action"
                );
                // TODO: Process player action
            }

            ClientMessage::ChatMessage { message, channel } => {
                // Broadcast chat to all clients
                let sender_name = self
                    .clients
                    .lock()
                    .await
                    .get(&client_id)
                    .map(|c| c.name.clone())
                    .unwrap_or_else(|| format!("Client{}", client_id));

                let chat = ServerMessage::ChatBroadcast { sender: sender_name, message, channel };

                self.broadcast_message(chat).await;
            }

            ClientMessage::SpawnRequest { prefab_id, x, y, z } => {
                debug!(
                    client_id = client_id,
                    prefab_id = prefab_id,
                    x = x,
                    y = y,
                    z = z,
                    "Spawn request"
                );
                // TODO: Spawn entity
            }
        }
    }

    /// Broadcast world state to all connected clients
    async fn broadcast_state(&self) -> ServerLoopResult<()> {
        let world = self.world.lock().await;
        let snapshot = WorldSnapshot::from_world(&world);

        // Convert snapshot to entity states
        let entities: Vec<EntityState> = snapshot
            .entities()
            .iter()
            .map(|&entity| EntityState {
                entity,
                x: 0.0,
                y: 0.0,
                z: 0.0,
                qx: 0.0,
                qy: 0.0,
                qz: 0.0,
                qw: 1.0,
                health: None,
                max_health: None,
            })
            .collect();

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let message = ServerMessage::StateUpdate { timestamp, entities };

        drop(world); // Release lock before broadcasting

        self.broadcast_message(message).await;

        Ok(())
    }

    /// Broadcast a message to all connected clients
    async fn broadcast_message(&self, message: ServerMessage) {
        let clients = self.clients.lock().await;

        for (client_id, client) in clients.iter() {
            if let Err(e) = client.message_tx.send(message.clone()) {
                error!(
                    client_id = client_id,
                    error = ?e,
                    "Failed to send broadcast message"
                );
            }
        }
    }

    /// Update performance statistics
    async fn update_stats(&mut self, tick_times: &[f64], messages_per_second: u64) {
        if tick_times.is_empty() {
            return;
        }

        let avg = tick_times.iter().sum::<f64>() / tick_times.len() as f64;
        let min = tick_times.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = tick_times.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        let client_count = self.clients.lock().await.len();

        self.stats = PerformanceStats {
            tick: self.tick,
            avg_tick_ms: avg,
            min_tick_ms: min,
            max_tick_ms: max,
            current_tps: tick_times.len() as f64,
            client_count,
            messages_per_second,
        };

        debug!(
            tick = self.tick,
            avg_tick_ms = format!("{:.2}", avg),
            min_tick_ms = format!("{:.2}", min),
            max_tick_ms = format!("{:.2}", max),
            tps = tick_times.len(),
            clients = client_count,
            messages_per_sec = messages_per_second,
            "Server performance"
        );

        // Warn if performance is degrading
        if avg > MAX_TICK_DURATION.as_secs_f64() * 1000.0 {
            warn!(
                avg_tick_ms = format!("{:.2}", avg),
                max_allowed_ms = MAX_TICK_DURATION.as_millis(),
                "Average tick duration exceeding target"
            );
        }
    }

    /// Get number of connected clients
    pub async fn client_count(&self) -> usize {
        self.clients.lock().await.len()
    }

    /// Get current tick number
    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// Check for and remove timed-out clients
    ///
    /// Removes clients that haven't sent any messages within CLIENT_TIMEOUT duration.
    /// Returns the number of clients that were removed.
    pub async fn check_timeouts(&mut self) -> usize {
        let now = Instant::now();
        let mut timed_out = Vec::new();

        // Find timed-out clients
        {
            let clients = self.clients.lock().await;
            for (client_id, client) in clients.iter() {
                let idle_duration = now.duration_since(client.last_message_at);
                if idle_duration > CLIENT_TIMEOUT {
                    timed_out.push((*client_id, client.address, idle_duration));
                }
            }
        }

        // Remove timed-out clients and cleanup their resources
        let mut removed_count = 0;
        for (client_id, address, idle_duration) in timed_out {
            if let Some(client) = self.clients.lock().await.remove(&client_id) {
                warn!(
                    client_id = client_id,
                    address = %address,
                    idle_secs = idle_duration.as_secs(),
                    "Client timed out due to inactivity"
                );

                // Despawn player entity if it exists
                if let Some(entity) = client.player_entity {
                    let mut world = self.world.lock().await;
                    world.despawn(entity);
                }

                removed_count += 1;
            }
        }

        if removed_count > 0 {
            info!(
                removed_count = removed_count,
                remaining_clients = self.clients.lock().await.len(),
                "Removed timed-out clients"
            );
        }

        removed_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_server_loop_creation() {
        let world = World::new();
        let server_loop = ServerLoop::new(world);

        assert_eq!(server_loop.tick(), 0);
        assert_eq!(server_loop.client_count().await, 0);
    }

    #[tokio::test]
    async fn test_performance_stats_initial() {
        let world = World::new();
        let server_loop = ServerLoop::new(world);
        let stats = server_loop.stats();

        assert_eq!(stats.tick, 0);
        assert_eq!(stats.client_count, 0);
        assert_eq!(stats.messages_per_second, 0);
    }

    #[tokio::test]
    async fn test_server_loop_tick_increment() {
        let world = World::new();
        let mut server_loop = ServerLoop::new(world);

        let mut tick_count = 0;

        // Run for a few ticks
        tokio::select! {
            _ = server_loop.run(|_world, _dt| {
                tick_count += 1;
            }) => {},
            _ = tokio::time::sleep(Duration::from_millis(100)) => {
                // Should have run ~6 ticks in 100ms at 60 TPS
                // Allow wider range to account for system load and timing variance
                assert!(
                    server_loop.tick() >= 3 && server_loop.tick() <= 10,
                    "Expected 3-10 ticks in 100ms at 60 TPS, got {}",
                    server_loop.tick()
                );
            }
        }
    }

    #[tokio::test]
    async fn test_client_timeout_detection() {
        let world = World::new();
        let mut server_loop = ServerLoop::new(world);

        // Create a test server to get a valid connection
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        // Spawn a task to accept the connection
        let accept_handle = tokio::spawn(async move { listener.accept().await.unwrap() });

        // Connect a client
        let client_stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        let (server_stream, _) = accept_handle.await.unwrap();

        let (message_tx, _message_rx) = mpsc::unbounded_channel();
        let client_id = 999u64;
        let old_timestamp = Instant::now() - Duration::from_secs(35); // 35 seconds ago (exceeds 30s timeout)

        {
            let mut clients = server_loop.clients.lock().await;
            clients.insert(
                client_id,
                ClientState {
                    connection: Arc::new(TcpConnection::new(server_stream).unwrap()),
                    player_entity: None,
                    address: addr,
                    name: "TestClient".to_string(),
                    connected_at: old_timestamp,
                    last_message_at: old_timestamp,
                    message_tx,
                },
            );
        }

        // Verify client was added
        assert_eq!(server_loop.client_count().await, 1);

        // Check for timeouts
        let removed = server_loop.check_timeouts().await;

        // Verify client was removed due to timeout
        assert_eq!(removed, 1, "Expected 1 client to be removed due to timeout");
        assert_eq!(
            server_loop.client_count().await,
            0,
            "Expected 0 clients remaining after timeout"
        );

        // Cleanup
        drop(client_stream);
    }

    #[tokio::test]
    async fn test_client_timeout_not_triggered_for_active_clients() {
        let world = World::new();
        let mut server_loop = ServerLoop::new(world);

        // Create a test server to get a valid connection
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        // Spawn a task to accept the connection
        let accept_handle = tokio::spawn(async move { listener.accept().await.unwrap() });

        // Connect a client
        let client_stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        let (server_stream, _) = accept_handle.await.unwrap();

        let (message_tx, _message_rx) = mpsc::unbounded_channel();
        let client_id = 998u64;
        let recent_timestamp = Instant::now() - Duration::from_secs(10); // Only 10 seconds ago (within 30s timeout)

        {
            let mut clients = server_loop.clients.lock().await;
            clients.insert(
                client_id,
                ClientState {
                    connection: Arc::new(TcpConnection::new(server_stream).unwrap()),
                    player_entity: None,
                    address: addr,
                    name: "ActiveClient".to_string(),
                    connected_at: recent_timestamp,
                    last_message_at: recent_timestamp,
                    message_tx,
                },
            );
        }

        // Verify client was added
        assert_eq!(server_loop.client_count().await, 1);

        // Check for timeouts
        let removed = server_loop.check_timeouts().await;

        // Verify client was NOT removed (still active)
        assert_eq!(removed, 0, "Expected 0 clients to be removed (client is active)");
        assert_eq!(
            server_loop.client_count().await,
            1,
            "Expected 1 client remaining (active client should not timeout)"
        );

        // Cleanup
        drop(client_stream);
    }
}
