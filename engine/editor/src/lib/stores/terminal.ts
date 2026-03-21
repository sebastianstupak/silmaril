// Terminal panel store — manages tab list and pending PTY data.
// Follows the module-level singleton pattern from console.ts.

export interface TerminalTab {
  id: string;
  title: string;   // "Terminal 1", "Terminal 2", etc.
  exited: boolean; // true after terminal-exit event; tab stays visible, dimmed
}

export interface TerminalState {
  tabs: TerminalTab[];
  activeTabId: string | null;
  pendingData: Map<string, string>; // per-tab unread PTY data buffer
}

let state: TerminalState = { tabs: [], activeTabId: null, pendingData: new Map() };
let listeners: (() => void)[] = [];
let tabCounter = 0; // monotonically increasing, never resets

function notify() {
  listeners.forEach(fn => fn());
}

export function getTerminalState(): TerminalState {
  return state;
}

export function subscribeTerminal(fn: () => void): () => void {
  listeners.push(fn);
  return () => { listeners = listeners.filter(l => l !== fn); };
}

export function addTab(id: string): void {
  tabCounter++;
  state.tabs = [...state.tabs, { id, title: `Terminal ${tabCounter}`, exited: false }];
  state.activeTabId = id;
  notify();
}

export function closeTab(id: string): void {
  const tab = state.tabs.find(t => t.id === id);
  if (!tab) return;

  // Block closing the last non-exited tab
  if (state.tabs.length === 1 && !tab.exited) return;

  const idx = state.tabs.indexOf(tab);
  state.tabs = state.tabs.filter(t => t.id !== id);
  state.pendingData.delete(id);

  // If we closed the active tab, switch to the nearest remaining tab
  if (state.activeTabId === id) {
    if (state.tabs.length === 0) {
      state.activeTabId = null;
    } else {
      const newIdx = Math.max(0, idx - 1);
      state.activeTabId = state.tabs[newIdx].id;
    }
  }
  notify();
}

export function setActiveTab(id: string): void {
  state.activeTabId = id;
  notify();
}

export function markExited(id: string): void {
  state.tabs = state.tabs.map(t => t.id === id ? { ...t, exited: true } : t);
  notify();
}

export function appendTerminalData(tabId: string, data: string): void {
  const existing = state.pendingData.get(tabId) ?? '';
  state.pendingData.set(tabId, existing + data);
  notify();
}

// NOTE: drainTerminalData does NOT call notify(). This is intentional —
// TerminalPanel.$effect reads pendingData via this function inside a $effect.
// If notify() were called here, it would trigger an infinite effect loop.
export function drainTerminalData(tabId: string): string {
  const data = state.pendingData.get(tabId) ?? '';
  state.pendingData.delete(tabId);
  return data;
}

/** For testing only — resets all module-level state. */
export function _resetForTest(): void {
  state = { tabs: [], activeTabId: null, pendingData: new Map() };
  listeners = [];
  tabCounter = 0;
}
