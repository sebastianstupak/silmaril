// Central scene state — single source of truth for the editor scene
// Uses a simple pub/sub pattern (not Svelte stores) so it works from any
// context: UI components, keyboard shortcuts, menu items, and AI agent
// commands via Tauri.

import type { EntityInfo } from '$lib/api';
import type { EntityComponentValues } from '$lib/inspector/inspector-utils';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface Vec3 {
  x: number;
  y: number;
  z: number;
}

export interface SceneEntity extends EntityInfo {
  position: Vec3;
  rotation: Vec3;
  scale: Vec3;
  visible: boolean;
  locked: boolean;
  /** Live field values for all components, keyed by component type name. */
  componentValues: EntityComponentValues;
}

export type ProjectionMode = 'ortho' | 'perspective';

export interface SceneCamera {
  position: Vec3;
  target: Vec3;
  zoom: number;
  fov: number;
  viewAngle: number; // 2D rotation angle in radians (0 = north-up)
  projection: ProjectionMode;
}

export type SceneTool = 'select' | 'move' | 'rotate' | 'scale';

export interface SceneState {
  entities: SceneEntity[];
  selectedEntityId: number | null;
  camera: SceneCamera;
  activeTool: SceneTool;
  gridVisible: boolean;
  snapToGrid: boolean;
  gridSize: number;
  nextEntityId: number;
}

// ---------------------------------------------------------------------------
// Default state
// ---------------------------------------------------------------------------

function defaultCamera(): SceneCamera {
  return {
    position: { x: 0, y: 5, z: 10 },
    target: { x: 0, y: 0, z: 0 },
    zoom: 1,
    fov: 60,
    viewAngle: 0,
    projection: 'perspective',
  };
}

function createDefaultState(): SceneState {
  return {
    entities: [],
    selectedEntityId: null,
    camera: defaultCamera(),
    activeTool: 'select',
    gridVisible: true,
    snapToGrid: false,
    gridSize: 1,
    nextEntityId: 1,
  };
}

// ---------------------------------------------------------------------------
// Module-level state
// ---------------------------------------------------------------------------

let state: SceneState = createDefaultState();

type Listener = () => void;
let listeners: Listener[] = [];

function notify() {
  for (const fn of listeners) fn();
}

// ---------------------------------------------------------------------------
// Getters
// ---------------------------------------------------------------------------

export function getSceneState(): SceneState {
  return state;
}

export function getSelectedEntity(): SceneEntity | null {
  if (state.selectedEntityId == null) return null;
  return state.entities.find((e) => e.id === state.selectedEntityId) ?? null;
}

export function getEntityById(id: number): SceneEntity | null {
  return state.entities.find((e) => e.id === id) ?? null;
}

// ---------------------------------------------------------------------------
// Subscriptions
// ---------------------------------------------------------------------------

export function subscribeScene(fn: Listener): () => void {
  listeners.push(fn);
  return () => {
    listeners = listeners.filter((l) => l !== fn);
  };
}

// ---------------------------------------------------------------------------
// Mutations (used by commands.ts — not exported directly to panels)
// ---------------------------------------------------------------------------

export function _mutate(updater: (s: SceneState) => SceneState): void {
  state = updater(state);
  notify();
}

export function _replaceState(next: SceneState): void {
  state = next;
  notify();
}

export function _resetState(): void {
  state = createDefaultState();
  notify();
}

// ---------------------------------------------------------------------------
// Tauri event listeners — mirror backend ECS changes into frontend state
// ---------------------------------------------------------------------------

let tauriListenersInitialised = false;

/**
 * Initialise Tauri event listeners that keep the frontend scene state in sync
 * with the backend ECS world.  Safe to call multiple times — only the first
 * call registers the listeners.
 */
export async function initTauriListeners(): Promise<void> {
  if (tauriListenersInitialised) return;
  tauriListenersInitialised = true;

  const { listen } = await import('@tauri-apps/api/event');

  await listen<{ id: number; name: string; parentId?: number }>('entity-created', (event) => {
    const { id, name, parentId } = event.payload;
    // Avoid duplicates
    if (state.entities.some((e) => e.id === id)) return;
    const newEntity: SceneEntity = {
      id,
      name,
      components: ['Transform'],
      parentId,
      position: { x: 0, y: 0, z: 0 },
      rotation: { x: 0, y: 0, z: 0 },
      scale: { x: 1, y: 1, z: 1 },
      visible: true,
      locked: false,
      componentValues: {},
    };
    state = {
      ...state,
      entities: [...state.entities, newEntity],
      nextEntityId: Math.max(state.nextEntityId, id + 1),
    };
    notify();
  });

  await listen<{ id: number }>('entity-deleted', (event) => {
    const { id } = event.payload;
    state = {
      ...state,
      entities: state.entities.filter((e) => e.id !== id),
      selectedEntityId: state.selectedEntityId === id ? null : state.selectedEntityId,
    };
    notify();
  });

  await listen<{ entityId: number; newParentId: number | null }>('entity-reparented', (event) => {
    const { entityId, newParentId } = event.payload;
    state = {
      ...state,
      entities: state.entities.map((e) =>
        e.id === entityId
          ? { ...e, parentId: newParentId ?? undefined }
          : e
      ),
    };
    notify();
  });

  await listen<{
    id: number;
    position: [number, number, number];
    rotation: [number, number, number, number];
    scale: [number, number, number];
  }>('entity-transform-changed', (event) => {
    const { id, position, rotation, scale } = event.payload;
    state = {
      ...state,
      entities: state.entities.map((e) => {
        if (e.id !== id) return e;
        return {
          ...e,
          position: { x: position[0], y: position[1], z: position[2] },
          // Store quaternion x/y/z as euler-like values for display;
          // a proper quaternion-to-euler conversion can be added later.
          rotation: { x: rotation[0], y: rotation[1], z: rotation[2] },
          scale: { x: scale[0], y: scale[1], z: scale[2] },
        };
      }),
    };
    notify();
  });
}

export { defaultCamera };
