import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

// ──────────────────────────────────────────────────────────────────────────────
// Helpers
// ──────────────────────────────────────────────────────────────────────────────

/** Load the api module fresh (after resetting modules + setting up mocks). */
async function loadApi() {
  const mod = await import('$lib/api');
  return mod;
}

// ──────────────────────────────────────────────────────────────────────────────
// Tests: non-Tauri environment (default jsdom, isTauri=false)
// ──────────────────────────────────────────────────────────────────────────────

describe('api — non-Tauri environment', () => {
  // In jsdom, window.__TAURI_INTERNALS__ is undefined → isTauri=false
  // All functions should be no-ops (resolve without invoking anything)

  it('viewportSetGridVisible resolves without error when not in Tauri', async () => {
    const { viewportSetGridVisible } = await loadApi();
    await expect(viewportSetGridVisible('vp-1', true)).resolves.toBeUndefined();
  });

  it('viewportSetGridVisible(false) resolves without error', async () => {
    const { viewportSetGridVisible } = await loadApi();
    await expect(viewportSetGridVisible('vp-1', false)).resolves.toBeUndefined();
  });

  it('viewportCameraSetOrientation resolves without error when not in Tauri', async () => {
    const { viewportCameraSetOrientation } = await loadApi();
    await expect(viewportCameraSetOrientation('vp-1', 0.5, -0.3)).resolves.toBeUndefined();
  });

  it('viewportCameraOrbit resolves without error when not in Tauri', async () => {
    const { viewportCameraOrbit } = await loadApi();
    await expect(viewportCameraOrbit('vp-1', 10, 5)).resolves.toBeUndefined();
  });

  it('viewportCameraReset resolves without error when not in Tauri', async () => {
    const { viewportCameraReset } = await loadApi();
    await expect(viewportCameraReset('vp-1')).resolves.toBeUndefined();
  });
});

// ──────────────────────────────────────────────────────────────────────────────
// Tests: Tauri environment (isTauri=true, invoke mocked)
// ──────────────────────────────────────────────────────────────────────────────

describe('api — Tauri environment', () => {
  let mockInvoke: ReturnType<typeof vi.fn>;

  beforeEach(async () => {
    // Set up Tauri detection before module re-import
    (window as any).__TAURI_INTERNALS__ = {};
    mockInvoke = vi.fn().mockResolvedValue(undefined);
    // Reset module registry so api.ts re-evaluates isTauri with the new window state
    vi.resetModules();
    // Mock @tauri-apps/api/core BEFORE importing api (doMock not hoisted)
    vi.doMock('@tauri-apps/api/core', () => ({ invoke: mockInvoke }));
  });

  afterEach(() => {
    delete (window as any).__TAURI_INTERNALS__;
    vi.resetModules();
    vi.restoreAllMocks();
  });

  it('viewportSetGridVisible calls invoke with correct command and args', async () => {
    const { viewportSetGridVisible } = await import('$lib/api');
    await viewportSetGridVisible('vp-1', true);
    expect(mockInvoke).toHaveBeenCalledWith('viewport_set_grid_visible', {
      viewportId: 'vp-1',
      visible: true,
    });
  });

  it('viewportSetGridVisible(false) passes false to invoke', async () => {
    const { viewportSetGridVisible } = await import('$lib/api');
    await viewportSetGridVisible('vp-2', false);
    expect(mockInvoke).toHaveBeenCalledWith('viewport_set_grid_visible', {
      viewportId: 'vp-2',
      visible: false,
    });
  });

  it('viewportCameraSetOrientation calls invoke with correct command and args', async () => {
    const { viewportCameraSetOrientation } = await import('$lib/api');
    await viewportCameraSetOrientation('vp-1', -Math.PI / 2, 0.5);
    expect(mockInvoke).toHaveBeenCalledWith('viewport_camera_set_orientation', {
      viewportId: 'vp-1',
      yaw: -Math.PI / 2,
      pitch: 0.5,
    });
  });

  it('viewportCameraSetOrientation snap to +X face (yaw=-PI/2, pitch=0)', async () => {
    const { viewportCameraSetOrientation } = await import('$lib/api');
    await viewportCameraSetOrientation('vp-1', -Math.PI / 2, 0);
    expect(mockInvoke).toHaveBeenCalledWith('viewport_camera_set_orientation', {
      viewportId: 'vp-1',
      yaw: -Math.PI / 2,
      pitch: 0,
    });
  });

  it('viewportCameraSetOrientation snap to top face (pitch=-1.5)', async () => {
    const { viewportCameraSetOrientation } = await import('$lib/api');
    await viewportCameraSetOrientation('vp-1', 0, -1.5);
    expect(mockInvoke).toHaveBeenCalledWith('viewport_camera_set_orientation', {
      viewportId: 'vp-1',
      yaw: 0,
      pitch: -1.5,
    });
  });

  it('viewportCameraOrbit calls invoke with correct command and deltas', async () => {
    const { viewportCameraOrbit } = await import('$lib/api');
    await viewportCameraOrbit('vp-1', 15, -8);
    expect(mockInvoke).toHaveBeenCalledWith('viewport_camera_orbit', {
      viewportId: 'vp-1',
      dx: 15,
      dy: -8,
    });
  });

  it('viewportCameraReset calls invoke with correct command', async () => {
    const { viewportCameraReset } = await import('$lib/api');
    await viewportCameraReset('vp-1');
    expect(mockInvoke).toHaveBeenCalledWith('viewport_camera_reset', {
      viewportId: 'vp-1',
    });
  });

  it('each call invokes exactly once', async () => {
    const { viewportSetGridVisible, viewportCameraSetOrientation } = await import('$lib/api');
    await viewportSetGridVisible('vp-1', true);
    await viewportCameraSetOrientation('vp-1', 0, 0);
    expect(mockInvoke).toHaveBeenCalledTimes(2);
  });
});
