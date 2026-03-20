// Docking layout state management with persistence and templates
import type { EditorLayout, LayoutNode, TabsNode, SplitNode, DropZone } from './types';
import { persistLoad, persistSave } from '../stores/persist';

const STORAGE_KEY = 'silmaril-editor-layout';

// ---------------------------------------------------------------------------
// Layout Templates
// ---------------------------------------------------------------------------

export const defaultLayout: EditorLayout = {
  root: {
    type: 'split',
    direction: 'horizontal',
    sizes: [20, 55, 25],
    children: [
      { type: 'tabs', activeTab: 0, panels: ['hierarchy'] },
      { type: 'tabs', activeTab: 0, panels: ['viewport'] },
      { type: 'tabs', activeTab: 0, panels: ['inspector'] },
    ],
  },
  bottomPanel: { type: 'tabs', activeTab: 0, panels: ['console'] },
};

export const tallLayout: EditorLayout = {
  root: {
    type: 'split',
    direction: 'horizontal',
    sizes: [20, 60, 20],
    children: [
      {
        type: 'split',
        direction: 'vertical',
        sizes: [60, 40],
        children: [
          { type: 'tabs', activeTab: 0, panels: ['hierarchy'] },
          { type: 'tabs', activeTab: 0, panels: ['assets'] },
        ],
      },
      { type: 'tabs', activeTab: 0, panels: ['viewport'] },
      { type: 'tabs', activeTab: 0, panels: ['inspector'] },
    ],
  },
  bottomPanel: { type: 'tabs', activeTab: 0, panels: ['console'] },
};

export const wideLayout: EditorLayout = {
  root: {
    type: 'split',
    direction: 'vertical',
    sizes: [70, 30],
    children: [
      {
        type: 'split',
        direction: 'horizontal',
        sizes: [20, 80],
        children: [
          { type: 'tabs', activeTab: 0, panels: ['hierarchy'] },
          { type: 'tabs', activeTab: 0, panels: ['viewport'] },
        ],
      },
      {
        type: 'split',
        direction: 'horizontal',
        sizes: [50, 50],
        children: [
          { type: 'tabs', activeTab: 0, panels: ['inspector'] },
          { type: 'tabs', activeTab: 0, panels: ['console'] },
        ],
      },
    ],
  },
  bottomPanel: { type: 'tabs', activeTab: 0, panels: [] },
};

// ---------------------------------------------------------------------------
// Deep clone utility
// ---------------------------------------------------------------------------

function cloneLayout(layout: EditorLayout): EditorLayout {
  return JSON.parse(JSON.stringify(layout));
}

// ---------------------------------------------------------------------------
// Persistence
// ---------------------------------------------------------------------------

export function loadLayout(): EditorLayout {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) {
      const parsed = JSON.parse(stored) as EditorLayout;
      if (parsed.root && parsed.bottomPanel) {
        return parsed;
      }
    }
  } catch {
    // ignore parse errors
  }
  return cloneLayout(defaultLayout);
}

let saveTimer: ReturnType<typeof setTimeout> | null = null;

export function saveLayout(layout: EditorLayout) {
  if (saveTimer) clearTimeout(saveTimer);
  saveTimer = setTimeout(() => {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(layout));
    } catch {
      // ignore storage errors
    }
    persistSave('layout', layout);
  }, 300);
}

/** Load layout from tauri-plugin-store and update the localStorage cache. */
export async function hydrateLayout(): Promise<EditorLayout | null> {
  const stored = await persistLoad<EditorLayout>('layout', null as any);
  if (stored?.root && stored?.bottomPanel) {
    try { localStorage.setItem(STORAGE_KEY, JSON.stringify(stored)); } catch { /* ignore */ }
    return stored;
  }
  return null;
}

// ---------------------------------------------------------------------------
// Drag state (module-level singleton, mouse-based)
// ---------------------------------------------------------------------------

export interface DragState {
  panelId: string;
  active: boolean;
  mouseX: number;
  mouseY: number;
  /** True when activated by a pop-out window drag, not an internal tab drag. */
  popout: boolean;
  /** The layout path of the DockDropZone panel currently being hovered (popout drag only). */
  dropPath: number[] | null;
  /** The zone within that panel currently being hovered. */
  dropZone: DropZone | null;
  /** Whether the hovered panel is in the bottom panel area. */
  dropIsBottom: boolean;
}

let _dragState: DragState = {
  panelId: '', active: false, mouseX: 0, mouseY: 0, popout: false,
  dropPath: null, dropZone: null, dropIsBottom: false,
};
let _dragListeners: Array<() => void> = [];

export function getDragState(): DragState {
  return _dragState;
}

export function subscribeDrag(listener: () => void): () => void {
  _dragListeners.push(listener);
  return () => {
    _dragListeners = _dragListeners.filter(l => l !== listener);
  };
}

function notifyDragListeners() {
  for (const l of _dragListeners) l();
}

export function startDrag(panelId: string, x: number, y: number, popout = false) {
  // Reset dropPath/dropZone on each tick so stale values don't persist when the
  // cursor moves into a gap between panels. DockDropZone re-sets them if hovered.
  _dragState = { panelId, active: true, mouseX: x, mouseY: y, popout, dropPath: null, dropZone: null, dropIsBottom: false };
  notifyDragListeners();
}

export function updateDrag(x: number, y: number) {
  _dragState = { ..._dragState, mouseX: x, mouseY: y };
  notifyDragListeners();
}

export function endDrag() {
  _dragState = {
    panelId: '', active: false, mouseX: 0, mouseY: 0, popout: false,
    dropPath: null, dropZone: null, dropIsBottom: false,
  };
  notifyDragListeners();
}

/** Called by DockDropZone to report which panel+zone the cursor is currently over. */
export function updateDropTarget(path: number[] | null, zone: DropZone | null, isBottom: boolean) {
  _dragState = { ..._dragState, dropPath: path, dropZone: zone, dropIsBottom: isBottom };
  // No listener notification needed — this is for querying at drop time only.
}

// ---------------------------------------------------------------------------
// Layout Mutations
// ---------------------------------------------------------------------------

/** Find a TabsNode by path indices through the tree */
function getNodeAtPath(root: LayoutNode, path: number[]): LayoutNode | null {
  let node: LayoutNode = root;
  for (const idx of path) {
    if (node.type !== 'split') return null;
    if (idx < 0 || idx >= node.children.length) return null;
    node = node.children[idx];
  }
  return node;
}

/** Remove a panel from wherever it currently lives in the tree.
 *  Returns a cleaned tree (empty tabs nodes are pruned, single-child splits are collapsed). */
export function removePanelFromLayout(layout: EditorLayout, panelId: string): EditorLayout {
  const result = cloneLayout(layout);
  removePanelFromNode(result.root, panelId);
  result.root = cleanNode(result.root);

  removePanelFromNode(result.bottomPanel, panelId);
  result.bottomPanel = cleanNode(result.bottomPanel);

  return result;
}

function removePanelFromNode(node: LayoutNode, panelId: string) {
  if (node.type === 'tabs') {
    const idx = node.panels.indexOf(panelId);
    if (idx !== -1) {
      node.panels.splice(idx, 1);
      if (node.activeTab >= node.panels.length) {
        node.activeTab = Math.max(0, node.panels.length - 1);
      }
    }
  } else {
    for (const child of node.children) {
      removePanelFromNode(child, panelId);
    }
  }
}

/** Clean up the tree: remove empty tab nodes, collapse single-child splits */
function cleanNode(node: LayoutNode): LayoutNode {
  if (node.type === 'tabs') {
    return node;
  }

  // Recursively clean children
  node.children = node.children.map(c => cleanNode(c));

  // Remove empty tabs nodes
  const nonEmpty: { node: LayoutNode; size: number }[] = [];
  for (let i = 0; i < node.children.length; i++) {
    const child = node.children[i];
    if (child.type === 'tabs' && child.panels.length === 0) continue;
    nonEmpty.push({ node: child, size: node.sizes[i] });
  }

  if (nonEmpty.length === 0) {
    return { type: 'tabs', activeTab: 0, panels: [] };
  }

  if (nonEmpty.length === 1) {
    return nonEmpty[0].node;
  }

  // Re-normalize sizes
  const totalSize = nonEmpty.reduce((s, e) => s + e.size, 0);
  node.children = nonEmpty.map(e => e.node);
  node.sizes = nonEmpty.map(e => (e.size / totalSize) * 100);

  return node;
}

/** Find the first TabsNode in a subtree (depth-first) */
function findFirstTabsNode(node: LayoutNode): TabsNode | null {
  if (node.type === 'tabs') return node;
  for (const child of node.children) {
    const found = findFirstTabsNode(child);
    if (found) return found;
  }
  return null;
}

/** Find a TabsNode by searching for it by one of its panel IDs */
function findTabsNodeWithPanel(root: LayoutNode, panelId: string, path: number[] = []): { node: TabsNode; path: number[] } | null {
  if (root.type === 'tabs') {
    if (root.panels.includes(panelId)) {
      return { node: root, path };
    }
    return null;
  }
  for (let i = 0; i < root.children.length; i++) {
    const result = findTabsNodeWithPanel(root.children[i], panelId, [...path, i]);
    if (result) return result;
  }
  return null;
}

/** Find any valid tabs node at or near the given path after tree restructuring.
 *  Falls back to searching the tree if the original path is stale. */
function findTargetNode(root: LayoutNode, originalPath: number[]): { node: LayoutNode; path: number[] } | null {
  // Try the original path first
  const directNode = getNodeAtPath(root, originalPath);
  if (directNode) return { node: directNode, path: originalPath };

  // The tree was restructured — try progressively shorter paths
  for (let len = originalPath.length - 1; len >= 0; len--) {
    const shorterPath = originalPath.slice(0, len);
    const node = getNodeAtPath(root, shorterPath);
    if (node) return { node, path: shorterPath };
  }

  // Last resort: return root
  return { node: root, path: [] };
}

/** Drop a panel onto a target location, producing a new layout */
export function dropPanel(
  layout: EditorLayout,
  panelId: string,
  targetPath: number[],
  zone: DropZone,
  isBottomPanel: boolean,
): EditorLayout {
  // First remove the panel from its current location
  let result = removePanelFromLayout(layout, panelId);

  if (isBottomPanel) {
    result.bottomPanel = insertPanelIntoTree(result.bottomPanel, targetPath, zone, panelId);
    return result;
  }

  // Root tree drop
  const newRoot = insertPanelIntoTree(result.root, targetPath, zone, panelId);
  result.root = newRoot;
  return result;
}

function replaceNodeAtPath(root: LayoutNode, path: number[], replacement: LayoutNode) {
  if (path.length === 0) return; // handled by caller for root
  const parentPath = path.slice(0, -1);
  const idx = path[path.length - 1];
  const parent = getNodeAtPath(root, parentPath);
  if (parent && parent.type === 'split') {
    parent.children[idx] = replacement;
  }
}

/** Insert a panel into a layout tree at the given path and zone. */
function insertPanelIntoTree(
  tree: LayoutNode,
  targetPath: number[],
  zone: DropZone,
  panelId: string,
): LayoutNode {
  const found = findTargetNode(tree, targetPath);
  if (!found) return tree;
  const { node: targetNode, path: resolvedPath } = found;

  if (zone === 'center') {
    // Find a tabs node to add into — targetNode might be a SplitNode if path is stale
    const tabsNode = targetNode.type === 'tabs' ? targetNode : findFirstTabsNode(targetNode);
    if (tabsNode) {
      tabsNode.panels.push(panelId);
      tabsNode.activeTab = tabsNode.panels.length - 1;
      return tree;
    }
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

/** Update sizes in a split node at a given path */
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
  const minSize = 5; // minimum 5%

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

/** Set active tab in a tabs node at the given path */
export function setActiveTab(
  layout: EditorLayout,
  path: number[],
  tabIndex: number,
  isBottomPanel: boolean,
): EditorLayout {
  const result = cloneLayout(layout);
  if (isBottomPanel) {
    const node = path.length === 0 ? result.bottomPanel : getNodeAtPath(result.bottomPanel, path);
    if (node && node.type === 'tabs') {
      node.activeTab = tabIndex;
    }
    return result;
  }
  const node = getNodeAtPath(result.root, path);
  if (node && node.type === 'tabs') {
    node.activeTab = tabIndex;
  }
  return result;
}

// ---------------------------------------------------------------------------
// Saved Layouts (named, persistent, per-slot keybinds)
// ---------------------------------------------------------------------------

export interface SavedLayout {
  id: string;
  name: string;
  layout: EditorLayout;
  /** Keybind string, e.g. "ctrl+1". Stored with the slot so it follows reorder. */
  keybind?: string;
}

const SAVED_LAYOUTS_KEY = 'silmaril-saved-layouts';

export const initialSavedLayouts: SavedLayout[] = [
  { id: 'builtin-edit',   name: 'Edit',   layout: defaultLayout, keybind: 'ctrl+1' },
  { id: 'builtin-assets', name: 'Assets', layout: tallLayout,    keybind: 'ctrl+2' },
  { id: 'builtin-review', name: 'Review', layout: wideLayout,    keybind: 'ctrl+3' },
];

export function loadSavedLayouts(): SavedLayout[] {
  try {
    const stored = localStorage.getItem(SAVED_LAYOUTS_KEY);
    if (stored) {
      const parsed = JSON.parse(stored) as SavedLayout[];
      if (Array.isArray(parsed) && parsed.length > 0) return parsed;
    }
  } catch {
    // ignore parse errors
  }
  return JSON.parse(JSON.stringify(initialSavedLayouts));
}

let _saveLayoutsTimer: ReturnType<typeof setTimeout> | null = null;

export function saveSavedLayouts(layouts: SavedLayout[]) {
  if (_saveLayoutsTimer) clearTimeout(_saveLayoutsTimer);
  _saveLayoutsTimer = setTimeout(() => {
    try {
      localStorage.setItem(SAVED_LAYOUTS_KEY, JSON.stringify(layouts));
    } catch {
      // ignore storage errors
    }
    persistSave('savedLayouts', layouts);
  }, 300);
}

/** Load saved layouts from tauri-plugin-store and update the localStorage cache. */
export async function hydrateSavedLayouts(): Promise<SavedLayout[] | null> {
  const stored = await persistLoad<SavedLayout[]>('savedLayouts', null as any);
  if (Array.isArray(stored) && stored.length > 0) {
    try { localStorage.setItem(SAVED_LAYOUTS_KEY, JSON.stringify(stored)); } catch { /* ignore */ }
    return stored;
  }
  return null;
}

// ── Tab cycle (Ctrl+Tab) ──────────────────────────────────────────────────────
//
// The focused DockContainer registers a callback; the global Ctrl+Tab handler
// in App.svelte calls cycleActiveTab() which delegates to that callback.
// Using a callback (rather than storing path + querying layout) keeps the
// cycling logic co-located with the container that owns it, and the callback
// closes over Svelte 5 $props() signals so it always reads fresh values.

type TabCycleFn = (direction: number) => void;
let _tabCycleFn: TabCycleFn | null = null;

/** Register the tab-cycle handler for the currently focused container. */
export function registerTabCycle(fn: TabCycleFn | null): void {
  _tabCycleFn = fn;
}

/** Cycle the active tab in the currently focused container. */
export function cycleActiveTab(direction: number): void {
  _tabCycleFn?.(direction);
}
