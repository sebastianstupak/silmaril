//! AI integration bridge — Tauri commands and channel wiring.
//!
//! Owns the "editor side" of `AiBridgeChannels`:
//! - Receives `CommandRequest` from the MCP server → emits `editor-run-command-ai` Tauri event,
//!   stores response_tx for the TypeScript `ai_scene_response` IPC call.
//! - Receives `PermissionRequest` → emits `ai:permission_request` Tauri event,
//!   stores response_tx for `ai_grant_permission` IPC call.
//! - Receives `ScreenshotRequest` → calls `NativeViewport::capture_png_bytes`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::{mpsc, oneshot, watch};

use engine_ai::{
    AiBridgeChannels, AiServer, CommandRequest, McpCommand, PermissionRequest, ScreenshotRequest,
};
use engine_ai::permissions::GrantLevel;
use crate::bridge::registry::CommandSpec;

// ── State managed by Tauri ────────────────────────────────────────────────────

pub struct AiBridgeState {
    pub server: Mutex<Option<AiServer>>,
    /// Pending permission requests awaiting user response (request_id → response channel).
    /// FIXME: Entries are never TTL-expired; if the frontend crashes, leaked senders
    /// will hold the corresponding MCP oneshot receivers open indefinitely.
    pub pending_permissions: Mutex<HashMap<String, oneshot::Sender<Option<GrantLevel>>>>,
    /// Pending command round-trips awaiting TypeScript response.
    /// FIXME: Entries are never TTL-expired; see pending_permissions note above.
    pub command_response_pending: Mutex<HashMap<String, oneshot::Sender<Result<Option<serde_json::Value>, String>>>>,
    /// Clone of the registry watch receiver — stored here so ai_server_start can clone it.
    pub registry_rx: Mutex<Option<watch::Receiver<Vec<CommandSpec>>>>,
}

impl AiBridgeState {
    pub fn new(registry_rx: watch::Receiver<Vec<CommandSpec>>) -> Self {
        Self {
            server: Mutex::new(None),
            pending_permissions: Mutex::new(HashMap::new()),
            command_response_pending: Mutex::new(HashMap::new()),
            registry_rx: Mutex::new(Some(registry_rx)),
        }
    }
}

// ── Helper: convert CommandSpec → McpCommand ──────────────────────────────────

pub fn spec_to_mcp(spec: &CommandSpec) -> McpCommand {
    McpCommand {
        id: spec.id.clone(),
        label: spec.label.clone(),
        category: spec.category.clone(),
        description: spec.description.clone(),
        args_schema: spec.args_schema.clone(),
        returns_data: spec.returns_data,
    }
}

/// Convert a `watch::Receiver<Vec<CommandSpec>>` into a `watch::Receiver<Vec<McpCommand>>`.
/// Spawns a background task that maps each registry update.
pub fn make_mcp_registry_rx(
    mut spec_rx: watch::Receiver<Vec<CommandSpec>>,
) -> watch::Receiver<Vec<McpCommand>> {
    let initial: Vec<McpCommand> = spec_rx.borrow().iter().map(spec_to_mcp).collect();
    let (mcp_tx, mcp_rx) = watch::channel(initial);
    tokio::spawn(async move {
        loop {
            if spec_rx.changed().await.is_err() { break; }
            let specs = spec_rx.borrow_and_update().clone();
            let cmds: Vec<McpCommand> = specs.iter().map(spec_to_mcp).collect();
            if mcp_tx.send(cmds).is_err() { break; }
        }
    });
    mcp_rx
}

// ── Tauri events ──────────────────────────────────────────────────────────────

#[derive(Serialize, Clone)]
struct RunCommandEventWithId {
    pub id: String,
    pub args: Option<serde_json::Value>,
    pub request_id: String,
}

#[derive(Serialize, Clone)]
pub struct PermissionRequestEvent {
    pub request_id: String,
    pub category: String,
    pub command_id: String,
}

// ── Channel pump tasks ────────────────────────────────────────────────────────

/// Spawn background tasks that forward MCP channel messages to the Tauri event bus.
pub fn spawn_bridge_tasks(
    app: AppHandle,
    mut command_rx: mpsc::Receiver<CommandRequest>,
    mut permission_rx: mpsc::Receiver<PermissionRequest>,
    mut screenshot_rx: mpsc::Receiver<ScreenshotRequest>,
) {
    // Command round-trip task
    let app_cmd = app.clone();
    tauri::async_runtime::spawn(async move {
        while let Some(req) = command_rx.recv().await {
            let app2 = app_cmd.clone();
            let request_id = req.request_id.clone();
            let response_tx = req.response_tx;

            let event = RunCommandEventWithId {
                id: req.id.clone(),
                args: req.args,
                request_id: request_id.clone(),
            };
            if let Err(e) = app2.emit("editor-run-command-ai", &event) {
                tracing::warn!(error = %e, "Failed to emit editor-run-command-ai");
                let _ = response_tx.send(Err(format!("Emit failed: {e}")));
                continue;
            }

            // Store the response channel so ai_scene_response can resolve it
            if let Some(state) = app2.try_state::<AiBridgeState>() {
                state.command_response_pending.lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .insert(request_id.clone(), response_tx);
                tracing::debug!(request_id = %request_id, "Command round-trip pending");
            } else {
                let _ = response_tx.send(Err("AiBridgeState not available".into()));
            }
        }
    });

    // Permission request task
    let app_perm = app.clone();
    tauri::async_runtime::spawn(async move {
        while let Some(req) = permission_rx.recv().await {
            let request_id = req.request_id.clone();
            let response_tx = req.response_tx;

            let event = PermissionRequestEvent {
                request_id: request_id.clone(),
                category: req.category,
                command_id: req.command_id,
            };
            if let Err(e) = app_perm.emit("ai:permission_request", &event) {
                tracing::warn!(error = %e, "Failed to emit ai:permission_request");
                let _ = response_tx.send(None);
                continue;
            }

            if let Some(state) = app_perm.try_state::<AiBridgeState>() {
                state.pending_permissions.lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .insert(request_id.clone(), response_tx);
                tracing::debug!(request_id = %request_id, "Permission request pending");
            } else {
                let _ = response_tx.send(None);
            }
        }
    });

    // Screenshot task — Windows only
    #[cfg(windows)]
    {
        let app_ss = app;
        tauri::async_runtime::spawn(async move {
            while let Some(req) = screenshot_rx.recv().await {
                use crate::bridge::commands::NativeViewportState;
                let result = if let Some(nvs) = app_ss.try_state::<NativeViewportState>() {
                    let registry = nvs.registry.lock()
                        .unwrap_or_else(|p| p.into_inner());
                    registry.first_viewport()
                        .map(|vp: &crate::viewport::native_viewport::NativeViewport| vp.capture_png_bytes())
                        .unwrap_or_else(|| Err("No active viewport".into()))
                } else {
                    Err("NativeViewportState not available".into())
                };
                tracing::debug!(ok = result.is_ok(), "Screenshot request fulfilled");
                let _ = req.response_tx.send(result);
            }
        });
    }

    // Non-Windows: screenshot_rx is dropped; requests will time out on the MCP side
    #[cfg(not(windows))]
    {
        tracing::info!("Screenshot capture is not supported on this platform; screenshot requests will time out");
        let _ = screenshot_rx;
    }
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct ServerStatus {
    pub running: bool,
    pub port: Option<u16>,
}

#[tauri::command]
pub async fn ai_server_start(
    port: u16,
    project_path: String,
    bridge: State<'_, AiBridgeState>,
    app: AppHandle,
) -> Result<u16, String> {
    // Check and early-return while guard is held — drop before await
    {
        let guard = bridge.server.lock().unwrap_or_else(|p| p.into_inner());
        if guard.is_some() {
            return Err("AI server is already running".into());
        }
        // guard dropped here
    }

    let (cmd_tx, cmd_rx) = mpsc::channel::<CommandRequest>(32);
    let (perm_tx, perm_rx) = mpsc::channel::<PermissionRequest>(8);
    let (ss_tx, ss_rx) = mpsc::channel::<ScreenshotRequest>(4);

    let spec_rx = bridge.registry_rx.lock().unwrap_or_else(|p| p.into_inner())
        .as_ref()
        .ok_or("Registry receiver not available")?
        .clone();

    let mcp_rx = make_mcp_registry_rx(spec_rx);
    let channels = AiBridgeChannels {
        command_tx: cmd_tx,
        permission_tx: perm_tx,
        screenshot_tx: ss_tx,
        registry_rx: mcp_rx,
    };

    let allow_all = std::env::var("SILMARIL_AI_ALLOW_ALL").is_ok();

    use engine_ai::permissions::PermissionStore;
    let permissions = Arc::new(Mutex::new(
        PermissionStore::with_path(std::path::Path::new(&project_path))
    ));

    // await is now outside any mutex guard
    let server = AiServer::start(port, channels, allow_all, permissions).await
        .map_err(|e| format!("Failed to start AI server: {e}"))?;
    let bound_port = server.port();

    // Re-acquire after await; handle the TOCTOU race
    let mut server_guard = bridge.server.lock().unwrap_or_else(|p| p.into_inner());
    if server_guard.is_some() {
        // Concurrent call won the race; stop the server we just started
        let mut s = server;
        s.stop();
        return Err("AI server is already running".into());
    }
    *server_guard = Some(server);
    drop(server_guard);

    spawn_bridge_tasks(app, cmd_rx, perm_rx, ss_rx);
    tracing::info!(port = bound_port, "AI MCP server started");
    Ok(bound_port)
}

#[tauri::command]
pub async fn ai_server_stop(bridge: State<'_, AiBridgeState>) -> Result<(), String> {
    let mut server_guard = bridge.server.lock().unwrap_or_else(|p| p.into_inner());
    if let Some(mut server) = server_guard.take() {
        server.stop();
        tracing::info!("AI MCP server stopped");
    }
    Ok(())
}

#[tauri::command]
pub fn ai_server_status(bridge: State<'_, AiBridgeState>) -> ServerStatus {
    let guard = bridge.server.lock().unwrap_or_else(|p| p.into_inner());
    ServerStatus {
        running: guard.is_some(),
        port: guard.as_ref().map(|s| s.port()),
    }
}

#[tauri::command]
pub fn ai_grant_permission(
    request_id: String,
    level: String,
    bridge: State<'_, AiBridgeState>,
) -> Result<(), String> {
    let grant = match level.as_str() {
        "once" => Some(GrantLevel::Once),
        "session" => Some(GrantLevel::Session),
        "always" => Some(GrantLevel::Always),
        "deny" | "" => None,
        other => return Err(format!("Unknown grant level: '{other}'")),
    };
    let mut pending = bridge.pending_permissions.lock().unwrap_or_else(|p| p.into_inner());
    if let Some(tx) = pending.remove(&request_id) {
        let _ = tx.send(grant);
        tracing::debug!(request_id = %request_id, "Permission grant resolved");
    } else {
        tracing::warn!(request_id = %request_id, "No pending permission for request_id");
    }
    Ok(())
}

/// Called by TypeScript after processing a data-returning AI command.
#[tauri::command]
pub fn ai_scene_response(
    request_id: String,
    data: Option<serde_json::Value>,
    error: Option<String>,
    bridge: State<'_, AiBridgeState>,
) -> Result<(), String> {
    let mut pending = bridge.command_response_pending.lock().unwrap_or_else(|p| p.into_inner());
    if let Some(tx) = pending.remove(&request_id) {
        let result = match error {
            Some(e) => Err(e),
            None => Ok(data),
        };
        let _ = tx.send(result);
        tracing::debug!(request_id = %request_id, "Command response resolved");
    } else {
        tracing::warn!(request_id = %request_id, "No pending command for request_id");
    }
    Ok(())
}
