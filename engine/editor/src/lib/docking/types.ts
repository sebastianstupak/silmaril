// Docking system types for Silmaril Editor
// Layout is modeled as a tree of splits and tab groups

/** A split node divides space between children */
export interface SplitNode {
  type: 'split';
  direction: 'horizontal' | 'vertical';
  children: LayoutNode[];
  /** Percentage sizes for each child, must sum to 100 */
  sizes: number[];
}

/** A tab group node holds one or more panels as tabs */
export interface TabsNode {
  type: 'tabs';
  activeTab: number;
  panels: string[];
}

export type LayoutNode = SplitNode | TabsNode;

/** Top-level editor layout with a main area and a bottom panel */
export interface EditorLayout {
  root: LayoutNode;
  bottomPanel: LayoutNode;
}

/** Drop zone position relative to a target panel */
export type DropZone = 'left' | 'right' | 'top' | 'bottom' | 'center';

/** Information about a panel being dragged */
export interface DragPayload {
  panelId: string;
  sourceNodePath: number[];
}

/** Panel metadata for the registry */
export interface PanelInfo {
  id: string;
  titleKey: string;
  icon?: string;
}

/** All known panels */
export const panelRegistry: PanelInfo[] = [
  { id: 'hierarchy', titleKey: 'panel.hierarchy' },
  { id: 'viewport', titleKey: 'panel.viewport' },
  { id: 'inspector', titleKey: 'panel.inspector' },
  { id: 'console', titleKey: 'panel.console' },
  { id: 'profiler', titleKey: 'panel.profiler' },
  { id: 'assets', titleKey: 'panel.assets' },
  { id: 'file-explorer', titleKey: 'panel.file_explorer' },
  { id: 'terminal', titleKey: 'panel.terminal' },
  { id: 'output', titleKey: 'panel.output' },
];

/** Get the base panel ID (strip instance suffix like 'viewport:2' → 'viewport') */
export function getBasePanelId(id: string): string {
  const colonIdx = id.indexOf(':');
  return colonIdx === -1 ? id : id.substring(0, colonIdx);
}

/** Get panel info by ID (supports instance IDs like 'viewport:2') */
export function getPanelInfo(id: string): PanelInfo | undefined {
  return panelRegistry.find(p => p.id === getBasePanelId(id));
}

let _instanceCounter = 0;

/** Create a new panel instance ID like 'viewport:1', 'viewport:2' */
export function createPanelInstance(baseId: string): string {
  _instanceCounter++;
  return `${baseId}:${_instanceCounter}`;
}
