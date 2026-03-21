# Editor Wiring Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire four previously-stubbed editor areas: omnibar recent items, hierarchy CRUD, inspector add/remove components, and assets panel population.

**Architecture:** Two new in-memory stores (`recent-items.ts`, `assets.ts`) follow the existing `console.ts` pattern (sync reads + notify subscribers). Two new Rust Tauri commands (`add_component`, `remove_component`) stub the ECS bridge. UI changes are contained to individual panel components; all business logic imports from `scene/commands.ts` directly (no prop-drilling).

**Tech Stack:** Svelte 5, TypeScript, Vitest, Rust, Tauri 2, `tauri-plugin-store` (via `persist.ts`)

**Spec:** `docs/superpowers/specs/2026-03-21-editor-wiring-design.md`

**Pre-existing dependencies (do NOT create):**
- `persistLoad` / `persistSave` → `engine/editor/src/lib/stores/persist.ts`
- `getSchemas` → `engine/editor/src/lib/inspector/schema-store.ts`
- `applyComponentDefaults` → `engine/editor/src/lib/inspector/inspector-utils.ts`
- `_mutate` → `engine/editor/src/lib/scene/state.ts`
- `createEntity`, `deleteEntity`, `duplicateEntity`, `renameEntity`, `selectEntity` → `engine/editor/src/lib/scene/commands.ts`
- `tauriInvoke` helper (private to api.ts) — use `openProject`, `scanProjectEntities` exports from `api.ts`

---

## File Map

**New files:**
```
engine/editor/src/lib/stores/recent-items.ts
engine/editor/src/lib/stores/recent-items.test.ts
engine/editor/src/lib/stores/assets.ts
engine/editor/src/lib/stores/assets.test.ts
```

**Modified files:**
```
engine/editor/src/lib/scene/commands.ts          ← addComponent, removeComponent, dispatch cases
engine/editor/src/lib/scene/commands.test.ts     ← new tests for above (create if absent)
engine/editor/src/lib/api.ts                     ← addComponent, removeComponent, scanAssets
engine/editor/src-tauri/bridge/commands.rs       ← add_component, remove_component Tauri commands
engine/editor/src-tauri/lib.rs                   ← register new commands in invoke_handler!
engine/editor/src/App.svelte                     ← hydrate recents, scan assets, thread props
engine/editor/src/lib/components/TitleBar.svelte ← recentItems prop thread
engine/editor/src/lib/omnibar/Omnibar.svelte     ← recentItems prop, case 'recent' handler, assets store
engine/editor/src/lib/components/HierarchyPanel.svelte   ← full CRUD
engine/editor/src/lib/components/InspectorPanel.svelte   ← remove/add component UI
engine/editor/src/lib/docking/panels/AssetsPanel.svelte  ← replace stub with wired panel
```

---

## Task 1: `stores/recent-items.ts`

**Files:**
- Create: `engine/editor/src/lib/stores/recent-items.ts`
- Create: `engine/editor/src/lib/stores/recent-items.test.ts`

- [ ] **Step 1: Write failing tests**

```typescript
// engine/editor/src/lib/stores/recent-items.test.ts
import { describe, it, expect, beforeEach } from 'vitest';
import {
  addRecentItem,
  getRecentItems,
  subscribeRecent,
  _resetRecentItems,
} from './recent-items';

beforeEach(() => _resetRecentItems());

describe('addRecentItem', () => {
  it('adds an item', () => {
    addRecentItem({ label: 'My Game', path: '/path/my-game', itemType: 'project' });
    expect(getRecentItems()).toHaveLength(1);
    expect(getRecentItems()[0].label).toBe('My Game');
  });

  it('deduplicates by path — re-opening moves item to front', () => {
    addRecentItem({ label: 'A', path: '/a', itemType: 'project' });
    addRecentItem({ label: 'B', path: '/b', itemType: 'project' });
    addRecentItem({ label: 'A2', path: '/a', itemType: 'project' });
    const items = getRecentItems();
    expect(items).toHaveLength(2);
    expect(items[0].path).toBe('/a');
    expect(items[0].label).toBe('A2'); // label updated
  });

  it('caps at 10 items, dropping oldest', () => {
    for (let i = 0; i < 12; i++) {
      addRecentItem({ label: `P${i}`, path: `/p${i}`, itemType: 'project' });
    }
    expect(getRecentItems()).toHaveLength(10);
    // newest items survive
    expect(getRecentItems()[0].path).toBe('/p11');
  });
});

describe('subscribeRecent', () => {
  it('notifies subscriber when item added', () => {
    let called = 0;
    const unsub = subscribeRecent(() => { called++; });
    addRecentItem({ label: 'X', path: '/x', itemType: 'project' });
    expect(called).toBe(1);
    unsub();
    addRecentItem({ label: 'Y', path: '/y', itemType: 'project' });
    expect(called).toBe(1); // unsubscribed
  });
});
```

- [ ] **Step 2: Run tests — expect FAIL**

```bash
cd engine/editor && npx vitest run src/lib/stores/recent-items.test.ts
```
Expected: `Cannot find module './recent-items'`

- [ ] **Step 3: Create `recent-items.ts`**

```typescript
// engine/editor/src/lib/stores/recent-items.ts
import { persistLoad, persistSave } from './persist';

export interface RecentItem {
  label: string;
  path: string;
  itemType: 'project' | 'scene';
  openedAt: number;
}

const MAX_RECENT = 10;
const PERSIST_KEY = 'recent-items';

let _items: RecentItem[] = [];
let _listeners: ((items: RecentItem[]) => void)[] = [];

function _notify() {
  _listeners.forEach((fn) => fn([..._items]));
}

/** Call once in onMount to load persisted items into memory. */
export async function hydrateRecentItems(): Promise<void> {
  _items = await persistLoad<RecentItem[]>(PERSIST_KEY, []);
  _notify();
}

/** Add or update a recent item. Moves to front if path already exists. */
export function addRecentItem(item: Omit<RecentItem, 'openedAt'>): void {
  const entry: RecentItem = { ...item, openedAt: Date.now() };
  _items = _items.filter((i) => i.path !== item.path);
  _items.unshift(entry);
  if (_items.length > MAX_RECENT) _items = _items.slice(0, MAX_RECENT);
  _notify();
  persistSave(PERSIST_KEY, _items); // fire-and-forget
}

export function getRecentItems(): RecentItem[] {
  return [..._items];
}

export function subscribeRecent(fn: (items: RecentItem[]) => void): () => void {
  _listeners.push(fn);
  return () => {
    _listeners = _listeners.filter((l) => l !== fn);
  };
}

/** Test-only: reset all state. */
export function _resetRecentItems(): void {
  _items = [];
  _listeners = [];
}
```

- [ ] **Step 4: Run tests — expect PASS**

```bash
cd engine/editor && npx vitest run src/lib/stores/recent-items.test.ts
```
Expected: 4 tests pass.

- [ ] **Step 5: Commit**

```bash
cd D:/dev/maethril/silmaril
git add engine/editor/src/lib/stores/recent-items.ts engine/editor/src/lib/stores/recent-items.test.ts
git commit -m "feat(editor): add recent-items persist store with TDD"
```

---

## Task 2: `stores/assets.ts`

**Files:**
- Create: `engine/editor/src/lib/stores/assets.ts`
- Create: `engine/editor/src/lib/stores/assets.test.ts`

- [ ] **Step 1: Write failing tests**

```typescript
// engine/editor/src/lib/stores/assets.test.ts
import { describe, it, expect, beforeEach } from 'vitest';
import { setAssets, getAssets, clearAssets, subscribeAssets } from './assets';

beforeEach(() => clearAssets());

describe('setAssets / getAssets', () => {
  it('stores and retrieves assets', () => {
    setAssets([{ path: '/a/player.png', assetType: 'texture', filename: 'player.png' }]);
    expect(getAssets()).toHaveLength(1);
    expect(getAssets()[0].filename).toBe('player.png');
  });

  it('replaces previous list on setAssets', () => {
    setAssets([{ path: '/a.png', assetType: 'texture', filename: 'a.png' }]);
    setAssets([{ path: '/b.glb', assetType: 'mesh', filename: 'b.glb' },
               { path: '/c.glb', assetType: 'mesh', filename: 'c.glb' }]);
    expect(getAssets()).toHaveLength(2);
  });
});

describe('clearAssets', () => {
  it('empties the list', () => {
    setAssets([{ path: '/x.wav', assetType: 'audio', filename: 'x.wav' }]);
    clearAssets();
    expect(getAssets()).toHaveLength(0);
  });
});

describe('subscribeAssets', () => {
  it('notifies on setAssets and clearAssets', () => {
    let count = 0;
    const unsub = subscribeAssets(() => { count++; });
    setAssets([]);
    clearAssets();
    expect(count).toBe(2);
    unsub();
    setAssets([]);
    expect(count).toBe(2); // no more notifications
  });
});
```

- [ ] **Step 2: Run tests — expect FAIL**

```bash
cd engine/editor && npx vitest run src/lib/stores/assets.test.ts
```

- [ ] **Step 3: Create `assets.ts`**

```typescript
// engine/editor/src/lib/stores/assets.ts

export interface AssetEntry {
  path: string;
  assetType: 'texture' | 'mesh' | 'audio' | 'config' | 'unknown';
  filename: string;
}

let _assets: AssetEntry[] = [];
let _listeners: ((assets: AssetEntry[]) => void)[] = [];

function _notify() {
  _listeners.forEach((fn) => fn([..._assets]));
}

export function setAssets(list: AssetEntry[]): void {
  _assets = list;
  _notify();
}

export function getAssets(): AssetEntry[] {
  return [..._assets];
}

export function clearAssets(): void {
  _assets = [];
  _notify();
}

export function subscribeAssets(fn: (assets: AssetEntry[]) => void): () => void {
  _listeners.push(fn);
  return () => {
    _listeners = _listeners.filter((l) => l !== fn);
  };
}
```

- [ ] **Step 4: Run tests — expect PASS**

```bash
cd engine/editor && npx vitest run src/lib/stores/assets.test.ts
```
Expected: 4 tests pass.

- [ ] **Step 5: Commit**

```bash
cd D:/dev/maethril/silmaril
git add engine/editor/src/lib/stores/assets.ts engine/editor/src/lib/stores/assets.test.ts
git commit -m "feat(editor): add assets in-memory store with TDD"
```

---

## Task 3: `addComponent` / `removeComponent` scene commands

**Files:**
- Modify: `engine/editor/src/lib/scene/commands.ts`
- Create: `engine/editor/src/lib/scene/commands.test.ts` (if it doesn't exist)

- [ ] **Step 1: Check if test file exists**

```bash
ls engine/editor/src/lib/scene/commands.test.ts 2>/dev/null || echo "MISSING"
```

- [ ] **Step 2: Write failing tests**

Create `engine/editor/src/lib/scene/commands.test.ts` (or append to it):

```typescript
// engine/editor/src/lib/scene/commands.test.ts
import { describe, it, expect, beforeEach } from 'vitest';
import { createEntity, addComponent, removeComponent, newScene } from './commands';

// Reset scene state before each test
beforeEach(() => { newScene(); });

describe('addComponent', () => {
  it('adds a component to an existing entity', () => {
    const e = createEntity('Tester');
    // Entity starts with ['Transform']
    addComponent(e.id, 'Health');
    // Import getEntityById to verify
    // Use dynamic import of state to peek
    const { getEntityById } = require('./state');
    // Actually re-import commands after mutation:
    const updated = getEntityById(e.id);
    expect(updated?.components).toContain('Health');
  });

  it('is idempotent — adding same component twice does not duplicate', () => {
    const e = createEntity('Tester');
    addComponent(e.id, 'Health');
    addComponent(e.id, 'Health');
    const { getEntityById } = require('./state');
    const updated = getEntityById(e.id);
    const count = updated?.components.filter((c: string) => c === 'Health').length ?? 0;
    expect(count).toBe(1);
  });
});

describe('removeComponent', () => {
  it('removes a component from an entity', () => {
    const e = createEntity('Tester');
    addComponent(e.id, 'Health');
    removeComponent(e.id, 'Health');
    const { getEntityById } = require('./state');
    const updated = getEntityById(e.id);
    expect(updated?.components).not.toContain('Health');
  });

  it('is safe to call for a component not on the entity', () => {
    const e = createEntity('Tester');
    expect(() => removeComponent(e.id, 'NonExistent')).not.toThrow();
  });
});
```

- [ ] **Step 3: Run tests — expect FAIL**

```bash
cd engine/editor && npx vitest run src/lib/scene/commands.test.ts
```
Expected: `addComponent is not a function` or similar.

- [ ] **Step 4: Add `addComponent` and `removeComponent` to `commands.ts`**

At the top of `commands.ts`, verify these imports already exist (they do):
```typescript
import { getSchemas } from '$lib/inspector/schema-store';
import { applyComponentDefaults } from '$lib/inspector/inspector-utils';
```

Append after the `setComponentField` function (around line 415):

```typescript
/** Add a component to an entity. No-op if already present. */
export function addComponent(entityId: number, componentName: string): void {
  const schema = getSchemas()[componentName];
  const defaults = schema ? applyComponentDefaults(schema) : {};
  _mutate((s) => {
    const entities = s.entities.map((e) => {
      if (e.id !== entityId) return e;
      if (e.components.includes(componentName)) return e;
      return {
        ...e,
        components: [...e.components, componentName],
        componentValues: { ...e.componentValues, [componentName]: defaults },
      };
    });
    return { ...s, entities };
  });
  logInfo(`Component added: ${componentName} → entity #${entityId}`);
  // Tauri forward — fire-and-forget, ignore errors
  if (typeof window !== 'undefined' && (window as any).__TAURI_INTERNALS__) {
    import('@tauri-apps/api/core').then(({ invoke }) =>
      invoke('add_component', { entityId, component: componentName }).catch(() => {}),
    );
  }
}

/** Remove a component from an entity. No-op if not present. */
export function removeComponent(entityId: number, componentName: string): void {
  _mutate((s) => {
    const entities = s.entities.map((e) => {
      if (e.id !== entityId) return e;
      const { [componentName]: _removed, ...rest } = e.componentValues ?? {};
      return {
        ...e,
        components: e.components.filter((c) => c !== componentName),
        componentValues: rest,
      };
    });
    return { ...s, entities };
  });
  logInfo(`Component removed: ${componentName} from entity #${entityId}`);
  if (typeof window !== 'undefined' && (window as any).__TAURI_INTERNALS__) {
    import('@tauri-apps/api/core').then(({ invoke }) =>
      invoke('remove_component', { entityId, component: componentName }).catch(() => {}),
    );
  }
}
```

Also add to `dispatchSceneCommand` switch (before the `default:` case):

```typescript
    case 'add_component':
      addComponent(args.id as number, args.component as string);
      return { ok: true };

    case 'remove_component':
      removeComponent(args.id as number, args.component as string);
      return { ok: true };
```

- [ ] **Step 5: Run tests — expect PASS**

```bash
cd engine/editor && npx vitest run src/lib/scene/commands.test.ts
```

- [ ] **Step 6: Commit**

```bash
cd D:/dev/maethril/silmaril
git add engine/editor/src/lib/scene/commands.ts engine/editor/src/lib/scene/commands.test.ts
git commit -m "feat(editor): add addComponent/removeComponent scene commands with TDD"
```

---

## Task 4: Tauri commands + api.ts

**Files:**
- Modify: `engine/editor/src-tauri/bridge/commands.rs`
- Modify: `engine/editor/src-tauri/lib.rs`
- Modify: `engine/editor/src/lib/api.ts`

- [ ] **Step 1: Append to `bridge/commands.rs`**

At the very end of `engine/editor/src-tauri/bridge/commands.rs`, append:

```rust
// ── Component mutation stubs (ECS not live yet) ───────────────────────────

#[tauri::command]
pub fn add_component(entity_id: u64, component: String) -> Result<(), String> {
    if component.is_empty() {
        return Err("component name required".into());
    }
    Ok(())
}

#[tauri::command]
pub fn remove_component(entity_id: u64, component: String) -> Result<(), String> {
    if component.is_empty() {
        return Err("component name required".into());
    }
    Ok(())
}
```

- [ ] **Step 2: Register in `lib.rs`**

In `engine/editor/src-tauri/lib.rs`, find the `invoke_handler!(tauri::generate_handler![` block and add before its closing `])`:

```rust
            commands::add_component,
            commands::remove_component,
```

- [ ] **Step 3: Cargo check**

```bash
cd engine/editor && cargo check -p silmaril-editor
```
Expected: no errors.

- [ ] **Step 4: Add to `api.ts`**

In `engine/editor/src/lib/api.ts`, append after the last export:

```typescript
export async function addComponent(entityId: number, component: string): Promise<void> {
  if (!isTauri) return; // browser: scene state already updated client-side
  return tauriInvoke('add_component', { entityId, component });
}

export async function removeComponent(entityId: number, component: string): Promise<void> {
  if (!isTauri) return;
  return tauriInvoke('remove_component', { entityId, component });
}

export async function scanAssets(
  projectPath: string,
): Promise<{ path: string; asset_type: string }[]> {
  if (!isTauri) return [];
  return tauriInvoke('scan_assets', { projectPath });
}
```

- [ ] **Step 5: TypeScript check**

```bash
cd engine/editor && npx tsc --noEmit 2>&1 | grep -i "api\|commands" | head -10
```
Expected: no errors.

- [ ] **Step 6: Commit**

```bash
cd D:/dev/maethril/silmaril
git add engine/editor/src-tauri/bridge/commands.rs engine/editor/src-tauri/lib.rs engine/editor/src/lib/api.ts
git commit -m "feat(editor): add add_component/remove_component Tauri stubs and scanAssets api wrapper"
```

---

## Task 5: Wire recent items end-to-end

**Files:**
- Modify: `engine/editor/src/App.svelte`
- Modify: `engine/editor/src/lib/components/TitleBar.svelte`
- Modify: `engine/editor/src/lib/omnibar/Omnibar.svelte`

- [ ] **Step 1: Update `App.svelte`**

**a) Add imports** near the top of the `<script>` (after existing store imports):
```typescript
import { hydrateRecentItems, addRecentItem, subscribeRecent, type RecentItem } from './lib/stores/recent-items';
```

**b) Add reactive state** near the other `$state` declarations:
```typescript
let recentItems = $state<RecentItem[]>([]);
```

**c) In `onMount`**, after the settings hydration line, add:
```typescript
    await hydrateRecentItems();
    recentItems = (await import('./lib/stores/recent-items')).getRecentItems();
    subscribeRecent((items) => { recentItems = items; });
```

**d) After `openProject()` succeeds** (in `handleOpenProject`, after `editorState = state`), add:
```typescript
    addRecentItem({ label: state.project_name ?? path, path, itemType: 'project' });
```

**e) In the template**, update the `<TitleBar>` usage to add:
```svelte
  {recentItems}
```
alongside the existing omnibar props.

- [ ] **Step 2: Update `TitleBar.svelte`**

Read the current `interface Props` block. Add to it:
```typescript
  recentItems?: RecentItem[];
```

Add to `$props()` destructure:
```typescript
  recentItems = [],
```

Add import at top of script:
```typescript
  import type { RecentItem } from '$lib/stores/recent-items';
```

In the template where `<Omnibar>` is rendered, add:
```svelte
    {recentItems}
```

- [ ] **Step 3: Update `Omnibar.svelte`**

**a) Add import** at top of script:
```typescript
  import { openProject, scanProjectEntities } from '$lib/api';
  import { setEntities, setSelectedEntityId } from '$lib/stores/editor-context';
  import type { RecentItem } from '$lib/stores/recent-items';
```

**b) Add prop** to `interface Props`:
```typescript
    recentItems?: RecentItem[];
```

And to destructure:
```typescript
    recentItems = [],
```

**c) Replace the `buildResults` call** in the `$effect`:
```typescript
    results = buildResults(query, merged, entities, assets, recentItems);
```

**d) Implement `case 'recent'`** in the `execute` function:
```typescript
      case 'recent': {
        const state = await openProject(result.path);
        const scannedEntities = await scanProjectEntities(result.path);
        setEntities(scannedEntities);
        setSelectedEntityId(null);
        break;
      }
```

- [ ] **Step 4: TypeScript check**

```bash
cd engine/editor && npx tsc --noEmit 2>&1 | grep -i "recent\|omnibar\|titlebar\|App" | head -20
```
Expected: no errors.

- [ ] **Step 5: Commit**

```bash
cd D:/dev/maethril/silmaril
git add engine/editor/src/App.svelte engine/editor/src/lib/components/TitleBar.svelte engine/editor/src/lib/omnibar/Omnibar.svelte
git commit -m "feat(omnibar): wire recent items — persist store, prop chain, case recent handler"
```

---

## Task 6: Wire assets end-to-end

**Files:**
- Modify: `engine/editor/src/App.svelte`
- Modify: `engine/editor/src/lib/omnibar/Omnibar.svelte`
- Modify: `engine/editor/src/lib/docking/panels/AssetsPanel.svelte`

- [ ] **Step 1: Update `App.svelte` — populate assets on project open**

**a) Add import**:
```typescript
import { setAssets, clearAssets, type AssetEntry } from './lib/stores/assets';
import { scanAssets } from './lib/api';
```

**b) In `handleOpenProject`**, after `setEntities(entities)`, add:
```typescript
    clearAssets();
    try {
      const raw = await scanAssets(path);
      setAssets(raw.map((a) => ({
        path: a.path,
        assetType: a.asset_type as AssetEntry['assetType'],
        filename: a.path.split(/[\\/]/).pop() ?? a.path,
      })));
    } catch {
      // non-fatal — assets panel will show empty
    }
```

- [ ] **Step 2: Update `Omnibar.svelte` — read from assets store instead of IPC**

**a) Add import**:
```typescript
  import { getAssets, subscribeAssets } from '$lib/stores/assets';
```

**b) Replace** the existing `let assets` state and lazy-fetch logic. Currently it has:
```typescript
  let assets: { path: string; assetType: string }[] = $state([]);
  let assetsFetched = false;
```
And the `$effect` has an IPC call when `query.startsWith('#')`.

Replace that state and the IPC block with:
```typescript
  // Assets come from the store (populated by App.svelte on project open).
  // Fall back to IPC scan only when the store is empty (no project open yet).
  let assets: { path: string; assetType: string }[] = $state(getAssets());

  onMount(() => {
    // Already have onMount for list_commands — add subscribe here.
    const unsubAssets = subscribeAssets((list) => { assets = list; });
    return unsubAssets; // onMount cleanup
  });
```

Remove the `assetsFetched` variable and the `if (query.startsWith('#') && !assetsFetched ...)` IPC block from the `$effect`.

**Note:** `onMount` in Svelte 5 accepts a cleanup function as its return value. Since there's already an `onMount`, merge the subscribe call into the existing `onMount`:

Find the existing `onMount`:
```typescript
  onMount(async () => {
    try {
      rustCommands = await invoke<EditorCommand[]>('list_commands');
    } catch {
      rustCommands = [];
    }
  });
```

Replace with:
```typescript
  onMount(async () => {
    try {
      rustCommands = await invoke<EditorCommand[]>('list_commands');
    } catch {
      rustCommands = [];
    }
    assets = getAssets();
    const unsubAssets = subscribeAssets((list) => { assets = list; });
    return unsubAssets;
  });
```

And remove `let assetsFetched = false;` and the lazy-IPC block from the `$effect`.

- [ ] **Step 3: Replace `AssetsPanel.svelte` stub**

Replace the entire file:

```svelte
<!-- engine/editor/src/lib/docking/panels/AssetsPanel.svelte -->
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { getAssets, subscribeAssets, type AssetEntry } from '$lib/stores/assets';
  import { fuzzyScore } from '$lib/omnibar/fuzzy';

  type Group = 'texture' | 'mesh' | 'audio' | 'config' | 'unknown';
  const GROUP_LABELS: Record<Group, string> = {
    texture: 'Textures',
    mesh: 'Meshes',
    audio: 'Audio',
    config: 'Config',
    unknown: 'Other',
  };
  const GROUP_ORDER: Group[] = ['texture', 'mesh', 'audio', 'config', 'unknown'];

  let assets = $state<AssetEntry[]>(getAssets());
  let filter = $state('');
  let collapsed = $state<Set<Group>>(new Set());
  let toast = $state('');
  let toastTimer: ReturnType<typeof setTimeout> | null = null;

  let unsub: (() => void) | null = null;
  onMount(() => {
    unsub = subscribeAssets((list) => { assets = list; });
  });
  onDestroy(() => unsub?.());

  let filtered = $derived(
    filter
      ? assets.filter((a) => fuzzyScore(a.filename, filter) >= 0)
      : assets,
  );

  function grouped(type: Group): AssetEntry[] {
    return filtered.filter((a) => a.assetType === type);
  }

  function toggleGroup(g: Group) {
    const next = new Set(collapsed);
    if (next.has(g)) next.delete(g); else next.add(g);
    collapsed = next;
  }

  async function copyPath(path: string) {
    await navigator.clipboard.writeText(path);
    if (toastTimer) clearTimeout(toastTimer);
    toast = 'Path copied';
    toastTimer = setTimeout(() => { toast = ''; }, 1500);
  }
</script>

<div class="assets-panel">
  {#if assets.length === 0}
    <p class="assets-empty">Open a project to browse assets</p>
  {:else}
    <div class="assets-header">
      <input
        class="assets-filter"
        type="text"
        placeholder="Filter assets…"
        bind:value={filter}
      />
    </div>

    <div class="assets-list">
      {#each GROUP_ORDER as group}
        {@const items = grouped(group)}
        {#if items.length > 0}
          <div class="group">
            <button class="group-header" onclick={() => toggleGroup(group)}>
              <span class="group-chevron">{collapsed.has(group) ? '▸' : '▾'}</span>
              <span class="group-label">{GROUP_LABELS[group]}</span>
              <span class="group-count">{items.length}</span>
            </button>
            {#if !collapsed.has(group)}
              {#each items as asset (asset.path)}
                <button
                  class="asset-row"
                  onclick={() => copyPath(asset.path)}
                  title={asset.path}
                >
                  {asset.filename}
                </button>
              {/each}
            {/if}
          </div>
        {/if}
      {/each}
    </div>
  {/if}

  {#if toast}
    <div class="toast">{toast}</div>
  {/if}
</div>

<style>
  .assets-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
    position: relative;
    background: var(--color-bgPanel, #252525);
  }

  .assets-empty {
    color: var(--color-textDim, #666);
    font-style: italic;
    padding: 12px 8px;
    font-size: 12px;
    text-align: center;
  }

  .assets-header {
    padding: 4px 8px;
    border-bottom: 1px solid var(--color-border, #404040);
    flex-shrink: 0;
  }

  .assets-filter {
    width: 100%;
    box-sizing: border-box;
    background: var(--color-bg, #1e1e1e);
    border: 1px solid var(--color-border, #404040);
    border-radius: 3px;
    padding: 3px 6px;
    font-size: 11px;
    color: var(--color-text, #ccc);
    outline: none;
  }

  .assets-filter:focus { border-color: var(--color-accent, #007acc); }

  .assets-list {
    flex: 1;
    overflow-y: auto;
    padding: 2px 0;
  }

  .group-header {
    all: unset;
    display: flex;
    align-items: center;
    gap: 4px;
    width: 100%;
    padding: 3px 8px;
    font-size: 10px;
    color: var(--color-textDim, #666);
    letter-spacing: 0.05em;
    cursor: pointer;
    box-sizing: border-box;
  }

  .group-header:hover { color: var(--color-text, #ccc); }

  .group-chevron { font-size: 9px; width: 10px; }
  .group-label { flex: 1; text-transform: uppercase; }
  .group-count {
    background: var(--color-bg, #1e1e1e);
    padding: 0 4px;
    border-radius: 3px;
    font-size: 10px;
  }

  .asset-row {
    all: unset;
    display: block;
    width: 100%;
    padding: 2px 8px 2px 22px;
    font-size: 11px;
    color: var(--color-textMuted, #999);
    cursor: pointer;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    box-sizing: border-box;
  }

  .asset-row:hover {
    background: var(--color-bgHeader, #2d2d2d);
    color: var(--color-text, #ccc);
  }

  .toast {
    position: absolute;
    bottom: 8px;
    left: 50%;
    transform: translateX(-50%);
    background: var(--color-accent, #007acc);
    color: #fff;
    font-size: 11px;
    padding: 4px 10px;
    border-radius: 4px;
    pointer-events: none;
    white-space: nowrap;
  }
</style>
```

- [ ] **Step 4: TypeScript check**

```bash
cd engine/editor && npx tsc --noEmit 2>&1 | grep -i "assets\|Assets" | head -10
```

- [ ] **Step 5: Commit**

```bash
cd D:/dev/maethril/silmaril
git add engine/editor/src/App.svelte engine/editor/src/lib/omnibar/Omnibar.svelte engine/editor/src/lib/docking/panels/AssetsPanel.svelte
git commit -m "feat(editor): wire assets panel — scan on project open, grouped list, filter, copy-path toast"
```

---

## Task 7: Hierarchy Panel CRUD

**Files:**
- Modify: `engine/editor/src/lib/components/HierarchyPanel.svelte`

This is a significant UI addition to the existing panel. Read the file first, then apply the changes.

- [ ] **Step 1: Replace `HierarchyPanel.svelte` with the CRUD version**

The panel already has: entity list, filter input, selection. We're adding: `+` header button, hover actions (duplicate/add-child/delete), inline rename, right-click context menu, Del key.

Replace the entire file with:

```svelte
<!-- engine/editor/src/lib/components/HierarchyPanel.svelte -->
<script lang="ts">
  import { t } from '$lib/i18n';
  import type { EntityInfo } from '$lib/api';
  import {
    createEntity,
    deleteEntity,
    duplicateEntity,
    renameEntity,
    selectEntity,
  } from '$lib/scene/commands';

  let { entities = [], selectedId = null, onSelect }: {
    entities: EntityInfo[];
    selectedId: number | null;
    onSelect: (id: number) => void;
  } = $props();

  let filter = $state('');
  let hoveredId = $state<number | null>(null);
  let renamingId = $state<number | null>(null);
  let renameValue = $state('');
  let contextMenu = $state<{ x: number; y: number; entityId: number } | null>(null);

  let filtered = $derived(
    filter
      ? entities.filter((e) => e.name.toLowerCase().includes(filter.toLowerCase()))
      : entities,
  );

  function handleNew() {
    const e = createEntity();
    onSelect(e.id);
  }

  function startRename(id: number, currentName: string) {
    renamingId = id;
    renameValue = currentName;
    contextMenu = null;
  }

  function commitRename() {
    if (renamingId !== null && renameValue.trim()) {
      renameEntity(renamingId, renameValue.trim());
    }
    renamingId = null;
    renameValue = '';
  }

  function cancelRename() {
    renamingId = null;
    renameValue = '';
  }

  function openContextMenu(e: MouseEvent, entityId: number) {
    e.preventDefault();
    contextMenu = { x: e.clientX, y: e.clientY, entityId };
  }

  function closeContextMenu() {
    contextMenu = null;
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Delete' && selectedId !== null && renamingId === null) {
      e.preventDefault();
      deleteEntity(selectedId);
    }
    if (e.key === 'Escape') cancelRename();
  }
</script>

<!-- svelte-ignore a11y-no-static-element-interactions -->
<div class="hierarchy" onkeydown={onKeydown} tabindex="-1" role="tree">

  <div class="hierarchy-header">
    <div class="hierarchy-search">
      <svg class="search-icon" width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
        <path d="M11.742 10.344a6.5 6.5 0 1 0-1.397 1.398h-.001l3.85 3.85a1 1 0 0 0 1.415-1.414l-3.85-3.85zm-5.242.156a5 5 0 1 1 0-10 5 5 0 0 1 0 10z"/>
      </svg>
      <input
        type="text"
        class="search-input"
        placeholder={t('hierarchy.search')}
        bind:value={filter}
      />
    </div>
    <button class="add-btn" onclick={handleNew} title="New Entity">+</button>
  </div>

  {#if entities.length === 0}
    <p class="hierarchy-empty">{t('placeholder.no_project')}</p>
  {:else if filtered.length === 0}
    <p class="hierarchy-empty">{t('hierarchy.empty')}</p>
  {:else}
    <div class="hierarchy-count">
      {t('hierarchy.count').replace('{count}', String(filtered.length))}
    </div>
    <ul class="entity-list" role="listbox" aria-label={t('panel.hierarchy')}>
      {#each filtered as entity (entity.id)}
        <li
          class="entity-row"
          class:selected={selectedId === entity.id}
          role="option"
          aria-selected={selectedId === entity.id}
          tabindex="0"
          onmouseenter={() => { hoveredId = entity.id; }}
          onmouseleave={() => { if (!contextMenu) hoveredId = null; }}
          onclick={() => {
            if (renamingId !== entity.id) {
              selectEntity(entity.id);
              onSelect(entity.id);
            }
          }}
          ondblclick={() => startRename(entity.id, entity.name)}
          oncontextmenu={(e) => openContextMenu(e, entity.id)}
          onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { selectEntity(entity.id); onSelect(entity.id); } }}
        >
          <span class="entity-chevron">
            <svg width="10" height="10" viewBox="0 0 16 16" fill="currentColor">
              <path d="M6 4l4 4-4 4" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </span>
          <span class="entity-icon">
            <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
              <path d="M8 1l6.5 3.75v6.5L8 15l-6.5-3.75v-6.5L8 1z" stroke="currentColor" stroke-width="1" fill="none"/>
            </svg>
          </span>

          {#if renamingId === entity.id}
            <!-- svelte-ignore a11y-autofocus -->
            <input
              class="rename-input"
              bind:value={renameValue}
              autofocus
              onblur={commitRename}
              onkeydown={(e) => {
                if (e.key === 'Enter') { e.preventDefault(); commitRename(); }
                if (e.key === 'Escape') { e.preventDefault(); cancelRename(); }
                e.stopPropagation();
              }}
              onclick={(e) => e.stopPropagation()}
            />
          {:else}
            <span class="entity-name">{entity.name}</span>
          {/if}

          {#if hoveredId === entity.id && renamingId !== entity.id}
            <span class="entity-actions">
              <button
                class="action-btn"
                title="Duplicate"
                onclick={(e) => { e.stopPropagation(); duplicateEntity(entity.id); }}
              >⧉</button>
              <button
                class="action-btn"
                title="Add Child"
                onclick={(e) => { e.stopPropagation(); const c = createEntity(); onSelect(c.id); }}
              >+</button>
              <button
                class="action-btn delete-btn"
                title="Delete"
                onclick={(e) => { e.stopPropagation(); deleteEntity(entity.id); }}
              >✕</button>
            </span>
          {:else if renamingId !== entity.id}
            <span class="entity-component-count" title={entity.components.join(', ')}>
              {entity.components.length}
            </span>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}

  {#if contextMenu}
    <div
      class="context-menu"
      style="left: {contextMenu.x}px; top: {contextMenu.y}px;"
      role="menu"
    >
      {@const cid = contextMenu.entityId}
      {@const cname = entities.find((e) => e.id === cid)?.name ?? ''}
      <button role="menuitem" onclick={() => { startRename(cid, cname); }}>Rename</button>
      <button role="menuitem" onclick={() => { duplicateEntity(cid); closeContextMenu(); }}>Duplicate</button>
      <button role="menuitem" onclick={() => { const c = createEntity(); onSelect(c.id); closeContextMenu(); }}>Add Child</button>
      <hr />
      <button role="menuitem" class="danger" onclick={() => { deleteEntity(cid); closeContextMenu(); }}>Delete</button>
    </div>
    <!-- svelte-ignore a11y-no-static-element-interactions -->
    <div class="context-backdrop" role="none" onclick={closeContextMenu}></div>
  {/if}
</div>

<style>
  .hierarchy {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
    outline: none;
    position: relative;
  }

  .hierarchy-header {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 8px;
    border-bottom: 1px solid var(--color-border, #404040);
    flex-shrink: 0;
  }

  .hierarchy-search {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: 1;
    min-width: 0;
  }

  .search-icon { color: var(--color-textDim, #666); flex-shrink: 0; }

  .search-input {
    flex: 1;
    background: var(--color-bg, #1e1e1e);
    border: 1px solid var(--color-border, #404040);
    border-radius: 3px;
    padding: 3px 6px;
    font-size: 11px;
    color: var(--color-text, #ccc);
    outline: none;
    min-width: 0;
  }

  .search-input:focus { border-color: var(--color-accent, #007acc); }
  .search-input::placeholder { color: var(--color-textDim, #666); }

  .add-btn {
    all: unset;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    border-radius: 3px;
    font-size: 16px;
    color: var(--color-textMuted, #999);
    cursor: pointer;
    flex-shrink: 0;
    line-height: 1;
  }

  .add-btn:hover { background: var(--color-bgHeader, #2d2d2d); color: var(--color-text, #ccc); }

  .hierarchy-empty {
    color: var(--color-textDim, #666);
    font-style: italic;
    padding: 12px 8px;
    font-size: 12px;
    text-align: center;
  }

  .hierarchy-count {
    font-size: 10px;
    color: var(--color-textDim, #666);
    padding: 2px 8px;
    flex-shrink: 0;
  }

  .entity-list { list-style: none; margin: 0; padding: 0; overflow-y: auto; flex: 1; }

  .entity-row {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 3px 8px;
    cursor: pointer;
    font-size: 12px;
    color: var(--color-text, #ccc);
    border: 1px solid transparent;
    user-select: none;
  }

  .entity-row:hover { background: var(--color-bgHeader, #2d2d2d); }
  .entity-row.selected { background: var(--color-accent, #007acc); color: #fff; }
  .entity-row:focus-visible { outline: 1px solid var(--color-accent, #007acc); outline-offset: -1px; }

  .entity-chevron { display: flex; align-items: center; color: var(--color-textDim, #666); flex-shrink: 0; width: 14px; }
  .entity-icon { display: flex; align-items: center; color: var(--color-textMuted, #999); flex-shrink: 0; }
  .entity-row.selected .entity-icon, .entity-row.selected .entity-chevron { color: rgba(255,255,255,0.7); }

  .entity-name { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

  .rename-input {
    flex: 1;
    background: var(--color-bg, #1e1e1e);
    border: 1px solid var(--color-accent, #007acc);
    border-radius: 2px;
    padding: 1px 4px;
    font-size: 12px;
    color: var(--color-text, #ccc);
    outline: none;
    font-family: inherit;
    min-width: 0;
  }

  .entity-component-count {
    font-size: 10px;
    color: var(--color-textDim, #666);
    background: var(--color-bg, #1e1e1e);
    padding: 0 4px;
    border-radius: 3px;
    flex-shrink: 0;
  }
  .entity-row.selected .entity-component-count { background: rgba(255,255,255,0.15); color: rgba(255,255,255,0.8); }

  .entity-actions {
    display: flex;
    gap: 1px;
    margin-left: auto;
    flex-shrink: 0;
  }

  .action-btn {
    all: unset;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    border-radius: 2px;
    font-size: 11px;
    color: var(--color-textDim, #666);
    cursor: pointer;
  }

  .action-btn:hover { background: rgba(255,255,255,0.1); color: var(--color-text, #ccc); }
  .delete-btn:hover { color: #f38ba8; }

  /* Context menu */
  .context-menu {
    position: fixed;
    background: var(--color-bgPanel, #252525);
    border: 1px solid var(--color-border, #404040);
    border-radius: 5px;
    padding: 4px;
    min-width: 140px;
    box-shadow: 0 4px 16px rgba(0,0,0,0.5);
    z-index: 10000;
    display: flex;
    flex-direction: column;
  }

  .context-menu button {
    all: unset;
    display: block;
    width: 100%;
    padding: 5px 10px;
    font-size: 12px;
    color: var(--color-text, #ccc);
    cursor: pointer;
    border-radius: 3px;
    box-sizing: border-box;
  }

  .context-menu button:hover { background: var(--color-bgHeader, #2d2d2d); }
  .context-menu button.danger { color: #f38ba8; }
  .context-menu hr { border: none; border-top: 1px solid var(--color-border, #404040); margin: 3px 0; }

  .context-backdrop { position: fixed; inset: 0; z-index: 9999; }
</style>
```

- [ ] **Step 2: TypeScript check**

```bash
cd engine/editor && npx tsc --noEmit 2>&1 | grep -i "hierarchy\|Hierarchy" | head -10
```
Expected: no errors.

- [ ] **Step 3: Commit**

```bash
cd D:/dev/maethril/silmaril
git add engine/editor/src/lib/components/HierarchyPanel.svelte
git commit -m "feat(editor): hierarchy panel CRUD — hover actions, inline rename, context menu, Del key"
```

---

## Task 8: Inspector Add/Remove Components

**Files:**
- Modify: `engine/editor/src/lib/components/InspectorPanel.svelte`

- [ ] **Step 1: Read current `InspectorPanel.svelte` to understand existing structure**

```bash
head -40 engine/editor/src/lib/components/InspectorPanel.svelte
```

Key things to know from the current file:
- `entity.components` is iterated with `{#each entity.components as componentName}`
- The component section header is inside that loop
- `schemas` state is already present: `let schemas: ComponentSchemas = $state(getSchemas())`

- [ ] **Step 2: Add imports and new state to the `<script>` section**

In the script block, add:
```typescript
  import { addComponent, removeComponent } from '$lib/scene/commands';

  let addingComponent = $state(false);
  let componentFilter = $state('');
```

- [ ] **Step 3: Add ✕ remove button to the component section header**

Find the component header element inside `{#each entity.components as componentName}`. It will look like a `<div>` or `<section>` with the component name. Add the ✕ button to its right side:

```svelte
<button
  class="remove-component-btn"
  title="Remove {componentName}"
  onclick={() => removeComponent(entity.id, componentName)}
>✕</button>
```

The exact location depends on the current markup. Look for the element that shows the component name (e.g., `<h4>`, `<div class="component-header">`) and add the button as a sibling to it, right-aligned.

- [ ] **Step 4: Add "Add Component" button and dropdown after the `{#each}` loop**

After `{/each}` and still inside the entity section (not the `{#if !entity}` else branch), add:

```svelte
<!-- Add Component picker -->
<div class="add-component-section">
  {#if !addingComponent}
    <button class="add-component-btn" onclick={() => { addingComponent = true; componentFilter = ''; }}>
      + Add Component…
    </button>
  {:else}
    <div class="component-picker">
      <input
        class="component-filter-input"
        type="text"
        placeholder="Filter components…"
        bind:value={componentFilter}
        autofocus
        onkeydown={(e) => { if (e.key === 'Escape') addingComponent = false; }}
      />
      <ul class="component-picker-list">
        {#each Object.values(schemas).filter(s =>
          !entity.components.includes(s.name) &&
          (componentFilter === '' || s.name.toLowerCase().includes(componentFilter.toLowerCase()))
        ) as schema (schema.name)}
          <li>
            <button
              onclick={() => {
                addComponent(entity.id, schema.name);
                addingComponent = false;
              }}
            >{schema.name}</button>
          </li>
        {/each}
      </ul>
    </div>
    <!-- backdrop to close picker -->
    <div
      class="picker-backdrop"
      role="none"
      onclick={() => { addingComponent = false; }}
    ></div>
  {/if}
</div>
```

- [ ] **Step 5: Add CSS**

In the `<style>` block, append:

```css
  .remove-component-btn {
    all: unset;
    margin-left: auto;
    color: var(--color-textDim, #585b70);
    cursor: pointer;
    font-size: 11px;
    padding: 0 3px;
    border-radius: 2px;
    line-height: 1;
    flex-shrink: 0;
  }
  .remove-component-btn:hover { color: #f38ba8; }

  .add-component-section {
    padding: 4px 6px 6px;
    position: relative;
  }

  .add-component-btn {
    all: unset;
    display: block;
    width: 100%;
    background: none;
    border: 1px dashed var(--color-border, #45475a);
    color: var(--color-textDim, #6c7086);
    border-radius: 4px;
    padding: 5px 8px;
    cursor: pointer;
    font-size: 11px;
    text-align: left;
    box-sizing: border-box;
    font-family: inherit;
  }
  .add-component-btn:hover { border-color: var(--color-accent, #89b4fa); color: var(--color-text, #cdd6f4); }

  .component-picker {
    background: var(--color-bgPanel, #1e1e2e);
    border: 1px solid var(--color-accent, #89b4fa);
    border-radius: 4px;
    overflow: hidden;
    position: relative;
    z-index: 100;
  }

  .component-filter-input {
    width: 100%;
    background: var(--color-bg, #181825);
    border: none;
    border-bottom: 1px solid var(--color-border, #313244);
    padding: 5px 8px;
    font-size: 11px;
    color: var(--color-text, #cdd6f4);
    outline: none;
    box-sizing: border-box;
    font-family: inherit;
  }

  .component-picker-list {
    list-style: none;
    margin: 0;
    padding: 3px 0;
    max-height: 160px;
    overflow-y: auto;
  }

  .component-picker-list button {
    all: unset;
    display: block;
    width: 100%;
    padding: 4px 10px;
    font-size: 11px;
    color: var(--color-textMuted, #a6adc8);
    cursor: pointer;
    box-sizing: border-box;
  }
  .component-picker-list button:hover {
    background: var(--color-bgHover, #313244);
    color: var(--color-text, #cdd6f4);
  }

  .picker-backdrop { position: fixed; inset: 0; z-index: 99; }
```

- [ ] **Step 6: TypeScript check**

```bash
cd engine/editor && npx tsc --noEmit 2>&1 | grep -i "inspector\|Inspector" | head -10
```
Expected: no errors.

- [ ] **Step 7: Commit**

```bash
cd D:/dev/maethril/silmaril
git add engine/editor/src/lib/components/InspectorPanel.svelte
git commit -m "feat(editor): inspector add/remove components — ✕ button per header, add-component picker"
```

---

## Task 9: Run All Tests

- [ ] **Step 1: Run all TS tests**

```bash
cd engine/editor && npx vitest run
```
Expected: all tests pass including: `recent-items.test.ts` (4), `assets.test.ts` (4), `commands.test.ts` (4), existing omnibar tests (21+).

- [ ] **Step 2: Run Rust tests**

```bash
cd engine/editor && cargo test --lib -p silmaril-editor
```
Expected: all tests pass (43+), no regressions.

- [ ] **Step 3: Fix any failures before proceeding**

If TS tests fail, run individual test files to isolate:
```bash
cd engine/editor && npx vitest run src/lib/stores/recent-items.test.ts
cd engine/editor && npx vitest run src/lib/stores/assets.test.ts
cd engine/editor && npx vitest run src/lib/scene/commands.test.ts
```

If Rust fails: `cd engine/editor && cargo test --lib -p silmaril-editor 2>&1 | grep FAILED`

- [ ] **Step 4: Commit any fixes**

```bash
cd D:/dev/maethril/silmaril
git add -p
git commit -m "fix(editor): address test failures from wiring pass"
```
