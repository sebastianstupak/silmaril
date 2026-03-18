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
