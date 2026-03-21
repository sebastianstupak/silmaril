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
#[derive(Debug, Clone, Serialize)]
pub struct CommandSpec {
    pub id: String,                          // "hierarchy.create_entity"
    pub module_id: String,                   // "hierarchy" — set by register_module()
    pub label: String,                       // "Create Entity"
    pub category: String,                    // "hierarchy" (namespace-derived)
    pub description: Option<String>,
    pub keybind: Option<String>,             // "Ctrl+Shift+N"
    pub args_schema: Option<serde_json::Value>, // JSON Schema for parameters
    pub returns_data: bool,                  // false = fire-and-forget, true = awaits result
}
```

`id` is always namespaced: `<module-id>.<action>`. `module_id` is populated by `register_module` from `module.id()` — callers never set it directly. The module id prefix is enforced at registration time. `returns_data` defaults to `false` for all commands unless explicitly set; user module manifests may set `returns_data = true` per command.

### `CommandRegistry` — the single catalog

```rust
pub struct CommandRegistry {
    commands: Vec<CommandSpec>,
    module_index: std::collections::HashMap<String, Vec<usize>>, // module_id → indices into commands
    registry_tx: tokio::sync::watch::Sender<Vec<CommandSpec>>,
}

impl CommandRegistry {
    /// Returns the registry and a watch receiver for the live catalog.
    /// In lib.rs: let (registry, registry_rx) = CommandRegistry::new();
    /// Store registry in Tauri managed state (Arc<Mutex<CommandRegistry>>).
    /// Pass registry_rx to AiBridgeChannels.
    pub fn new() -> (Self, tokio::sync::watch::Receiver<Vec<CommandSpec>>);
    pub fn register_module(&mut self, module: &dyn EditorModule); // also calls registry_tx.send(...)
    pub fn list(&self) -> &[CommandSpec];
    pub fn get(&self, id: &str) -> Option<&CommandSpec>;
    pub fn by_module(&self, module_id: &str) -> Vec<&CommandSpec>;
    pub fn get_by_keybind(&self, keybind: &str) -> Option<&CommandSpec>;
}
```

**In `lib.rs` startup:** Replace `CommandRegistryState(Mutex<CommandRegistry>)` with `Arc<Mutex<CommandRegistry>>` stored as Tauri managed state. The `watch::Receiver` is extracted at creation and passed to `AiBridgeChannels`. Example:

```rust
let (registry, registry_rx) = CommandRegistry::new();
let registry = Arc::new(Mutex::new(registry));
// register all built-in modules:
{ let mut r = registry.lock().unwrap(); r.register_module(&HierarchyModule); /* ... */ }
// store in Tauri state:
.manage(registry.clone())
// pass receiver to AI bridge:
AiBridgeChannels { registry_rx, /* ... */ }
```

`register_module` calls `module.commands()`, sets `module_id` on each spec, and validates that every command id starts with `module.id() + "."`. Panics in debug builds on violation. After inserting, calls `registry_tx.send(self.commands.clone())`.

The TypeScript side maintains a mirrored `Map<string, CommandSpec>` (the "registry snapshot"). It is populated at startup via `populateRegistry(specs)` — see the TypeScript module registration section below.

### `dispatchCommand` — the single execution point

TypeScript, exported from `src/lib/commands/dispatch.ts`:

```typescript
// TypeScript shape as returned by invoke('list_commands'):
export interface CommandSpec {
  id: string;
  module_id: string;
  label: string;
  category: string;
  description?: string;
  keybind?: string;
  args_schema?: Record<string, unknown>; // JSON Schema
  returns_data: boolean;
}

// Populate the local registry snapshot from the Rust registry.
// Called once at startup after invoke('list_commands') resolves.
// Also called after a user module is installed (registry refresh).
export function populateRegistry(specs: CommandSpec[]): void

// Register a TypeScript-side handler for a command id.
// Called by each module's register() function at startup.
export function registerCommandHandler(
  id: string,
  fn: (args?: Record<string, unknown>) => unknown | Promise<unknown>
): void

// The single dispatch function every caller uses.
export async function dispatchCommand(
  id: string,
  args?: Record<string, unknown>
): Promise<unknown>
```

`dispatchCommand` logic:

1. Look up the command in the registry snapshot (error if not found: `"Unknown command: <id>"`)
2. Check the handler table for a registered TypeScript handler for this id
3. **If found:**
   - Call `handler(args)` and `await` the result (handlers may be sync or async)
   - If `CommandSpec.returns_data` is `true`: return the resolved value
   - If `CommandSpec.returns_data` is `false`: discard the return value, resolve with `undefined`
   - If the handler throws or rejects: re-throw the error (caller receives a rejected Promise)
4. **If not found:** fire `editor-run-command { id, args }` Tauri event.
   - For fire-and-forget commands (`returns_data: false`): resolve immediately with `undefined`
   - For data-returning commands (`returns_data: true`): generate a unique `requestId` (e.g., `crypto.randomUUID()`), include it in the Tauri event payload as `editor-run-command { id, args, requestId }`, and await `editor-command-result` filtered by matching `requestId`. The `editor-command-result` payload is `{ id, requestId, result?, error? }`. Filtering by `requestId` eliminates the concurrent-call race. Timeout: **5 seconds** → reject with `"Command timed out: <id>"`. If event arrives with `error` set: reject with that error string.

**`editor-run-command` Rust-side listener (new — in `runner.rs`):** A Tauri event listener registered on app startup receives `editor-run-command { id, args, requestId }` events. The listener **first checks whether the id is a known Rust-handled command** (for v1: `viewport.screenshot`). If it is not in the known Rust set, it logs a `tracing::warn!` and drops the event — this represents a programming error where a TypeScript-handled command was erroneously routed via the event path. For known Rust-handled commands, it calls `run_command_inner`. For `returns_data: true` commands, it emits `editor-command-result { id, requestId, result?, error? }` after execution. For fire-and-forget commands it does not emit a result event. This replaces the old hardcoded `editor-run-command` mapping in `App.svelte`.

**`editor-command-result` event** (new — emitted from `runner.rs`): Rust-side handlers that produce a return value emit this event after execution. Fire-and-forget Rust commands do not emit it. The event payload: `{ id: string, requestId: string, result?: unknown, error?: string }`.

> **Fire-and-forget caveat:** For `returns_data: false` Tauri commands, `dispatchCommand` resolves before the Rust handler executes. There is no execution guarantee at the time the Promise resolves. Callers requiring completion confirmation should use `returns_data: true` or a follow-up read command. If `run_command_inner` returns `Err`, the error is logged via `tracing::error!` and dropped — no event is emitted back to the frontend.

The hardcoded `editor-run-command` event mapping in `App.svelte` (lines 477–488) is removed entirely — replaced by TypeScript handlers registered by module files.

### TypeScript vs Rust execution routing

Every command is either **TypeScript-handled** or **Rust-handled**:

- **TypeScript-handled**: a `registerCommandHandler(id, fn)` call registers a handler. `dispatchCommand` calls it directly. These commands never cross the Tauri event bridge.
- **Rust-handled**: no TypeScript handler is registered. `dispatchCommand` fires `editor-run-command` → the Rust listener in `runner.rs` calls `run_command_inner` → result flows back via `editor-command-result` (if `returns_data: true`).

| Module | Routing |
|---|---|
| `hierarchy.*`, `inspector.*`, `console.*`, `assets.*` | TypeScript — handlers call `dispatchSceneCommand` or scene store mutations |
| `scene.*`, `project.*`, `editor.*` (UI operations) | TypeScript |
| `viewport.toggle_grid`, `viewport.toggle_snap`, `viewport.set_grid_visible`, `viewport.set_grid_size`, `viewport.set_projection`, `viewport.set_tool_*`, `viewport.orbit`, `viewport.pan`, `viewport.zoom`, `viewport.reset_camera`, `viewport.focus_entity` | TypeScript — handlers call `dispatchSceneCommand` |
| `viewport.screenshot` | **Rust** — no TS handler; `runner.rs` calls `NativeViewport::capture_png_bytes()` |

`dispatchSceneCommand` (in `src/lib/scene/commands.ts`) continues to exist as the implementation layer for TypeScript-side viewport and scene mutations. It is not removed — it is called from module handlers instead of being called directly from panels.

### TypeScript module registration

Each module file in `src/lib/modules/` exports a `register()` function that calls `registerCommandHandler` for each of its commands:

```typescript
// src/lib/modules/hierarchy.ts
import { registerCommandHandler } from '$lib/commands/dispatch';
import { createEntity, deleteEntity, duplicateEntity, selectEntity } from '$lib/scene/commands';

export function register() {
  registerCommandHandler('hierarchy.create_entity', (args) =>
    createEntity(args?.name as string | undefined));
  registerCommandHandler('hierarchy.delete_entity', (args) =>
    deleteEntity(args!.id as number));
  registerCommandHandler('hierarchy.duplicate_entity', (args) =>
    duplicateEntity(args!.id as number));
  registerCommandHandler('hierarchy.select_entity', (args) =>
    selectEntity((args?.id as number) ?? null));
}
```

A `viewport.ts` module example showing the split:

```typescript
// src/lib/modules/viewport.ts
import { registerCommandHandler } from '$lib/commands/dispatch';
import { dispatchSceneCommand } from '$lib/scene/commands';

export function register() {
  registerCommandHandler('viewport.toggle_grid', () =>
    dispatchSceneCommand({ type: 'toggle_grid' }));
  registerCommandHandler('viewport.orbit', (args) =>
    dispatchSceneCommand({ type: 'orbit_camera', dx: args!.dx as number, dy: args!.dy as number }));
  // viewport.screenshot has NO handler registered — routes to Rust via editor-run-command
}
```

`App.svelte` startup sequence (explicit ordering in `onMount`):

1. Call all `register()` functions synchronously — handlers are wired before any dispatch can happen
2. `await invoke('list_commands')` — fetch the full `CommandSpec[]` from Rust
3. Call `populateRegistry(specs)` — the snapshot is now available for validation

Any `dispatchCommand` call before step 3 completes will fail with `"Unknown command: <id>"` (the snapshot is empty). This is acceptable — no commands are dispatched before `onMount` finishes. The `FrontendCommand.run()` lambda pattern is gone — handlers live in module files, not in App.svelte.

**Layout/project state in module files:** `editor.*` layout commands and `project.open` need access to layout state and project-open logic that currently live as closures in `App.svelte`. After the refactor, this state moves to shared Svelte stores: `src/lib/stores/layout.ts` (layout slots, active layout), `src/lib/stores/ui.ts` (`omnibarOpen`, `settingsOpen`), and `src/lib/stores/project.ts` (project-open logic). Module handlers import directly from these stores. App.svelte no longer owns the handler logic.

`editor.toggle_omnibar` → `modules/editor.ts` handler writes to `uiStore.omnibarOpen`. `editor.open_settings` → writes to `uiStore.settingsOpen`. Layout mutations (`editor.apply_layout`, etc.) → `modules/editor.ts` handler reads/writes `layoutStore`.

---

## Built-in Modules

Each built-in panel and subsystem becomes a module. They live in `src/lib/modules/` (TypeScript side) with a corresponding Rust registration in `bridge/modules/`.

| Module id | Commands |
|-----------|----------|
| `hierarchy` | `create_entity`, `delete_entity`, `rename_entity`, `duplicate_entity`, `select_entity` (`rename_entity` args: `{ id: number, name: string }`; available for MCP/AI access; the inline rename widget in HierarchyPanel remains a direct call and does not dispatch this command) |
| `inspector` | `add_component`, `remove_component`, `set_component_field` |
| `viewport` | `screenshot`, `orbit`, `pan`, `zoom`, `reset_camera`, `set_projection`, `focus_entity`, `set_grid_visible`, `set_grid_size`, `toggle_grid`, `toggle_snap`, `set_tool_select`, `set_tool_move`, `set_tool_rotate`, `set_tool_scale` |
| `console` | `clear`, `filter_level`, `copy_logs` |
| `assets` | `list_assets`, `copy_asset_path`, `refresh_assets` |
| `scene` | `get_state`, `new_scene`, `save_scene`, `populate_from_scan` (`populate_from_scan` args: `{ projectPath: string }`) |
| `project` | `open`, `build`, `run`, `add_module`, `list_modules` |
| `editor` | `undo`, `redo`, `open_settings`, `toggle_omnibar`, `toggle_ai_server`, `reset_layout`, `apply_layout`, `save_layout_slot`, `rename_layout_slot`, `duplicate_layout_slot`, `delete_layout_slot`, `create_layout`, `add_panel`, `open_template`, `close_template`, `execute_template` |

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
args_schema = { type = "object", properties = { steps = { type = "integer", default = 1 } } }
```

At project open, the editor scans installed modules, loads their manifests, and registers them. These commands appear in the omnibar, MCP `tools/list`, and anywhere else commands are consumed — automatically.

**User module command execution path:** User module commands have no TypeScript handler and no match in `run_command_inner`'s built-in id match. They route via the `editor-run-command` event path to `run_command_inner`, which calls a free function `run_module_command(module_id, id, args, app)` defined in `src-tauri/bridge/modules/user_modules.rs`. This function looks up the module's registered handler in `UserModuleHandlerState` (Tauri managed state: `HashMap<String, Box<dyn ModuleCommandHandler>>`). For v1, user module commands that have no registered native handler return `Err("not implemented")`, which becomes `-32000 Server error (not implemented)` in MCP. `run_module_command` is a plain Rust function — not a Tauri IPC command — called directly from `run_command_inner`. Add `src-tauri/bridge/modules/user_modules.rs` to the new files list.

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

All layout management props (`onApplyLayout`, `onSaveToSlot`, `onRenameSlot`, `onDuplicateSlot`, `onDeleteSlot`, `onCreateLayout`, `onAddPanel`) are removed and replaced with `editor.*` command dispatches:

```svelte
<!-- before -->
<button onclick={onApplyLayout(slot)}>Apply</button>

<!-- after -->
<button onclick={() => dispatchCommand('editor.apply_layout', { slot })}>Apply</button>
```

### Keyboard shortcuts

Current: `App.svelte` keyboard handler is a hardcoded if-else chain calling functions directly.

After: shortcuts are looked up from the registry:

```typescript
// App.svelte — handleKeyDown (the ONLY keyboard handler registered via onkeydown)
const cmd = registry.getByKeybind(shortcut);
if (cmd) dispatchCommand(cmd.id);
// handleKeyDown contains ONLY the above two calls
```

`cycleActiveTab` (`Ctrl+Tab`) and per-slot layout shortcuts are moved out of `handleKeyDown` into their own `$effect` listeners registered separately. `handleKeyDown` contains exactly `registry.getByKeybind` and `dispatchCommand` — nothing else.

The xtask lint (test 6) scans `handleKeyDown`'s function body (extracted via brace-counter from the opening `{`) and fails if it finds any identifier followed by `(` that is not `registry.getByKeybind` or `dispatchCommand`. The allowlist is exactly these two. The `Ctrl+Shift+Z` redo branch calls `dispatchCommand` — it passes the lint.

**Keybinds for commands that currently have hardcoded shortcuts:**

| Command id | `CommandSpec.keybind` |
|---|---|
| `editor.undo` | `"Ctrl+Z"` |
| `editor.redo` | `"Ctrl+Y"` (also `Ctrl+Shift+Z` — see note) |
| `editor.open_settings` | `"Ctrl+,"` |
| `editor.toggle_omnibar` | `"Ctrl+K"` |
| `viewport.set_tool_select` | `"Q"` |
| `viewport.set_tool_move` | `"W"` |
| `viewport.set_tool_rotate` | `"E"` |
| `viewport.set_tool_scale` | `"R"` |
| `editor.toggle_ai_server` | `"Ctrl+Shift+A"` |
| `scene.save_scene` | `"Ctrl+S"` |
| `viewport.toggle_grid` | `"G"` (old Rust registry used `"Ctrl+G"` — intentionally changed to `"G"`) |
| `viewport.toggle_snap` | `"S"` |

> **Dual-shortcut note:** `editor.redo` has two traditional bindings (`Ctrl+Y` and `Ctrl+Shift+Z`). `CommandSpec.keybind` is a single `Option<String>`. For v1, only `Ctrl+Y` is stored in the spec. `Ctrl+Shift+Z` is handled as an additional `if (shortcut === 'Ctrl+Shift+Z') dispatchCommand('editor.redo')` line inside `handleKeyDown`. Since this only calls `dispatchCommand`, it passes the xtask lint. A future `keybinds: Vec<String>` field is deferred. **Intentional v1 limitation:** `Ctrl+Shift+Z` is not stored in `CommandSpec.keybind` and will not appear in the omnibar keybind hint display.

Keybinds are declared in `CommandSpec.keybind`. Adding a keybind to a command automatically makes it work — no separate handler needed.

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

The permission category for a command is determined by this rule, applied in order (first match wins):

1. If the command id matches `*.get_*` or `*.list_*` → `read`
2. If the command id is `editor.toggle_omnibar`, `editor.open_settings`, `console.clear`, `console.filter_level`, `console.copy_logs` (reads engine data to clipboard; no scene mutation), or matches `assets.*` → `read`
3. If the module id is `hierarchy`, `inspector`, or `scene` → `scene`
4. If the command id matches `editor.undo`, `editor.redo`, `editor.reset_layout`, `editor.apply_layout`, `editor.*_layout*`, `editor.add_panel` → `scene` (state-mutating editor operations)
5. If the module id is `viewport` → `viewport`
6. If the command id matches `project.build` or `project.run` → `build`
7. If the command id matches `project.generate_*` or `project.add_module` → `codegen`
8. All other commands (user modules: `physics.*`, `audio.*`, etc.) → `modules`

Implemented in `permissions.rs` as a pure function: `fn category_for(cmd: &CommandSpec) -> &'static str`.

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

**Channel receiver wiring (in `lib.rs` at startup):**

- `command_tx` → receiver lives in a Tauri background task. The Tauri `run_command` handler and this background task both call a shared free function extracted from `runner.rs`:
  ```rust
  pub fn run_command_inner(
      id: &str,
      args: Option<serde_json::Value>,
      registry: &CommandRegistry,
      app: &AppHandle,  // used to retrieve NativeViewportState and other managed state
  ) -> Result<serde_json::Value, String>
  ```
  `run_command_inner` dispatches via a match on `id`. For `viewport.screenshot`, it calls `app.state::<NativeViewportState>().capture_png_bytes()`. The `CommandRegistry` validates the id exists; the match covers only known Rust-side command ids. For v1, the only Rust-handled command is `viewport.screenshot`. The background task holds an `Arc<Mutex<CommandRegistry>>` and an `AppHandle` cloned at startup. The `CommandRequest` carries `{ id, args, response_tx: oneshot::Sender<Result<serde_json::Value, String>> }`.
- `permission_tx` → receiver lives in the Tauri main thread (or a dedicated task); on receive it emits the `ai:permission_request` Tauri event to the frontend. The `PermissionRequest` carries `{ request_id, category, command_id, response_tx: oneshot::Sender<PermissionLevel> }`.
- `screenshot_tx` → receiver lives in the render thread (or a task that can signal it). The `ScreenshotRequest` carries `{ response_tx: oneshot::Sender<Result<Vec<u8>, String>> }`.
- `registry_rx` → consumed by the MCP server in `mcp.rs` to serve `tools/list` without cross-thread locks. Updated via `watch::Sender` whenever `register_module` is called.

### Types

```rust
// permissions.rs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PermissionLevel { Once, Session, Always, Deny }

// ai_bridge.rs — Tauri managed state wrapping the in-flight permission requests
pub struct AiBridgeState {
    // pending permission requests waiting for user response
    pending_permissions: Mutex<HashMap<String, oneshot::Sender<PermissionLevel>>>,
}
// The permission_tx receiver inserts the response_tx into pending_permissions.
// ai_grant_permission removes and resolves it.

// ai_bridge.rs — Tauri command called by the frontend permission dialog
#[tauri::command]
pub fn ai_grant_permission(
    request_id: String,
    level: PermissionLevel,
    bridge_state: tauri::State<AiBridgeState>,
) -> Result<(), String> // resolves the oneshot::Sender for the given request_id

// registry_bridge.rs
pub struct McpTool {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
}
pub fn translate_to_mcp_tools(specs: &[CommandSpec]) -> Vec<McpTool>
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

### Test helper: `build_full_registry()`

All tests use this helper. It lives in `src-tauri/bridge/tests/helpers.rs` and constructs a `CommandRegistry` in-process with all built-in modules registered — no Tauri app handle required:

```rust
pub fn build_full_registry() -> CommandRegistry {
    let (mut registry, _rx) = CommandRegistry::new(); // discard receiver in tests
    registry.register_module(&HierarchyModule);
    registry.register_module(&InspectorModule);
    registry.register_module(&ViewportModule);
    registry.register_module(&ConsoleModule);
    registry.register_module(&AssetsModule);
    registry.register_module(&SceneModule);
    registry.register_module(&ProjectModule);
    registry.register_module(&EditorBuiltinModule); // "editor" id; distinct from the EditorModule trait
    registry
}
```

Each built-in module is a **zero-sized unit struct** implementing the `EditorModule` trait. They derive `Default`. No app handle or Tauri state is needed to construct them. The struct for the `editor` built-in module is named `EditorBuiltinModule` (not `EditorModule`) to avoid the naming collision with the trait. Example: `pub struct HierarchyModule;`, `pub struct EditorBuiltinModule;`.

### 1. Module completeness

Every registered module must have at least one command. Enforced at compile time by the `EditorModule` trait (the `commands()` method returning an empty vec triggers a debug assert at registration time).

### 2. Command id namespace invariant

`cmd.module_id` is set by `register_module` from `module.id()`.

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
            if let Some(prev_id) = seen.insert(kb.clone(), cmd.id.clone()) {
                panic!("Keybind '{}' used by both '{}' and '{}'", kb, prev_id, cmd.id);
            }
        }
    }
}
```

### 4. Command manifest snapshot

Requires `insta = "1"` as a dev-dependency. Add to `engine/editor/src-tauri/Cargo.toml` under `[dev-dependencies]`.

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

**Initial setup:** Run `cargo test` once (test fails with "missing snapshot"), then `cargo insta review` to accept. Committed snapshot file lives at `engine/editor/src-tauri/bridge/tests/snapshots/bridge__tests__command_manifest_snapshot.snap`.

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

An `xtask lint` subcommand (add to `xtask/src/main.rs`) that reads the keyboard handler function body from `App.svelte` and verifies it contains no direct non-`dispatchCommand` calls. The check is intentionally coarse: after the refactor, the entire keyboard handler function is two lines (keybind lookup + dispatch). Any growth in that function indicates a bypass.

Implementation approach — add to `xtask`:

```rust
// xtask lint: keyboard handler in App.svelte must only call dispatchCommand
let app_svelte = fs::read_to_string("engine/editor/src/App.svelte")?;
// Extract lines between "function handleKeyDown" and the closing "}"
// Assert that the extracted block contains no identifiers followed by "("
// other than "dispatchCommand" and "registry.getByKeybind"
```

The exact implementation is left to the developer. The invariant to enforce: **the keyboard handler calls exactly `registry.getByKeybind` and `dispatchCommand`, nothing else**.

---

## `SceneSnapshot` Schema

Returned by `scene.get_state` (`returns_data: true`). The return value is the JSON-serialized form of the TypeScript `SceneSnapshot` type below, produced by the frontend's in-memory scene state. The Rust side does not produce this — `scene.get_state` is a TypeScript-side command with a registered handler that reads `getSceneState()` from `src/lib/scene/state.ts`. This module holds `entities`, `selectedEntityId`, `camera`, `gridVisible`, `snapToGrid`, and `gridSize`. The handler serializes the full store value into the snapshot shape below. Mirrors `SceneState` in `src/lib/scene/state.ts`:

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

## Intentionally Excluded from Registry

The following `dispatchSceneCommand` message types exist today but are **not registered as commands**. They are continuous interaction events fired on every mouse move, not discrete user-invokable actions:

- `move_entity`, `rotate_entity`, `scale_entity` — emitted on every drag tick from viewport gizmos
- `pan_camera`, `orbit_camera`, `zoom_camera`, `set_view_angle` — emitted on every mouse/wheel event

These remain as direct `dispatchSceneCommand` calls from the viewport interaction handlers (not from `dispatchCommand`). They are not suitable for the omnibar, MCP, or undo registry because they fire many times per gesture.

---

## Known Limitations

- **Undo/redo for AI mutations:** AI-driven scene mutations go through `dispatchCommand` → `dispatchSceneCommand` and currently do not push to the undo stack. Future work.
- **Inline edit operations** (entity rename, component field editing) remain as direct function calls — they are continuous operations, not discrete commands, and are not suitable for command dispatch.
- **Module command args v1:** user module manifests support simple flat schemas only; deeply nested arg schemas are deferred.
- **Headless scene reads:** `scene.get_state` requires the TypeScript WebView to be running.

---

## File Structure Summary

**New files:**
- `src/lib/commands/dispatch.ts` — `dispatchCommand()`, `populateRegistry()`, `registerCommandHandler()`, handler table
- `src/lib/modules/index.ts` — barrel file; exports `registerAll()` which calls `register()` on each module in order; `App.svelte` `onMount` calls `registerAll()` as step 1 of the startup sequence
- `src/lib/modules/hierarchy.ts`, `inspector.ts`, `viewport.ts`, `console.ts`, `assets.ts`, `scene.ts`, `project.ts`, `editor.ts` — one file per built-in module
- `src/lib/stores/layout.ts` — layout slot state (moved from App.svelte)
- `src/lib/stores/ui.ts` — `omnibarOpen`, `settingsOpen` (moved from App.svelte)
- `src/lib/stores/project.ts` — project-open logic (moved from App.svelte)
- `src-tauri/bridge/modules/hierarchy.rs`, `inspector.rs`, `viewport.rs`, `console.rs`, `assets.rs`, `scene.rs`, `project.rs`, `editor_builtin.rs` — one file per built-in module, each containing `pub struct <Name>Module;` implementing `EditorModule`
- `src-tauri/bridge/modules/mod.rs` — re-exports all module structs
- `src-tauri/bridge/ai_bridge.rs` — Tauri bridge for MCP channels, `AiBridgeState`, `ai_grant_permission`
- `engine/ai/` — MCP server crate (new)

**Modified files:**
- `src-tauri/bridge/registry.rs` — rename `EditorCommand` → `CommandSpec` (add `module_id`, `returns_data`, `args_schema` fields), replace the old `register(cmd: EditorCommand)` method with `register_module(module: &dyn EditorModule)`, introduce the `EditorModule` trait
- `src-tauri/bridge/runner.rs` — the `run_command` Tauri IPC command is **removed** (it is no longer invoked from TypeScript; `dispatchCommand` now fires `editor-run-command` events instead). `list_commands` Tauri IPC command is **kept** and now returns `Vec<CommandSpec>` instead of `Vec<EditorCommand>`. Replace `run_command` with: (a) a `runner.rs` setup function called from `lib.rs` `.setup()` closure that registers the event listener via `app.listen("editor-run-command", move |event| { ... })`, (b) `run_command_inner(id, args, registry, app) -> Result<serde_json::Value, String>` free function dispatching Rust-handled commands by id match
- `src/lib/omnibar/Omnibar.svelte` — remove dual execution path, use `dispatchCommand`
- `src/lib/omnibar/registry.ts` — remove `FrontendCommand`, remove `run()` field
- `src/lib/omnibar/types.ts` — remove `FrontendCommand` interface
- `src/App.svelte` — remove hardcoded keyboard handler chain, remove `editor-run-command` mapping, wire shortcuts via registry keybind lookup
- `src/lib/components/TitleBar.svelte` — remove action callback props, use `dispatchCommand`
- `src/lib/components/HierarchyPanel.svelte` — wire discrete actions to `dispatchCommand`
- `src/lib/components/InspectorPanel.svelte` — wire add/remove component to `dispatchCommand`
- `src/lib/scene/commands.ts` — no change needed for `set_component_field`; the `inspector.set_component_field` TypeScript handler calls `invoke('set_component_field', { id, component, field, value })` directly (the existing Tauri IPC path is kept as-is)

---

## Command Migration Table

### Existing TypeScript (frontend) commands

| Old id (TypeScript registry) | New id | Module |
|---|---|---|
| `edit.undo` | `editor.undo` | `editor` |
| `edit.redo` | `editor.redo` | `editor` |
| `ui.open_settings` | `editor.open_settings` | `editor` |
| `ui.open_project` | `project.open` | `project` |
| `ui.layout.reset` | `editor.reset_layout` | `editor` |
| (TitleBar "Save Scene" menu item — no old command id) | `scene.save_scene` | `scene` (new; `Ctrl+S` keybind) |

### Existing Rust `CommandRegistry` commands

The existing Rust `CommandRegistry` uses these IDs. They are renamed under the new namespace scheme:

| Old id (Rust registry) | New id | Module |
|---|---|---|
| `editor.toggle_grid` | `viewport.toggle_grid` | `viewport` |
| (TypeScript-only `setGridSize()` call — no old Rust id) | `viewport.set_grid_size` | `viewport` (new Rust registration; TS handler calls existing `setGridSize()`) |
| `editor.toggle_snap` | `viewport.toggle_snap` | `viewport` |
| `editor.toggle_projection` | `viewport.set_projection` | `viewport` |
| `editor.set_tool.select` | `viewport.set_tool_select` | `viewport` |
| `editor.set_tool.move` | `viewport.set_tool_move` | `viewport` |
| `editor.set_tool.rotate` | `viewport.set_tool_rotate` | `viewport` |
| `editor.set_tool.scale` | `viewport.set_tool_scale` | `viewport` |
| `editor.reset_camera` | `viewport.reset_camera` | `viewport` |
| (was direct IPC `viewport_set_grid_visible`) | `viewport.set_grid_visible` | `viewport` |
| (IPC `scan_assets`) | `assets.list_assets` | `assets` (TypeScript handler calls `invoke('scan_assets', { projectPath })`; IPC command kept as-is) |
| `editor.undo` | `editor.undo` | `editor` (TypeScript-handled; TS handler calls `undo()`. `EditorModule` Rust struct must still declare it in `commands()` so it appears in the catalog. Not currently in the Rust `CommandRegistry` — add it.) |
| `editor.redo` | `editor.redo` | `editor` (same pattern as undo) |
| `editor.new_scene` | `scene.new_scene` | `scene` |
| `template.open` | `editor.open_template` | `editor` |
| `template.close` | `editor.close_template` | `editor` |
| `template.execute` | `editor.execute_template` | `editor` |

The full `viewport` command set (replacing scattered `editor.*` viewport commands):

| Command | Args | Returns data |
|---|---|---|
| `viewport.screenshot` | — | yes (`{ data: string, mimeType: "image/png" }` — base64-encoded PNG bytes returned directly; no file is written) |
| `viewport.orbit` | `{ dx, dy: number }` | no |
| `viewport.pan` | `{ dx, dy: number }` | no |
| `viewport.zoom` | `{ delta: number }` | no |
| `viewport.reset_camera` | — | no |
| `viewport.set_projection` | `{ mode: 'perspective' \| 'ortho' }` | no |
| `viewport.focus_entity` | `{ id: number }` | no |
| `viewport.set_grid_visible` | `{ visible: boolean }` | no |
| `viewport.toggle_grid` | — | no |
| `viewport.toggle_snap` | — | no |
| `viewport.set_grid_size` | `{ size: number }` | no |
| `viewport.set_tool_select` | — | no |
| `viewport.set_tool_move` | — | no |
| `viewport.set_tool_rotate` | — | no |
| `viewport.set_tool_scale` | — | no |

---

## Out of Scope

- AI chat panel UI (sub-project 3)
- BYOK provider configuration
- Per-project chat history
- Agent loop / multi-turn conversation
