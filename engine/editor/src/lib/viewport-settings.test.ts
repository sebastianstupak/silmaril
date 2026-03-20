import { describe, it, expect, beforeEach } from 'vitest';
import {
  saveViewportSettings,
  loadViewportSettings,
  clearViewportSettings,
  type ViewportUISettings,
} from './viewport-settings';

const defaults: ViewportUISettings = {
  activeTool: 'select',
  gridVisible: true,
  snapToGrid: false,
  projection: 'perspective',
};

describe('viewport-settings', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it('returns null for an unknown viewportId', () => {
    expect(loadViewportSettings('viewport')).toBeNull();
  });

  it('round-trips all fields through localStorage', () => {
    const s: ViewportUISettings = {
      activeTool: 'move',
      gridVisible: false,
      snapToGrid: true,
      projection: 'ortho',
    };
    saveViewportSettings('viewport', s);
    expect(loadViewportSettings('viewport')).toEqual(s);
  });

  it('isolates settings per viewportId', () => {
    saveViewportSettings('viewport', { ...defaults, activeTool: 'rotate' });
    saveViewportSettings('viewport:2', { ...defaults, activeTool: 'scale' });

    expect(loadViewportSettings('viewport')?.activeTool).toBe('rotate');
    expect(loadViewportSettings('viewport:2')?.activeTool).toBe('scale');
  });

  it('overwrites on repeated save for same viewportId', () => {
    saveViewportSettings('viewport', { ...defaults, activeTool: 'select' });
    saveViewportSettings('viewport', { ...defaults, activeTool: 'move' });
    expect(loadViewportSettings('viewport')?.activeTool).toBe('move');
  });

  it('clearViewportSettings removes the entry', () => {
    saveViewportSettings('viewport', defaults);
    clearViewportSettings('viewport');
    expect(loadViewportSettings('viewport')).toBeNull();
  });

  it('clearViewportSettings does not affect other viewports', () => {
    saveViewportSettings('viewport', defaults);
    saveViewportSettings('viewport:2', defaults);
    clearViewportSettings('viewport');
    expect(loadViewportSettings('viewport:2')).not.toBeNull();
  });

  it('ignores corrupted localStorage entry gracefully', () => {
    localStorage.setItem('vp-ui:viewport', 'not-json{{{');
    expect(loadViewportSettings('viewport')).toBeNull();
  });

  it('round-trips cameraYawRad and cameraPitchRad', () => {
    const s: ViewportUISettings = {
      activeTool: 'select',
      gridVisible: true,
      snapToGrid: false,
      projection: 'persp',
      cameraYawRad: -0.785,
      cameraPitchRad: 0.523,
    };
    saveViewportSettings('viewport', s);
    const loaded = loadViewportSettings('viewport');
    expect(loaded?.cameraYawRad).toBeCloseTo(-0.785, 3);
    expect(loaded?.cameraPitchRad).toBeCloseTo(0.523, 3);
  });

  it('loads settings without cameraYawRad/cameraPitchRad gracefully', () => {
    const s: ViewportUISettings = {
      activeTool: 'select',
      gridVisible: true,
      snapToGrid: false,
      projection: 'persp',
    };
    saveViewportSettings('viewport', s);
    const loaded = loadViewportSettings('viewport');
    expect(loaded?.cameraYawRad).toBeUndefined();
    expect(loaded?.cameraPitchRad).toBeUndefined();
  });
});
