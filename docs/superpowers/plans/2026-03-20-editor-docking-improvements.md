# Editor Docking Improvements Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix gutter-style resize handles, harden multi-splitter drag, and enable full split/tab drops into the bottom panel row.

**Architecture:** Three independent fixes applied to the Svelte 5 docking system. Task 1 and 2 are pure CSS/JS changes to `DockSplitter.svelte`. Task 3 promotes `bottomPanel` from a flat `TabsNode` to a full `LayoutNode` tree, requiring coordinated changes across `types.ts`, `store.ts`, `DockContainer.svelte`, and `App.svelte`.

**Tech Stack:** Svelte 5 (runes), TypeScript, Vite, Vitest

---

## File Map

| File | Change |
|------|--------|
| `engine/editor/src/lib/docking/DockSplitter.svelte` | CSS gutter style + mousedown mutex |
| `engine/editor/src/lib/docking/types.ts` | `bottomPanel: TabsNode` → `LayoutNode` |
| `engine/editor/src/lib/docking/store.ts` | Fix `removePanelFromLayout`, `setActiveTab`, `resizeSplit`; extract `insertPanelIntoTree`; extend `dropPanel` |
| `engine/editor/src/lib/docking/DockContainer.svelte` | Pass `isBottomPanel` to child containers in split nodes |
| `engine/editor/src/App.svelte` | Fix `addPanelToLayout` zone=bottom branch; fix visibility guard |
| `engine/editor/src/lib/docking/store.test.ts` | New — unit tests for store mutations |

---

## Task 1: Gutter-Style Resize Handles

**Files:**
- Modify: `engine/editor/src/lib/docking/DockSplitter.svelte`

- [ ] **Step 1: Open the file and read the current CSS block**

  Read `engine/editor/src/lib/docking/DockSplitter.svelte` to confirm the current `<style>` block (lines 47–66).

- [ ] **Step 2: Replace the style block**

  Replace the entire `<style>` block with:

  ```css
  <style>
    .dock-splitter {
      flex-shrink: 0;
      background: var(--color-bg, #1e1e1e);
      z-index: 10;
      position: relative;
      box-sizing: border-box;
      transition: border-color 0.15s;
    }
    .dock-splitter.horizontal {
      width: 8px;
      cursor: col-resize;
      border-left: 1px solid var(--color-border, #404040);
      border-right: 1px solid var(--color-border, #404040);
    }
    .dock-splitter.vertical {
      height: 8px;
      cursor: row-resize;
      border-top: 1px solid var(--color-border, #404040);
      border-bottom: 1px solid var(--color-border, #404040);
    }
    .dock-splitter:hover,
    .dock-splitter.dragging {
      border-color: var(--color-accent, #007acc);
    }
  </style>
  ```

- [ ] **Step 3: Start the dev server and visually verify**

  ```bash
  cd engine/editor && npm run dev
  ```

  Open the editor in a browser. Confirm:
  - Splitters between panels show as a subtle two-line channel (not a bold line)
  - Hovering a splitter turns the border lines blue (accent)
  - Dragging still works correctly

- [ ] **Step 4: Commit**

  ```bash
  cd engine/editor
  git add src/lib/docking/DockSplitter.svelte
  git commit -m "feat(editor): gutter-style resize handles with two-line channel"
  ```

---

## Task 2: Multi-Splitter Drag Hardening

**Files:**
- Modify: `engine/editor/src/lib/docking/DockSplitter.svelte`

- [ ] **Step 1: Add the module-level mutex and stopPropagation**

  In `DockSplitter.svelte`, replace the entire `<script>` block with:

  ```ts
  <script lang="ts">
    // Module-level mutex: only one DockSplitter instance may be dragging at a time.
    // Module-level variables in .svelte files are shared across all instances
    // in the same JS module scope, which is the intended behavior here.
    let activeSplitter: symbol | null = null;

    interface Props {
      direction: 'horizontal' | 'vertical';
      onResize: (deltaPx: number) => void;
    }

    let { direction, onResize }: Props = $props();
    let dragging = $state(false);

    function onMouseDown(e: MouseEvent) {
      // Guard: ignore if another splitter is already dragging
      if (activeSplitter !== null) return;

      e.preventDefault();
      e.stopPropagation();

      const id = Symbol();
      activeSplitter = id;
      dragging = true;

      let lastX = e.clientX;
      let lastY = e.clientY;

      function onMove(e: MouseEvent) {
        const dx = e.clientX - lastX;
        const dy = e.clientY - lastY;
        lastX = e.clientX;
        lastY = e.clientY;
        onResize(direction === 'horizontal' ? dx : dy);
      }

      function onMouseUp() {
        if (activeSplitter === id) activeSplitter = null;
        dragging = false;
        window.removeEventListener('mousemove', onMove);
        window.removeEventListener('mouseup', onMouseUp);
        document.body.style.cursor = '';
        document.body.style.userSelect = '';
      }

      document.body.style.cursor = direction === 'horizontal' ? 'col-resize' : 'row-resize';
      document.body.style.userSelect = 'none';
      window.addEventListener('mousemove', onMove);
      window.addEventListener('mouseup', onMouseUp);
    }
  </script>
  ```

- [ ] **Step 2: Verify in browser**

  Rapidly click different splitters in quick succession — only the first one should resize. Resize a splitter in a nested split — only the two adjacent panels should move.

- [ ] **Step 3: Commit**

  ```bash
  cd engine/editor
  git add src/lib/docking/DockSplitter.svelte
  git commit -m "fix(editor): mutex prevents concurrent multi-splitter drag"
  ```

---

## Task 3: Promote `bottomPanel` to `LayoutNode` — Types

**Files:**
- Modify: `engine/editor/src/lib/docking/types.ts`

- [ ] **Step 1: Change the type annotation**

  In `types.ts` line 25, change:
  ```ts
  // Before:
  bottomPanel: TabsNode;
  // After:
  bottomPanel: LayoutNode;
  ```

  Full updated interface:
  ```ts
  export interface EditorLayout {
    root: LayoutNode;
    bottomPanel: LayoutNode;
  }
  ```

- [ ] **Step 2: Run TypeScript compiler to see all breakage**

  ```bash
  cd engine/editor && npx tsc --noEmit 2>&1 | head -60
  ```

  Expected: TypeScript errors in `store.ts` and `App.svelte` pointing at `TabsNode`-specific property accesses on `bottomPanel`. This is expected — subsequent tasks fix them one by one.

- [ ] **Step 3: Commit the type change alone**

  ```bash
  cd engine/editor
  git add src/lib/docking/types.ts
  git commit -m "refactor(editor): promote bottomPanel type from TabsNode to LayoutNode"
  ```

---

## Task 4: Fix `store.ts` — Write Tests First

**Files:**
- Create: `engine/editor/src/lib/docking/store.test.ts`
- Modify: `engine/editor/src/lib/docking/store.ts`

These tests will fail until the store fixes in Task 5 are applied.

- [ ] **Step 1: Create the test file**

  Create `engine/editor/src/lib/docking/store.test.ts`:

  ```ts
  import { describe, it, expect } from 'vitest';
  import type { EditorLayout, TabsNode } from './types';
  import {
    removePanelFromLayout,
    dropPanel,
    setActiveTab,
    resizeSplit,
  } from './store';

  // Minimal layout with a split bottom panel (the new supported shape)
  function makeLayoutWithSplitBottom(): EditorLayout {
    return {
      root: { type: 'tabs', activeTab: 0, panels: ['hierarchy'] },
      bottomPanel: {
        type: 'split',
        direction: 'horizontal',
        sizes: [50, 50],
        children: [
          { type: 'tabs', activeTab: 0, panels: ['console'] },
          { type: 'tabs', activeTab: 0, panels: ['profiler'] },
        ],
      },
    };
  }

  // Minimal layout with a flat bottom panel (the legacy shape — must still work)
  function makeLayoutWithFlatBottom(): EditorLayout {
    return {
      root: { type: 'tabs', activeTab: 0, panels: ['hierarchy'] },
      bottomPanel: { type: 'tabs', activeTab: 0, panels: ['console', 'profiler'] },
    };
  }

  describe('removePanelFromLayout', () => {
    it('removes panel from flat bottomPanel', () => {
      const layout = makeLayoutWithFlatBottom();
      const result = removePanelFromLayout(layout, 'profiler');
      const bp = result.bottomPanel as TabsNode;
      expect(bp.panels).toEqual(['console']);
    });

    it('removes panel from split bottomPanel', () => {
      const layout = makeLayoutWithSplitBottom();
      const result = removePanelFromLayout(layout, 'profiler');
      // After removing profiler, the split should collapse to a single tabs node
      expect(result.bottomPanel.type).toBe('tabs');
      const bp = result.bottomPanel as TabsNode;
      expect(bp.panels).toEqual(['console']);
    });

    it('removes panel from root, leaves bottomPanel untouched', () => {
      const layout = makeLayoutWithFlatBottom();
      const result = removePanelFromLayout(layout, 'hierarchy');
      expect((result.root as TabsNode).panels).toEqual([]);
      const bp = result.bottomPanel as TabsNode;
      expect(bp.panels).toEqual(['console', 'profiler']);
    });
  });

  describe('dropPanel — isBottomPanel=true', () => {
    it('center drop adds panel as tab in flat bottomPanel', () => {
      const layout = makeLayoutWithFlatBottom();
      const result = dropPanel(layout, 'hierarchy', [], 'center', true);
      const bp = result.bottomPanel as TabsNode;
      expect(bp.panels).toContain('hierarchy');
      expect((result.root as TabsNode).panels).not.toContain('hierarchy');
    });

    it('right drop creates horizontal split in bottomPanel', () => {
      const layout = makeLayoutWithFlatBottom();
      const result = dropPanel(layout, 'hierarchy', [], 'right', true);
      expect(result.bottomPanel.type).toBe('split');
      if (result.bottomPanel.type === 'split') {
        expect(result.bottomPanel.direction).toBe('horizontal');
        // hierarchy removed from root, placed in bottomPanel
        const allPanels = JSON.stringify(result.bottomPanel);
        expect(allPanels).toContain('hierarchy');
      }
    });

    it('left drop creates horizontal split with new panel on left', () => {
      const layout = makeLayoutWithFlatBottom();
      const result = dropPanel(layout, 'hierarchy', [], 'left', true);
      expect(result.bottomPanel.type).toBe('split');
      if (result.bottomPanel.type === 'split') {
        expect(result.bottomPanel.direction).toBe('horizontal');
        const firstChild = result.bottomPanel.children[0];
        expect(firstChild.type).toBe('tabs');
        if (firstChild.type === 'tabs') {
          expect(firstChild.panels).toContain('hierarchy');
        }
      }
    });

    it('panel is not lost after drop — removed from source, appears in target', () => {
      const layout = makeLayoutWithFlatBottom();
      const result = dropPanel(layout, 'hierarchy', [], 'right', true);
      expect((result.root as TabsNode).panels).not.toContain('hierarchy');
      expect(JSON.stringify(result.bottomPanel)).toContain('hierarchy');
    });
  });

  describe('setActiveTab — isBottomPanel=true', () => {
    it('sets activeTab on flat bottomPanel', () => {
      const layout = makeLayoutWithFlatBottom();
      const result = setActiveTab(layout, [], 1, true);
      expect((result.bottomPanel as TabsNode).activeTab).toBe(1);
    });

    it('sets activeTab on nested TabsNode via path in split bottomPanel', () => {
      const layout = makeLayoutWithSplitBottom();
      // Path [1] → second child (profiler tabs node)
      const result = setActiveTab(layout, [1], 0, true);
      if (result.bottomPanel.type === 'split') {
        const child = result.bottomPanel.children[1];
        expect(child.type).toBe('tabs');
        if (child.type === 'tabs') expect(child.activeTab).toBe(0);
      }
    });
  });

  describe('resizeSplit — isBottomPanel=true', () => {
    it('resizes a split node inside bottomPanel', () => {
      const layout = makeLayoutWithSplitBottom();
      const result = resizeSplit(layout, [], 1, 50, 1000, true);
      if (result.bottomPanel.type === 'split') {
        // delta = +50px on 1000px container = +5% — sizes should shift
        expect(result.bottomPanel.sizes[0]).toBeCloseTo(55, 0);
        expect(result.bottomPanel.sizes[1]).toBeCloseTo(45, 0);
      }
    });
  });
  ```

- [ ] **Step 2: Run the tests — they should fail**

  ```bash
  cd engine/editor && npx vitest run src/lib/docking/store.test.ts 2>&1 | tail -30
  ```

  Expected: TypeScript errors and test failures referencing missing `isBottomPanel` param on `resizeSplit`, and wrong behavior in `removePanelFromLayout` / `dropPanel`.

- [ ] **Step 3: Commit the failing tests**

  ```bash
  cd engine/editor
  git add src/lib/docking/store.test.ts
  git commit -m "test(editor): failing tests for bottomPanel LayoutNode promotion"
  ```

---

## Task 5: Fix `store.ts` — Implement Changes

**Files:**
- Modify: `engine/editor/src/lib/docking/store.ts`

- [ ] **Step 1: Fix `removePanelFromLayout`**

  Replace lines 187–193 (the `bottomPanel`-specific block) with:

  ```ts
  // Before:
  const bIdx = result.bottomPanel.panels.indexOf(panelId);
  if (bIdx !== -1) {
    result.bottomPanel.panels.splice(bIdx, 1);
    if (result.bottomPanel.activeTab >= result.bottomPanel.panels.length) {
      result.bottomPanel.activeTab = Math.max(0, result.bottomPanel.panels.length - 1);
    }
  }

  // After:
  removePanelFromNode(result.bottomPanel, panelId);
  result.bottomPanel = cleanNode(result.bottomPanel);
  ```

- [ ] **Step 2: Extract `insertPanelIntoTree` helper**

  After the `replaceNodeAtPath` function (after line 347), add this private helper:

  ```ts
  /** Insert a panel into a layout tree at the given path and zone.
   *  Handles center (add as tab), and directional (split) zones. */
  function insertPanelIntoTree(
    tree: LayoutNode,
    targetPath: number[],
    zone: DropZone,
    panelId: string,
  ): LayoutNode {
    const found = findTargetNode(tree, targetPath);
    if (!found) return tree;
    const { node: targetNode, path: resolvedPath } = found;

    if (zone === 'center' && targetNode.type === 'tabs') {
      targetNode.panels.push(panelId);
      targetNode.activeTab = targetNode.panels.length - 1;
      return tree;
    }

    const newPanel: TabsNode = { type: 'tabs', activeTab: 0, panels: [panelId] };
    const direction: 'horizontal' | 'vertical' =
      zone === 'left' || zone === 'right' ? 'horizontal' : 'vertical';
    const insertBefore = zone === 'left' || zone === 'top';
    const targetClone: LayoutNode = JSON.parse(JSON.stringify(targetNode));

    const splitNode: SplitNode = {
      type: 'split',
      direction,
      children: insertBefore ? [newPanel, targetClone] : [targetClone, newPanel],
      sizes: insertBefore ? [30, 70] : [70, 30],
    };

    if (resolvedPath.length === 0) {
      return splitNode;
    }
    replaceNodeAtPath(tree, resolvedPath, splitNode);
    return tree;
  }
  ```

- [ ] **Step 3: Rewrite the root-drop section of `dropPanel` to use the helper**

  In `dropPanel`, replace lines 299–336 (from `if (isBottomPanel)` to the end of the function) with:

  ```ts
  if (isBottomPanel) {
    result.bottomPanel = insertPanelIntoTree(result.bottomPanel, targetPath, zone, panelId);
    return result;
  }

  // Root tree drop
  const newRoot = insertPanelIntoTree(result.root, targetPath, zone, panelId);
  result.root = newRoot;
  return result;
  ```

- [ ] **Step 4: Fix `setActiveTab` for bottom panel path-based lookup**

  Replace the `isBottomPanel` branch (lines 391–393):

  ```ts
  // Before:
  if (isBottomPanel) {
    result.bottomPanel.activeTab = tabIndex;
    return result;
  }

  // After:
  if (isBottomPanel) {
    const node = path.length === 0 ? result.bottomPanel : getNodeAtPath(result.bottomPanel, path);
    if (node && node.type === 'tabs') {
      node.activeTab = tabIndex;
    }
    return result;
  }
  ```

- [ ] **Step 5: Add `isBottomPanel` param to `resizeSplit`**

  Replace the `resizeSplit` signature and body:

  ```ts
  export function resizeSplit(
    layout: EditorLayout,
    path: number[],
    index: number,
    deltaPx: number,
    containerSizePx: number,
    isBottomPanel: boolean = false,
  ): EditorLayout {
    const result = cloneLayout(layout);
    const tree = isBottomPanel ? result.bottomPanel : result.root;
    const node = getNodeAtPath(tree, path);
    if (!node || node.type !== 'split') return result;
    if (index < 1 || index >= node.sizes.length) return result;

    const deltaPct = (deltaPx / containerSizePx) * 100;
    const minSize = 5;

    let newPrev = node.sizes[index - 1] + deltaPct;
    let newCurr = node.sizes[index] - deltaPct;

    if (newPrev < minSize) {
      newCurr += newPrev - minSize;
      newPrev = minSize;
    }
    if (newCurr < minSize) {
      newPrev += newCurr - minSize;
      newCurr = minSize;
    }

    node.sizes[index - 1] = newPrev;
    node.sizes[index] = newCurr;

    return result;
  }
  ```

- [ ] **Step 6: Run tests — all should pass now**

  ```bash
  cd engine/editor && npx vitest run src/lib/docking/store.test.ts 2>&1
  ```

  Expected: All tests green.

- [ ] **Step 7: Commit**

  ```bash
  cd engine/editor
  git add src/lib/docking/store.ts
  git commit -m "fix(editor): extend store to support LayoutNode bottomPanel with full drop/resize/tab"
  ```

---

## Task 6: Fix `DockContainer.svelte` — Pass `isBottomPanel` to Children

**Files:**
- Modify: `engine/editor/src/lib/docking/DockContainer.svelte`

**Problem:** When `DockContainer` renders a split node, it creates child `DockContainer` instances but does not pass `isBottomPanel` down. This means when `bottomPanel` contains a `SplitNode`, all child containers will have `isBottomPanel = false` and route drops/resizes to the wrong tree.

- [ ] **Step 1: Pass `isBottomPanel` to child containers in the split render**

  In `DockContainer.svelte`, find the split rendering section (around line 156–163):

  ```svelte
  <!-- Before: -->
  <DockContainer
    node={child}
    {layout}
    path={[...path, i]}
    {panelComponents}
    {onLayoutChange}
  />

  <!-- After: -->
  <DockContainer
    node={child}
    {layout}
    path={[...path, i]}
    {panelComponents}
    {onLayoutChange}
    {isBottomPanel}
  />
  ```

- [ ] **Step 2: Pass `isBottomPanel` to `resizeSplit` in `handleResize`**

  In `handleResize` (around line 45–52):

  ```ts
  // Before:
  function handleResize(index: number, deltaPx: number) {
    if (!containerEl) return;
    const size = node.type === 'split' && node.direction === 'horizontal'
      ? containerEl.clientWidth
      : containerEl.clientHeight;
    const newLayout = resizeSplit(layout, path, index, deltaPx, size);
    onLayoutChange(newLayout);
  }

  // After:
  function handleResize(index: number, deltaPx: number) {
    if (!containerEl) return;
    const size = node.type === 'split' && node.direction === 'horizontal'
      ? containerEl.clientWidth
      : containerEl.clientHeight;
    const newLayout = resizeSplit(layout, path, index, deltaPx, size, isBottomPanel);
    onLayoutChange(newLayout);
  }
  ```

- [ ] **Step 3: Run TypeScript check**

  ```bash
  cd engine/editor && npx tsc --noEmit 2>&1 | grep -v "node_modules"
  ```

  Expected: Errors should now only be in `App.svelte` (next task).

- [ ] **Step 4: Commit**

  ```bash
  cd engine/editor
  git add src/lib/docking/DockContainer.svelte
  git commit -m "fix(editor): propagate isBottomPanel to child containers and resizeSplit"
  ```

---

## Task 7: Fix `App.svelte`

**Files:**
- Modify: `engine/editor/src/App.svelte`

- [ ] **Step 1: Fix the `addPanelToLayout` zone=bottom branch**

  In `App.svelte`, replace lines 102–109 (the `zone === 'bottom'` block):

  ```ts
  // Before:
  if (zone === 'bottom') {
    layout.bottomPanel.panels.push(panelId);
    layout.bottomPanel.activeTab = layout.bottomPanel.panels.length - 1;
    layout = { ...layout };
    saveLayout(layout);
    return;
  }

  // After:
  if (zone === 'bottom') {
    // Find the first TabsNode in the bottomPanel tree and add there.
    // Uses findFirstTabsNode which already exists in this file.
    const target = findFirstTabsNode(layout.bottomPanel);
    if (target) {
      target.panels.push(panelId);
      target.activeTab = target.panels.length - 1;
    } else {
      // bottomPanel is empty — replace with a fresh tabs node
      layout.bottomPanel = { type: 'tabs', activeTab: 0, panels: [panelId] };
    }
    layout = { ...layout };
    saveLayout(layout);
    return;
  }
  ```

- [ ] **Step 2: Fix the bottom panel visibility guard**

  Add a `hasAnyPanels` helper after `findFirstTabsNode` (around line 175):

  ```ts
  function hasAnyPanels(node: import('./lib/docking/types').LayoutNode): boolean {
    if (node.type === 'tabs') return node.panels.length > 0;
    return node.children.some(hasAnyPanels);
  }
  ```

  Then replace line 346:
  ```svelte
  <!-- Before: -->
  {#if layout.bottomPanel.panels.length > 0}

  <!-- After: -->
  {#if hasAnyPanels(layout.bottomPanel)}
  ```

- [ ] **Step 3: Run TypeScript check — should be clean**

  ```bash
  cd engine/editor && npx tsc --noEmit 2>&1 | grep -v "node_modules"
  ```

  Expected: No errors.

- [ ] **Step 4: Run all tests**

  ```bash
  cd engine/editor && npx vitest run 2>&1
  ```

  Expected: All tests pass.

- [ ] **Step 5: Commit**

  ```bash
  cd engine/editor
  git add src/App.svelte
  git commit -m "fix(editor): update App.svelte for LayoutNode bottomPanel — addPanel and visibility guard"
  ```

---

## Task 8: End-to-End Verification

- [ ] **Step 1: Start the dev server**

  ```bash
  cd engine/editor && npm run dev
  ```

- [ ] **Step 2: Verify resize handles**

  - Resize handles show a two-line gutter (not a bold fill) at rest
  - Hovering turns the border lines to accent blue
  - Dragging resizes only the two adjacent panels

- [ ] **Step 3: Verify bottom panel drops**

  - Drag a panel (e.g. Inspector) from the main area and hover over the console panel
  - Drop on the **right** zone → Inspector appears next to console in a horizontal split
  - Drop on the **center** zone → Inspector appears as a tab in the console group
  - Drop on the **left** zone → Inspector appears to the left of console

- [ ] **Step 4: Verify layout persistence**

  - After creating a split bottom panel, reload the page
  - The split layout should be preserved (localStorage round-trip)
  - No console errors on load

- [ ] **Step 5: Final commit and TypeScript check**

  ```bash
  cd engine/editor && npx tsc --noEmit 2>&1 | grep -v "node_modules"
  npx vitest run 2>&1
  ```

  Both should be clean.
