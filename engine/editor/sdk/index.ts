// @silmaril/editor-sdk — API types, theme, component re-exports
export * from '../src/lib/theme/tokens';
export type { EntityId, ComponentData, SubscriptionConfig } from '../src/lib/api';

// Plugin API — available inside Tier 3 iframe panels
export interface SilmarilPluginAPI {
  getComponent(entityId: number, componentName: string): Promise<unknown>;
  setComponent(entityId: number, componentName: string, data: unknown): Promise<void>;
  subscribe(channel: string, callback: (data: unknown) => void): () => void;
  undo(): Promise<void>;
  redo(): Promise<void>;
}

// Declared on window by the editor shell
declare global {
  interface Window {
    silmaril?: SilmarilPluginAPI;
  }
}
