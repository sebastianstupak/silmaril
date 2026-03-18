// Editor settings — persisted to localStorage (Tauri has access)

export interface EditorSettings {
  theme: string;
  language: string;
  leftPanelWidth: number;
  rightPanelWidth: number;
  bottomPanelHeight: number;
  fontSize: number;
  autoSave: 'off' | 'on_focus_change' | 'after_delay';
}

const STORAGE_KEY = 'silmaril-editor-settings';

const defaults: EditorSettings = {
  theme: 'dark',
  language: 'en',
  leftPanelWidth: 250,
  rightPanelWidth: 300,
  bottomPanelHeight: 200,
  fontSize: 13,
  autoSave: 'off',
};

export function loadSettings(): EditorSettings {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) {
      return { ...defaults, ...JSON.parse(stored) };
    }
  } catch {
    // ignore parse errors
  }
  return { ...defaults };
}

export function saveSettings(settings: EditorSettings) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
  } catch {
    // ignore storage errors
  }
}
