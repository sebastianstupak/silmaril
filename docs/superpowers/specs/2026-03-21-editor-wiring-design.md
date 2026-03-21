# Editor Wiring — Design Spec

**Date:** 2026-03-21
**Scope:** Omnibar recent items persist store + Hierarchy CRUD + Inspector add/remove components + Assets panel wiring

---

## 1. Overview

Four related areas that together make the editor panels fully functional rather than scaffolded:

1. **Omnibar recent items** — persist recent projects/scenes across sessions; replace the `[]` stub
2. **Hierarchy panel CRUD** — create, duplicate, delete, rename entities via hover icons + right-click menu
3. **Inspector add/remove components** — ✕ remove button per component header + inline searchable "Add Component…" picker
4. **Assets panel** — wire `scan_assets` on project open; grouped collapsible list with filter + click-to-copy

---

## 2. Data Layer

### 2.1 `stores/recent-items.ts` (new)

Wraps existing `persist.ts` pattern (tauri-plugin-store with localStorage fallback).

```typescript
export interface RecentItem {
  label: string;       // project name or scene filename
  path: string;        // absolute path
  itemType: 'project' | 'scene';
  openedAt: number;    // Date.now() timestamp for sort/display
}

export function addRecentItem(item: Omit<RecentItem, 'openedAt'>): void
export function getRecentItems(): RecentItem[]
export function subscribeRecent(fn: (items: RecentItem[]) => void): () => void
```

- Max 10 items; oldest dropped when full
- Deduplicates by `path` (updating `openedAt` on re-open)
- Persist key: `'recent-items'`

### 2.2 `stores/assets.ts` (new)

Simple in-memory store populated on project open.

```typescript
export interface AssetEntry {
  path: string;
  assetType: 'texture' | 'mesh' | 'audio' | 'config' | 'unknown';
  filename: string;    // basename — derived from path, cached here for display
}

export function setAssets(list: AssetEntry[]): void
export function getAssets(): AssetEntry[]
export function subscribeAssets(fn: (assets: AssetEntry[]) => void): () => void
export function clearAssets(): void
```

- Not persisted (re-scanned on each project open)
- Used by both AssetsPanel and Omnibar `#` prefix (no second scan)

### 2.3 `scene/commands.ts` — two new commands

```typescript
export function addComponent(entityId: number, componentName: string): void
// 1. Look up schema for componentName via getSchemas() (schema-store)
// 2. Build default componentValues via applyComponentDefaults()
// 3. Push componentName to entity.components, merge componentValues
// 4. Call setComponentField IPC for each default field (Tauri forward)
// 5. _mutate() scene state

export function removeComponent(entityId: number, componentName: string): void
// 1. Remove componentName from entity.components
// 2. Delete componentName key from entity.componentValues
// 3. _mutate() scene state
// 4. invoke('remove_component', { entityId, component: componentName }) — fire-and-forget
```

Also add both to `dispatchSceneCommand` router:
- `'add_component'` → `addComponent(args.entity_id, args.component)`
- `'remove_component'` → `removeComponent(args.entity_id, args.component)`

### 2.4 Tauri commands (new, `bridge/commands.rs`)

```rust
#[tauri::command]
pub fn add_component(entity_id: u64, component: String) -> Result<(), String> {
    // Validate component name is non-empty; ECS not live yet — return Ok(())
    if component.is_empty() { return Err("component name required".into()); }
    Ok(())
}

#[tauri::command]
pub fn remove_component(entity_id: u64, component: String) -> Result<(), String> {
    if component.is_empty() { return Err("component name required".into()); }
    Ok(())
}
```

Register both in `invoke_handler!` and `api.ts`.

---

## 3. Omnibar Recent Items

### 3.1 App.svelte changes

- Import `addRecentItem`, `getRecentItems`, `subscribeRecent` from `stores/recent-items`
- In `onMount`: call `hydrateRecentItems()` (load from persist store into memory)
- After `openProject()` succeeds: `addRecentItem({ label: state.project_name ?? path, path, itemType: 'project' })`
- Add reactive `recentItems` state wired via `subscribeRecent`
- Pass `recentItems` as prop to `<TitleBar>` → `<Omnibar>`

### 3.2 Omnibar.svelte changes

- Add prop `recentItems: RecentItem[] = []`
- Replace `buildResults(query, merged, entities, assets, [])` → `buildResults(query, merged, entities, assets, recentItems)`
- Implement `case 'recent'` in `execute()`:
  ```typescript
  case 'recent':
    // Re-open the project
    const state = await invoke<EditorState>('open_project', { path: result.path });
    const entities = await invoke<EntityInfo[]>('scan_project_entities', { projectPath: result.path });
    setEntities(entities);
    setSelectedEntityId(null);
    break;
  ```

### 3.3 TitleBar.svelte

- Add `recentItems` prop (passed through to Omnibar)

---

## 4. Hierarchy Panel CRUD

### 4.1 Interaction model (C — hover row icons)

| Action | Trigger | Command |
|--------|---------|---------|
| New root entity | `+` in panel header | `createEntity()` |
| Add child | `+` hover icon · right-click → Add Child | `createEntity()` with parent ref (future — v1 creates flat) |
| Duplicate | `⧉` hover icon · right-click | `duplicateEntity(id)` |
| Delete | `✕` hover icon · right-click · `Del` key | `deleteEntity(id)` |
| Rename | double-click name · right-click → Rename | `renameEntity(id, newName)` |
| Select | single click | `selectEntity(id)` |

All commands already exist in `scene/commands.ts` — no new scene commands needed.

### 4.2 HierarchyPanel.svelte changes

- Add filter `<input>` to header (client-side fuzzy filter over `entities`)
- Add `+` button in header → `createEntity()`; new entity auto-selected
- Each entity row:
  - `onmouseenter` / `onmouseleave` → track `hoveredId`
  - Show ⧉ / + / ✕ icon buttons when `hoveredId === entity.id`
  - `ondblclick` on name span → enter rename mode (replace span with `<input>`, commit on Enter/blur, cancel on Escape)
  - `oncontextmenu` → open context menu at cursor position
- Context menu component (inline, no external library): Rename · Duplicate · Add Child · — · Delete
- `Del` key handler on the panel: `deleteEntity(selectedId)` when panel is focused
- All commands imported from `scene/commands.ts`

### 4.3 HierarchyWrapper.svelte

- Pass `onCreateEntity`, `onDeleteEntity`, `onDuplicateEntity`, `onRenameEntity` callbacks if needed (or HierarchyPanel imports commands directly — prefer direct import, consistent with InspectorPanel pattern)

---

## 5. Inspector Add/Remove Components

### 5.1 InspectorPanel.svelte changes

**Remove component:**
- Each component section header gets a `✕` button (right-aligned)
- `onclick`: `removeComponent(entity.id, componentName)`
- No confirmation dialog — the action is immediately visible and reversible via future undo

**Add Component picker:**
- "＋ Add Component…" button rendered below the last component section
- Click → open inline dropdown (not a modal)
- Dropdown contains:
  - Filter `<input>` (fuzzy match on component name)
  - List of schemas from `getSchemas()` (schema-store) filtered to exclude already-added components
  - Click a schema name → `addComponent(entity.id, schema.name)` → dropdown closes
  - Close on: Escape, outside click
- State: `let addingComponent = $state(false)` — local to InspectorPanel

### 5.2 Schema availability

`getSchemas()` from `schema-store.ts` is already populated via `getComponentSchemas()` IPC on mount. No new data fetching needed.

---

## 6. Assets Panel

### 6.1 App.svelte — populate on project open

After `openProject()` + `scanProjectEntities()` succeed:
```typescript
const rawAssets = await invoke<{ path: string; asset_type: string }[]>('scan_assets', { projectPath: path });
setAssets(rawAssets.map(a => ({
  path: a.path,
  assetType: a.asset_type as AssetEntry['assetType'],
  filename: a.path.split(/[\\/]/).pop() ?? a.path,
})));
```

On project close or new project open: `clearAssets()`.

### 6.2 AssetsPanel.svelte

- Subscribe to `subscribeAssets`
- Filter `<input>` — fuzzy-matches `filename` across all groups
- Groups: Textures · Meshes · Audio · Config · Unknown — each collapsible (`▾`/`▸`)
- Each asset row: click → `navigator.clipboard.writeText(asset.path)` + brief toast (`"Path copied"`)
- Empty state: "Open a project to browse assets"
- No drag-drop, no import pipeline in v1

### 6.3 Omnibar `#` prefix

Already implemented in `Omnibar.svelte` — calls `scan_assets` IPC lazily. Change: instead of calling IPC directly, read from the assets store first; only call IPC as fallback if store is empty (avoids redundant scan after project open).

---

## 7. API Layer additions (`api.ts`)

```typescript
export async function addComponent(entityId: number, component: string): Promise<void>
export async function removeComponent(entityId: number, component: string): Promise<void>
```

Both with `isTauri` guard and browser no-op fallback.

---

## 8. File Summary

**New files:**
- `engine/editor/src/lib/stores/recent-items.ts`
- `engine/editor/src/lib/stores/assets.ts`

**Modified files:**
- `engine/editor/src/App.svelte` — recent items hydrate/push, assets populate, prop threads
- `engine/editor/src/lib/omnibar/Omnibar.svelte` — recentItems prop, case 'recent' handler
- `engine/editor/src/lib/components/TitleBar.svelte` — recentItems prop thread
- `engine/editor/src/lib/components/HierarchyPanel.svelte` — full CRUD UI
- `engine/editor/src/lib/docking/panels/HierarchyWrapper.svelte` — (minor, if needed)
- `engine/editor/src/lib/components/InspectorPanel.svelte` — remove/add component UI
- `engine/editor/src/lib/components/AssetsPanel.svelte` — wired from assets store
- `engine/editor/src/lib/scene/commands.ts` — addComponent, removeComponent
- `engine/editor/src/lib/api.ts` — addComponent, removeComponent wrappers
- `engine/editor/src-tauri/bridge/commands.rs` — add_component, remove_component
- `engine/editor/src-tauri/lib.rs` — register new commands

---

## 9. Testing

- Unit tests for `recent-items.ts`: addRecentItem deduplication, max-10 eviction, subscribe
- Unit tests for `assets.ts`: setAssets, getAssets, subscribe, clear
- Unit tests for `addComponent` / `removeComponent` in commands.ts: default values applied, schema lookup, already-removed no-op
- Existing tests must remain green
