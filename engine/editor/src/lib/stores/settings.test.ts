import { describe, it, expect, vi, beforeEach } from 'vitest';

// Reset module cache + localStorage before each test so tests are isolated.
beforeEach(() => {
  vi.resetModules();
  localStorage.clear();
});

describe('loadSettings — compactMenu', () => {
  it('defaults compactMenu to false when no stored settings exist', async () => {
    const { loadSettings } = await import('$lib/stores/settings');
    const settings = loadSettings();
    expect(settings.compactMenu).toBe(false);
  });

  it('returns false for compactMenu when stored settings lack the field', async () => {
    localStorage.setItem('silmaril-editor-settings', JSON.stringify({ theme: 'dark' }));
    const { loadSettings } = await import('$lib/stores/settings');
    const settings = loadSettings();
    expect(settings.compactMenu).toBe(false);
  });

  it('round-trips compactMenu=true through saveSettings → loadSettings', async () => {
    const { loadSettings, saveSettings } = await import('$lib/stores/settings');
    const settings = loadSettings();
    saveSettings({ ...settings, compactMenu: true });
    const reloaded = loadSettings();
    expect(reloaded.compactMenu).toBe(true);
  });
});
