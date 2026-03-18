# Silmaril Editor Architecture — Design Spec

**Date:** 2026-03-18
**Status:** Approved
**ROADMAP:** Phase 0.8 (Foundation) + Phase 4.9 (Advanced)

---

## Goal

Build a native desktop editor for Silmaril using Tauri 2 + Svelte 5 + shadcn-svelte. The editor is a pure shell — every panel (hierarchy, inspector, viewport, console, profiler) is a module-provided plugin. The editor and CLI share the same operation layer (`engine/ops`), making them interchangeable frontends.

---

## Core Principles

1. **Everything is a module.** The editor ships with zero built-in panels. Core engine features (ECS, renderer, profiler) are modules that register their own panels.
2. **Shared operations.** `engine/ops` owns all operations. CLI and editor are thin frontends.
3. **Code-first.** Editor writes code files and YAML scenes, not binary formats. Everything is git-committable.
4. **AI-native.** MCP integration for AI-driven testing and debugging. Screenshots as feedback loops.

---

## Architecture Overview

```
engine/ops/           <- shared operation layer (CLI + editor + future frontends)
engine/cli/           <- thin clap frontend over ops
engine/editor/        <- Tauri + Svelte frontend over ops
  src-tauri/          <- Rust backend (commands, bridge, viewport)
  src/                <- Svelte frontend (shell, panel loader, theme)
  sdk/                <- @silmaril/editor-sdk npm package for plugin authors
```

### Communication

- **Svelte -> Rust:** `invoke()` for commands (user actions, writes)
- **Rust -> Svelte:** `emit()` for events (state changes, pushed reads)
- **Subscription model:** Svelte subscribes to channels with optional throttling
- **Viewport:** Direct native Vulkan rendering, no IPC

### Progress Reporting

Operations in `engine/ops` report progress via a trait:

```rust
pub trait ProgressSink: Send + Sync {
    fn on_start(&self, operation: &str, total_steps: usize);
    fn on_step(&self, operation: &str, step: usize, message: &str);
    fn on_done(&self, operation: &str, success: bool);
}
```

CLI implements with indicatif spinners. Editor implements by emitting Tauri events. Diagnostic logging continues via `tracing` as-is.

---

## `engine/ops` — Shared Operation Layer

Extract all operations from `engine/cli/src/commands/` into a standalone crate with no CLI framework dependency.

| Module | Operations | Maps to CLI source |
|--------|-----------|-------------------|
| `ops::project` | `create_project`, `load_project_config`, `find_project_root` | `commands/new.rs`, `commands/template.rs`, `commands/add/wiring.rs` |
| `ops::codegen` | `add_component`, `add_system` | `commands/add/component.rs`, `commands/add/system.rs`, `codegen/*` |
| `ops::module` | `add_module`, `remove_module`, `list_modules` | `commands/add/module.rs`, `commands/module/*` |
| `ops::build` | `build_platforms`, `package_platforms`, env merging, platform resolution, WASM builds (trunk), native/cross builds, installer generation | `commands/build/mod.rs`, `env.rs`, `native.rs`, `wasm.rs`, `package.rs`, `installer.rs` |
| `ops::scene` | `save_scene`, `load_scene`, `scene_to_bincode` | New (uses existing `WorldState` serialization) |
| `ops::undo` | `UndoStack`, `EditorAction`, execute/reverse logic | New (shared so both editor and future frontends can use it) |

**Editor-only (not in ops):**

| Module | Responsibility |
|--------|---------------|
| `editor::world` | Live ECS world management — `create_entity`, `delete_entity`, `set_component`, `query_entities`. These operate on an in-memory `World` and are editor-specific. The CLI does not maintain a live world. |
| `editor::viewport` | Vulkan surface, render loop, entity picking |
| `editor::bridge` | Tauri commands, subscriptions, events |
| `editor::plugins` | Panel plugin loader and registry |

**`silm dev` stays CLI-only.** The editor's play mode is architecturally different from `silm dev` (which manages external processes). The editor runs the game loop in-process. The dev command's file watcher (`commands/dev/watcher.rs`) may be reused by `silm build --watch` via `ops::build`, but the dev orchestrator, process manager, and reload client stay in the CLI.

**What stays in `engine/cli`:** Clap structs, `main()` dispatch, spinners, terminal formatting, `silm dev` command.

**What's new in `engine/editor`:** Tauri commands (thin wrappers over ops + editor-only modules), subscription bridge, viewport, plugin loader.

### Error Types

Per CLAUDE.md, both new crates define custom error types with reserved code ranges:

```rust
// engine/ops — error codes 2200-2299
define_error! {
    pub enum OpsError {
        ProjectNotFound { path: String } = ErrorCode::OpsProjectNotFound, ErrorSeverity::Error,
        GameTomlInvalid { reason: String } = ErrorCode::OpsGameTomlInvalid, ErrorSeverity::Error,
        BuildFailed { platform: String, reason: String } = ErrorCode::OpsBuildFailed, ErrorSeverity::Error,
        ToolNotFound { tool: String } = ErrorCode::OpsToolNotFound, ErrorSeverity::Error,
        SceneParseFailed { path: String, reason: String } = ErrorCode::OpsSceneParseFailed, ErrorSeverity::Error,
        ModuleNotFound { name: String } = ErrorCode::OpsModuleNotFound, ErrorSeverity::Error,
        CodegenFailed { reason: String } = ErrorCode::OpsCodegenFailed, ErrorSeverity::Error,
    }
}

// engine/editor — error codes 2300-2399
define_error! {
    pub enum EditorError {
        ViewportInitFailed { reason: String } = ErrorCode::EditorViewportInit, ErrorSeverity::Critical,
        PluginLoadFailed { name: String, reason: String } = ErrorCode::EditorPluginLoad, ErrorSeverity::Error,
        UndoStackEmpty = ErrorCode::EditorUndoEmpty, ErrorSeverity::Warning,
        RedoStackEmpty = ErrorCode::EditorRedoEmpty, ErrorSeverity::Warning,
        SubscriptionUnknown { channel: String } = ErrorCode::EditorSubUnknown, ErrorSeverity::Error,
        EntityNotFound { id: String } = ErrorCode::EditorEntityNotFound, ErrorSeverity::Error,
        SceneSaveFailed { reason: String } = ErrorCode::EditorSceneSave, ErrorSeverity::Error,
    }
}
```

**Note:** `engine/ops` uses custom `OpsError` (not `anyhow`). The CLI wraps `OpsError` into `anyhow` at the CLI boundary. The editor wraps it into `EditorError` or returns it via Tauri command error handling.

---

## Editor Bridge — Commands, Subscriptions, Events

### Commands (Svelte -> Rust via `invoke()`)

```rust
// Entity operations
create_entity() -> EntityId
delete_entity(id)
set_component(id, name, data: Value)
add_component(id, name)
remove_component(id, name)
select_entity(id: Option<EntityId>)

// Scene operations
save_scene(path)
load_scene(path)
new_scene()

// Project operations (delegates to ops)
open_project(path) -> ProjectInfo
build(platforms: Vec<String>, release: bool)
package(platforms: Vec<String>)

// Codegen (delegates to ops)
add_component_codegen(name, target, domain, fields)
add_system_codegen(name, target, domain, query)

// Module management (delegates to ops)
add_module(name, source: ModuleSource)
remove_module(name)
list_modules() -> Vec<ModuleInfo>

// Editor state
undo()
redo()
subscribe(channel, config: SubscriptionConfig)
unsubscribe(channel)

// Playback
play()
pause()
stop()
```

### Subscriptions (Rust -> Svelte via `emit()`)

Svelte subscribes to channels. Rust pushes data only for active subscriptions, throttled.

```rust
pub struct SubscriptionConfig {
    pub entity_id: Option<EntityId>,  // filter to specific entity (None = all)
    pub throttle_ms: Option<u64>,     // minimum push interval
}

// Returns a subscription ID for targeted unsubscribe
subscribe(channel, config) -> SubscriptionId
unsubscribe(subscription_id: SubscriptionId)
```

**Subscription lifecycle:**
- `subscribe()` returns a `SubscriptionId` (u64). Multiple subscriptions on the same channel with different filters are allowed.
- `unsubscribe(id)` removes a specific subscription.
- **Auto-cleanup:** The SDK (`@silmaril/editor-sdk`) wraps subscriptions in a Svelte `onDestroy` hook. When a panel unmounts, all its subscriptions are automatically unsubscribed.
- **Mode transitions:** Subscriptions survive Edit -> Play -> Pause -> Stop. The Rust side simply has no data to push in Edit mode (no game loop), so subscriptions idle.
- **Error delivery:** If a subscription callback fails (e.g., serialization error), the error is emitted on a separate `subscription:error` channel.

**Known limitation (Phase 0.8):** `entity_id` filters to a single entity. Filtering by component type or entity set is deferred.

| Channel | Data | When |
|---------|------|------|
| `entity:selected` | `{ id, components: [...] }` | Selection changes |
| `entity:updated` | `{ id, components: [...] }` | Selected entity data changed (play mode) |
| `hierarchy:changed` | `{ added: [...], removed: [...] }` | Entity spawn/despawn diffs |
| `console:log` | `{ level, message, timestamp }` | Log entries arrive |
| `metrics:frame` | `{ fps, frame_ms, entity_count }` | Profiler data |
| `build:progress` | `{ platform, status }` | Build progress |
| `project:loaded` | `{ name, version, platforms }` | Project opened |

### Throttling

```rust
struct Subscription {
    id: SubscriptionId,
    channel: String,
    filter: Option<EntityId>,
    throttle: Duration,
    last_push: Instant,
}
```

Rust checks elapsed time since last push per subscription. Skips if throttle hasn't elapsed. Keeps IPC bounded regardless of game tick rate.

### Tracing Bridge

The `console:log` channel is fed by a custom `tracing` subscriber. The editor registers a `TracingBridge` subscriber that captures `tracing` spans/events and pushes them to subscriptions on the `console:log` channel. This means all engine `tracing::info!` / `tracing::warn!` / `tracing::error!` calls automatically appear in the editor console panel.

---

## Undo/Redo System

Command pattern with snapshot checkpoints.

```rust
pub enum EditorAction {
    SetComponent { entity: EntityId, name: String, old: Value, new: Value },
    AddComponent { entity: EntityId, name: String },
    RemoveComponent { entity: EntityId, name: String, snapshot: Value },
    CreateEntity { id: EntityId },
    DeleteEntity { id: EntityId, snapshot: EntitySnapshot },
    RenameEntity { id: EntityId, old_name: String, new_name: String },
    Batch { label: String, actions: Vec<EditorAction> },
}

pub struct UndoStack {
    done: Vec<EditorAction>,
    undone: Vec<EditorAction>,
    max_depth: usize,  // default 100
}
```

**Rules:**
- Every mutation pushes an `EditorAction` before executing
- `undo()` pops from `done`, reverses, pushes to `undone`
- `redo()` pops from `undone`, re-executes, pushes to `done`
- New action clears `undone`
- `Batch` groups related actions (one undo step)
- Play mode suspends the undo stack

---

## Scene Files

YAML for development, Bincode for release.

- **Development:** `.scene.yaml` — human-readable, git-diffable, AI-readable
- **Release:** `silm package` converts to Bincode automatically
- Uses existing `WorldState` serialization infrastructure

---

## Panel Plugin System

Three tiers of panel authoring.

### Tier 1: Schema (Cargo.toml metadata)

Zero code. Module declares fields in `Cargo.toml`:

```toml
[package.metadata.silmaril.editor]
panel_title = "Combat Settings"

[[package.metadata.silmaril.editor.sections]]
name = "Damage"
fields = [
    { name = "base_damage", type = "f32", range = [0, 100], default = 10.0 },
    { name = "crit_multiplier", type = "f32", range = [1, 5], default = 2.0 },
    { name = "damage_type", type = "enum", options = ["Physical", "Magical", "True"] },
]
```

Supported field types: `f32`, `i32`, `bool`, `String`, `enum`, `Vec3`, `Color`, `EntityRef`, `asset_path`

Editor auto-generates Svelte UI from the schema via a generic `<SchemaPanel>` component.

### Tier 2: Rust `panel!` macro (Phase 4.9)

Declarative, more control, still no JS. The macro expands to a build-script-generated JSON descriptor file (`<crate>/editor-panel.json`) that is functionally equivalent to Tier 1 schema metadata. The editor loads this JSON the same way it loads Tier 1 schemas.

```rust
use silmaril_editor::panel;

panel! {
    title: "Combat Settings",
    section "Damage" {
        slider "Base Damage" => combat.base_damage { range: 0.0..100.0 },
        dropdown "Type" => combat.damage_type { options: ["Physical", "Magical", "True"] },
    }
}
```

**Note:** The `custom` escape hatch (loading a JS bundle from within a Tier 2 macro) is removed. If a module needs JS, it should use Tier 3 directly. Mixing tiers in one module is confusing.

### Tier 3: Full Svelte (via `silm editor panel`)

Maximum power. Module ships a compiled JS bundle.

```bash
silm editor panel init       # scaffolds editor/ with SDK, Vite, template
silm editor panel dev        # hot-reload preview in standalone window
silm editor panel build      # outputs assets/editor/panel.js
```

Module structure:
```
my-module/
├── Cargo.toml
├── src/lib.rs
└── editor/
    ├── package.json        <- depends on @silmaril/editor-sdk
    ├── Panel.svelte        <- custom panel component
    └── build.js            <- outputs assets/editor/panel.js
```

### `@silmaril/editor-sdk` npm package

Provides to Tier 3 plugin authors:
- shadcn-svelte components pre-styled with editor theme
- Design tokens (colors, spacing, typography)
- Typed API: `getComponent()`, `setComponent()`, `subscribe()`, `undo()`
- Vite build config for producing loadable bundles

### Tier 3 Sandboxing

Tier 3 JS bundles are loaded inside an `<iframe>` with `sandbox="allow-scripts"`. The iframe communicates with the parent Svelte app via `postMessage`. The SDK wraps `postMessage` into typed API calls (`getComponent()`, `setComponent()`, `subscribe()`).

**Allowed inside iframe:** JavaScript execution, SDK API calls (proxied to Rust via parent).
**Blocked:** Direct filesystem access, `invoke()` calls, `fetch()` to arbitrary URLs, DOM access outside the iframe.

Core module panels (hierarchy, inspector, viewport, console) are **not sandboxed** — they are compiled directly into the editor Svelte app since they are first-party and need full access to the Tauri bridge.

### Plugin loading

1. On project open, read `game.toml [modules]`
2. For each module, read `Cargo.toml [package.metadata.silmaril.editor]`
3. If `panel_title` exists but no `panel_bundle` → Tier 1 schema, render via `<SchemaPanel>`
4. If `panel_bundle` path exists → Tier 3, load via `<PluginFrame>` (iframe sandboxed)
5. If `editor-panel.json` exists in crate (generated by Tier 2 macro) → treat as Tier 1

### Core modules provide their own panels

| Module | Panel | Tier |
|--------|-------|------|
| `engine/core` | Hierarchy, Inspector | 3 (complex tree/form UI) |
| `engine/renderer` | Viewport, Scene controls | 3 (native Vulkan + controls) |
| `engine/profiling` | Profiler (flamegraph, metrics) | 3 (complex visualization) |
| `engine/audio` | Audio mixer | 2 (sliders, toggles) |
| `engine/physics` | Physics debugger | 3 (overlay visualization) |
| `engine/cli` (ops) | Console, Build panel | 3 (log stream, progress) |

---

## Viewport and Play Mode

### Three modes

| Mode | Viewport | ECS | Undo | Subscriptions |
|------|----------|-----|------|---------------|
| **Edit** | Renders scene, editor camera | Frozen — user edits only | Active | On user action |
| **Play** | Game loop at 60fps | Systems execute each tick | Suspended | Active — push updates |
| **Pause** | Frozen frame | Frozen at current tick | Active | On user action |

### Mode transitions

- **Edit -> Play:** Snapshot world state (Bincode, 0.126ms/1000 entities), start game loop
- **Play -> Stop:** Restore snapshot, discard all play-mode changes
- **Play -> Pause:** Stop ticking, keep state, re-enable undo
- **Pause -> Play:** Resume ticking, suspend undo

### Viewport implementation

- Creates Vulkan surface from Tauri window's `raw-window-handle`
- Owns render loop on separate thread
- Edit mode: render on demand (camera move, entity change)
- Play mode: render every frame at target FPS

### Entity picking

Click in viewport -> Rust GPU raycast or color-ID pass -> returns EntityId -> bridge emits `entity:selected`

---

## Project Structure

```
engine/editor/
├── Cargo.toml
├── tauri.conf.json
├── build.rs
├── src-tauri/
│   ├── main.rs                       <- Tauri entry, plugin registration
│   ├── bridge/
│   │   ├── mod.rs                    <- EditorBridge
│   │   ├── commands.rs               <- #[tauri::command] handlers
│   │   ├── subscriptions.rs          <- Subscription tracking, throttling
│   │   ├── events.rs                 <- Event emission helpers
│   │   └── tracing_bridge.rs         <- Custom tracing subscriber -> console:log
│   ├── viewport/
│   │   ├── mod.rs                    <- Vulkan surface from raw window handle
│   │   ├── render_loop.rs            <- Edit/play/pause rendering
│   │   └── picking.rs               <- Entity picking
│   ├── world/
│   │   ├── mod.rs                    <- Live ECS world (editor-only, not in ops)
│   │   └── queries.rs               <- Entity/component query helpers
│   ├── plugins/
│   │   ├── mod.rs                    <- Plugin loader
│   │   ├── schema_renderer.rs        <- Schema -> component props
│   │   └── registry.rs              <- Panel registry
│   ├── scene/
│   │   ├── mod.rs                    <- Save/load orchestration
│   │   └── yaml.rs                   <- YAML <-> WorldState
│   └── state.rs                      <- EditorState
├── src/                              <- Svelte frontend
│   ├── app.html
│   ├── App.svelte                    <- Root: panel shell + docking
│   ├── lib/
│   │   ├── api.ts                    <- Typed invoke()/listen() wrappers
│   │   ├── stores/
│   │   │   ├── editor.ts             <- Selected entity, mode, project
│   │   │   ├── hierarchy.ts          <- Entity tree (reactive via events)
│   │   │   └── console.ts            <- Log entries
│   │   ├── components/
│   │   │   ├── PanelShell.svelte     <- Dockable panel container
│   │   │   ├── SchemaPanel.svelte    <- Auto-renders from schema
│   │   │   └── PluginFrame.svelte    <- Loads Tier 3 JS bundles
│   │   └── theme/
│   │       └── tokens.ts             <- Design tokens for SDK
│   └── panels/                       <- Empty — all panels from modules
├── sdk/                              <- @silmaril/editor-sdk
│   ├── package.json
│   ├── index.ts                      <- API types, theme, components
│   ├── components/                   <- Re-exported shadcn components
│   └── vite.config.ts                <- Build config for panel bundles
└── static/
    └── favicon.png
```

Core module panels:
```
engine/core/editor/               <- Hierarchy + Inspector (Tier 3)
engine/renderer/editor/           <- Viewport panel (Tier 3)
engine/profiling/editor/          <- Profiler panel (Tier 3)
engine/cli/editor/                <- Console + Build panels (Tier 3)
```

---

## Testing Strategy

### Layer 1: `engine/ops` unit tests (CI — fast, no GPU)

Pure logic, mocked ProgressSink. UndoStack lives in `ops::undo` so it is testable here:
- All operations tested independently
- Undo/redo architecture tests (in `ops::undo`):
  - SetComponent undo restores old value
  - CreateEntity undo removes entity
  - DeleteEntity undo restores entity + all components
  - Redo after undo re-applies action
  - New action clears redo stack
  - Batch undo reverses all in one step
  - Max depth evicts oldest
  - Empty undo/redo returns error
- Scene round-trip (YAML save -> load, Bincode save -> load)
- Plugin schema parsing from Cargo.toml metadata

### Layer 2: Bridge integration tests (CI — fast, no GPU)

Rust-side tests with mocked ECS:
- Commands produce correct mutations
- Subscriptions deliver events at correct throttle rate
- Mode transitions snapshot/restore correctly
- Plugin registry discovers module panels

### Layer 3: Svelte UI tests (Playwright + MCP — CI headless)

Playwright drives the Svelte UI, MCP server exposes test controls:
- SchemaPanel renders correct inputs per field type
- PluginFrame loads and sandboxes JS bundles
- Stores update reactively on events
- Screenshot capture after each interaction
- Visual regression against baseline screenshots

```
tests/editor/screenshots/
├── baselines/               <- committed golden screenshots
│   ├── hierarchy-empty.png
│   ├── inspector-health.png
│   └── ...
└── diffs/                   <- generated on failure (gitignored)
```

### Layer 4: E2E editor tests (Playwright + Tauri — local/CI with GPU)

Full Tauri app via tauri-driver:
- Open project -> panels load -> hierarchy shows entities
- Edit component -> undo -> value restored
- Play -> entities move -> stop -> state restored
- Build from editor -> progress events -> completion
- AI-driven exploration: MCP + screenshots feedback loop

---

## Performance Targets

| Metric | Target |
|--------|--------|
| Editor startup | < 3s |
| Viewport FPS | 60+ |
| UI panel responsiveness | < 16ms |
| IPC invoke round-trip | < 1ms |
| Event push latency | < 5ms |
| Undo/redo execution | < 1ms |
| Scene save (1000 entities) | < 10ms |
| Memory overhead (editor) | < 500MB |

---

## Implementation Phases

### Phase 0.8: Foundation (3-4 weeks)

1. Extract `engine/ops` from CLI
2. Create `engine/editor` Tauri project (shell + plugin loader)
3. Implement bridge (commands + subscriptions + events)
4. Implement undo/redo system
5. Implement viewport (Vulkan surface in Tauri window)
6. Implement core module panels (hierarchy, inspector, viewport, console)
7. Scene save/load (YAML)
8. Plugin system (Tier 1 schema + Tier 3 JS bundles)
9. `@silmaril/editor-sdk` npm package
10. `silm editor panel` CLI commands

### Phase 4.9: Advanced (3-4 weeks)

1. Drag-drop entity manipulation (gizmos, multi-select)
2. Full AI integration (code generation, debugging, MCP)
3. Asset import pipeline (GLTF, FBX, PNG)
4. Material editor (visual PBR)
5. Profiler flamegraph UI
6. Tier 2 `panel!` macro
7. Playwright + MCP testing infrastructure

---

## Conventions

### `engine/*/editor/` directories

Core engine crates may contain an `editor/` subdirectory with panel source code (Svelte projects for Tier 3, or schema metadata for Tier 1). This is explicitly sanctioned and distinct from the forbidden `engine/*/examples/` pattern in CLAUDE.md. The `editor/` directories contain production panel code that ships with the editor, not throwaway examples.

### Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| Ctrl+Z / Cmd+Z | Undo |
| Ctrl+Y / Cmd+Shift+Z | Redo |
| Ctrl+S / Cmd+S | Save scene |
| Delete | Delete selected entity |
| Ctrl+D / Cmd+D | Duplicate selected entity |
| F5 | Play |
| Shift+F5 | Stop |
| F6 | Pause |
| Ctrl+B / Cmd+B | Build |

Shortcuts are handled by the Svelte shell and forwarded as `invoke()` calls.

### Scene Editor Metadata

Editor-specific state (camera position, selection, panel layout) is stored in a sidecar file `<scene>.editor.yaml` alongside the scene file. This file is gitignored by default (added to the template `.gitignore`). It is not required — the editor works without it, using defaults.

```yaml
# my-scene.editor.yaml
camera:
  position: [0, 10, -20]
  rotation: [0.3, 0, 0, 1]
  zoom: 1.0
selected_entities: [42, 17]
panel_layout: "default"
```

---

## Known Limitations (Phase 0.8)

- No drag-drop gizmos (Phase 4.9)
- No asset import pipeline (Phase 4.9)
- No material editor (Phase 4.9)
- No profiler flamegraph (Phase 4.9 — basic metrics only)
- No Tier 2 `panel!` macro (Phase 4.9)
- macOS Vulkan via MoltenVK (may have viewport quirks)
- Panel docking is fixed layout initially (resizable but not rearrangeable)
