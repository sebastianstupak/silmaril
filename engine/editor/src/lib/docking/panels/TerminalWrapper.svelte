<!-- engine/editor/src/lib/docking/panels/TerminalWrapper.svelte -->
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { invoke } from '@tauri-apps/api/core';
  import TerminalPanel from './TerminalPanel.svelte';
  import {
    getTerminalState,
    subscribeTerminal,
    addTab,
    closeTab,
    markExited,
    appendTerminalData,
    type TerminalState,
  } from '$lib/stores/terminal';
  import { logError } from '$lib/stores/console';

  const isTauri = typeof window !== 'undefined' && !!(window as any).__TAURI_INTERNALS__;

  let state: TerminalState = $state(getTerminalState());
  let unsubscribe: (() => void) | null = null;
  // Per-tab unlisten functions
  const unlisteners = new Map<string, Array<() => void>>();

  async function setupTabListeners(tabId: string): Promise<void> {
    const unlistenData = await listen<string>(`terminal-data:${tabId}`, e => {
      appendTerminalData(tabId, e.payload);
    });
    const unlistenExit = await listen(`terminal-exit:${tabId}`, () => {
      markExited(tabId);
      cleanupTabListeners(tabId);
    });
    unlisteners.set(tabId, [unlistenData, unlistenExit]);
  }

  function cleanupTabListeners(tabId: string): void {
    unlisteners.get(tabId)?.forEach(fn => fn());
    unlisteners.delete(tabId);
  }

  async function openNewTab(): Promise<void> {
    try {
      const tabId = await invoke<string>('terminal_new_tab');
      addTab(tabId);
      await setupTabListeners(tabId);
    } catch (e) {
      logError(`Terminal: failed to open new tab — ${e}`);
    }
  }

  async function handleCloseTab(id: string): Promise<void> {
    cleanupTabListeners(id);
    try { await invoke('terminal_close_tab', { tabId: id }); } catch { /* ignore */ }
    closeTab(id);
  }

  onMount(async () => {
    unsubscribe = subscribeTerminal(() => {
      state = getTerminalState();
    });

    if (!isTauri) return;

    // Open first tab immediately (terminal works without a project)
    await openNewTab();
  });

  onDestroy(async () => {
    unsubscribe?.();
    // Clean up all listeners and close all open tabs
    const tabIds = [...unlisteners.keys()];
    for (const tabId of tabIds) {
      cleanupTabListeners(tabId);
      if (isTauri) {
        try { await invoke('terminal_close_tab', { tabId }); } catch { /* ignore */ }
      }
    }
  });
</script>

<div class="panel-opaque">
  <TerminalPanel
    {state}
    onNewTab={openNewTab}
    onCloseTab={handleCloseTab}
  />
</div>

<style>
  .panel-opaque {
    width: 100%;
    height: 100%;
    background: var(--color-bgPanel, #1e1e1e);
  }
</style>
