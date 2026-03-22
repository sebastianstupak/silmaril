# Viewport Toolbar Modernisation Design

> **Status:** Approved

---

## Goal

Modernise the viewport toolbar to match the aesthetic of Rider/Unity/Blender:
compact icon-only buttons, better-proportioned icons, refined glass-dark background,
and polished hover/active states. No functional changes — visual only.

## Architecture

Single file change: `engine/editor/src/lib/docking/panels/ViewportPanel.svelte`.
Icon library stays as `@lucide/svelte`. No new components or CSS files needed.

---

## Section 1 — Button & Icon Sizing

**Current:** 26×26 px buttons, 12×12 px icons (icons are 46% of button area — too small, buttons feel blocky).

**Target:** 22×22 px buttons, 14×14 px icons (icons are 64% of button area — fills the space cleanly, matching Rider/Unity proportions).

Change all icon usages from `width={12} height={12}` to `width={14} height={14}`.

Update `.tool-btn`:
```css
.tool-btn {
  width: 22px;
  height: 22px;
}
```

---

## Section 2 — Toolbar Container

**Current:** `padding: 4px 10px; gap: 2px; border-radius: 6px; background: rgba(0,0,0,0.72); border: 1px solid rgba(255,255,255,0.06);`

**Target** (Rider-style dark surface, slightly elevated feel):
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

Key changes:
- Padding tightened: `4px 10px` → `3px 6px`
- Background: near-black (`rgba(0,0,0,0.72)`) → dark surface (`rgba(37,37,37,0.92)`)
- Border subtly brightened: `0.06` → `0.09` opacity
- Border radius: `6px` → `8px` (softer, more modern)
- Drop shadow added for depth/floating feel

---

## Section 3 — Button States

**Current:** default `color: #666`, hover `color: #ccc` + faint border, active `color: #61afef` + blue tint.

**Target** (Rider/VS Code-style icon buttons):

```css
.tool-btn {
  background: none;
  border: 1px solid transparent;
  border-radius: 4px;
  color: rgba(204, 204, 204, 0.55);   /* muted default — Rider's unselected icon look */
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

Changes from current:
- Default icon colour: `#666` → `rgba(204,204,204,0.55)` (light but desaturated — icons are visible but don't compete with content)
- Hover: removes the explicit border flash; uses just a subtle fill
- Active: slightly increased contrast on blue tint background
- `transition` added for smooth 80ms state changes (fast enough to feel snappy, not jarring)
- `:active` (mousedown) pseudo-class added for press feedback

---

## Section 4 — Separators

**Current:** `height: 16px; margin: 0 5px; background: rgba(255,255,255,0.1)`

**Target:**
```css
.toolbar-separator {
  width: 1px;
  height: 14px;
  background: rgba(255, 255, 255, 0.12);
  margin: 0 3px;
  flex-shrink: 0;   /* prevent separator collapsing at narrow viewport widths */
}
```

Changes: height `16px` → `14px` (proportional to smaller buttons), margin `5px` → `3px` (tighter),
separator opacity slightly increased `0.10` → `0.12` for better visibility.

---

## Section 5 — Tooltip

Minor refinement to match the tighter aesthetic:

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

---

## Section 6 — Icon Swap: Scale Tool

**Decision: keep `Maximize2`.** The `Scale` export in lucide-svelte v0.577.0 is a balance/weighing
icon (justice scales) — not a geometry icon. `Scale3D` exists but renders three directional
arrows that can look cluttered at 14px. `Maximize2` (diagonal corner arrows) is widely recognised
as a resize/scale affordance and is used for scale in Blender's toolbar. Keep it.

No icon swaps needed. All current icons (`MousePointer2`, `Move`, `RotateCw`, `Maximize2`,
`Grid2X2`, `Magnet`, `Video`, `ScanLine`, `CirclePlus`) are retained.

**HUD sync:** Line ~801 of `ViewportPanel.svelte` independently hardcodes `Maximize2` for the
scale tool in the HUD overlay. Since the icon is unchanged, the HUD requires no modification and
the `Maximize2` import is unaffected.

---

## Section 7 — Toolbar Group Spacing

**Current:** `gap: 2px` within groups.

**Target:** keep `gap: 2px` within groups (buttons already close enough). No change needed here —
the tighter button size (22px vs 26px) naturally makes groups feel less spread out.

---

## Summary of All Changes (diff view)

| Property | Before | After |
|---|---|---|
| Button size | 26×26 px | 22×22 px |
| Icon size | 12×12 px | 14×14 px |
| Default icon colour | `#666` | `rgba(204,204,204,0.55)` |
| Hover background | `rgba(255,255,255,0.06)` | `rgba(255,255,255,0.07)` |
| Hover border | `rgba(255,255,255,0.12)` | `transparent` |
| Active background | `rgba(97,175,239,0.12)` | `rgba(97,175,239,0.14)` |
| Button transition | none | `80ms ease` |
| Toolbar bg | `rgba(0,0,0,0.72)` | `rgba(37,37,37,0.92)` |
| Toolbar border | `rgba(255,255,255,0.06)` | `rgba(255,255,255,0.09)` |
| Toolbar border-radius | `6px` | `8px` |
| Toolbar box-shadow | none | `0 2px 8px rgba(0,0,0,0.45)` |
| Toolbar padding | `4px 10px` | `3px 6px` |
| Separator height | `16px` | `14px` |
| Separator margin | `0 5px` | `0 3px` |
| Separator opacity | `0.10` | `0.12` |
| Scale icon | `Maximize2` | `Maximize2` (unchanged — `Scale` in lucide is a balance icon) |

---

## Files Touched

| File | Change |
|------|--------|
| `engine/editor/src/lib/docking/panels/ViewportPanel.svelte` | Update `.viewport-toolbar`, `.tool-btn`, `.toolbar-separator`, `:global(.tooltip-content)` CSS; change all icon sizes from 12 to 14; no icon swaps |

---

## Testing

No automated tests needed — this is a pure visual change. Verify manually (or via Playwright
screenshot) that:
- All four transform tools are visible and the active one is clearly highlighted
- Tooltip appears with shortcut on hover
- Toolbar width is not wider than the viewport on a 1280px window
- Icons are legible at 14px size in the 22px button

Existing Playwright tests that assert `button.tool-btn[aria-label="Select"]` etc. are unaffected
(no HTML structure changes, only CSS).
