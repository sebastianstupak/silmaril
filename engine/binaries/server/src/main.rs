use engine_core::ecs::World;
use engine_networking::{ServerLoop, TcpServer};
use engine_observability::metrics::start_metrics_server;
use std::env;
use tokio::signal;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// TCP bind address
    pub tcp_bind_addr: String,
    /// UDP bind address (for future use)
    pub udp_bind_addr: String,
    /// Maximum clients
    pub max_clients: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            tcp_bind_addr: "0.0.0.0:7777".to_string(),
            udp_bind_addr: "0.0.0.0:7778".to_string(),
            max_clients: 1000,
        }
    }
}

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,engine_networking=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Silmaril Server starting...");

    // Start Prometheus metrics server
    let metrics_port = env::var("METRICS_PORT").unwrap_or_else(|_| "9090".to_string());
    let metrics_addr = format!("0.0.0.0:{}", metrics_port);

    // Clone for the async task
    let metrics_addr_clone = metrics_addr.clone();
    let metrics_handle = tokio::spawn(async move {
        if let Err(e) = start_metrics_server(&metrics_addr_clone).await {
            error!(error = ?e, "Prometheus metrics server failed");
        }
    });

    info!(
        addr = %metrics_addr,
        "Prometheus metrics available at http://{}/metrics",
        metrics_addr
    );

    // Load configuration
    let config = ServerConfig::default();
    info!(
        tcp_addr = %config.tcp_bind_addr,
        udp_addr = %config.udp_bind_addr,
        max_clients = config.max_clients,
        "Server configuration loaded"
    );

    // Initialize game world
    let world = World::new();
    info!("Game world initialized");

    // Create TCP server
    let tcp_server = match TcpServer::bind(&config.tcp_bind_addr).await {
        Ok(server) => {
            info!(
                tcp_addr = %server.local_addr().unwrap(),
                "TCP server listening"
            );
            server
        }
        Err(e) => {
            error!(error = ?e, "Failed to bind TCP server");
            metrics_handle.abort();
            return;
        }
    };

    // Create server loop
    let mut server_loop = ServerLoop::new(world);
    info!("Server loop initialized");

    // Start accepting client connections
    server_loop.start_accepting(tcp_server).await;
    info!("Server accepting connections");

    // Setup graceful shutdown
    let shutdown = tokio::spawn(async move {
        signal::ctrl_c().await.ok();
        info!("Shutdown signal received");
    });

    // Run server loop with game logic callback
    info!("Server ready at 60 TPS");

    tokio::select! {
        result = server_loop.run(|_world, _dt| {
            // Game logic goes here
            // For now, just a stub that does nothing
            // In the future, this will:
            // - Update physics
            // - Process AI
            // - Handle game events
            // - Update entity positions
        }) => {
            if let Err(e) = result {
                error!(error = ?e, "Server loop error");
            }
        }
        _ = shutdown => {
            info!("Server shutting down...");
        }
    }

    // Stop metrics server
    metrics_handle.abort();

    info!("Server shutdown complete");
}
