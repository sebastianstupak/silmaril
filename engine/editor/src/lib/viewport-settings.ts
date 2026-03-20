/** Per-viewport UI settings persisted in localStorage.
 *
 * Keyed by the stable dock panel ID (e.g. "viewport", "viewport:2") so
 * settings survive panel moves, tab switches, pop-out/dock-back transitions,
 * and full page reloads.
 */

export interface ViewportUISettings {
  activeTool: string;
  gridVisible: boolean;
  snapToGrid: boolean;
  projection: string;
  cameraZoom?: number;
  cameraYawRad?: number;
  cameraPitchRad?: number;
}

const PREFIX = 'vp-ui:';

export function saveViewportSettings(viewportId: string, settings: ViewportUISettings): void {
  try {
    localStorage.setItem(`${PREFIX}${viewportId}`, JSON.stringify(settings));
  } catch {
    // localStorage unavailable (private browsing with strict settings, etc.)
  }
}

export function loadViewportSettings(viewportId: string): ViewportUISettings | null {
  try {
    const raw = localStorage.getItem(`${PREFIX}${viewportId}`);
    if (!raw) return null;
    return JSON.parse(raw) as ViewportUISettings;
  } catch {
    return null;
  }
}

export function clearViewportSettings(viewportId: string): void {
  try {
    localStorage.removeItem(`${PREFIX}${viewportId}`);
  } catch {
    // ignore
  }
}
