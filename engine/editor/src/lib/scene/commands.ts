// Scene commands — unified API called by UI, menus, keyboard shortcuts, and
// AI agents. Every scene manipulation goes through one of these functions so
// that the undo system (when wired) has a single choke-point.

import type { EntityInfo } from '$lib/api';
import { createEntityChild as apiCreateEntityChild } from '$lib/api';
import {
  getSceneState,
  getEntityById,
  _mutate,
  _resetState,
  defaultCamera,
  type SceneEntity,
  type SceneTool,
  type ProjectionMode,
  type Vec3,
} from './state';
import { logInfo, logDebug, logError } from '$lib/stores/console';
import { applyComponentDefaults, buildInitialComponentValues, type FieldValue } from '$lib/inspector/inspector-utils';
import { getSchemas } from '$lib/inspector/schema-store';
import { templateExecute } from '$lib/api';
import { getActiveTemplatePath, onTemplateMutated } from '$lib/stores/undo-history';

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
  if (id != null) {
    const entity = getEntityById(id);
    logDebug(`Selected: ${entity?.name ?? id}`);
  }
}

/** Create a new entity with an optional name. Returns the new entity info. */
export function createEntity(name?: string): SceneEntity {
  let created!: SceneEntity;
  _mutate((s) => {
    const id = s.nextEntityId;
    const position = {
      x: (Math.random() - 0.5) * 10,
      y: 0,
      z: (Math.random() - 0.5) * 10,
    };
    const entity: SceneEntity = {
      id,
      name: name ?? `Entity ${id}`,
      components: ['Transform'],
      position,
      rotation: { x: 0, y: 0, z: 0 },
      scale: { x: 1, y: 1, z: 1 },
      visible: true,
      locked: false,
      componentValues: buildInitialComponentValues(
        ['Transform'],
        getSchemas(),
        {
          Transform: {
            position,
            rotation: { x: 0, y: 0, z: 0 },
            scale: { x: 1, y: 1, z: 1 },
          },
        },
      ),
    };
    created = entity;
    return {
      ...s,
      entities: [...s.entities, entity],
      nextEntityId: id + 1,
      selectedEntityId: id,
    };
  });
  logInfo(`Entity created: ${created.name} (#${created.id})`);
  return created;
}

/**
 * Create a child entity under a parent.
 * Calls the Tauri IPC so the child appears in the ECS and gets a real id.
 * Returns a promise that resolves with the new entity id.
 */
export async function createEntityChild(parentId: number, name?: string): Promise<number> {
  const childId = await apiCreateEntityChild(parentId, name);
  // The `entity-created` Tauri event (with parentId set) will update the scene state,
  // but we also need to select the new child.
  _mutate((s) => ({ ...s, selectedEntityId: childId }));
  logInfo(`Child entity created under #${parentId}: #${childId}`);
  return childId;
}

/** Delete an entity by id. Deselects if currently selected. */
export function deleteEntity(id: number): void {
  const entity = getEntityById(id);
  _mutate((s) => ({
    ...s,
    entities: s.entities.filter((e) => e.id !== id),
    selectedEntityId: s.selectedEntityId === id ? null : s.selectedEntityId,
  }));
  logInfo(`Entity deleted: ${entity?.name ?? id}`);
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
  logInfo(`Entity duplicated: ${created.name} (#${created.id})`);
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

/** Pan the camera by a screen-space delta, rotated into world-space. */
export function panCamera(dx: number, dy: number): void {
  _mutate((s) => {
    const cosA = Math.cos(s.camera.viewAngle);
    const sinA = Math.sin(s.camera.viewAngle);
    // Transform screen-space delta to world-space
    const worldDx = dx * cosA + dy * sinA;
    const worldDz = -dx * sinA + dy * cosA;
    return {
      ...s,
      camera: {
        ...s.camera,
        target: {
          x: s.camera.target.x + worldDx,
          y: s.camera.target.y,
          z: s.camera.target.z + worldDz,
        },
        position: {
          x: s.camera.position.x + worldDx,
          y: s.camera.position.y,
          z: s.camera.position.z + worldDz,
        },
      },
    };
  });
}

/** Orbit the camera around the target by delta angles (degrees).
 *  In the 2D SVG viewport this rotates the view angle. */
export function orbitCamera(dx: number, _dy: number): void {
  _mutate((s) => ({
    ...s,
    camera: {
      ...s.camera,
      viewAngle: s.camera.viewAngle + dx * Math.PI / 180,
    },
  }));
}

/** Snap the camera view angle to an exact value (radians). */
export function setViewAngle(angle: number): void {
  _mutate((s) => ({
    ...s,
    camera: { ...s.camera, viewAngle: angle },
  }));
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

/** Toggle between orthographic and perspective projection. */
export function toggleProjection(): void {
  _mutate((s) => ({
    ...s,
    camera: {
      ...s.camera,
      projection: s.camera.projection === 'ortho' ? 'perspective' : 'ortho',
    },
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
    const entities: SceneEntity[] = infos.map((info) => ({
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
      componentValues: buildInitialComponentValues(info.components, getSchemas()),
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
// Component field editing
// ---------------------------------------------------------------------------

/** Update a single component field value and sync related inline transform fields. */
export function setComponentField(
  entityId: number,
  componentName: string,
  fieldName: string,
  value: unknown,
): void {
  _mutate((s) => ({
    ...s,
    entities: s.entities.map((e) => {
      if (e.id !== entityId) return e;
      return {
        ...e,
        componentValues: {
          ...e.componentValues,
          [componentName]: {
            ...e.componentValues[componentName],
            [fieldName]: value as FieldValue,
          },
        },
      };
    }),
  }));

  // Sync inline transform fields when a Transform field changes.
  // Branch on fieldName — do NOT spread objects into function args (objects are
  // not iterable; spreading { x, y, z } produces nothing).
  if (componentName === 'Transform') {
    const v = value as { x: number; y: number; z: number };
    if (fieldName === 'position') moveEntity(entityId, v.x, v.y, v.z);
    else if (fieldName === 'rotation') rotateEntity(entityId, v.x, v.y, v.z);
    else if (fieldName === 'scale') scaleEntity(entityId, v.x, v.y, v.z);
  }

  // Forward through CQRS (undo/redo support) — fire-and-forget, optimistic update already applied
  const path = getActiveTemplatePath();
  if (path) {
    const entity = getEntityById(entityId);
    const componentData = entity?.componentValues[componentName] ?? {};
    templateExecute(path, { SetComponent: { id: entityId, type_name: componentName, data: componentData } })
      .then(() => onTemplateMutated())
      .catch(() => {}); // silent — local state already reflects the change
  }
}

// ---------------------------------------------------------------------------
// Component management
// ---------------------------------------------------------------------------

/** Add a component to an entity. No-op if already present. */
export function addComponent(entityId: number, componentName: string): void {
  const schema = getSchemas()[componentName];
  const defaults = schema ? applyComponentDefaults(schema) : {};
  _mutate((s) => {
    const entities = s.entities.map((e) => {
      if (e.id !== entityId) return e;
      if (e.components.includes(componentName)) return e;
      return {
        ...e,
        components: [...e.components, componentName],
        componentValues: { ...e.componentValues, [componentName]: defaults },
      };
    });
    return { ...s, entities };
  });
  logInfo(`Component added: ${componentName} → entity #${entityId}`);
  const _addPath = getActiveTemplatePath();
  if (_addPath) {
    templateExecute(_addPath, { AddComponent: { id: entityId, type_name: componentName, data: defaults } })
      .then(() => onTemplateMutated())
      .catch((e) => logError(`AddComponent failed: ${e}`));
  }
}

/** Remove a component from an entity. No-op if not present. */
export function removeComponent(entityId: number, componentName: string): void {
  _mutate((s) => {
    const entities = s.entities.map((e) => {
      if (e.id !== entityId) return e;
      const { [componentName]: _removed, ...rest } = e.componentValues ?? {};
      return {
        ...e,
        components: e.components.filter((c) => c !== componentName),
        componentValues: rest,
      };
    });
    return { ...s, entities };
  });
  logInfo(`Component removed: ${componentName} from entity #${entityId}`);
  const _removePath = getActiveTemplatePath();
  if (_removePath) {
    templateExecute(_removePath, { RemoveComponent: { id: entityId, type_name: componentName } })
      .then(() => onTemplateMutated())
      .catch((e) => logError(`RemoveComponent failed: ${e}`));
  }
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

    case 'set_view_angle':
      setViewAngle(args.angle as number);
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

    case 'toggle_projection':
      toggleProjection();
      return { ok: true };

    case 'new_scene':
      newScene();
      return { ok: true };

    case 'get_state':
      return getSceneState();

    case 'add_component':
      addComponent(args.id as number, args.component as string);
      return { ok: true };

    case 'remove_component':
      removeComponent(args.id as number, args.component as string);
      return { ok: true };

    case 'set_component_field':
      setComponentField(
        args.id as number,
        args.component as string,
        args.field as string,
        args.value,
      );
      return { ok: true };

    default:
      return { error: `Unknown scene command: ${command}` };
  }
}
