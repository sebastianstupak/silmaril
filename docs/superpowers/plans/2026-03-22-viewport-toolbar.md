# Viewport Toolbar Modernisation — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Modernise the viewport toolbar to a compact Rider/Unity-style aesthetic — 22×22px buttons with 14×14px icons, refined dark-surface background, polished hover/active states.

**Architecture:** Pure CSS + one icon size change in ViewportPanel.svelte. No HTML structure changes, no new components, no backend changes.

**Tech Stack:** Svelte 5, CSS (scoped component styles), @lucide/svelte

---

## File Under Edit

All changes are in one file:
`engine/editor/src/lib/docking/panels/ViewportPanel.svelte`

---

## Task 1 — Update icon sizes from 12→14 in the toolbar HTML

**Time estimate:** 2–3 minutes

**What:** Change every `width={12} height={12}` icon attribute in the toolbar HTML section (lines 625–724) to `width={14} height={14}`. The HUD section (lines 816–819, 837) uses icons in a different context — leave those unchanged; the spec explicitly confirms no HUD modification is needed.

**Lines to update (toolbar section only):**
- Line 625: `<tool.Icon width={12} height={12} />` (inside `{#each tools}` loop)
- Line 654: `<Grid2X2 width={12} height={12} />`
- Line 671: `<Magnet width={12} height={12} />`
- Line 694: `<ScanLine width={12} height={12} />`
- Line 696: `<Video width={12} height={12} />`
- Line 720: `<CirclePlus width={12} height={12} />`

**Steps:**

- [ ] Open `engine/editor/src/lib/docking/panels/ViewportPanel.svelte`
- [ ] Replace all six icon usages in lines 625–724 — change `width={12} height={12}` to `width={14} height={14}` for:
  - `<tool.Icon width={14} height={14} />`
  - `<Grid2X2 width={14} height={14} />`
  - `<Magnet width={14} height={14} />`
  - `<ScanLine width={14} height={14} />`
  - `<Video width={14} height={14} />`
  - `<CirclePlus width={14} height={14} />`
- [ ] Confirm lines 816–837 (HUD section) are **not** modified — those `width={12} height={12}` usages stay as-is
- [ ] Run: `cd engine/editor && npm run preview` — visually verify toolbar icons are slightly larger, all still render correctly
- [ ] Run: `cd engine/editor && npm run test:e2e` — confirm all tests pass (no HTML structure changes, so all `aria-label` selectors still resolve)
- [ ] Commit: `style(editor): increase toolbar icon size from 12 to 14px`

---

## Task 2 — Update `.viewport-toolbar` container CSS

**Time estimate:** 2 minutes

**What:** Replace the `.viewport-toolbar` CSS block (lines 899–913) with the new spec values: tighter padding, elevated dark surface background, subtle border brightened, larger border-radius, and a drop shadow.

**Current block (lines 899–913):**
```css
.viewport-toolbar {
  position: absolute;
  top: 8px;
  left: 50%;
  transform: translateX(-50%);
  display: flex;
  align-items: center;
  gap: 2px;
  padding: 4px 10px;
  background: rgba(0, 0, 0, 0.72);
  border: 1px solid rgba(255, 255, 255, 0.06);
  border-radius: 6px;
  z-index: 10;
  pointer-events: auto;
}
```

**New block (complete replacement):**
```css
.viewport-toolbar {
  position: absolute;
  top: 8px;
  left: 50%;
  transform: translateX(-50%);
  display: flex;
  align-items: center;
  gap: 2px;
  padding: 3px 6px;
  background: rgba(37, 37, 37, 0.92);
  border: 1px solid rgba(255, 255, 255, 0.09);
  border-radius: 8px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.45);
  z-index: 10;
  pointer-events: auto;
}
```

**Steps:**

- [ ] Replace the `.viewport-toolbar` block at lines 899–913 with the new block above
- [ ] Run: `cd engine/editor && npm run preview` — visually verify toolbar is slightly tighter, has a visible drop shadow, slightly softer corners
- [ ] Run: `cd engine/editor && npm run test:e2e` — confirm all tests still pass
- [ ] Commit: `style(editor): refine viewport-toolbar container — tighter padding, dark surface bg, shadow`

---

## Task 3 — Update `.tool-btn` and all button state CSS

**Time estimate:** 3–4 minutes

**What:** Replace the `.tool-btn`, `.tool-btn:hover`, and `.tool-btn.active` CSS blocks (lines 927–953) with the new spec. This adds button size (22×22), muted default icon colour, smooth 80ms transitions, a `:active` press state, and a `.tool-btn.active:hover` refinement.

**Current blocks (lines 927–953):**
```css
.tool-btn {
  background: none;
  border: 1px solid transparent;
  border-radius: 4px;
  color: #666;
  padding: 0;
  cursor: pointer;
  line-height: 1;
  width: 26px;
  height: 26px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.tool-btn:hover {
  color: #ccc;
  border-color: rgba(255, 255, 255, 0.12);
  background: rgba(255, 255, 255, 0.06);
}

.tool-btn.active {
  color: #61afef;
  border-color: rgba(97, 175, 239, 0.4);
  background: rgba(97, 175, 239, 0.12);
}
```

**New blocks (complete replacement — insert all five rules):**
```css
.tool-btn {
  background: none;
  border: 1px solid transparent;
  border-radius: 4px;
  color: rgba(204, 204, 204, 0.55);
  padding: 0;
  cursor: pointer;
  line-height: 1;
  width: 22px;
  height: 22px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  transition: color 80ms ease, background 80ms ease, border-color 80ms ease;
}

.tool-btn:hover {
  color: rgba(204, 204, 204, 0.9);
  background: rgba(255, 255, 255, 0.07);
  border-color: transparent;
}

.tool-btn:active {
  background: rgba(255, 255, 255, 0.04);
}

.tool-btn.active {
  color: #61afef;
  background: rgba(97, 175, 239, 0.14);
  border-color: rgba(97, 175, 239, 0.35);
}

.tool-btn.active:hover {
  background: rgba(97, 175, 239, 0.2);
  border-color: rgba(97, 175, 239, 0.5);
}
```

**Steps:**

- [ ] Replace the three existing button-state blocks (`.tool-btn`, `.tool-btn:hover`, `.tool-btn.active`) at lines 927–953 with the five new blocks above
- [ ] Run: `cd engine/editor && npm run preview` — visually verify:
  - Buttons are 22×22px (smaller, tighter)
  - Default icon colour is muted (not `#666` grey — slightly lighter but desaturated)
  - Hover produces a subtle fill, no border flash
  - Active (selected) tool button is blue-highlighted
  - Clicking a button shows a brief press feedback (`:active` state)
- [ ] Run: `cd engine/editor && npm run test:e2e` — confirm `.active` class assertions in `viewport.spec.ts` still pass (class name unchanged)
- [ ] Commit: `style(editor): modernise tool-btn states — 22px, muted default, 80ms transition, press state`

---

## Task 4 — Update `.toolbar-separator` CSS

**Time estimate:** 2 minutes

**What:** Replace the `.toolbar-separator` block (lines 920–925) with the new spec values: proportionally shorter height (16→14px), tighter margin (5px→3px), and slightly higher-contrast line (opacity 0.10→0.12). Also add `flex-shrink: 0` to prevent separator collapsing at narrow viewport widths.

**Current block (lines 920–925):**
```css
.toolbar-separator {
  width: 1px;
  height: 16px;
  background: rgba(255, 255, 255, 0.1);
  margin: 0 5px;
}
```

**New block (complete replacement):**
```css
.toolbar-separator {
  width: 1px;
  height: 14px;
  background: rgba(255, 255, 255, 0.12);
  margin: 0 3px;
  flex-shrink: 0;
}
```

**Steps:**

- [ ] Replace the `.toolbar-separator` block at lines 920–925 with the new block above
- [ ] Run: `cd engine/editor && npm run preview` — visually verify separators are slightly shorter and the spacing between groups is tighter
- [ ] Run: `cd engine/editor && npm run test:e2e` — confirm all tests pass
- [ ] Commit: `style(editor): tighten toolbar-separator — 14px height, 3px margin, flex-shrink: 0`

---

## Task 5 — Update tooltip CSS (`:global(.tooltip-content)` and `:global(.tooltip-shortcut)`)

**Time estimate:** 2–3 minutes

**What:** Replace the two global tooltip blocks (lines 955–971) with the spec values. Changes: slightly darker/more opaque background, brightened border, adjusted border-radius (4→5px), line-height added, slightly tighter padding, drop shadow, and `margin-left` on shortcut bumped from 4px to 5px with `font-size: 10px` added.

**Current blocks (lines 955–971):**
```css
:global(.tooltip-content) {
  background: rgba(20, 20, 20, 0.95);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 4px;
  color: #ccc;
  font-size: 11px;
  padding: 4px 8px;
  pointer-events: none;
  white-space: nowrap;
  z-index: 9999;
}

:global(.tooltip-shortcut) {
  color: #888;
  margin-left: 4px;
  font-family: monospace;
}
```

**New blocks (complete replacement):**
```css
:global(.tooltip-content) {
  background: rgba(28, 28, 28, 0.97);
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 5px;
  color: #d4d4d4;
  font-size: 11px;
  line-height: 1.4;
  padding: 3px 7px;
  pointer-events: none;
  white-space: nowrap;
  z-index: 9999;
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.4);
}

:global(.tooltip-shortcut) {
  color: #888;
  margin-left: 5px;
  font-family: monospace;
  font-size: 10px;
}
```

**Steps:**

- [ ] Replace the two tooltip blocks at lines 955–971 with the two new blocks above
- [ ] Run: `cd engine/editor && npm run preview` — hover over a toolbar button and visually verify the tooltip has a slightly darker background, drop shadow, and the shortcut key text is monospace and legible
- [ ] Run: `cd engine/editor && npm run test:e2e` — confirm all tests pass (tooltip selectors in tests use `.tooltip-content` class which is unchanged)
- [ ] Commit: `style(editor): refine tooltip — darker bg, shadow, tighter padding, line-height`

---

## Task 6 — Final E2E verification pass

**Time estimate:** 2 minutes

**What:** Full E2E run across all three test suites to confirm no regressions after all five CSS tasks. This is a safety net — individual tasks already ran tests, but this ensures the complete diff is clean together.

**Steps:**

- [ ] Ensure the dev preview is running: `cd engine/editor && npm run preview`
- [ ] Run the full E2E suite: `cd engine/editor && npm run test:e2e`
  - Expected: all tests in `e2e/viewport.spec.ts`, `e2e/editor.spec.ts`, and `e2e/panels-registry.spec.ts` pass
  - Key assertions to confirm visually in test output:
    - `button.tool-btn[aria-label="Select"]` — resolves (HTML unchanged)
    - `button.tool-btn[aria-label="Move"]` — resolves
    - `button.tool-btn[aria-label="Rotate"]` — resolves
    - `button.tool-btn[aria-label="Scale"]` — resolves
    - `.tool-btn.active` class assertions — pass (class name unchanged, only CSS values differ)
    - `button[aria-label="Perspective"]` / `button[aria-label="Orthographic"]` — resolves
- [ ] If any test fails: check that no HTML `aria-label`, class name, or element structure was accidentally modified; revert only the offending CSS property
- [ ] If all tests pass: no additional commit needed (all changes already committed per-task above)

---

## Summary of All Property Changes

| Property | Before | After | Task |
|---|---|---|---|
| Icon size (toolbar) | `width={12} height={12}` | `width={14} height={14}` | Task 1 |
| Button size | `26×26 px` | `22×22 px` | Task 3 |
| Default icon colour | `#666` | `rgba(204,204,204,0.55)` | Task 3 |
| Hover background | `rgba(255,255,255,0.06)` | `rgba(255,255,255,0.07)` | Task 3 |
| Hover border | `rgba(255,255,255,0.12)` | `transparent` | Task 3 |
| Active background | `rgba(97,175,239,0.12)` | `rgba(97,175,239,0.14)` | Task 3 |
| Active border | `rgba(97,175,239,0.4)` | `rgba(97,175,239,0.35)` | Task 3 |
| Button transition | none | `80ms ease` on color/bg/border | Task 3 |
| `:active` press state | none | `rgba(255,255,255,0.04)` bg | Task 3 |
| `.active:hover` state | none | enhanced blue highlight | Task 3 |
| Toolbar padding | `4px 10px` | `3px 6px` | Task 2 |
| Toolbar background | `rgba(0,0,0,0.72)` | `rgba(37,37,37,0.92)` | Task 2 |
| Toolbar border opacity | `0.06` | `0.09` | Task 2 |
| Toolbar border-radius | `6px` | `8px` | Task 2 |
| Toolbar box-shadow | none | `0 2px 8px rgba(0,0,0,0.45)` | Task 2 |
| Separator height | `16px` | `14px` | Task 4 |
| Separator margin | `0 5px` | `0 3px` | Task 4 |
| Separator opacity | `0.10` | `0.12` | Task 4 |
| Separator flex-shrink | not set | `flex-shrink: 0` | Task 4 |
| Tooltip background | `rgba(20,20,20,0.95)` | `rgba(28,28,28,0.97)` | Task 5 |
| Tooltip border opacity | `0.10` | `0.12` | Task 5 |
| Tooltip border-radius | `4px` | `5px` | Task 5 |
| Tooltip colour | `#ccc` | `#d4d4d4` | Task 5 |
| Tooltip line-height | not set | `1.4` | Task 5 |
| Tooltip padding | `4px 8px` | `3px 7px` | Task 5 |
| Tooltip box-shadow | none | `0 2px 6px rgba(0,0,0,0.4)` | Task 5 |
| Tooltip shortcut margin | `4px` | `5px` | Task 5 |
| Tooltip shortcut font-size | not set | `10px` | Task 5 |

---

## Files Touched

| File | Lines Modified | Change Type |
|------|---------------|-------------|
| `engine/editor/src/lib/docking/panels/ViewportPanel.svelte` | 625, 654, 671, 694, 696, 720 | Icon size attributes (HTML) |
| `engine/editor/src/lib/docking/panels/ViewportPanel.svelte` | 899–913 | `.viewport-toolbar` CSS block |
| `engine/editor/src/lib/docking/panels/ViewportPanel.svelte` | 920–925 | `.toolbar-separator` CSS block |
| `engine/editor/src/lib/docking/panels/ViewportPanel.svelte` | 927–953 | `.tool-btn` CSS blocks (all states) |
| `engine/editor/src/lib/docking/panels/ViewportPanel.svelte` | 955–971 | `:global(.tooltip-content)` + `:global(.tooltip-shortcut)` CSS |

No other files are modified. The HUD icon usages at lines 816–819 and 837 are intentionally left at `width={12} height={12}`.
