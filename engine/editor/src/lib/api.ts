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
    get_viewport_frame: generateMockSvg(args),
    pick_viewport_entity: mockPickEntity(args),
  };
  return (mocks[cmd] ?? null) as T;
}

/** Generate a simple mock SVG for browser/Playwright testing. */
function generateMockSvg(args?: Record<string, unknown>): string {
  const req = (args?.request ?? {}) as Partial<ViewportFrameRequest>;
  const w = req.width ?? 800;
  const h = req.height ?? 600;
  const entities = req.entities ?? [];
  const selectedId = req.selected_entity_id ?? null;
  const tool = req.tool ?? 'select';

  let svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${w}" height="${h}" viewBox="0 0 ${w} ${h}" style="display:block">`;
  svg += `<rect width="${w}" height="${h}" fill="#1a1a2e"/>`;

  // Grid
  for (let x = 0; x < w; x += 50) {
    svg += `<line x1="${x}" y1="0" x2="${x}" y2="${h}" stroke="#252545" stroke-width="0.5"/>`;
  }
  for (let y = 0; y < h; y += 50) {
    svg += `<line x1="0" y1="${y}" x2="${w}" y2="${y}" stroke="#252545" stroke-width="0.5"/>`;
  }

  // Entities
  for (const e of entities) {
    const px = w / 2 + (e.x - 0.5) * w;
    const py = h / 2 + (e.y - 0.5) * h;
    const sel = selectedId === e.id;
    if (sel) {
      svg += `<circle cx="${px}" cy="${py}" r="19" fill="none" stroke="#61afef" stroke-width="2.5"/>`;
    }
    svg += `<circle cx="${px}" cy="${py}" r="${sel ? 14 : 10}" fill="${e.color}" stroke="${sel ? '#fff' : '#aaa'}" stroke-width="${sel ? 2 : 0.8}" opacity="0.9" data-entity-id="${e.id}"/>`;
    svg += `<text x="${px}" y="${py + 24}" fill="#ccc" font-family="sans-serif" font-size="10" text-anchor="middle">${e.name}</text>`;

    // Draw gizmo on selected entity
    if (sel && tool !== 'select') {
      svg += generateMockGizmo(px, py, tool);
    }
  }

  if (entities.length === 0) {
    svg += `<text x="${w / 2}" y="${h / 2}" fill="#555" font-family="sans-serif" font-size="14" text-anchor="middle" dominant-baseline="middle">No entities in scene</text>`;
  }

  svg += '</svg>';
  return svg;
}

/** Generate gizmo SVG elements for the mock renderer. */
function generateMockGizmo(cx: number, cy: number, tool: string): string {
  let g = '';
  if (tool === 'move') {
    const len = 40;
    // X axis (red arrow)
    g += `<line x1="${cx}" y1="${cy}" x2="${cx + len}" y2="${cy}" stroke="#ff4444" stroke-width="2"/>`;
    g += `<polygon points="${cx + len},${cy - 4} ${cx + len},${cy + 4} ${cx + len + 8},${cy}" fill="#ff4444"/>`;
    // Y axis (green arrow)
    g += `<line x1="${cx}" y1="${cy}" x2="${cx}" y2="${cy - len}" stroke="#44ff44" stroke-width="2"/>`;
    g += `<polygon points="${cx - 4},${cy - len} ${cx + 4},${cy - len} ${cx},${cy - len - 8}" fill="#44ff44"/>`;
    // Z axis (blue dashed)
    g += `<line x1="${cx}" y1="${cy}" x2="${cx - len * 0.5}" y2="${cy + len * 0.5}" stroke="#4444ff" stroke-width="2" stroke-dasharray="4,2"/>`;
    // Centre square
    g += `<rect x="${cx - 4}" y="${cy - 4}" width="8" height="8" fill="yellow" opacity="0.6"/>`;
  } else if (tool === 'rotate') {
    const r = 35;
    g += `<ellipse cx="${cx}" cy="${cy}" rx="${r}" ry="${r * 0.3}" fill="none" stroke="#ff4444" stroke-width="1.5"/>`;
    g += `<ellipse cx="${cx}" cy="${cy}" rx="${r * 0.3}" ry="${r}" fill="none" stroke="#44ff44" stroke-width="1.5"/>`;
    g += `<circle cx="${cx}" cy="${cy}" r="${r + 5}" fill="none" stroke="white" stroke-width="1" opacity="0.5"/>`;
  } else if (tool === 'scale') {
    const len = 35;
    const cube = 5;
    g += `<line x1="${cx}" y1="${cy}" x2="${cx + len}" y2="${cy}" stroke="#ff4444" stroke-width="2"/>`;
    g += `<rect x="${cx + len - cube / 2}" y="${cy - cube / 2}" width="${cube}" height="${cube}" fill="#ff4444"/>`;
    g += `<line x1="${cx}" y1="${cy}" x2="${cx}" y2="${cy - len}" stroke="#44ff44" stroke-width="2"/>`;
    g += `<rect x="${cx - cube / 2}" y="${cy - len - cube / 2}" width="${cube}" height="${cube}" fill="#44ff44"/>`;
    g += `<rect x="${cx - cube / 2}" y="${cy - cube / 2}" width="${cube}" height="${cube}" fill="white" opacity="0.8"/>`;
  }
  return g;
}

/** Mock entity picking for browser testing. */
function mockPickEntity(args?: Record<string, unknown>): number | null {
  const req = (args?.request ?? {}) as Partial<PickEntityRequest>;
  const cx = req.click_x ?? 0;
  const cy = req.click_y ?? 0;
  const w = req.width ?? 800;
  const h = req.height ?? 600;
  const entities = req.entities ?? [];

  for (const e of entities) {
    const px = w / 2 + (e.x - 0.5) * w;
    const py = h / 2 + (e.y - 0.5) * h;
    const dx = cx - px;
    const dy = cy - py;
    if (dx * dx + dy * dy <= 14 * 14) {
      return e.id;
    }
  }
  return null;
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
// Viewport
// ---------------------------------------------------------------------------

export interface ViewportEntity {
  id: number;
  name: string;
  x: number;
  y: number;
  color: string;
}

export interface ViewportCamera {
  offset_x: number;
  offset_y: number;
  zoom: number;
}

export interface ViewportFrameRequest {
  width: number;
  height: number;
  selected_entity_id: number | null;
  camera: ViewportCamera | null;
  entities: ViewportEntity[];
  /** Active tool: controls which gizmo is drawn on the selected entity. */
  tool: string;
}

export interface PickEntityRequest {
  click_x: number;
  click_y: number;
  width: number;
  height: number;
  entities: ViewportEntity[];
  camera: ViewportCamera | null;
}

/** Generate an SVG viewport frame from the backend. */
export async function getViewportFrame(request: ViewportFrameRequest): Promise<string> {
  return tauriInvoke<string>('get_viewport_frame', { request });
}

/** Pick the entity at the given click coordinates. */
export async function pickViewportEntity(request: PickEntityRequest): Promise<number | null> {
  return tauriInvoke<number | null>('pick_viewport_entity', { request });
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
