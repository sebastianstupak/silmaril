//! Engine Networking
//!
//! Provides client-server networking:
//! - TCP for reliable messages
//! - UDP for real-time updates
//! - State synchronization
//! - Client prediction

#![warn(missing_docs)]

pub mod protocol;
pub mod tcp;
pub mod udp;
pub mod state_sync;
pub mod prediction;

// Re-export commonly used types
pub use protocol::{Message, Packet};
pub use tcp::Server;
pub use udp::Client;
