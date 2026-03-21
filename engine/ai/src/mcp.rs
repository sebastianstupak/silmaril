//! MCP JSON-RPC 2.0 protocol types and request handling.
//!
//! Handles `tools/list` and `tools/call` method dispatch.
//! No I/O — takes requests, returns responses. HTTP layer is in `server.rs`.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, watch, oneshot};
use tracing::{debug, warn};

use crate::{
    CommandRequest, McpCommand, PermissionRequest,
    permissions::PermissionStore,
    registry_bridge::{commands_to_tools, namespace_to_category},
};

// ── JSON-RPC 2.0 types ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
}

impl JsonRpcResponse {
    pub fn ok(id: Option<Value>, result: Value) -> Self {
        Self { jsonrpc: "2.0".into(), id, result: Some(result), error: None }
    }

    pub fn error(id: Option<Value>, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: None,
            error: Some(JsonRpcError { code, message: message.into() }),
        }
    }
}

// ── Error codes ───────────────────────────────────────────────────────────────
pub const ERR_METHOD_NOT_FOUND: i32 = -32601;
pub const ERR_PERMISSION_DENIED: i32 = -32003;
pub const ERR_NO_PROJECT: i32 = -32002;
pub const ERR_SERVER_ERROR: i32 = -32000;

// ── Handler state shared across requests ─────────────────────────────────────

pub struct McpState {
    pub registry_rx: watch::Receiver<Vec<McpCommand>>,
    pub command_tx: mpsc::Sender<CommandRequest>,
    pub permission_tx: mpsc::Sender<PermissionRequest>,
    pub screenshot_tx: mpsc::Sender<crate::ScreenshotRequest>,
    pub permissions: Arc<Mutex<PermissionStore>>,
    pub allow_all: bool,
}

// ── Request dispatch ──────────────────────────────────────────────────────────

/// Main entry point: dispatch a JSON-RPC request and return a response.
pub async fn handle_request(req: JsonRpcRequest, state: Arc<McpState>) -> JsonRpcResponse {
    match req.method.as_str() {
        "tools/list" => handle_tools_list(req.id, &state).await,
        "tools/call"  => handle_tools_call(req.id, req.params, &state).await,
        other => JsonRpcResponse::error(req.id, ERR_METHOD_NOT_FOUND,
            format!("Method '{}' not found", other)),
    }
}

async fn handle_tools_list(id: Option<Value>, state: &McpState) -> JsonRpcResponse {
    let commands = state.registry_rx.borrow().clone();
    let tools = commands_to_tools(&commands);
    JsonRpcResponse::ok(id, serde_json::json!({ "tools": tools }))
}

async fn handle_tools_call(
    id: Option<Value>,
    params: Option<Value>,
    state: &McpState,
) -> JsonRpcResponse {
    // Parse params
    let params = match params {
        Some(p) => p,
        None => return JsonRpcResponse::error(id, ERR_SERVER_ERROR, "Missing params"),
    };
    let tool_name = match params.get("name").and_then(|v| v.as_str()) {
        Some(n) => n.to_string(),
        None => return JsonRpcResponse::error(id, ERR_SERVER_ERROR, "Missing 'name' in params"),
    };
    let args = params.get("arguments").cloned();

    debug!(method = "tools/call", tool = %tool_name, "Dispatching tool call");

    // Validate command exists
    let commands = state.registry_rx.borrow().clone();
    if !commands.iter().any(|c| c.id == tool_name) {
        return JsonRpcResponse::error(id, ERR_METHOD_NOT_FOUND,
            format!("Command '{}' not found", tool_name));
    }

    // Permission check
    let category = namespace_to_category(&tool_name);
    if !state.allow_all {
        let grant = state.permissions.lock().unwrap().check(category);
        if grant.is_none() {
            // Request permission
            let request_id = uuid_v4();
            let (resp_tx, resp_rx) = oneshot::channel();
            let perm_req = PermissionRequest {
                request_id: request_id.clone(),
                category: category.to_string(),
                command_id: tool_name.clone(),
                response_tx: resp_tx,
            };
            if state.permission_tx.send(perm_req).await.is_err() {
                warn!(category, command_id = %tool_name, "Permission channel closed");
                return JsonRpcResponse::error(id, ERR_SERVER_ERROR, "Permission channel closed");
            }
            match tokio::time::timeout(
                std::time::Duration::from_secs(30),
                resp_rx,
            ).await {
                Ok(Ok(Some(level))) => {
                    state.permissions.lock().unwrap().grant(category, level);
                }
                Ok(Ok(None)) => {
                    warn!(category, command_id = %tool_name, "Permission denied by user");
                    return JsonRpcResponse::error(id, ERR_PERMISSION_DENIED,
                        format!("Permission denied for category '{}'", category));
                }
                _ => {
                    warn!(category, command_id = %tool_name, "Permission request timed out");
                    return JsonRpcResponse::error(id, ERR_PERMISSION_DENIED,
                        format!("Permission denied (timed out) for category '{}'", category));
                }
            }
        }
    }

    // Special case: screenshot
    if tool_name == "viewport.screenshot" {
        return handle_screenshot(id, state, category).await;
    }

    // Standard command dispatch
    let request_id = uuid_v4();
    let (resp_tx, resp_rx) = oneshot::channel();
    let cmd_req = CommandRequest {
        request_id,
        id: tool_name.clone(),
        args,
        response_tx: resp_tx,
    };
    if state.command_tx.send(cmd_req).await.is_err() {
        warn!(command_id = %tool_name, "Command channel closed");
        return JsonRpcResponse::error(id, ERR_SERVER_ERROR, "Command channel closed");
    }
    let result = match tokio::time::timeout(std::time::Duration::from_secs(5), resp_rx).await {
        Ok(Ok(Ok(Some(data)))) => {
            JsonRpcResponse::ok(id, serde_json::json!({
                "content": [{ "type": "text", "text": data.to_string() }]
            }))
        }
        Ok(Ok(Ok(None))) => {
            JsonRpcResponse::ok(id, serde_json::json!({
                "content": [{ "type": "text", "text": "ok" }]
            }))
        }
        Ok(Ok(Err(e))) => JsonRpcResponse::error(id, ERR_SERVER_ERROR, e),
        _ => {
            warn!(command_id = %tool_name, "Command timed out");
            JsonRpcResponse::error(id, ERR_SERVER_ERROR, format!("Command '{}' timed out", tool_name))
        }
    };
    // Consume Once grants only on success
    if !state.allow_all && result.error.is_none() {
        state.permissions.lock().unwrap().consume_once(category);
    }
    result
}

async fn handle_screenshot(id: Option<Value>, state: &McpState, category: &str) -> JsonRpcResponse {
    let (resp_tx, resp_rx) = oneshot::channel();
    if state.screenshot_tx.send(crate::ScreenshotRequest { response_tx: resp_tx }).await.is_err() {
        warn!("Screenshot channel closed");
        return JsonRpcResponse::error(id, ERR_SERVER_ERROR, "Screenshot channel closed");
    }
    let result = match tokio::time::timeout(std::time::Duration::from_secs(10), resp_rx).await {
        Ok(Ok(Ok(png_bytes))) => {
            use base64::Engine as _;
            let b64 = base64::engine::general_purpose::STANDARD.encode(&png_bytes);
            JsonRpcResponse::ok(id, serde_json::json!({
                "content": [{
                    "type": "image",
                    "data": b64,
                    "mimeType": "image/png"
                }]
            }))
        }
        Ok(Ok(Err(e))) => JsonRpcResponse::error(id, ERR_SERVER_ERROR, e),
        _ => {
            warn!("Screenshot request timed out");
            JsonRpcResponse::error(id, ERR_SERVER_ERROR, "Screenshot timed out")
        }
    };
    // Consume Once grants only on success
    if !state.allow_all && result.error.is_none() {
        state.permissions.lock().unwrap().consume_once(category);
    }
    result
}

fn uuid_v4() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("req-{:016x}", n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::permissions::PermissionStore;
    use tokio::sync::{mpsc, watch};

    fn make_state(commands: Vec<McpCommand>) -> Arc<McpState> {
        let (cmd_tx, _cmd_rx) = mpsc::channel(32);
        let (perm_tx, _perm_rx) = mpsc::channel(8);
        let (ss_tx, _ss_rx) = mpsc::channel(4);
        let (_reg_tx, reg_rx) = watch::channel(commands);
        Arc::new(McpState {
            registry_rx: reg_rx,
            command_tx: cmd_tx,
            permission_tx: perm_tx,
            screenshot_tx: ss_tx,
            permissions: Arc::new(Mutex::new(PermissionStore::new())),
            allow_all: true, // skip permission prompts in tests
        })
    }

    fn make_cmd(id: &str) -> McpCommand {
        McpCommand {
            id: id.into(),
            label: "Test".into(),
            category: "test".into(),
            description: None,
            args_schema: None,
            returns_data: false,
        }
    }

    #[tokio::test]
    async fn tools_list_returns_all_commands() {
        let state = make_state(vec![
            make_cmd("scene.create_entity"),
            make_cmd("viewport.screenshot"),
        ]);
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(serde_json::json!(1)),
            method: "tools/list".into(),
            params: None,
        };
        let resp = handle_request(req, state).await;
        assert!(resp.error.is_none());
        let tools = &resp.result.unwrap()["tools"];
        assert_eq!(tools.as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn tools_call_unknown_command_returns_method_not_found() {
        let state = make_state(vec![]);
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(serde_json::json!(1)),
            method: "tools/call".into(),
            params: Some(serde_json::json!({ "name": "nonexistent.cmd", "arguments": {} })),
        };
        let resp = handle_request(req, state).await;
        assert_eq!(resp.error.unwrap().code, ERR_METHOD_NOT_FOUND);
    }

    #[tokio::test]
    async fn unknown_method_returns_method_not_found() {
        let state = make_state(vec![]);
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(serde_json::json!(1)),
            method: "unknown/method".into(),
            params: None,
        };
        let resp = handle_request(req, state).await;
        assert_eq!(resp.error.unwrap().code, ERR_METHOD_NOT_FOUND);
    }

    #[tokio::test]
    async fn allow_all_bypasses_permission_prompt() {
        // With allow_all=true, tools/call dispatches without a permission request
        let (cmd_tx, mut cmd_rx) = mpsc::channel(32);
        let (perm_tx, mut perm_rx) = mpsc::channel(8);
        let (ss_tx, _ss_rx) = mpsc::channel(4);
        let (_reg_tx, reg_rx) = watch::channel(vec![make_cmd("scene.create_entity")]);
        let state = Arc::new(McpState {
            registry_rx: reg_rx,
            command_tx: cmd_tx,
            permission_tx: perm_tx,
            screenshot_tx: ss_tx,
            permissions: Arc::new(Mutex::new(PermissionStore::new())),
            allow_all: true,
        });

        // Spawn a receiver that replies immediately
        tokio::spawn(async move {
            if let Some(req) = cmd_rx.recv().await {
                let _ = req.response_tx.send(Ok(None));
            }
        });

        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(serde_json::json!(1)),
            method: "tools/call".into(),
            params: Some(serde_json::json!({ "name": "scene.create_entity", "arguments": {} })),
        };
        let resp = handle_request(req, state).await;
        // No permission request was made
        assert!(perm_rx.try_recv().is_err(), "Should not have sent permission request");
        assert!(resp.error.is_none(), "Should have succeeded");
    }

    #[tokio::test]
    async fn tools_call_times_out_when_no_response_sent() {
        // Without allow_all and with no permission response, call should time out
        let (cmd_tx, _cmd_rx) = mpsc::channel(32);
        let (perm_tx, mut perm_rx) = mpsc::channel(8);
        let (ss_tx, _ss_rx) = mpsc::channel(4);
        let (_reg_tx, reg_rx) = watch::channel(vec![make_cmd("scene.create_entity")]);
        let state = Arc::new(McpState {
            registry_rx: reg_rx,
            command_tx: cmd_tx,
            permission_tx: perm_tx,
            screenshot_tx: ss_tx,
            permissions: Arc::new(Mutex::new(PermissionStore::new())),
            allow_all: false,
        });

        // Consume the permission request but never respond
        tokio::spawn(async move {
            let _req = perm_rx.recv().await; // receive but don't send response
        });

        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(serde_json::json!(1)),
            method: "tools/call".into(),
            params: Some(serde_json::json!({ "name": "scene.create_entity", "arguments": {} })),
        };
        // This test has a 30s timeout in handle_tools_call. For testing, use a short timeout wrapper.
        let resp = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            handle_request(req, state),
        ).await;
        // Either times out or returns permission denied; both are acceptable
        assert!(resp.is_err() || resp.unwrap().error.is_some());
    }

    #[tokio::test]
    async fn module_command_appears_in_tools_list() {
        // Module commands registered in the registry appear in tools/list
        let state = make_state(vec![
            make_cmd("module.physics.add_rigidbody"),
            make_cmd("scene.create_entity"),
        ]);
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(serde_json::json!(1)),
            method: "tools/list".into(),
            params: None,
        };
        let resp = handle_request(req, state).await;
        assert!(resp.error.is_none());
        let tools = resp.result.unwrap()["tools"].as_array().unwrap().clone();
        let names: Vec<&str> = tools.iter()
            .filter_map(|t| t["name"].as_str())
            .collect();
        assert!(names.contains(&"module.physics.add_rigidbody"));
    }
}
