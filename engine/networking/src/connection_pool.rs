//! Connection Pool and Reconnection Management
//!
//! Provides connection pooling, automatic reconnection, and state recovery
//! for production-grade reliability at scale.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Connection state for tracking and recovery
#[derive(Debug, Clone)]
pub struct ConnectionState {
    /// Client ID
    pub client_id: u64,
    /// Last known sequence number
    pub last_sequence: u32,
    /// Last activity timestamp
    pub last_activity: Instant,
    /// Connection address
    pub addr: SocketAddr,
    /// Custom state data (serialized)
    pub state_data: Vec<u8>,
}

/// Connection pool configuration
#[derive(Debug, Clone)]
pub struct ConnectionPoolConfig {
    /// Maximum connections per pool
    pub max_connections: usize,
    /// Idle timeout before cleanup
    pub idle_timeout: Duration,
    /// Reconnection timeout
    pub reconnect_timeout: Duration,
    /// Maximum reconnection attempts
    pub max_reconnect_attempts: u32,
}

impl Default for ConnectionPoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 10_000,
            idle_timeout: Duration::from_secs(300), // 5 minutes
            reconnect_timeout: Duration::from_secs(30),
            max_reconnect_attempts: 3,
        }
    }
}

/// Connection pool for managing client connections at scale
pub struct ConnectionPool {
    config: ConnectionPoolConfig,
    connections: Arc<RwLock<HashMap<u64, ConnectionState>>>,
    disconnected: Arc<RwLock<HashMap<u64, (ConnectionState, Instant)>>>,
}

impl ConnectionPool {
    /// Create a new connection pool
    pub fn new(config: ConnectionPoolConfig) -> Self {
        info!(
            max_connections = config.max_connections,
            idle_timeout_secs = config.idle_timeout.as_secs(),
            "Creating connection pool"
        );

        Self {
            config,
            connections: Arc::new(RwLock::new(HashMap::new())),
            disconnected: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a new connection
    pub async fn register(&self, client_id: u64, addr: SocketAddr) -> Result<(), String> {
        let mut connections = self.connections.write().await;

        if connections.len() >= self.config.max_connections {
            return Err(format!(
                "Connection pool full ({}/{})",
                connections.len(),
                self.config.max_connections
            ));
        }

        let state = ConnectionState {
            client_id,
            last_sequence: 0,
            last_activity: Instant::now(),
            addr,
            state_data: Vec::new(),
        };

        connections.insert(client_id, state);

        info!(client_id, %addr, "Connection registered");
        Ok(())
    }

    /// Update connection activity
    pub async fn update_activity(&self, client_id: u64, sequence: u32) {
        if let Some(state) = self.connections.write().await.get_mut(&client_id) {
            state.last_activity = Instant::now();
            state.last_sequence = sequence;
        }
    }

    /// Save connection state for recovery
    pub async fn save_state(&self, client_id: u64, state_data: Vec<u8>) {
        if let Some(state) = self.connections.write().await.get_mut(&client_id) {
            state.state_data = state_data;
            debug!(client_id, state_size = state.state_data.len(), "Connection state saved");
        }
    }

    /// Disconnect a client (but keep state for reconnection)
    pub async fn disconnect(&self, client_id: u64) {
        let mut connections = self.connections.write().await;

        if let Some(state) = connections.remove(&client_id) {
            let mut disconnected = self.disconnected.write().await;
            disconnected.insert(client_id, (state, Instant::now()));

            info!(client_id, "Connection disconnected, state preserved for reconnection");
        }
    }

    /// Attempt to reconnect a client
    pub async fn reconnect(
        &self,
        client_id: u64,
        addr: SocketAddr,
    ) -> Result<ConnectionState, String> {
        let mut disconnected = self.disconnected.write().await;

        if let Some((mut state, disconnect_time)) = disconnected.remove(&client_id) {
            let elapsed = disconnect_time.elapsed();

            if elapsed > self.config.reconnect_timeout {
                return Err(format!(
                    "Reconnection timeout ({:?} > {:?})",
                    elapsed, self.config.reconnect_timeout
                ));
            }

            // Update address and activity
            state.addr = addr;
            state.last_activity = Instant::now();

            // Restore connection
            let mut connections = self.connections.write().await;
            connections.insert(client_id, state.clone());

            info!(
                client_id,
                %addr,
                reconnect_time_ms = elapsed.as_millis(),
                "Client reconnected successfully"
            );

            Ok(state)
        } else {
            Err(format!("No disconnected state found for client {}", client_id))
        }
    }

    /// Cleanup idle and expired connections
    pub async fn cleanup(&self) -> (usize, usize) {
        let now = Instant::now();
        let mut removed_active = 0;
        let mut removed_disconnected = 0;

        // Cleanup idle active connections
        let mut connections = self.connections.write().await;
        connections.retain(|client_id, state| {
            let idle_time = now.duration_since(state.last_activity);
            if idle_time > self.config.idle_timeout {
                warn!(client_id, idle_time_secs = idle_time.as_secs(), "Removing idle connection");
                removed_active += 1;
                false
            } else {
                true
            }
        });

        // Cleanup expired disconnected connections
        let mut disconnected = self.disconnected.write().await;
        disconnected.retain(|client_id, (_, disconnect_time)| {
            let elapsed = now.duration_since(*disconnect_time);
            if elapsed > self.config.reconnect_timeout {
                warn!(client_id, "Removing expired disconnected state");
                removed_disconnected += 1;
                false
            } else {
                true
            }
        });

        if removed_active > 0 || removed_disconnected > 0 {
            info!(removed_active, removed_disconnected, "Connection cleanup completed");
        }

        (removed_active, removed_disconnected)
    }

    /// Get connection count
    pub async fn connection_count(&self) -> usize {
        self.connections.read().await.len()
    }

    /// Get disconnected count
    pub async fn disconnected_count(&self) -> usize {
        self.disconnected.read().await.len()
    }

    /// Get connection state
    pub async fn get_state(&self, client_id: u64) -> Option<ConnectionState> {
        self.connections.read().await.get(&client_id).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connection_pool_register() {
        let pool = ConnectionPool::new(ConnectionPoolConfig::default());
        let addr = "127.0.0.1:12345".parse().unwrap();

        assert!(pool.register(1, addr).await.is_ok());
        assert_eq!(pool.connection_count().await, 1);
    }

    #[tokio::test]
    async fn test_connection_pool_max_connections() {
        let config = ConnectionPoolConfig { max_connections: 2, ..Default::default() };
        let pool = ConnectionPool::new(config);
        let addr = "127.0.0.1:12345".parse().unwrap();

        assert!(pool.register(1, addr).await.is_ok());
        assert!(pool.register(2, addr).await.is_ok());
        assert!(pool.register(3, addr).await.is_err()); // Should fail - pool full
    }

    #[tokio::test]
    async fn test_reconnection() {
        let config = ConnectionPoolConfig {
            reconnect_timeout: Duration::from_secs(10),
            ..Default::default()
        };
        let pool = ConnectionPool::new(config);
        let addr = "127.0.0.1:12345".parse().unwrap();

        // Register and disconnect
        pool.register(1, addr).await.unwrap();
        pool.save_state(1, vec![1, 2, 3, 4]).await;
        pool.disconnect(1).await;

        assert_eq!(pool.connection_count().await, 0);
        assert_eq!(pool.disconnected_count().await, 1);

        // Reconnect
        let state = pool.reconnect(1, addr).await.unwrap();
        assert_eq!(state.state_data, vec![1, 2, 3, 4]);
        assert_eq!(pool.connection_count().await, 1);
        assert_eq!(pool.disconnected_count().await, 0);
    }

    #[tokio::test]
    async fn test_reconnection_timeout() {
        let config = ConnectionPoolConfig {
            reconnect_timeout: Duration::from_millis(10),
            ..Default::default()
        };
        let pool = ConnectionPool::new(config);
        let addr = "127.0.0.1:12345".parse().unwrap();

        pool.register(1, addr).await.unwrap();
        pool.disconnect(1).await;

        // Wait for timeout
        tokio::time::sleep(Duration::from_millis(20)).await;

        // Reconnection should fail
        assert!(pool.reconnect(1, addr).await.is_err());
    }

    #[tokio::test]
    async fn test_cleanup_idle() {
        let config =
            ConnectionPoolConfig { idle_timeout: Duration::from_millis(10), ..Default::default() };
        let pool = ConnectionPool::new(config);
        let addr = "127.0.0.1:12345".parse().unwrap();

        pool.register(1, addr).await.unwrap();

        // Wait for idle timeout
        tokio::time::sleep(Duration::from_millis(20)).await;

        let (removed, _) = pool.cleanup().await;
        assert_eq!(removed, 1);
        assert_eq!(pool.connection_count().await, 0);
    }

    #[tokio::test]
    async fn test_activity_update() {
        let config =
            ConnectionPoolConfig { idle_timeout: Duration::from_millis(50), ..Default::default() };
        let pool = ConnectionPool::new(config);
        let addr = "127.0.0.1:12345".parse().unwrap();

        pool.register(1, addr).await.unwrap();

        // Update activity before timeout
        tokio::time::sleep(Duration::from_millis(20)).await;
        pool.update_activity(1, 100).await;

        tokio::time::sleep(Duration::from_millis(20)).await;
        pool.update_activity(1, 200).await;

        // Should not be cleaned up
        let (removed, _) = pool.cleanup().await;
        assert_eq!(removed, 0);
        assert_eq!(pool.connection_count().await, 1);

        // Verify sequence was updated
        let state = pool.get_state(1).await.unwrap();
        assert_eq!(state.last_sequence, 200);
    }
}
