//! MMORPG Server
//!
//! Authoritative game server handling player connections, game logic, and world state.

use anyhow::Result;
use engine_core::ecs::{Entity, World};
use engine_networking::{TcpConnection, TcpServer};
use mmorpg_shared::{config, ClientMessage, Player, PlayerState, ServerMessage};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};

/// Game server state
struct GameServer {
    /// ECS world
    world: Arc<RwLock<World>>,
    /// Connected clients (addr -> entity)
    clients: Arc<Mutex<HashMap<SocketAddr, Entity>>>,
    /// Player data (entity -> player info)
    players: Arc<Mutex<HashMap<Entity, Player>>>,
}

impl GameServer {
    fn new() -> Self {
        Self {
            world: Arc::new(RwLock::new(World::new())),
            clients: Arc::new(Mutex::new(HashMap::new())),
            players: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Spawn a new player
    async fn spawn_player(&self, name: String) -> Entity {
        let mut world = self.world.write().await;
        let entity = world.spawn();

        let player = Player::new(entity, name, config::SPAWN_X, config::SPAWN_Y);

        let mut players = self.players.lock().await;
        players.insert(entity, player);

        info!(entity = ?entity, "Player spawned");
        entity
    }

    /// Remove a player
    async fn despawn_player(&self, entity: Entity) {
        let mut world = self.world.write().await;
        world.despawn(entity);

        let mut players = self.players.lock().await;
        players.remove(&entity);

        info!(entity = ?entity, "Player despawned");
    }

    /// Update player position
    async fn update_player_position(&self, entity: Entity, x: f32, y: f32) {
        let mut players = self.players.lock().await;
        if let Some(player) = players.get_mut(&entity) {
            // Clamp to world bounds
            player.position.x = x.clamp(0.0, config::WORLD_WIDTH);
            player.position.y = y.clamp(0.0, config::WORLD_HEIGHT);
        }
    }

    /// Get all player states for broadcasting
    async fn get_player_states(&self) -> Vec<PlayerState> {
        let players = self.players.lock().await;
        players
            .values()
            .map(|p| PlayerState {
                entity: p.entity,
                name: p.name.clone(),
                x: p.position.x,
                y: p.position.y,
            })
            .collect()
    }

    /// Broadcast message to all clients except sender
    async fn broadcast_except(
        &self,
        sender: SocketAddr,
        msg: ServerMessage,
        connections: &HashMap<SocketAddr, Arc<TcpConnection>>,
    ) {
        let data = bincode::serialize(&msg).unwrap();

        for (addr, conn) in connections.iter() {
            if *addr != sender {
                if let Err(e) = conn.send(&data).await {
                    error!(addr = ?addr, error = ?e, "Failed to broadcast message");
                }
            }
        }
    }

    /// Broadcast message to all clients
    async fn broadcast_all(
        &self,
        msg: ServerMessage,
        connections: &HashMap<SocketAddr, Arc<TcpConnection>>,
    ) {
        let data = bincode::serialize(&msg).unwrap();

        for (addr, conn) in connections.iter() {
            if let Err(e) = conn.send(&data).await {
                error!(addr = ?addr, error = ?e, "Failed to broadcast message");
            }
        }
    }
}

/// Handle a single client connection
async fn handle_client(
    server: Arc<GameServer>,
    connection: Arc<TcpConnection>,
    connections: Arc<Mutex<HashMap<SocketAddr, Arc<TcpConnection>>>>,
) {
    let addr = connection.peer_addr();
    info!(client = ?addr, "Client connected");

    loop {
        match connection.recv().await {
            Ok(data) => {
                // Deserialize client message
                let msg: ClientMessage = match bincode::deserialize(&data) {
                    Ok(msg) => msg,
                    Err(e) => {
                        error!(error = ?e, "Failed to deserialize client message");
                        continue;
                    }
                };

                info!(client = ?addr, message = ?msg, "Received message");

                match msg {
                    ClientMessage::Join { player_name } => {
                        // Spawn player entity
                        let entity = server.spawn_player(player_name.clone()).await;

                        // Register client
                        server.clients.lock().await.insert(addr, entity);

                        // Send welcome message
                        let welcome = ServerMessage::Welcome {
                            player_entity: entity,
                            player_name: player_name.clone(),
                        };
                        let data = bincode::serialize(&welcome).unwrap();
                        if let Err(e) = connection.send(&data).await {
                            error!(error = ?e, "Failed to send welcome message");
                        }

                        // Send full state snapshot
                        let players = server.get_player_states().await;
                        let snapshot = ServerMessage::StateSnapshot { players };
                        let data = bincode::serialize(&snapshot).unwrap();
                        if let Err(e) = connection.send(&data).await {
                            error!(error = ?e, "Failed to send state snapshot");
                        }

                        // Broadcast player joined to others
                        let joined = ServerMessage::PlayerJoined {
                            entity,
                            name: player_name,
                            x: config::SPAWN_X,
                            y: config::SPAWN_Y,
                        };
                        let conns = connections.lock().await;
                        server.broadcast_except(addr, joined, &conns).await;

                        info!(client = ?addr, entity = ?entity, "Player joined game");
                    }

                    ClientMessage::Move { x, y } => {
                        // Get player entity
                        let entity = match server.clients.lock().await.get(&addr) {
                            Some(e) => *e,
                            None => {
                                warn!(client = ?addr, "Move from unregistered client");
                                continue;
                            }
                        };

                        // Update position
                        server.update_player_position(entity, x, y).await;

                        // Broadcast move to all clients
                        let moved = ServerMessage::PlayerMoved { entity, x, y };
                        let conns = connections.lock().await;
                        server.broadcast_all(moved, &conns).await;

                        info!(client = ?addr, entity = ?entity, x, y, "Player moved");
                    }

                    ClientMessage::Disconnect => {
                        info!(client = ?addr, "Client requested disconnect");
                        break;
                    }
                }
            }
            Err(engine_networking::TcpError::ConnectionClosed) => {
                info!(client = ?addr, "Client disconnected");
                break;
            }
            Err(e) => {
                error!(client = ?addr, error = ?e, "Error receiving from client");
                break;
            }
        }
    }

    // Cleanup on disconnect
    if let Some(entity) = server.clients.lock().await.remove(&addr) {
        server.despawn_player(entity).await;

        // Broadcast player left
        let left = ServerMessage::PlayerLeft { entity };
        let conns = connections.lock().await;
        server.broadcast_all(left, &conns).await;

        info!(client = ?addr, entity = ?entity, "Player left game");
    }

    // Remove connection
    connections.lock().await.remove(&addr);
}

/// Server tick loop (60 Hz)
async fn tick_loop(_server: Arc<GameServer>) {
    let mut ticker = interval(Duration::from_millis(config::TICK_DURATION_MS));

    loop {
        ticker.tick().await;

        // Game logic updates go here
        // For this demo, we just have player movement handled by client inputs
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    info!("Starting MMORPG server");

    // Create game server
    let server = Arc::new(GameServer::new());

    // Start tick loop
    let tick_server = server.clone();
    tokio::spawn(async move {
        tick_loop(tick_server).await;
    });

    // Bind TCP server
    let tcp_server = TcpServer::bind("127.0.0.1:7777").await?;
    let addr = tcp_server.local_addr()?;
    info!(addr = ?addr, "Server listening");

    // Track all connections
    let connections: Arc<Mutex<HashMap<SocketAddr, Arc<TcpConnection>>>> =
        Arc::new(Mutex::new(HashMap::new()));

    // Accept connections
    loop {
        match tcp_server.accept().await {
            Ok(conn) => {
                let addr = conn.peer_addr();
                let conn = Arc::new(conn);

                connections.lock().await.insert(addr, conn.clone());

                let server = server.clone();
                let connections = connections.clone();

                tokio::spawn(async move {
                    handle_client(server, conn, connections).await;
                });
            }
            Err(e) => {
                error!(error = ?e, "Failed to accept connection");
            }
        }
    }
}
