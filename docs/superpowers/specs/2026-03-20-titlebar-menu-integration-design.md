# Titlebar Menu Integration

**Date:** 2026-03-20
**Status:** Draft

## Problem

The editor currently renders two separate 32px bars stacked at the top:

1. `TitleBar.svelte` — window chrome (drag, minimize/maximize/close, layout presets)
2. `MenuBar.svelte` — File · Edit · View · Entity · Build · Modules · Help

This wastes 32px of vertical screen space and splits related concerns across two components.

## Goal

Merge the menu bar into the title bar, producing a single 32px top row. Add a **compact menu** setting that collapses text labels to icon-only triggers.

---

## Design

### Layout

```
[icon · "Silmaril"  |  File Edit View Entity Build Modules Help]  [drag spacer]  [layout-btns · min/max/close]
└── .titlebar-logo ─┘  └──── .titlebar-menus ─────────────────┘  └─ center ──┘  └──── titlebar-right ────┘
└──────────────────────── .titlebar-left ────────────────────────────────────────────────────────────────────
```

- **`.titlebar-logo`** — icon + app name. `pointer-events: none` so this sub-area is a drag-through region.
- **`.titlebar-menus`** — menu triggers. `pointer-events: auto` so clicks register. Excluded from drag because the existing `onTitlebarMousedown` guard calls `closest('button, .layout-toggles')`, and `DropdownMenu.Trigger` renders as a `<button>`.
- **No separator** between logo and menus.
- Drag region (`.titlebar-center`) and window controls remain unchanged.

**CSS migration:** The current `.titlebar-left { pointer-events: none }` rule is split into two new rules: `.titlebar-logo { pointer-events: none }` and `.titlebar-menus { pointer-events: auto }`. The `.titlebar-left` rule itself drops the `pointer-events` declaration.

**Drag guard:** The existing guard string in both `onTitlebarMousedown` and `onTitlebarDblclick` is:

```ts
(e.target as HTMLElement).closest('button, .layout-toggles')
```

Leave this string **unchanged**. The `button` part already covers menu triggers. Do not simplify or extend the selector.

### Menu Triggers

Each trigger renders an icon + a `<span class="menu-label">` text label. The `menu-label` class must be added explicitly to each trigger's label text — it does not exist in the current `MenuBar.svelte` markup.

| Menu    | Icon           | Size   |
|---------|----------------|--------|
| File    | Folder         | 14×14  |
| Edit    | Pencil         | 14×14  |
| View    | Eye            | 14×14  |
| Entity  | Cube / box     | 14×14  |
| Build   | Hammer         | 14×14  |
| Modules | Puzzle piece   | 14×14  |
| Help    | Question mark  | 14×14  |

All icons are inline SVGs, consistent with existing toolbar icons.

In **compact mode** the `.titlebar-menus` container receives a `compact` CSS class. The selector:

```css
.titlebar-menus.compact :global(.menu-label) { display: none; }
```

Icon-only triggers retain explicit `min-width: 28px; padding: 0 6px` so they stay consistent with the 32px bar height. Each trigger has `title="File"` etc. for tooltip.

> **Note:** In Tauri v2 / WebView2 on Windows, `title` attribute tooltips have ~500ms system delay and no style control. This is acceptable for a developer tool; a custom tooltip layer can be added later if needed.

### Component Changes

#### `TitleBar.svelte` — extended

New props:

```ts
interface Props {
  onLayoutSelect?: (template: string) => void;
  onSettingsOpen?: () => void;
  onOpenProject?: () => void;
  onLayoutReset?: () => void;
  compactMenu?: boolean;
}
```

The full menu markup (previously in `MenuBar.svelte`) moves here, using the existing `DropdownMenu.*` primitives from `$lib/components/ui/dropdown-menu`.

**`File > Exit`** must be wired to `invoke('window_close')`, the same Tauri command used by the close button's `close()` function in the same file. In `MenuBar.svelte` the `File > Exit` item (line 34) has no `onclick` — this must be added when porting:

```svelte
<DropdownMenu.Item onclick={() => invoke('window_close').catch(() => {})}>
  {t('menu.file.exit')}
</DropdownMenu.Item>
```

`sideOffset` on each `DropdownMenu.Content` should be verified visually after the merge. The menus now open from within the title bar rather than from a separate bar below it; `sideOffset={2}` may need a small adjustment (try `4`) to maintain comfortable visual separation from the bottom edge of the bar.

#### `MenuBar.svelte` — deleted

Before deleting, search the codebase for all `MenuBar` import references to confirm `App.svelte` is the only consumer. Any other import sites must be updated or removed.

#### `App.svelte` — updated

- Remove `import MenuBar` and `<MenuBar>` usage.
- Pass new props to `<TitleBar>`:

```svelte
<TitleBar
  onLayoutSelect={handleLayoutSelect}
  onSettingsOpen={() => showSettings = true}
  onOpenProject={handleOpenProject}
  onLayoutReset={handleLayoutReset}
  compactMenu={settings.compactMenu}
/>
```

- In the `$effect` that drives `saveSettings` (line 227), add `settings.compactMenu;` to the explicitly-tracked property list so changes trigger the debounced save:

```ts
settings.theme; settings.fontSize; settings.language; settings.autoSave; settings.compactMenu;
```

### Settings

#### `settings.ts`

Add field to `EditorSettings` and its `defaults` object:

```ts
compactMenu: boolean  // default: false
```

`loadSettings` merges stored values with `defaults`, so existing stored settings without the field will correctly fall back to `false`.

#### `compactMenu` persistence and broadcast

`compactMenu` is persisted to localStorage via the existing debounced `saveSettings` call (triggered by the `$effect` addition above).

`compactMenu` is **not broadcast** to pop-out windows. Pop-out windows render no menu bar and have no use for this value. The `invoke('broadcast_settings', { theme, fontSize, language })` call in `handleSettingsChange` must **not** include `compactMenu` in its payload.

#### `SettingsDialog.svelte`

Under the **Editor** tab, add:

```
[✓] Compact menu
    Show icons only in the title bar menu
```

Bound to `settings.compactMenu`.

---

## Behaviour Notes

- **Drag interaction**: `onTitlebarMousedown` skips `button` elements (and `.layout-toggles`). Menu triggers are `<button>` elements, so they are excluded from drag automatically. Dropdown items are portalled to the document root and are outside the titlebar DOM entirely — they cannot accidentally trigger drag.
- **Double-click to maximize**: Guard checks `closest('button, .layout-toggles')`, so double-clicking a menu trigger will not toggle maximize.

---

## Out of Scope

- Keyboard navigation of the menu bar (F10 / Alt activation) — future work.
- Custom tooltip layer for compact mode — future work if native tooltip delay is unacceptable.
- Per-menu icon customisation via settings.
- Reordering or hiding individual menus.
