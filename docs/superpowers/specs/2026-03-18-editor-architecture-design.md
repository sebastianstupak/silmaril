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

| Module | Operations |
|--------|-----------|
| `ops::project` | `create_project`, `load_project_config`, `find_project_root` |
| `ops::codegen` | `add_component`, `add_system` |
| `ops::module` | `add_module`, `remove_module`, `list_modules` |
| `ops::build` | `build_platforms`, `package_platforms`, env merging, platform resolution |
| `ops::scene` | `save_scene`, `load_scene`, `scene_to_bincode` |
| `ops::world` | `create_entity`, `delete_entity`, `set_component`, `query_entities` |

**What stays in `engine/cli`:** Clap structs, `main()` dispatch, spinners, terminal formatting.

**What's new in `engine/editor`:** Tauri commands (thin wrappers over ops), subscription bridge, viewport, undo stack, plugin loader.

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
    pub entity_id: Option<EntityId>,
    pub throttle_ms: Option<u64>,
}
```

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
    channel: String,
    filter: Option<EntityId>,
    throttle: Duration,
    last_push: Instant,
}
```

Rust checks elapsed time since last push per subscription. Skips if throttle hasn't elapsed. Keeps IPC bounded regardless of game tick rate.

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

### Tier 2: Rust `panel!` macro

Declarative, more control, still no JS:

```rust
use silmaril_editor::panel;

panel! {
    title: "Combat Settings",
    section "Damage" {
        slider "Base Damage" => combat.base_damage { range: 0.0..100.0 },
        dropdown "Type" => combat.damage_type { options: ["Physical", "Magical", "True"] },
    }
    custom "Combo Editor" => "assets/editor/combo-editor.js",
}
```

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

### Plugin loading

1. On project open, read `game.toml [modules]`
2. For each module, read `Cargo.toml [package.metadata.silmaril.editor]`
3. Tier 1/2: render via `<SchemaPanel>` component
4. Tier 3: load JS bundle via `<PluginFrame>` component (sandboxed)

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
│   │   └── events.rs                 <- Event emission helpers
│   ├── viewport/
│   │   ├── mod.rs                    <- Vulkan surface from raw window handle
│   │   ├── render_loop.rs            <- Edit/play/pause rendering
│   │   └── picking.rs               <- Entity picking
│   ├── undo/
│   │   ├── mod.rs                    <- UndoStack
│   │   └── actions.rs               <- EditorAction enum, execute/reverse
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

Pure logic, mocked ProgressSink:
- All operations tested independently
- Undo/redo architecture tests:
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

## Known Limitations (Phase 0.8)

- No drag-drop gizmos (Phase 4.9)
- No asset import pipeline (Phase 4.9)
- No material editor (Phase 4.9)
- No profiler flamegraph (Phase 4.9 — basic metrics only)
- No Tier 2 `panel!` macro (Phase 4.9)
- macOS Vulkan via MoltenVK (may have viewport quirks)
- Panel docking is fixed layout initially (resizable but not rearrangeable)
