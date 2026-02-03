//! UDP socket implementation
//!
//! Provides async UDP client and server using tokio with:
//! - Unreliable packet transmission
//! - Connection tracking
//! - Low-latency communication
//! - Datagram size validation

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket as TokioUdpSocket;
use tokio::sync::Mutex;

/// Maximum UDP packet size (1500 bytes - typical MTU minus headers)
const MAX_PACKET_SIZE: usize = 1400;

/// UDP errors
#[derive(Debug)]
pub enum UdpError {
    /// I/O error
    Io(std::io::Error),
    /// Packet too large
    PacketTooLarge(usize),
    /// Invalid packet
    InvalidPacket,
    /// No data received
    NoData,
}

impl std::fmt::Display for UdpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UdpError::Io(e) => write!(f, "I/O error: {}", e),
            UdpError::PacketTooLarge(size) => write!(f, "Packet too large: {} bytes", size),
            UdpError::InvalidPacket => write!(f, "Invalid packet"),
            UdpError::NoData => write!(f, "No data received"),
        }
    }
}

impl std::error::Error for UdpError {}

impl From<std::io::Error> for UdpError {
    fn from(e: std::io::Error) -> Self {
        UdpError::Io(e)
    }
}

/// Result type for UDP operations
pub type UdpResult<T> = Result<T, UdpError>;

/// UDP socket wrapper
pub struct UdpSocket {
    socket: Arc<TokioUdpSocket>,
    local_addr: SocketAddr,
}

impl UdpSocket {
    /// Bind to an address
    pub async fn bind(addr: &str) -> UdpResult<Self> {
        let socket = TokioUdpSocket::bind(addr).await?;
        let local_addr = socket.local_addr()?;
        Ok(Self { socket: Arc::new(socket), local_addr })
    }

    /// Get the local address
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Send a packet to an address
    pub async fn send_to(&self, data: &[u8], addr: SocketAddr) -> UdpResult<usize> {
        if data.len() > MAX_PACKET_SIZE {
            return Err(UdpError::PacketTooLarge(data.len()));
        }

        let bytes_sent = self.socket.send_to(data, addr).await?;
        Ok(bytes_sent)
    }

    /// Receive a packet
    pub async fn recv_from(&self) -> UdpResult<(Vec<u8>, SocketAddr)> {
        let mut buffer = vec![0u8; MAX_PACKET_SIZE];
        let (len, addr) = self.socket.recv_from(&mut buffer).await?;

        if len == 0 {
            return Err(UdpError::NoData);
        }

        buffer.truncate(len);
        Ok((buffer, addr))
    }

    /// Connect to a specific address (filters packets from other sources)
    pub async fn connect(&self, addr: SocketAddr) -> UdpResult<()> {
        self.socket.connect(addr).await?;
        Ok(())
    }

    /// Send data (only works after connect)
    pub async fn send(&self, data: &[u8]) -> UdpResult<usize> {
        if data.len() > MAX_PACKET_SIZE {
            return Err(UdpError::PacketTooLarge(data.len()));
        }

        let bytes_sent = self.socket.send(data).await?;
        Ok(bytes_sent)
    }

    /// Receive data (only works after connect)
    pub async fn recv(&self) -> UdpResult<Vec<u8>> {
        let mut buffer = vec![0u8; MAX_PACKET_SIZE];
        let len = self.socket.recv(&mut buffer).await?;

        if len == 0 {
            return Err(UdpError::NoData);
        }

        buffer.truncate(len);
        Ok(buffer)
    }
}

/// UDP client
pub struct UdpClient {
    socket: UdpSocket,
    server_addr: SocketAddr,
}

impl UdpClient {
    /// Create a new UDP client and connect to server
    pub async fn connect(server_addr: &str) -> UdpResult<Self> {
        // Bind to any available port
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        let server_addr: SocketAddr = server_addr.parse().map_err(|_| {
            UdpError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid server address",
            ))
        })?;

        Ok(Self { socket, server_addr })
    }

    /// Send a packet to the server
    pub async fn send(&self, data: &[u8]) -> UdpResult<usize> {
        self.socket.send_to(data, self.server_addr).await
    }

    /// Receive a packet (blocking)
    pub async fn recv(&self) -> UdpResult<Vec<u8>> {
        let (data, addr) = self.socket.recv_from().await?;

        // Verify packet is from server
        if addr != self.server_addr {
            return Err(UdpError::InvalidPacket);
        }

        Ok(data)
    }

    /// Get local address
    pub fn local_addr(&self) -> SocketAddr {
        self.socket.local_addr()
    }
}

/// UDP server
pub struct UdpServer {
    socket: UdpSocket,
    clients: Arc<Mutex<HashMap<SocketAddr, ()>>>,
}

impl UdpServer {
    /// Bind to an address
    pub async fn bind(addr: &str) -> UdpResult<Self> {
        let socket = UdpSocket::bind(addr).await?;
        Ok(Self { socket, clients: Arc::new(Mutex::new(HashMap::new())) })
    }

    /// Get the local address
    pub fn local_addr(&self) -> SocketAddr {
        self.socket.local_addr()
    }

    /// Receive a packet from any client
    pub async fn recv_from(&self) -> UdpResult<(Vec<u8>, SocketAddr)> {
        let (data, addr) = self.socket.recv_from().await?;

        // Track client
        let mut clients = self.clients.lock().await;
        clients.insert(addr, ());

        Ok((data, addr))
    }

    /// Send a packet to a specific client
    pub async fn send_to(&self, data: &[u8], addr: SocketAddr) -> UdpResult<usize> {
        self.socket.send_to(data, addr).await
    }

    /// Run an echo server (for testing)
    pub async fn run_echo_server(self) -> UdpResult<()> {
        loop {
            match self.recv_from().await {
                Ok((data, addr)) => {
                    if let Err(e) = self.send_to(&data, addr).await {
                        tracing::error!(error = ?e, addr = ?addr, "Failed to echo packet");
                    }
                }
                Err(e) => {
                    tracing::error!(error = ?e, "Failed to receive packet");
                }
            }
        }
    }

    /// Get number of connected clients
    pub async fn client_count(&self) -> usize {
        self.clients.lock().await.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_udp_send_recv() {
        // Start echo server
        let server = UdpServer::bind("127.0.0.1:0").await.unwrap();
        let server_addr = server.local_addr();

        tokio::spawn(async move {
            server.run_echo_server().await.ok();
        });

        // Give server time to start
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Connect client
        let client = UdpClient::connect(&server_addr.to_string()).await.unwrap();

        // Send packet
        let message = b"Hello, UDP!";
        client.send(message).await.unwrap();

        // Receive echo
        let response = client.recv().await.unwrap();
        assert_eq!(response, message);
    }

    #[tokio::test]
    async fn test_udp_multiple_packets() {
        let server = UdpServer::bind("127.0.0.1:0").await.unwrap();
        let server_addr = server.local_addr();

        tokio::spawn(async move {
            server.run_echo_server().await.ok();
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let client = UdpClient::connect(&server_addr.to_string()).await.unwrap();

        // Send multiple packets
        for i in 0..10 {
            let message = format!("Packet {}", i);
            client.send(message.as_bytes()).await.unwrap();
            let response = client.recv().await.unwrap();
            assert_eq!(response, message.as_bytes());
        }
    }

    #[tokio::test]
    async fn test_udp_large_packet() {
        let server = UdpServer::bind("127.0.0.1:0").await.unwrap();
        let server_addr = server.local_addr();

        tokio::spawn(async move {
            server.run_echo_server().await.ok();
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let client = UdpClient::connect(&server_addr.to_string()).await.unwrap();

        // Send packet near max size
        let message = vec![0x42u8; MAX_PACKET_SIZE];
        client.send(&message).await.unwrap();
        let response = client.recv().await.unwrap();
        assert_eq!(response.len(), message.len());
        assert_eq!(response, message);
    }

    #[tokio::test]
    async fn test_udp_packet_too_large() {
        let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();

        // Try to send packet larger than MAX_PACKET_SIZE
        let message = vec![0u8; MAX_PACKET_SIZE + 1];
        let result = socket.send_to(&message, "127.0.0.1:9999".parse().unwrap()).await;
        assert!(matches!(result, Err(UdpError::PacketTooLarge(_))));
    }

    #[tokio::test]
    async fn test_udp_multiple_clients() {
        let server = UdpServer::bind("127.0.0.1:0").await.unwrap();
        let server_addr = server.local_addr();

        // Spawn server task with the original server
        tokio::spawn(async move {
            server.run_echo_server().await.ok();
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Create multiple clients
        let client1 = UdpClient::connect(&server_addr.to_string()).await.unwrap();
        let client2 = UdpClient::connect(&server_addr.to_string()).await.unwrap();

        // Both clients send messages
        client1.send(b"Client 1").await.unwrap();
        client2.send(b"Client 2").await.unwrap();

        // Both clients receive responses
        let resp1 = client1.recv().await.unwrap();
        let resp2 = client2.recv().await.unwrap();

        assert_eq!(resp1, b"Client 1");
        assert_eq!(resp2, b"Client 2");
    }
}
