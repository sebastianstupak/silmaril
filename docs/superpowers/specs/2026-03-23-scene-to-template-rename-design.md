# Scene → Template Rename Design

> **Status:** Approved

---

## Goal

Align the frontend vocabulary with the Rust backend, which already uses "template" throughout (`template.open`, `template.execute`, `template.undo`, etc.). Every user-visible "Scene" label and every internal `scene.*` command dispatch key / function name becomes "Template" / `template.*`.

This is a mechanical rename — no new behaviour, no new Rust-side IPC wire names.

> **Rust IPC wire names are NOT changed.** The inner `commands.runCommand('file.save_scene', null)` calls inside the file handlers stay as-is — only the frontend dispatch key changes. The Rust backend continues to respond to its existing command names.

---

## What Changes

### 1. i18n strings — `src/lib/i18n/locales/en.ts`

| Old key | New key | Old string | New string |
|---------|---------|------------|------------|
| `menu.file.save_scene` | `menu.file.save_template` | Save Scene | Save Template |
| `menu.file.save_scene_as` | `menu.file.save_template_as` | Save Scene As... | Save Template As... |
| `menu.file.new_scene` | `menu.file.new_template` | New Scene | New Template |
| `breadcrumb.no_scene` | `breadcrumb.no_template` | Untitled Scene | Untitled Template |
| `status.saved` *(key unchanged)* | — | Scene saved | Template saved |
| `viewport.no_entities` *(key unchanged)* | — | No entities in scene | No entities in template |
| `layout.scene` | `layout.template` | Scene | Template |
| `layout.default` *(key unchanged)* | — | Scene | Template |
| `scene.create_entity` | `template.create_entity` | Create Entity | Create Entity |
| `scene.delete_entity` | `template.delete_entity` | Delete Entity | Delete Entity |
| `scene.duplicate_entity` | `template.duplicate_entity` | Duplicate Entity | Duplicate Entity |

### 2. Frontend command dispatch keys

These are the keys used in `registerCommandHandler(...)` / `dispatch(...)`. They are **frontend-only** — the inner Rust IPC call strings inside each handler are kept unchanged.

| Old dispatch key | New dispatch key | File |
|------------------|------------------|------|
| `file.save_scene` | `file.save_template` | `commands/file.ts` |
| `file.save_scene_as` | `file.save_template_as` | `commands/file.ts` |
| `file.open_scene` | `file.open_template` | `commands/file.ts` |
| `scene.new_entity` | `template.new_entity` | `commands/template-entities.ts` |
| `scene.delete_entity` | `template.delete_entity` | `commands/template-entities.ts` |
| `scene.duplicate_entity` | `template.duplicate_entity` | `commands/template-entities.ts` |
| `scene.focus_entity` | `template.focus_entity` | `commands/template-entities.ts` |
| `editor.new_scene` | `editor.new_template` | `App.svelte` dispatch table |

### 3. Files renamed / moved

| Old path | New path | Notes |
|----------|----------|-------|
| `src/lib/commands/scene.ts` | `src/lib/commands/template-entities.ts` | Command IDs + handler name updated inside |
| `src/lib/scene/commands.ts` | `src/lib/template/commands.ts` | `dispatchSceneCommand` → `dispatchTemplateCommand`; `selectEntity`, `populateFromScan`, etc. kept |
| `src/lib/scene/state.ts` | `src/lib/template/state.ts` | Type `SceneEntity` → `TemplateEntity`; `SceneTool` → `TemplateTool` |
| `src/lib/scene/` *(directory)* | Deleted | Both files moved to `src/lib/template/` |

`src/lib/commands/template.ts` (lifecycle: open/close/execute/undo/redo/history) is unchanged.

### 4. Functions and types renamed

| Old name | New name | File |
|----------|----------|------|
| `dispatchSceneCommand` | `dispatchTemplateCommand` | `src/lib/template/commands.ts` |
| `sceneUndo` (export) | `templateUndo` | `src/lib/stores/undo-history.ts` |
| `sceneRedo` (export) | `templateRedo` | `src/lib/stores/undo-history.ts` |
| `_sceneUndoRedoInFlight` | `_templateUndoRedoInFlight` | `src/lib/stores/undo-history.ts` |
| `registerSceneHandlers` | `registerTemplateEntityHandlers` | `src/lib/commands/template-entities.ts` |
| `SceneEntity` (type) | `TemplateEntity` | `src/lib/template/state.ts` + all importers |
| `SceneCamera` (type) | `TemplateCamera` | `src/lib/template/state.ts` + all importers |
| `SceneState` (type) | `TemplateState` | `src/lib/template/state.ts` + all importers |
| `SceneTool` (type) | `TemplateTool` | `src/lib/template/state.ts` + all importers |
| `getSceneState()` | `getTemplateState()` | `src/lib/template/state.ts` + all importers |
| `subscribeScene()` | `subscribeTemplate()` | `src/lib/template/state.ts` + all importers |

Log messages in `undo-history.ts`: "Scene undo/redo failed" → "Template undo/redo failed". Comments referencing `scene_undo / scene_redo` updated.

### 5. Call sites — import paths updated

All files importing from `$lib/scene/commands` or `$lib/scene/state` must update their import paths to `$lib/template/commands` / `$lib/template/state`:

| File | What changes |
|------|-------------|
| `src/App.svelte` | Import `dispatchTemplateCommand` from `./lib/template/commands`; import `initTauriListeners` from `./lib/template/state`; `templateUndo`/`templateRedo` from undo-history; `t('breadcrumb.no_template')`; `editor.new_template` dispatch entry |
| `src/lib/commands/index.ts` | Import `registerTemplateEntityHandlers` from `./template-entities`; call it |
| `src/lib/commands/file.ts` | Dispatch keys renamed (see §2); inner `runCommand` strings unchanged |
| `src/lib/omnibar/Omnibar.svelte` | Import `selectEntity` from `$lib/template/commands` |
| `src/lib/components/HierarchyPanel.svelte` | Import from `$lib/template/commands` |
| `src/lib/components/InspectorPanel.svelte` | Import `TemplateEntity` type + functions from `$lib/template/state` / `$lib/template/commands` |
| `src/lib/docking/panels/InspectorWrapper.svelte` | Import `TemplateEntity` type from `$lib/template/state` |
| `src/lib/docking/panels/ViewportPanel.svelte` | Import `TemplateTool`, `ProjectionMode` from `$lib/template/state`; import functions from `$lib/template/commands` |
| `src/lib/stores/editor-context.ts` | Import from `$lib/template/state` + `$lib/template/commands` |

### 6. Test files updated

| File | What changes |
|------|-------------|
| `src/lib/commands/index.test.ts` | Expected command IDs updated: `file.save_scene` → `file.save_template`, `scene.new_entity` → `template.new_entity`, etc. |
| `src/lib/stores/editor-context.test.ts` | Mock paths updated: `vi.mock('$lib/template/state', ...)` and `vi.mock('$lib/template/commands', ...)` |
| `src/lib/scene/commands.test.ts` | Moved to `src/lib/template/commands.test.ts`; update file header comment |

---

## What Does NOT Change

- Rust-side IPC wire names: `file.save_scene`, `file.open_scene`, `template.open`, `template.undo`, etc. — all unchanged
- The inner `commands.runCommand(...)` strings inside each frontend handler — unchanged
- `undo-history.ts` already calls `commands.runCommand('template.undo', ...)` internally — only exported names change
- Viewport gizmo, inspector logic, hierarchy logic — no behavioural changes
- `ProjectionMode` type — not scene-specific, unchanged

---

## Implementation Order

Single commit. Edits in this order to keep the build green at each step:

1. `en.ts` — rename i18n keys and update strings
2. `scene/state.ts` → `template/state.ts` — move file, rename types (`SceneEntity` → `TemplateEntity`, `SceneTool` → `TemplateTool`)
3. `scene/commands.ts` → `template/commands.ts` — move file, rename `dispatchSceneCommand` → `dispatchTemplateCommand`
4. `commands/scene.ts` → `commands/template-entities.ts` — move file, update command IDs + handler name
5. `stores/undo-history.ts` — rename exports + internal guard + log messages
6. `commands/file.ts` — rename dispatch keys (keep inner runCommand strings)
7. `commands/index.ts` — update import + registration call
8. All component/store call sites — update import paths + renamed identifiers
9. All test files — update mock paths + command ID expectations
10. Grep-verify: `git grep -rn "'scene\." src/` returns zero matches (excluding comments)

---

## Testing

- `npm run typecheck` — zero new TS errors
- `npm run test` (vitest) — all unit tests pass
- `npx playwright test` — all E2E tests pass (they check tab labels, not command IDs)
- Manual: breadcrumb shows "Untitled Template"; File menu shows "New Template", "Save Template", "Save Template As..."

---

## Files Touched

| File | Change |
|------|--------|
| `src/lib/i18n/locales/en.ts` | Rename keys + update strings |
| `src/lib/commands/scene.ts` | Deleted — moved to `template-entities.ts` |
| `src/lib/commands/template-entities.ts` | New; entity command IDs + handler name |
| `src/lib/commands/file.ts` | Dispatch keys renamed; inner runCommand unchanged |
| `src/lib/commands/index.ts` | Import + registration call updated |
| `src/lib/commands/index.test.ts` | Expected command ID strings updated |
| `src/lib/scene/commands.ts` | Deleted — moved to `src/lib/template/commands.ts` |
| `src/lib/scene/state.ts` | Deleted — moved to `src/lib/template/state.ts` |
| `src/lib/scene/commands.test.ts` | Moved to `src/lib/template/commands.test.ts` |
| `src/lib/template/commands.ts` | New (from scene/); `dispatchTemplateCommand` |
| `src/lib/template/state.ts` | New (from scene/); `TemplateEntity`, `TemplateTool` types |
| `src/lib/template/commands.test.ts` | Moved from scene/; header updated |
| `src/lib/stores/undo-history.ts` | `sceneUndo/sceneRedo` → `templateUndo/templateRedo`; internal guard + logs |
| `src/lib/stores/editor-context.ts` | Import paths updated |
| `src/lib/stores/editor-context.test.ts` | Mock paths updated |
| `src/App.svelte` | Import paths, call sites, dispatch table entry, i18n key |
| `src/lib/omnibar/Omnibar.svelte` | Import path updated |
| `src/lib/components/HierarchyPanel.svelte` | Import path updated |
| `src/lib/components/InspectorPanel.svelte` | Import path + `TemplateEntity` type |
| `src/lib/docking/panels/InspectorWrapper.svelte` | Import path + `TemplateEntity` type |
| `src/lib/docking/panels/ViewportPanel.svelte` | Import path + `TemplateTool` type |
