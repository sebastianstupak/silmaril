// engine/editor/src/lib/contributions/registry.ts
import type { Component } from 'svelte';
import { getBasePanelId } from '../docking/types';

export interface PanelContribution {
  id: string;
  title: string;
  icon?: string;
  component: Component;
  source: string; // 'builtin' | cargo crate name | module id
}

export interface InspectorFieldContribution {
  componentType: string;
  renderer: Component;
  source: string;
}

// ── State ──────────────────────────────────────────────────────────────────
let _panels: PanelContribution[] = [];
let _inspectorFields: InspectorFieldContribution[] = [];
let _panelListeners: (() => void)[] = [];

function notifyPanels() {
  _panelListeners.forEach((fn) => fn());
}

// ── Panel registration ─────────────────────────────────────────────────────
export function registerPanel(c: PanelContribution): void {
  // Replace if id already registered (idempotent re-registration)
  const idx = _panels.findIndex((p) => p.id === c.id);
  if (idx !== -1) {
    _panels[idx] = c;
  } else {
    _panels = [..._panels, c];
  }
  notifyPanels();
}

export function unregisterPanel(id: string): void {
  _panels = _panels.filter((p) => p.id !== id);
  notifyPanels();
}

export function getPanelContributions(): PanelContribution[] {
  return _panels;
}

/** Subscribe to panel list changes. Returns unsubscribe function. */
export function subscribePanelContributions(fn: () => void): () => void {
  _panelListeners.push(fn);
  return () => {
    _panelListeners = _panelListeners.filter((l) => l !== fn);
  };
}

/** Look up a component by panel ID (supports instance IDs like 'viewport:2'). */
export function getPanelComponent(id: string): Component | undefined {
  const base = getBasePanelId(id);
  return (_panels.find((p) => p.id === id) ?? _panels.find((p) => p.id === base))?.component;
}

/** Look up a panel's display title by ID (supports instance IDs). */
export function getPanelTitle(id: string): string {
  const base = getBasePanelId(id);
  return (_panels.find((p) => p.id === id) ?? _panels.find((p) => p.id === base))?.title ?? id;
}

// ── Inspector field registration (API scaffold — wiring deferred) ───────────
export function registerInspectorField(c: InspectorFieldContribution): void {
  const idx = _inspectorFields.findIndex(
    (f) => f.componentType === c.componentType && f.source === c.source,
  );
  if (idx !== -1) {
    _inspectorFields[idx] = c;
  } else {
    _inspectorFields = [..._inspectorFields, c];
  }
}

export function unregisterInspectorField(componentType: string, source: string): void {
  _inspectorFields = _inspectorFields.filter(
    (f) => !(f.componentType === componentType && f.source === source),
  );
}

export function getInspectorFieldContributions(): InspectorFieldContribution[] {
  return _inspectorFields;
}
