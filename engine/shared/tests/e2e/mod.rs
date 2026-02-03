//! End-to-End (E2E) test infrastructure
//!
//! This module provides utilities for testing the complete networking stack
//! by spawning real servers and clients in test scenarios.
//!
//! # Architecture
//!
//! E2E tests use the actual networking code (not mocks) to validate:
//! - Client-server connectivity
//! - Message passing
//! - State synchronization
//! - Multi-client scenarios
//!
//! # Usage
//!
//! ```no_run
//! use engine_shared::e2e::helpers::{spawn_test_server, spawn_test_client};
//!
//! #[tokio::test]
//! async fn test_example() {
//!     let server = spawn_test_server("127.0.0.1:0").await.unwrap();
//!     let client = spawn_test_client(server.address()).await.unwrap();
//!
//!     assert!(client.is_connected().await);
//!
//!     client.disconnect().await;
//!     server.shutdown();
//! }
//! ```

pub mod connectivity_test;
pub mod helpers;
