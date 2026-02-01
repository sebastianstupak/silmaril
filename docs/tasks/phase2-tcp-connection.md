# Phase 2.3: TCP Connection (Reliable)

**Status:** ⚪ Not Started
**Estimated Time:** 3-4 days
**Priority:** Critical (foundation for reliable messaging)

---

## 🎯 **Objective**

Implement reliable TCP connection layer for critical game messages (join/leave, state snapshots, chat). Handles connection management, message framing, and error handling.

**TCP Used For:**
- Client join/leave
- Full state snapshots
- Chat messages
- Server commands
- Critical events

---

## 📋 **Detailed Tasks**

### **1. TCP Server** (Day 1-2)

**File:** `engine/networking/src/tcp/server.rs`

```rust
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use std::collections::HashMap;
use std::net::SocketAddr;

/// TCP server for reliable connections
pub struct TcpServer {
    listener: TcpListener,
    connections: HashMap<u64, TcpConnection>,
    next_client_id: u64,
    event_tx: mpsc::UnboundedSender<ServerEvent>,
}

/// Server events
#[derive(Debug)]
pub enum ServerEvent {
    ClientConnected { client_id: u64, addr: SocketAddr },
    ClientDisconnected { client_id: u64 },
    MessageReceived { client_id: u64, data: Vec<u8> },
}

impl TcpServer {
    /// Create TCP server
    pub async fn new(bind_addr: &str) -> Result<Self, NetworkError> {
        let listener = TcpListener::bind(bind_addr)
            .await
            .map_err(|e| NetworkError::TcpBindFailed {
                details: e.to_string(),
            })?;

        tracing::info!("TCP server listening on {}", bind_addr);

        let (event_tx, _event_rx) = mpsc::unbounded_channel();

        Ok(Self {
            listener,
            connections: HashMap::new(),
            next_client_id: 1,
            event_tx,
        })
    }

    /// Run server accept loop
    pub async fn run(&mut self) -> Result<(), NetworkError> {
        loop {
            let (stream, addr) = self
                .listener
                .accept()
                .await
                .map_err(|e| NetworkError::TcpAcceptFailed {
                    details: e.to_string(),
                })?;

            tracing::info!("Client connected from {}", addr);

            let client_id = self.next_client_id;
            self.next_client_id += 1;

            // Spawn connection handler
            let connection = TcpConnection::new(client_id, stream, self.event_tx.clone());
            self.connections.insert(client_id, connection);

            self.event_tx
                .send(ServerEvent::ClientConnected { client_id, addr })
                .ok();
        }
    }

    /// Send message to client
    pub async fn send_to_client(
        &mut self,
        client_id: u64,
        data: Vec<u8>,
    ) -> Result<(), NetworkError> {
        if let Some(connection) = self.connections.get_mut(&client_id) {
            connection.send(data).await?;
        }
        Ok(())
    }

    /// Broadcast message to all clients
    pub async fn broadcast(&mut self, data: Vec<u8>) -> Result<(), NetworkError> {
        for connection in self.connections.values_mut() {
            connection.send(data.clone()).await?;
        }
        Ok(())
    }

    /// Disconnect client
    pub fn disconnect_client(&mut self, client_id: u64) {
        if let Some(mut connection) = self.connections.remove(&client_id) {
            connection.close();
            tracing::info!("Client {} disconnected", client_id);
        }
    }

    /// Get event receiver
    pub fn event_receiver(&self) -> mpsc::UnboundedReceiver<ServerEvent> {
        let (tx, rx) = mpsc::unbounded_channel();
        // Clone the sender and return receiver
        rx
    }
}
```

---

### **2. TCP Connection** (Day 2)

**File:** `engine/networking/src/tcp/connection.rs`

```rust
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;

/// TCP connection handler
pub struct TcpConnection {
    client_id: u64,
    stream: TcpStream,
    event_tx: mpsc::UnboundedSender<ServerEvent>,
    send_queue: mpsc::UnboundedSender<Vec<u8>>,
}

impl TcpConnection {
    /// Create new connection
    pub fn new(
        client_id: u64,
        stream: TcpStream,
        event_tx: mpsc::UnboundedSender<ServerEvent>,
    ) -> Self {
        let (send_tx, send_rx) = mpsc::unbounded_channel();

        // Spawn read task
        let mut read_stream = stream.try_clone().unwrap();
        let read_event_tx = event_tx.clone();
        tokio::spawn(async move {
            Self::read_loop(client_id, &mut read_stream, read_event_tx).await;
        });

        // Spawn write task
        let mut write_stream = stream.try_clone().unwrap();
        tokio::spawn(async move {
            Self::write_loop(&mut write_stream, send_rx).await;
        });

        Self {
            client_id,
            stream,
            event_tx,
            send_queue: send_tx,
        }
    }

    /// Read loop (receives messages)
    async fn read_loop(
        client_id: u64,
        stream: &mut TcpStream,
        event_tx: mpsc::UnboundedSender<ServerEvent>,
    ) {
        let mut length_buf = [0u8; 4];

        loop {
            // Read message length (4 bytes, big-endian)
            match stream.read_exact(&mut length_buf).await {
                Ok(_) => {}
                Err(e) => {
                    tracing::error!("Failed to read length from client {}: {}", client_id, e);
                    event_tx
                        .send(ServerEvent::ClientDisconnected { client_id })
                        .ok();
                    break;
                }
            }

            let length = u32::from_be_bytes(length_buf) as usize;

            // Validate length (prevent attacks)
            if length > MAX_MESSAGE_SIZE {
                tracing::error!("Message too large from client {}: {} bytes", client_id, length);
                event_tx
                    .send(ServerEvent::ClientDisconnected { client_id })
                    .ok();
                break;
            }

            // Read message data
            let mut data = vec![0u8; length];
            match stream.read_exact(&mut data).await {
                Ok(_) => {
                    event_tx
                        .send(ServerEvent::MessageReceived { client_id, data })
                        .ok();
                }
                Err(e) => {
                    tracing::error!("Failed to read data from client {}: {}", client_id, e);
                    event_tx
                        .send(ServerEvent::ClientDisconnected { client_id })
                        .ok();
                    break;
                }
            }
        }

        tracing::info!("Read loop ended for client {}", client_id);
    }

    /// Write loop (sends messages)
    async fn write_loop(
        stream: &mut TcpStream,
        mut send_rx: mpsc::UnboundedReceiver<Vec<u8>>,
    ) {
        while let Some(data) = send_rx.recv().await {
            // Write length prefix
            let length = data.len() as u32;
            let length_buf = length.to_be_bytes();

            if let Err(e) = stream.write_all(&length_buf).await {
                tracing::error!("Failed to write length: {}", e);
                break;
            }

            // Write data
            if let Err(e) = stream.write_all(&data).await {
                tracing::error!("Failed to write data: {}", e);
                break;
            }

            // Flush
            if let Err(e) = stream.flush().await {
                tracing::error!("Failed to flush: {}", e);
                break;
            }
        }

        tracing::info!("Write loop ended");
    }

    /// Send message
    pub async fn send(&mut self, data: Vec<u8>) -> Result<(), NetworkError> {
        self.send_queue
            .send(data)
            .map_err(|_| NetworkError::TcpSendFailed {
                details: "Send queue closed".to_string(),
            })
    }

    /// Close connection
    pub fn close(&mut self) {
        // Drop send queue to signal write loop to stop
        drop(self.send_queue.clone());
    }
}

const MAX_MESSAGE_SIZE: usize = 10 * 1024 * 1024; // 10 MB
```

---

### **3. TCP Client** (Day 2-3)

**File:** `engine/networking/src/tcp/client.rs`

```rust
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;

/// TCP client
pub struct TcpClient {
    stream: Option<TcpStream>,
    event_tx: mpsc::UnboundedSender<ClientEvent>,
    send_queue: mpsc::UnboundedSender<Vec<u8>>,
}

/// Client events
#[derive(Debug)]
pub enum ClientEvent {
    Connected,
    Disconnected,
    MessageReceived { data: Vec<u8> },
}

impl TcpClient {
    /// Create TCP client
    pub fn new() -> Self {
        let (event_tx, _event_rx) = mpsc::unbounded_channel();
        let (send_tx, _send_rx) = mpsc::unbounded_channel();

        Self {
            stream: None,
            event_tx,
            send_queue: send_tx,
        }
    }

    /// Connect to server
    pub async fn connect(&mut self, server_addr: &str) -> Result<(), NetworkError> {
        let stream = TcpStream::connect(server_addr)
            .await
            .map_err(|e| NetworkError::TcpConnectFailed {
                details: e.to_string(),
            })?;

        tracing::info!("Connected to server at {}", server_addr);

        // Spawn read task
        let mut read_stream = stream.try_clone().unwrap();
        let read_event_tx = self.event_tx.clone();
        tokio::spawn(async move {
            Self::read_loop(&mut read_stream, read_event_tx).await;
        });

        // Spawn write task
        let mut write_stream = stream.try_clone().unwrap();
        let (send_tx, send_rx) = mpsc::unbounded_channel();
        tokio::spawn(async move {
            Self::write_loop(&mut write_stream, send_rx).await;
        });

        self.stream = Some(stream);
        self.send_queue = send_tx;

        self.event_tx.send(ClientEvent::Connected).ok();

        Ok(())
    }

    /// Read loop
    async fn read_loop(
        stream: &mut TcpStream,
        event_tx: mpsc::UnboundedSender<ClientEvent>,
    ) {
        let mut length_buf = [0u8; 4];

        loop {
            // Read message length
            match stream.read_exact(&mut length_buf).await {
                Ok(_) => {}
                Err(e) => {
                    tracing::error!("Failed to read length: {}", e);
                    event_tx.send(ClientEvent::Disconnected).ok();
                    break;
                }
            }

            let length = u32::from_be_bytes(length_buf) as usize;

            if length > MAX_MESSAGE_SIZE {
                tracing::error!("Message too large: {} bytes", length);
                event_tx.send(ClientEvent::Disconnected).ok();
                break;
            }

            // Read message data
            let mut data = vec![0u8; length];
            match stream.read_exact(&mut data).await {
                Ok(_) => {
                    event_tx.send(ClientEvent::MessageReceived { data }).ok();
                }
                Err(e) => {
                    tracing::error!("Failed to read data: {}", e);
                    event_tx.send(ClientEvent::Disconnected).ok();
                    break;
                }
            }
        }

        tracing::info!("Read loop ended");
    }

    /// Write loop
    async fn write_loop(
        stream: &mut TcpStream,
        mut send_rx: mpsc::UnboundedReceiver<Vec<u8>>,
    ) {
        while let Some(data) = send_rx.recv().await {
            let length = data.len() as u32;
            let length_buf = length.to_be_bytes();

            if stream.write_all(&length_buf).await.is_err() {
                break;
            }

            if stream.write_all(&data).await.is_err() {
                break;
            }

            if stream.flush().await.is_err() {
                break;
            }
        }

        tracing::info!("Write loop ended");
    }

    /// Send message
    pub async fn send(&mut self, data: Vec<u8>) -> Result<(), NetworkError> {
        self.send_queue
            .send(data)
            .map_err(|_| NetworkError::TcpSendFailed {
                details: "Not connected".to_string(),
            })
    }

    /// Disconnect
    pub async fn disconnect(&mut self) {
        if let Some(mut stream) = self.stream.take() {
            stream.shutdown().await.ok();
        }
        self.event_tx.send(ClientEvent::Disconnected).ok();
    }

    /// Get event receiver
    pub fn event_receiver(&self) -> mpsc::UnboundedReceiver<ClientEvent> {
        let (tx, rx) = mpsc::unbounded_channel();
        rx
    }
}

const MAX_MESSAGE_SIZE: usize = 10 * 1024 * 1024; // 10 MB
```

---

### **4. Message Framing** (Day 3)

**File:** `engine/networking/src/tcp/framing.rs`

```rust
/// Frame encoder/decoder (length-prefixed)
pub struct FrameCodec;

impl FrameCodec {
    /// Encode message with length prefix
    pub fn encode(data: &[u8]) -> Vec<u8> {
        let length = data.len() as u32;
        let mut encoded = Vec::with_capacity(4 + data.len());
        encoded.extend_from_slice(&length.to_be_bytes());
        encoded.extend_from_slice(data);
        encoded
    }

    /// Decode length prefix
    pub fn decode_length(length_buf: &[u8; 4]) -> u32 {
        u32::from_be_bytes(*length_buf)
    }

    /// Validate message length
    pub fn validate_length(length: u32) -> bool {
        length > 0 && length <= MAX_MESSAGE_SIZE as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_codec() {
        let data = b"Hello, World!";
        let encoded = FrameCodec::encode(data);

        // Check length prefix
        let length = u32::from_be_bytes([encoded[0], encoded[1], encoded[2], encoded[3]]);
        assert_eq!(length, data.len() as u32);

        // Check data
        assert_eq!(&encoded[4..], data);
    }

    #[test]
    fn test_validate_length() {
        assert!(FrameCodec::validate_length(100));
        assert!(FrameCodec::validate_length(MAX_MESSAGE_SIZE as u32));
        assert!(!FrameCodec::validate_length(0));
        assert!(!FrameCodec::validate_length(MAX_MESSAGE_SIZE as u32 + 1));
    }
}
```

---

### **5. Error Handling** (Day 4)

**File:** `engine/networking/src/error.rs`

```rust
define_error! {
    pub enum NetworkError {
        TcpBindFailed { details: String } = ErrorCode::TcpBindFailed, ErrorSeverity::Critical,
        TcpAcceptFailed { details: String } = ErrorCode::TcpAcceptFailed, ErrorSeverity::Error,
        TcpConnectFailed { details: String } = ErrorCode::TcpConnectFailed, ErrorSeverity::Error,
        TcpSendFailed { details: String } = ErrorCode::TcpSendFailed, ErrorSeverity::Error,
        TcpReceiveFailed { details: String } = ErrorCode::TcpReceiveFailed, ErrorSeverity::Error,
        MessageTooLarge { size: usize } = ErrorCode::MessageTooLarge, ErrorSeverity::Warning,
        ConnectionClosed = ErrorCode::ConnectionClosed, ErrorSeverity::Warning,
    }
}
```

---

## ✅ **Acceptance Criteria**

- [ ] TCP server accepts connections
- [ ] TCP client connects to server
- [ ] Messages sent/received correctly
- [ ] Length-prefixed framing works
- [ ] Connection closure handled gracefully
- [ ] Multiple clients supported
- [ ] Broadcast works
- [ ] Error handling robust
- [ ] No memory leaks
- [ ] Works on all platforms

---

## 🎯 **Performance Targets**

| Operation | Target | Critical |
|-----------|--------|----------|
| Connection establishment | < 50ms | < 100ms |
| Send small message (<1KB) | < 1ms | < 5ms |
| Send large message (1MB) | < 50ms | < 100ms |
| Throughput | > 100 MB/s | > 50 MB/s |
| Concurrent connections | > 1000 | > 100 |

---

## 🧪 **Tests**

```rust
#[tokio::test]
async fn test_tcp_connection() {
    // Start server
    let mut server = TcpServer::new("127.0.0.1:7777").await.unwrap();
    tokio::spawn(async move {
        server.run().await.unwrap();
    });

    // Wait for server to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Connect client
    let mut client = TcpClient::new();
    client.connect("127.0.0.1:7777").await.unwrap();

    // Send message
    let data = b"Hello, Server!";
    client.send(data.to_vec()).await.unwrap();

    // Wait for message
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Verify received
    // (Use event receiver to check)
}

#[tokio::test]
async fn test_multiple_clients() {
    let mut server = TcpServer::new("127.0.0.1:7778").await.unwrap();

    // Connect 10 clients
    let mut clients = Vec::new();
    for _ in 0..10 {
        let mut client = TcpClient::new();
        client.connect("127.0.0.1:7778").await.unwrap();
        clients.push(client);
    }

    // Broadcast message
    server.broadcast(b"Broadcast!".to_vec()).await.unwrap();

    // All clients should receive
    // (Verify with event receivers)
}
```

---

**Dependencies:** [phase2-network-protocol.md](phase2-network-protocol.md)
**Next:** [phase2-udp-packets.md](phase2-udp-packets.md)
