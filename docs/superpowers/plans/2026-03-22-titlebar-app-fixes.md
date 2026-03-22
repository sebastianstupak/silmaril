# Title Bar / App Chrome Fixes — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix dropdown menu open/click bugs, align minimize button dash, remove rounded corners in fullscreen, and add thin dark-themed scrollbars.

**Architecture:** Three files in the Svelte frontend (TitleBar.svelte, dropdown-menu-content.svelte, app.css) plus a new +layout.svelte for fullscreen detection. No backend changes.

**Tech Stack:** Svelte 5, Tauri v2 API (@tauri-apps/api/window), bits-ui, TypeScript, CSS

---

## Task 1 — Fix `onTitlebarMousedown` + `onTitlebarDblclick` exclusion selectors

**File:** `engine/editor/src/lib/components/TitleBar.svelte`
**Lines:** 185–193

**Root cause:** Both handlers call `invoke('window_start_drag')` / `invoke('window_toggle_maximize')`
for any mousedown/dblclick that does not land on a known interactive element. The `.titlebar-menus`
container and the bits-ui dropdown elements are absent from the exclusion selector. When a user
mousedowns a `DropdownMenu.Trigger`, `window_start_drag` fires first; the OS drag swallows
subsequent pointer events and the menu never opens.

**Before (lines 185–193):**
```ts
function onTitlebarMousedown(e: MouseEvent) {
  if (e.button !== 0) return;
  if ((e.target as HTMLElement).closest('button, input, .slot-wrapper, .overflow-wrapper, .panels-btn-wrapper, .omnibar-wrapper')) return;
  invoke('window_start_drag').catch(() => {});
}

function onTitlebarDblclick(e: MouseEvent) {
  if ((e.target as HTMLElement).closest('button, input, .slot-wrapper, .overflow-wrapper, .panels-btn-wrapper, .omnibar-wrapper')) return;
  invoke('window_toggle_maximize').catch(() => {});
}
```

**After:**
```ts
function onTitlebarMousedown(e: MouseEvent) {
  if (e.button !== 0) return;
  if ((e.target as HTMLElement).closest(
    'button, input, .slot-wrapper, .overflow-wrapper, ' +
    '.panels-btn-wrapper, .omnibar-wrapper, ' +
    '.titlebar-menus, [data-dropdown-menu-content], ' +
    '[data-dropdown-menu-item], ' +
    '[data-dropdown-menu-trigger][data-state="open"]'
  )) return;
  invoke('window_start_drag').catch(() => {});
}

function onTitlebarDblclick(e: MouseEvent) {
  if ((e.target as HTMLElement).closest(
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
- `.titlebar-menus` — the wrapper `<div class="titlebar-menus">` at line 221 that contains all
  `DropdownMenu.Root` elements. Blocking the entire container prevents drag on any click within
  the menu bar region.
- `[data-dropdown-menu-content]` / `[data-dropdown-menu-item]` — actual DOM `data-slot` attributes
  set by bits-ui (the rendered elements carry `data-slot="dropdown-menu-content"` and
  `data-slot="dropdown-menu-item"`). Note: the spec references `data-dropdown-menu-*` selectors;
  inspect the rendered DOM in devtools to confirm the exact attribute name if bits-ui uses a
  different convention (e.g., `data-bits-*`). Adjust the selector string to match.
- `[data-dropdown-menu-trigger][data-state="open"]` — scoped to open triggers only, avoiding
  the overly broad `[data-state="open"]` that bits-ui applies to many other components.

**Steps:**
- [ ] Open `engine/editor/src/lib/components/TitleBar.svelte`
- [ ] Replace lines 185–193 with the "After" block above
- [ ] Verify the file still compiles: `cd engine/editor && npx tsc --noEmit` (or `bun run check`)

**Manual test (requires running Tauri app):**
- Launch the editor (`cargo tauri dev` from `engine/editor`)
- Click the "File" menu trigger — menu must open without triggering a drag
- Double-click empty titlebar space — maximize should still toggle
- Double-click the "Edit" text — maximize must NOT toggle

**Playwright test (browser preview, no Tauri IPC):**
```ts
// engine/editor/e2e/editor.spec.ts
test('File menu opens on click', async ({ page }) => {
  await page.goto('http://localhost:5173');
  await page.click('.menu-trigger:has-text("File")');
  await expect(page.locator('[data-slot="dropdown-menu-content"]')).toBeVisible();
});
```

---

## Task 2 — Fix stacked click listeners in `showPanelsMenu` effect

**File:** `engine/editor/src/lib/components/TitleBar.svelte`
**Line:** 180

**Root cause:** `document.addEventListener('click', close)` is called without `{ once: true }`.
Every open-then-close cycle adds a permanent listener. After a few cycles, many stacked listeners
fire simultaneously on the next click; each triggers `showPanelsMenu = false`, collapsing menus
before item `onclick` handlers can fire.

Contrast with the already-correct patterns at lines 105 and 155 (`contextMenu` and `showOverflow`
effects), which both pass `{ once: true }`.

**Before (line 180):**
```ts
$effect(() => {
  if (!showPanelsMenu) return;
  function close(e: MouseEvent) {
    if (!(e.target as HTMLElement).closest('.panels-btn-wrapper')) showPanelsMenu = false;
  }
  const id = setTimeout(() => document.addEventListener('click', close), 0);
  return () => { clearTimeout(id); document.removeEventListener('click', close); };
});
```

**After:**
```ts
$effect(() => {
  if (!showPanelsMenu) return;
  function close(e: MouseEvent) {
    if (!(e.target as HTMLElement).closest('.panels-btn-wrapper')) showPanelsMenu = false;
  }
  const id = setTimeout(() => document.addEventListener('click', close, { once: true }), 0);
  return () => { clearTimeout(id); document.removeEventListener('click', close); };
});
```

The only change is adding `{ once: true }` as the third argument to `addEventListener`.

**Steps:**
- [ ] Open `engine/editor/src/lib/components/TitleBar.svelte`
- [ ] Find line 180: `const id = setTimeout(() => document.addEventListener('click', close), 0);`
- [ ] Replace that line with: `const id = setTimeout(() => document.addEventListener('click', close, { once: true }), 0);`
- [ ] Verify: `bun run check` passes

**Manual test:**
- Open the panels menu (click the panels button in the titlebar)
- Close it by clicking elsewhere
- Repeat 5 times
- On the 6th open, click a panel item — it must respond; the menu must not close before the item fires

**Playwright test:**
```ts
// engine/editor/e2e/editor.spec.ts
test('Panels menu closes cleanly after repeated open/close', async ({ page }) => {
  await page.goto('http://localhost:5173');
  // Open and close 5 times
  for (let i = 0; i < 5; i++) {
    await page.click('[data-testid="panels-btn"]');        // adjust selector as needed
    await page.click('body', { position: { x: 10, y: 10 } });
  }
  // 6th open — menu should still be functional
  await page.click('[data-testid="panels-btn"]');
  await expect(page.locator('.panels-btn-wrapper')).toBeVisible();
});
```

---

## Task 3 — Fix dropdown z-index

**Files:**
- `engine/editor/src/lib/components/ui/dropdown-menu/dropdown-menu-content.svelte` (line 18)
- `engine/editor/src/lib/components/ui/dropdown-menu/dropdown-menu-sub-content.svelte` (line 16)

**Root cause:** Both components use Tailwind class `z-50` (resolves to `z-index: 50`). The
`.titlebar` element also uses `z-index: 50`. If the dropdown portal renders within the titlebar
stacking context, the content sits at the same z-level and may be clipped or covered by adjacent
panels that also use high z-indices.

**Before — dropdown-menu-content.svelte line 18 (class string, abbreviated):**
```
"... z-50 max-h-(--bits-dropdown-menu-content-available-height) ..."
```

**After — dropdown-menu-content.svelte:**
```
"... z-[200] max-h-(--bits-dropdown-menu-content-available-height) ..."
```

**Before — dropdown-menu-sub-content.svelte line 16 (class string, abbreviated):**
```
"... z-50 min-w-[8rem] ..."
```

**After — dropdown-menu-sub-content.svelte:**
```
"... z-[200] min-w-[8rem] ..."
```

`z-index: 200` clears the titlebar (`50`) and all editor panels. The existing context menu in
TitleBar.svelte uses `z-index: 9999` (line 1057) — dropdown menus at `200` sit below it, which
is the correct stacking order.

**Steps:**

For `dropdown-menu-content.svelte`:
- [ ] Open `engine/editor/src/lib/components/ui/dropdown-menu/dropdown-menu-content.svelte`
- [ ] On line 18, replace the first occurrence of `z-50` with `z-[200]` in the class string
- [ ] Full updated line 18 class string:
  ```
  "bg-popover text-popover-foreground data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 data-[side=bottom]:slide-in-from-top-2 data-[side=left]:slide-in-from-end-2 data-[side=right]:slide-in-from-start-2 data-[side=top]:slide-in-from-bottom-2 z-[200] max-h-(--bits-dropdown-menu-content-available-height) min-w-[8rem] origin-(--bits-dropdown-menu-content-transform-origin) overflow-x-hidden overflow-y-auto rounded-md border p-1 shadow-md outline-none"
  ```

For `dropdown-menu-sub-content.svelte`:
- [ ] Open `engine/editor/src/lib/components/ui/dropdown-menu/dropdown-menu-sub-content.svelte`
- [ ] On line 16, replace the first occurrence of `z-50` with `z-[200]` in the class string
- [ ] Full updated line 16 class string:
  ```
  "bg-popover text-popover-foreground data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 data-[side=bottom]:slide-in-from-top-2 data-[side=left]:slide-in-from-end-2 data-[side=right]:slide-in-from-start-2 data-[side=top]:slide-in-from-bottom-2 z-[200] min-w-[8rem] origin-(--bits-dropdown-menu-content-transform-origin) overflow-hidden rounded-md border p-1 shadow-lg"
  ```

**Steps (continued):**
- [ ] Run `bun run check` to verify no type errors

**Manual test:**
- Open any titlebar menu — content must appear on top of all panels, never behind them
- Hover over "Recent Projects" (sub-menu) — sub-content must also appear on top

**Playwright test:**
```ts
// engine/editor/e2e/editor.spec.ts
test('File menu item is clickable', async ({ page }) => {
  await page.goto('http://localhost:5173');
  await page.click('.menu-trigger:has-text("File")');
  const menuContent = page.locator('[data-slot="dropdown-menu-content"]');
  await expect(menuContent).toBeVisible();
  // Verify z-index is applied (computed style)
  const zIndex = await menuContent.evaluate(el =>
    getComputedStyle(el).zIndex
  );
  expect(Number(zIndex)).toBeGreaterThanOrEqual(200);
});
```

**Commit after Tasks 1–3:**
```
git add engine/editor/src/lib/components/TitleBar.svelte \
        engine/editor/src/lib/components/ui/dropdown-menu/dropdown-menu-content.svelte \
        engine/editor/src/lib/components/ui/dropdown-menu/dropdown-menu-sub-content.svelte
git commit -m "fix(editor): fix dropdown menu open/click bugs — exclusion selectors, once listener, z-index"
```

---

## Task 4 — Fix minimize button alignment

**File:** `engine/editor/src/lib/components/TitleBar.svelte`
**Insert after:** line 1052 (the `.wc-close:hover` rule)

**Context:** The `.wc-btn` rule (lines 1038–1050) sets `align-items: center` for all three
window control buttons. Windows 11 convention places the minimize dash slightly below centre
(roughly the 55–60% vertical mark). The fix adds a `.wc-minimize` override.

**Before (lines 1036–1052):**
```css
/* ── Window controls ─────────────────────────────────────────────────────── */
.window-controls { display: flex; align-items: stretch; height: 100%; }
.wc-btn {
  width: 46px;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  background: none;
  border: none;
  color: var(--color-textDim, #666);
  cursor: default;
  transition: background 0.1s, color 0.1s;
  padding: 0;
}
.wc-btn:hover { background: rgba(255,255,255,0.08); color: var(--color-textMuted, #999); }
.wc-close:hover { background: #c42b1c; color: #fff; }
```

**After (add one rule after `.wc-close:hover`):**
```css
/* ── Window controls ─────────────────────────────────────────────────────── */
.window-controls { display: flex; align-items: stretch; height: 100%; }
.wc-btn {
  width: 46px;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  background: none;
  border: none;
  color: var(--color-textDim, #666);
  cursor: default;
  transition: background 0.1s, color 0.1s;
  padding: 0;
}
.wc-btn:hover { background: rgba(255,255,255,0.08); color: var(--color-textMuted, #999); }
.wc-close:hover { background: #c42b1c; color: #fff; }
.wc-minimize { align-items: flex-end; padding-bottom: 10px; }
```

With a 32 px tall button, `flex-end` + `padding-bottom: 10px` places the 1 px dash at row
`32 − 10 − 1 = 21`, which is 65.6% down the button — matching the Windows 11 convention.
Maximize and close keep `align-items: center` unchanged.

If during visual review the position looks slightly off (e.g., at non-standard DPI or window
height), the alternative is to add `style="margin-top: 6px"` directly to the `<svg>` element
inside the minimize button instead of using the CSS rule. Use whichever looks correct.

**Steps:**
- [ ] Open `engine/editor/src/lib/components/TitleBar.svelte`
- [ ] Find line 1052: `.wc-close:hover { background: #c42b1c; color: #fff; }`
- [ ] Insert a new line immediately after: `.wc-minimize { align-items: flex-end; padding-bottom: 10px; }`
- [ ] Verify the minimize button has class `wc-minimize` in the template (search for `wc-minimize` in the file to confirm the class is already applied to the minimize `<button>`)
- [ ] Build and visually inspect: the dash must appear in the lower third of the button

**Manual test:**
- Launch the editor
- Observe the minimize (−) button: the dash must sit noticeably below centre, matching the
  Windows 11 taskbar and title bar conventions

**Playwright test (visual only — no assertion, just a screenshot for review):**
```ts
test('Minimize button dash is below centre', async ({ page }) => {
  await page.goto('http://localhost:5173');
  await page.screenshot({ path: 'e2e/screenshots/titlebar-minimize.png', clip: { x: 0, y: 0, width: 200, height: 40 } });
  // Review screenshot manually — no automated assertion for pixel position
});
```

**Commit:**
```
git add engine/editor/src/lib/components/TitleBar.svelte
git commit -m "fix(editor): bottom-align minimize button dash per Windows 11 convention"
```

---

## Task 5 — Add fullscreen border-radius handling

### 5a — Create `+layout.svelte`

**File:** `engine/editor/src/routes/+layout.svelte` (NEW FILE — does not currently exist)

The file does not exist yet. Create it with the following content:

```svelte
<script lang="ts">
  import { onMount } from 'svelte';

  let { children } = $props();

  onMount(async () => {
    if (typeof window === 'undefined') return;
    // Guard: only run inside Tauri
    const tauriWindow = await import('@tauri-apps/api/window').catch(() => null);
    if (!tauriWindow) return;

    const win = tauriWindow.getCurrentWindow();

    // Set initial state
    document.documentElement.dataset.fullscreen = String(await win.isFullscreen());

    // React to resize events (fullscreen toggle changes the window size)
    const unlisten = await win.onResized(async () => {
      document.documentElement.dataset.fullscreen = String(await win.isFullscreen());
    });

    return unlisten;
  });
</script>

{@render children()}
```

**Notes:**
- The dynamic `import('@tauri-apps/api/window').catch(() => null)` guard prevents a crash when
  the page is loaded in a plain browser (e.g., during `bun run dev` without Tauri). If the import
  fails (non-Tauri environment), `data-fullscreen` is simply never set and the CSS rule below has
  no effect.
- `onMount` returns `unlisten` which is called on component destroy — no listener leak.
- `{@render children()}` is the Svelte 5 way to render nested routes in a layout.

**Steps:**
- [ ] Create `engine/editor/src/routes/+layout.svelte` with the content above
- [ ] Verify SvelteKit picks up the new layout: run `bun run dev` and check the browser console
      for errors; the app must render normally
- [ ] Check that `document.documentElement.dataset.fullscreen` exists in the DOM (devtools) when
      running inside Tauri (`cargo tauri dev`)

### 5b — Add CSS rule to `app.css`

**File:** `engine/editor/src/app.css`
**Insert after:** line 48 (end of the `html, body { ... }` block)

**Before (lines 41–48):**
```css
html, body {
  margin: 0;
  padding: 0;
  height: 100%;
  overflow: hidden;
  font-family: var(--font-body, system-ui, -apple-system, sans-serif);
  font-size: 13px;
}
```

**After:**
```css
html, body {
  margin: 0;
  padding: 0;
  height: 100%;
  overflow: hidden;
  font-family: var(--font-body, system-ui, -apple-system, sans-serif);
  font-size: 13px;
}

/* Remove rounded corners when the Tauri window is fullscreen */
html[data-fullscreen="true"],
html[data-fullscreen="true"] body,
html[data-fullscreen="true"] #app {
  border-radius: 0 !important;
}
```

**Steps:**
- [ ] Open `engine/editor/src/app.css`
- [ ] Insert the CSS block after line 48

**Manual test:**
- In the Tauri app, press F11 or the OS fullscreen shortcut
- The window corners must be perfectly square (no rounded clipping)
- Restore windowed mode — corners return to whatever the OS applies

**Playwright test:** Not applicable — fullscreen detection requires Tauri IPC. Mark as manual-only.

**Commit:**
```
git add engine/editor/src/routes/+layout.svelte \
        engine/editor/src/app.css
git commit -m "feat(editor): add fullscreen border-radius fix via Tauri onResized listener"
```

---

## Task 6 — Add thin scrollbar CSS

**File:** `engine/editor/src/app.css`
**Insert after:** the fullscreen rule added in Task 5b (after line ~58 post-edit)

**Before:** (end of file after Task 5b additions)

**After (append to `app.css`):**
```css
/* ── Webkit scrollbar: thin dark theme ──────────────────────────────────── */
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

/* Firefox (standard property — no-op in Tauri's Chromium WebView, harmless) */
* {
  scrollbar-width: thin;
  scrollbar-color: rgba(255, 255, 255, 0.15) transparent;
}
```

**Design rationale:**
- `6px` width matches VS Code and Rider panel scrollbars.
- Semi-transparent white thumb (`rgba(255,255,255,0.15)`) is unobtrusive on the dark `#1e1e1e`
  background; brightens to `0.25` on hover.
- No track background — scrollbar region is invisible until scrolling occurs.
- Firefox `scrollbar-width: thin` is included for completeness; it is a no-op in Tauri's Chromium
  WebView but harmless and future-proof.

**Steps:**
- [ ] Open `engine/editor/src/app.css`
- [ ] Append the CSS block above at the end of the file
- [ ] In `bun run dev`, open any panel with overflow content (e.g., the scene hierarchy with many
      entities) and verify the scrollbar is thin and dark-themed

**Playwright test:**
```ts
test('Scrollbar is thin (6px wide)', async ({ page }) => {
  await page.goto('http://localhost:5173');
  // Inject a scrollable div to test the scrollbar style
  await page.evaluate(() => {
    const div = document.createElement('div');
    div.style.cssText = 'width:200px;height:100px;overflow-y:scroll;position:fixed;top:0;right:0;';
    div.innerHTML = '<div style="height:400px"></div>';
    div.id = 'scrollbar-test';
    document.body.appendChild(div);
  });
  const scrollbarWidth = await page.evaluate(() => {
    const el = document.getElementById('scrollbar-test')!;
    return el.offsetWidth - el.clientWidth;
  });
  expect(scrollbarWidth).toBeLessThanOrEqual(8); // 6px + rounding tolerance
});
```

**Commit:**
```
git add engine/editor/src/app.css
git commit -m "feat(editor): add thin dark-themed webkit scrollbars globally"
```

---

## Implementation Order Summary

| # | Task | File(s) | Risk |
|---|------|---------|------|
| 1 | Exclusion selector fix | TitleBar.svelte | Low — string edit only |
| 2 | `{ once: true }` fix | TitleBar.svelte | Low — single arg addition |
| 3 | z-index `z-50` → `z-[200]` | dropdown-menu-content.svelte, dropdown-menu-sub-content.svelte | Low — Tailwind class swap |
| 4 | Minimize button alignment | TitleBar.svelte (CSS) | Low — additive CSS rule |
| 5 | Fullscreen border-radius | +layout.svelte (new), app.css | Medium — new file, Tauri API |
| 6 | Thin scrollbars | app.css | Low — additive CSS |

Tasks 1–3 address the same dropdown bug family and should be implemented and tested together.
Tasks 4, 5, 6 are independent and can be done in any order.

## Files Modified

| File | Change Type |
|------|-------------|
| `engine/editor/src/lib/components/TitleBar.svelte` | Edit — Tasks 1, 2, 4 |
| `engine/editor/src/lib/components/ui/dropdown-menu/dropdown-menu-content.svelte` | Edit — Task 3 |
| `engine/editor/src/lib/components/ui/dropdown-menu/dropdown-menu-sub-content.svelte` | Edit — Task 3 |
| `engine/editor/src/app.css` | Edit — Tasks 5b, 6 |
| `engine/editor/src/routes/+layout.svelte` | **Create** — Task 5a |
