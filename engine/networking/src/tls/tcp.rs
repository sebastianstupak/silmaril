//! TLS over TCP implementation
//!
//! Provides TLS-encrypted TCP connections with transparent integration.

use super::error::{TlsError, TlsResult};
use rustls::{ClientConfig, ServerConfig};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::{client::TlsStream as ClientTlsStream, server::TlsStream as ServerTlsStream};
use tokio_rustls::{TlsAcceptor, TlsConnector};
use tracing::{debug, info};

/// Maximum message size (10MB) - same as non-TLS TCP
const MAX_MESSAGE_SIZE: u32 = 10 * 1024 * 1024;

/// TLS-encrypted TCP connection (client side)
pub struct TlsClientConnection {
    stream: ClientTlsStream<TcpStream>,
    peer_addr: SocketAddr,
}

impl TlsClientConnection {
    /// Create a new TLS client connection
    pub async fn connect(
        addr: impl AsRef<str>,
        server_name: impl AsRef<str>,
        config: Arc<ClientConfig>,
    ) -> TlsResult<Self> {
        let addr_str = addr.as_ref();
        let server_name = server_name.as_ref();

        info!(
            addr = %addr_str,
            server_name = %server_name,
            "Establishing TLS connection"
        );

        // Connect TCP socket
        let stream = TcpStream::connect(addr_str).await.map_err(|e| TlsError::ConnectionError {
            reason: format!("Failed to connect TCP socket: {}", e),
            #[cfg(feature = "backtrace")]
            backtrace: std::backtrace::Backtrace::capture(),
        })?;

        let peer_addr = stream.peer_addr().map_err(|e| TlsError::ConnectionError {
            reason: format!("Failed to get peer address: {}", e),
            #[cfg(feature = "backtrace")]
            backtrace: std::backtrace::Backtrace::capture(),
        })?;

        // Perform TLS handshake
        let connector = TlsConnector::from(config);
        let server_name =
            rustls::ServerName::try_from(server_name).map_err(|e| TlsError::HandshakeFailed {
                reason: format!("Invalid server name: {}", e),
                #[cfg(feature = "backtrace")]
                backtrace: std::backtrace::Backtrace::capture(),
            })?;

        let stream = connector.connect(server_name, stream).await.map_err(|e| {
            TlsError::HandshakeFailed {
                reason: format!("TLS handshake failed: {}", e),
                #[cfg(feature = "backtrace")]
                backtrace: std::backtrace::Backtrace::capture(),
            }
        })?;

        info!(peer_addr = %peer_addr, "TLS connection established");

        Ok(Self { stream, peer_addr })
    }

    /// Get peer address
    pub fn peer_addr(&self) -> SocketAddr {
        self.peer_addr
    }

    /// Send a message with length prefix (same format as non-TLS TCP)
    pub async fn send(&mut self, data: &[u8]) -> TlsResult<()> {
        let len = data.len() as u32;
        if len > MAX_MESSAGE_SIZE {
            return Err(TlsError::EncryptionError {
                reason: format!("Message too large: {} bytes (max {})", len, MAX_MESSAGE_SIZE),
                #[cfg(feature = "backtrace")]
                backtrace: std::backtrace::Backtrace::capture(),
            });
        }

        // Write length prefix (4 bytes, big-endian)
        self.stream.write_u32(len).await.map_err(|e| TlsError::EncryptionError {
            reason: format!("Failed to write length prefix: {}", e),
            #[cfg(feature = "backtrace")]
            backtrace: std::backtrace::Backtrace::capture(),
        })?;

        // Write message data
        self.stream.write_all(data).await.map_err(|e| TlsError::EncryptionError {
            reason: format!("Failed to write message: {}", e),
            #[cfg(feature = "backtrace")]
            backtrace: std::backtrace::Backtrace::capture(),
        })?;

        // Flush to ensure data is sent
        self.stream.flush().await.map_err(|e| TlsError::EncryptionError {
            reason: format!("Failed to flush stream: {}", e),
            #[cfg(feature = "backtrace")]
            backtrace: std::backtrace::Backtrace::capture(),
        })?;

        Ok(())
    }

    /// Receive a message with length prefix
    pub async fn recv(&mut self) -> TlsResult<Vec<u8>> {
        // Read length prefix (4 bytes, big-endian)
        let len = match self.stream.read_u32().await {
            Ok(len) => len,
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                return Err(TlsError::ConnectionError {
                    reason: "Connection closed".to_string(),
                    #[cfg(feature = "backtrace")]
                    backtrace: std::backtrace::Backtrace::capture(),
                });
            }
            Err(e) => {
                return Err(TlsError::DecryptionError {
                    reason: format!("Failed to read length prefix: {}", e),
                    #[cfg(feature = "backtrace")]
                    backtrace: std::backtrace::Backtrace::capture(),
                });
            }
        };

        if len > MAX_MESSAGE_SIZE {
            return Err(TlsError::DecryptionError {
                reason: format!("Message too large: {} bytes (max {})", len, MAX_MESSAGE_SIZE),
                #[cfg(feature = "backtrace")]
                backtrace: std::backtrace::Backtrace::capture(),
            });
        }

        // Read message data
        let mut buffer = vec![0u8; len as usize];
        self.stream
            .read_exact(&mut buffer)
            .await
            .map_err(|e| TlsError::DecryptionError {
                reason: format!("Failed to read message: {}", e),
                #[cfg(feature = "backtrace")]
                backtrace: std::backtrace::Backtrace::capture(),
            })?;

        Ok(buffer)
    }

    /// Close the connection
    pub async fn close(mut self) -> TlsResult<()> {
        self.stream.shutdown().await.map_err(|e| TlsError::ConnectionError {
            reason: format!("Failed to shutdown connection: {}", e),
            #[cfg(feature = "backtrace")]
            backtrace: std::backtrace::Backtrace::capture(),
        })?;
        Ok(())
    }
}

/// TLS-encrypted TCP connection (server side)
pub struct TlsServerConnection {
    stream: ServerTlsStream<TcpStream>,
    peer_addr: SocketAddr,
}

impl TlsServerConnection {
    /// Get peer address
    pub fn peer_addr(&self) -> SocketAddr {
        self.peer_addr
    }

    /// Send a message with length prefix
    pub async fn send(&mut self, data: &[u8]) -> TlsResult<()> {
        let len = data.len() as u32;
        if len > MAX_MESSAGE_SIZE {
            return Err(TlsError::EncryptionError {
                reason: format!("Message too large: {} bytes (max {})", len, MAX_MESSAGE_SIZE),
                #[cfg(feature = "backtrace")]
                backtrace: std::backtrace::Backtrace::capture(),
            });
        }

        // Write length prefix (4 bytes, big-endian)
        self.stream.write_u32(len).await.map_err(|e| TlsError::EncryptionError {
            reason: format!("Failed to write length prefix: {}", e),
            #[cfg(feature = "backtrace")]
            backtrace: std::backtrace::Backtrace::capture(),
        })?;

        // Write message data
        self.stream.write_all(data).await.map_err(|e| TlsError::EncryptionError {
            reason: format!("Failed to write message: {}", e),
            #[cfg(feature = "backtrace")]
            backtrace: std::backtrace::Backtrace::capture(),
        })?;

        // Flush to ensure data is sent
        self.stream.flush().await.map_err(|e| TlsError::EncryptionError {
            reason: format!("Failed to flush stream: {}", e),
            #[cfg(feature = "backtrace")]
            backtrace: std::backtrace::Backtrace::capture(),
        })?;

        Ok(())
    }

    /// Receive a message with length prefix
    pub async fn recv(&mut self) -> TlsResult<Vec<u8>> {
        // Read length prefix (4 bytes, big-endian)
        let len = match self.stream.read_u32().await {
            Ok(len) => len,
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                return Err(TlsError::ConnectionError {
                    reason: "Connection closed".to_string(),
                    #[cfg(feature = "backtrace")]
                    backtrace: std::backtrace::Backtrace::capture(),
                });
            }
            Err(e) => {
                return Err(TlsError::DecryptionError {
                    reason: format!("Failed to read length prefix: {}", e),
                    #[cfg(feature = "backtrace")]
                    backtrace: std::backtrace::Backtrace::capture(),
                });
            }
        };

        if len > MAX_MESSAGE_SIZE {
            return Err(TlsError::DecryptionError {
                reason: format!("Message too large: {} bytes (max {})", len, MAX_MESSAGE_SIZE),
                #[cfg(feature = "backtrace")]
                backtrace: std::backtrace::Backtrace::capture(),
            });
        }

        // Read message data
        let mut buffer = vec![0u8; len as usize];
        self.stream
            .read_exact(&mut buffer)
            .await
            .map_err(|e| TlsError::DecryptionError {
                reason: format!("Failed to read message: {}", e),
                #[cfg(feature = "backtrace")]
                backtrace: std::backtrace::Backtrace::capture(),
            })?;

        Ok(buffer)
    }

    /// Close the connection
    pub async fn close(mut self) -> TlsResult<()> {
        self.stream.shutdown().await.map_err(|e| TlsError::ConnectionError {
            reason: format!("Failed to shutdown connection: {}", e),
            #[cfg(feature = "backtrace")]
            backtrace: std::backtrace::Backtrace::capture(),
        })?;
        Ok(())
    }
}

/// TLS-enabled TCP server
pub struct TlsServer {
    listener: TcpListener,
    acceptor: TlsAcceptor,
}

impl TlsServer {
    /// Create a new TLS server
    pub async fn bind(addr: impl AsRef<str>, config: Arc<ServerConfig>) -> TlsResult<Self> {
        let addr_str = addr.as_ref();

        info!(addr = %addr_str, "Binding TLS server");

        let listener =
            TcpListener::bind(addr_str).await.map_err(|e| TlsError::ConnectionError {
                reason: format!("Failed to bind listener: {}", e),
                #[cfg(feature = "backtrace")]
                backtrace: std::backtrace::Backtrace::capture(),
            })?;

        let acceptor = TlsAcceptor::from(config);

        info!(local_addr = ?listener.local_addr(), "TLS server listening");

        Ok(Self { listener, acceptor })
    }

    /// Get the local address
    pub fn local_addr(&self) -> TlsResult<SocketAddr> {
        self.listener.local_addr().map_err(|e| TlsError::ConnectionError {
            reason: format!("Failed to get local address: {}", e),
            #[cfg(feature = "backtrace")]
            backtrace: std::backtrace::Backtrace::capture(),
        })
    }

    /// Accept a new TLS connection
    pub async fn accept(&self) -> TlsResult<TlsServerConnection> {
        // Accept TCP connection
        let (stream, peer_addr) =
            self.listener.accept().await.map_err(|e| TlsError::ConnectionError {
                reason: format!("Failed to accept connection: {}", e),
                #[cfg(feature = "backtrace")]
                backtrace: std::backtrace::Backtrace::capture(),
            })?;

        debug!(peer_addr = %peer_addr, "Accepted TCP connection, performing TLS handshake");

        // Perform TLS handshake
        let stream = self.acceptor.accept(stream).await.map_err(|e| TlsError::HandshakeFailed {
            reason: format!("TLS handshake failed: {}", e),
            #[cfg(feature = "backtrace")]
            backtrace: std::backtrace::Backtrace::capture(),
        })?;

        info!(peer_addr = %peer_addr, "TLS connection accepted");

        Ok(TlsServerConnection { stream, peer_addr })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tls::certificates::selfsigned::{
        generate_and_save_self_signed_cert, SelfSignedConfig,
    };
    use crate::tls::config::{
        CertificateVerification, TlsClientConfigBuilder, TlsServerConfigBuilder,
    };
    use std::env;

    async fn setup_test_certificates() -> (String, String) {
        use std::time::{SystemTime, UNIX_EPOCH};

        let temp_dir = env::temp_dir();
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let cert_path =
            temp_dir.join(format!("test_tls_cert_{}_{}.pem", std::process::id(), timestamp));
        let key_path =
            temp_dir.join(format!("test_tls_key_{}_{}.pem", std::process::id(), timestamp));

        let config = SelfSignedConfig::new("localhost").add_san("127.0.0.1").validity_days(1);

        generate_and_save_self_signed_cert(&config, &cert_path, &key_path)
            .expect("Failed to generate test certificate");

        (cert_path.to_string_lossy().to_string(), key_path.to_string_lossy().to_string())
    }

    #[tokio::test]
    async fn test_tls_connection() {
        let (cert_path, key_path) = setup_test_certificates().await;

        // Create server config
        let server_config = TlsServerConfigBuilder::new()
            .certificate(&cert_path, &key_path)
            .build()
            .expect("Failed to build server config");

        // Start TLS server
        let server = TlsServer::bind("127.0.0.1:0", server_config)
            .await
            .expect("Failed to bind server");
        let server_addr = server.local_addr().expect("Failed to get server address");

        // Spawn server task
        tokio::spawn(async move {
            let mut conn = server.accept().await.expect("Failed to accept connection");
            let msg = conn.recv().await.expect("Failed to receive message");
            conn.send(&msg).await.expect("Failed to send echo");
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Create client config (disable verification for self-signed cert)
        let client_config = TlsClientConfigBuilder::new()
            .verification(CertificateVerification::Disabled)
            .build()
            .expect("Failed to build client config");

        // Connect client
        let mut client =
            TlsClientConnection::connect(server_addr.to_string(), "localhost", client_config)
                .await
                .expect("Failed to connect client");

        // Send message
        let message = b"Hello, TLS!";
        client.send(message).await.expect("Failed to send message");

        // Receive echo
        let response = client.recv().await.expect("Failed to receive response");
        assert_eq!(response, message);

        // Clean up
        std::fs::remove_file(&cert_path).ok();
        std::fs::remove_file(&key_path).ok();
    }

    #[tokio::test]
    async fn test_tls_multiple_messages() {
        let (cert_path, key_path) = setup_test_certificates().await;

        let server_config = TlsServerConfigBuilder::new()
            .certificate(&cert_path, &key_path)
            .build()
            .expect("Failed to build server config");

        let server = TlsServer::bind("127.0.0.1:0", server_config)
            .await
            .expect("Failed to bind server");
        let server_addr = server.local_addr().expect("Failed to get server address");

        tokio::spawn(async move {
            let mut conn = server.accept().await.expect("Failed to accept connection");
            for _ in 0..5 {
                let msg = conn.recv().await.expect("Failed to receive message");
                conn.send(&msg).await.expect("Failed to send echo");
            }
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let client_config = TlsClientConfigBuilder::new()
            .verification(CertificateVerification::Disabled)
            .build()
            .expect("Failed to build client config");

        let mut client =
            TlsClientConnection::connect(server_addr.to_string(), "localhost", client_config)
                .await
                .expect("Failed to connect client");

        // Send multiple messages
        for i in 0..5 {
            let message = format!("Message {}", i);
            client.send(message.as_bytes()).await.expect("Failed to send message");
            let response = client.recv().await.expect("Failed to receive response");
            assert_eq!(response, message.as_bytes());
        }

        // Clean up
        std::fs::remove_file(&cert_path).ok();
        std::fs::remove_file(&key_path).ok();
    }
}
