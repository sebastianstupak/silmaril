// Central scene state — single source of truth for the editor scene
// Uses a simple pub/sub pattern (not Svelte stores) so it works from any
// context: UI components, keyboard shortcuts, menu items, and AI agent
// commands via Tauri.

import type { EntityInfo } from '$lib/api';

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
}

export interface SceneCamera {
  position: Vec3;
  target: Vec3;
  zoom: number;
  fov: number;
  viewAngle: number; // 2D rotation angle in radians (0 = north-up)
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

export { defaultCamera };
