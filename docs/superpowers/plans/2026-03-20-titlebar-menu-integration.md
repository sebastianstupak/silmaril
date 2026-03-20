# Titlebar Menu Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Merge the separate `MenuBar` into `TitleBar`, eliminating a wasted 32px row, and add a compact-menu setting that collapses labels to icon-only triggers.

**Architecture:** Menu `DropdownMenu.*` markup moves from `MenuBar.svelte` into `TitleBar.svelte` via surgical additions (new props + new `.titlebar-menus` section). `MenuBar.svelte` is deleted. `TitleBar` receives five new props alongside its existing layout-management props. `App.svelte` wires the new props and removes `<MenuBar>`. A `compactMenu` field is added to `EditorSettings` and exposed via the Settings dialog's Editor tab.

**Tech Stack:** Svelte 5, TypeScript, shadcn-svelte `DropdownMenu.*` (`$lib/components/ui/dropdown-menu`), Tauri v2 `invoke`, Vitest.

**Spec:** `docs/superpowers/specs/2026-03-20-titlebar-menu-integration-design.md`

---

## Current State (read before touching anything)

`TitleBar.svelte` is a complex untracked file with **12 existing props** (savedLayouts, activeLayoutId, isDirty, activePanels, onApplyLayout, onSaveToSlot, onResetSlot, onRenameSlot, onDuplicateSlot, onDeleteSlot, onCreateLayout, onAddPanel). It manages layout slots, hover cards, rename-in-place, overflow dropdown, panels dropdown, and window controls. **Do not replace the file wholesale — make targeted additions only.**

The drag guard in `TitleBar.svelte` is (lines 160 and 165):
```ts
(e.target as HTMLElement).closest('button, input, .slot-wrapper, .overflow-wrapper, .panels-btn-wrapper')
```
Menu triggers render as `<button>`, so they are already excluded. **Do not modify the drag guard.**

`App.svelte` currently passes 12 props to `<TitleBar>` and still has `<MenuBar>` as a separate component below it.

---

## File Map

| File | Action | Purpose |
|------|--------|---------|
| `engine/editor/src/lib/stores/settings.ts` | Modify | Add `compactMenu: boolean` to interface + defaults |
| `engine/editor/src/lib/stores/settings.test.ts` | Create | Test `compactMenu` default and load/save round-trip |
| `engine/editor/src/lib/components/TitleBar.svelte` | Modify (surgical) | Add 5 props + menus markup + compact CSS |
| `engine/editor/src/App.svelte` | Modify | Remove MenuBar, add 5 new TitleBar props, update $effect |
| `engine/editor/src/lib/components/SettingsDialog.svelte` | Modify | Add compact menu toggle in Editor tab |
| `engine/editor/src/lib/components/MenuBar.svelte` | Delete | No longer needed after merge |

---

## Task 1: Add `compactMenu` to settings

**Files:**
- Modify: `engine/editor/src/lib/stores/settings.ts`
- Create: `engine/editor/src/lib/stores/settings.test.ts`

- [ ] **Step 1: Write the failing test**

Create `engine/editor/src/lib/stores/settings.test.ts`:

```typescript
import { describe, it, expect, vi, beforeEach } from 'vitest';

// Reset module cache + localStorage before each test so tests are isolated.
beforeEach(() => {
  vi.resetModules();
  localStorage.clear();
});

describe('loadSettings — compactMenu', () => {
  it('defaults compactMenu to false when no stored settings exist', async () => {
    const { loadSettings } = await import('$lib/stores/settings');
    const settings = loadSettings();
    expect(settings.compactMenu).toBe(false);
  });

  it('returns false for compactMenu when stored settings lack the field', async () => {
    localStorage.setItem('silmaril-editor-settings', JSON.stringify({ theme: 'dark' }));
    const { loadSettings } = await import('$lib/stores/settings');
    const settings = loadSettings();
    expect(settings.compactMenu).toBe(false);
  });

  it('round-trips compactMenu=true through saveSettings → loadSettings', async () => {
    const { loadSettings, saveSettings } = await import('$lib/stores/settings');
    const settings = loadSettings();
    saveSettings({ ...settings, compactMenu: true });
    const reloaded = loadSettings();
    expect(reloaded.compactMenu).toBe(true);
  });
});
```

- [ ] **Step 2: Run test to confirm it fails**

```bash
cd engine/editor && npm test -- --reporter=verbose 2>&1 | grep -A5 "compactMenu"
```

Expected: FAIL — `compactMenu` does not exist on `EditorSettings`.

- [ ] **Step 3: Add `compactMenu` to `settings.ts`**

In `engine/editor/src/lib/stores/settings.ts`, add `compactMenu: boolean;` to the `EditorSettings` interface after `autoSave`, and `compactMenu: false` to the `defaults` object:

```typescript
export interface EditorSettings {
  theme: string;
  language: string;
  leftPanelWidth: number;
  rightPanelWidth: number;
  bottomPanelHeight: number;
  fontSize: number;
  autoSave: 'off' | 'on_focus_change' | 'after_delay';
  compactMenu: boolean;
}

const defaults: EditorSettings = {
  theme: 'dark',
  language: 'en',
  leftPanelWidth: 250,
  rightPanelWidth: 300,
  bottomPanelHeight: 200,
  fontSize: 13,
  autoSave: 'off',
  compactMenu: false,
};
```

- [ ] **Step 4: Run tests to confirm they pass**

```bash
cd engine/editor && npm test -- --reporter=verbose 2>&1 | grep -A5 "compactMenu"
```

Expected: 3 passing tests for `compactMenu`.

- [ ] **Step 5: Commit**

```bash
git add engine/editor/src/lib/stores/settings.ts engine/editor/src/lib/stores/settings.test.ts
git commit -m "feat(editor): add compactMenu field to EditorSettings"
```

---

## Task 2: Add menus to TitleBar (surgical edit)

**Files:**
- Modify: `engine/editor/src/lib/components/TitleBar.svelte`

**WARNING:** Do NOT replace this file. It has 12 existing props and complex layout-management UI. Make only the additions described below.

- [ ] **Step 1: Add two new imports at the top of the `<script>` block**

After the existing imports (lines 2–4), add:

```ts
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
  import { t } from '$lib/i18n';
```

- [ ] **Step 2: Add 5 new props to the `Props` interface**

Find the closing `}` of the `Props` interface (after `onAddPanel?: ...`). Add before it:

```ts
    onSettingsOpen?: () => void;
    onOpenProject?: () => void;
    onLayoutReset?: () => void;
    onLayoutSelect?: (template: string) => void;
    compactMenu?: boolean;
```

- [ ] **Step 3: Add 5 new bindings to the destructuring**

Find the destructuring block `let { savedLayouts = [], ..., onAddPanel, }: Props = $props();`. Add the new props:

```ts
    onSettingsOpen,
    onOpenProject,
    onLayoutReset,
    onLayoutSelect,
    compactMenu = false,
```

- [ ] **Step 4: Add menus markup to the HTML**

Find the `.titlebar-left` block in the template:

```svelte
  <!-- Left: app identity -->
  <div class="titlebar-left">
    <svg class="app-icon" ...>...</svg>
    <span class="app-name">Silmaril</span>
  </div>
```

Replace it with (wrapping the logo in `.titlebar-logo`, then adding `.titlebar-menus`):

```svelte
  <!-- Left: logo + menus -->
  <div class="titlebar-left">
    <!-- Logo sub-area (drag-through) -->
    <div class="titlebar-logo">
      <svg class="app-icon" width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden="true">
        <polygon points="6,1 10,1 15,6 15,10 10,15 6,15 1,10 1,6" fill="none" stroke="var(--color-accent,#007acc)" stroke-width="1.2"/>
        <polygon points="6.5,4 9.5,4 12,6.5 12,9.5 9.5,12 6.5,12 4,9.5 4,6.5" fill="var(--color-accent,#007acc)" opacity="0.35"/>
      </svg>
      <span class="app-name">Silmaril</span>
    </div>

    <!-- Menus sub-area -->
    <div class="titlebar-menus" class:compact={compactMenu} role="menubar">

      <!-- File -->
      <DropdownMenu.Root>
        <DropdownMenu.Trigger class="menu-trigger" title={t('menu.file')}>
          <svg width="14" height="14" viewBox="0 0 14 14" fill="currentColor" aria-hidden="true">
            <path d="M1 3a1 1 0 011-1h3.3l1.7 1.7H12a1 1 0 011 1v6a1 1 0 01-1 1H2a1 1 0 01-1-1V3z"/>
          </svg>
          <span class="menu-label">{t('menu.file')}</span>
        </DropdownMenu.Trigger>
        <DropdownMenu.Content align="start" sideOffset={4} class="min-w-[200px]">
          <DropdownMenu.Item onclick={() => onOpenProject?.()}>{t('menu.file.open_project')}</DropdownMenu.Item>
          <DropdownMenu.Item>
            {t('menu.file.save_scene')}
            <DropdownMenu.Shortcut>Ctrl+S</DropdownMenu.Shortcut>
          </DropdownMenu.Item>
          <DropdownMenu.Item>{t('menu.file.save_scene_as')}</DropdownMenu.Item>
          <DropdownMenu.Item>{t('menu.file.new_scene')}</DropdownMenu.Item>
          <DropdownMenu.Sub>
            <DropdownMenu.SubTrigger>{t('menu.file.recent_projects')}</DropdownMenu.SubTrigger>
            <DropdownMenu.SubContent>
              <DropdownMenu.Item disabled>No recent projects</DropdownMenu.Item>
            </DropdownMenu.SubContent>
          </DropdownMenu.Sub>
          <DropdownMenu.Separator />
          <DropdownMenu.Item onclick={() => invoke('window_close').catch(() => {})}>{t('menu.file.exit')}</DropdownMenu.Item>
        </DropdownMenu.Content>
      </DropdownMenu.Root>

      <!-- Edit -->
      <DropdownMenu.Root>
        <DropdownMenu.Trigger class="menu-trigger" title={t('menu.edit')}>
          <svg width="14" height="14" viewBox="0 0 14 14" fill="currentColor" aria-hidden="true">
            <path d="M9.5 1.5l3 3-7 7-3.5.5.5-3.5 7-7zm0-1a.5.5 0 01.35.15l3 3a.5.5 0 010 .7l-7 7a.5.5 0 01-.25.14L2.1 12a.5.5 0 01-.6-.6l.5-3.5a.5.5 0 01.14-.26l7-7A.5.5 0 019.5.5z"/>
          </svg>
          <span class="menu-label">{t('menu.edit')}</span>
        </DropdownMenu.Trigger>
        <DropdownMenu.Content align="start" sideOffset={4} class="min-w-[200px]">
          <DropdownMenu.Item>
            {t('menu.edit.undo')}
            <DropdownMenu.Shortcut>Ctrl+Z</DropdownMenu.Shortcut>
          </DropdownMenu.Item>
          <DropdownMenu.Item>
            {t('menu.edit.redo')}
            <DropdownMenu.Shortcut>Ctrl+Y</DropdownMenu.Shortcut>
          </DropdownMenu.Item>
          <DropdownMenu.Separator />
          <DropdownMenu.Item>
            {t('menu.edit.cut')}
            <DropdownMenu.Shortcut>Ctrl+X</DropdownMenu.Shortcut>
          </DropdownMenu.Item>
          <DropdownMenu.Item>
            {t('menu.edit.copy')}
            <DropdownMenu.Shortcut>Ctrl+C</DropdownMenu.Shortcut>
          </DropdownMenu.Item>
          <DropdownMenu.Item>
            {t('menu.edit.paste')}
            <DropdownMenu.Shortcut>Ctrl+V</DropdownMenu.Shortcut>
          </DropdownMenu.Item>
          <DropdownMenu.Separator />
          <DropdownMenu.Item>
            {t('menu.edit.duplicate')}
            <DropdownMenu.Shortcut>Ctrl+D</DropdownMenu.Shortcut>
          </DropdownMenu.Item>
          <DropdownMenu.Item>
            {t('menu.edit.delete')}
            <DropdownMenu.Shortcut>Del</DropdownMenu.Shortcut>
          </DropdownMenu.Item>
          <DropdownMenu.Item>
            {t('menu.edit.select_all')}
            <DropdownMenu.Shortcut>Ctrl+A</DropdownMenu.Shortcut>
          </DropdownMenu.Item>
        </DropdownMenu.Content>
      </DropdownMenu.Root>

      <!-- View -->
      <DropdownMenu.Root>
        <DropdownMenu.Trigger class="menu-trigger" title={t('menu.view')}>
          <svg width="14" height="14" viewBox="0 0 14 14" fill="currentColor" aria-hidden="true">
            <path d="M7 2C4 2 1.5 4.5 1 7c.5 2.5 3 5 6 5s5.5-2.5 6-5c-.5-2.5-3-5-6-5zm0 8a3 3 0 110-6 3 3 0 010 6zm0-4.5a1.5 1.5 0 100 3 1.5 1.5 0 000-3z"/>
          </svg>
          <span class="menu-label">{t('menu.view')}</span>
        </DropdownMenu.Trigger>
        <DropdownMenu.Content align="start" sideOffset={4} class="min-w-[200px]">
          <DropdownMenu.Sub>
            <DropdownMenu.SubTrigger>{t('menu.view.layout')}</DropdownMenu.SubTrigger>
            <DropdownMenu.SubContent>
              <DropdownMenu.Item onclick={() => onLayoutSelect?.('default')}>{t('layout.default')}</DropdownMenu.Item>
              <DropdownMenu.Item onclick={() => onLayoutSelect?.('tall')}>{t('layout.tall')}</DropdownMenu.Item>
              <DropdownMenu.Item onclick={() => onLayoutSelect?.('wide')}>{t('layout.wide')}</DropdownMenu.Item>
            </DropdownMenu.SubContent>
          </DropdownMenu.Sub>
          <DropdownMenu.Separator />
          <DropdownMenu.Item onclick={() => onLayoutReset?.()}>{t('layout.reset')}</DropdownMenu.Item>
        </DropdownMenu.Content>
      </DropdownMenu.Root>

      <!-- Entity -->
      <DropdownMenu.Root>
        <DropdownMenu.Trigger class="menu-trigger" title={t('menu.entity')}>
          <svg width="14" height="14" viewBox="0 0 14 14" fill="currentColor" aria-hidden="true">
            <path d="M7 1L1 4.5v5L7 13l6-3.5v-5L7 1zm0 1.2l4.5 2.6v.7L7 8.1 2.5 5.5v-.7L7 2.2zM2 6.4l4.5 2.6v3.3L2 9.7V6.4zm5.5 5.9V8.9L12 6.4v3.3l-4.5 2.6z"/>
          </svg>
          <span class="menu-label">{t('menu.entity')}</span>
        </DropdownMenu.Trigger>
        <DropdownMenu.Content align="start" sideOffset={4} class="min-w-[200px]">
          <DropdownMenu.Item>{t('menu.entity.create_empty')}</DropdownMenu.Item>
          <DropdownMenu.Item>{t('menu.entity.create_from_template')}</DropdownMenu.Item>
          <DropdownMenu.Separator />
          <DropdownMenu.Item>{t('menu.entity.add_component')}</DropdownMenu.Item>
        </DropdownMenu.Content>
      </DropdownMenu.Root>

      <!-- Build -->
      <DropdownMenu.Root>
        <DropdownMenu.Trigger class="menu-trigger" title={t('menu.build')}>
          <svg width="14" height="14" viewBox="0 0 14 14" fill="currentColor" aria-hidden="true">
            <path d="M9.5 1a3 3 0 00-2.83 4L1.5 10.2a1 1 0 000 1.4l.9.9a1 1 0 001.4 0L9 7.33A3 3 0 109.5 1zm0 4.5a1.5 1.5 0 110-3 1.5 1.5 0 010 3z"/>
          </svg>
          <span class="menu-label">{t('menu.build')}</span>
        </DropdownMenu.Trigger>
        <DropdownMenu.Content align="start" sideOffset={4} class="min-w-[200px]">
          <DropdownMenu.Item>
            {t('menu.build.build_project')}
            <DropdownMenu.Shortcut>Ctrl+B</DropdownMenu.Shortcut>
          </DropdownMenu.Item>
          <DropdownMenu.Item>{t('menu.build.build_release')}</DropdownMenu.Item>
          <DropdownMenu.Separator />
          <DropdownMenu.Item>{t('menu.build.package')}</DropdownMenu.Item>
          <DropdownMenu.Item>{t('menu.build.platform_settings')}</DropdownMenu.Item>
        </DropdownMenu.Content>
      </DropdownMenu.Root>

      <!-- Modules -->
      <DropdownMenu.Root>
        <DropdownMenu.Trigger class="menu-trigger" title={t('menu.modules')}>
          <svg width="14" height="14" viewBox="0 0 14 14" fill="currentColor" aria-hidden="true">
            <path d="M5.5 1A1.5 1.5 0 004 2.5v.5H2.5A1.5 1.5 0 001 4.5v7A1.5 1.5 0 002.5 13h9a1.5 1.5 0 001.5-1.5v-7A1.5 1.5 0 0011.5 3H10v-.5A1.5 1.5 0 008.5 1h-3zM5 2.5a.5.5 0 01.5-.5h3a.5.5 0 01.5.5V3H5v-.5zM3.5 5h1a.5.5 0 010 1h-1a.5.5 0 010-1zm3 0h4a.5.5 0 010 1h-4a.5.5 0 010-1zm-3 3h1a.5.5 0 010 1h-1a.5.5 0 010-1zm3 0h4a.5.5 0 010 1h-4a.5.5 0 010-1z"/>
          </svg>
          <span class="menu-label">{t('menu.modules')}</span>
        </DropdownMenu.Trigger>
        <DropdownMenu.Content align="start" sideOffset={4} class="min-w-[200px]">
          <DropdownMenu.Item>{t('menu.modules.add_module')}</DropdownMenu.Item>
          <DropdownMenu.Item>{t('menu.modules.manage_modules')}</DropdownMenu.Item>
        </DropdownMenu.Content>
      </DropdownMenu.Root>

      <!-- Help -->
      <DropdownMenu.Root>
        <DropdownMenu.Trigger class="menu-trigger" title={t('menu.help')}>
          <svg width="14" height="14" viewBox="0 0 14 14" fill="currentColor" aria-hidden="true">
            <path d="M7 1a6 6 0 100 12A6 6 0 007 1zm0 9.5a.75.75 0 110-1.5.75.75 0 010 1.5zm.75-3.5a.75.75 0 01-1.5 0V7c0-.41.34-.75.75-.75a1.25 1.25 0 000-2.5A1.25 1.25 0 005.75 5a.75.75 0 01-1.5 0 2.75 2.75 0 115.5 0c0 1.07-.62 2-1.5 2.45V7z"/>
          </svg>
          <span class="menu-label">{t('menu.help')}</span>
        </DropdownMenu.Trigger>
        <DropdownMenu.Content align="start" sideOffset={4} class="min-w-[200px]">
          <DropdownMenu.Item>{t('menu.help.documentation')}</DropdownMenu.Item>
          <DropdownMenu.Separator />
          <DropdownMenu.Item onclick={() => onSettingsOpen?.()}>{t('settings.title')}</DropdownMenu.Item>
          <DropdownMenu.Separator />
          <DropdownMenu.Item>{t('menu.help.about')}</DropdownMenu.Item>
        </DropdownMenu.Content>
      </DropdownMenu.Root>

    </div>
  </div>
```

- [ ] **Step 5: Update CSS in the `<style>` block**

Find the `.titlebar-left` CSS rule:

```css
  .titlebar-left {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 0 14px;
    flex-shrink: 0;
    pointer-events: none;
  }
```

Replace with (remove pointer-events, add min-width, remove gap/padding — those move to sub-elements):

```css
  .titlebar-left {
    display: flex;
    align-items: center;
    flex-shrink: 0;
    min-width: 0;
  }

  /* Logo sub-area: drag-through, non-interactive */
  .titlebar-logo {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 0 10px 0 14px;
    flex-shrink: 0;
    pointer-events: none;
  }

  /* Menus sub-area */
  .titlebar-menus {
    display: flex;
    align-items: center;
    gap: 0;
    pointer-events: auto;
  }

  .titlebar-menus :global(.menu-trigger) {
    all: unset;
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 0 7px;
    font-size: 12px;
    font-weight: 500;
    color: var(--color-textMuted, #999);
    cursor: default;
    user-select: none;
    border-radius: 3px;
    height: 22px;
    white-space: nowrap;
    min-width: 28px;
    box-sizing: border-box;
  }

  .titlebar-menus :global(.menu-trigger:hover),
  .titlebar-menus :global(.menu-trigger[data-state="open"]) {
    background: rgba(255, 255, 255, 0.08);
    color: var(--color-textBright, #fff);
  }

  /* Compact mode: icon only */
  .titlebar-menus.compact :global(.menu-label) {
    display: none;
  }
```

- [ ] **Step 6: Verify TypeScript compiles cleanly**

```bash
cd engine/editor && npx svelte-check --tsconfig ./tsconfig.json 2>&1 | tail -8
```

Expected: `0 errors`.

- [ ] **Step 7: Run tests**

```bash
cd engine/editor && npm test
```

Expected: All tests pass.

- [ ] **Step 8: Commit**

```bash
git add engine/editor/src/lib/components/TitleBar.svelte
git commit -m "feat(editor): add menu bar into TitleBar with icons and compact mode"
```

---

## Task 3: Update App.svelte and delete MenuBar

**Files:**
- Modify: `engine/editor/src/App.svelte`
- Delete: `engine/editor/src/lib/components/MenuBar.svelte`

- [ ] **Step 1: Confirm MenuBar is only imported in App.svelte**

```bash
cd engine/editor && grep -r "MenuBar" src/
```

Expected: only `src/App.svelte`. If any other files appear, update them before proceeding.

- [ ] **Step 2: Remove MenuBar import from App.svelte**

Remove this line:
```ts
import MenuBar from './lib/components/MenuBar.svelte';
```

- [ ] **Step 3: Remove the `<MenuBar>` block from the template**

Remove (lines ~398–404):
```svelte
  <!-- Menu Bar -->
  <MenuBar
    onSettingsOpen={() => showSettings = true}
    onOpenProject={handleOpenProject}
    onLayoutReset={handleLayoutReset}
    onLayoutSelect={handleLayoutSelect}
  />
```

- [ ] **Step 4: Add the 5 new props to the existing `<TitleBar>` call**

Find the `<TitleBar>` block (which ends with `onAddPanel={(id) => addPanelToLayout(id)}`). Add the five new props:

```svelte
    <TitleBar
        {savedLayouts}
        {activeLayoutId}
        {isDirty}
        {activePanels}
        onApplyLayout={applyLayout}
        onSaveToSlot={saveToSlot}
        onResetSlot={resetSlot}
        onRenameSlot={renameSlot}
        onDuplicateSlot={duplicateSlot}
        onDeleteSlot={deleteSlot}
        onCreateLayout={createLayout}
        onAddPanel={(id) => addPanelToLayout(id)}
        onSettingsOpen={() => showSettings = true}
        onOpenProject={handleOpenProject}
        onLayoutReset={handleLayoutReset}
        onLayoutSelect={handleLayoutSelect}
        compactMenu={settings.compactMenu}
      />
```

- [ ] **Step 5: Add `settings.compactMenu` to the save `$effect` tracking list**

Find the `$effect` around line 225 that starts:
```ts
  $effect(() => {
    // Access properties explicitly so the effect re-runs when any of these change.
    settings.theme; settings.fontSize; settings.language; settings.autoSave;
```

Add `settings.compactMenu;` to the list:
```ts
    settings.theme; settings.fontSize; settings.language; settings.autoSave; settings.compactMenu;
```

- [ ] **Step 6: Delete MenuBar.svelte**

```bash
git rm engine/editor/src/lib/components/MenuBar.svelte
```

- [ ] **Step 7: Verify TypeScript compiles cleanly**

```bash
cd engine/editor && npx svelte-check --tsconfig ./tsconfig.json 2>&1 | tail -8
```

Expected: `0 errors`.

- [ ] **Step 8: Run tests**

```bash
cd engine/editor && npm test
```

Expected: All tests pass.

- [ ] **Step 9: Commit**

```bash
git add engine/editor/src/App.svelte
git commit -m "feat(editor): wire menus through TitleBar, remove MenuBar"
```

---

## Task 4: Add compact menu toggle to SettingsDialog

**Files:**
- Modify: `engine/editor/src/lib/components/SettingsDialog.svelte`

The Editor tab (line ~149) is currently a placeholder paragraph.

- [ ] **Step 1: Replace the Editor tab placeholder**

Find:
```svelte
      <!-- Editor (placeholder) -->
      <Tabs.Content value="editor" class="flex-1 pt-1">
        <p class="text-sm text-muted-foreground italic">{t('settings.editor')}</p>
      </Tabs.Content>
```

Replace with:
```svelte
      <!-- Editor -->
      <Tabs.Content value="editor" class="flex-1 pt-1 space-y-4">
        <div class="flex items-center justify-between gap-4">
          <div class="flex flex-col gap-0.5">
            <Label>Compact menu</Label>
            <p class="text-xs text-muted-foreground">Show icons only in the title bar menu</p>
          </div>
          <button
            role="switch"
            aria-checked={settings.compactMenu}
            class="w-9 h-5 rounded-full border border-[var(--color-border,#404040)] relative transition-colors
              {settings.compactMenu
                ? 'bg-[var(--color-accent,#007acc)]'
                : 'bg-[var(--color-bgInput,#333)]'}"
            onclick={() => updateSetting('compactMenu', !settings.compactMenu)}
          >
            <span class="block w-3.5 h-3.5 rounded-full bg-white absolute top-0.5 transition-transform
              {settings.compactMenu ? 'translate-x-4' : 'translate-x-0.5'}">
            </span>
          </button>
        </div>
      </Tabs.Content>
```

- [ ] **Step 2: Verify TypeScript compiles cleanly**

```bash
cd engine/editor && npx svelte-check --tsconfig ./tsconfig.json 2>&1 | tail -8
```

Expected: `0 errors`.

- [ ] **Step 3: Run tests**

```bash
cd engine/editor && npm test
```

Expected: All tests pass.

- [ ] **Step 4: Commit**

```bash
git add engine/editor/src/lib/components/SettingsDialog.svelte
git commit -m "feat(editor): add compact menu toggle to settings editor tab"
```

---

## Manual Verification Checklist

After all tasks complete, launch the editor and verify:

- [ ] Title bar shows icon + "Silmaril" + File/Edit/View/Entity/Build/Modules/Help with icons and text labels
- [ ] Each menu opens on click with correct items; no menu opens on drag
- [ ] File > Exit closes the window
- [ ] View > Layout presets switch layouts; View > Reset Layout resets
- [ ] Entity, Build, Modules, Help menus open with correct items
- [ ] Help > Settings opens the settings dialog
- [ ] Dragging the title bar (click-drag on the center drag region) moves the window
- [ ] Double-clicking the center drag region toggles maximize
- [ ] Existing layout slot buttons still work (click to apply, right-click for context menu, rename, etc.)
- [ ] Settings > Editor > Compact menu toggle hides text labels, showing icon-only triggers with hover tooltips
- [ ] `compactMenu` preference persists after closing and reopening the editor
- [ ] No duplicate menu bar appears below the title bar
