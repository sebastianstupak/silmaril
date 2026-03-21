// Tauri API wrapper with browser fallback for Playwright testing
import { getPanelInfo } from '$lib/docking/types';
import type { ComponentSchema } from '$lib/inspector/schema';

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

const mockSchemas: ComponentSchema[] = [
  {
    name: 'Transform',
    label: 'Transform',
    category: 'Core',
    fields: [
      { name: 'position', label: 'Position', field_type: { kind: 'vec3' } },
      { name: 'rotation', label: 'Rotation', field_type: { kind: 'vec3' } },
      { name: 'scale',    label: 'Scale',    field_type: { kind: 'vec3' } },
    ],
  },
  {
    name: 'Health',
    label: 'Health',
    category: 'Core',
    fields: [
      { name: 'current', label: 'Current HP', field_type: { kind: 'f32', min: 0, max: 10000 } },
      { name: 'max',     label: 'Max HP',     field_type: { kind: 'f32', min: 1, max: 10000 } },
    ],
  },
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
    get_component_schemas: mockSchemas,
    set_component_field: null,
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

/** Create (or reposition) a named viewport instance in the calling window.
 *  Bounds are in physical (device) pixels. Safe to call again on remount —
 *  the Rust side upserts so no duplicate Vulkan contexts are created. */
export async function createNativeViewport(
  viewportId: string,
  x: number,
  y: number,
  width: number,
  height: number,
): Promise<void> {
  if (!isTauri) return;
  return tauriInvoke<void>('create_native_viewport', { viewportId, x, y, width, height });
}

/** Update the scissor bounds for an existing viewport instance. */
export async function resizeNativeViewport(
  viewportId: string,
  x: number,
  y: number,
  width: number,
  height: number,
): Promise<void> {
  if (!isTauri) return;
  return tauriInvoke<void>('resize_native_viewport', { viewportId, x, y, width, height });
}

/** Remove a viewport instance from the registry. */
export async function destroyNativeViewport(viewportId: string): Promise<void> {
  if (!isTauri) return;
  return tauriInvoke<void>('destroy_native_viewport', { viewportId });
}

/** Show or hide a specific viewport instance. */
export async function setViewportVisible(viewportId: string, visible: boolean): Promise<void> {
  if (!isTauri) return;
  return tauriInvoke<void>('set_viewport_visible', { viewportId, visible });
}

/** Orbit the camera for a specific viewport instance. */
export async function viewportCameraOrbit(viewportId: string, dx: number, dy: number): Promise<void> {
  if (!isTauri) return;
  return tauriInvoke<void>('viewport_camera_orbit', { viewportId, dx, dy });
}

/** Pan the camera for a specific viewport instance. */
export async function viewportCameraPan(viewportId: string, dx: number, dy: number): Promise<void> {
  if (!isTauri) return;
  return tauriInvoke<void>('viewport_camera_pan', { viewportId, dx, dy });
}

/** Zoom the camera for a specific viewport instance. */
export async function viewportCameraZoom(viewportId: string, delta: number): Promise<void> {
  if (!isTauri) return;
  return tauriInvoke<void>('viewport_camera_zoom', { viewportId, delta });
}

/** Reset the camera for a specific viewport instance to its default state. */
export async function viewportCameraReset(viewportId: string): Promise<void> {
  if (!isTauri) return;
  return tauriInvoke<void>('viewport_camera_reset', { viewportId });
}

/** Show or hide the grid for a specific viewport instance. */
export async function viewportSetGridVisible(viewportId: string, visible: boolean): Promise<void> {
  if (!isTauri) return;
  return tauriInvoke<void>('viewport_set_grid_visible', { viewportId, visible });
}

/** Set absolute camera yaw and pitch for a specific viewport instance.
 *  Used for snap-to-axis — does not apply the mouse-pixel scaling of orbit. */
export async function viewportCameraSetOrientation(
  viewportId: string,
  yaw: number,
  pitch: number,
): Promise<void> {
  if (!isTauri) return;
  return tauriInvoke<void>('viewport_camera_set_orientation', { viewportId, yaw, pitch });
}

/** Switch between perspective (isOrtho=false) and orthographic (isOrtho=true)
 *  projection for a specific viewport instance. */
export async function viewportSetProjection(viewportId: string, isOrtho: boolean): Promise<void> {
  if (!isTauri) return;
  return tauriInvoke<void>('viewport_set_projection', { viewportId, isOrtho });
}

// ---------------------------------------------------------------------------
// Template CQRS types
// ---------------------------------------------------------------------------

export interface TemplateComponent {
  type_name: string;
  data: unknown;
}

export interface TemplateEntity {
  id: number;
  name: string | null;
  components: TemplateComponent[];
}

export interface TemplateState {
  name: string;
  entities: TemplateEntity[];
}

export interface CommandResult {
  action_id: number;
  new_state: TemplateState;
}

export interface ActionSummary {
  action_id: number;
  description: string;
}

export type TemplateCommand =
  | { CreateEntity: { name: string | null } }
  | { DeleteEntity: { id: number } }
  | { RenameEntity: { id: number; name: string | null } }
  | { DuplicateEntity: { id: number } }
  | { SetComponent: { id: number; type_name: string; data: unknown } }
  | { AddComponent: { id: number; type_name: string; data: unknown } }
  | { RemoveComponent: { id: number; type_name: string } };

// ---------------------------------------------------------------------------
// Template IPC calls
// ---------------------------------------------------------------------------

export async function templateOpen(templatePath: string): Promise<TemplateState> {
  return tauriInvoke<TemplateState>('template_open', { templatePath });
}

export async function templateClose(templatePath: string): Promise<void> {
  return tauriInvoke<void>('template_close', { templatePath });
}

export async function templateExecute(
  templatePath: string,
  command: TemplateCommand,
): Promise<CommandResult> {
  return tauriInvoke<CommandResult>('template_execute', { templatePath, command });
}

export async function templateUndo(templatePath: string): Promise<number | null> {
  return tauriInvoke<number | null>('template_undo', { templatePath });
}

export async function templateRedo(templatePath: string): Promise<number | null> {
  return tauriInvoke<number | null>('template_redo', { templatePath });
}

export async function templateHistory(templatePath: string): Promise<ActionSummary[]> {
  return tauriInvoke<ActionSummary[]>('template_history', { templatePath });
}

export async function getComponentSchemas(): Promise<ComponentSchema[]> {
  return tauriInvoke<ComponentSchema[]>('get_component_schemas');
}


export async function scanAssets(
  projectPath: string,
): Promise<{ path: string; asset_type: string }[]> {
  if (!isTauri) return [];
  return tauriInvoke('scan_assets', { projectPath });
}

// ---------------------------------------------------------------------------
// Gizmo IPC
// ---------------------------------------------------------------------------

export interface GizmoHit {
  axis: string;
  mode: string;
}

/** Test whether a screen-space position hits any gizmo handle for an entity.
 *  Returns the hit axis/mode if a handle was hit, or null if not. */
export async function gizmoHitTest(
  viewportId: string,
  screenX: number,
  screenY: number,
  entityId: number,
): Promise<GizmoHit | null> {
  if (!isTauri) return null;
  return tauriInvoke<GizmoHit | null>('gizmo_hit_test', {
    viewportId,
    screenX,
    screenY,
    entityId,
  });
}

/** Apply one mouse-move step to the active gizmo drag. */
export async function gizmoDrag(
  viewportId: string,
  screenX: number,
  screenY: number,
): Promise<void> {
  if (!isTauri) return;
  return tauriInvoke<void>('gizmo_drag', { viewportId, screenX, screenY });
}

/** Finalise an active gizmo drag: clears drag state and pushes undo entry. */
export async function gizmoDragEnd(viewportId: string): Promise<void> {
  if (!isTauri) return;
  return tauriInvoke<void>('gizmo_drag_end', { viewportId });
}

/** Set the active gizmo mode. Accepted values: "move", "rotate", "scale". */
export async function setGizmoMode(mode: 'move' | 'rotate' | 'scale'): Promise<void> {
  if (!isTauri) return;
  return tauriInvoke<void>('set_gizmo_mode', { mode });
}

// ---------------------------------------------------------------------------
// Scene undo / redo
// ---------------------------------------------------------------------------

export interface UndoRedoState {
  canUndo: boolean;
  canRedo: boolean;
}

/** Undo the last scene action. Returns the new undo/redo availability. */
export async function sceneUndo(): Promise<UndoRedoState> {
  if (!isTauri) return { canUndo: false, canRedo: false };
  return tauriInvoke<UndoRedoState>('scene_undo');
}

/** Redo the last undone scene action. Returns the new undo/redo availability. */
export async function sceneRedo(): Promise<UndoRedoState> {
  if (!isTauri) return { canUndo: false, canRedo: false };
  return tauriInvoke<UndoRedoState>('scene_redo');
}
