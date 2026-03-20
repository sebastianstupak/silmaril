import type { SavedLayout } from './docking/store';

const MODIFIER_KEYS = new Set(['Control', 'Shift', 'Alt', 'Meta']);

/**
 * Convert a KeyboardEvent to a canonical keybind string like "ctrl+shift+s".
 * Returns null if only a modifier key was pressed (no complete binding yet).
 */
export function captureKeybind(e: KeyboardEvent): string | null {
  if (MODIFIER_KEYS.has(e.key)) return null;

  const parts: string[] = [];
  if (e.ctrlKey)  parts.push('ctrl');
  if (e.shiftKey) parts.push('shift');
  if (e.altKey)   parts.push('alt');
  if (e.metaKey)  parts.push('meta');
  parts.push(e.key.toLowerCase());
  return parts.join('+');
}

/**
 * Format a keybind string for display, e.g. "ctrl+shift+s" → "Ctrl+Shift+S".
 */
export function formatKeybindDisplay(keybind: string): string {
  return keybind
    .split('+')
    .map(part => part.charAt(0).toUpperCase() + part.slice(1))
    .join('+');
}

/**
 * Find the first layout in `layouts` (other than `excludeId`) that already
 * uses the given keybind.  Returns undefined if there is no conflict.
 */
export function findKeybindConflict(
  keybind: string | undefined,
  excludeId: string,
  layouts: SavedLayout[],
): SavedLayout | undefined {
  if (!keybind) return undefined;
  return layouts.find(l => l.id !== excludeId && l.keybind === keybind);
}
