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
  └── ai_bridge.rs  ←  Tauri bridge (event round-trips to TypeScript)
```

The server runs on a background Tokio task. Tauri commands `ai_server_start` and `ai_server_stop` manage its lifecycle.

**`engine/ai` has no Tauri dependency.** It communicates with the Tauri layer via channels injected at startup (see Bridge section below). This keeps the crate fully testable without a Tauri runtime.

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

| Tool | Description | Args | Returns |
|------|-------------|------|---------|
| `get_scene_state` | Full scene (entities, components, camera) | — | `SceneSnapshot` JSON |
| `get_entity` | Single entity by id | `id: u64` | `EntitySnapshot` JSON |
| `viewport_screenshot` | PNG of the Vulkan viewport | — | base64 PNG string |
| `list_assets` | All assets in the open project | — | `AssetInfo[]` JSON |
| `get_project_info` | Project name, path, game.toml contents | — | `ProjectInfo` JSON |

**`SceneSnapshot` schema** mirrors the TypeScript `SceneState` type in `src/lib/scene/state.ts`:
```typescript
{
  entities: Array<{
    id: number;
    name: string;
    components: string[];
    position: { x: number; y: number; z: number };
    rotation: { x: number; y: number; z: number };
    scale:    { x: number; y: number; z: number };
    visible: boolean;
    locked: boolean;
    componentValues: Record<string, Record<string, unknown>>;
  }>;
  selectedEntityId: number | null;
  camera: {
    position: { x: number; y: number; z: number };
    target:   { x: number; y: number; z: number };
    zoom: number;
    fov: number;
    viewAngle: number;
    projection: 'perspective' | 'ortho';
  };
  gridVisible: boolean;
  snapToGrid: boolean;
  gridSize: number;
  // activeTool and nextEntityId are intentionally excluded: activeTool is
  // editor UI state (not game data), and nextEntityId is an internal counter
  // that agents should not depend on.
}
```

`EntitySnapshot` is one element of `entities` above.

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
| `silm_run_command` | Run a registered silm command by id | `id: string` |
| `generate_component` | Code-generate a new ECS component | `name: string, fields: FieldSpec[]` |
| `generate_system` | Code-generate a new ECS system | `name: string` |

**Note on `set_component_field`:** `dispatchSceneCommand` in `src/lib/scene/commands.ts` currently has no `set_component_field` case. The implementation must add one that calls the existing `setComponentField(entityId, componentName, fieldName, value)` function.

**Note on `silm_run_command`:** this tool requires the `build` permission category. Before executing, the bridge must look up the command's own category in the `CommandRegistry` and verify the caller also holds that category's grant. This prevents using `silm_run_command` to bypass narrower permission grants.

---

## Permission System

Mirrors Claude Code's permission model. No mutation or build tool executes without a grant. Read-only tools (`read` category) require a grant too — they are just granted more readily.

### Permission categories

| Category | Tools |
|----------|-------|
| `read` | `get_scene_state`, `get_entity`, `list_assets`, `get_project_info`, `viewport_screenshot` |
| `scene` | All entity/component mutation tools (`create_entity`, `delete_entity`, `rename_entity`, `duplicate_entity`, `add_component`, `remove_component`, `set_component_field`, `select_entity`, `move_entity`) |
| `build` | `silm_build`, `silm_run_command` |
| `codegen` | `generate_component`, `generate_system` |
| `modules` | `silm_add_module`, `silm_list_modules` |

### Grant levels

- **Once** — allow this single call, ask again next time
- **Session** — allow for the lifetime of this editor session
- **Always** — persist to `<project>/.silmaril/ai-permissions.json`

### Permission request flow

1. Tool call arrives at the registry.
2. Registry checks the permission store for the tool's category.
3. If no grant exists: tool handler sends a `PermissionRequest { category, tool_name, response_tx: oneshot::Sender<GrantLevel> }` to the Tauri bridge via `permission_tx: mpsc::Sender<PermissionRequest>`.
4. The Tauri bridge fires a `ai:permission_request` Tauri event to the frontend.
5. Frontend shows a non-blocking dialog: `"Claude Code wants to use <category> — Allow once / This session / Always / Deny"`.
6. User clicks a response. Frontend calls the Tauri command `ai_grant_permission(category: String, level: String)`.
7. The Tauri bridge sends the grant level back on `response_tx`.
8. The permission store records the grant (for Session/Always). The tool handler resumes.
9. **Timeout:** if no response arrives within **30 seconds**, the permission request resolves as `Deny` and the tool returns JSON-RPC error `-32003 Permission denied (timed out)`.

### Headless / CI mode

When the editor is started with the environment variable `SILMARIL_AI_ALLOW_ALL=1` (or CLI flag `--ai-allow-all`), all permission requests are auto-granted as `Session`. This is intended only for CI pipelines where no human is present to approve dialogs. **Security note:** do not set this flag in development or production environments; it allows any MCP client on the machine to mutate the project without prompts.

### Persistence format

`<project>/.silmaril/ai-permissions.json`:
```json
{
  "grants": {
    "read": "always",
    "scene": "session",
    "build": null,
    "codegen": null,
    "modules": null
  }
}
```

---

## Tauri Bridge (`src-tauri/bridge/ai_bridge.rs`)

The bridge is the glue layer between `engine/ai` (no Tauri) and the Tauri runtime. It owns:

1. **Scene command round-trip:** All AI-driven scene mutations and reads go through the TypeScript scene state (which is the source of truth). The bridge converts an `AiSceneCommand` enum into a Tauri event (`ai:scene_command { command, args }`) fired at the WebView. TypeScript receives it, calls `dispatchSceneCommand(command, args)`, then for commands that return data (read tools), posts a `ai:scene_response { request_id, data }` Tauri event back. The bridge correlates request and response via a `request_id` uuid, with a `oneshot::Receiver` waiting on the Rust side. **Timeout: 5 seconds.**

2. **Permission request handling:** Receives `PermissionRequest` from `engine/ai` via `mpsc::Receiver`, fires `ai:permission_request` Tauri event, registers the `response_tx` in a `HashMap<request_id, oneshot::Sender<GrantLevel>>`. When `ai_grant_permission(request_id, level)` Tauri command is called by the frontend, the bridge looks up and sends on the oneshot.

3. **Screenshot:** Receives a `ScreenshotRequest` from `engine/ai` via `mpsc::Receiver`. The bridge calls `NativeViewport::capture_png_bytes()` directly (no TypeScript round-trip needed — the capture logic is pure Rust/Vulkan). It base64-encodes the result and sends it back on the `oneshot::Sender` embedded in the request. **Timeout: 10 seconds.**

**New Tauri commands exposed by the bridge:**

| Command | Args | Purpose |
|---------|------|---------|
| `ai_server_start` | `project_path: String, port: u16` | Start MCP server, returns bound port |
| `ai_server_stop` | — | Stop MCP server |
| `ai_server_status` | — | `{ running: bool, port: Option<u16> }` |
| `ai_grant_permission` | `request_id: String, level: String` | Resolve pending permission request |

---

## NativeViewport Screenshot API (New)

The `NativeViewport` struct in `src-tauri/viewport/native_viewport.rs` currently has no screenshot method. A new method must be added:

```rust
pub fn capture_png_bytes(&self) -> Result<Vec<u8>, String>
```

This method:
1. Signals the render thread to copy the current swapchain image into a CPU-readable buffer
2. Waits for the copy to complete (blocking, max 1 second)
3. Encodes the raw RGBA bytes as PNG using the `png` crate
4. Returns the PNG bytes

The render thread already has access to the `vk::Device`, `vk::Queue`, and swapchain images required for a readback blit. The `engine-renderer` crate's `capture` module (`src/capture/mod.rs`) contains the Vulkan blit + readback logic and can be reused.

---

## `engine/ai` Crate Structure

```
engine/ai/
├── src/
│   ├── lib.rs              — public API: AiServer, AiBridge channels, start(), stop()
│   ├── server.rs           — axum HTTP server setup, routes
│   ├── mcp.rs              — MCP JSON-RPC 2.0 protocol types + dispatcher
│   ├── tools/
│   │   ├── mod.rs          — ToolRegistry, ToolHandler trait, request/response types
│   │   ├── read.rs         — get_scene_state, get_entity, list_assets, get_project_info
│   │   ├── scene.rs        — entity/component mutation tools
│   │   ├── project.rs      — build, modules, codegen tools
│   │   └── screenshot.rs   — viewport_screenshot round-trip
│   └── permissions.rs      — PermissionStore, grant check, persist to JSON
└── Cargo.toml
```

**Channels injected at `AiServer::new(...)`:**

```rust
pub struct AiBridgeChannels {
    /// Send a scene command to TypeScript and await the response.
    pub scene_tx: mpsc::Sender<SceneRequest>,
    /// Send a permission request and await the user's grant.
    pub permission_tx: mpsc::Sender<PermissionRequest>,
    /// Request a viewport screenshot and await PNG bytes.
    pub screenshot_tx: mpsc::Sender<ScreenshotRequest>,
}
```

All three are `mpsc::Sender`s; the Rust side of each embeds a `oneshot::Sender` for the response, making the round-trip async without coupling to Tauri.

---

## Server Lifecycle

- **Auto-start:** when a project is opened and the `ai` feature is compiled in
- **Manual toggle:** View menu → "AI Server" or `Ctrl+Shift+A`
- **Port conflict:** if `7878` is taken, auto-increment to `7879`…`7888` (10 attempts), then error
- **Status bar:** `MCP :7878` badge shown when running (clickable copies `http://localhost:7878` to clipboard)

---

## Error Handling

| Situation | JSON-RPC error |
|-----------|----------------|
| Unknown tool name | `-32601 Method not found` |
| Permission denied | `-32003 Permission denied` (includes category name) |
| Permission request timed out | `-32003 Permission denied (timed out waiting for user response)` |
| No project open | `-32002 No project open` (mutation + project tools only; reads return empty state) |
| Tool execution failure | `-32000 Server error` with underlying error string |
| Scene response timed out | `-32000 Server error (scene command timed out)` |
| Screenshot timed out | `-32000 Server error (screenshot timed out)` |

---

## Known Limitations

- **Undo/redo:** AI-driven scene mutations go through `dispatchSceneCommand` in TypeScript, which does NOT currently push to the undo stack. The user cannot Ctrl+Z AI changes. This is a known limitation for v1; a future version should route AI mutations through the undo-aware command processor.
- **Headless scene reads:** `get_scene_state` requires the TypeScript WebView to be running and responsive. Fully headless read of scene state is not supported in v1.

---

## Testing

- **Unit — permission store:** grant/deny/persist/load, timeout simulation
- **Unit — tool registry:** dispatch to correct handler, unknown tool returns `-32601`
- **Unit — permission bypass check for `silm_run_command`:** command with `scene` category denied when only `build` is granted
- **Integration — read tools:** start server, call `get_scene_state` via `reqwest`, assert response matches `SceneSnapshot` schema
- **Integration — permission round-trip:** call a mutation tool; inject simulated grant on `permission_rx` side; assert tool succeeds. Inject no response; assert `-32003` after timeout.
- **Integration — permission denied:** call mutation tool with no grant and no injected response; assert `-32003`
- **Integration — `SILMARIL_AI_ALLOW_ALL`:** set env var, call mutation tool, assert no permission prompt and success

No Tauri runtime required for any test; all channels are injected via `AiBridgeChannels`.

---

## Out of Scope (Sub-project 2)

- AI chat panel UI (conversation list, chat view, inline screenshots)
- BYOK provider configuration (Anthropic, OpenRouter, Ollama)
- Per-project chat history persistence
- Agent loop / multi-turn conversation management
