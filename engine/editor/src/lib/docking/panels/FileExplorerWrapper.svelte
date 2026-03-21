<!-- engine/editor/src/lib/docking/panels/FileExplorerWrapper.svelte -->
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { invoke } from '@tauri-apps/api/core';
  import FileExplorerPanel from './FileExplorerPanel.svelte';
  import {
    getFileExplorerState,
    subscribeFileExplorer,
    loadTree,
    refreshTree,
    type FileExplorerState,
  } from '$lib/stores/file-explorer';
  import { logWarn } from '$lib/stores/console';

  const isTauri = typeof window !== 'undefined' && !!(window as any).__TAURI_INTERNALS__;

  let state: FileExplorerState = $state(getFileExplorerState());
  let unsubscribe: (() => void) | null = null;
  let unlisten: (() => void) | null = null;

  onMount(async () => {
    // Subscribe to store changes
    unsubscribe = subscribeFileExplorer(() => {
      state = getFileExplorerState();
    });

    if (!isTauri) return;

    // If a project is already loaded, start the watcher and load the tree
    try {
      const editorState = await invoke<{ project_path?: string }>('get_editor_state');
      if (editorState.project_path) {
        await invoke('start_file_watch', { root: editorState.project_path });
        await loadTree(editorState.project_path);
      }
    } catch (e) {
      logWarn(`FileExplorerWrapper: could not load initial project state — ${e}`);
    }

    // Listen for file system changes from Rust watcher
    unlisten = await listen<{ root: string }>('file-tree-changed', async () => {
      await refreshTree();
    });
  });

  onDestroy(async () => {
    unsubscribe?.();
    unlisten?.();
    if (isTauri) {
      try { await invoke('stop_file_watch'); } catch { /* ignore */ }
    }
  });
</script>

<div class="panel-opaque">
  <FileExplorerPanel {state} />
</div>

<style>
  .panel-opaque {
    width: 100%;
    height: 100%;
    background: var(--color-bgPanel, #1e1e1e);
  }
</style>
