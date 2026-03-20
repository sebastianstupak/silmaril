/**
 * Tauri-plugin-store wrapper for the Silmaril editor.
 *
 * All editor settings, layout, and saved-layouts are persisted to a single
 * `silmaril.json` file in the OS app-data directory (AppData/Roaming on
 * Windows).  This survives WebView2 cache clears and is accessible from Rust.
 *
 * Usage:
 *   const settings = await persistLoad('settings', defaultSettings);
 *   await persistSave('settings', updatedSettings);
 */

// Detect Tauri runtime — persist.ts is also imported in the vitest / browser
// dev-server environment where @tauri-apps/plugin-store is unavailable.
const IS_TAURI =
  typeof window !== 'undefined' && !!(window as any).__TAURI_INTERNALS__;

// Lazily-initialised store — created once on first access.
let _storePromise: Promise<import('@tauri-apps/plugin-store').Store> | null = null;

async function getStore() {
  if (!IS_TAURI) return null;
  if (!_storePromise) {
    _storePromise = import('@tauri-apps/plugin-store').then(({ Store }) =>
      Store.load('silmaril.json'),
    );
  }
  return _storePromise;
}

/**
 * Load a value from the persistent store.
 * Falls back to `defaultValue` when running outside Tauri or when the key
 * has never been written.
 */
export async function persistLoad<T>(key: string, defaultValue: T): Promise<T> {
  try {
    const store = await getStore();
    if (!store) return defaultValue;
    const val = await store.get<T>(key);
    return val ?? defaultValue;
  } catch {
    return defaultValue;
  }
}

/**
 * Persist a value to the store and flush to disk.
 * Silently no-ops outside Tauri.
 */
export async function persistSave<T>(key: string, value: T): Promise<void> {
  try {
    const store = await getStore();
    if (!store) return;
    await store.set(key, value);
    await store.save();
  } catch {
    // ignore write errors
  }
}
