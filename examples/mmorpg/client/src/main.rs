//! MMORPG Client
//!
//! Handles connection to server, input, and rendering (terminal-based for CI compatibility).

use anyhow::Result;
use engine_core::ecs::Entity;
use engine_networking::TcpClient;
use mmorpg_shared::{config, ClientMessage, PlayerState, ServerMessage};
use std::collections::HashMap;
use std::io::{self, Write};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};

/// Client state
struct GameClient {
    /// TCP connection to server
    connection: TcpClient,
    /// Our player entity
    player_entity: Option<Entity>,
    /// Our player name
    player_name: String,
    /// Other players (entity -> state)
    players: Arc<Mutex<HashMap<Entity, PlayerState>>>,
    /// Our position
    position: Arc<Mutex<(f32, f32)>>,
}

impl GameClient {
    async fn new(server_addr: &str, player_name: String) -> Result<Self> {
        let connection = TcpClient::connect(server_addr).await?;
        info!("Connected to server");

        Ok(Self {
            connection,
            player_entity: None,
            player_name,
            players: Arc::new(Mutex::new(HashMap::new())),
            position: Arc::new(Mutex::new((config::SPAWN_X, config::SPAWN_Y))),
        })
    }

    /// Send join message
    async fn join(&self) -> Result<()> {
        let msg = ClientMessage::Join { player_name: self.player_name.clone() };
        let data = bincode::serialize(&msg)?;
        self.connection.send(&data).await?;
        info!("Sent join request");
        Ok(())
    }

    /// Send move command
    async fn move_to(&self, x: f32, y: f32) -> Result<()> {
        let msg = ClientMessage::Move { x, y };
        let data = bincode::serialize(&msg)?;
        self.connection.send(&data).await?;

        // Update local position
        *self.position.lock().await = (x, y);

        Ok(())
    }

    /// Disconnect from server
    async fn disconnect(&self) -> Result<()> {
        let msg = ClientMessage::Disconnect;
        let data = bincode::serialize(&msg)?;
        self.connection.send(&data).await?;
        info!("Sent disconnect");
        Ok(())
    }

    /// Render game state (ASCII art)
    async fn render(&self) {
        let players = self.players.lock().await;
        let (our_x, our_y) = *self.position.lock().await;

        // Clear screen
        print!("\x1B[2J\x1B[1;1H");

        // Draw border
        println!("=== MMORPG Demo ===");
        println!("Player: {} at ({:.1}, {:.1})", self.player_name, our_x, our_y);
        println!("Players in game: {}", players.len());
        println!();

        // Draw players
        for (entity, player) in players.iter() {
            let is_us = Some(*entity) == self.player_entity;
            let marker = if is_us { "[YOU]" } else { "" };
            println!(
                "  {:?} {} at ({:.1}, {:.1}) {}",
                entity, player.name, player.x, player.y, marker
            );
        }

        println!();
        println!("Commands:");
        println!("  move <x> <y> - Move to position");
        println!("  quit - Disconnect and exit");
        println!();
        print!("> ");
        io::stdout().flush().ok();
    }
}

/// Handle server messages
async fn handle_server_messages(client: Arc<Mutex<GameClient>>) {
    loop {
        let data = {
            let c = client.lock().await;
            match c.connection.recv().await {
                Ok(data) => data,
                Err(engine_networking::TcpError::ConnectionClosed) => {
                    info!("Server closed connection");
                    break;
                }
                Err(e) => {
                    error!(error = ?e, "Error receiving from server");
                    break;
                }
            }
        };

        // Deserialize server message
        let msg: ServerMessage = match bincode::deserialize(&data) {
            Ok(msg) => msg,
            Err(e) => {
                error!(error = ?e, "Failed to deserialize server message");
                continue;
            }
        };

        // Handle message
        let mut c = client.lock().await;
        match msg {
            ServerMessage::Welcome { player_entity, player_name } => {
                c.player_entity = Some(player_entity);
                info!(entity = ?player_entity, name = %player_name, "Joined game");

                // Add ourselves to player list
                let (x, y) = *c.position.lock().await;
                c.players.lock().await.insert(
                    player_entity,
                    PlayerState { entity: player_entity, name: player_name.clone(), x, y },
                );
            }

            ServerMessage::StateSnapshot { players } => {
                let mut player_map = c.players.lock().await;
                for player in players {
                    player_map.insert(player.entity, player);
                }
                info!(count = player_map.len(), "Received state snapshot");
            }

            ServerMessage::PlayerJoined { entity, name, x, y } => {
                c.players
                    .lock()
                    .await
                    .insert(entity, PlayerState { entity, name: name.clone(), x, y });
                info!(entity = ?entity, name = %name, "Player joined");
            }

            ServerMessage::PlayerMoved { entity, x, y } => {
                let mut players = c.players.lock().await;
                if let Some(player) = players.get_mut(&entity) {
                    player.x = x;
                    player.y = y;
                }

                // If it's us, update our position
                if Some(entity) == c.player_entity {
                    *c.position.lock().await = (x, y);
                }
            }

            ServerMessage::PlayerLeft { entity } => {
                c.players.lock().await.remove(&entity);
                info!(entity = ?entity, "Player left");
            }
        }
    }
}

/// Render loop (10 Hz)
async fn render_loop(client: Arc<Mutex<GameClient>>) {
    let mut ticker = interval(Duration::from_millis(100));

    loop {
        ticker.tick().await;

        let c = client.lock().await;
        c.render().await;
    }
}

/// Handle user input
async fn input_loop(client: Arc<Mutex<GameClient>>) -> Result<()> {
    let stdin = io::stdin();
    let mut buffer = String::new();

    loop {
        buffer.clear();
        stdin.read_line(&mut buffer)?;

        let input = buffer.trim();
        let parts: Vec<&str> = input.split_whitespace().collect();

        if parts.is_empty() {
            continue;
        }

        match parts[0] {
            "move" if parts.len() == 3 => {
                let x: f32 = match parts[1].parse() {
                    Ok(v) => v,
                    Err(_) => {
                        warn!("Invalid x coordinate");
                        continue;
                    }
                };
                let y: f32 = match parts[2].parse() {
                    Ok(v) => v,
                    Err(_) => {
                        warn!("Invalid y coordinate");
                        continue;
                    }
                };

                let c = client.lock().await;
                if let Err(e) = c.move_to(x, y).await {
                    error!(error = ?e, "Failed to send move command");
                }
            }
            "quit" => {
                let c = client.lock().await;
                c.disconnect().await?;
                info!("Disconnected");
                break;
            }
            _ => {
                warn!("Unknown command: {}", parts[0]);
            }
        }
    }

    Ok(())
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

    info!("Starting MMORPG client");

    // Get player name from args or use default
    let args: Vec<String> = std::env::args().collect();
    let player_name = if args.len() > 1 { args[1].clone() } else { "Player".to_string() };

    // Get server address from env or use default
    let server_addr =
        std::env::var("SERVER_ADDR").unwrap_or_else(|_| "127.0.0.1:7777".to_string());

    // Connect to server
    let client = Arc::new(Mutex::new(GameClient::new(&server_addr, player_name).await?));

    // Send join message
    {
        let c = client.lock().await;
        c.join().await?;
    }

    // Spawn message handler
    let msg_client = client.clone();
    tokio::spawn(async move {
        handle_server_messages(msg_client).await;
    });

    // Spawn render loop
    let render_client = client.clone();
    tokio::spawn(async move {
        render_loop(render_client).await;
    });

    // Wait a bit for initial state
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Handle user input (blocking)
    input_loop(client.clone()).await?;

    Ok(())
}
