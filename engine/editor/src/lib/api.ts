// Tauri API wrapper with browser fallback for Playwright testing
import { getPanelInfo } from '$lib/docking/types';

export interface EditorState {
  mode: string;
  project_name: string | null;
  project_path: string | null;
}

export type EntityId = number;

export interface ComponentData {
  type_name: string;
  data: unknown;
}

export interface EntityInfo {
  id: number;
  name: string;
  components: string[];
}

/** Detect if running inside Tauri or standalone browser */
const isTauri = typeof window !== 'undefined' && !!(window as any).__TAURI_INTERNALS__;

async function tauriInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauri) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<T>(cmd, args);
  }
  // Browser fallback — return mock data for Playwright testing
  return browserMock<T>(cmd, args);
}

const mockEntities: EntityInfo[] = [
  { id: 1, name: 'Player', components: ['Transform', 'Health', 'Velocity'] },
  { id: 2, name: 'Enemy', components: ['Transform', 'Health', 'AI'] },
  { id: 3, name: 'Camera', components: ['Transform', 'Camera'] },
  { id: 4, name: 'Light', components: ['Transform', 'PointLight'] },
  { id: 5, name: 'Ground', components: ['Transform', 'MeshRenderer', 'Collider'] },
];

function browserMock<T>(cmd: string, args?: Record<string, unknown>): T {
  const mocks: Record<string, unknown> = {
    get_editor_state: {
      mode: 'edit',
      project_name: null,
      project_path: null,
    } satisfies EditorState,
    open_project: {
      mode: 'edit',
      project_name: 'Mock Project',
      project_path: '/mock/path',
    } satisfies EditorState,
    open_project_dialog: '/mock/project/path',
    scan_project_entities: mockEntities,
  };
  return (mocks[cmd] ?? null) as T;
}

export async function getEditorState(): Promise<EditorState> {
  return tauriInvoke<EditorState>('get_editor_state');
}

export async function openProject(path: string): Promise<EditorState> {
  return tauriInvoke<EditorState>('open_project', { path });
}

export async function openProjectDialog(): Promise<string | null> {
  if (!isTauri) {
    return '/mock/project/path';
  }
  const { open } = await import('@tauri-apps/plugin-dialog');
  const selected = await open({ directory: true, title: 'Open Silmaril Project' });
  return selected as string | null;
}

export async function scanProjectEntities(projectPath: string): Promise<EntityInfo[]> {
  return tauriInvoke<EntityInfo[]>('scan_project_entities', { projectPath });
}

// ---------------------------------------------------------------------------
// Pop-out panel to external window
// ---------------------------------------------------------------------------

/**
 * Pop a panel out into a new Tauri window at the given screen position.
 * In browser mode (Playwright), this is a no-op.
 */
export async function popOutPanel(panelId: string, title: string, x: number, y: number): Promise<void> {
  if (!isTauri) return;

  try {
    await tauriInvoke<void>('create_popout_window', {
      panelId,
      title: `${title} — Silmaril Editor`,
      x: Math.round(x) - 250,
      y: Math.round(y) - 50,
      width: 500,
      height: 600,
    });
  } catch (e) {
    console.error('[silmaril] popOutPanel error:', e);
  }
}

// ---------------------------------------------------------------------------
// Native viewport (child window for Vulkan rendering)
// ---------------------------------------------------------------------------

/** Create the native child window for the Vulkan viewport.
 *  Bounds are in physical (device) pixels. */
export async function createNativeViewport(
  x: number,
  y: number,
  width: number,
  height: number,
): Promise<void> {
  if (!isTauri) return; // no-op in browser mode
  return tauriInvoke<void>('create_native_viewport', { x, y, width, height });
}

/** Reposition/resize the native viewport child window. */
export async function resizeNativeViewport(
  x: number,
  y: number,
  width: number,
  height: number,
): Promise<void> {
  if (!isTauri) return;
  return tauriInvoke<void>('resize_native_viewport', { x, y, width, height });
}

/** Destroy the native viewport child window. */
export async function destroyNativeViewport(): Promise<void> {
  if (!isTauri) return;
  return tauriInvoke<void>('destroy_native_viewport', {});
}

/** Show or hide the native viewport child window.
 *  Used during drag operations so the webview drop zone overlay is visible. */
export async function setViewportVisible(visible: boolean): Promise<void> {
  if (!isTauri) return;
  return tauriInvoke<void>('set_viewport_visible', { visible });
}

// ---------------------------------------------------------------------------
// Scene commands (AI agent API)
// ---------------------------------------------------------------------------

/**
 * Execute a scene command by name with the given arguments.
 * This is the primary entry point for AI agents to manipulate the scene via
 * Tauri. In the browser (Playwright testing), it dispatches locally.
 */
export async function executeSceneCommand(
  command: string,
  args: Record<string, unknown> = {},
): Promise<unknown> {
  if (isTauri) {
    return tauriInvoke<unknown>('scene_command', {
      command,
      args: JSON.stringify(args),
    });
  }
  // Browser fallback — dispatch through local scene commands
  const mod = await import('$lib/scene/commands');
  return mod.dispatchSceneCommand(command, args);
}
