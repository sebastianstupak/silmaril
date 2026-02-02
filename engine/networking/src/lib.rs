//! Engine Networking
//!
//! Provides client-server networking:
//! - TCP for reliable messages
//! - UDP for real-time updates
//! - State synchronization
//! - Client prediction

#![warn(missing_docs)]

/// TCP connection implementation
pub mod tcp;

/// UDP socket implementation
pub mod udp;

/// Network delta encoding
pub mod delta;

/// Network protocol and message types
pub mod protocol;

/// World snapshot utilities
pub mod snapshot;

/// Network simulator for testing
pub mod simulator;

/// Interest management integration
pub mod interest_filtering;

/// TLS/DTLS encryption
pub mod tls;

// Re-export commonly used types
pub use delta::{AdaptiveDeltaStrategy, NetworkDelta};
pub use interest_filtering::{InterestFilter, InterestFilterStats};
pub use protocol::{
    deserialize_client_message, deserialize_server_message, serialize_client_message,
    serialize_server_message, ClientMessage, EntityState, FramedMessage, ProtocolError,
    SerializationFormat, ServerMessage, PROTOCOL_VERSION,
};
pub use simulator::{NetworkConditions, NetworkProfile, NetworkSimulator};
pub use snapshot::WorldSnapshot;
pub use tcp::{TcpClient, TcpConnection, TcpError, TcpResult, TcpServer};
pub use tls::{
    generate_self_signed_cert, AcmeClient, AcmeConfig, CertificateInfo, CertificateManager,
    CertificateStatus, CertificateVerification, CipherSuiteSelection, SelfSignedConfig,
    SessionCache, SessionCacheStats, TlsClientConfigBuilder, TlsClientConnection, TlsError,
    TlsResult, TlsServer, TlsServerConfigBuilder, TlsServerConnection, TlsVersion,
};
pub use udp::{UdpClient, UdpError, UdpResult, UdpServer, UdpSocket};
