// src/lib/inspector/schema-store.ts
// Loads component schemas from Tauri once and caches them.
// Follows the same pub/sub pattern as scene/state.ts.

import type { ComponentSchemas } from './schema';
import { getComponentSchemas } from '$lib/api';

let schemas: ComponentSchemas = {};
let loaded = false;
const listeners: Array<() => void> = [];

function notify() {
  for (const fn of listeners) fn();
}

/** Load schemas from the backend (idempotent — safe to call multiple times). */
export async function loadSchemas(): Promise<void> {
  if (loaded) return;
  const list = await getComponentSchemas();
  schemas = Object.fromEntries(list.map((s) => [s.name, s]));
  loaded = true;
  notify();
}

/** Get the cached schema map. Returns empty object before loadSchemas() resolves. */
export function getSchemas(): ComponentSchemas {
  return schemas;
}

/** Subscribe to schema updates. Returns unsubscribe function. */
export function subscribeSchemas(fn: () => void): () => void {
  listeners.push(fn);
  return () => {
    const i = listeners.indexOf(fn);
    if (i >= 0) listeners.splice(i, 1);
  };
}
