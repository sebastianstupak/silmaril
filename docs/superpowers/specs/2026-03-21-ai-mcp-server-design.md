# Silmaril Editor — AI MCP Server Design

> **For agentic workers:** Use `superpowers:writing-plans` to produce the implementation plan from this spec.

**Goal:** Embed a Model Context Protocol (MCP) server in the Silmaril editor so that external AI agents (Claude Code, CI pipelines, remote orchestrators) can read and manipulate the editor over HTTP without any UI dependency.

**Scope:** MCP server only. The AI chat panel is a separate sub-project that builds on top of this.

---

## Architecture

A new `engine/ai` Rust crate, compiled only when the `ai` Cargo feature is enabled on `silmaril-editor`. The crate owns:

- The MCP HTTP+SSE server (runs inside the Tauri process on a configurable port, default `7878`)
- The tool registry (maps MCP tool names → Rust handler functions)
- The permission store (per-project JSON, permission-gated tool execution)

The editor starts the server when a project is opened. Claude Code or any MCP-compatible client points at `http://localhost:7878`.

```
Claude Code / external agent
        │  HTTP + SSE (MCP protocol)
        ▼
silmaril-editor Tauri process
  ├── engine/ai  ←  MCP server (axum, port 7878)
  │     ├── tool registry
  │     └── permission store
  ├── engine/ops  ←  project ops (build, codegen, module)
  └── Tauri IPC  ←  scene state, viewport screenshot
```

The server runs on a background Tokio task. Tauri commands `ai_server_start` and `ai_server_stop` manage its lifecycle. A `tokio::sync::broadcast` channel carries permission-request events from the tool registry to the Tauri frontend (which shows the permission dialog).

---

## MCP Protocol

Standard MCP over HTTP+SSE ([spec](https://modelcontextprotocol.io)):

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `GET /` | GET | Server info (name, version, capabilities) |
| `POST /mcp` | POST | JSON-RPC 2.0 — `tools/list`, `tools/call` |
| `GET /mcp/sse` | GET | SSE stream for server-initiated messages |

All requests and responses are JSON-RPC 2.0. The server identifies itself as `silmaril-editor/0.1`.

---

## Tool Surface

### Read-only tools

| Tool | Description | Args |
|------|-------------|------|
| `get_scene_state` | Full scene JSON (entities, components, camera) | — |
| `get_entity` | Single entity by id | `id: u64` |
| `viewport_screenshot` | PNG screenshot of the Vulkan viewport (base64) | — |
| `list_assets` | All assets in the open project | — |
| `get_project_info` | Project name, path, game.toml contents | — |

### Scene mutation tools

| Tool | Description | Args |
|------|-------------|------|
| `create_entity` | Create a new entity | `name?: string` |
| `delete_entity` | Delete entity by id | `id: u64` |
| `rename_entity` | Rename entity | `id: u64, name: string` |
| `duplicate_entity` | Duplicate entity | `id: u64` |
| `add_component` | Add component to entity | `id: u64, component: string` |
| `remove_component` | Remove component from entity | `id: u64, component: string` |
| `set_component_field` | Set a field on a component | `id: u64, component: string, field: string, value: any` |
| `select_entity` | Select entity in editor | `id: u64 \| null` |
| `move_entity` | Set entity position | `id: u64, x: f64, y: f64, z: f64` |

### Project operation tools

| Tool | Description | Args |
|------|-------------|------|
| `silm_build` | Build the project for a platform | `platform: string` |
| `silm_add_module` | Add a module to the project | `name: string` |
| `silm_list_modules` | List installed modules | — |
| `silm_run_command` | Run any registered silm command by id | `id: string` |
| `generate_component` | Code-generate a new ECS component | `name: string, fields: FieldSpec[]` |
| `generate_system` | Code-generate a new ECS system | `name: string` |

---

## Permission System

Mirrors Claude Code's permission model. No tool executes without a grant.

### Permission categories

| Category | Tools |
|----------|-------|
| `read` | `get_scene_state`, `get_entity`, `list_assets`, `get_project_info`, `viewport_screenshot` |
| `scene` | All entity/component mutation tools |
| `build` | `silm_build`, `silm_run_command` |
| `codegen` | `generate_component`, `generate_system` |
| `modules` | `silm_add_module`, `silm_list_modules` |

### Grant levels

- **Once** — allow this single call, ask again next time
- **Session** — allow for the lifetime of this editor session
- **Always** — persist to `<project>/.silmaril/ai-permissions.json`

### Flow

1. Tool call arrives at the registry
2. Registry checks the permission store for the tool's category
3. If no grant: emit a `ai:permission_request` Tauri event to the frontend
4. Frontend shows a non-blocking toast/dialog: `"Claude Code wants to use scene mutations — Allow once / This session / Always / Deny"`
5. User responds; grant is stored; tool executes (or returns a permission-denied error)
6. If the server has no connected frontend (headless), deny by default unless `--ai-allow-all` flag is passed at startup (for CI use)

### Persistence format

`<project>/.silmaril/ai-permissions.json`:
```json
{
  "grants": {
    "read": "always",
    "scene": "session",
    "build": null
  }
}
```

---

## Server Lifecycle

- **Auto-start:** when a project is opened and the `ai` feature is compiled in
- **Manual toggle:** View menu → "AI Server" or `Ctrl+Shift+A`
- **Tauri commands:**
  - `ai_server_start(project_path: String, port: u16) → Result<u16, String>` — returns bound port
  - `ai_server_stop() → Result<(), String>`
  - `ai_server_status() → ServerStatus` — `{ running: bool, port: Option<u16> }`
- **Status bar:** shows `MCP :7878` badge when running (clickable to copy URL)
- **Port conflict:** if `7878` is taken, auto-increment to `7879`, `7880`, etc. (up to 10 attempts)

---

## `engine/ai` Crate Structure

```
engine/ai/
├── src/
│   ├── lib.rs           — public API: AiServer, start(), stop()
│   ├── server.rs        — axum HTTP server setup, routes
│   ├── mcp.rs           — MCP JSON-RPC protocol types + dispatcher
│   ├── tools/
│   │   ├── mod.rs       — ToolRegistry, ToolHandler trait
│   │   ├── read.rs      — get_scene_state, get_entity, list_assets, get_project_info
│   │   ├── scene.rs     — entity/component mutation tools
│   │   ├── project.rs   — build, modules, codegen tools
│   │   └── screenshot.rs — viewport_screenshot (Tauri event round-trip)
│   └── permissions.rs   — PermissionStore, grant check, persist to JSON
└── Cargo.toml
```

The crate has no dependency on Tauri. It communicates with the Tauri layer via two channels passed in at startup:
- `scene_tx: mpsc::Sender<SceneCommand>` — to execute scene mutations
- `permission_tx: broadcast::Sender<PermissionRequest>` — to request UI permission grants
- `permission_rx` side held by the Tauri layer, which fires events to the frontend

This keeps `engine/ai` testable without a Tauri runtime.

---

## Screenshot Tool

`viewport_screenshot` works via a request/response pattern:

1. Tool handler sends a `ScreenshotRequest` on a one-shot channel
2. Tauri layer receives it, calls the existing `NativeViewport` screenshot API
3. Raw bytes returned on the one-shot channel
4. Tool encodes to base64 PNG and returns as MCP content type `image`

The viewport screenshot API already exists in `engine/renderer`. This tool is the only one that requires the Tauri process to be running with a visible window.

---

## Integration Points in `silmaril-editor`

- `engine/editor/Cargo.toml` — add `engine-ai` as optional dep under `[features] ai = ["engine-ai"]`
- `src-tauri/lib.rs` — conditionally start `AiServer` after project open, register `ai_server_start/stop/status` commands
- `src-tauri/bridge/ai_bridge.rs` (new) — thin bridge: receives `SceneCommand` from AI crate, calls `dispatchSceneCommand` equivalent in Rust; handles `PermissionRequest` by firing Tauri events
- Status bar component — add `MCP :PORT` badge

---

## Error Handling

- Invalid tool name → JSON-RPC error `-32601 Method not found`
- Permission denied → JSON-RPC error `-32003 Permission denied` with message explaining which category to grant
- Tool execution failure → JSON-RPC error `-32000 Server error` with the underlying error string
- No project open → `-32002 No project open` for all mutation and project tools; read tools return empty state

---

## Testing

- Unit tests in `engine/ai/tests/` — tool registry dispatch, permission store grant/deny/persist logic
- Integration test: start server, call `get_scene_state` via `reqwest`, assert response shape
- Integration test: call a mutation tool without permission, assert `-32003`; grant permission, assert success
- No Tauri runtime required for any test (channels are injected)

---

## Out of Scope (Sub-project 2)

- AI chat panel UI (conversation list, chat view, inline screenshots)
- BYOK provider configuration (Anthropic, OpenRouter, Ollama)
- Per-project chat history persistence
- Agent loop / multi-turn conversation management
