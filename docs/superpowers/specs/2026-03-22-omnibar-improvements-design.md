# Omnibar Improvements Design

> **Status:** Approved
> **References:** `docs/superpowers/specs/2026-03-20-omnibar-design.md` (approved base spec)

---

## Goal

Four improvements to the omnibar, grounded in the approved base spec:
1. **Wider active modal** — stretch to use available center-column space up to a comfortable max.
2. **Prefix hints** — show `>` / `@` / `#` shortcut hints in the dropdown when input is empty.
3. **Grouped results** — section headers and separators when showing mixed-type results.
4. **Dropdown below titlebar** — position the modal below the titlebar, not overlapping it.

## Architecture

Single file: `engine/editor/src/lib/omnibar/Omnibar.svelte`.
Supporting file: `engine/editor/src/lib/omnibar/providers.ts` (prefix routing, no structural change needed — only the result type may need a `group` field added).
No backend changes.

---

## Section 1 — Wider Active Modal

**Current:** fixed `width: 360px` regardless of available space.

**Target:** responsive width using CSS `clamp()` — expands with the window up to a comfortable
reading width:

```css
.omnibar-active {
  position: fixed;          /* see Section 4 */
  top: calc(var(--titlebar-height, 32px) + 2px);   /* see Section 4 */
  left: 50%;
  transform: translateX(-50%);
  width: clamp(360px, 40vw, 580px);
  z-index: 10000;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.6);
}
```

`clamp(360px, 40vw, 580px)`:
- At 900px window:  `40vw = 360px` → floored at 360px (same as current).
- At 1280px window: `40vw = 512px` → modal is 512px wide.
- At 1600px window: `40vw = 640px` → capped at 580px.
- At 1920px window: `40vw = 768px` → capped at 580px (stops growing; comfortable reading width).

The `40vw` middle value matches the center column of the titlebar grid (`1fr auto 1fr` — center
column takes roughly 30–40% at typical editor widths), so the modal feels anchored to its
natural space without overflowing into panels at the sides.

---

## Section 2 — Prefix Hints When Empty

When the omnibar is active and the input is **empty**, display a hint list instead of search
results. Each hint row explains a prefix shortcut, matching the base spec's prefix routing:

```
┌─────────────────────────────────────────┐
│  🔍  Search commands, entities, assets…  │
├─────────────────────────────────────────┤
│  >   Commands                           │
│  @   Scene entities                     │
│  #   Assets                             │
└─────────────────────────────────────────┘
```

### 2a — Hint data

```ts
const PREFIX_HINTS = [
  { prefix: '>',  label: 'Commands',       description: 'Run editor commands' },
  { prefix: '@',  label: 'Scene entities', description: 'Find entities in the scene' },
  { prefix: '#',  label: 'Assets',         description: 'Find project assets' },
] as const;
```

### 2b — Conditional rendering in template

```svelte
{#if isOpen}
  <div class="omnibar-dropdown">
    <div class="omnibar-input-row"> … </div>

    {#if query.trim() === ''}
      <!-- Hint list -->
      <ul class="omnibar-results">
        {#each PREFIX_HINTS as hint}
          <li class="omnibar-result omnibar-hint"
              role="option"
              onclick={() => { query = hint.prefix; inputEl?.focus(); }}>
            <span class="hint-prefix">{hint.prefix}</span>
            <span class="result-label">{hint.label}</span>
            <span class="result-meta">{hint.description}</span>
          </li>
        {/each}
      </ul>
    {:else}
      <!-- Normal results list (existing code) -->
      <ul class="omnibar-results"> … </ul>
    {/if}
  </div>
{/if}
```

Clicking a hint row types the prefix into the input and focuses it, letting the user continue
typing without needing to know the shortcuts in advance.

### 2c — CSS for hint rows

```css
.omnibar-hint {
  opacity: 0.75;
}

.omnibar-hint:hover {
  opacity: 1;
}

.hint-prefix {
  font-family: monospace;
  font-size: 13px;
  font-weight: 600;
  color: var(--color-accent, #89b4fa);
  min-width: 18px;
  text-align: center;
  flex-shrink: 0;
}
```

---

## Section 3 — Grouped Results with Section Headers

When the query is non-empty and results span multiple source types (commands, entities, assets,
recent), display a section header above each group.

### 3a — Result type (types.ts)

`OmnibarResult` is a **discriminated union** in `engine/editor/src/lib/omnibar/types.ts`.
Add `group?: string` to **each arm** of the union — do not create a new flat interface:

```ts
// In types.ts — add group? to every variant of OmnibarResult
export type OmnibarResult =
  | { kind: 'command'; id: string; label: string; group?: string; meta?: string; keybind?: string; action: () => void | Promise<void>; }
  | { kind: 'entity';  id: string; label: string; group?: string; meta?: string; action: () => void | Promise<void>; }
  | { kind: 'asset';   id: string; label: string; group?: string; meta?: string; action: () => void | Promise<void>; }
  | { kind: 'recent';  id: string; label: string; group?: string; meta?: string; action: () => void | Promise<void>; };
  // (add group? to each existing arm — exact shape depends on current types.ts content)
```

`buildResults()` in `providers.ts` already routes by prefix and returns a flat array. Extend it
to set `group` on each result:

```ts
// commands section
commandResults.forEach(r => { r.group = 'Commands'; });
entityResults.forEach(r =>  { r.group = 'Entities'; });
assetResults.forEach(r =>   { r.group = 'Assets'; });
recentResults.forEach(r =>  { r.group = 'Recent'; });
```

When a prefix filter is active (e.g. `>`), only one group is present — no header is shown
(redundant to display "Commands" header when the `>` prefix already conveys that).
Header visibility rule: **show section headers only when 2+ groups appear in the results.**

### 3b — Rendering grouped results in template

Replace the flat `{#each results}` loop with a grouped render:

```svelte
{#if showGroups}
  {#each groupedResults as group}
    <li class="omnibar-section-header" role="presentation">{group.label}</li>
    {#each group.items as result, i}
      <!-- existing result row -->
    {/each}
  {/each}
{:else}
  {#each results as result, i}
    <!-- existing result row (no header) -->
  {/each}
{/if}
```

`groupedResults` is a `$derived` that groups the flat `results` array by `group` field,
preserving order. `showGroups` is `$derived` as `new Set(results.map(r => r.group)).size > 1`.

### 3c — CSS for section headers

```css
.omnibar-section-header {
  padding: 4px 10px 2px;
  font-size: 10px;
  font-weight: 600;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: var(--color-textDim, #555);
  user-select: none;
  pointer-events: none;
}
```

Section headers are not focusable and not navigable with arrow keys.

**Keyboard navigation (Option A — index into `results[]`):** `selectedIndex` remains an index
into the flat `results` array (matching the existing `onKeydown` in `Omnibar.svelte`). Section
headers are render-only — they are never entered into the index space. Arrow key increments and
decrements operate on `results[]` exactly as today; no DOM inspection or `classList` check is
needed. The grouped `{#if showGroups}` branch renders headers between groups purely for display,
but the selected item is always identified by its position in `results[]`.

---

## Section 4 — Dropdown Positioned Below Titlebar

**Current:** `.omnibar-active { position: absolute; top: 0; … }` — the modal is absolutely
positioned relative to the nearest positioned ancestor (`.titlebar`, `z-index: 50`) and starts
at `top: 0`, which means it overlaps the titlebar itself.

**Target:** modal opens *below* the titlebar, aligned to the titlebar's bottom edge.

**Fix:** change to `position: fixed` with `top` driven by a CSS custom property:

```css
/* TitleBar.svelte — add to .titlebar rule */
.titlebar {
  --titlebar-height: 32px;   /* ← new: single source of truth */
  height: var(--titlebar-height);
  /* … existing rules unchanged … */
}
```

```css
/* Omnibar.svelte — .omnibar-active */
.omnibar-active {
  position: fixed;                                      /* ← was absolute */
  top: calc(var(--titlebar-height, 32px) + 2px);        /* ← drops below titlebar + 2px gap */
  left: 50%;
  transform: translateX(-50%);
  width: clamp(360px, 40vw, 580px);
  z-index: 10000;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.6);
}
```

Using `fixed` (rather than `absolute`) removes the dependency on any ancestor's stacking context
and guarantees the modal always renders relative to the viewport — the same strategy used by
VS Code's command palette and Rider's search bar.

The `--titlebar-height` CSS custom property is set on `.titlebar` so it is accessible to any
child or sibling via the cascade, and provides a single source of truth if the height ever
changes. The `32px` fallback in `var(--titlebar-height, 32px)` ensures the omnibar degrades
gracefully if loaded outside the full editor shell.

The backdrop (`.omnibar-backdrop`) is already `position: fixed; inset: 0` — no change needed.

---

## Section 5 — Idle Pill Width

The idle pill already uses `min-width: 200px; max-width: 320px; width: 100%` which is
responsive within its parent. No change needed to the idle state — the center column of the
titlebar grid (`auto`) constrains it naturally.

If the center column feels too narrow at small window sizes, the titlebar grid template can be
adjusted: `grid-template-columns: minmax(180px, 1fr) auto minmax(180px, 1fr)` ensures the outer
columns don't collapse below 180px and push the omnibar out of view. This is an optional
refinement — include only if testing shows the idle pill gets squeezed below 200px.

---

## Section 6 — Data Flow Summary

```
User presses Ctrl+K or clicks idle pill
  → isOpen = true
  → .omnibar-active renders at fixed top: 34px, width: clamp(360px, 40vw, 580px)

Input is empty
  → PREFIX_HINTS rendered as hint rows
  → User clicks hint → prefix typed into input → normal search begins

Input has text
  → buildResults() called → flat results[] with group field set
  → showGroups derived → if 2+ groups, section headers rendered
  → keyboard navigation skips .omnibar-section-header items

User presses Escape or clicks backdrop
  → isOpen = false → .omnibar-active removed
```

---

## Section 7 — Error Handling

- `buildResults()` is already guarded against empty/null results. The `group` field is optional
  — if a provider doesn't set it, the result appears ungrouped (consistent with current flat
  rendering).
- `clamp()` is supported in all Chromium versions shipped in Tauri v2 — no fallback needed.
- If `top: 34px` causes the modal to be clipped on very small windows (< 400px height), CSS
  `max-height: calc(100vh - 50px)` on `.omnibar-results` handles this gracefully (already
  has `max-height: 280px`; reduce to `min(280px, calc(100vh - 90px))` if needed).

---

## Section 8 — Testing

**Playwright tests** (`engine/editor/e2e/editor.spec.ts`):

1. **`omnibar opens below titlebar on Ctrl+K`** — Press `Ctrl+K`, assert `.omnibar-active` is
   visible and its bounding box top is ≥ 32px from viewport top.
2. **`omnibar shows prefix hints when empty`** — Open omnibar, assert three `.omnibar-hint`
   items are visible with prefixes `>`, `@`, `#`.
3. **`clicking hint types prefix into input`** — Click the `>` hint row, assert input value
   is `>`.
4. **`omnibar width is responsive`** (visual only) — Open omnibar, assert `.omnibar-active`
   width is between 360px and 580px.

---

## Files Touched

| File | Change |
|------|--------|
| `engine/editor/src/lib/components/TitleBar.svelte` | Add `--titlebar-height: 32px` CSS custom property to `.titlebar` rule |
| `engine/editor/src/lib/omnibar/types.ts` | Add `group?: string` to each arm of the `OmnibarResult` discriminated union |
| `engine/editor/src/lib/omnibar/providers.ts` | Set `group` on each result in `buildResults()` |
| `engine/editor/src/lib/omnibar/Omnibar.svelte` | Change `.omnibar-active` to `position: fixed; top: calc(var(--titlebar-height,32px) + 2px); width: clamp(360px, 40vw, 580px)`; add `PREFIX_HINTS` const; add empty-state hint list render; add `showGroups` + `groupedResults` derived; add section header render in results loop (Option A: headers are render-only, selectedIndex stays in results[] space); add `.omnibar-hint`, `.hint-prefix`, `.omnibar-section-header` CSS |
