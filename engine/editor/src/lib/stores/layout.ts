// Shared layout store — allows view command handlers (and other non-Svelte
// modules) to toggle panel visibility without prop-drilling through App.svelte.
//
// App.svelte owns the authoritative layout state; it calls setLayout() whenever
// the user drags/resizes panels, and subscribes via subscribeLayout() to pick
// up changes made by command handlers.

import type { EditorLayout, LayoutNode, TabsNode } from '../docking/types';
import { removePanelFromLayout } from '../docking/store';

// ---------------------------------------------------------------------------
// Module-level state
// ---------------------------------------------------------------------------

let _layout: EditorLayout | null = null;
let _listeners: Array<() => void> = [];

function notify(): void {
  for (const fn of _listeners) fn();
}

// ---------------------------------------------------------------------------
// Public API — used by App.svelte
// ---------------------------------------------------------------------------

/** Replace the stored layout (call this from App.svelte whenever layout changes). */
export function setLayout(layout: EditorLayout): void {
  _layout = layout;
  // Do NOT notify listeners here — App.svelte is the source of truth for normal
  // mutations.  We only notify when command handlers mutate via togglePanel().
}

/** Read the current layout (may be null before App.svelte has initialised). */
export function getLayout(): EditorLayout | null {
  return _layout;
}

/** Register a callback invoked when togglePanel() modifies the layout. */
export function subscribeLayout(fn: () => void): () => void {
  _listeners.push(fn);
  return () => {
    _listeners = _listeners.filter((l) => l !== fn);
  };
}

// ---------------------------------------------------------------------------
// Panel toggle helpers
// ---------------------------------------------------------------------------

function collectPanels(node: LayoutNode, out: Set<string>): void {
  if (node.type === 'tabs') {
    for (const p of node.panels) out.add(p);
  } else {
    for (const c of node.children) collectPanels(c, out);
  }
}

function isPanelVisible(layout: EditorLayout, panelId: string): boolean {
  const panels = new Set<string>();
  collectPanels(layout.root, panels);
  collectPanels(layout.bottomPanel, panels);
  return panels.has(panelId);
}

function addPanelToBottom(layout: EditorLayout, panelId: string): EditorLayout {
  const result: EditorLayout = JSON.parse(JSON.stringify(layout));
  const bottom = result.bottomPanel;
  if (bottom.type === 'tabs') {
    bottom.panels.push(panelId);
    bottom.activeTab = bottom.panels.length - 1;
  } else {
    // Find the first tabs node in the bottom panel
    const first = findFirstTabs(bottom);
    if (first) {
      first.panels.push(panelId);
      first.activeTab = first.panels.length - 1;
    } else {
      result.bottomPanel = { type: 'tabs', activeTab: 0, panels: [panelId] };
    }
  }
  return result;
}

function findFirstTabs(node: LayoutNode): TabsNode | null {
  if (node.type === 'tabs') return node;
  for (const child of node.children) {
    const found = findFirstTabs(child);
    if (found) return found;
  }
  return null;
}

/**
 * Toggle a panel's visibility in the current layout.
 *
 * If the panel is currently visible, it is removed from the layout tree.
 * If the panel is hidden, it is added to the bottom panel as a new tab.
 *
 * This function notifies layout subscribers (i.e. App.svelte) so the change
 * is reflected in the UI.
 *
 * No-op when the layout has not been initialised yet.
 */
export function togglePanel(panelId: string): void {
  if (_layout === null) return;

  if (isPanelVisible(_layout, panelId)) {
    _layout = removePanelFromLayout(_layout, panelId);
  } else {
    _layout = addPanelToBottom(_layout, panelId);
  }

  notify();
}

// ---------------------------------------------------------------------------
// Convenience exports for each panel used by view commands
// ---------------------------------------------------------------------------

export function toggleHierarchy(): void { togglePanel('hierarchy'); }
export function toggleInspector(): void { togglePanel('inspector'); }
export function toggleConsole(): void { togglePanel('console'); }
export function toggleAssetBrowser(): void { togglePanel('assets'); }
