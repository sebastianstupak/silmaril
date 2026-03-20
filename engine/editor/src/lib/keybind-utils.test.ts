import { describe, it, expect } from 'vitest';
import { captureKeybind, formatKeybindDisplay, findKeybindConflict } from './keybind-utils';
import type { SavedLayout } from './docking/store';

// ── Fixtures ──────────────────────────────────────────────────────────────────

const layouts: SavedLayout[] = [
  { id: 'a', name: 'Edit',   layout: {} as any, keybind: 'ctrl+1' },
  { id: 'b', name: 'Assets', layout: {} as any, keybind: 'ctrl+2' },
  { id: 'c', name: 'Review', layout: {} as any },               // no keybind
];

// ── captureKeybind ────────────────────────────────────────────────────────────

describe('captureKeybind', () => {
  it('returns null when only a modifier key is pressed', () => {
    expect(captureKeybind(new KeyboardEvent('keydown', { key: 'Control', ctrlKey: true }))).toBeNull();
    expect(captureKeybind(new KeyboardEvent('keydown', { key: 'Shift',   shiftKey: true }))).toBeNull();
    expect(captureKeybind(new KeyboardEvent('keydown', { key: 'Alt',     altKey: true  }))).toBeNull();
    expect(captureKeybind(new KeyboardEvent('keydown', { key: 'Meta',    metaKey: true }))).toBeNull();
  });

  it('builds ctrl+key format', () => {
    const e = new KeyboardEvent('keydown', { key: '1', ctrlKey: true });
    expect(captureKeybind(e)).toBe('ctrl+1');
  });

  it('builds ctrl+shift+key format with modifiers in fixed order', () => {
    const e = new KeyboardEvent('keydown', { key: 's', ctrlKey: true, shiftKey: true });
    expect(captureKeybind(e)).toBe('ctrl+shift+s');
  });

  it('normalises the key to lowercase', () => {
    const e = new KeyboardEvent('keydown', { key: 'A', ctrlKey: true });
    expect(captureKeybind(e)).toBe('ctrl+a');
  });

  it('handles function keys without modifiers', () => {
    const e = new KeyboardEvent('keydown', { key: 'F5' });
    expect(captureKeybind(e)).toBe('f5');
  });

  it('handles Escape without modifiers', () => {
    const e = new KeyboardEvent('keydown', { key: 'Escape' });
    expect(captureKeybind(e)).toBe('escape');
  });

  it('includes alt modifier', () => {
    const e = new KeyboardEvent('keydown', { key: 'z', ctrlKey: true, altKey: true });
    expect(captureKeybind(e)).toBe('ctrl+alt+z');
  });
});

// ── formatKeybindDisplay ──────────────────────────────────────────────────────

describe('formatKeybindDisplay', () => {
  it('capitalises Ctrl', () => {
    expect(formatKeybindDisplay('ctrl+1')).toBe('Ctrl+1');
  });

  it('capitalises all modifier segments', () => {
    expect(formatKeybindDisplay('ctrl+shift+s')).toBe('Ctrl+Shift+S');
  });

  it('uppercases single-letter keys', () => {
    expect(formatKeybindDisplay('ctrl+a')).toBe('Ctrl+A');
  });

  it('capitalises function keys', () => {
    expect(formatKeybindDisplay('f5')).toBe('F5');
  });

  it('capitalises Escape', () => {
    expect(formatKeybindDisplay('escape')).toBe('Escape');
  });
});

// ── findKeybindConflict ───────────────────────────────────────────────────────

describe('findKeybindConflict', () => {
  it('returns the conflicting layout when another slot has the same keybind', () => {
    const conflict = findKeybindConflict('ctrl+1', 'b', layouts);
    expect(conflict?.id).toBe('a');
  });

  it('returns undefined when no other slot has the keybind', () => {
    expect(findKeybindConflict('ctrl+9', 'a', layouts)).toBeUndefined();
  });

  it('ignores the excluded id (self-check does not conflict)', () => {
    expect(findKeybindConflict('ctrl+1', 'a', layouts)).toBeUndefined();
  });

  it('returns undefined for an empty keybind string', () => {
    expect(findKeybindConflict('', 'a', layouts)).toBeUndefined();
  });

  it('does not treat undefined keybind as a conflict', () => {
    // slot 'c' has no keybind — searching for undefined should not match
    expect(findKeybindConflict(undefined as any, 'a', layouts)).toBeUndefined();
  });
});
