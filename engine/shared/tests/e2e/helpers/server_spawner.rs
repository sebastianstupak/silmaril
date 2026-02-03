//! Test server spawner helper
//!
//! Provides utilities for spawning test servers in background tasks.

use engine_core::ecs::World;
use engine_networking::{ServerLoop, TcpServer};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info};

/// Handle to a spawned test server
pub struct TestServerHandle {
    /// Server loop (for accessing state)
    server_loop: Arc<Mutex<ServerLoop>>,
    /// Shutdown signal
    shutdown: Arc<AtomicBool>,
    /// Server address
    address: String,
}

impl TestServerHandle {
    /// Get the number of connected clients
    pub async fn client_count(&self) -> usize {
        self.server_loop.lock().await.client_count().await
    }

    /// Get current tick number
    pub async fn tick(&self) -> u64 {
        self.server_loop.lock().await.tick()
    }

    /// Get server address
    pub fn address(&self) -> &str {
        &self.address
    }

    /// Shutdown the server
    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::SeqCst);
    }
}

impl Drop for TestServerHandle {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Spawn a test server on the given address
///
/// Returns a handle to the running server. The server will run in the background
/// until the handle is dropped or `shutdown()` is called.
pub async fn spawn_test_server(addr: &str) -> Result<TestServerHandle, String> {
    // Create TCP server
    let tcp_server = TcpServer::bind(addr)
        .await
        .map_err(|e| format!("Failed to bind server: {}", e))?;

    let actual_addr = tcp_server
        .local_addr()
        .map_err(|e| format!("Failed to get local addr: {}", e))?
        .to_string();

    info!(address = %actual_addr, "Test server bound");

    // Create server loop
    let world = World::new();
    let server_loop = ServerLoop::new(world);
    let server_loop = Arc::new(Mutex::new(server_loop));
    let server_loop_clone = server_loop.clone();

    // Start accepting connections
    server_loop.lock().await.start_accepting(tcp_server);

    // Create shutdown signal
    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_clone = shutdown.clone();

    // Spawn server loop task
    tokio::spawn(async move {
        let mut server_loop = server_loop_clone.lock().await;

        tokio::select! {
            result = server_loop.run(|_world, _dt| {
                // Empty game logic for E2E tests
            }) => {
                if let Err(e) = result {
                    error!(error = ?e, "Server loop error");
                }
            }
            _ = async {
                loop {
                    if shutdown_clone.load(Ordering::SeqCst) {
                        break;
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            } => {
                info!("Server shutting down");
            }
        }
    });

    // Give server time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    Ok(TestServerHandle { server_loop, shutdown, address: actual_addr })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_spawn_server() {
        let server = spawn_test_server("127.0.0.1:0").await.unwrap();
        assert_eq!(server.client_count().await, 0);
        server.shutdown();
    }

    #[tokio::test]
    async fn test_server_runs_ticks() {
        let server = spawn_test_server("127.0.0.1:0").await.unwrap();

        // Wait for a few ticks
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let tick = server.tick().await;
        assert!(tick > 0, "Server should have processed ticks");

        server.shutdown();
    }
}
