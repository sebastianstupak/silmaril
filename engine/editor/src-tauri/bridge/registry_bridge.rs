//! Registry bridge — exposes the command registry to the MCP server.
//!
//! This module is a stub. Full implementation is deferred to the MCP Server plan.
//! It will provide:
//! - A watch receiver for live registry updates (for MCP tool discovery)
//! - A function to list all commands as MCP tool definitions
//! - A function to invoke a command by id from the MCP server

use tokio::sync::watch;
use crate::bridge::registry::CommandSpec;

/// Returns a clone of the registry watch receiver so the MCP server
/// can subscribe to live catalog updates.
///
/// Not yet implemented — returns a placeholder receiver.
#[allow(dead_code)]
pub fn registry_watch_rx() -> watch::Receiver<Vec<CommandSpec>> {
    let (_tx, rx) = watch::channel(Vec::new());
    rx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_watch_rx_returns_empty_initial_value() {
        let rx = registry_watch_rx();
        assert!(rx.borrow().is_empty());
    }
}
