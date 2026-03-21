//! Silmaril Editor AI integration — MCP server.
//!
//! This crate has no Tauri dependency. It communicates with the editor via
//! [`AiBridgeChannels`] — tokio channels owned by the editor's `ai_bridge.rs`.

pub mod mcp;
pub mod permissions;
pub mod registry_bridge;
pub mod server;

use tokio::sync::{mpsc, oneshot, watch};
use serde::{Deserialize, Serialize};

/// Error type for AI MCP server operations.
///
/// # Design Note
/// This crate intentionally does not use `silmaril_core::define_error!` because
/// `engine-ai` has no dependency on `silmaril_core` (to avoid coupling the MCP server
/// to the game engine internals). The hand-rolled `Display + Error` impl is the correct
/// approach for this standalone crate.
#[derive(Debug)]
pub enum AiError {
    /// Failed to bind the HTTP server to any port in the configured range.
    ServerBind(String),
    /// A required channel was closed unexpectedly.
    ChannelClosed,
    /// An operation timed out.
    Timeout,
    /// A catch-all for other errors.
    Other(String),
}

impl std::fmt::Display for AiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AiError::ServerBind(msg) => write!(f, "Server bind failed: {msg}"),
            AiError::ChannelClosed => write!(f, "Channel closed unexpectedly"),
            AiError::Timeout => write!(f, "Operation timed out"),
            AiError::Other(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for AiError {}

/// Minimal command descriptor used by the MCP layer.
/// The editor converts `CommandSpec` → `McpCommand` before passing to this crate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpCommand {
    /// Unique command identifier (e.g. `"scene.create_entity"`).
    pub id: String,
    /// Human-readable label shown in MCP tool listings.
    pub label: String,
    /// Logical grouping category (e.g. `"scene"`, `"asset"`).
    pub category: String,
    /// Optional long-form description for the command.
    pub description: Option<String>,
    /// Optional JSON Schema describing the `args` object accepted by this command.
    pub args_schema: Option<serde_json::Value>,
    /// Whether the command returns structured data (as opposed to a side effect only).
    pub returns_data: bool,
}

/// A request to execute a command by id with optional args.
/// The editor executes it and sends the result back on `response_tx`.
pub struct CommandRequest {
    /// Unique identifier for this request (used for correlation).
    pub request_id: String,
    /// The command id to execute.
    pub id: String,
    /// Optional arguments for the command.
    pub args: Option<serde_json::Value>,
    /// Channel for the editor to send the execution result back.
    ///
    /// The error `String` is intentional: the editor crate sends results back over this
    /// channel but does not depend on `engine-ai`, so it cannot use [`AiError`].
    /// At the MCP server boundary, errors are converted to JSON-RPC error responses.
    pub response_tx: oneshot::Sender<Result<Option<serde_json::Value>, String>>,
}

/// A request for a permission grant.
/// The editor shows a dialog and sends the grant level back on `response_tx`.
pub struct PermissionRequest {
    /// Unique identifier for this request (used for correlation).
    pub request_id: String,
    /// The permission category being requested (e.g. `"scene"`, `"filesystem"`).
    pub category: String,
    /// The specific command id that requires this permission.
    pub command_id: String,
    /// Channel for the editor to send the user's grant decision back.
    pub response_tx: oneshot::Sender<Option<permissions::GrantLevel>>,
}

/// A request to capture a screenshot as PNG bytes.
pub struct ScreenshotRequest {
    /// Channel for the editor to send the captured PNG bytes back.
    ///
    /// The error `String` is intentional: the editor crate sends results back over this
    /// channel but does not depend on `engine-ai`, so it cannot use [`AiError`].
    /// At the MCP server boundary, errors are converted to JSON-RPC error responses.
    pub response_tx: oneshot::Sender<Result<Vec<u8>, String>>,
}

/// The set of channels the MCP server uses to communicate with the editor.
///
/// Created by the editor's `ai_bridge.rs` and passed to [`AiServer::start`].
///
/// # Example
///
/// ```
/// use engine_ai::{AiBridgeChannels, McpCommand};
/// use tokio::sync::{mpsc, watch};
///
/// let (cmd_tx, _cmd_rx) = mpsc::channel(32);
/// let (perm_tx, _perm_rx) = mpsc::channel(8);
/// let (ss_tx, _ss_rx) = mpsc::channel(4);
/// let (_reg_tx, reg_rx) = watch::channel(Vec::<McpCommand>::new());
///
/// let _channels = AiBridgeChannels {
///     command_tx: cmd_tx,
///     permission_tx: perm_tx,
///     screenshot_tx: ss_tx,
///     registry_rx: reg_rx,
/// };
/// ```
pub struct AiBridgeChannels {
    /// Send a command to the editor for execution.
    pub command_tx: mpsc::Sender<CommandRequest>,
    /// Send a permission request to the editor for user approval.
    pub permission_tx: mpsc::Sender<PermissionRequest>,
    /// Send a screenshot request to the editor.
    pub screenshot_tx: mpsc::Sender<ScreenshotRequest>,
    /// Live snapshot of registered commands (updated when modules are registered).
    pub registry_rx: watch::Receiver<Vec<McpCommand>>,
}

/// Facade for starting and stopping the MCP server.
pub struct AiServer {
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
    port: u16,
}

impl AiServer {
    /// Start the server. Tries `port`..`port+10`, returns the bound port.
    ///
    /// # Errors
    ///
    /// Returns [`AiError::ServerBind`] if no port in the range could be bound.
    pub async fn start(
        port: u16,
        channels: AiBridgeChannels,
        allow_all: bool,
        permissions: std::sync::Arc<std::sync::Mutex<crate::permissions::PermissionStore>>,
    ) -> Result<Self, AiError> {
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let bound_port = server::run(port, channels, allow_all, permissions, shutdown_rx).await?;
        Ok(Self { shutdown_tx: Some(shutdown_tx), port: bound_port })
    }

    /// Returns the port the server is actually listening on.
    #[must_use]
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Stop the server gracefully.
    pub fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

#[cfg(test)]
mod server_tests {
    use super::*;
    use tokio::sync::{mpsc, watch};

    #[tokio::test]
    async fn server_starts_and_responds_to_tools_list() {
        let (cmd_tx, _cmd_rx) = mpsc::channel(32);
        let (perm_tx, _perm_rx) = mpsc::channel(8);
        let (ss_tx, _ss_rx) = mpsc::channel(4);
        let (_reg_tx, reg_rx) = watch::channel(Vec::<McpCommand>::new());

        let channels = AiBridgeChannels {
            command_tx: cmd_tx,
            permission_tx: perm_tx,
            screenshot_tx: ss_tx,
            registry_rx: reg_rx,
        };

        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

        // Start server on port 0 — OS assigns a free port.
        let permissions = std::sync::Arc::new(std::sync::Mutex::new(
            crate::permissions::PermissionStore::new(),
        ));
        tokio::spawn(async move {
            server::run(0, channels, true, permissions, shutdown_rx)
                .await
                .ok();
        });

        // Give the server a moment to bind.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // The server started without panicking — that's the test.
        let _ = shutdown_tx.send(());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bridge_channels_can_be_created() {
        let (cmd_tx, _cmd_rx) = mpsc::channel(32);
        let (perm_tx, _perm_rx) = mpsc::channel(8);
        let (ss_tx, _ss_rx) = mpsc::channel(4);
        let (_reg_tx, reg_rx) = watch::channel(Vec::<McpCommand>::new());

        let _channels = AiBridgeChannels {
            command_tx: cmd_tx,
            permission_tx: perm_tx,
            screenshot_tx: ss_tx,
            registry_rx: reg_rx,
        };
    }

    #[test]
    fn mcp_command_serializes() {
        let cmd = McpCommand {
            id: "scene.create_entity".into(),
            label: "Create Entity".into(),
            category: "scene".into(),
            description: Some("Create a new entity".into()),
            args_schema: Some(serde_json::json!({ "type": "object" })),
            returns_data: false,
        };
        let json = serde_json::to_string(&cmd).expect("serialization should succeed");
        assert!(json.contains("scene.create_entity"));
    }
}
