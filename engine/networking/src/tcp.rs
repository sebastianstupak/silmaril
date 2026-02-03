//! TCP connection implementation
//!
//! Provides async TCP client and server using tokio with:
//! - Connection management
//! - Message framing with length prefixes
//! - Graceful disconnect handling
//! - Error propagation

use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;

/// Maximum message size (10MB)
const MAX_MESSAGE_SIZE: u32 = 10 * 1024 * 1024;

/// TCP connection errors
#[derive(Debug)]
pub enum TcpError {
    /// I/O error
    Io(std::io::Error),
    /// Connection closed
    ConnectionClosed,
    /// Message too large
    MessageTooLarge(u32),
    /// Invalid message format
    InvalidMessage,
}

impl std::fmt::Display for TcpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TcpError::Io(e) => write!(f, "I/O error: {}", e),
            TcpError::ConnectionClosed => write!(f, "Connection closed"),
            TcpError::MessageTooLarge(size) => write!(f, "Message too large: {} bytes", size),
            TcpError::InvalidMessage => write!(f, "Invalid message format"),
        }
    }
}

impl std::error::Error for TcpError {}

impl From<std::io::Error> for TcpError {
    fn from(e: std::io::Error) -> Self {
        TcpError::Io(e)
    }
}

/// Result type for TCP operations
pub type TcpResult<T> = Result<T, TcpError>;

/// TCP connection wrapper
#[derive(Debug)]
pub struct TcpConnection {
    stream: Arc<Mutex<TcpStream>>,
    peer_addr: std::net::SocketAddr,
}

impl TcpConnection {
    /// Create a new TCP connection from a stream
    pub fn new(stream: TcpStream) -> TcpResult<Self> {
        let peer_addr = stream.peer_addr()?;
        Ok(Self { stream: Arc::new(Mutex::new(stream)), peer_addr })
    }

    /// Get peer address
    pub fn peer_addr(&self) -> std::net::SocketAddr {
        self.peer_addr
    }

    /// Send a message with length prefix
    pub async fn send(&self, data: &[u8]) -> TcpResult<()> {
        let len = data.len() as u32;
        if len > MAX_MESSAGE_SIZE {
            return Err(TcpError::MessageTooLarge(len));
        }

        let mut stream = self.stream.lock().await;

        // Write length prefix (4 bytes, big-endian)
        stream.write_u32(len).await?;

        // Write message data
        stream.write_all(data).await?;

        // Flush to ensure data is sent
        stream.flush().await?;

        Ok(())
    }

    /// Receive a message with length prefix
    pub async fn recv(&self) -> TcpResult<Vec<u8>> {
        let mut stream = self.stream.lock().await;

        // Read length prefix (4 bytes, big-endian)
        let len = match stream.read_u32().await {
            Ok(len) => len,
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                return Err(TcpError::ConnectionClosed);
            }
            Err(e) => return Err(TcpError::Io(e)),
        };

        if len > MAX_MESSAGE_SIZE {
            return Err(TcpError::MessageTooLarge(len));
        }

        // Read message data
        let mut buffer = vec![0u8; len as usize];
        stream.read_exact(&mut buffer).await?;

        Ok(buffer)
    }

    /// Close the connection
    pub async fn close(&self) -> TcpResult<()> {
        let mut stream = self.stream.lock().await;
        stream.shutdown().await?;
        Ok(())
    }
}

/// TCP client
pub struct TcpClient {
    connection: TcpConnection,
}

impl TcpClient {
    /// Connect to a server
    pub async fn connect(addr: &str) -> TcpResult<Self> {
        let stream = TcpStream::connect(addr).await?;
        let connection = TcpConnection::new(stream)?;
        Ok(Self { connection })
    }

    /// Send a message
    pub async fn send(&self, data: &[u8]) -> TcpResult<()> {
        self.connection.send(data).await
    }

    /// Receive a message
    pub async fn recv(&self) -> TcpResult<Vec<u8>> {
        self.connection.recv().await
    }

    /// Close the connection
    pub async fn close(&self) -> TcpResult<()> {
        self.connection.close().await
    }

    /// Get peer address
    pub fn peer_addr(&self) -> std::net::SocketAddr {
        self.connection.peer_addr()
    }
}

/// TCP server
pub struct TcpServer {
    listener: TcpListener,
}

impl TcpServer {
    /// Bind to an address
    pub async fn bind(addr: &str) -> TcpResult<Self> {
        let listener = TcpListener::bind(addr).await?;
        Ok(Self { listener })
    }

    /// Get the local address
    pub fn local_addr(&self) -> TcpResult<std::net::SocketAddr> {
        Ok(self.listener.local_addr()?)
    }

    /// Accept a new connection
    pub async fn accept(&self) -> TcpResult<TcpConnection> {
        let (stream, _) = self.listener.accept().await?;
        TcpConnection::new(stream)
    }

    /// Run an echo server (for testing)
    pub async fn run_echo_server(self) -> TcpResult<()> {
        loop {
            let conn = self.accept().await?;

            tokio::spawn(async move {
                loop {
                    match conn.recv().await {
                        Ok(data) => {
                            if let Err(e) = conn.send(&data).await {
                                tracing::error!(error = ?e, "Failed to echo message");
                                break;
                            }
                        }
                        Err(TcpError::ConnectionClosed) => {
                            tracing::debug!("Client disconnected");
                            break;
                        }
                        Err(e) => {
                            tracing::error!(error = ?e, "Failed to receive message");
                            break;
                        }
                    }
                }
            });
        }
    }
}

/// Message handler trait
pub trait MessageHandler: Send + Sync + 'static {
    /// Handle a received message
    fn handle_message(&self, data: Vec<u8>) -> Vec<u8>;
}

/// Echo handler (returns the same message)
pub struct EchoHandler;

impl MessageHandler for EchoHandler {
    fn handle_message(&self, data: Vec<u8>) -> Vec<u8> {
        data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tcp_connect_and_send() {
        // Start echo server
        let server = TcpServer::bind("127.0.0.1:0").await.unwrap();
        let addr = server.local_addr().unwrap();

        tokio::spawn(async move {
            server.run_echo_server().await.ok();
        });

        // Give server time to start
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Connect client
        let client = TcpClient::connect(&addr.to_string()).await.unwrap();

        // Send message
        let message = b"Hello, World!";
        client.send(message).await.unwrap();

        // Receive echo
        let response = client.recv().await.unwrap();
        assert_eq!(response, message);

        client.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_tcp_multiple_messages() {
        let server = TcpServer::bind("127.0.0.1:0").await.unwrap();
        let addr = server.local_addr().unwrap();

        tokio::spawn(async move {
            server.run_echo_server().await.ok();
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let client = TcpClient::connect(&addr.to_string()).await.unwrap();

        // Send multiple messages
        for i in 0..10 {
            let message = format!("Message {}", i);
            client.send(message.as_bytes()).await.unwrap();
            let response = client.recv().await.unwrap();
            assert_eq!(response, message.as_bytes());
        }

        client.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_tcp_large_message() {
        let server = TcpServer::bind("127.0.0.1:0").await.unwrap();
        let addr = server.local_addr().unwrap();

        tokio::spawn(async move {
            server.run_echo_server().await.ok();
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let client = TcpClient::connect(&addr.to_string()).await.unwrap();

        // Send large message (1MB)
        let message = vec![0x42u8; 1024 * 1024];
        client.send(&message).await.unwrap();
        let response = client.recv().await.unwrap();
        assert_eq!(response.len(), message.len());
        assert_eq!(response, message);

        client.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_tcp_message_too_large() {
        let server = TcpServer::bind("127.0.0.1:0").await.unwrap();
        let addr = server.local_addr().unwrap();

        tokio::spawn(async move {
            server.run_echo_server().await.ok();
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let client = TcpClient::connect(&addr.to_string()).await.unwrap();

        // Try to send message larger than MAX_MESSAGE_SIZE
        let message = vec![0u8; (MAX_MESSAGE_SIZE + 1) as usize];
        let result = client.send(&message).await;
        assert!(matches!(result, Err(TcpError::MessageTooLarge(_))));

        client.close().await.unwrap();
    }
}
