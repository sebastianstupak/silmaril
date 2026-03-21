import { persistLoad, persistSave } from './persist';

export interface RecentItem {
  label: string;
  path: string;
  itemType: 'project' | 'scene';
  openedAt: number;
}

const MAX_RECENT = 10;
const PERSIST_KEY = 'recent-items';

let _items: RecentItem[] = [];
let _listeners: ((items: RecentItem[]) => void)[] = [];

function _notify() {
  _listeners.forEach((fn) => fn([..._items]));
}

/** Call once in onMount to load persisted items into memory. */
export async function hydrateRecentItems(): Promise<void> {
  _items = await persistLoad<RecentItem[]>(PERSIST_KEY, []);
  _notify();
}

/** Add or update a recent item. Moves to front if path already exists. */
export function addRecentItem(item: Omit<RecentItem, 'openedAt'>): void {
  const entry: RecentItem = { ...item, openedAt: Date.now() };
  _items = _items.filter((i) => i.path !== item.path);
  _items.unshift(entry);
  if (_items.length > MAX_RECENT) _items = _items.slice(0, MAX_RECENT);
  _notify();
  persistSave(PERSIST_KEY, _items); // fire-and-forget
}

export function getRecentItems(): RecentItem[] {
  return [..._items];
}

export function subscribeRecent(fn: (items: RecentItem[]) => void): () => void {
  _listeners.push(fn);
  return () => {
    _listeners = _listeners.filter((l) => l !== fn);
  };
}

/** Test-only: reset all state. */
export function _resetRecentItems(): void {
  _items = [];
  _listeners = [];
}
