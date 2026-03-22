# Editor Layout Presets & Dynamic Panel Registry — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace hardcoded panel lists and layout presets with a dynamic ContributionRegistry, swap the three builtin layouts to workflow-stage presets (Scene / Code / Perf), and remove the redundant View menu.

**Architecture:** A new `registry.ts` module holds a plain array of `PanelContribution` objects with a subscriber pattern (same as `console.ts`). Builtins self-register via `builtins.ts` before first render. The TitleBar spawn menu, DockContainer component lookup, and DockTabBar tab labels all read from the registry instead of hardcoded lists.

**Tech Stack:** Svelte 5 (runes in `.svelte` files, plain TS subscriber pattern in `.ts` modules), TypeScript, Vite. Rust stub only (no Tauri command implementation).

---

## File Map

| File | Action | Responsibility |
|------|--------|----------------|
| `engine/editor/src/lib/contributions/registry.ts` | **Create** | ContributionRegistry: plain array + subscriber API |
| `engine/editor/src/lib/contributions/builtins.ts` | **Create** | Registers all 9 builtin panels |
| `engine/editor/src/lib/docking/types.ts` | **Modify** | Remove `panelRegistry` array and `getPanelInfo()`; keep `getBasePanelId`, `createPanelInstance`, `getPanelInfo` usage in DockTabBar handled by registry |
| `engine/editor/src/lib/docking/store.ts` | **Modify** | Replace `defaultLayout`/`tallLayout`/`wideLayout` + `initialSavedLayouts`; add `migrateIfNeeded()` called in both load paths |
| `engine/editor/src/lib/docking/DockContainer.svelte` | **Modify** | Remove `panelComponents` prop; call `getPanelComponent()` from registry inline |
| `engine/editor/src/lib/docking/DockTabBar.svelte` | **Modify** | Replace `getPanelInfo` + `t(titleKey)` with `getPanelTitle()` from registry |
| `engine/editor/src/lib/components/TitleBar.svelte` | **Modify** | Remove `ALL_PANELS` + View menu block; accept `panelContributions` prop; add AI server indicator button |
| `engine/editor/src/App.svelte` | **Modify** | Call `registerBuiltinPanels()`; remove `panelComponents` map; pass `panelContributions` to TitleBar |
| `engine/editor/src-tauri/bridge/contributions.rs` | **Create** | Rust stub: `EditorContributor` trait + `PanelMeta`/`InspectorFieldMeta` |
| `engine/editor/src-tauri/bridge/mod.rs` | **Modify** | Declare `pub mod contributions;` |

---

## Task 1: Create ContributionRegistry

**Files:**
- Create: `engine/editor/src/lib/contributions/registry.ts`

The registry follows the same subscriber pattern as `engine/editor/src/lib/stores/console.ts`. No Svelte runes — plain arrays with listener callbacks so it works in `.ts` files.

- [ ] **Step 1: Create the file**

```ts
// engine/editor/src/lib/contributions/registry.ts
import type { Component } from 'svelte';
import { getBasePanelId } from '../docking/types';

export interface PanelContribution {
  id: string;
  title: string;
  icon?: string;
  component: Component;
  source: string; // 'builtin' | cargo crate name | module id
}

export interface InspectorFieldContribution {
  componentType: string;
  renderer: Component;
  source: string;
}

// ── State ──────────────────────────────────────────────────────────────────
let _panels: PanelContribution[] = [];
let _inspectorFields: InspectorFieldContribution[] = [];
let _panelListeners: (() => void)[] = [];

function notifyPanels() {
  _panelListeners.forEach((fn) => fn());
}

// ── Panel registration ─────────────────────────────────────────────────────
export function registerPanel(c: PanelContribution): void {
  // Replace if id already registered (idempotent re-registration)
  const idx = _panels.findIndex((p) => p.id === c.id);
  if (idx !== -1) {
    _panels[idx] = c;
  } else {
    _panels = [..._panels, c];
  }
  notifyPanels();
}

export function unregisterPanel(id: string): void {
  _panels = _panels.filter((p) => p.id !== id);
  notifyPanels();
}

export function getPanelContributions(): PanelContribution[] {
  return _panels;
}

/** Subscribe to panel list changes. Returns unsubscribe function. */
export function subscribePanelContributions(fn: () => void): () => void {
  _panelListeners.push(fn);
  return () => {
    _panelListeners = _panelListeners.filter((l) => l !== fn);
  };
}

/** Look up a component by panel ID (supports instance IDs like 'viewport:2'). */
export function getPanelComponent(id: string): Component | undefined {
  const base = getBasePanelId(id);
  return (_panels.find((p) => p.id === id) ?? _panels.find((p) => p.id === base))?.component;
}

/** Look up a panel's display title by ID (supports instance IDs). */
export function getPanelTitle(id: string): string {
  const base = getBasePanelId(id);
  return (_panels.find((p) => p.id === id) ?? _panels.find((p) => p.id === base))?.title ?? id;
}

// ── Inspector field registration (API scaffold — wiring deferred) ───────────
export function registerInspectorField(c: InspectorFieldContribution): void {
  const idx = _inspectorFields.findIndex(
    (f) => f.componentType === c.componentType && f.source === c.source,
  );
  if (idx !== -1) {
    _inspectorFields[idx] = c;
  } else {
    _inspectorFields = [..._inspectorFields, c];
  }
}

export function unregisterInspectorField(componentType: string, source: string): void {
  _inspectorFields = _inspectorFields.filter(
    (f) => !(f.componentType === componentType && f.source === source),
  );
}

export function getInspectorFieldContributions(): InspectorFieldContribution[] {
  return _inspectorFields;
}
```

- [ ] **Step 2: Verify type-check passes**

```bash
cd engine/editor && npm run typecheck
```

Expected: no errors related to the new file. (Ignore any pre-existing errors.)

- [ ] **Step 3: Commit**

```bash
git add engine/editor/src/lib/contributions/registry.ts
git commit -m "feat(editor): add ContributionRegistry for dynamic panel registration"
```

---

## Task 2: Register Builtin Panels + Wire Into App.svelte

**Files:**
- Create: `engine/editor/src/lib/contributions/builtins.ts`
- Modify: `engine/editor/src/App.svelte`

- [ ] **Step 1: Create `builtins.ts`**

```ts
// engine/editor/src/lib/contributions/builtins.ts
import { registerPanel } from './registry';
import HierarchyWrapper from '../docking/panels/HierarchyWrapper.svelte';
import ViewportPanel from '../docking/panels/ViewportPanel.svelte';
import InspectorWrapper from '../docking/panels/InspectorWrapper.svelte';
import ConsoleWrapper from '../docking/panels/ConsoleWrapper.svelte';
import ProfilerPanel from '../docking/panels/ProfilerPanel.svelte';
import AssetsPanel from '../docking/panels/AssetsPanel.svelte';
import FileExplorerWrapper from '../docking/panels/FileExplorerWrapper.svelte';
import TerminalWrapper from '../docking/panels/TerminalWrapper.svelte';
import OutputWrapper from '../docking/panels/OutputWrapper.svelte';

export function registerBuiltinPanels(): void {
  registerPanel({ id: 'hierarchy',     title: 'Hierarchy',     component: HierarchyWrapper as any,    source: 'builtin' });
  registerPanel({ id: 'viewport',      title: 'Viewport',      component: ViewportPanel as any,        source: 'builtin' });
  registerPanel({ id: 'inspector',     title: 'Inspector',     component: InspectorWrapper as any,     source: 'builtin' });
  registerPanel({ id: 'console',       title: 'Console',       component: ConsoleWrapper as any,       source: 'builtin' });
  registerPanel({ id: 'profiler',      title: 'Profiler',      component: ProfilerPanel as any,        source: 'builtin' });
  registerPanel({ id: 'assets',        title: 'Assets',        component: AssetsPanel as any,          source: 'builtin' });
  registerPanel({ id: 'file-explorer', title: 'File Explorer', component: FileExplorerWrapper as any,  source: 'builtin' });
  registerPanel({ id: 'terminal',      title: 'Terminal',      component: TerminalWrapper as any,      source: 'builtin' });
  registerPanel({ id: 'output',        title: 'Output',        component: OutputWrapper as any,        source: 'builtin' });
}
```

> Note: `as any` casts are needed because Svelte 5 component generics don't always match the `Component` interface exactly when imported into plain `.ts` files. This is safe — the cast just tells TS to trust the runtime type.

- [ ] **Step 2: In `App.svelte`, add imports and call `registerBuiltinPanels()`**

In the `<script>` imports section (around line 34 where panel components are currently imported), add:

```ts
import { registerBuiltinPanels } from './lib/contributions/builtins';
import { getPanelContributions, subscribePanelContributions } from './lib/contributions/registry';
import type { PanelContribution } from './lib/contributions/registry';
```

Then remove the 9 individual panel component imports (lines 35-43, the `import HierarchyWrapper...` through `import OutputWrapper...` block).

- [ ] **Step 3: In `App.svelte`, replace `panelComponents` with registry state**

Find and remove the `panelComponents` constant (lines 99-109):
```ts
// DELETE this entire block:
const panelComponents: Record<string, any> = {
  hierarchy: HierarchyWrapper,
  viewport: ViewportPanel,
  ...
};
```

Add in its place (near the top of the reactive state declarations, around line 55):
```ts
// Call before first render so DockContainer can resolve components immediately
registerBuiltinPanels();

let panelContributions = $state<PanelContribution[]>(getPanelContributions());
```

- [ ] **Step 4: In `App.svelte`, subscribe to registry changes**

`App.svelte` already imports `onDestroy` from `svelte` (line 2) — do NOT add a duplicate import.

Inside the `onMount` block (after the existing setup code), add the subscription:

```ts
const _unsubPanels = subscribePanelContributions(() => {
  panelContributions = getPanelContributions();
});
```

Then find the existing `onDestroy` call in `App.svelte` and add `_unsubPanels()` to it. It will look like:

```ts
onDestroy(() => {
  // ... existing cleanup (unlistenCatalog?.(), etc.) ...
  _unsubPanels();
});
```

If `App.svelte` uses multiple `onDestroy` calls rather than one, add a new `onDestroy(() => { _unsubPanels(); });` call — multiple `onDestroy` calls are valid in Svelte 5.

- [ ] **Step 5: Verify type-check**

```bash
cd engine/editor && npm run typecheck
```

Expected: errors about `{panelComponents}` prop still passed to `<DockContainer>` (these get fixed in Task 3). No other new errors.

- [ ] **Step 6: Commit**

```bash
git add engine/editor/src/lib/contributions/builtins.ts engine/editor/src/App.svelte
git commit -m "feat(editor): self-register builtin panels via ContributionRegistry"
```

---

## Task 3: Migrate DockContainer — Remove panelComponents Prop

**Files:**
- Modify: `engine/editor/src/lib/docking/DockContainer.svelte`
- Modify: `engine/editor/src/App.svelte`

- [ ] **Step 1: Update `DockContainer.svelte` Props interface**

Remove `panelComponents` from the `Props` interface (line 18) and from the destructuring (line 28):

```ts
// BEFORE
interface Props {
  node: LayoutNode;
  layout: EditorLayout;
  path?: number[];
  panelComponents: Record<string, Component>;
  onLayoutChange: (layout: EditorLayout) => void;
  isBottomPanel?: boolean;
}

let {
  node,
  layout,
  path = [],
  panelComponents,
  onLayoutChange,
  isBottomPanel = false,
}: Props = $props();
```

```ts
// AFTER
interface Props {
  node: LayoutNode;
  layout: EditorLayout;
  path?: number[];
  onLayoutChange: (layout: EditorLayout) => void;
  isBottomPanel?: boolean;
}

let {
  node,
  layout,
  path = [],
  onLayoutChange,
  isBottomPanel = false,
}: Props = $props();
```

- [ ] **Step 2: Add registry import and replace `resolveComponent`**

Add import at the top of the `<script>` block:
```ts
import { getPanelComponent } from '$lib/contributions/registry';
```

Remove the `resolveComponent` function (lines 135-138):
```ts
// DELETE:
function resolveComponent(id: string): Component | undefined {
  return panelComponents[id] ?? panelComponents[getBasePanelId(id)];
}
```

> **Important:** Do NOT remove the `import { getBasePanelId } from './types'` line (line 3). `getBasePanelId` is still used at line 129 in the `$effect` that manages viewport visibility: `if (getBasePanelId(panelId) === 'viewport')`. Deleting that import will silently break the viewport pause/resume logic.

- [ ] **Step 3: Update the template — remove prop pass-through and update component call**

In the split branch (around line 156), remove `{panelComponents}` from the recursive `<DockContainer>` call:

```svelte
<!-- BEFORE -->
<DockContainer
  node={child}
  {layout}
  path={[...path, i]}
  {panelComponents}
  {onLayoutChange}
  {isBottomPanel}
/>

<!-- AFTER -->
<DockContainer
  node={child}
  {layout}
  path={[...path, i]}
  {onLayoutChange}
  {isBottomPanel}
/>
```

In the tabs branch (around line 186), replace `resolveComponent(panelId)` with the registry call:

```svelte
<!-- BEFORE -->
{#each node.panels as panelId, i (panelId)}
  {@const Comp = resolveComponent(panelId)}

<!-- AFTER -->
{#each node.panels as panelId, i (panelId)}
  {@const Comp = getPanelComponent(panelId)}
```

- [ ] **Step 4: In `App.svelte`, remove `{panelComponents}` from both DockContainer usages**

Find the two `<DockContainer>` usages (around lines 706 and 718). Remove `{panelComponents}` from both:

```svelte
<!-- BEFORE -->
<DockContainer
  node={layout.root}
  {layout}
  {panelComponents}
  onLayoutChange={handleLayoutChange}
/>
<DockContainer
  node={layout.bottomPanel}
  {layout}
  {panelComponents}
  ...
/>

<!-- AFTER -->
<DockContainer
  node={layout.root}
  {layout}
  onLayoutChange={handleLayoutChange}
/>
<DockContainer
  node={layout.bottomPanel}
  {layout}
  ...
/>
```

- [ ] **Step 5: Verify type-check**

```bash
cd engine/editor && npm run typecheck
```

Expected: no type errors from DockContainer. `panelComponents` errors should be gone.

- [ ] **Step 6: Commit**

```bash
git add engine/editor/src/lib/docking/DockContainer.svelte engine/editor/src/App.svelte
git commit -m "refactor(editor): DockContainer resolves panels from registry, drop panelComponents prop"
```

---

## Task 4: Migrate DockTabBar — Replace getPanelInfo with Registry

**Files:**
- Modify: `engine/editor/src/lib/docking/DockTabBar.svelte`

`DockTabBar` calls `getPanelInfo` in three places:
1. Line 70: drag-out pop-out title resolution
2. Line 144: context-menu pop-out title resolution
3. Line 157: template tab label rendering

- [ ] **Step 1: Update imports in `DockTabBar.svelte`**

```ts
// BEFORE (line 3):
import { getPanelInfo, getBasePanelId, createPanelInstance } from './types';

// AFTER:
import { getBasePanelId, createPanelInstance } from './types';
import { getPanelTitle } from '$lib/contributions/registry';
```

- [ ] **Step 2: Replace `getPanelInfo` calls in the drag handler (line 70)**

```ts
// BEFORE:
const _info = getPanelInfo(panelId);
const _title = _info ? t(_info.titleKey) : panelId;
popOutPanel(panelId, _title, ev.screenX, ev.screenY);

// AFTER:
popOutPanel(panelId, getPanelTitle(panelId), ev.screenX, ev.screenY);
```

- [ ] **Step 3: Replace `getPanelInfo` call in context-menu pop-out handler (line 144)**

```ts
// BEFORE:
const _info = getPanelInfo(pid);
const _title = _info ? t(_info.titleKey) : pid;
popOutPanel(pid, _title, contextMenu.x + window.screenX, contextMenu.y + window.screenY);

// AFTER:
popOutPanel(pid, getPanelTitle(pid), contextMenu.x + window.screenX, contextMenu.y + window.screenY);
```

- [ ] **Step 4: Replace `getPanelInfo` in the template tab label (line 157-169)**

```svelte
<!-- BEFORE -->
{#each panels as panelId, i}
  {@const info = getPanelInfo(panelId)}
  <div class="dock-tab" ...>
    <span class="dock-tab-label">{info ? t(info.titleKey) : panelId}</span>

<!-- AFTER -->
{#each panels as panelId, i}
  <div class="dock-tab" ...>
    <span class="dock-tab-label">{getPanelTitle(panelId)}</span>
```

- [ ] **Step 5: Verify type-check**

```bash
cd engine/editor && npm run typecheck
```

Expected: no errors from DockTabBar. The `t` import in DockTabBar is still used by `dock.close_tab` and other keys — do NOT remove it.

- [ ] **Step 6: Commit**

```bash
git add engine/editor/src/lib/docking/DockTabBar.svelte
git commit -m "refactor(editor): DockTabBar gets panel titles from registry, drop getPanelInfo"
```

---

## Task 5: Clean Up types.ts

**Files:**
- Modify: `engine/editor/src/lib/docking/types.ts`

- [ ] **Step 1: Remove `panelRegistry` array and `getPanelInfo` function**

Open `types.ts`. Delete:
- The `panelRegistry: PanelInfo[]` export (lines 45-55)
- The `getPanelInfo` function (lines 64-66)
- The `PanelInfo` interface (lines 38-42) — only if no other file still imports it; check first

Check for remaining usages:
```bash
cd engine/editor && grep -r "PanelInfo\|panelRegistry\|getPanelInfo" src/
```

All three should show zero results (Tasks 3 and 4 removed the callers). If any remain, fix them before deleting.

- [ ] **Step 2: Verify `getBasePanelId` and `createPanelInstance` remain unchanged**

These two functions must stay — `DockTabBar` still imports `createPanelInstance`, and `registry.ts` imports `getBasePanelId`.

- [ ] **Step 3: Type-check**

```bash
cd engine/editor && npm run typecheck
```

Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add engine/editor/src/lib/docking/types.ts
git commit -m "refactor(editor): remove static panelRegistry and getPanelInfo from types.ts"
```

---

## Task 6: Replace Layout Presets + Add Migration

**Files:**
- Modify: `engine/editor/src/lib/docking/store.ts`

- [ ] **Step 1: Replace the three layout constant definitions**

Find `defaultLayout`, `tallLayout`, and `wideLayout` (lines 11-82). Replace entirely with:

```ts
// Scene — scene composition, viewport dominant
export const sceneLayout: EditorLayout = {
  root: {
    type: 'split',
    direction: 'horizontal',
    sizes: [20, 55, 25],
    children: [
      {
        type: 'split',
        direction: 'vertical',
        sizes: [65, 35],
        children: [
          { type: 'tabs', activeTab: 0, panels: ['hierarchy'] },
          { type: 'tabs', activeTab: 0, panels: ['assets', 'file-explorer'] },
        ],
      },
      { type: 'tabs', activeTab: 0, panels: ['viewport'] },
      { type: 'tabs', activeTab: 0, panels: ['inspector'] },
    ],
  },
  bottomPanel: { type: 'tabs', activeTab: 0, panels: ['console', 'output'] },
};

// Code — dev iteration, full tool strip at bottom
export const codeLayout: EditorLayout = {
  root: {
    type: 'split',
    direction: 'horizontal',
    sizes: [20, 50, 30],
    children: [
      {
        type: 'split',
        direction: 'vertical',
        sizes: [60, 40],
        children: [
          { type: 'tabs', activeTab: 0, panels: ['hierarchy'] },
          { type: 'tabs', activeTab: 0, panels: ['assets'] },
        ],
      },
      { type: 'tabs', activeTab: 0, panels: ['viewport'] },
      { type: 'tabs', activeTab: 0, panels: ['inspector'] },
    ],
  },
  bottomPanel: { type: 'tabs', activeTab: 0, panels: ['console', 'terminal', 'output'] },
};

// Perf — profiler in main area strip, empty bottom panel
export const perfLayout: EditorLayout = {
  root: {
    type: 'split',
    direction: 'vertical',
    sizes: [65, 35],
    children: [
      {
        type: 'split',
        direction: 'horizontal',
        sizes: [15, 55, 30],
        children: [
          { type: 'tabs', activeTab: 0, panels: ['hierarchy'] },
          { type: 'tabs', activeTab: 0, panels: ['viewport'] },
          { type: 'tabs', activeTab: 0, panels: ['inspector'] },
        ],
      },
      {
        type: 'split',
        direction: 'horizontal',
        sizes: [70, 30],
        children: [
          { type: 'tabs', activeTab: 0, panels: ['profiler'] },
          { type: 'tabs', activeTab: 0, panels: ['console', 'output'] },
        ],
      },
    ],
  },
  bottomPanel: { type: 'tabs', activeTab: 0, panels: [] },
};

// Default for new installs — same as Scene
export const defaultLayout = sceneLayout;
```

- [ ] **Step 2: Update `loadLayout()` fallback**

`loadLayout()` currently falls back to `cloneLayout(defaultLayout)`. This is already correct since `defaultLayout` now points to `sceneLayout`.

- [ ] **Step 3: Replace `initialSavedLayouts`**

Find the `initialSavedLayouts` constant (around line 466). Replace:

```ts
// BEFORE:
export const initialSavedLayouts: SavedLayout[] = [
  { id: 'builtin-edit',   name: 'Edit',   layout: defaultLayout, keybind: 'ctrl+1' },
  { id: 'builtin-assets', name: 'Assets', layout: tallLayout,    keybind: 'ctrl+2' },
  { id: 'builtin-review', name: 'Review', layout: wideLayout,    keybind: 'ctrl+3' },
];

// AFTER:
export const initialSavedLayouts: SavedLayout[] = [
  { id: 'builtin-scene', name: 'Scene', layout: sceneLayout, keybind: 'ctrl+1' },
  { id: 'builtin-code',  name: 'Code',  layout: codeLayout,  keybind: 'ctrl+2' },
  { id: 'builtin-perf',  name: 'Perf',  layout: perfLayout,  keybind: 'ctrl+3' },
];
```

- [ ] **Step 4: Add `migrateIfNeeded` and apply it in both load paths**

Add this function above `loadSavedLayouts`:

```ts
const OLD_BUILTIN_IDS = new Set(['builtin-edit', 'builtin-assets', 'builtin-review']);

function migrateIfNeeded(layouts: SavedLayout[]): SavedLayout[] {
  const hasOldBuiltins = layouts.some((l) => OLD_BUILTIN_IDS.has(l.id));
  if (!hasOldBuiltins) return layouts;
  const userLayouts = layouts.filter((l) => !OLD_BUILTIN_IDS.has(l.id));
  return [...cloneLayout_arr(initialSavedLayouts), ...userLayouts];
}

// Deep-clone an array of SavedLayouts (reuses the existing cloneLayout pattern)
function cloneLayout_arr(layouts: SavedLayout[]): SavedLayout[] {
  return JSON.parse(JSON.stringify(layouts));
}
```

Update `loadSavedLayouts()` to call it:

```ts
export function loadSavedLayouts(): SavedLayout[] {
  try {
    const stored = localStorage.getItem(SAVED_LAYOUTS_KEY);
    if (stored) {
      const parsed = JSON.parse(stored) as SavedLayout[];
      if (Array.isArray(parsed) && parsed.length > 0) {
        return migrateIfNeeded(parsed);  // ← add this
      }
    }
  } catch {
    // ignore parse errors
  }
  return cloneLayout_arr(initialSavedLayouts);
}
```

Update `hydrateSavedLayouts()` to call it and re-persist if migrated:

```ts
export async function hydrateSavedLayouts(): Promise<SavedLayout[] | null> {
  const stored = await persistLoad<SavedLayout[]>('savedLayouts', null as any);
  if (Array.isArray(stored) && stored.length > 0) {
    const migrated = migrateIfNeeded(stored);
    try { localStorage.setItem(SAVED_LAYOUTS_KEY, JSON.stringify(migrated)); } catch { /* ignore */ }
    // Re-persist if migration happened (old builtins replaced with new ones)
    if (migrated !== stored) {
      persistSave('savedLayouts', migrated);
    }
    return migrated;
  }
  return null;
}
```

- [ ] **Step 5: Check for hardcoded references to old layout IDs**

```bash
cd engine/editor && grep -r "builtin-edit\|builtin-assets\|builtin-review\|tallLayout\|wideLayout" src/
```

The TitleBar has hardcoded `onApplyLayout?.('builtin-edit')` etc. in the View menu — those get removed in Task 7 anyway. Any other hits must be updated now.

- [ ] **Step 6: Type-check**

```bash
cd engine/editor && npm run typecheck
```

- [ ] **Step 7: Commit**

```bash
git add engine/editor/src/lib/docking/store.ts
git commit -m "feat(editor): replace Edit/Assets/Review layouts with Scene/Code/Perf workflow presets"
```

---

## Task 7: Update TitleBar — Remove View Menu, Dynamic Spawn Menu, AI Server Button

**Files:**
- Modify: `engine/editor/src/lib/components/TitleBar.svelte`
- Modify: `engine/editor/src/App.svelte`

- [ ] **Step 1: Update TitleBar Props interface**

Add `panelContributions` prop, remove nothing else yet:

```ts
// In Props interface, add:
panelContributions?: PanelContribution[];

// In destructuring, add:
panelContributions = [],
```

Add the import at the top of `<script>`:
```ts
import type { PanelContribution } from '$lib/contributions/registry';
```

- [ ] **Step 2: Remove `ALL_PANELS` hardcoded array**

Delete lines 15-22:
```ts
// DELETE:
const ALL_PANELS: { id: string; label: string }[] = [
  { id: 'hierarchy', label: 'Hierarchy' },
  ...
];
```

- [ ] **Step 3: Update the panels dropdown template to use `panelContributions` prop**

Find the `{#each ALL_PANELS as panel}` block (around line 569). Replace:

```svelte
<!-- BEFORE -->
{#each ALL_PANELS as panel}
  {@const isOpen = activePanels.has(panel.id)}
  <button
    class="panel-item"
    class:is-open={isOpen}
    onclick={(e) => { e.stopPropagation(); if (!isOpen) { onAddPanel?.(panel.id); } showPanelsMenu = false; }}
    role="menuitem"
    title={isOpen ? `${panel.label} is already open` : `Add ${panel.label}`}
  >
    <span class="panel-indicator" aria-hidden="true">
      ...
    </span>
    <span>{panel.label}</span>
  </button>
{/each}

<!-- AFTER -->
{#each panelContributions as panel}
  {@const isOpen = activePanels.has(panel.id)}
  <button
    class="panel-item"
    class:is-open={isOpen}
    onclick={(e) => { e.stopPropagation(); if (!isOpen) { onAddPanel?.(panel.id); } showPanelsMenu = false; }}
    role="menuitem"
    title={isOpen ? `${panel.title} is already open` : `Add ${panel.title}`}
  >
    <span class="panel-indicator" aria-hidden="true">
      ...identical SVG icons...
    </span>
    <span>{panel.title}</span>
  </button>
{/each}
```

Note: keep the SVG icon markup inside `<span class="panel-indicator">` exactly as-is — only `panel.label` → `panel.title` changes.

- [ ] **Step 4: Remove the View menu block**

> **Before deleting:** `TitleBar.svelte` imports `aiServerRunning`, `startAiServer`, and `stopAiServer` at the top of `<script>`. Do NOT remove these imports — they are used by the new AI server indicator button added in Step 5.

Find the `<!-- View -->` comment and the entire `<DropdownMenu.Root>` block it introduces (lines 308-332). Delete it entirely:

```svelte
<!-- DELETE this entire block: -->
<!-- View -->
<DropdownMenu.Root>
  <DropdownMenu.Trigger class="menu-trigger" title={t('menu.view')}>
    ...
  </DropdownMenu.Trigger>
  <DropdownMenu.Content ...>
    <DropdownMenu.Sub>
      <DropdownMenu.SubTrigger>{t('menu.view.layout')}</DropdownMenu.SubTrigger>
      <DropdownMenu.SubContent>
        <DropdownMenu.Item onclick={() => onApplyLayout?.('builtin-edit')}>...</DropdownMenu.Item>
        <DropdownMenu.Item onclick={() => onApplyLayout?.('builtin-assets')}>...</DropdownMenu.Item>
        <DropdownMenu.Item onclick={() => onApplyLayout?.('builtin-review')}>...</DropdownMenu.Item>
      </DropdownMenu.SubContent>
    </DropdownMenu.Sub>
    <DropdownMenu.Separator />
    <DropdownMenu.Item onclick={() => onLayoutReset?.()}>...</DropdownMenu.Item>
    <DropdownMenu.Separator />
    <DropdownMenu.Item onclick={() => $aiServerRunning ? stopAiServer() : startAiServer('')}>
      ...
    </DropdownMenu.Item>
  </DropdownMenu.Content>
</DropdownMenu.Root>
```

- [ ] **Step 5: Add AI Server indicator button in the titlebar right area**

Find the `<!-- ── Panel management ── -->` comment (around line 549). Insert the AI server indicator button BEFORE it:

```svelte
<!-- ── AI Server indicator ── -->
<button
  class="icon-btn ai-server-btn"
  class:ai-running={$aiServerRunning}
  onclick={() => $aiServerRunning ? stopAiServer() : startAiServer('')}
  title={$aiServerRunning ? 'AI Server running — click to stop' : 'Start AI Server'}
  aria-label={$aiServerRunning ? 'Stop AI Server' : 'Start AI Server'}
>
  <!-- Lightning bolt icon -->
  <svg width="12" height="14" viewBox="0 0 12 14" fill="currentColor" aria-hidden="true">
    <path d="M7 1L1 8h5l-1 5 6-7H6l1-5z"/>
  </svg>
  <span class="ai-server-dot" aria-hidden="true"></span>
</button>
```

Add styles at the end of the `<style>` block:

```css
/* ── AI Server indicator ─────────────────────────────────────────────────── */
.ai-server-btn {
  position: relative;
}
.ai-server-dot {
  position: absolute;
  top: 3px;
  right: 3px;
  width: 5px;
  height: 5px;
  border-radius: 50%;
  background: var(--color-textDim, #666);
  transition: background 0.2s;
}
.ai-server-btn.ai-running .ai-server-dot {
  background: #4caf50;
  animation: ai-pulse 1.5s ease-in-out infinite;
}
@keyframes ai-pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}
```

- [ ] **Step 6: Pass `panelContributions` from App.svelte to TitleBar**

In `App.svelte`, find the `<TitleBar>` usage (around line 634). Add the prop:

```svelte
<TitleBar
  ...existing props...
  {panelContributions}
  ...
/>
```

The `panelContributions` variable is already declared as `$state` from Task 2.

- [ ] **Step 7: Type-check**

```bash
cd engine/editor && npm run typecheck
```

Expected: clean. If `$aiServerRunning` / `startAiServer` / `stopAiServer` imports were only used in the View menu and are now unused, keep them — they're still used by the new indicator button.

- [ ] **Step 8: Build check**

```bash
cd engine/editor && npm run build
```

Expected: successful build, no warnings about missing exports.

- [ ] **Step 9: Commit**

```bash
git add engine/editor/src/lib/components/TitleBar.svelte engine/editor/src/App.svelte
git commit -m "feat(editor): dynamic panel spawn menu; remove View menu; add AI server indicator button"
```

---

## Task 8: Rust EditorContributor Stub

**Files:**
- Create: `engine/editor/src-tauri/bridge/contributions.rs`
- Modify: `engine/editor/src-tauri/bridge/mod.rs`

This is a forward declaration only — no Tauri command is implemented, no handler registered. The structs and trait define the contract for future Rust module bridge work.

- [ ] **Step 1: Create `contributions.rs`**

```rust
//! Editor contribution points — forward declaration for the module bridge.
//!
//! Rust cargo modules implement [`EditorContributor`] to declare which panels
//! and inspector fields they contribute. The frontend companion for each module
//! registers the Svelte component under the matching panel ID.
//!
//! The Tauri command `list_module_contributions` (not yet implemented) will
//! collect all registered contributors and return their metadata to the frontend.

/// Metadata for a panel contributed by a Rust module.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PanelMeta {
    pub id: &'static str,
    pub title: &'static str,
    pub icon: Option<&'static str>,
}

/// Metadata for an inspector field renderer contributed by a Rust module.
#[derive(Debug, Clone, serde::Serialize)]
pub struct InspectorFieldMeta {
    /// ECS component type name this renderer handles, e.g. `"NetworkTransform"`.
    pub component_type: &'static str,
    pub source: &'static str,
}

/// Implement this trait on a unit struct in each Rust module that contributes
/// editor panels or inspector fields.
///
/// # Example
/// ```rust
/// use silmaril_editor_bridge::contributions::{EditorContributor, PanelMeta};
///
/// pub struct NetworkingContributions;
///
/// impl EditorContributor for NetworkingContributions {
///     fn panels(&self) -> Vec<PanelMeta> {
///         vec![PanelMeta { id: "networking-monitor", title: "Network Monitor", icon: None }]
///     }
///     fn inspector_fields(&self) -> Vec<InspectorFieldMeta> { vec![] }
/// }
/// ```
pub trait EditorContributor: Send + Sync {
    fn panels(&self) -> Vec<PanelMeta>;
    fn inspector_fields(&self) -> Vec<InspectorFieldMeta>;
}
```

- [ ] **Step 2: Declare module in `bridge/mod.rs`**

Open `engine/editor/src-tauri/bridge/mod.rs`. **Append** `pub mod contributions;` alongside the existing `pub mod` declarations — do not replace or delete any existing lines. The file already contains `pub mod registry;`, `pub mod registry_bridge;`, and others. Just add the new line:

```rust
// existing lines stay — add contributions alongside them:
pub mod contributions;
```

- [ ] **Step 3: Verify Rust compilation**

```bash
cd engine/editor/src-tauri && cargo check
```

Expected: compiles cleanly. The `serde::Serialize` derive requires `serde` — check `Cargo.toml` for the dependency (it's almost certainly already there given the existing bridge code).

- [ ] **Step 4: Commit**

```bash
git add engine/editor/src-tauri/bridge/contributions.rs engine/editor/src-tauri/bridge/mod.rs
git commit -m "feat(editor): add EditorContributor trait stub for future Rust module bridge"
```

---

## Task 9: Final Verification

- [ ] **Step 1: Full type-check**

```bash
cd engine/editor && npm run typecheck
```

Expected: zero errors.

- [ ] **Step 2: Full build**

```bash
cd engine/editor && npm run build
```

Expected: successful build.

- [ ] **Step 3: Visual smoke test**

Run the editor (`npm run dev` or via Tauri). Verify:
- [ ] The title bar has no "View" menu between Edit and Entity
- [ ] The panels button (grid icon) opens a menu listing all 9 panels (not 6)
- [ ] Panels already open show a checkmark; closed panels show a + and can be spawned
- [ ] The AI server indicator button (lightning bolt) appears; clicking starts/stops server with dot color change
- [ ] Ctrl+1 loads Scene layout (hierarchy left, assets/file-explorer below, viewport center, inspector right, console+output at bottom)
- [ ] Ctrl+2 loads Code layout (viewport smaller, full console+terminal+output at bottom)
- [ ] Ctrl+3 loads Perf layout (profiler strip in main area, no bottom panel)
- [ ] Tab labels still display correct panel names
- [ ] Drag-to-rearrange still works
- [ ] Pop-out (drag outside window or right-click → Pop out) still shows correct panel title

- [ ] **Step 4: Commit if any fixes were needed**

```bash
git add -p  # stage only the fixes
git commit -m "fix(editor): smoke test corrections"
```
