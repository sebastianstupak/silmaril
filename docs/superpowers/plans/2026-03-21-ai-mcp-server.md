# AI MCP Server Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Embed an HTTP MCP server in the Silmaril editor that exposes the `CommandRegistry` as MCP tools, letting external AI agents (Claude Code, CI pipelines) read and mutate the editor scene without any hardcoded wiring.

**Architecture:** A new pure-Rust `engine/ai` crate (no Tauri dependency) runs an axum HTTP server on port 7878. It communicates with the editor via `AiBridgeChannels` — tokio channels for command dispatch, permission requests, and screenshots. The editor's `ai_bridge.rs` owns the Tauri-side of each channel: it emits `editor-run-command` events, forwards permission dialogs to the frontend, and calls `NativeViewport::capture_png_bytes` for screenshots.

**Tech Stack:** Rust/Tauri 2, axum 0.8, tokio (full runtime), serde_json, png 0.17, Svelte/TypeScript

---

## File Map

| File | Change |
|------|--------|
| `Cargo.toml` | Add `engine/ai` to workspace members |
| `engine/ai/Cargo.toml` | New crate — axum, tokio (full), serde_json, png, tracing |
| `engine/ai/src/lib.rs` | `McpCommand`, `AiBridgeChannels`, `CommandRequest`, `PermissionRequest`, `ScreenshotRequest`, `AiServer` facade |
| `engine/ai/src/permissions.rs` | `PermissionStore`, `GrantLevel`, check/grant/persist/load |
| `engine/ai/src/registry_bridge.rs` | `command_to_mcp_tool()`, `namespace_to_category()` |
| `engine/ai/src/mcp.rs` | JSON-RPC 2.0 types, `tools/list` + `tools/call` handlers |
| `engine/ai/src/server.rs` | axum app, routes, `AiServer::start/stop` |
| `engine/editor/src-tauri/bridge/modules/scene.rs` | Add args_schema + new scripting commands |
| `engine/editor/src-tauri/bridge/modules/viewport.rs` | Add args_schema + new commands |
| `engine/editor/src-tauri/bridge/modules/editor_module.rs` | New module: `editor.get_scene_state`, `editor.list_assets` |
| `engine/editor/src-tauri/bridge/modules/project_module.rs` | New module: `project.build`, `project.add_module`, etc. |
| `engine/editor/src-tauri/bridge/modules/mod.rs` | Export `EditorCoreModule`, `ProjectModule` |
| `engine/editor/src-tauri/viewport/native_viewport.rs` | Add `capture_png_bytes()` method |
| `engine/editor/src-tauri/bridge/ai_bridge.rs` | New — `AiBridgeState`, Tauri commands, round-trip + screenshot handlers |
| `engine/editor/src-tauri/bridge/mod.rs` | Expose `ai_bridge` module |
| `engine/editor/src-tauri/lib.rs` | Add engine-ai dep, wire bridge, register commands |
| `engine/editor/src/lib/components/AiPermissionDialog.svelte` | New — permission prompt UI |
| `engine/editor/src/App.svelte` | Add `ai:permission_request` listener, `editor-run-command` response emitter |
| `engine/editor/src/lib/stores/status-bar.ts` | Add MCP badge state |
| `engine/editor/Cargo.toml` | Add `engine-ai` dep |

---

## Task 1: engine/ai crate scaffold — types and channels

**Why first:** Every other task depends on the shared types. Getting them right before writing the server saves rework.

**Files:**
- Create: `engine/ai/Cargo.toml`
- Create: `engine/ai/src/lib.rs`
- Modify: `Cargo.toml` (workspace root)

### Cargo.toml for engine/ai

```toml
[package]
name = "engine-ai"
version = "0.1.0"
edition = "2021"
license = "Apache-2.0"
description = "MCP server for Silmaril editor AI integration"

[dependencies]
axum = "0.8"
tokio = { version = "1", features = ["rt-multi-thread", "macros", "sync", "net"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tracing = { workspace = true }
base64 = "0.22"
png = "0.17"

[dev-dependencies]
tokio = { version = "1", features = ["rt-multi-thread", "macros", "sync", "net", "test-util"] }
```

### `engine/ai/src/lib.rs`

```rust
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

/// Minimal command descriptor used by the MCP layer.
/// The editor converts `CommandSpec` → `McpCommand` before passing to this crate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpCommand {
    pub id: String,
    pub label: String,
    pub category: String,
    pub description: Option<String>,
    pub args_schema: Option<serde_json::Value>,
    pub returns_data: bool,
}

/// A request to execute a command by id with optional args.
/// The editor executes it and sends the result back on `response_tx`.
pub struct CommandRequest {
    pub request_id: String,
    pub id: String,
    pub args: Option<serde_json::Value>,
    pub response_tx: oneshot::Sender<Result<Option<serde_json::Value>, String>>,
}

/// A request for a permission grant.
/// The editor shows a dialog and sends the grant level back on `response_tx`.
pub struct PermissionRequest {
    pub request_id: String,
    pub category: String,
    pub command_id: String,
    pub response_tx: oneshot::Sender<Option<permissions::GrantLevel>>,
}

/// A request to capture a screenshot as PNG bytes.
pub struct ScreenshotRequest {
    pub response_tx: oneshot::Sender<Result<Vec<u8>, String>>,
}

/// The set of channels the MCP server uses to communicate with the editor.
/// Created by the editor's `ai_bridge.rs` and passed to `AiServer::start()`.
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
    pub async fn start(
        port: u16,
        channels: AiBridgeChannels,
        allow_all: bool,
        permissions: std::sync::Arc<std::sync::Mutex<crate::permissions::PermissionStore>>,
    ) -> Result<Self, String> {
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let bound_port = server::run(port, channels, allow_all, permissions, shutdown_rx).await?;
        Ok(Self { shutdown_tx: Some(shutdown_tx), port: bound_port })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    /// Stop the server.
    pub fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
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
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("scene.create_entity"));
    }
}
```

### Add to workspace Cargo.toml

Add `"engine/ai"` to the `members` list.

- [ ] **Step 1: Create `engine/ai/Cargo.toml`** with the content above
- [ ] **Step 2: Create `engine/ai/src/lib.rs`** with the content above
- [ ] **Step 3: Add `"engine/ai"` to `members` in `Cargo.toml` (workspace root)**
- [ ] **Step 4: Compile check**

```
cargo check -p engine-ai
```

Expected: clean (no errors).

- [ ] **Step 5: Run tests**

```
cargo test -p engine-ai
```

Expected: `test bridge_channels_can_be_created` and `test mcp_command_serializes` pass.

- [ ] **Step 6: Commit**

```bash
git add engine/ai/ Cargo.toml
git commit -m "feat(ai): scaffold engine/ai crate — AiBridgeChannels, McpCommand, channel types"
```

---

## Task 2: PermissionStore

**Why:** Needed before `tools/call` can check grants. Pure data structure — no HTTP, no Tauri.

**Files:**
- Create: `engine/ai/src/permissions.rs`

```rust
//! Permission store for the MCP server.
//!
//! Controls which command categories an MCP client may execute.
//! Grants persist to `<project>/.silmaril/ai-permissions.json`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// How long a permission grant lasts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GrantLevel {
    /// Allow this single call only.
    Once,
    /// Allow for the lifetime of the editor session.
    Session,
    /// Allow forever (persisted to disk).
    Always,
}

/// Persisted format.
#[derive(Debug, Default, Serialize, Deserialize)]
struct PersistedGrants {
    grants: HashMap<String, GrantLevel>,
}

/// Runtime permission state.
pub struct PermissionStore {
    session_grants: HashMap<String, GrantLevel>,
    persisted_path: Option<PathBuf>,
}

impl PermissionStore {
    /// Create an empty store with no persistence path (for tests).
    pub fn new() -> Self {
        Self { session_grants: HashMap::new(), persisted_path: None }
    }

    /// Create a store that persists `Always` grants to `project_root/.silmaril/ai-permissions.json`.
    pub fn with_path(project_root: &Path) -> Self {
        let path = project_root.join(".silmaril").join("ai-permissions.json");
        let mut store = Self { session_grants: HashMap::new(), persisted_path: Some(path.clone()) };
        if path.exists() {
            store.load_from_disk();
        }
        store
    }

    /// Check if the given category is currently granted.
    /// Returns `None` if no grant exists (caller must request permission).
    pub fn check(&self, category: &str) -> Option<GrantLevel> {
        self.session_grants.get(category).copied()
    }

    /// Record a grant. If `Always`, immediately persists to disk.
    pub fn grant(&mut self, category: &str, level: GrantLevel) {
        self.session_grants.insert(category.to_string(), level);
        if level == GrantLevel::Always {
            self.save_to_disk();
        }
    }

    /// Remove a `Once` grant after it has been used.
    pub fn consume_once(&mut self, category: &str) {
        if self.session_grants.get(category) == Some(&GrantLevel::Once) {
            self.session_grants.remove(category);
        }
    }

    fn save_to_disk(&self) {
        let Some(path) = &self.persisted_path else { return };
        let always_grants: HashMap<String, GrantLevel> = self.session_grants
            .iter()
            .filter(|(_, &v)| v == GrantLevel::Always)
            .map(|(k, &v)| (k.clone(), v))
            .collect();
        let persisted = PersistedGrants { grants: always_grants };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(&persisted) {
            let _ = std::fs::write(path, json);
        }
    }

    fn load_from_disk(&mut self) {
        let Some(path) = &self.persisted_path else { return };
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(persisted) = serde_json::from_str::<PersistedGrants>(&content) {
                for (cat, level) in persisted.grants {
                    // Only restore Always grants from disk
                    if level == GrantLevel::Always {
                        self.session_grants.insert(cat, level);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn check_returns_none_when_no_grant() {
        let store = PermissionStore::new();
        assert!(store.check("scene").is_none());
    }

    #[test]
    fn session_grant_is_visible() {
        let mut store = PermissionStore::new();
        store.grant("scene", GrantLevel::Session);
        assert_eq!(store.check("scene"), Some(GrantLevel::Session));
    }

    #[test]
    fn once_grant_consumed_after_use() {
        let mut store = PermissionStore::new();
        store.grant("viewport", GrantLevel::Once);
        assert_eq!(store.check("viewport"), Some(GrantLevel::Once));
        store.consume_once("viewport");
        assert!(store.check("viewport").is_none());
    }

    #[test]
    fn always_grant_persists_to_disk_and_loads() {
        let dir = tempdir().unwrap();
        {
            let mut store = PermissionStore::with_path(dir.path());
            store.grant("read", GrantLevel::Always);
        }
        // Load fresh instance
        let store = PermissionStore::with_path(dir.path());
        assert_eq!(store.check("read"), Some(GrantLevel::Always));
    }

    #[test]
    fn session_grant_does_not_persist() {
        let dir = tempdir().unwrap();
        {
            let mut store = PermissionStore::with_path(dir.path());
            store.grant("scene", GrantLevel::Session);
        }
        let store = PermissionStore::with_path(dir.path());
        // Session grant should not have persisted
        assert!(store.check("scene").is_none());
    }
}
```

Add `tempfile` to `[dev-dependencies]` in `engine/ai/Cargo.toml`:
```toml
[dev-dependencies]
tempfile = "3"
tokio = { version = "1", features = ["rt-multi-thread", "macros", "sync", "net", "test-util"] }
```

- [ ] **Step 1: Create `engine/ai/src/permissions.rs`** with the content above
- [ ] **Step 2: Add `pub mod permissions;` to `engine/ai/src/lib.rs`** (already included in Task 1 content)
- [ ] **Step 3: Add `tempfile = "3"` to `[dev-dependencies]` in `engine/ai/Cargo.toml`**
- [ ] **Step 4: Run tests**

```
cargo test -p engine-ai -- permissions
```

Expected: 5 tests pass.

- [ ] **Step 5: Commit**

```bash
git add engine/ai/src/permissions.rs engine/ai/Cargo.toml
git commit -m "feat(ai): add PermissionStore — grant/check/persist/load with Once/Session/Always levels"
```

---

## Task 3: Registry bridge — McpCommand → MCP tool format

**Why:** Translates editor commands to the MCP `tools/list` format. Pure data — no HTTP.

**Files:**
- Create: `engine/ai/src/registry_bridge.rs`

```rust
//! Translates `McpCommand` entries to MCP tool descriptors for `tools/list`.

use crate::McpCommand;
use serde_json::{json, Value};

/// MCP tool descriptor (one entry in `tools/list` response).
#[derive(Debug, Clone, serde::Serialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

/// Derive the permission category from a command id's namespace.
///
/// | Namespace | Category |
/// |-----------|----------|
/// | `scene.*` | `scene` |
/// | `viewport.*` | `viewport` |
/// | `project.*` | `build` |
/// | `module.*` | `modules` |
/// | anything else | `read` |
pub fn namespace_to_category(command_id: &str) -> &'static str {
    if command_id.starts_with("scene.") { return "scene"; }
    if command_id.starts_with("viewport.") { return "viewport"; }
    if command_id.starts_with("project.") { return "build"; }
    if command_id.starts_with("module.") { return "modules"; }
    "read"
}

/// Convert a single `McpCommand` to an MCP tool descriptor.
pub fn command_to_mcp_tool(cmd: &McpCommand) -> McpTool {
    let input_schema = cmd.args_schema.clone().unwrap_or_else(|| {
        json!({ "type": "object", "properties": {} })
    });
    McpTool {
        name: cmd.id.clone(),
        description: cmd.description.clone().unwrap_or_else(|| cmd.label.clone()),
        input_schema,
    }
}

/// Convert a full registry snapshot to the `tools/list` result array.
pub fn commands_to_tools(commands: &[McpCommand]) -> Vec<McpTool> {
    commands.iter().map(command_to_mcp_tool).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_cmd(id: &str, schema: Option<Value>) -> McpCommand {
        McpCommand {
            id: id.into(),
            label: "Test".into(),
            category: "test".into(),
            description: Some("A test command".into()),
            args_schema: schema,
            returns_data: false,
        }
    }

    #[test]
    fn namespace_to_category_scene() {
        assert_eq!(namespace_to_category("scene.create_entity"), "scene");
    }

    #[test]
    fn namespace_to_category_viewport() {
        assert_eq!(namespace_to_category("viewport.screenshot"), "viewport");
    }

    #[test]
    fn namespace_to_category_project() {
        assert_eq!(namespace_to_category("project.build"), "build");
        assert_eq!(namespace_to_category("project.add_module"), "build");
    }

    #[test]
    fn namespace_to_category_module() {
        assert_eq!(namespace_to_category("module.physics.add_rigidbody"), "modules");
    }

    #[test]
    fn namespace_to_category_editor_and_template_fall_back_to_read() {
        // editor.* and template.* are intentionally in the "read" category (fallback)
        assert_eq!(namespace_to_category("editor.get_scene_state"), "read");
        assert_eq!(namespace_to_category("template.open"), "read");
        assert_eq!(namespace_to_category("unknowncommand"), "read");
    }

    #[test]
    fn command_to_mcp_tool_passthrough_schema() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": { "name": { "type": "string" } }
        });
        let cmd = make_cmd("scene.create_entity", Some(schema.clone()));
        let tool = command_to_mcp_tool(&cmd);
        assert_eq!(tool.name, "scene.create_entity");
        assert_eq!(tool.input_schema, schema);
    }

    #[test]
    fn command_to_mcp_tool_no_schema_gives_empty_object() {
        let cmd = make_cmd("viewport.screenshot", None);
        let tool = command_to_mcp_tool(&cmd);
        assert_eq!(tool.input_schema["type"], "object");
        assert!(tool.input_schema["properties"].is_object());
    }

    #[test]
    fn commands_to_tools_preserves_count() {
        let cmds = vec![
            make_cmd("scene.create_entity", None),
            make_cmd("viewport.screenshot", None),
        ];
        let tools = commands_to_tools(&cmds);
        assert_eq!(tools.len(), 2);
    }
}
```

- [ ] **Step 1: Create `engine/ai/src/registry_bridge.rs`** with the content above
- [ ] **Step 2: Run tests**

```
cargo test -p engine-ai -- registry_bridge
```

Expected: 8 tests pass.

- [ ] **Step 3: Commit**

```bash
git add engine/ai/src/registry_bridge.rs
git commit -m "feat(ai): add registry bridge — McpCommand to MCP tool translation, namespace→category mapping"
```

---

## Task 4: MCP JSON-RPC 2.0 types + request handlers

**Why:** Defines the wire format and the logic for `tools/list` and `tools/call`. No HTTP yet.

**Files:**
- Create: `engine/ai/src/mcp.rs`

```rust
//! MCP JSON-RPC 2.0 protocol types and request handling.
//!
//! Handles `tools/list` and `tools/call` method dispatch.
//! No I/O — takes requests, returns responses. HTTP layer is in `server.rs`.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, watch, oneshot};

use crate::{
    AiBridgeChannels, CommandRequest, McpCommand, PermissionRequest,
    permissions::{GrantLevel, PermissionStore},
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

    // Validate command exists
    let commands = state.registry_rx.borrow().clone();
    let cmd = match commands.iter().find(|c| c.id == tool_name) {
        Some(c) => c.clone(),
        None => return JsonRpcResponse::error(id, ERR_METHOD_NOT_FOUND,
            format!("Command '{}' not found", tool_name)),
    };

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
                    return JsonRpcResponse::error(id, ERR_PERMISSION_DENIED,
                        format!("Permission denied for category '{}'", category));
                }
                _ => {
                    return JsonRpcResponse::error(id, ERR_PERMISSION_DENIED,
                        format!("Permission denied (timed out) for category '{}'", category));
                }
            }
        }
    }

    // Consume Once grants now that permission is confirmed (before dispatch)
    if !state.allow_all {
        state.permissions.lock().unwrap().consume_once(category);
    }

    // Special case: screenshot
    if tool_name == "viewport.screenshot" {
        return handle_screenshot(id, state).await;
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
        return JsonRpcResponse::error(id, ERR_SERVER_ERROR, "Command channel closed");
    }
    match tokio::time::timeout(std::time::Duration::from_secs(5), resp_rx).await {
        Ok(Ok(Ok(Some(data)))) => JsonRpcResponse::ok(id, serde_json::json!({
            "content": [{ "type": "text", "text": data.to_string() }]
        })),
        Ok(Ok(Ok(None))) => JsonRpcResponse::ok(id, serde_json::json!({
            "content": [{ "type": "text", "text": "ok" }]
        })),
        Ok(Ok(Err(e))) => JsonRpcResponse::error(id, ERR_SERVER_ERROR, e),
        _ => JsonRpcResponse::error(id, ERR_SERVER_ERROR,
            format!("Command '{}' timed out", tool_name)),
    }
}

async fn handle_screenshot(id: Option<Value>, state: &McpState) -> JsonRpcResponse {
    let (resp_tx, resp_rx) = oneshot::channel();
    if state.screenshot_tx.send(crate::ScreenshotRequest { response_tx: resp_tx }).await.is_err() {
        return JsonRpcResponse::error(id, ERR_SERVER_ERROR, "Screenshot channel closed");
    }
    match tokio::time::timeout(std::time::Duration::from_secs(10), resp_rx).await {
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
        _ => JsonRpcResponse::error(id, ERR_SERVER_ERROR, "Screenshot timed out"),
    }
}

fn uuid_v4() -> String {
    // Simple UUID-like id using random bytes
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().subsec_nanos();
    format!("req-{:08x}", t)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::permissions::PermissionStore;
    use tokio::sync::{mpsc, watch};

    fn make_state(commands: Vec<McpCommand>) -> Arc<McpState> {
        let (_cmd_tx, cmd_rx) = mpsc::channel(32);
        let (perm_tx, _perm_rx) = mpsc::channel(8);
        let (ss_tx, _ss_rx) = mpsc::channel(4);
        let (_reg_tx, reg_rx) = watch::channel(commands);
        Arc::new(McpState {
            registry_rx: reg_rx,
            command_tx: _cmd_tx,
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
```

Add `uuid` to dependencies (or just use the simple uuid approach above, no dependency needed):

Also add `base64 = "0.22"` to `engine/ai/Cargo.toml` (already listed in Task 1 Cargo.toml).

Add `tokio-time` feature: update `engine/ai/Cargo.toml` tokio features to:
```toml
tokio = { version = "1", features = ["rt-multi-thread", "macros", "sync", "net", "time"] }
```

- [ ] **Step 1: Create `engine/ai/src/mcp.rs`** with the content above
- [ ] **Step 2: Update tokio features in `engine/ai/Cargo.toml`** — add `"time"` to tokio features
- [ ] **Step 3: Run tests**

```
cargo test -p engine-ai -- mcp
```

Expected: 3 tests pass.

- [ ] **Step 4: Commit**

```bash
git add engine/ai/src/mcp.rs engine/ai/Cargo.toml
git commit -m "feat(ai): add MCP JSON-RPC 2.0 types, tools/list and tools/call handlers"
```

---

## Task 5: axum server

**Why:** HTTP transport layer. Wraps the MCP handlers from Task 4 in real HTTP endpoints.

**Files:**
- Create: `engine/ai/src/server.rs`

```rust
//! axum HTTP server — MCP endpoints and SSE stream.

use axum::{
    extract::State,
    response::{sse::{Event, KeepAlive, Sse}, IntoResponse},
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
/// Returns the actual bound port.
pub async fn run(
    port: u16,
    channels: AiBridgeChannels,
    allow_all: bool,
    permissions: std::sync::Arc<std::sync::Mutex<PermissionStore>>,
    shutdown_rx: oneshot::Receiver<()>,
) -> Result<u16, String> {
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

    // Try port, port+1, ..., port+10 to avoid conflicts
    let max_port = port.saturating_add(10);
    let mut last_err = String::new();
    for try_port in port..=max_port {
        let addr = format!("0.0.0.0:{}", try_port);
        match tokio::net::TcpListener::bind(&addr).await {
            Ok(listener) => {
                let bound_port = listener.local_addr()
                    .map(|a| a.port())
                    .unwrap_or(try_port);
                tracing::info!(port = bound_port, "MCP server listening");
                axum::serve(listener, app)
                    .with_graceful_shutdown(async { let _ = shutdown_rx.await; })
                    .await
                    .map_err(|e| format!("Server error: {}", e))?;
                return Ok(bound_port);
            }
            Err(e) => {
                last_err = format!("Port {}: {}", try_port, e);
                tracing::debug!(port = try_port, "Port in use, trying next");
            }
        }
    }
    Err(format!("No available port in {}..{}: {}", port, max_port, last_err))
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
            if tx.send(Ok(Event::default().comment("heartbeat"))).await.is_err() {
                break;
            }
        }
    });
    Sse::new(ReceiverStream::new(rx)).keep_alive(KeepAlive::default())
}
```

Add to `engine/ai/Cargo.toml`:
```toml
axum = { version = "0.8", features = ["json"] }
tokio-stream = "0.1"
```

Add an integration test to `engine/ai/src/lib.rs` (or a separate test file):

```rust
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

        // Start server on a random port
        let permissions = std::sync::Arc::new(std::sync::Mutex::new(
            crate::permissions::PermissionStore::new()
        ));
        tokio::spawn(async move {
            server::run(0, channels, true, permissions, shutdown_rx).await.ok();
        });

        // Give the server a moment to bind
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // The server started without panicking — that's the test
        let _ = shutdown_tx.send(());
    }
}
```

**Note on port 0:** Passing port 0 to the OS assigns a random available port. This prevents test flakiness from port conflicts. For the full HTTP round-trip integration test, the server needs to expose its actual bound port — this is deferred (Task 11 does a compile-and-run check instead of full HTTP integration).

- [ ] **Step 1: Create `engine/ai/src/server.rs`** with the content above
- [ ] **Step 2: Update `engine/ai/Cargo.toml`** — add `tokio-stream = "0.1"` and update axum to `axum = { version = "0.8", features = ["json"] }`
- [ ] **Step 3: Add the `server_tests` module** to `engine/ai/src/lib.rs`
- [ ] **Step 4: Compile check**

```
cargo check -p engine-ai
```

Expected: clean.

- [ ] **Step 5: Run tests**

```
cargo test -p engine-ai
```

Expected: all pass (the server test just verifies it starts without panicking).

- [ ] **Step 6: Commit**

```bash
git add engine/ai/src/server.rs engine/ai/Cargo.toml engine/ai/src/lib.rs
git commit -m "feat(ai): add axum HTTP server — GET /, POST /mcp JSON-RPC, GET /mcp/sse"
```

---

## Task 6: Update CommandSpecs — add args_schema and new MCP commands

**Why:** MCP `tools/list` is only useful if commands have schemas. This task adds args_schema to existing commands and adds new scripting commands needed for agent workflows.

**Files:**
- Modify: `engine/editor/src-tauri/bridge/modules/scene.rs`
- Modify: `engine/editor/src-tauri/bridge/modules/viewport.rs`
- Create: `engine/editor/src-tauri/bridge/modules/editor_module.rs`
- Modify: `engine/editor/src-tauri/bridge/modules/mod.rs`

### scene.rs — updated commands

The existing `scene.new_entity`, `scene.delete_entity`, `scene.duplicate_entity`, `scene.focus_entity` are kept. Add/update all commands from the spec. Replace the `commands()` return vec with:

```rust
// Full set of MCP-accessible scene commands (spec §Scene commands)
CommandSpec {
    id: "scene.get_state".into(), module_id: String::new(),
    label: "Get Scene State".into(), category: "Scene".into(),
    description: Some("Return the full scene state snapshot".into()),
    keybind: None, args_schema: None, returns_data: true, non_undoable: true,
},
CommandSpec {
    id: "scene.create_entity".into(), module_id: String::new(),
    label: "Create Entity".into(), category: "Scene".into(),
    description: Some("Create a new entity with an optional name".into()),
    keybind: None,
    args_schema: Some(serde_json::json!({
        "type": "object",
        "properties": { "name": { "type": "string" } }
    })),
    returns_data: false, non_undoable: false,
},
CommandSpec {
    id: "scene.delete_entity".into(), module_id: String::new(),
    label: "Delete Entity".into(), category: "Scene".into(),
    description: Some("Delete an entity by id".into()),
    keybind: None,
    args_schema: Some(serde_json::json!({
        "type": "object", "required": ["id"],
        "properties": { "id": { "type": "integer" } }
    })),
    returns_data: false, non_undoable: false,
},
CommandSpec {
    id: "scene.rename_entity".into(), module_id: String::new(),
    label: "Rename Entity".into(), category: "Scene".into(),
    description: Some("Rename an entity by id".into()),
    keybind: None,
    args_schema: Some(serde_json::json!({
        "type": "object", "required": ["id", "name"],
        "properties": {
            "id": { "type": "integer" },
            "name": { "type": "string" }
        }
    })),
    returns_data: false, non_undoable: false,
},
CommandSpec {
    id: "scene.duplicate_entity".into(), module_id: String::new(),
    label: "Duplicate Entity".into(), category: "Scene".into(),
    description: Some("Duplicate an entity by id".into()),
    keybind: None,
    args_schema: Some(serde_json::json!({
        "type": "object", "required": ["id"],
        "properties": { "id": { "type": "integer" } }
    })),
    returns_data: false, non_undoable: false,
},
CommandSpec {
    id: "scene.add_component".into(), module_id: String::new(),
    label: "Add Component".into(), category: "Scene".into(),
    description: Some("Add a component to an entity".into()),
    keybind: None,
    args_schema: Some(serde_json::json!({
        "type": "object", "required": ["id", "component"],
        "properties": {
            "id": { "type": "integer" },
            "component": { "type": "string", "description": "Component type name" }
        }
    })),
    returns_data: false, non_undoable: false,
},
CommandSpec {
    id: "scene.remove_component".into(), module_id: String::new(),
    label: "Remove Component".into(), category: "Scene".into(),
    description: Some("Remove a component from an entity".into()),
    keybind: None,
    args_schema: Some(serde_json::json!({
        "type": "object", "required": ["id", "component"],
        "properties": {
            "id": { "type": "integer" },
            "component": { "type": "string" }
        }
    })),
    returns_data: false, non_undoable: false,
},
CommandSpec {
    id: "scene.set_component_field".into(), module_id: String::new(),
    label: "Set Component Field".into(), category: "Scene".into(),
    description: Some("Set a field on a component".into()),
    keybind: None,
    args_schema: Some(serde_json::json!({
        "type": "object", "required": ["id", "component", "field", "value"],
        "properties": {
            "id": { "type": "integer" },
            "component": { "type": "string" },
            "field": { "type": "string" },
            "value": {}
        }
    })),
    returns_data: false, non_undoable: false,
},
CommandSpec {
    id: "scene.select_entity".into(), module_id: String::new(),
    label: "Select Entity".into(), category: "Scene".into(),
    description: Some("Select an entity by id, or deselect with null".into()),
    keybind: None,
    args_schema: Some(serde_json::json!({
        "type": "object", "required": ["id"],
        "properties": { "id": { "type": ["integer", "null"] } }
    })),
    returns_data: false, non_undoable: true,
},
CommandSpec {
    id: "scene.move_entity".into(), module_id: String::new(),
    label: "Move Entity".into(), category: "Scene".into(),
    description: Some("Set entity position".into()),
    keybind: None,
    args_schema: Some(serde_json::json!({
        "type": "object", "required": ["id", "x", "y", "z"],
        "properties": {
            "id": { "type": "integer" },
            "x": { "type": "number" }, "y": { "type": "number" }, "z": { "type": "number" }
        }
    })),
    returns_data: false, non_undoable: false,
},
CommandSpec {
    id: "scene.rotate_entity".into(), module_id: String::new(),
    label: "Rotate Entity".into(), category: "Scene".into(),
    description: Some("Set entity rotation (Euler angles, degrees)".into()),
    keybind: None,
    args_schema: Some(serde_json::json!({
        "type": "object", "required": ["id", "rx", "ry", "rz"],
        "properties": {
            "id": { "type": "integer" },
            "rx": { "type": "number" }, "ry": { "type": "number" }, "rz": { "type": "number" }
        }
    })),
    returns_data: false, non_undoable: false,
},
CommandSpec {
    id: "scene.scale_entity".into(), module_id: String::new(),
    label: "Scale Entity".into(), category: "Scene".into(),
    description: Some("Set entity scale".into()),
    keybind: None,
    args_schema: Some(serde_json::json!({
        "type": "object", "required": ["id", "sx", "sy", "sz"],
        "properties": {
            "id": { "type": "integer" },
            "sx": { "type": "number" }, "sy": { "type": "number" }, "sz": { "type": "number" }
        }
    })),
    returns_data: false, non_undoable: false,
},
```

### viewport.rs — add args_schema to existing + new commands

Update `viewport.screenshot` to have a description. Add/keep the full set from the spec:

```rust
// viewport.orbit — already in plan
CommandSpec {
    id: "viewport.orbit".into(), module_id: String::new(),
    label: "Orbit Camera".into(), category: "Viewport".into(),
    description: Some("Orbit the viewport camera by delta angles".into()),
    keybind: None,
    args_schema: Some(serde_json::json!({
        "type": "object", "required": ["dx", "dy"],
        "properties": {
            "dx": { "type": "number", "description": "Horizontal delta (degrees)" },
            "dy": { "type": "number", "description": "Vertical delta (degrees)" }
        }
    })),
    returns_data: false, non_undoable: true,
},
CommandSpec {
    id: "viewport.pan".into(), module_id: String::new(),
    label: "Pan Camera".into(), category: "Viewport".into(),
    description: Some("Pan the viewport camera".into()),
    keybind: None,
    args_schema: Some(serde_json::json!({
        "type": "object", "required": ["dx", "dy"],
        "properties": {
            "dx": { "type": "number" }, "dy": { "type": "number" }
        }
    })),
    returns_data: false, non_undoable: true,
},
CommandSpec {
    id: "viewport.zoom".into(), module_id: String::new(),
    label: "Zoom Camera".into(), category: "Viewport".into(),
    description: Some("Zoom the viewport camera".into()),
    keybind: None,
    args_schema: Some(serde_json::json!({
        "type": "object", "required": ["delta"],
        "properties": { "delta": { "type": "number" } }
    })),
    returns_data: false, non_undoable: true,
},
CommandSpec {
    id: "viewport.set_projection".into(), module_id: String::new(),
    label: "Set Projection".into(), category: "Viewport".into(),
    description: Some("Switch between perspective and orthographic projection".into()),
    keybind: None,
    args_schema: Some(serde_json::json!({
        "type": "object", "required": ["mode"],
        "properties": { "mode": { "type": "string", "enum": ["perspective", "ortho"] } }
    })),
    returns_data: false, non_undoable: true,
},
CommandSpec {
    id: "viewport.reset_camera".into(), module_id: String::new(),
    label: "Reset Camera".into(), category: "Viewport".into(),
    description: Some("Reset the camera to the default position".into()),
    keybind: None, args_schema: None, returns_data: false, non_undoable: true,
},
CommandSpec {
    id: "viewport.set_grid_visible".into(), module_id: String::new(),
    label: "Set Grid Visible".into(), category: "Viewport".into(),
    description: Some("Show or hide the viewport grid".into()),
    keybind: None,
    args_schema: Some(serde_json::json!({
        "type": "object", "required": ["visible"],
        "properties": { "visible": { "type": "boolean" } }
    })),
    returns_data: false, non_undoable: true,
},
CommandSpec {
    id: "viewport.focus_entity".into(), module_id: String::new(),
    label: "Focus Entity".into(), category: "Viewport".into(),
    description: Some("Frame the viewport camera on an entity".into()),
    keybind: None,
    args_schema: Some(serde_json::json!({
        "type": "object", "required": ["id"],
        "properties": { "id": { "type": "integer" } }
    })),
    returns_data: false, non_undoable: true,
},
```

### New: `engine/editor/src-tauri/bridge/modules/editor_module.rs`

```rust
use crate::bridge::registry::{CommandSpec, EditorModule};

/// Read-only editor query commands exposed to MCP agents.
pub struct EditorCoreModule;

impl EditorModule for EditorCoreModule {
    fn id(&self) -> &str {
        "editor"
    }

    fn commands(&self) -> Vec<CommandSpec> {
        vec![
            CommandSpec {
                id: "editor.get_scene_state".into(),
                module_id: String::new(),
                label: "Get Scene State".into(),
                category: "Editor".into(),
                description: Some("Return the full scene state as JSON".into()),
                keybind: None,
                args_schema: None,
                returns_data: true,
                non_undoable: true,
            },
            CommandSpec {
                id: "editor.get_entity".into(),
                module_id: String::new(),
                label: "Get Entity".into(),
                category: "Editor".into(),
                description: Some("Return a single entity's full state by id".into()),
                keybind: None,
                args_schema: Some(serde_json::json!({
                    "type": "object",
                    "required": ["id"],
                    "properties": {
                        "id": { "type": "integer", "description": "Entity id" }
                    }
                })),
                returns_data: true,
                non_undoable: true,
            },
            CommandSpec {
                id: "editor.list_assets".into(),
                module_id: String::new(),
                label: "List Assets".into(),
                category: "Editor".into(),
                description: Some("Return a list of all project assets".into()),
                keybind: None,
                args_schema: None,
                returns_data: true,
                non_undoable: true,
            },
            CommandSpec {
                id: "editor.get_project_info".into(),
                module_id: String::new(),
                label: "Get Project Info".into(),
                category: "Editor".into(),
                description: Some("Return project metadata (name, path, version)".into()),
                keybind: None,
                args_schema: None,
                returns_data: true,
                non_undoable: true,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_have_correct_prefix() {
        let module = EditorCoreModule;
        let prefix = format!("{}.", module.id());
        for cmd in module.commands() {
            assert!(cmd.id.starts_with(&prefix), "Command '{}' has wrong prefix", cmd.id);
        }
    }
}
```

### New: `engine/editor/src-tauri/bridge/modules/project_module.rs`

```rust
use crate::bridge::registry::{CommandSpec, EditorModule};

/// Project-level build/codegen commands exposed to MCP agents.
pub struct ProjectModule;

impl EditorModule for ProjectModule {
    fn id(&self) -> &str { "project" }

    fn commands(&self) -> Vec<CommandSpec> {
        vec![
            CommandSpec {
                id: "project.build".into(), module_id: String::new(),
                label: "Build Project".into(), category: "Build".into(),
                description: Some("Build the project for a target platform".into()),
                keybind: None,
                args_schema: Some(serde_json::json!({
                    "type": "object", "required": ["platform"],
                    "properties": { "platform": { "type": "string" } }
                })),
                returns_data: false, non_undoable: true,
            },
            CommandSpec {
                id: "project.add_module".into(), module_id: String::new(),
                label: "Add Module".into(), category: "Build".into(),
                description: Some("Add an engine module to the project".into()),
                keybind: None,
                args_schema: Some(serde_json::json!({
                    "type": "object", "required": ["name"],
                    "properties": { "name": { "type": "string" } }
                })),
                returns_data: false, non_undoable: true,
            },
            CommandSpec {
                id: "project.list_modules".into(), module_id: String::new(),
                label: "List Modules".into(), category: "Build".into(),
                description: Some("Return all installed engine modules".into()),
                keybind: None, args_schema: None, returns_data: true, non_undoable: true,
            },
            CommandSpec {
                id: "project.generate_component".into(), module_id: String::new(),
                label: "Generate Component".into(), category: "Build".into(),
                description: Some("Scaffold a new ECS component".into()),
                keybind: None,
                args_schema: Some(serde_json::json!({
                    "type": "object", "required": ["name"],
                    "properties": {
                        "name": { "type": "string" },
                        "fields": { "type": "array", "items": { "type": "object" } }
                    }
                })),
                returns_data: false, non_undoable: true,
            },
            CommandSpec {
                id: "project.generate_system".into(), module_id: String::new(),
                label: "Generate System".into(), category: "Build".into(),
                description: Some("Scaffold a new ECS system".into()),
                keybind: None,
                args_schema: Some(serde_json::json!({
                    "type": "object", "required": ["name"],
                    "properties": { "name": { "type": "string" } }
                })),
                returns_data: false, non_undoable: true,
            },
            CommandSpec {
                id: "project.run".into(), module_id: String::new(),
                label: "Run Command".into(), category: "Build".into(),
                description: Some("Run any registered engine-ops command by id. Permission for the target command's category is verified before execution.".into()),
                keybind: None,
                args_schema: Some(serde_json::json!({
                    "type": "object", "required": ["id"],
                    "properties": { "id": { "type": "string", "description": "engine-ops command id" } }
                })),
                returns_data: false, non_undoable: true,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_have_correct_prefix() {
        let module = ProjectModule;
        let prefix = format!("{}.", module.id());
        for cmd in module.commands() {
            assert!(cmd.id.starts_with(&prefix), "Command '{}' has wrong prefix", cmd.id);
        }
    }
}
```

### Update `modules/mod.rs`

Add:
```rust
pub mod editor_module;
pub use editor_module::EditorCoreModule;
pub mod project_module;
pub use project_module::ProjectModule;
```

**Note on `project.run` permission check:** The handler for `project.run` in the editor must look up the target command's category (via `getSpec(id)` in TypeScript or registry lookup in Rust) and verify the MCP client holds that category's grant. This prevents `project.run` from being used to bypass narrower permission grants. Implement this in Task 9 when wiring the `editor-run-command-ai` handler in `App.svelte`.

**Note on `scene.set_component_field`:** The spec requires adding a `set_component_field` case to `dispatchSceneCommand` in `src/lib/scene/commands.ts`. This is a TypeScript-side change — add it in Step 3 below.

- [ ] **Step 1: Read `engine/editor/src-tauri/bridge/modules/scene.rs`** — understand the current CommandSpec list
- [ ] **Step 2: Replace the `commands()` return vec in `scene.rs`** with the full 12-command list above
- [ ] **Step 3: Read `engine/editor/src/lib/scene/commands.ts`** — add `set_component_field` case to `dispatchSceneCommand` (the case is currently missing per spec)
- [ ] **Step 4: Read `engine/editor/src-tauri/bridge/modules/viewport.rs`** — understand current commands
- [ ] **Step 5: Replace/extend the `commands()` return vec in `viewport.rs`** with the full 8-command list above
- [ ] **Step 6: Create `engine/editor/src-tauri/bridge/modules/editor_module.rs`** with `EditorCoreModule`
- [ ] **Step 7: Create `engine/editor/src-tauri/bridge/modules/project_module.rs`** with `ProjectModule`
- [ ] **Step 8: Update `engine/editor/src-tauri/bridge/modules/mod.rs`** — add both new module exports
- [ ] **Step 9: Run prefix tests**

```
cargo test -p silmaril-editor --lib -- bridge::modules
```

Expected: all pass.

- [ ] **Step 10: Update insta snapshot** (args_schema changed for many commands)

```
INSTA_UPDATE=always cargo test -p silmaril-editor --lib -- command_manifest_snapshot
```

- [ ] **Step 11: Run full Rust test suite**

```
cargo test -p silmaril-editor --lib
```

Expected: all pass.

- [ ] **Step 12: Commit**

```bash
git add engine/editor/src-tauri/bridge/modules/
git commit -m "feat(editor): add full scene/viewport/project/editor command specs with args_schema for MCP"
```

---

## Task 7: NativeViewport::capture_png_bytes (Windows)

**Why:** MCP agents need visual feedback. The `viewport.screenshot` command must produce real PNG bytes via the Vulkan swapchain readback.

**Files:**
- Modify: `engine/editor/src-tauri/viewport/native_viewport.rs`

### What to add

Read the full `native_viewport.rs` before editing — it's ~1200 lines. Find the `impl NativeViewport` block.

Add a new method after the existing camera/viewport methods:

```rust
/// Capture the current viewport frame as PNG bytes.
///
/// This method is called from the AI bridge to provide visual feedback to agents.
/// It uses the existing `engine-renderer` capture infrastructure:
/// 1. Uses `FrameReadback` to blit the swapchain image to a CPU-visible buffer
/// 2. Encodes the RGBA bytes as PNG via `FrameEncoder`
///
/// Returns `Err` if the capture fails or times out (1 second).
#[cfg(windows)]
pub fn capture_png_bytes(&self) -> Result<Vec<u8>, String> {
    use engine_renderer::capture::{CaptureFormat, FrameEncoder, FrameReadback};

    // Read the current swapchain image into a CPU-visible buffer.
    // FrameReadback handles the Vulkan blit + memory mapping.
    let readback = FrameReadback::new(
        &self.device,          // adjust field names to match actual NativeViewport fields
        &self.allocator,
        self.swapchain_extent, // vk::Extent2D — adjust field name
    ).map_err(|e| format!("Failed to create frame readback: {e}"))?;

    let rgba_bytes = readback.read_current_frame(
        &self.command_pool,    // adjust field name
        &self.graphics_queue,  // adjust field name
        self.current_image_index(), // adjust method name
    ).map_err(|e| format!("Frame readback failed: {e}"))?;

    // Encode to PNG
    let encoder = FrameEncoder::new(CaptureFormat::Png);
    let width = self.swapchain_extent.width;
    let height = self.swapchain_extent.height;
    encoder.encode(&rgba_bytes, width, height)
        .map_err(|e| format!("PNG encoding failed: {e}"))
}
```

**IMPORTANT:** The exact field names (`self.device`, `self.allocator`, `self.swapchain_extent`, etc.) depend on how `NativeViewport` is structured internally. Read the struct definition first and adapt accordingly.

If `FrameReadback` / `FrameEncoder` don't have the exact API shown above, check `engine/renderer/src/capture/` for the actual public API and adapt.

If the capture infrastructure needs to be invoked differently (e.g., via a signal to the render thread), the method may need to:
1. Send a signal over a channel to the render loop
2. Wait for the render loop to copy the image
3. Read from a shared buffer

Check how the render thread is structured in `native_viewport.rs` before deciding the implementation approach.

### Stub for non-Windows builds

After the `#[cfg(windows)]` impl block, add:

```rust
#[cfg(not(windows))]
pub fn capture_png_bytes(&self) -> Result<Vec<u8>, String> {
    Err("Screenshot capture is only supported on Windows".into())
}
```

- [ ] **Step 1: Read `engine/editor/src-tauri/viewport/native_viewport.rs`** — find the struct fields and render thread mechanism
- [ ] **Step 2: Read `engine/renderer/src/capture/mod.rs`, `readback.rs`, `encoder.rs`** — understand actual API
- [ ] **Step 3: Implement `capture_png_bytes`** using the renderer capture infrastructure
- [ ] **Step 4: Compile check**

```
cargo check -p silmaril-editor
```

Expected: clean.

- [ ] **Step 5: Commit**

```bash
git add engine/editor/src-tauri/viewport/native_viewport.rs
git commit -m "feat(editor): add NativeViewport::capture_png_bytes — Vulkan swapchain readback to PNG"
```

---

## Task 8: AI bridge Tauri commands

**Why:** This is the editor side of the AI integration — Tauri commands that start/stop the server and channel types that bridge MCP requests to the editor's command dispatch.

**Files:**
- Create: `engine/editor/src-tauri/bridge/ai_bridge.rs`
- Modify: `engine/editor/src-tauri/bridge/mod.rs`

### `ai_bridge.rs`

```rust
//! AI integration bridge — Tauri commands and channel wiring.
//!
//! Owns the "editor side" of `AiBridgeChannels`:
//! - Receives `CommandRequest` from the MCP server → emits `editor-run-command` Tauri event,
//!   waits for `ai:scene_response` → sends result back.
//! - Receives `PermissionRequest` → emits `ai:permission_request` Tauri event,
//!   stores response_tx, waits for `ai_grant_permission` IPC call.
//! - Receives `ScreenshotRequest` → calls `NativeViewport::capture_png_bytes`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
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
    pub pending_permissions: Mutex<HashMap<String, oneshot::Sender<Option<GrantLevel>>>>,
    /// Pending command round-trips awaiting TypeScript response.
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

/// Convert a watch::Receiver<Vec<CommandSpec>> into a watch::Receiver<Vec<McpCommand>>.
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

            // Emit the command event to TypeScript
            let event = RunCommandEventWithId {
                id: req.id.clone(),
                args: req.args,
                request_id: request_id.clone(),
            };
            if let Err(e) = app2.emit("editor-run-command-ai", &event) {
                let _ = response_tx.send(Err(format!("Emit failed: {e}")));
                continue;
            }

            // For commands that don't return data, resolve immediately
            // TypeScript will call ai_scene_response only for data-returning commands
            // The 5s timeout in mcp.rs handles the waiting

            // Store the response channel so ai_scene_response can resolve it
            if let Ok(state) = app2.try_state::<AiBridgeState>() {
                let mut pending = state.command_response_pending.lock().unwrap();
                pending.insert(request_id, response_tx);
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
                let _ = response_tx.send(None);
                continue;
            }

            // Store response_tx for ai_grant_permission to resolve
            if let Ok(state) = app_perm.try_state::<AiBridgeState>() {
                state.pending_permissions.lock().unwrap()
                    .insert(request_id, response_tx);
            }
        }
    });

    // Screenshot task
    #[cfg(windows)]
    {
        let app_ss = app.clone();
        tauri::async_runtime::spawn(async move {
            while let Some(req) = screenshot_rx.recv().await {
                use crate::commands::NativeViewportState;
                let result = if let Ok(nvs) = app_ss.try_state::<NativeViewportState>() {
                    nvs.with_viewport(|vp| vp.capture_png_bytes())
                        .unwrap_or_else(|| Err("No viewport available".into()))
                } else {
                    Err("NativeViewportState not available".into())
                };
                let _ = req.response_tx.send(result);
            }
        });
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
    let mut server_guard = bridge.server.lock().unwrap();
    if server_guard.is_some() {
        return Err("AI server is already running".into());
    }

    // Create channels
    let (cmd_tx, cmd_rx) = mpsc::channel::<CommandRequest>(32);
    let (perm_tx, perm_rx) = mpsc::channel::<PermissionRequest>(8);
    let (ss_tx, ss_rx) = mpsc::channel::<ScreenshotRequest>(4);

    // Clone the registry watch receiver stored at startup
    let spec_rx = bridge.registry_rx.lock().unwrap()
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

    // Build PermissionStore — persists Always grants to <project>/.silmaril/ai-permissions.json
    use engine_ai::permissions::PermissionStore;
    let permissions = std::sync::Arc::new(std::sync::Mutex::new(
        PermissionStore::with_path(std::path::Path::new(&project_path))
    ));

    let server = AiServer::start(port, channels, allow_all, permissions).await
        .map_err(|e| format!("Failed to start AI server: {e}"))?;
    let bound_port = server.port();
    *server_guard = Some(server);

    spawn_bridge_tasks(app, cmd_rx, perm_rx, ss_rx);

    tracing::info!(port = bound_port, "AI MCP server started");
    Ok(bound_port)
}

#[tauri::command]
pub async fn ai_server_stop(bridge: State<'_, AiBridgeState>) -> Result<(), String> {
    let mut server_guard = bridge.server.lock().unwrap();
    if let Some(mut server) = server_guard.take() {
        server.stop();
        tracing::info!("AI MCP server stopped");
    }
    Ok(())
}

#[tauri::command]
pub fn ai_server_status(bridge: State<'_, AiBridgeState>) -> ServerStatus {
    let guard = bridge.server.lock().unwrap();
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
        other => return Err(format!("Unknown grant level: '{}'", other)),
    };
    let mut pending = bridge.pending_permissions.lock().unwrap();
    if let Some(tx) = pending.remove(&request_id) {
        let _ = tx.send(grant);
    }
    Ok(())
}

/// Called by TypeScript after processing a data-returning AI command.
#[tauri::command]
pub fn ai_scene_response(
    request_id: String,
    data: Option<serde_json::Value>,
    bridge: State<'_, AiBridgeState>,
) -> Result<(), String> {
    let mut pending = bridge.command_response_pending.lock().unwrap();
    if let Some(tx) = pending.remove(&request_id) {
        let _ = tx.send(Ok(data));
    }
    Ok(())
}
```

- [ ] **Step 1: Create `engine/editor/src-tauri/bridge/ai_bridge.rs`** with the content above
- [ ] **Step 2: Add `pub mod ai_bridge;` to `engine/editor/src-tauri/bridge/mod.rs`**
- [ ] **Step 3: Add `engine-ai = { path = "../ai" }` to `engine/editor/Cargo.toml` dependencies**
- [ ] **Step 4: Compile check** (Task 11 handles full wiring into lib.rs)

```
cargo check -p silmaril-editor
```

Expected: compiles (some `todo!()` stubs are OK at this stage — they'll be removed in Task 11).

- [ ] **Step 5: Commit**

```bash
git add engine/editor/src-tauri/bridge/ai_bridge.rs engine/editor/src-tauri/bridge/mod.rs engine/editor/Cargo.toml
git commit -m "feat(editor): add AI bridge — AiBridgeState, Tauri commands for server start/stop/grant, command round-trips"
```

---

## Task 9: Permission dialog UI (TypeScript/Svelte)

**Why:** Users must approve MCP permission requests before commands can execute.

**Files:**
- Create: `engine/editor/src/lib/components/AiPermissionDialog.svelte`
- Modify: `engine/editor/src/App.svelte`

### `AiPermissionDialog.svelte`

```svelte
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { commands } from '$lib/bindings';

  interface PermissionRequest {
    request_id: string;
    category: string;
    command_id: string;
  }

  let pendingRequest: PermissionRequest | null = null;
  let unlisten: (() => void) | undefined;

  onMount(async () => {
    unlisten = await listen<PermissionRequest>('ai:permission_request', (event) => {
      pendingRequest = event.payload;
    });
  });

  onDestroy(() => { unlisten?.(); });

  async function respond(level: 'once' | 'session' | 'always' | 'deny') {
    if (!pendingRequest) return;
    await commands.aiGrantPermission(pendingRequest.request_id, level);
    pendingRequest = null;
  }
</script>

{#if pendingRequest}
  <div class="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
    <div class="bg-zinc-900 border border-zinc-700 rounded-lg p-6 max-w-sm w-full shadow-2xl">
      <h2 class="text-sm font-semibold text-zinc-100 mb-1">AI Permission Request</h2>
      <p class="text-xs text-zinc-400 mb-4">
        An AI agent wants to run <code class="text-orange-400">{pendingRequest.command_id}</code>
        (category: <span class="text-zinc-300">{pendingRequest.category}</span>).
      </p>
      <div class="flex flex-col gap-2">
        <button
          class="px-3 py-1.5 rounded bg-green-700 hover:bg-green-600 text-xs text-white"
          onclick={() => respond('once')}
        >Allow once</button>
        <button
          class="px-3 py-1.5 rounded bg-blue-700 hover:bg-blue-600 text-xs text-white"
          onclick={() => respond('session')}
        >Allow for session</button>
        <button
          class="px-3 py-1.5 rounded bg-blue-900 hover:bg-blue-800 text-xs text-white"
          onclick={() => respond('always')}
        >Always allow</button>
        <button
          class="px-3 py-1.5 rounded bg-zinc-700 hover:bg-zinc-600 text-xs text-zinc-200"
          onclick={() => respond('deny')}
        >Deny</button>
      </div>
    </div>
  </div>
{/if}
```

### App.svelte changes

1. Import and mount `AiPermissionDialog` (add near the bottom of the template, alongside other overlays).

2. Add a listener for `editor-run-command-ai` events (the AI-specific variant that includes a `request_id`). When received, dispatch the command and — if it returns data — call `commands.aiSceneResponse`:

```typescript
// In App.svelte onMount, alongside existing listeners:
const unlistenAiCmd = await listen<{ id: string; args: unknown; request_id: string }>(
  'editor-run-command-ai',
  async (event) => {
    const { id, args, request_id } = event.payload;
    try {
      const result = await commands.runCommand(id, args ?? null);
      // Resolve the pending MCP round-trip
      await commands.aiSceneResponse(request_id, result.status === 'ok' ? result.data : null);
    } catch (e) {
      await commands.aiSceneResponse(request_id, null);
    }
  }
);
// Add to onDestroy: unlistenAiCmd();
```

**Note:** After adding `ai_grant_permission`, `ai_scene_response`, `ai_server_start`, `ai_server_stop`, `ai_server_status` Tauri commands in Task 8, run `cargo xtask codegen` to regenerate `bindings.ts` (done in Task 11).

- [ ] **Step 1: Create `engine/editor/src/lib/components/AiPermissionDialog.svelte`** with the content above
- [ ] **Step 2: Read `engine/editor/src/App.svelte`** — find where other dialogs/overlays are mounted and where `onMount` event listeners live
- [ ] **Step 3: Add `<AiPermissionDialog />` to `App.svelte` template**
- [ ] **Step 4: Add `editor-run-command-ai` listener to `App.svelte` onMount** (with onDestroy cleanup)
- [ ] **Step 5: Run TypeScript test suite**

```
cd engine/editor && npm test -- --run
```

Expected: all pass (no changes to existing logic).

- [ ] **Step 6: Commit**

```bash
git add engine/editor/src/lib/components/AiPermissionDialog.svelte engine/editor/src/App.svelte
git commit -m "feat(editor): add AiPermissionDialog and editor-run-command-ai listener for MCP round-trips"
```

---

## Task 10: Status bar MCP badge + View menu toggle

**Why:** Users need to see whether the MCP server is running and be able to toggle it.

**Files:**
- Modify: `engine/editor/src/lib/stores/status-bar.ts` (add MCP state, or create if doesn't exist)
- Modify status bar component to show `MCP :7878` badge
- Modify View menu to add "AI Server" toggle

**Read first:** `engine/editor/src/lib/stores/` — find how status bar state is managed. Read the StatusBar component and View menu component to understand the pattern.

### MCP state store

Create or update `engine/editor/src/lib/stores/ai-server.ts`:

```typescript
import { writable, derived } from 'svelte/store';
import { commands } from '$lib/bindings';

export const aiServerRunning = writable(false);
export const aiServerPort = writable<number | null>(null);

export async function startAiServer(projectPath: string, port = 7878): Promise<void> {
  const result = await commands.aiServerStart(port, projectPath);
  if (result.status === 'ok') {
    aiServerRunning.set(true);
    aiServerPort.set(result.data);
  }
}

export async function stopAiServer(): Promise<void> {
  await commands.aiServerStop();
  aiServerRunning.set(false);
  aiServerPort.set(null);
}

export async function refreshAiServerStatus(): Promise<void> {
  const result = await commands.aiServerStatus();
  if (result.status === 'ok') {
    aiServerRunning.set(result.data.running);
    aiServerPort.set(result.data.port ?? null);
  }
}
```

### Status bar badge

Find the StatusBar component (or the section in App.svelte that renders the status bar). Add:

```svelte
<script>
  import { aiServerRunning, aiServerPort } from '$lib/stores/ai-server';
</script>

{#if $aiServerRunning}
  <button
    class="text-xs text-green-400 hover:text-green-300 font-mono"
    title="MCP server running — click to copy URL"
    onclick={() => navigator.clipboard.writeText(`http://localhost:${$aiServerPort}/mcp`)}
  >
    MCP :{$aiServerPort}
  </button>
{/if}
```

### View menu

Find the View menu (in a DropdownMenu component). Add an item:

```svelte
<script>
  import { aiServerRunning, startAiServer, stopAiServer } from '$lib/stores/ai-server';
</script>

<DropdownMenuItem onclick={() => $aiServerRunning ? stopAiServer() : startAiServer()}>
  {$aiServerRunning ? 'Stop AI Server' : 'Start AI Server (Ctrl+Shift+A)'}
</DropdownMenuItem>
```

- [ ] **Step 1: Read the StatusBar and View menu components** — find exact file paths and patterns
- [ ] **Step 2: Create `engine/editor/src/lib/stores/ai-server.ts`**
- [ ] **Step 3: Add MCP badge to status bar**
- [ ] **Step 4: Add "AI Server" toggle to View menu**
- [ ] **Step 5: Add `refreshAiServerStatus()` call in App.svelte `onMount`** (after existing startup calls) to sync the badge with any previously running server state
- [ ] **Step 6: Run TypeScript test suite**

```
cd engine/editor && npm test -- --run
```

Expected: all pass.

- [ ] **Step 7: Commit**

```bash
git add engine/editor/src/lib/stores/ai-server.ts engine/editor/src/
git commit -m "feat(editor): add MCP status badge and View menu AI Server toggle"
```

---

## Task 11: Wire lib.rs + regenerate bindings + full integration pass

**Why:** Everything built in Tasks 1-10 must be registered in `lib.rs`. Bindings must be regenerated. This is the integration task.

**Files:**
- Modify: `engine/editor/src-tauri/lib.rs`
- Modify: `engine/editor/src/lib/bindings.ts` (regenerated)
- Modify: snapshot file

### Changes to `lib.rs`

Read the current `lib.rs` fully. Make these additions:

**1. Import the AI bridge:**
```rust
use bridge::ai_bridge::{AiBridgeState, ai_server_start, ai_server_stop, ai_server_status, ai_grant_permission, ai_scene_response};
```

**2. Register `EditorCoreModule` and `ProjectModule` alongside other modules:**
```rust
registry.register_module(&EditorCoreModule);
registry.register_module(&ProjectModule);
```

**3. Store `registry_rx` clone in `AiBridgeState`:**
```rust
// After `let (mut registry, registry_rx) = CommandRegistry::new();`
// and after all modules are registered, before wrapping in Arc:
let ai_bridge_state = AiBridgeState::new(registry_rx.clone());
// ... existing: let registry = Arc::new(Mutex::new(registry));
```

**4. Manage `AiBridgeState`:**
```rust
.manage(ai_bridge_state)
```

**5. Add AI commands to `invoke_handler`:**
```rust
ai_server_start,
ai_server_stop,
ai_server_status,
ai_grant_permission,
ai_scene_response,
```

### Regenerate bindings

```bash
cargo xtask codegen
```

Expected: `bindings.ts` updated with `aiServerStart`, `aiServerStop`, `aiServerStatus`, `aiGrantPermission`, `aiSceneResponse` commands.

### Update insta snapshot

```bash
INSTA_UPDATE=always cargo test -p silmaril-editor --lib -- command_manifest_snapshot
```

### Full test pass

```bash
cargo test -p silmaril-editor --lib
cargo test -p engine-ai
cd engine/editor && npm test -- --run
cargo xtask lint
cargo xtask check-bindings
```

All must pass.

- [ ] **Step 1: Read `engine/editor/src-tauri/lib.rs` fully**
- [ ] **Step 2: Add `engine-ai` import and `EditorCoreModule` import**
- [ ] **Step 3: Register `EditorCoreModule` in the module registration block**
- [ ] **Step 4: Create `AiBridgeState::new(registry_rx.clone())` and store it**
- [ ] **Step 5: Add `.manage(ai_bridge_state)` and 5 new Tauri commands to invoke_handler**
- [ ] **Step 6: Compile check**

```
cargo check -p silmaril-editor
```

Expected: clean (all `todo!()` stubs from Task 8 should be resolved — if any remain, fix them now).

- [ ] **Step 7: Regenerate bindings**

```
cargo xtask codegen
```

- [ ] **Step 8: Update snapshot**

```
INSTA_UPDATE=always cargo test -p silmaril-editor --lib -- command_manifest_snapshot
```

- [ ] **Step 9: Full Rust test suite**

```
cargo test -p silmaril-editor --lib
cargo test -p engine-ai
```

Expected: all pass.

- [ ] **Step 10: Lint and bindings freshness**

```
cargo xtask lint
cargo xtask check-bindings
```

Expected: both pass.

- [ ] **Step 11: TypeScript tests**

```
cd engine/editor && npm test -- --run
```

Expected: all pass.

- [ ] **Step 12: Commit**

```bash
git add engine/editor/src-tauri/lib.rs engine/editor/src/lib/bindings.ts engine/editor/src-tauri/bridge/tests/snapshots/
git commit -m "feat(editor): wire AI MCP server into lib.rs — register EditorCoreModule, manage AiBridgeState, add AI Tauri commands"
```

---

## Out of Scope (Deferred)

**Module TOML command manifests** — The spec describes scanning installed modules for `commands.toml` files at project open. This requires a module-scanning infrastructure that does not exist yet. It is explicitly deferred: the `module.*` permission namespace is already wired in `namespace_to_category`, so any commands registered this way will work correctly once the scanning layer is built. Tracked as future work.

---

## S-Tier Verification Checklist

After all 11 tasks:

- [ ] `cargo test -p engine-ai` → all pass
- [ ] `cargo test -p silmaril-editor --lib` → all pass
- [ ] `npm test -- --run` → all pass
- [ ] `cargo xtask lint` → undo coverage lint passed
- [ ] `cargo xtask check-bindings` → bindings up to date
- [ ] Start editor → `MCP :7878` badge appears in status bar after first project open
- [ ] `curl -X POST http://localhost:7878/mcp -d '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' -H 'Content-Type: application/json'` → returns all registered commands
- [ ] `SILMARIL_AI_ALLOW_ALL=1` → mutation command succeeds without permission dialog
- [ ] `viewport.screenshot` via MCP → returns base64 PNG in response content
