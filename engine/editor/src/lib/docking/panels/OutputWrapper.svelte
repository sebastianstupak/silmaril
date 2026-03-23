<!-- engine/editor/src/lib/docking/panels/OutputWrapper.svelte -->
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { invoke } from '@tauri-apps/api/core';
  import { t } from '$lib/i18n';
  import OutputPanel from './OutputPanel.svelte';
  import {
    getOutputState,
    subscribeOutput,
    appendLine,
    setRunning,
    setFinished,
    clearOutput,
    type OutputState,
  } from '$lib/stores/output';
  import { logError } from '$lib/stores/console';

  const isTauri = typeof window !== 'undefined' && !!(window as any).__TAURI_INTERNALS__;

  let state: OutputState = $state(getOutputState());
  let hasProject = $state(false);
  let unsubscribe: (() => void) | null = null;
  let unlistenData: (() => void) | null = null;
  let unlistenExit: (() => void) | null = null;
  let pollInterval: ReturnType<typeof setInterval> | null = null;

  async function handleRun(command: string, args: string[]): Promise<void> {
    const label = `${command} ${args.join(' ')}`;
    setRunning(label);
    try {
      await invoke('output_run', { command, args });
    } catch (e) {
      logError(`Output: failed to start command — ${e}`);
      setFinished(null, false);
    }
  }

  async function handleCancel(): Promise<void> {
    try {
      await invoke('output_cancel');
    } catch (e) {
      logError(`Output: failed to cancel — ${e}`);
    }
  }

  onMount(async () => {
    unsubscribe = subscribeOutput(() => {
      state = getOutputState();
    });

    if (!isTauri) return;

    // Check project on mount and keep polling so the panel reacts
    // when a project is opened after the panel was already mounted.
    async function refreshHasProject(): Promise<void> {
      try {
        const editorState = await invoke<{ project_path?: string }>('get_editor_state');
        hasProject = !!editorState.project_path;
      } catch { /* leave hasProject unchanged */ }
    }

    await refreshHasProject();

    // Re-check every 2 seconds so opening a project shows the panel without remounting.
    pollInterval = setInterval(refreshHasProject, 2000);

    unlistenData = await listen<{ line: string; stream: 'stdout' | 'stderr' }>(
      'output-data',
      e => appendLine(e.payload.line, e.payload.stream)
    );

    unlistenExit = await listen<{ code: number | null; cancelled: boolean }>(
      'output-exit',
      e => setFinished(e.payload.code, e.payload.cancelled)
    );
  });

  onDestroy(() => {
    unsubscribe?.();
    unlistenData?.();
    unlistenExit?.();
    if (pollInterval !== null) clearInterval(pollInterval);
    // Note: does NOT cancel running process — build continues in background.
  });
</script>

<div class="panel-opaque">
  {#if !hasProject}
    <div class="placeholder">{t('placeholder.no_project')}</div>
  {:else}
    <OutputPanel
      {state}
      onRun={handleRun}
      onCancel={handleCancel}
      onClear={clearOutput}
    />
  {/if}
</div>

<style>
  .panel-opaque {
    width: 100%;
    height: 100%;
    background: var(--color-bgPanel, #1e1e1e);
    display: flex;
    flex-direction: column;
  }
  .placeholder {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--color-textMuted, #666);
    font-size: 13px;
  }
</style>
