# Omnibar Design Spec

**Date:** 2026-03-20
**Status:** Approved
**Area:** Editor — TitleBar

---

## Overview

A persistent omnibar in the center of the TitleBar. Mirrors VS Code's center search bar: always visible as a clickable input, expands into a grouped dropdown on focus. A single input surfaces commands, scene template entities, assets, keybind hints, and recent projects/scenes — all dynamically discovered, none hardcoded.

---

## Goals

- Power-user keyboard-first workflow via `Ctrl+K`
- Discoverable for new users (always visible in TitleBar center)
- AI agent automation: agents can query the full command list via Tauri IPC without going through the WebView
- Extensible: future plugins and engine subsystems register commands where they live (Rust or TS) without modifying the omnibar

---

## Non-Goals

- No natural language / AI query parsing (out of scope for v1)
- No command argument prompting (commands that need args handle that themselves)
- No fuzzy ranking ML — simple prefix + substring match is sufficient

---

## Architecture

### Two Registries, Merged at Query Time

| Registry | Location | Owns |
|---|---|---|
| `CommandRegistry` (Rust) | `src-tauri/bridge/registry.rs` | Engine/editor commands |
| `FrontendCommandRegistry` (TS) | `src/lib/omnibar/registry.ts` | UI-only commands (open dialogs, toggle panels) |

The omnibar fetches Rust commands once on startup via `list_commands` IPC, caches them, and re-fetches when plugins load. TS commands are always in memory.

**Execution routing:**
1. Check TS registry first → call `run()` locally
2. Not found → `invoke('run_command', { id, args? })`

`run_command` returns `Result<(), String>` on the Rust side, serialized to `{ ok: true }` or `{ error: string }` on the TS side. The frontend must handle both the "command not found" case (error string) and execution failures distinctly.

**Namespacing & deduplication:** IDs are namespaced by prefix (`editor.*` for Rust engine commands, `ui.*` for TS UI commands). If both registries contain the same ID, the TS registry takes precedence (TS-first routing). Plugins must use a unique namespace (e.g. `myplugin.*`) to avoid shadowing built-ins.

This means UI commands (open settings dialog, pop out a panel) never need a Rust round-trip.

---

## Data Model

### Rust — `EditorCommand`

```rust
#[derive(Serialize, Clone)]
pub struct EditorCommand {
    pub id: String,              // "editor.toggle_grid"
    pub label: String,           // "Toggle Grid"
    pub category: String,        // "View" | "Entity" | "Scene" | "Tool" | "Layout"
    pub keybind: Option<String>, // "Ctrl+G"
    pub description: Option<String>,
}
```

### TypeScript — `FrontendCommand`

```ts
interface FrontendCommand {
  id: string;              // "ui.open_settings"
  label: string;
  category: string;
  keybind?: string;
  description?: string;
  run: () => void | Promise<void>;  // local handler — never goes to Rust
}
```

### Omnibar Result Union

```ts
type OmnibarResult =
  | { kind: 'command';  command: EditorCommand | FrontendCommand }
  | { kind: 'entity';   id: number; name: string; components: string[] }
  | { kind: 'asset';    path: string; assetType: string }
  | { kind: 'recent';   label: string; path: string; itemType: 'project' | 'scene' }
```

---

## Result Sources (5 Providers)

| Provider | Data source | Notes |
|---|---|---|
| Commands | Rust `list_commands` + TS registry | Keybind hint shown inline on each result |
| Scene Template Entities | `editor-context` store (`getEditorContext().entities`) | Reflects entities from the currently open scene template |
| Assets | `scan_assets` Tauri command (new) | Only active when a project is open |
| Recent | `persist` store key `omnibar.recent` (max 10 entries) | Shown on empty input alongside top 5 most-used commands |
| Keybind hints | Decorates command results | Not a standalone source |

`scan_assets` is a new Tauri command added in `bridge/commands.rs`. Signature: `scan_assets(project_path: String) -> Result<Vec<AssetInfo>, String>` where `AssetInfo { path: String, asset_type: String }`. Performs a recursive scan for known extensions (`.png`, `.jpg`, `.gltf`, `.glb`, `.wav`, `.ogg`, `.toml`). Called lazily on first `#` prefix use (not eagerly on project open) to avoid blocking startup.

---

## Prefix Routing

| Prefix | Filters to |
|---|---|
| `>` | Commands only |
| `@` | Scene template entities only |
| `#` | Assets only |
| _(none)_ | All sources, ranked by relevance |

Empty input shows: recent projects/scenes + top 5 most-recently-used commands.

---

## UI & Interaction

**Idle state (TitleBar center):**
```
[ 🔍  Search commands, entities, assets…          Ctrl+K ]
```

**Active state:** input border highlights, dropdown opens below, rest of editor dims slightly.

**Keyboard navigation:**
- `Ctrl+K` — open from anywhere
- `↑ / ↓` — navigate results
- `Enter` — execute selected, dismiss
- `Escape` — dismiss, restore previous focus
- `Tab` — cycle between result groups (Commands → Entities → Assets → Recent); empty groups are skipped

**Result row anatomy:**
```
[icon]  Label                   [Category badge]  [Ctrl+G]
```
Matched characters highlighted in accent color. Category badge muted. Keybind right-aligned, dimmed.

---

## File Structure

### New Files

```
engine/editor/src/lib/omnibar/
├── Omnibar.svelte          ← input + dropdown component
├── registry.ts             ← FrontendCommandRegistry
├── providers.ts            ← entity, asset, recent result providers
├── types.ts                ← OmnibarResult, FrontendCommand
└── fuzzy.ts                ← fuzzy match + result ranking

engine/editor/src-tauri/bridge/
├── registry.rs             ← CommandRegistry struct, register(), list_commands()
└── runner.rs               ← run_command() dispatcher
```

`registry.rs` and `runner.rs` are added as submodules of the existing `bridge/` module. `bridge/mod.rs` must `pub mod registry; pub mod runner;`.

### Modified Files

| File | Change |
|---|---|
| `TitleBar.svelte` | Mount `<Omnibar />` in center; add omnibar's root CSS class to the `.closest()` selector guard in `onTitlebarMousedown` and `onTitlebarDblclick` so clicks don't trigger window drag (see Drag Region note below) |
| `lib.rs` | Register built-in Rust commands on startup via `bridge::registry::CommandRegistry`; add `list_commands`, `run_command`, and `scan_assets` to `invoke_handler!` macro |
| `bridge/commands.rs` | Add `scan_assets` command |
| `bridge/mod.rs` | Add `pub mod registry; pub mod runner;` |
| `App.svelte` | Add `Ctrl+K` case to the existing `$effect` keydown handler (alongside `Ctrl+Tab`, `Ctrl+,`) |

### Drag Region Note

The TitleBar handles drag manually via `onTitlebarMousedown` → `invoke('window_start_drag')` with a `.closest(selector)` guard to exclude interactive elements. The `<Omnibar />` root element must have a stable CSS class (`.omnibar-wrapper`) that is added to this `.closest()` exclusion selector in both `onTitlebarMousedown` and `onTitlebarDblclick`. The `data-tauri-drag-region` HTML attribute is not used by this codebase and must not be relied upon.

### Unchanged

`dispatchSceneCommand` and the existing scene command API are untouched. The command registry is a **discovery + routing layer on top**, not a replacement.

---

## Built-in Commands (Initial Registration)

Rust side registers on startup:

| ID | Label | Category | Keybind |
|---|---|---|---|
| `editor.toggle_grid` | Toggle Grid | View | Ctrl+G |
| `editor.toggle_snap` | Toggle Snap to Grid | View | — |
| `editor.toggle_projection` | Toggle Projection | View | — |
| `editor.new_scene` | New Scene | Scene | — |
| `editor.reset_camera` | Reset Camera | View | — |
| `editor.set_tool.select` | Set Tool: Select | Tool | Q |
| `editor.set_tool.move` | Set Tool: Move | Tool | W |
| `editor.set_tool.rotate` | Set Tool: Rotate | Tool | E |
| `editor.set_tool.scale` | Set Tool: Scale | Tool | R |

TS side registers:

| ID | Label | Category |
|---|---|---|
| `ui.open_settings` | Open Settings | Editor |
| `ui.open_project` | Open Project… | File |
| `ui.layout.reset` | Reset Layout | Layout |

---

## Testing

- Unit: fuzzy match ranking, prefix routing, result deduplication (TS)
- Unit: Rust `CommandRegistry` register/list/run dispatch
- Integration: `list_commands` IPC returns expected commands on startup
- Integration: `run_command` executes and returns `{ ok: true }`
- E2E (Playwright): open omnibar with `Ctrl+K`, type `>tog`, arrow-select "Toggle Grid", Enter → grid state changes
