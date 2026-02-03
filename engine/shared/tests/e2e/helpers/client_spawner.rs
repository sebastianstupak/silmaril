//! Test client spawner helper
//!
//! Provides utilities for spawning test clients that connect to servers.

use engine_core::ecs::Entity;
use engine_networking::{
    deserialize_server_message, serialize_client_message, ClientMessage, FramedMessage,
    SerializationFormat, ServerMessage, TcpClient, PROTOCOL_VERSION,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

/// Test client connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientState {
    /// Disconnected
    Disconnected,
    /// Connecting
    Connecting,
    /// Connected and handshake completed
    Connected,
    /// Error state
    Error,
}

/// Handle to a spawned test client
pub struct TestClientHandle {
    /// TCP client
    client: Arc<Mutex<Option<TcpClient>>>,
    /// Client state
    state: Arc<Mutex<ClientState>>,
    /// Player entity (after handshake)
    player_entity: Arc<Mutex<Option<Entity>>>,
    /// Shutdown signal
    shutdown: Arc<AtomicBool>,
    /// Server address
    server_address: String,
}

impl TestClientHandle {
    /// Check if client is connected
    pub async fn is_connected(&self) -> bool {
        matches!(*self.state.lock().await, ClientState::Connected)
    }

    /// Get client state
    pub async fn state(&self) -> ClientState {
        *self.state.lock().await
    }

    /// Get player entity (if handshake completed)
    pub async fn player_entity(&self) -> Option<Entity> {
        *self.player_entity.lock().await
    }

    /// Send a message to the server
    pub async fn send(&self, message: ClientMessage) -> Result<(), String> {
        let client = self.client.lock().await;
        if let Some(client) = client.as_ref() {
            let framed = serialize_client_message(&message, SerializationFormat::Bincode)
                .map_err(|e| format!("Failed to serialize message: {}", e))?;

            client
                .send(&framed.payload)
                .await
                .map_err(|e| format!("Failed to send message: {}", e))?;

            Ok(())
        } else {
            Err("Client not connected".to_string())
        }
    }

    /// Disconnect from server
    pub async fn disconnect(&self) {
        self.shutdown.store(true, Ordering::SeqCst);

        let mut client = self.client.lock().await;
        if let Some(client) = client.take() {
            if let Err(e) = client.close().await {
                warn!(error = ?e, "Error closing client connection");
            }
        }

        *self.state.lock().await = ClientState::Disconnected;
    }

    /// Get server address
    pub fn server_address(&self) -> &str {
        &self.server_address
    }
}

impl Drop for TestClientHandle {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
    }
}

/// Spawn a test client and connect to the given server
///
/// Returns a handle to the connected client. The client will automatically
/// perform handshake and listen for messages in the background.
pub async fn spawn_test_client(server_addr: &str) -> Result<TestClientHandle, String> {
    spawn_test_client_with_name(server_addr, "TestClient").await
}

/// Spawn a test client with a custom name
pub async fn spawn_test_client_with_name(
    server_addr: &str,
    client_name: &str,
) -> Result<TestClientHandle, String> {
    info!(
        server_addr = %server_addr,
        client_name = %client_name,
        "Connecting test client"
    );

    // Connect to server
    let state = Arc::new(Mutex::new(ClientState::Connecting));
    let tcp_client = TcpClient::connect(server_addr)
        .await
        .map_err(|e| format!("Failed to connect to server: {}", e))?;

    let client = Arc::new(Mutex::new(Some(tcp_client)));
    let player_entity = Arc::new(Mutex::new(None));
    let shutdown = Arc::new(AtomicBool::new(false));

    let handle = TestClientHandle {
        client: client.clone(),
        state: state.clone(),
        player_entity: player_entity.clone(),
        shutdown: shutdown.clone(),
        server_address: server_addr.to_string(),
    };

    // Send handshake
    let handshake = ClientMessage::Handshake {
        version: PROTOCOL_VERSION,
        client_name: client_name.to_string(),
    };

    handle
        .send(handshake)
        .await
        .map_err(|e| format!("Failed to send handshake: {}", e))?;

    // Spawn background task to receive messages
    let client_clone = client.clone();
    let state_clone = state.clone();
    let player_entity_clone = player_entity.clone();
    let shutdown_clone = shutdown.clone();

    tokio::spawn(async move {
        loop {
            // Check shutdown signal
            if shutdown_clone.load(Ordering::SeqCst) {
                debug!("Client receiver task shutting down");
                break;
            }

            // Try to receive message
            let message = {
                let client_lock = client_clone.lock().await;
                if let Some(client) = client_lock.as_ref() {
                    match tokio::time::timeout(
                        tokio::time::Duration::from_millis(100),
                        client.recv(),
                    )
                    .await
                    {
                        Ok(Ok(data)) => Some(data),
                        Ok(Err(e)) => {
                            error!(error = ?e, "Client recv error");
                            *state_clone.lock().await = ClientState::Error;
                            break;
                        }
                        Err(_) => None, // Timeout, continue loop
                    }
                } else {
                    break;
                }
            };

            if let Some(data) = message {
                // Deserialize message
                match FramedMessage::new(data) {
                    Ok(framed) => {
                        match deserialize_server_message(&framed, SerializationFormat::Bincode) {
                            Ok(msg) => {
                                debug!(message = ?msg, "Client received message");

                                // Handle handshake response
                                if let ServerMessage::HandshakeResponse {
                                    version: _,
                                    server_name: _,
                                    player_entity: entity,
                                } = msg
                                {
                                    *player_entity_clone.lock().await = Some(entity);
                                    *state_clone.lock().await = ClientState::Connected;
                                    info!(player_entity = ?entity, "Client handshake complete");
                                }
                            }
                            Err(e) => {
                                error!(error = ?e, "Failed to deserialize server message");
                            }
                        }
                    }
                    Err(e) => {
                        error!(error = ?e, "Failed to parse framed message");
                    }
                }
            }
        }

        debug!("Client receiver task exited");
    });

    // Wait for handshake to complete
    for _ in 0..50 {
        // 5 seconds max
        if *state.lock().await == ClientState::Connected {
            break;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    let final_state = *state.lock().await;
    if final_state != ClientState::Connected {
        return Err(format!("Handshake failed, client state: {:?}", final_state));
    }

    Ok(handle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::e2e::helpers::server_spawner::spawn_test_server;

    #[tokio::test]
    async fn test_spawn_client_connects() {
        let server = spawn_test_server("127.0.0.1:0").await.unwrap();
        let client = spawn_test_client(server.address()).await.unwrap();

        assert!(client.is_connected().await);
        assert!(client.player_entity().await.is_some());

        client.disconnect().await;
        server.shutdown();
    }

    #[tokio::test]
    async fn test_client_with_custom_name() {
        let server = spawn_test_server("127.0.0.1:0").await.unwrap();
        let client = spawn_test_client_with_name(server.address(), "MyCustomClient").await.unwrap();

        assert!(client.is_connected().await);

        client.disconnect().await;
        server.shutdown();
    }
}
