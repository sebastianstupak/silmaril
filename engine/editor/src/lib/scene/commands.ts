// Scene commands — unified API called by UI, menus, keyboard shortcuts, and
// AI agents. Every scene manipulation goes through one of these functions so
// that the undo system (when wired) has a single choke-point.

import type { EntityInfo } from '$lib/api';
import {
  getSceneState,
  getEntityById,
  _mutate,
  _resetState,
  defaultCamera,
  type SceneEntity,
  type SceneTool,
  type Vec3,
} from './state';

// ---------------------------------------------------------------------------
// Entity colour palette (matches viewport rendering)
// ---------------------------------------------------------------------------

const ENTITY_COLORS = [
  '#e06c75', '#61afef', '#98c379', '#e5c07b',
  '#c678dd', '#56b6c2', '#d19a66', '#be5046',
];

// ---------------------------------------------------------------------------
// Entity manipulation
// ---------------------------------------------------------------------------

/** Select an entity by id, or deselect by passing null. */
export function selectEntity(id: number | null): void {
  _mutate((s) => ({ ...s, selectedEntityId: id }));
}

/** Create a new entity with an optional name. Returns the new entity info. */
export function createEntity(name?: string): SceneEntity {
  let created!: SceneEntity;
  _mutate((s) => {
    const id = s.nextEntityId;
    const entity: SceneEntity = {
      id,
      name: name ?? `Entity ${id}`,
      components: ['Transform'],
      position: {
        x: (Math.random() - 0.5) * 10,
        y: 0,
        z: (Math.random() - 0.5) * 10,
      },
      rotation: { x: 0, y: 0, z: 0 },
      scale: { x: 1, y: 1, z: 1 },
      visible: true,
      locked: false,
    };
    created = entity;
    return {
      ...s,
      entities: [...s.entities, entity],
      nextEntityId: id + 1,
      selectedEntityId: id,
    };
  });
  return created;
}

/** Delete an entity by id. Deselects if currently selected. */
export function deleteEntity(id: number): void {
  _mutate((s) => ({
    ...s,
    entities: s.entities.filter((e) => e.id !== id),
    selectedEntityId: s.selectedEntityId === id ? null : s.selectedEntityId,
  }));
}

/** Duplicate an entity by id. Returns the new copy. */
export function duplicateEntity(id: number): SceneEntity | null {
  const source = getEntityById(id);
  if (!source) return null;

  let created!: SceneEntity;
  _mutate((s) => {
    const newId = s.nextEntityId;
    const copy: SceneEntity = {
      ...structuredClone(source),
      id: newId,
      name: `${source.name} (copy)`,
      position: {
        x: source.position.x + 1,
        y: source.position.y,
        z: source.position.z,
      },
    };
    created = copy;
    return {
      ...s,
      entities: [...s.entities, copy],
      nextEntityId: newId + 1,
      selectedEntityId: newId,
    };
  });
  return created;
}

/** Rename an entity. */
export function renameEntity(id: number, name: string): void {
  _mutate((s) => ({
    ...s,
    entities: s.entities.map((e) => (e.id === id ? { ...e, name } : e)),
  }));
}

// ---------------------------------------------------------------------------
// Transform manipulation
// ---------------------------------------------------------------------------

/** Set absolute position of an entity. */
export function moveEntity(id: number, x: number, y: number, z: number): void {
  _mutate((s) => ({
    ...s,
    entities: s.entities.map((e) =>
      e.id === id ? { ...e, position: { x, y, z } } : e,
    ),
  }));
}

/** Set absolute rotation of an entity (euler degrees). */
export function rotateEntity(id: number, rx: number, ry: number, rz: number): void {
  _mutate((s) => ({
    ...s,
    entities: s.entities.map((e) =>
      e.id === id ? { ...e, rotation: { x: rx, y: ry, z: rz } } : e,
    ),
  }));
}

/** Set absolute scale of an entity. */
export function scaleEntity(id: number, sx: number, sy: number, sz: number): void {
  _mutate((s) => ({
    ...s,
    entities: s.entities.map((e) =>
      e.id === id ? { ...e, scale: { x: sx, y: sy, z: sz } } : e,
    ),
  }));
}

/** Translate an entity by a delta (relative move). */
export function translateEntity(id: number, dx: number, dy: number, dz: number): void {
  _mutate((s) => ({
    ...s,
    entities: s.entities.map((e) =>
      e.id === id
        ? { ...e, position: { x: e.position.x + dx, y: e.position.y + dy, z: e.position.z + dz } }
        : e,
    ),
  }));
}

/** Rotate an entity by a delta (relative rotation in degrees). */
export function rotateEntityBy(id: number, drx: number, dry: number, drz: number): void {
  _mutate((s) => ({
    ...s,
    entities: s.entities.map((e) =>
      e.id === id
        ? { ...e, rotation: { x: e.rotation.x + drx, y: e.rotation.y + dry, z: e.rotation.z + drz } }
        : e,
    ),
  }));
}

/** Scale an entity by a multiplicative factor (relative scale). */
export function scaleEntityBy(id: number, fx: number, fy: number, fz: number): void {
  _mutate((s) => ({
    ...s,
    entities: s.entities.map((e) =>
      e.id === id
        ? { ...e, scale: { x: e.scale.x * fx, y: e.scale.y * fy, z: e.scale.z * fz } }
        : e,
    ),
  }));
}

// ---------------------------------------------------------------------------
// Camera controls
// ---------------------------------------------------------------------------

/** Pan the camera by a delta in world-space. */
export function panCamera(dx: number, dy: number): void {
  _mutate((s) => ({
    ...s,
    camera: {
      ...s.camera,
      position: {
        x: s.camera.position.x + dx,
        y: s.camera.position.y + dy,
        z: s.camera.position.z,
      },
      target: {
        x: s.camera.target.x + dx,
        y: s.camera.target.y + dy,
        z: s.camera.target.z,
      },
    },
  }));
}

/** Orbit the camera around the target by delta angles (degrees). */
export function orbitCamera(dx: number, dy: number): void {
  _mutate((s) => {
    const cam = s.camera;
    // Simplified orbit: adjust position relative to target using angular deltas
    const dist = Math.sqrt(
      (cam.position.x - cam.target.x) ** 2 +
      (cam.position.y - cam.target.y) ** 2 +
      (cam.position.z - cam.target.z) ** 2,
    );
    const theta = Math.atan2(
      cam.position.x - cam.target.x,
      cam.position.z - cam.target.z,
    ) + (dx * Math.PI) / 180;
    const phi = Math.acos(
      Math.max(-1, Math.min(1, (cam.position.y - cam.target.y) / Math.max(dist, 0.001))),
    ) + (dy * Math.PI) / 180;
    const clampedPhi = Math.max(0.1, Math.min(Math.PI - 0.1, phi));

    return {
      ...s,
      camera: {
        ...cam,
        position: {
          x: cam.target.x + dist * Math.sin(clampedPhi) * Math.sin(theta),
          y: cam.target.y + dist * Math.cos(clampedPhi),
          z: cam.target.z + dist * Math.sin(clampedPhi) * Math.cos(theta),
        },
      },
    };
  });
}

/** Zoom the camera (positive = zoom in, negative = zoom out). */
export function zoomCamera(delta: number): void {
  _mutate((s) => ({
    ...s,
    camera: {
      ...s.camera,
      zoom: Math.max(0.1, Math.min(10, s.camera.zoom + delta)),
    },
  }));
}

/** Focus the camera on a specific entity. */
export function focusEntity(id: number): void {
  const entity = getEntityById(id);
  if (!entity) return;

  _mutate((s) => ({
    ...s,
    selectedEntityId: id,
    camera: {
      ...s.camera,
      target: { ...entity.position },
      position: {
        x: entity.position.x,
        y: entity.position.y + 5,
        z: entity.position.z + 10,
      },
    },
  }));
}

/** Reset camera to default view. */
export function resetCamera(): void {
  _mutate((s) => ({
    ...s,
    camera: defaultCamera(),
  }));
}

// ---------------------------------------------------------------------------
// Tool system
// ---------------------------------------------------------------------------

/** Switch the active tool (select, move, rotate, scale). */
export function setActiveTool(tool: SceneTool): void {
  _mutate((s) => ({ ...s, activeTool: tool }));
}

// ---------------------------------------------------------------------------
// Grid settings
// ---------------------------------------------------------------------------

/** Toggle grid visibility. */
export function toggleGrid(): void {
  _mutate((s) => ({ ...s, gridVisible: !s.gridVisible }));
}

/** Toggle snap-to-grid. */
export function toggleSnapToGrid(): void {
  _mutate((s) => ({ ...s, snapToGrid: !s.snapToGrid }));
}

/** Set grid cell size. */
export function setGridSize(size: number): void {
  _mutate((s) => ({ ...s, gridSize: Math.max(0.1, size) }));
}

// ---------------------------------------------------------------------------
// Scene management
// ---------------------------------------------------------------------------

/** Reset to an empty scene. */
export function newScene(): void {
  _resetState();
}

/** Populate scene entities from project scan results.
 *  Assigns random positions so they appear spread out in the viewport. */
export function populateFromScan(infos: EntityInfo[]): void {
  _mutate((s) => {
    const entities: SceneEntity[] = infos.map((info, i) => ({
      ...info,
      position: {
        x: (Math.random() - 0.5) * 10,
        y: 0,
        z: (Math.random() - 0.5) * 10,
      },
      rotation: { x: 0, y: 0, z: 0 },
      scale: { x: 1, y: 1, z: 1 },
      visible: true,
      locked: false,
    }));
    const maxId = entities.reduce((m, e) => Math.max(m, e.id), 0);
    return {
      ...s,
      entities,
      nextEntityId: maxId + 1,
      selectedEntityId: null,
    };
  });
}

// ---------------------------------------------------------------------------
// Command dispatcher — maps string command names to functions
// Used by the Tauri bridge for AI agent access.
// ---------------------------------------------------------------------------

export function dispatchSceneCommand(
  command: string,
  args: Record<string, unknown>,
): unknown {
  switch (command) {
    case 'select_entity':
      selectEntity((args.id as number) ?? null);
      return { ok: true };

    case 'create_entity':
      return createEntity(args.name as string | undefined);

    case 'delete_entity':
      deleteEntity(args.id as number);
      return { ok: true };

    case 'duplicate_entity':
      return duplicateEntity(args.id as number);

    case 'rename_entity':
      renameEntity(args.id as number, args.name as string);
      return { ok: true };

    case 'move_entity':
      moveEntity(
        args.id as number,
        args.x as number,
        args.y as number,
        args.z as number,
      );
      return { ok: true };

    case 'rotate_entity':
      rotateEntity(
        args.id as number,
        args.rx as number,
        args.ry as number,
        args.rz as number,
      );
      return { ok: true };

    case 'scale_entity':
      scaleEntity(
        args.id as number,
        args.sx as number,
        args.sy as number,
        args.sz as number,
      );
      return { ok: true };

    case 'pan_camera':
      panCamera(args.dx as number, args.dy as number);
      return { ok: true };

    case 'orbit_camera':
      orbitCamera(args.dx as number, args.dy as number);
      return { ok: true };

    case 'zoom_camera':
      zoomCamera(args.delta as number);
      return { ok: true };

    case 'focus_entity':
      focusEntity(args.id as number);
      return { ok: true };

    case 'reset_camera':
      resetCamera();
      return { ok: true };

    case 'set_tool':
      setActiveTool(args.tool as SceneTool);
      return { ok: true };

    case 'toggle_grid':
      toggleGrid();
      return { ok: true };

    case 'toggle_snap':
      toggleSnapToGrid();
      return { ok: true };

    case 'new_scene':
      newScene();
      return { ok: true };

    case 'get_state':
      return getSceneState();

    default:
      return { error: `Unknown scene command: ${command}` };
  }
}
