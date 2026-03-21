import { describe, it, expect, vi, beforeEach } from 'vitest';

// Mock the bindings module before importing dispatch
vi.mock('./bindings', () => ({
  commands: {
    listCommands: vi.fn(),
    runCommand: vi.fn().mockResolvedValue({ status: 'ok', data: null }),
  },
}));

import {
  populateRegistry,
  registerCommandHandler,
  dispatchCommand,
  getSpec,
  listSpecs,
  _resetRegistryForTesting,
  setUndoVerifier,
} from './dispatch';
import { commands } from './bindings';

// ──────────────────────────────────────────────────────────────────────────────
// Fixtures
// ──────────────────────────────────────────────────────────────────────────────

const sampleSpecs = [
  {
    id: 'file.save_scene',
    module_id: 'file',
    label: 'Save Scene',
    category: 'File',
    description: null,
    keybind: 'Ctrl+S',
    args_schema: null,
    returns_data: false,
    non_undoable: true,
  },
  {
    id: 'viewport.screenshot',
    module_id: 'viewport',
    label: 'Take Screenshot',
    category: 'Viewport',
    description: null,
    keybind: null,
    args_schema: null,
    returns_data: true,
    non_undoable: true,
  },
  {
    id: 'template.execute',
    module_id: 'template',
    label: 'Execute Template',
    category: 'Template',
    description: null,
    keybind: null,
    args_schema: null,
    returns_data: false,
    non_undoable: false,
  },
];

// ──────────────────────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────────────────────

describe('dispatch', () => {
  beforeEach(() => {
    _resetRegistryForTesting();
    populateRegistry(sampleSpecs);
    vi.clearAllMocks();
  });

  // ──────────────────────────────────────────────────────────────────────────
  // populateRegistry
  // ──────────────────────────────────────────────────────────────────────────

  describe('populateRegistry', () => {
    it('populates specs from list', () => {
      expect(listSpecs()).toHaveLength(3);
      expect(getSpec('file.save_scene')).toBeDefined();
    });

    it('clears previous specs on repopulate', () => {
      populateRegistry([sampleSpecs[0]]);
      expect(listSpecs()).toHaveLength(1);
    });

    it('stores all provided specs by id', () => {
      expect(getSpec('viewport.screenshot')).toBeDefined();
      expect(getSpec('template.execute')).toBeDefined();
    });
  });

  // ──────────────────────────────────────────────────────────────────────────
  // getSpec / listSpecs
  // ──────────────────────────────────────────────────────────────────────────

  describe('getSpec', () => {
    it('returns undefined for an unknown id', () => {
      expect(getSpec('does.not.exist')).toBeUndefined();
    });

    it('returns the correct spec', () => {
      const spec = getSpec('file.save_scene');
      expect(spec?.label).toBe('Save Scene');
      expect(spec?.keybind).toBe('Ctrl+S');
    });
  });

  describe('listSpecs', () => {
    it('returns an array with all specs', () => {
      const all = listSpecs();
      expect(all).toHaveLength(3);
      const ids = all.map((s) => s.id);
      expect(ids).toContain('file.save_scene');
      expect(ids).toContain('viewport.screenshot');
      expect(ids).toContain('template.execute');
    });

    it('returns empty array after populateRegistry with empty list', () => {
      populateRegistry([]);
      expect(listSpecs()).toHaveLength(0);
    });
  });

  // ──────────────────────────────────────────────────────────────────────────
  // dispatchCommand
  // ──────────────────────────────────────────────────────────────────────────

  describe('dispatchCommand', () => {
    it('throws for unknown command', async () => {
      await expect(dispatchCommand('nonexistent.cmd')).rejects.toThrow('Unknown command');
    });

    it('calls registered TypeScript handler', async () => {
      const handler = vi.fn().mockResolvedValue(undefined);
      registerCommandHandler('file.save_scene', handler);
      await dispatchCommand('file.save_scene');
      expect(handler).toHaveBeenCalledOnce();
    });

    it('passes args to TypeScript handler', async () => {
      const handler = vi.fn().mockResolvedValue(undefined);
      registerCommandHandler('file.save_scene', handler);
      await dispatchCommand('file.save_scene', { extra: true });
      expect(handler).toHaveBeenCalledWith({ extra: true });
    });

    it('routes to Rust when no TS handler registered', async () => {
      await dispatchCommand('template.execute');
      expect(commands.runCommand).toHaveBeenCalledWith('template.execute', null);
    });

    it('passes args to Rust command', async () => {
      await dispatchCommand('template.execute', { key: 'value' });
      expect(commands.runCommand).toHaveBeenCalledWith('template.execute', { key: 'value' });
    });

    it('passes null to Rust when args is undefined', async () => {
      await dispatchCommand('template.execute');
      expect(commands.runCommand).toHaveBeenCalledWith('template.execute', null);
    });

    it('TS handler takes priority over Rust route', async () => {
      const handler = vi.fn().mockResolvedValue(undefined);
      registerCommandHandler('template.execute', handler);
      await dispatchCommand('template.execute');
      expect(handler).toHaveBeenCalledOnce();
      expect(commands.runCommand).not.toHaveBeenCalled();
    });

    it('routes returns_data command to Rust when no TS handler', async () => {
      await dispatchCommand('viewport.screenshot');
      expect(commands.runCommand).toHaveBeenCalledWith('viewport.screenshot', null);
    });
  });
});

describe('undo verifier', () => {
  beforeEach(() => {
    _resetRegistryForTesting();
    vi.clearAllMocks();
  });

  it('warns when non_undoable=false command handler skips undo', async () => {
    const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
    populateRegistry([{
      ...sampleSpecs[0], // reuse any spec as base
      id: 'test.undoable',
      non_undoable: false
    }]);

    setUndoVerifier(() => false);

    const handler = vi.fn().mockResolvedValue(undefined);
    registerCommandHandler('test.undoable', handler);

    await dispatchCommand('test.undoable');

    expect(warnSpy).toHaveBeenCalledWith(
      expect.stringContaining("no undo operation was recorded")
    );
    warnSpy.mockRestore();
  });

  it('does not warn when verifier returns true', async () => {
    const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
    populateRegistry([{
      ...sampleSpecs[0],
      id: 'test.undoable2',
      non_undoable: false
    }]);

    setUndoVerifier(() => true);

    const handler = vi.fn().mockResolvedValue(undefined);
    registerCommandHandler('test.undoable2', handler);

    await dispatchCommand('test.undoable2');

    expect(warnSpy).not.toHaveBeenCalled();
    warnSpy.mockRestore();
  });
});
