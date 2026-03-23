// Shared editor context accessible by docked panels without prop drilling.
//
// This module now delegates to the centralized scene state so that entity
// data has a single source of truth.  Legacy callers that used setEntities /
// setSelectedEntityId still work — they write-through to scene state.

import type { EntityInfo } from '$lib/api';
import {
  getTemplateState,
  getSelectedEntity as _getSelectedEntity,
  subscribeTemplate,
  type TemplateEntity,
} from '$lib/template/state';
import { selectEntity, populateFromScan } from '$lib/template/commands';

interface EditorContext {
  entities: EntityInfo[];
  selectedEntityId: number | null;
}

/** Read the editor context (derived from template state). */
export function getEditorContext(): EditorContext {
  const s = getTemplateState();
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

/** Get the id of the currently selected entity (or null). */
export function getSelectedEntityId(): number | null {
  return getTemplateState().selectedEntityId;
}

/** Get the currently selected entity from template state. */
export function getSelectedEntity(): TemplateEntity | null {
  return _getSelectedEntity();
}

/** Subscribe to changes — delegates to template state pub/sub. */
export function subscribeContext(fn: () => void): () => void {
  return subscribeTemplate(fn);
}
