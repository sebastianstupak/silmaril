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

/// Server tick loop
pub mod server_loop;

/// Client-side prediction and reconciliation
pub mod prediction;

/// Connection pool and reconnection management
pub mod connection_pool;

/// High-precision timing for esports
pub mod precision_time;

/// Rate limiting and DDoS protection
pub mod rate_limiter;

/// Anti-cheat integration framework
pub mod anti_cheat;

/// Esports-grade replay system
pub mod replay;

/// Spectator mode for esports broadcasting
pub mod spectator;

// Re-export commonly used types
pub use anti_cheat::{AntiCheatConfig, AntiCheatManager, ValidationResult};
pub use connection_pool::{ConnectionPool, ConnectionPoolConfig, ConnectionState};
pub use delta::{AdaptiveDeltaStrategy, NetworkDelta};
pub use interest_filtering::{InterestFilter, InterestFilterStats};
pub use prediction::{
    AdaptiveErrorCorrector, BufferedInput, ClientPredictor, ErrorCorrector, InputBuffer,
    PredictionConfig,
};
pub use protocol::{
    deserialize_client_message, deserialize_server_message, serialize_client_message,
    serialize_server_message, ClientMessage, EntityState, FramedMessage, ProtocolError,
    SerializationFormat, ServerMessage, PROTOCOL_VERSION,
};
pub use rate_limiter::{RateLimitConfig, RateLimiter};
pub use replay::{ReplayConfig, ReplayFrame, ReplayPlayer, ReplayRecorder};
pub use server_loop::{
    PerformanceStats, ServerLoop, ServerLoopError, ServerLoopResult, TARGET_TPS,
};
pub use simulator::{NetworkConditions, NetworkProfile, NetworkSimulator};
pub use snapshot::WorldSnapshot;
pub use spectator::{GameEvent, SpectatorConfig, SpectatorManager, SpectatorPerspective};
pub use tcp::{TcpClient, TcpConnection, TcpError, TcpResult, TcpServer};
pub use tls::{
    generate_self_signed_cert, AcmeClient, AcmeConfig, CertificateInfo, CertificateManager,
    CertificateStatus, CertificateVerification, CipherSuiteSelection, SelfSignedConfig,
    SessionCache, SessionCacheStats, TlsClientConfigBuilder, TlsClientConnection, TlsError,
    TlsResult, TlsServer, TlsServerConfigBuilder, TlsServerConnection, TlsVersion,
};
pub use udp::{UdpClient, UdpError, UdpResult, UdpServer, UdpSocket};
