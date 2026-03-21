// Undo/redo state and actions for the currently open template.
//
// Call setActiveTemplate(path) when a template is opened in the editor.
// All mutations that go through CommandProcessor on the Rust side automatically
// update the undo stack — this store just tracks can-undo / can-redo locally
// so the UI can reflect the state without polling.

import { templateUndo, templateRedo, templateHistory, sceneUndo as _sceneUndo, sceneRedo as _sceneRedo } from '$lib/api';
import { logInfo, logWarn, logError } from '$lib/stores/console';

let _path: string | null = null;
let _canUndo = false;
let _canRedo = false;
let _listeners: (() => void)[] = [];

// Scene undo/redo state (independent of the template undo stack).
let _sceneCanUndo = false;
let _sceneCanRedo = false;

function notify(): void {
  _listeners.forEach((fn) => fn());
}

/** Register a callback invoked whenever canUndo / canRedo changes. */
export function subscribeUndoHistory(fn: () => void): () => void {
  _listeners.push(fn);
  return () => { _listeners = _listeners.filter((l) => l !== fn); };
}

export function getCanUndo(): boolean { return _canUndo; }
export function getCanRedo(): boolean { return _canRedo; }
export function getActiveTemplatePath(): string | null { return _path; }

export function getSceneCanUndo(): boolean { return _sceneCanUndo; }
export function getSceneCanRedo(): boolean { return _sceneCanRedo; }

// Track whether the viewport panel is currently under the mouse cursor.
// ViewportPanel sets this via setViewportFocused(); App.svelte reads it to
// route Ctrl+Z / Ctrl+Y to scene_undo / scene_redo instead of template undo.
let _viewportFocused = false;
export function setViewportFocused(focused: boolean): void { _viewportFocused = focused; }
export function getViewportFocused(): boolean { return _viewportFocused; }

/** Undo the last scene action and update local canUndo/canRedo state. */
export async function sceneUndo(): Promise<void> {
  try {
    const result = await _sceneUndo();
    _sceneCanUndo = result.canUndo;
    _sceneCanRedo = result.canRedo;
    notify();
  } catch (e) {
    logError(`Scene undo failed: ${e}`);
  }
}

/** Redo the last undone scene action and update local canUndo/canRedo state. */
export async function sceneRedo(): Promise<void> {
  try {
    const result = await _sceneRedo();
    _sceneCanUndo = result.canUndo;
    _sceneCanRedo = result.canRedo;
    notify();
  } catch (e) {
    logError(`Scene redo failed: ${e}`);
  }
}

/**
 * Set the template that undo/redo operates on.
 * Call this after template_open succeeds on the Rust side.
 */
export async function setActiveTemplate(path: string | null): Promise<void> {
  _path = path;
  if (!path) {
    _canUndo = false;
    _canRedo = false;
    notify();
    return;
  }
  await _refreshState();
}

async function _refreshState(): Promise<void> {
  if (!_path) return;
  try {
    const history = await templateHistory(_path);
    _canUndo = history.length > 0;
    notify();
  } catch {
    // Template not open on Rust side yet — not an error.
  }
}

/** Undo the last action on the active template. */
export async function undo(): Promise<void> {
  if (!_path) {
    logWarn('Undo: no template is open');
    return;
  }
  try {
    const actionId = await templateUndo(_path);
    if (actionId === null) {
      logInfo('Nothing to undo');
    } else {
      logInfo(`Undo (action ${actionId})`);
      _canRedo = true;
    }
    await _refreshState();
  } catch (e) {
    logError(`Undo failed: ${e}`);
  }
}

/** Redo the last undone action on the active template. */
export async function redo(): Promise<void> {
  if (!_path) {
    logWarn('Redo: no template is open');
    return;
  }
  try {
    const actionId = await templateRedo(_path);
    if (actionId === null) {
      logInfo('Nothing to redo');
      _canRedo = false;
    } else {
      logInfo(`Redo (action ${actionId})`);
    }
    await _refreshState();
  } catch (e) {
    logError(`Redo failed: ${e}`);
  }
}

/**
 * Call this after every template mutation so the UI reflects updated state.
 * CommandProcessor clears the redo stack on execute, so canRedo becomes false.
 */
export async function onTemplateMutated(): Promise<void> {
  _canRedo = false;
  await _refreshState();
}
