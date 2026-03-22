# Editor Layout Presets & Dynamic Panel Registry

**Date:** 2026-03-22
**Status:** Approved

---

## Overview

Three changes to the editor shell:

1. Remove the redundant **View menu** from the title bar menu strip
2. Replace the three hardcoded layout presets (Edit / Assets / Review) with **workflow-stage presets** (Scene / Code / Perf) with appropriate panel arrangements
3. Replace all hardcoded panel lists with a **dynamic ContributionRegistry** that builtin modules, Rust cargo modules, and runtime `.silm` modules can all feed into

---

## 1. Remove the View Menu

The View menu (File / Edit / **View** / Entity / Build / Modules / Help) is removed entirely. Its contents redistribute:

| Item | New location |
|------|-------------|
| Layout sub-menu (switch preset) | Already present in right-side slot switcher — just drop |
| Layout reset | Moves into the right-side slot switcher dropdown (already has a reset action) |
| AI Server toggle | New status indicator button in titlebar right area |

**AI Server indicator button:** placed between the panels button and the settings button in the titlebar right area. Appearance: small icon with a status dot — grey = stopped, pulsing green = running. Click toggles the server. Replaces the current text-based toggle in the View menu.

---

## 2. Workflow-Stage Layout Presets

Replaces the three builtin slots in `initialSavedLayouts` in `engine/editor/src/lib/docking/store.ts`.

### `builtin-scene` — "Scene" (Ctrl+1)

Scene composition. Viewport dominant, asset browsing accessible on the left.

```
root: horizontal split [20%, 55%, 25%]
  left: vertical split [65%, 35%]
    top:    tabs [hierarchy]
    bottom: tabs [assets, file-explorer]
  center: tabs [viewport]
  right:  tabs [inspector]
bottomPanel: tabs [console, output]
```

> **Note:** `terminal` is intentionally absent from the Scene preset. Scene composition does not require a shell; keeping the bottom strip minimal reduces distraction. Terminal is available via the spawn menu or the Code preset. This is a deliberate design choice, not an omission.

### `builtin-code` — "Code" (Ctrl+2)

Dev iteration. Viewport present but smaller; full tool strip at the bottom.

```
root: horizontal split [20%, 50%, 30%]
  left: vertical split [60%, 40%]
    top:    tabs [hierarchy]
    bottom: tabs [assets]
  center: tabs [viewport]
  right:  tabs [inspector]
bottomPanel: tabs [console, terminal, output]
```

### `builtin-perf` — "Perf" (Ctrl+3)

Performance profiling. Profiler gets a dedicated horizontal strip inside the main area; bottom panel is empty.

```
root: vertical split [65%, 35%]
  top: horizontal split [15%, 55%, 30%]
    left:   tabs [hierarchy]
    center: tabs [viewport]
    right:  tabs [inspector]
  bottom: horizontal split [70%, 30%]
    left:  tabs [profiler]
    right: tabs [console, output]
bottomPanel: tabs []
```

> **Empty bottom panel encoding:** `tabs []` (an empty `TabsNode` with zero panels) is the correct way to express "no bottom panel" — `EditorLayout.bottomPanel` is non-optional and typed as `LayoutNode`. `DockContainer` already handles empty tab nodes (renders nothing). This is intentional, not a structural mistake.

---

## 3. Dynamic Contribution Registry

### `types.ts` cleanup

`panelRegistry` (static array) is removed from `types.ts`. The utility functions `getBasePanelId` and `createPanelInstance` **stay** in `types.ts` — they are pure string utilities unrelated to the registry. `getPanelInfo` is removed; callers that needed display metadata now call `getPanelComponent` from the registry or read the `PanelContribution` object directly.

`getPanelComponent` in the registry uses `getBasePanelId` internally to support instance IDs (`'viewport:2'` → looks up `'viewport'`). `DockContainer` continues to import `getBasePanelId` from `types.ts` for any instance-ID stripping it does outside of component lookup.

`DockTabBar.svelte` currently calls `getPanelInfo` to get the tab label (`titleKey`). It is updated to call `getPanelComponent`'s companion lookup — specifically, a new exported helper `getPanelTitle(id: string): string` on the registry that returns the `title` field of the matching `PanelContribution` (using `getBasePanelId` internally). `DockTabBar.svelte` is added to the Affected Files table.

### Problem

Two parallel hardcoded panel lists exist today:

- `panelRegistry` in `docking/types.ts` — 9 items, used for tab labels
- `ALL_PANELS` in `TitleBar.svelte` — 6 items (missing file-explorer, terminal, output), used for the spawn menu
- `panelComponents` in `App.svelte` — 9 items, maps ID → Svelte component

These cannot be extended from outside the editor codebase.

### Solution: ContributionRegistry

A new module `engine/editor/src/lib/contributions/registry.ts` owns a single reactive store. All panel registration and lookup flows through it.

#### Frontend API

The registry uses Svelte 5 runes internally — no Svelte 4 `Readable` stores. State is held in a module-level `$state` array; the read functions return the array directly (they are called inside reactive contexts such as `$derived`).

```ts
// engine/editor/src/lib/contributions/registry.ts

export interface PanelContribution {
  id: string;
  title: string;           // display name (replaces titleKey for now)
  icon?: string;           // SVG string or undefined
  component: Component;    // Svelte component
  source: string;          // 'builtin' | cargo crate name | module id
}

export interface InspectorFieldContribution {
  componentType: string;   // ECS component name, e.g. 'NetworkTransform'
  renderer: Component;     // Svelte component
  source: string;
}

// Module-level reactive state (Svelte 5 $state)
// let _panels = $state<PanelContribution[]>([]);
// let _inspectorFields = $state<InspectorFieldContribution[]>([]);

// Registration
export function registerPanel(c: PanelContribution): void
export function unregisterPanel(id: string): void

// Inspector fields — API only, wiring deferred to inspector task
export function registerInspectorField(c: InspectorFieldContribution): void
export function unregisterInspectorField(componentType: string, source: string): void

// Read — returns reactive $state array; call inside $derived or reactive blocks
export function getPanelContributions(): PanelContribution[]
export function getInspectorFieldContributions(): InspectorFieldContribution[]

// Look up component by ID (supports instance IDs like 'viewport:2' via getBasePanelId)
export function getPanelComponent(id: string): Component | undefined
```

In `App.svelte`:
```ts
let panelContributions = $derived(getPanelContributions());
```

#### Builtin panel self-registration

Each of the 9 existing panels registers itself by calling `registerPanel()` from a new file `engine/editor/src/lib/contributions/builtins.ts`, imported once in `App.svelte` before mount. This replaces both `panelRegistry` in types.ts (for spawn menu metadata) and the `panelComponents` map in App.svelte (for component lookup).

```ts
// contributions/builtins.ts
import { registerPanel } from './registry';
import HierarchyWrapper from '../docking/panels/HierarchyWrapper.svelte';
// ... other imports

export function registerBuiltinPanels() {
  registerPanel({ id: 'hierarchy',     title: 'Hierarchy',      component: HierarchyWrapper,    source: 'builtin' });
  registerPanel({ id: 'viewport',      title: 'Viewport',       component: ViewportPanel,        source: 'builtin' });
  registerPanel({ id: 'inspector',     title: 'Inspector',      component: InspectorWrapper,     source: 'builtin' });
  registerPanel({ id: 'console',       title: 'Console',        component: ConsoleWrapper,       source: 'builtin' });
  registerPanel({ id: 'profiler',      title: 'Profiler',       component: ProfilerPanel,        source: 'builtin' });
  registerPanel({ id: 'assets',        title: 'Assets',         component: AssetsPanel,          source: 'builtin' });
  registerPanel({ id: 'file-explorer', title: 'File Explorer',  component: FileExplorerWrapper,  source: 'builtin' });
  registerPanel({ id: 'terminal',      title: 'Terminal',       component: TerminalWrapper,      source: 'builtin' });
  registerPanel({ id: 'output',        title: 'Output',         component: OutputWrapper,        source: 'builtin' });
}
```

#### TitleBar spawn menu

`ALL_PANELS` in `TitleBar.svelte` is removed. The panels dropdown receives the reactive contribution list as a prop:

```svelte
<!-- TitleBar.svelte -->
{#each panelContributions as panel}
  {@const isOpen = activePanels.has(panel.id)}
  <button onclick={() => !isOpen && onAddPanel?.(panel.id)}>
    {panel.title}
  </button>
{/each}
```

`panelContributions` is passed down from App.svelte via `$derived` from `getPanelContributions()`.

#### App.svelte component lookup

The hardcoded `panelComponents` Record in `App.svelte` is removed entirely. `DockContainer.svelte` calls `getPanelComponent(panelId)` from the registry directly — it no longer receives a `panelComponents` prop.

**Migration of DockContainer:** The `panelComponents: Record<string, Component>` entry is removed from `DockContainer`'s `Props` interface. All call-sites are updated:
- The two top-level `<DockContainer>` usages in `App.svelte` (root and bottomPanel) drop the prop.
- The recursive `<DockContainer {panelComponents} ...>` call inside `DockContainer`'s own template is updated to `<DockContainer ...>` (prop no longer exists).

**Reactivity of `getPanelComponent` inside `DockContainer`:** `getPanelComponent()` reads from the registry's module-level `$state` array. Svelte 5 tracks `$state` reads that happen during template rendering — so calling it directly inside the `{#each}` block (e.g. `{@const Comp = getPanelComponent(panel)}`) is reactive and will re-render when the registry changes. No additional `$derived` wrapper is needed at the call site; the inline `{@const}` inside the reactive `{#each}` is the correct pattern.

### Rust module bridge (out of scope, contract documented)

Each Rust crate that contributes editor panels implements:

```rust
pub trait EditorContributor {
    fn panels(&self) -> Vec<PanelMeta>;
    fn inspector_fields(&self) -> Vec<InspectorFieldMeta>;
}

pub struct PanelMeta {
    pub id: &'static str,
    pub title: &'static str,
    pub icon: Option<&'static str>,
}
```

At startup, the editor calls a Tauri command `list_module_contributions()` which collects all registered `EditorContributor` implementations and returns their metadata. The frontend companion for each Rust module pre-registers the Svelte component under the matching ID. The Rust crate declares *that* it has a panel; the editor companion registers *what it looks like*.

This bridge is not implemented in this task. The `EditorContributor` trait is defined in Rust as a forward declaration; the Tauri command is stubbed. The registry's `source` field accommodates the module ID when it arrives.

---

## Affected Files

| File | Change |
|------|--------|
| `engine/editor/src/lib/components/TitleBar.svelte` | Remove View menu block; add AI server indicator button; remove `ALL_PANELS`; accept `panelContributions` prop |
| `engine/editor/src/lib/docking/store.ts` | Replace `initialSavedLayouts` with Scene/Code/Perf presets; rename `defaultLayout`/`tallLayout`/`wideLayout` to match |
| `engine/editor/src/lib/docking/types.ts` | Remove static `panelRegistry` array (replaced by contributions registry) |
| `engine/editor/src/lib/contributions/registry.ts` | **New** — ContributionRegistry store + API |
| `engine/editor/src/lib/contributions/builtins.ts` | **New** — registers all 9 builtin panels |
| `engine/editor/src/App.svelte` | Call `registerBuiltinPanels()`; remove `panelComponents` map; pass `panelContributions` to TitleBar; wire AI server indicator |
| `engine/editor/src/lib/docking/DockContainer.svelte` | Look up component via `getPanelComponent()` instead of prop map; remove `panelComponents` prop including recursive pass-through |
| `engine/editor/src/lib/docking/DockTabBar.svelte` | Replace `getPanelInfo` call with `getPanelTitle(id)` from registry |
| Rust (stub only) | Define `EditorContributor` trait + `PanelMeta`/`InspectorFieldMeta` structs |

---

## Persisted Layout Migration

`loadSavedLayouts()` returns whatever is in `localStorage` when non-empty, so users with the old `builtin-edit` / `builtin-assets` / `builtin-review` IDs would never see the new presets.

**Migration strategy:** A shared `migrateIfNeeded` function handles the upgrade. It detects old builtin IDs, replaces them with the new presets, and preserves any user-created layouts. This function is called in **both** `loadSavedLayouts()` (localStorage path) and `hydrateSavedLayouts()` (Tauri plugin-store path) so neither path bypasses migration.

```ts
const OLD_BUILTIN_IDS = new Set(['builtin-edit', 'builtin-assets', 'builtin-review']);

function migrateIfNeeded(layouts: SavedLayout[]): SavedLayout[] {
  const hasOldBuiltins = layouts.some(l => OLD_BUILTIN_IDS.has(l.id));
  if (!hasOldBuiltins) return layouts;
  // Preserve user-created layouts; replace old builtins with new presets
  const userLayouts = layouts.filter(l => !OLD_BUILTIN_IDS.has(l.id));
  return [...JSON.parse(JSON.stringify(initialSavedLayouts)), ...userLayouts];
}
```

`hydrateSavedLayouts` is updated to wrap its returned value through `migrateIfNeeded` before returning, and also re-persists the migrated result so the migration runs only once.

---

## Out of Scope

- Rust → frontend module bridge (`list_module_contributions` Tauri command implementation)
- Runtime `.silm` module loading and hot-registration
- Inspector field rendering (registry API is scaffolded, wiring deferred)
- i18n key migration for panel titles (title is a plain string for now; i18n pass is separate)
