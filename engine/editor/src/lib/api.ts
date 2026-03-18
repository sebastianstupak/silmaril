// Tauri API wrapper with browser fallback for Playwright testing

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

function browserMock<T>(cmd: string, _args?: Record<string, unknown>): T {
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
  };
  return (mocks[cmd] ?? null) as T;
}

export async function getEditorState(): Promise<EditorState> {
  return tauriInvoke<EditorState>('get_editor_state');
}

export async function openProject(path: string): Promise<EditorState> {
  return tauriInvoke<EditorState>('open_project', { path });
}
