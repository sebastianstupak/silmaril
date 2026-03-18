// Shared editor context accessible by docked panels without prop drilling
// Uses module-level state with Svelte 5 $state via getter/setter pattern

import type { EntityInfo } from '$lib/api';

interface EditorContext {
  entities: EntityInfo[];
  selectedEntityId: number | null;
}

let _context: EditorContext = {
  entities: [],
  selectedEntityId: null,
};

let _listeners: Array<() => void> = [];

function notify() {
  for (const fn of _listeners) fn();
}

export function getEditorContext(): EditorContext {
  return _context;
}

export function setEntities(entities: EntityInfo[]) {
  _context = { ..._context, entities };
  notify();
}

export function setSelectedEntityId(id: number | null) {
  _context = { ..._context, selectedEntityId: id };
  notify();
}

export function getSelectedEntity(): EntityInfo | null {
  if (_context.selectedEntityId == null) return null;
  return _context.entities.find(e => e.id === _context.selectedEntityId) ?? null;
}

export function subscribeContext(fn: () => void): () => void {
  _listeners.push(fn);
  return () => {
    _listeners = _listeners.filter(l => l !== fn);
  };
}
