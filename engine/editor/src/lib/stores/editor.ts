// Editor state store — selected entity, mode, project info
// TODO: Implement with Svelte stores when Tauri is integrated

export type EditorMode = 'edit' | 'play' | 'pause';

export interface EditorState {
  mode: EditorMode;
  selectedEntityId: number | null;
  projectName: string | null;
  projectPath: string | null;
}
