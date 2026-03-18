// Shared editor context accessible by docked panels without prop drilling.
//
// This module now delegates to the centralized scene state so that entity
// data has a single source of truth.  Legacy callers that used setEntities /
// setSelectedEntityId still work — they write-through to scene state.

import type { EntityInfo } from '$lib/api';
import {
  getSceneState,
  getSelectedEntity as _getSelectedEntity,
  subscribeScene,
  type SceneEntity,
} from '$lib/scene/state';
import { selectEntity, populateFromScan } from '$lib/scene/commands';

interface EditorContext {
  entities: EntityInfo[];
  selectedEntityId: number | null;
}

/** Read the editor context (derived from scene state). */
export function getEditorContext(): EditorContext {
  const s = getSceneState();
  return {
    entities: s.entities,
    selectedEntityId: s.selectedEntityId,
  };
}

/** Populate entities — delegates to scene commands. */
export function setEntities(entities: EntityInfo[]) {
  populateFromScan(entities);
}

/** Select an entity — delegates to scene commands. */
export function setSelectedEntityId(id: number | null) {
  selectEntity(id);
}

/** Get the currently selected entity from scene state. */
export function getSelectedEntity(): SceneEntity | null {
  return _getSelectedEntity();
}

/** Subscribe to changes — delegates to scene state pub/sub. */
export function subscribeContext(fn: () => void): () => void {
  return subscribeScene(fn);
}
