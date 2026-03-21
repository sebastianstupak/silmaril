import { writable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';

export const aiServerRunning = writable(false);
export const aiServerPort = writable<number | null>(null);

export async function startAiServer(projectPath: string, port = 7878): Promise<void> {
  try {
    const boundPort = await invoke<number>('ai_server_start', { port, project_path: projectPath });
    aiServerRunning.set(true);
    aiServerPort.set(boundPort);
  } catch (e) {
    console.error('Failed to start AI server:', e);
  }
}

export async function stopAiServer(): Promise<void> {
  try {
    await invoke('ai_server_stop');
  } catch {
    // ignore
  }
  aiServerRunning.set(false);
  aiServerPort.set(null);
}

export async function refreshAiServerStatus(): Promise<void> {
  try {
    const status = await invoke<{ running: boolean; port: number | null }>('ai_server_status');
    aiServerRunning.set(status.running);
    aiServerPort.set(status.port ?? null);
  } catch {
    // ignore
  }
}
