# Silmaril Editor — Command Architecture Design

> **For agentic workers:** Use `superpowers:writing-plans` to produce the implementation plan from this spec.

**Goal:** Replace the three separate command execution paths in the editor (TypeScript registry, Rust registry, direct function calls) with a single `EditorModule` + `CommandRegistry` system. Every operation in the editor — from a panel button to an AI agent tool call — flows through one dispatch function. MCP is a consumer of this registry, not a separate tool list.

---

## Problem Statement

The editor currently has three disconnected execution paths:

| Path | Registration | Execution | Example |
|------|-------------|-----------|---------|
| TypeScript commands | `registerCommand()` in App.svelte | `cmd.run()` | `edit.undo` |
| Rust commands | `reg.register()` in lib.rs | `invoke()` → event → hardcoded mapping | `editor.toggle_grid` |
| Direct calls | None | Direct import | `HierarchyPanel → createEntity()` |

Menus and keyboard shortcuts bypass the registry entirely. Panels call functions directly. No command supports parameters. External tools (MCP, AI, scripts) cannot discover or invoke the full command surface.

---

## Core Concepts

### `EditorModule` — the unit of command ownership

Every subsystem that exposes operations to the editor is an `EditorModule`. Built-in panels are core modules. User-installed modules are identical in structure.

```rust
pub trait EditorModule: Send + Sync {
    fn id(&self) -> &str;
    fn commands(&self) -> Vec<CommandSpec>;
}
```

The module is the **source of truth** for its commands. Nobody registers a command without owning a module.

### `CommandSpec` — the full descriptor

```rust
pub struct CommandSpec {
    pub id: String,                          // "hierarchy.create_entity"
    pub label: String,                       // "Create Entity"
    pub category: String,                    // "hierarchy" (namespace-derived)
    pub description: Option<String>,
    pub keybind: Option<String>,             // "Ctrl+Shift+N"
    pub args_schema: Option<serde_json::Value>, // JSON Schema for parameters
}
```

`id` is always namespaced: `<module-id>.<action>`. The module id prefix is enforced at registration time.

### `CommandRegistry` — the single catalog

```rust
pub struct CommandRegistry {
    commands: Vec<CommandSpec>,
    // module_id → Vec<CommandSpec> index, for fast module lookup
}

impl CommandRegistry {
    pub fn register_module(&mut self, module: &dyn EditorModule);
    pub fn list(&self) -> &[CommandSpec];
    pub fn get(&self, id: &str) -> Option<&CommandSpec>;
    pub fn by_module(&self, module_id: &str) -> Vec<&CommandSpec>;
}
```

`register_module` calls `module.commands()` and validates that every command id starts with `module.id() + "."`. Panics in debug builds on violation.

### `dispatchCommand` — the single execution point

TypeScript:

```typescript
async function dispatchCommand(id: string, args?: Record<string, unknown>): Promise<unknown>
```

Every caller — keyboard shortcut, menu item, panel button, omnibar, MCP, AI — calls this one function. It:

1. Looks up the command in the registry (error if not found)
2. Checks if there is a registered TypeScript handler for this id
3. If yes: calls the handler with args
4. If no: fires `editor-run-command { id, args }` Tauri event; awaits `editor-command-result { id, data }` for commands that return data (timeout: 5s)

The hardcoded `editor-run-command` event mapping in `App.svelte` (lines 477–488) is replaced by a handler table populated at module registration time.

---

## Built-in Modules

Each built-in panel and subsystem becomes a module. They live in `src/lib/modules/` (TypeScript side) with a corresponding Rust registration in `bridge/modules/`.

| Module id | Commands |
|-----------|----------|
| `hierarchy` | `create_entity`, `delete_entity`, `rename_entity`, `duplicate_entity`, `select_entity` |
| `inspector` | `add_component`, `remove_component`, `set_component_field` |
| `viewport` | `screenshot`, `orbit`, `pan`, `zoom`, `reset_camera`, `set_projection`, `focus_entity`, `set_grid_visible`, `toggle_grid`, `toggle_snap` |
| `console` | `clear`, `filter_level`, `copy_logs` |
| `assets` | `list_assets`, `copy_asset_path`, `refresh_assets` |
| `scene` | `get_state`, `new_scene`, `save_scene`, `populate_from_scan` |
| `project` | `open`, `build`, `run`, `add_module`, `list_modules` |
| `editor` | `undo`, `redo`, `open_settings`, `toggle_omnibar`, `reset_layout` |

---

## User Module Commands

Each installed user module (e.g., `physics`, `audio`) ships a `commands.toml` manifest in its module directory:

```toml
module_id = "physics"

[[commands]]
id = "physics.add_rigidbody"
label = "Add Rigidbody"
description = "Attach a rigidbody component to the selected entity"

[[commands]]
id = "physics.simulate_step"
label = "Simulate Physics Step"
description = "Advance physics simulation by one fixed step"

[commands.args_schema]
type = "object"
properties.steps = { type = "integer", default = 1 }
```

At project open, the editor scans installed modules, loads their manifests, and registers them. These commands appear in the omnibar, MCP `tools/list`, and anywhere else commands are consumed — automatically.

---

## Omnibar Refactor

### Current problem

`Omnibar.svelte` has two execution paths:

```typescript
// current — two paths
if (isFrontendCommand(cmd)) {
    await cmd.run();              // TypeScript path
} else {
    await invoke('run_command', { id: cmd.id });  // Rust path
}
```

TypeScript commands (registered in App.svelte) are invisible to MCP, AI agents, and external tools.

### After refactor

```typescript
// unified
await dispatchCommand(cmd.id, cmd.args);
```

All commands — regardless of where they execute — go through `dispatchCommand`. The omnibar no longer needs to know whether a command is "frontend" or "backend". The `isFrontendCommand` type guard and the separate `FrontendCommand` interface are removed.

### Omnibar result building

The omnibar still mixes command results with entity results, asset results, and recent items. This stays as-is (it's correct). The only change is in the execution path.

---

## Menu and Keyboard Shortcut Refactor

### Menu bar

Current: `TitleBar.svelte` uses callback props (`onUndo`, `onRedo`, `onOpenProject`, etc.) wired to direct function calls.

After: menu items dispatch by command id:

```svelte
<!-- before -->
<button onclick={onUndo}>Undo</button>

<!-- after -->
<button onclick={() => dispatchCommand('editor.undo')}>Undo</button>
```

`TitleBar` no longer receives action callback props. It imports `dispatchCommand` directly. The long prop chain (App → TitleBar → menu item) for actions is removed.

### Keyboard shortcuts

Current: `App.svelte` keyboard handler is a hardcoded if-else chain calling functions directly.

After: shortcuts are looked up from the registry:

```typescript
// App.svelte onkeydown
const cmd = registry.getByKeybind(shortcut);
if (cmd) dispatchCommand(cmd.id);
```

Keybinds are declared in `CommandSpec.keybind`. The if-else chain in App.svelte is removed. Adding a keybind to a command automatically makes it work — no separate handler needed.

---

## Panel Refactor

Panel action buttons dispatch through `dispatchCommand` rather than importing and calling functions directly. This is a **soft refactor** — panels still import scene functions for complex operations that don't map cleanly to a single command (e.g., the rename inline edit flow). Only the discrete, nameable actions are wired to the registry.

| Panel action | Before | After |
|---|---|---|
| Hierarchy + button | `createEntity()` | `dispatchCommand('hierarchy.create_entity')` |
| Hierarchy Del key | `deleteEntity(id)` | `dispatchCommand('hierarchy.delete_entity', { id })` |
| Hierarchy duplicate | `duplicateEntity(id)` | `dispatchCommand('hierarchy.duplicate_entity', { id })` |
| Inspector ✕ component | `removeComponent(id, name)` | `dispatchCommand('inspector.remove_component', { id, component: name })` |
| Inspector add component | `addComponent(id, name)` | `dispatchCommand('inspector.add_component', { id, component: name })` |
| Console clear | `clearLogs()` | `dispatchCommand('console.clear')` |

Inline rename in HierarchyPanel and field editing in InspectorPanel remain as direct function calls — they are continuous edit operations, not discrete commands.

---

## MCP Server (Revised)

With the unified registry, the MCP server is simple: it has no tool list of its own.

- **`tools/list`** = `registry.list()` translated to MCP tool format (one-liner per command)
- **`tools/call`** = permission check → `dispatchCommand(id, args)` → return result

No hardcoded tools. No separate `engine/ai` tool modules. When a new module is installed and registers commands, they automatically appear in `tools/list` on the next call.

### Permission categories

The namespace prefix determines the permission category:

| Namespace | MCP permission category |
|-----------|------------------------|
| `editor.*`, `console.*`, `scene.get_*`, `assets.*` | `read` |
| `hierarchy.*`, `inspector.*`, `scene.*` (mutations) | `scene` |
| `viewport.*` | `viewport` |
| `project.build`, `project.run` | `build` |
| `project.generate_*`, `project.add_module` | `codegen` |
| `physics.*`, `audio.*`, user modules | `modules` |

### Architecture

```
engine/ai/
├── src/
│   ├── lib.rs              — AiServer, AiBridgeChannels, start(), stop()
│   ├── server.rs           — axum routes, SSE
│   ├── mcp.rs              — JSON-RPC 2.0, tools/list + tools/call
│   ├── registry_bridge.rs  — CommandSpec → MCP tool translation
│   └── permissions.rs      — PermissionStore, grant check, JSON persistence
└── Cargo.toml
```

`AiBridgeChannels` injected at startup:

```rust
pub struct AiBridgeChannels {
    pub command_tx:    mpsc::Sender<CommandRequest>,     // id + args → result
    pub permission_tx: mpsc::Sender<PermissionRequest>,  // category → grant
    pub screenshot_tx: mpsc::Sender<ScreenshotRequest>,  // → PNG bytes
    pub registry_rx:   watch::Receiver<Vec<CommandSpec>>, // live catalog
}
```

### Permission request flow

1. Tool call arrives; bridge checks permission store for the command's namespace category
2. No grant: send `PermissionRequest { request_id, category, command_id, response_tx }` via `permission_tx`
3. Bridge fires `ai:permission_request { request_id, category, command_id }` Tauri event
4. Frontend shows dialog: `"<client> wants to use <category> — Once / Session / Always / Deny"`
5. User calls `ai_grant_permission(request_id, level)` Tauri command
6. Bridge resolves `response_tx`; tool executes
7. **Timeout: 30 seconds** → auto-deny `-32003`

### Screenshot

`viewport.screenshot` dispatches to the Rust bridge which calls `NativeViewport::capture_png_bytes()` directly (no TypeScript round-trip). The method is new and must be added to `NativeViewport`:

```rust
pub fn capture_png_bytes(&self) -> Result<Vec<u8>, String>
```

It signals the render thread to blit the swapchain image to a CPU buffer, waits (max 1s), encodes as PNG via the `png` crate, returns bytes. Reuses `engine-renderer` capture module logic.

### Server lifecycle

- Auto-starts on project open (when `ai` feature compiled in)
- `Ctrl+Shift+A` or View → "AI Server" to toggle
- Port `7878`, auto-increment to `7888` on conflict
- Status bar: `MCP :7878` badge, clickable copies URL

### Error codes

| Situation | JSON-RPC error |
|-----------|----------------|
| Unknown command | `-32601 Method not found` |
| Permission denied | `-32003 Permission denied` |
| Permission timed out | `-32003 Permission denied (timed out)` |
| No project open | `-32002 No project open` |
| Execution failed | `-32000 Server error` |
| Command timed out | `-32000 Server error (timed out)` |

### Headless / CI

`SILMARIL_AI_ALLOW_ALL=1` or `--ai-allow-all` auto-grants all as `Session`. CI use only.

### Permission persistence

`<project>/.silmaril/ai-permissions.json`:
```json
{
  "grants": {
    "read": "always",
    "scene": "session",
    "viewport": null,
    "build": null,
    "codegen": null,
    "modules": null
  }
}
```

---

## Architecture Tests

These tests are the enforcement mechanism. They run in CI and fail if the invariants are violated.

### 1. Module completeness

Every registered module must have at least one command. Enforced at compile time by the `EditorModule` trait (the `commands()` method returning an empty vec triggers a debug assert at registration time).

### 2. Command id namespace invariant

```rust
#[test]
fn command_ids_match_module_namespace() {
    let registry = build_full_registry();
    for cmd in registry.list() {
        let expected_prefix = format!("{}.", cmd.module_id);
        assert!(cmd.id.starts_with(&expected_prefix),
            "Command '{}' does not start with module prefix '{}'", cmd.id, expected_prefix);
    }
}
```

### 3. Keybind uniqueness

```rust
#[test]
fn no_duplicate_keybinds() {
    let registry = build_full_registry();
    let mut seen = std::collections::HashMap::new();
    for cmd in registry.list() {
        if let Some(kb) = &cmd.keybind {
            assert!(seen.insert(kb.clone(), cmd.id.clone()).is_none(),
                "Keybind '{}' used by both '{}' and '{}'", kb, seen[kb], cmd.id);
        }
    }
}
```

### 4. Command manifest snapshot

```rust
#[test]
fn command_manifest_snapshot() {
    let registry = build_full_registry();
    let manifest: Vec<_> = registry.list().iter()
        .map(|c| (c.id.clone(), c.label.clone(), c.category.clone()))
        .collect();
    insta::assert_json_snapshot!(manifest);
}
```

The snapshot is committed to the repo. Any change to the command surface — adding, removing, renaming — requires updating the snapshot intentionally. This makes command surface changes visible in code review.

### 5. MCP tools/list invariant

```rust
#[test]
fn mcp_tools_match_registry() {
    let registry = build_full_registry();
    let mcp_tools = translate_to_mcp_tools(registry.list());
    assert_eq!(mcp_tools.len(), registry.list().len());
    for tool in &mcp_tools {
        assert!(registry.get(&tool.name).is_some(),
            "MCP tool '{}' has no registry entry", tool.name);
    }
}
```

### 6. No orphan keybind handlers

A lint-level test (or CI grep) that checks `App.svelte` for any `onkeydown` handler that does not call `dispatchCommand`. Ensures no shortcuts bypass the registry after the refactor.

---

## `SceneSnapshot` Schema

Returned by `scene.get_state`. Mirrors `SceneState` in `src/lib/scene/state.ts`:

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
  // activeTool and nextEntityId intentionally excluded:
  // activeTool is UI state; nextEntityId is an internal counter.
}
```

---

## Known Limitations

- **Undo/redo for AI mutations:** AI-driven scene mutations go through `dispatchCommand` → `dispatchSceneCommand` and currently do not push to the undo stack. Future work.
- **Inline edit operations** (entity rename, component field editing) remain as direct function calls — they are continuous operations, not discrete commands, and are not suitable for command dispatch.
- **Module command args v1:** user module manifests support simple flat schemas only; deeply nested arg schemas are deferred.
- **Headless scene reads:** `scene.get_state` requires the TypeScript WebView to be running.

---

## File Structure Summary

**New files:**
- `src/lib/commands/dispatch.ts` — `dispatchCommand()`, handler table, registration API
- `src/lib/modules/` — one file per built-in module (`hierarchy.ts`, `viewport.ts`, etc.)
- `src-tauri/bridge/modules/` — Rust registration functions per module
- `src-tauri/bridge/ai_bridge.rs` — Tauri bridge for MCP channels
- `engine/ai/` — MCP server crate (new)

**Modified files:**
- `src-tauri/bridge/registry.rs` — add `args_schema` to `EditorCommand`, add `register_module()`
- `src-tauri/bridge/runner.rs` — add `args: Option<serde_json::Value>` to `run_command`
- `src/lib/omnibar/Omnibar.svelte` — remove dual execution path, use `dispatchCommand`
- `src/lib/omnibar/registry.ts` — remove `FrontendCommand`, remove `run()` field
- `src/lib/omnibar/types.ts` — remove `FrontendCommand` interface
- `src/App.svelte` — remove hardcoded keyboard handler chain, remove `editor-run-command` mapping, wire shortcuts via registry keybind lookup
- `src/lib/components/TitleBar.svelte` — remove action callback props, use `dispatchCommand`
- `src/lib/components/HierarchyPanel.svelte` — wire discrete actions to `dispatchCommand`
- `src/lib/components/InspectorPanel.svelte` — wire add/remove component to `dispatchCommand`
- `src/lib/scene/commands.ts` — add `set_component_field` case to `dispatchSceneCommand`

---

## Out of Scope

- AI chat panel UI (sub-project 3)
- BYOK provider configuration
- Per-project chat history
- Agent loop / multi-turn conversation
