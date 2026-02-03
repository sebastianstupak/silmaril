//! E2E test helpers
//!
//! Utilities for spawning test servers and clients.

pub mod client_spawner;
pub mod server_spawner;

pub use client_spawner::{spawn_test_client, spawn_test_client_with_name, TestClientHandle};
pub use server_spawner::{spawn_test_server, TestServerHandle};
