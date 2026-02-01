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

// Re-export commonly used types
pub use delta::{AdaptiveDeltaStrategy, NetworkDelta};
pub use protocol::{
    ClientMessage, ServerMessage, EntityState, FramedMessage,
    SerializationFormat, ProtocolError, PROTOCOL_VERSION,
    serialize_client_message, deserialize_client_message,
    serialize_server_message, deserialize_server_message,
};
pub use snapshot::WorldSnapshot;
pub use simulator::{NetworkConditions, NetworkProfile, NetworkSimulator};
pub use tcp::{TcpClient, TcpServer, TcpConnection, TcpError, TcpResult};
pub use udp::{UdpClient, UdpServer, UdpSocket, UdpError, UdpResult};
