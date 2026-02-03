//! rotating-cube - Server Binary
//!
//! Server-authoritative game logic

use tracing::{info, Level};
use tracing_subscriber;

fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("rotating-cube server starting...");

    // TODO: Initialize game server
    // - Load server config
    // - Initialize ECS world
    // - Start network server
    // - Run game loop (60 TPS)

    info!("rotating-cube server running on 0.0.0.0:7777");

    // Keep server running
    std::thread::park();

    Ok(())
}
