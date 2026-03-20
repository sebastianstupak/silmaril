# Editor Docking Improvements — Design Spec

**Date:** 2026-03-20
**Scope:** Three targeted fixes to the editor's panel docking system

---

## 1. Gutter-Style Resize Handles

### Problem
Current splitters are 4px transparent bars that flash solid accent-blue on hover. This gives no persistent visual affordance that panels are resizable.

### Design
- **Width/height:** 8px (up from 4px)
- **CSS borders by direction:**
  - `direction="horizontal"` (vertical bar, `col-resize` cursor): `border-left` + `border-right` 1px `--color-border`
  - `direction="vertical"` (horizontal bar, `row-resize` cursor): `border-top` + `border-bottom` 1px `--color-border`
- **Default state:** `--color-bg` background; border lines visible at `--color-border`
- **Hover/drag state:** border lines shift to `--color-accent`; background unchanged
- No bold fill — just a two-line channel

### Files Affected
- `engine/editor/src/lib/docking/DockSplitter.svelte` — CSS only

---

## 2. Multi-Panel Resize Hardening

### Problem
When resizing panels in nested split layouts, more than two panels sometimes change size simultaneously.

### Analysis
Each `DockSplitter` instance uses component-local `$state` for `dragging`, and creates fresh `mousemove`/`mouseup` closures inside each `onMouseDown` call. This means two splitters cannot be mechanically driven at once by normal mouse input. The most likely visual cause is CSS flex redistribution: resizing one panel in a nested split changes the parent container's rendered size, causing flex siblings to visually reflow. The App.svelte bottom-height splitter is also a peer-level DockSplitter sharing the same window listener scope.

### Fix
1. **`stopPropagation` on `mousedown`** — prevents any ancestor element from accidentally starting a second interaction.
2. **Module-level active-splitter mutex** — a `symbol`-based lock (`let activeSplitter: symbol | null = null`) set on `mousedown` and cleared on `mouseup`. Module-level variables in `.svelte` files are shared across all instances of the component in the same JS module scope (the desired behavior). If `activeSplitter` is already set when `mousedown` fires on a second splitter, the event is ignored.
3. **`min-size` CSS clamp on `.dock-split`** — `.dock-split` itself needs `min-width: 0` / `min-height: 0` on its flex children from the outer flex container (`.dock-child` already has these, but the outer container wrapping `.dock-split` nodes may not).

These are defensive hardening measures rather than a targeted root cause fix.

### Files Affected
- `engine/editor/src/lib/docking/DockSplitter.svelte`
- `engine/editor/src/lib/docking/DockContainer.svelte` — CSS flex min-size clamp

---

## 3. Bottom Row Drop — Full Split Support

### Problem
Dragging a panel to the bottom row and dropping it left/right/center destroys the panel. Only `center` is partially handled; all other zones discard the panel after removing it from the source tree.

### Root Cause
`bottomPanel` is typed as `TabsNode` (a flat tab group). `dropPanel()` handles `isBottomPanel + zone === 'center'` but the `left`, `right`, `top`, `bottom` branches are missing — the panel is removed from `root` and never inserted.

### Design: Promote `bottomPanel` to `LayoutNode`

The user expects left/right drops to split the bottom row. This requires changing `bottomPanel` from `TabsNode` to `LayoutNode` (`SplitNode | TabsNode`) — the same type already used by `root`.

**Note on migration:** `TabsNode` already satisfies `LayoutNode` since `LayoutNode = SplitNode | TabsNode`. Old persisted layouts load and work without structural changes; only the TypeScript type annotation changes.

---

### `types.ts`
- Change `EditorLayout.bottomPanel` from `TabsNode` to `LayoutNode`.

---

### `store.ts` — affected functions

**`removePanelFromLayout`** (lines 187–195)
Currently accesses `result.bottomPanel.panels` and `result.bottomPanel.activeTab` directly (assumes `TabsNode`). After promotion these properties may not exist. Replace with the same `removePanelFromNode` + `cleanNode` pattern already used for `root`:
```typescript
// Before (TabsNode-specific):
const bIdx = result.bottomPanel.panels.indexOf(panelId);
// ...

// After (LayoutNode-generic):
result.bottomPanel = removePanelFromNode(result.bottomPanel, panelId);
result.bottomPanel = cleanNode(result.bottomPanel); // cleanNode never returns null
```

**`setActiveTab`** (lines 391–393)
Currently accesses `result.bottomPanel.activeTab` directly. Replace with a tree-walk helper that finds the `TabsNode` at the given path and sets its `activeTab`, mirroring how the `root` path is already handled in the same function.

**`dropPanel`** — extract inline split logic, then extend
The split-insertion logic for `root` (lines 313–336) is currently written inline. Extract it into a private helper `insertPanelIntoTree(tree: LayoutNode, path: number[], zone: DropZone, panelId: string): LayoutNode`. Then use this helper for both `root` and `bottomPanel`:
```typescript
if (isBottomPanel) {
  result.bottomPanel = insertPanelIntoTree(result.bottomPanel, path, zone, panelId);
} else {
  result.root = insertPanelIntoTree(result.root, path, zone, panelId);
}
```

---

### `App.svelte` — affected code

**`addPanelToLayout` handler** (lines 104–106)
Currently pushes directly to `layout.bottomPanel.panels` (TabsNode-specific). Replace with a store mutation that treats `bottomPanel` as a `LayoutNode` — find the first `TabsNode` descendant and push to it, or use `dropPanel` with `zone: 'center'`.

**Bottom panel visibility guard** (line 339)
Currently: `{#if layout.bottomPanel.panels.length > 0}`.
After promotion, `bottomPanel` may be a `SplitNode`. Replace with a helper `hasAnyPanels(node: LayoutNode): boolean` that walks the tree and returns true if any `TabsNode` has panels, or inline the equivalent check.

---

### `DockContainer.svelte`
No prop changes. The bottom `DockContainer` already receives `isBottomPanel` and a layout node — it will render `DockSplitter` children naturally when `bottomPanel` contains a `SplitNode`.

---

## Success Criteria

- [ ] Resize handles show a two-line gutter at rest (`--color-border`); lines turn `--color-accent` on hover/drag
- [ ] Dragging a single splitter resizes exactly two adjacent panels and no others
- [ ] Dragging a panel to the bottom row's left/right zone creates a horizontal split within the bottom row
- [ ] Dragging a panel to the bottom row's center zone adds it as a tab
- [ ] Old `TabsNode` bottom panel layouts in localStorage load and render correctly without errors
- [ ] No regressions in viewport GPU visibility optimization (`setViewportVisible` still fires correctly)
