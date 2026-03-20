// Editor settings — persisted via tauri-plugin-store (primary) + localStorage (fallback/cache)

import { persistLoad, persistSave } from './persist';

export interface EditorSettings {
  theme: string;
  language: string;
  leftPanelWidth: number;
  rightPanelWidth: number;
  bottomPanelHeight: number;
  fontSize: number;
  autoSave: 'off' | 'on_focus_change' | 'after_delay';
  compactMenu: boolean;
}

const STORAGE_KEY = 'silmaril-editor-settings';
const PERSIST_KEY = 'settings';

export const defaultSettings: EditorSettings = {
  theme: 'dark',
  language: 'en',
  leftPanelWidth: 250,
  rightPanelWidth: 300,
  bottomPanelHeight: 200,
  fontSize: 13,
  autoSave: 'off',
  compactMenu: false,
};

/** Synchronous load from localStorage cache — used for initial $state() hydration. */
export function loadSettings(): EditorSettings {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) {
      return { ...defaultSettings, ...JSON.parse(stored) };
    }
  } catch {
    // ignore parse errors
  }
  return { ...defaultSettings };
}

/** Save to localStorage (instant) and tauri-plugin-store (async, durable). */
export function saveSettings(settings: EditorSettings) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
  } catch {
    // ignore storage errors
  }
  persistSave(PERSIST_KEY, settings);
}

/**
 * Load from tauri-plugin-store and update the localStorage cache.
 * Call in onMount to hydrate from the durable store after initial render.
 */
export async function hydrateSettings(): Promise<EditorSettings> {
  const stored = await persistLoad<EditorSettings>(PERSIST_KEY, loadSettings());
  const merged = { ...defaultSettings, ...stored };
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(merged));
  } catch { /* ignore */ }
  return merged;
}
