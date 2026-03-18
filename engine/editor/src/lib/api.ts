import { invoke } from '@tauri-apps/api/core';

export interface EditorState {
  mode: string;
  project_name: string | null;
  project_path: string | null;
}

export type EntityId = number;

export interface ComponentData {
  type_name: string;
  data: unknown;
}

export async function getEditorState(): Promise<EditorState> {
  return await invoke('get_editor_state');
}

export async function openProject(path: string): Promise<EditorState> {
  return await invoke('open_project', { path });
}
