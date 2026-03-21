import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('../dispatch', () => ({
  registerCommandHandler: vi.fn(),
}));

// Mock bindings so file/asset/viewport/template handlers don't fail on import
vi.mock('../bindings', () => ({
  commands: {
    runCommand: vi.fn(),
    listCommands: vi.fn(),
  },
}));

// Mock undo-history store so edit handlers don't fail on import
vi.mock('../stores/undo-history', () => ({
  undo: vi.fn(),
  redo: vi.fn(),
}));

import { registerCommandHandler } from '../dispatch';
import { registerAllHandlers } from './index';

describe('registerAllHandlers', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('registers handlers for all known command ids', () => {
    registerAllHandlers();
    const registered = (registerCommandHandler as ReturnType<typeof vi.fn>)
      .mock.calls.map(([id]: [string]) => id);

    const expected = [
      'file.save_scene',
      'file.save_scene_as',
      'file.open_scene',
      'file.new_project',
      'file.open_project',
      'edit.undo',
      'edit.redo',
      'view.toggle_hierarchy',
      'view.toggle_inspector',
      'view.toggle_console',
      'view.toggle_asset_browser',
      'view.zoom_in',
      'view.zoom_out',
      'view.zoom_reset',
      'scene.new_entity',
      'scene.delete_entity',
      'scene.duplicate_entity',
      'scene.focus_entity',
      'asset.scan',
      'asset.import',
      'asset.refresh',
      'build.run',
      'build.build',
      'build.package',
      'viewport.screenshot',
      'viewport.toggle_grid',
      'viewport.toggle_gizmos',
      'template.open',
      'template.close',
      'template.execute',
      'template.undo',
      'template.redo',
      'template.history',
    ];

    for (const id of expected) {
      expect(registered, `Missing handler for ${id}`).toContain(id);
    }
  });
});
