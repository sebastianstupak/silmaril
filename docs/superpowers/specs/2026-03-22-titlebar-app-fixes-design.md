# Title Bar / App Chrome Fixes Design

> **Status:** Approved

---

## Goal

Four fixes to the title bar and app shell:
1. **Dropdown menus** — fix "sometimes doesn't open" and "can't click items".
2. **Minimize button** — bottom-align the dash icon (Windows convention).
3. **Rounded corners in fullscreen** — remove corner radius when fullscreen.
4. **Scrollbars** — thin, dark-themed webkit scrollbars globally.

## Architecture

Primary file: `engine/editor/src/lib/components/TitleBar.svelte` (items 1–3).
Secondary file: `engine/editor/src/app.css` (items 3–4).
Library: `bits-ui` `DropdownMenu` (already used — no dependency changes).

---

## Section 1 — Dropdown Menu Fixes

### Root Cause Analysis

Two independent bugs:

**Bug A — "Sometimes doesn't open":** `onTitlebarMousedown` (line ~185) starts a native Tauri
window drag for any mousedown on the titlebar that doesn't hit a known interactive element.
`.menu-trigger` is absent from the exclusion selector list. When the user clicks a menu trigger,
`window_start_drag` is invoked before bits-ui sees the event; the OS drag operation swallows
subsequent pointer events, so the dropdown never opens.

**Bug B — "Can't click contents" / stacking:** The `showPanelsMenu` reactive effect (line ~180)
registers a global `document.addEventListener('click', close)` **without** `{ once: true }`.
Each time the panels menu opens and closes, a new permanent listener is added. After a few
cycles, many listeners stack and a click anywhere (including on `DropdownMenu.Item`) triggers
multiple close handlers, collapsing the UI before the click action fires.

**Bug C — z-index conflict:** `DropdownMenu.Content` uses `z-50` (Tailwind → `z-index: 50`).
The `.titlebar` CSS has `z-index: 50`. If the dropdown content does NOT portal outside the
titlebar stacking context, it renders at the same level and can be clipped or covered.

### 1a — Fix `onTitlebarMousedown` exclusion list

Add `.titlebar-menus`, `[data-bits-dropdown-menu-content]`, `[data-bits-dropdown-menu-item]`,
and `[data-state="open"]` to the early-return guard:

```ts
function onTitlebarMousedown(e: MouseEvent) {
  if (e.button !== 0) return;
  const target = e.target as HTMLElement;
  if (target.closest(
    'button, input, .slot-wrapper, .overflow-wrapper, ' +
    '.panels-btn-wrapper, .omnibar-wrapper, ' +
    '.titlebar-menus, [data-dropdown-menu-content], ' +               // ← new (bits-ui attr)
    '[data-dropdown-menu-item], ' +                                    // ← new (bits-ui attr)
    '[data-dropdown-menu-trigger][data-state="open"]'                  // ← new (open trigger only)
  )) return;
  invoke('window_start_drag').catch(() => {});
}
```

Apply the same additions to `onTitlebarDblclick`:

```ts
function onTitlebarDblclick(e: MouseEvent) {
  const target = e.target as HTMLElement;
  if (target.closest(
    'button, input, .slot-wrapper, .overflow-wrapper, ' +
    '.panels-btn-wrapper, .omnibar-wrapper, ' +
    '.titlebar-menus, [data-dropdown-menu-content], ' +
    '[data-dropdown-menu-item], ' +
    '[data-dropdown-menu-trigger][data-state="open"]'
  )) return;
  invoke('window_toggle_maximize').catch(() => {});
}
```

**Selector rationale:**
- `[data-dropdown-menu-content]` / `[data-dropdown-menu-item]` — actual DOM attributes set by
  bits-ui (variant = `"dropdown-menu"`, name = `"content"` / `"item"`). Not `data-bits-*`.
- `[data-dropdown-menu-trigger][data-state="open"]` — targeted to open dropdown triggers only,
  avoiding the overly broad `[data-state="open"]` which bits-ui applies to many components
  (accordions, dialogs, sub-triggers, etc.).
```

### 1b — Fix stacked click listeners in `showPanelsMenu` effect

Add `{ once: true }` so the listener removes itself after a single click:

```ts
$effect(() => {
  if (!showPanelsMenu) return;
  function close(e: MouseEvent) {
    if (!(e.target as HTMLElement).closest('.panels-btn-wrapper')) showPanelsMenu = false;
  }
  const id = setTimeout(() => document.addEventListener('click', close, { once: true }), 0);  // ← add once
  return () => { clearTimeout(id); document.removeEventListener('click', close); };
});
```

### 1c — Raise dropdown z-index

In `engine/editor/src/lib/components/ui/dropdown-menu/dropdown-menu-content.svelte`,
replace `z-50` with `z-[200]` (or equivalent inline style `z-index: 200`):

```svelte
<!-- before -->
<DropdownMenuPrimitive.Content class="z-50 …" …>

<!-- after -->
<DropdownMenuPrimitive.Content class="z-[200] …" …>
```

`200` clears all editor panels and the titlebar (`50`). The existing context menu already uses
`z-index: 9999` — dropdown menus at `200` sit below the context menu, which is correct.

Also apply the same z-index fix to `dropdown-menu-sub-content.svelte` (confirmed to exist at
`engine/editor/src/lib/components/ui/dropdown-menu/dropdown-menu-sub-content.svelte`).

---

## Section 2 — Minimize Button Bottom-Alignment

**Current:** The `10×1` SVG dash is vertically centred in the 32px-tall button (appears
at pixel 16). Windows 11 convention places the minimize dash slightly below centre — at
roughly the 55–60% vertical mark — making it feel "grounded" rather than floating.

**Fix:** Add `padding-top` to push the icon below centre, or use `align-items: flex-end`
with explicit padding:

```css
.wc-minimize {
  align-items: flex-end;
  padding-bottom: 10px;   /* positions the 1px dash at 32−10−1 = row 21 ≈ 66% down */
}
```

This overrides the inherited `align-items: center` from `.wc-btn` only for the minimize
button. Maximize and close keep `align-items: center`.

If the implementation finds the padding feels off at other window heights, the alternative is
a fixed `margin-top` on the SVG element:

```svelte
<svg style="margin-top: 6px" width="10" height="1" viewBox="0 0 10 1" aria-hidden="true">
```

Use whichever approach produces the most natural look; both are equivalent.

---

## Section 3 — No Rounded Corners in Fullscreen

**Context:** The app uses a custom frameless window (Tauri `decorations: false`). Rounded
corners are applied by the OS (Windows 11 / macOS) to floating windows. When fullscreen, the
OS removes them automatically — so this is a no-op for most users.

However, if the app's root element has a CSS `border-radius` (e.g., applied for a custom
borderless look), it must be removed in fullscreen. Add a fullscreen state listener in
`engine/editor/src/routes/+layout.svelte` (or `app.css`):

```ts
// +layout.svelte <script>
import { onMount } from 'svelte';
import { getCurrentWindow } from '@tauri-apps/api/window';

onMount(async () => {
  if (typeof window === 'undefined') return;
  const win = await getCurrentWindow();
  // set initial state
  document.documentElement.dataset.fullscreen =
    String(await win.isFullscreen());
  // react to changes
  const unlisten = await win.onResized(async () => {
    document.documentElement.dataset.fullscreen =
      String(await win.isFullscreen());
  });
  return unlisten;
});
```

Then in `app.css`:

```css
/* Remove any rounded corners when fullscreen */
html[data-fullscreen="true"],
html[data-fullscreen="true"] body,
html[data-fullscreen="true"] #app {
  border-radius: 0 !important;
}
```

If no `border-radius` is currently applied to the root, this section can be implemented as
a no-op placeholder CSS rule (harmless). The `onResized` listener is still valuable for
future-proofing and for macOS where corner radius is handled via CSS variable.

---

## Section 4 — Thin Themed Scrollbars

Add to `engine/editor/src/app.css` (or equivalent global stylesheet):

```css
/* ── Webkit scrollbar: thin dark theme ──────────────────────────── */
::-webkit-scrollbar {
  width: 6px;
  height: 6px;
}

::-webkit-scrollbar-track {
  background: transparent;
}

::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.15);
  border-radius: 3px;
}

::-webkit-scrollbar-thumb:hover {
  background: rgba(255, 255, 255, 0.25);
}

::-webkit-scrollbar-corner {
  background: transparent;
}
```

6px matches common IDE panel scrollbars (VS Code, Rider). The thumb is semi-transparent white
on dark backgrounds (matches the editor's dark theme). No track background keeps scrollbars
unobtrusive until hovered.

Firefox equivalent (for completeness, though the editor runs in Tauri's WebView which is
Chromium-based on all platforms):

```css
* {
  scrollbar-width: thin;
  scrollbar-color: rgba(255, 255, 255, 0.15) transparent;
}
```

Both blocks can be added — the webkit rules take precedence in Chromium/WebKit, Firefox
uses the standard property.

---

## Section 5 — Data Flow / Summary

No state changes or IPC additions. All changes are:
- `TitleBar.svelte`: string literals in two exclusion checks + one `{ once: true }` flag + one CSS rule
- `dropdown-menu-content.svelte` (+ sub-content if present): one class change
- `app.css`: scrollbar CSS block (~20 lines) + fullscreen CSS rule
- `+layout.svelte`: Tauri window resize listener (~12 lines)

---

## Section 6 — Error Handling

- `getCurrentWindow()` / `win.isFullscreen()` — both are Tauri API calls; guard with
  `typeof window !== 'undefined'` (already shown above) to prevent SSR errors. If the Tauri API
  is unavailable, the fullscreen state simply never updates (graceful degradation).
- `window_start_drag` is already wrapped in `.catch(() => {})` — no change needed.

---

## Section 7 — Testing

**Playwright tests** (`engine/editor/e2e/editor.spec.ts`):

1. **`File menu opens on click`** — Click `.menu-trigger` with text "File" → menu content
   visible (`.dropdown-menu-content` or equivalent bits-ui rendered element).
2. **`File menu item is clickable`** — Open File menu, click first `[role="menuitem"]` → no
   error thrown, menu closes.
3. **`Repeated open/close does not stack listeners`** — Open and close the panels menu 5 times
   rapidly; assert that the 6th click outside closes the menu (would fail before the `once` fix).

Items 2–3 may be challenging in browser preview mode (no Tauri backend). Mark them as
`test.skip` with `// requires Tauri` if bits-ui menu requires IPC for any item.

---

## Files Touched

| File | Change |
|------|--------|
| `engine/editor/src/lib/components/TitleBar.svelte` | Extend exclusion selectors in `onTitlebarMousedown` + `onTitlebarDblclick`; add `{ once: true }` to `showPanelsMenu` effect; add `.wc-minimize { align-items: flex-end; padding-bottom: 10px }` |
| `engine/editor/src/lib/components/ui/dropdown-menu/dropdown-menu-content.svelte` | Change `z-50` → `z-[200]` |
| `engine/editor/src/lib/components/ui/dropdown-menu/dropdown-menu-sub-content.svelte` | Change `z-50` → `z-[200]` (if file exists) |
| `engine/editor/src/app.css` | Add webkit + Firefox scrollbar CSS; add fullscreen border-radius override |
| `engine/editor/src/routes/+layout.svelte` | **Create new file** — add Tauri window resize listener; set `data-fullscreen` on `<html>` (file does not currently exist) |
