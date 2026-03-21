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
/// Returns the actual bound port immediately — the serve loop runs in a spawned task.
///
/// # Errors
///
/// Returns an error if no port in the range could be bound.
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

    // Bind the listener first, then return the port immediately.
    // axum::serve runs in a spawned task so run() does not block.
    let listener = bind_listener(port).await?;
    let bound_port = listener
        .local_addr()
        .map_err(|e| crate::AiError::ServerBind(e.to_string()))?
        .port();

    tracing::info!(port = bound_port, "MCP server listening");

    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app)
            .with_graceful_shutdown(async { let _ = shutdown_rx.await; })
            .await
        {
            tracing::error!(error = %e, "MCP server error");
        }
    });

    Ok(bound_port)
}

/// Bind a TcpListener to the given port, auto-incrementing up to `port + 10` on conflict.
///
/// Special case: port 0 skips the loop and lets the OS assign any free port.
async fn bind_listener(port: u16) -> Result<tokio::net::TcpListener, crate::AiError> {
    // Port 0: OS assigns a free port immediately.
    if port == 0 {
        return tokio::net::TcpListener::bind("0.0.0.0:0")
            .await
            .map_err(|e| crate::AiError::ServerBind(e.to_string()));
    }

    let max_port = port.saturating_add(10);
    let mut last_err = String::new();
    for try_port in port..=max_port {
        let addr = format!("0.0.0.0:{}", try_port);
        match tokio::net::TcpListener::bind(&addr).await {
            Ok(listener) => return Ok(listener),
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
