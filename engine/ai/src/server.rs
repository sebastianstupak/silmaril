//! axum HTTP server — MCP endpoints and SSE stream.

use axum::{
    extract::State,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse,
    },
    routing::{get, post},
    Json, Router,
};
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;
use tokio_stream::wrappers::ReceiverStream;

use crate::{
    mcp::{handle_request, JsonRpcRequest, JsonRpcResponse, McpState},
    permissions::PermissionStore,
    AiBridgeChannels,
};

/// Start the axum server.
///
/// Tries to bind to `0.0.0.0:{port}`, auto-incrementing up to `port + 10` on conflict.
/// Port `0` is handled as a special case: binds once and lets the OS assign a port.
/// Returns the actual bound port.
///
/// # Errors
///
/// Returns an error string if no port in the range could be bound.
pub async fn run(
    port: u16,
    channels: AiBridgeChannels,
    allow_all: bool,
    permissions: Arc<Mutex<PermissionStore>>,
    shutdown_rx: oneshot::Receiver<()>,
) -> Result<u16, crate::AiError> {
    let state = Arc::new(McpState {
        registry_rx: channels.registry_rx,
        command_tx: channels.command_tx,
        permission_tx: channels.permission_tx,
        screenshot_tx: channels.screenshot_tx,
        permissions,
        allow_all,
    });

    let app = Router::new()
        .route("/", get(server_info))
        .route("/mcp", post(mcp_post))
        .route("/mcp/sse", get(mcp_sse))
        .with_state(state);

    // Port 0: let the OS assign any available port — used in tests.
    if port == 0 {
        let listener = tokio::net::TcpListener::bind("0.0.0.0:0")
            .await
            .map_err(|e| crate::AiError::ServerBind(format!("Failed to bind: {}", e)))?;
        let bound_port = listener
            .local_addr()
            .map(|a| a.port())
            .unwrap_or(0);
        tracing::info!(port = bound_port, "MCP server listening");
        axum::serve(listener, app)
            .with_graceful_shutdown(async { let _ = shutdown_rx.await; })
            .await
            .map_err(|e| crate::AiError::ServerBind(format!("Server error: {}", e)))?;
        return Ok(bound_port);
    }

    // Try port, port+1, ..., port+10 to avoid conflicts.
    let max_port = port.saturating_add(10);
    let mut last_err = String::new();
    for try_port in port..=max_port {
        let addr = format!("0.0.0.0:{}", try_port);
        match tokio::net::TcpListener::bind(&addr).await {
            Ok(listener) => {
                let bound_port = listener
                    .local_addr()
                    .map(|a| a.port())
                    .unwrap_or(try_port);
                tracing::info!(port = bound_port, "MCP server listening");
                axum::serve(listener, app)
                    .with_graceful_shutdown(async { let _ = shutdown_rx.await; })
                    .await
                    .map_err(|e| crate::AiError::ServerBind(format!("Server error: {}", e)))?;
                return Ok(bound_port);
            }
            Err(e) => {
                last_err = format!("Port {}: {}", try_port, e);
                tracing::debug!(port = try_port, "Port in use, trying next");
            }
        }
    }
    Err(crate::AiError::ServerBind(format!(
        "No available port in {}..{}: {}",
        port, max_port, last_err
    )))
}

async fn server_info() -> impl IntoResponse {
    Json(serde_json::json!({
        "name": "silmaril-editor",
        "version": "0.1",
        "capabilities": ["tools"]
    }))
}

async fn mcp_post(
    State(state): State<Arc<McpState>>,
    Json(req): Json<JsonRpcRequest>,
) -> Json<JsonRpcResponse> {
    Json(handle_request(req, state).await)
}

async fn mcp_sse(
    State(_state): State<Arc<McpState>>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, std::convert::Infallible>>> {
    // SSE stream — currently sends a heartbeat only.
    // Future: push tool catalog updates when registry changes.
    let (tx, rx) = tokio::sync::mpsc::channel(16);
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            if tx
                .send(Ok(Event::default().comment("heartbeat")))
                .await
                .is_err()
            {
                break;
            }
        }
    });
    Sse::new(ReceiverStream::new(rx)).keep_alive(KeepAlive::default())
}
