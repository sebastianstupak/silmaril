use anyhow::Result;
use tokio::signal;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,agent_game_engine=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Agent Game Engine Server starting...");

    // TODO: Load configuration
    // let config = ServerConfig::from_file("server_config.toml")?;
    // tracing::info!("Configuration loaded: {:?}", config);

    // TODO: Initialize server
    // let mut server = GameServer::new(config).await?;
    // tracing::info!("Server initialized");

    // TODO: Spawn server task
    // let server_task = tokio::spawn(async move {
    //     server.run().await
    // });

    // TODO: Setup graceful shutdown
    tracing::info!("Server ready, waiting for Ctrl+C to shutdown...");
    signal::ctrl_c().await?;
    tracing::info!("Shutdown signal received");

    // TODO: Cleanup
    // server_task.abort();
    // tracing::info!("Server task stopped");

    tracing::info!("Server shutdown complete");
    tracing::info!("TODO: Implement server tick loop and networking");

    Ok(())
}

// TODO: Server configuration
// #[derive(Debug, Clone)]
// pub struct ServerConfig {
//     pub tcp_bind_addr: String,
//     pub udp_bind_addr: String,
//     pub ticks_per_second: u32,
//     pub max_clients: usize,
// }
//
// impl Default for ServerConfig {
//     fn default() -> Self {
//         Self {
//             tcp_bind_addr: "0.0.0.0:7777".to_string(),
//             udp_bind_addr: "0.0.0.0:7778".to_string(),
//             ticks_per_second: 60,
//             max_clients: 1000,
//         }
//     }
// }
