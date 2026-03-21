<!-- engine/editor/src/lib/omnibar/Omnibar.svelte -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import type { EditorCommand, OmnibarResult, AnyCommand } from './types';
  import { isFrontendCommand } from './types';
  import { listFrontendCommands } from './registry';
  import { buildResults } from './providers';
  import { getEditorContext, setEntities, setSelectedEntityId } from '$lib/stores/editor-context';
  import { selectEntity } from '$lib/scene/commands';
  import { openProject, scanProjectEntities } from '$lib/api';
  import type { RecentItem } from '$lib/stores/recent-items';

  interface Props {
    projectPath?: string | null;
    open?: boolean;
    onOpen?: () => void;
    onClose?: () => void;
    recentItems?: RecentItem[];
  }

  let { projectPath = null, open = $bindable(false), onOpen, onClose, recentItems = [] }: Props = $props();

  let query = $state('');
  let results: OmnibarResult[] = $state([]);
  let selectedIndex = $state(0);
  let inputEl: HTMLInputElement | undefined = $state(undefined);

  // Cached Rust commands (fetched once on mount)
  let rustCommands: EditorCommand[] = $state([]);
  // Cached assets (fetched lazily on first '#' use)
  let assets: { path: string; assetType: string }[] = $state([]);
  let assetsFetched = false;

  onMount(async () => {
    try {
      rustCommands = await invoke<EditorCommand[]>('list_commands');
    } catch {
      rustCommands = [];
    }
  });

  // Recompute results whenever query or open state changes
  $effect(() => {
    if (!open) { results = []; selectedIndex = 0; return; }

    const tsCommands = listFrontendCommands();
    // Merge: TS-first (TS shadows Rust on same id)
    const tsIds = new Set(tsCommands.map(c => c.id));
    const merged: AnyCommand[] = [
      ...tsCommands,
      ...rustCommands.filter(c => !tsIds.has(c.id)),
    ];

    const ctx = getEditorContext();
    const entities = ctx.entities.map(e => ({
      id: e.id as number,
      name: e.name,
      components: e.components,
    }));

    // Fetch assets lazily on first '#' use
    if (query.startsWith('#') && !assetsFetched && projectPath) {
      assetsFetched = true;
      invoke<{ path: string; asset_type: string }[]>('scan_assets', { projectPath })
        .then(list => { assets = list.map(a => ({ path: a.path, assetType: a.asset_type })); })
        .catch(() => {});
    }

    results = buildResults(query, merged, entities, assets, recentItems);
    selectedIndex = 0;
  });

  // Focus input when opened
  $effect(() => {
    if (open && inputEl) {
      inputEl.focus();
    }
  });

  async function execute(result: OmnibarResult) {
    close();
    switch (result.kind) {
      case 'command': {
        const cmd = result.command;
        if (isFrontendCommand(cmd)) {
          await cmd.run();
        } else {
          try {
            await invoke('run_command', { id: cmd.id });
          } catch (e) {
            console.error('[omnibar] run_command failed:', e);
          }
        }
        break;
      }
      case 'entity':
        selectEntity(result.id);
        break;
      case 'asset':
        // Future: focus asset in assets panel
        break;
      case 'recent': {
        const state = await openProject(result.path);
        const scannedEntities = await scanProjectEntities(result.path);
        setEntities(scannedEntities);
        setSelectedEntityId(null);
        break;
      }
    }
  }

  function close() {
    open = false;
    query = '';
    onClose?.();
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') { e.preventDefault(); close(); return; }
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      selectedIndex = Math.min(selectedIndex + 1, results.length - 1);
      return;
    }
    if (e.key === 'ArrowUp') {
      e.preventDefault();
      selectedIndex = Math.max(selectedIndex - 1, 0);
      return;
    }
    if (e.key === 'Enter' && results[selectedIndex]) {
      e.preventDefault();
      execute(results[selectedIndex]);
      return;
    }
  }

  function resultLabel(r: OmnibarResult): string {
    if (r.kind === 'command') return r.command.label;
    if (r.kind === 'entity') return r.name;
    if (r.kind === 'asset') return r.path.split(/[\\/]/).pop() ?? r.path;
    return r.label;
  }

  function resultMeta(r: OmnibarResult): string {
    if (r.kind === 'command') return r.command.category;
    if (r.kind === 'entity') return r.components.join(', ');
    if (r.kind === 'asset') return r.assetType;
    return r.itemType;
  }

  function resultKeybind(r: OmnibarResult): string | undefined {
    if (r.kind === 'command') return r.command.keybind;
    return undefined;
  }
</script>

<div class="omnibar-wrapper" role="none" onkeydown={onKeydown}>
  <!-- Idle pill (always visible) -->
  {#if !open}
    <button
      class="omnibar-idle"
      onclick={() => { open = true; onOpen?.(); }}
      aria-label="Open omnibar (Ctrl+K)"
    >
      <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
        <circle cx="6" cy="6" r="4.5" stroke="currentColor" stroke-width="1.5" fill="none"/>
        <line x1="9.5" y1="9.5" x2="13" y2="13" stroke="currentColor" stroke-width="1.5"/>
      </svg>
      <span class="omnibar-placeholder">Search…</span>
      <kbd class="omnibar-hint">Ctrl K</kbd>
    </button>
  {:else}
    <!-- Active state -->
    <div class="omnibar-active">
      <div class="omnibar-input-row">
        <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor" class="omnibar-icon" aria-hidden="true">
          <circle cx="6" cy="6" r="4.5" stroke="currentColor" stroke-width="1.5" fill="none"/>
          <line x1="9.5" y1="9.5" x2="13" y2="13" stroke="currentColor" stroke-width="1.5"/>
        </svg>
        <input
          bind:this={inputEl}
          bind:value={query}
          type="text"
          class="omnibar-input"
          placeholder="Search commands, entities, assets…"
          autocomplete="off"
          spellcheck="false"
        />
        <kbd class="omnibar-hint">Esc</kbd>
      </div>

      {#if results.length > 0}
        <ul class="omnibar-results" role="listbox">
          {#each results as result, i}
            <li
              class="omnibar-result"
              class:selected={i === selectedIndex}
              role="option"
              aria-selected={i === selectedIndex}
              onmouseenter={() => selectedIndex = i}
              onclick={() => execute(result)}
            >
              <span class="result-label">{resultLabel(result)}</span>
              <span class="result-meta">{resultMeta(result)}</span>
              {#if resultKeybind(result)}
                <kbd class="result-keybind">{resultKeybind(result)}</kbd>
              {/if}
            </li>
          {/each}
        </ul>
      {/if}
    </div>

    <!-- Backdrop to dismiss on outside click -->
    <div class="omnibar-backdrop" role="none" onclick={close}></div>
  {/if}
</div>

<style>
  .omnibar-wrapper {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .omnibar-idle {
    display: flex;
    align-items: center;
    gap: 6px;
    background: var(--color-bg, #1e1e2e);
    border: 1px solid var(--color-border, #404040);
    border-radius: 6px;
    height: 24px;
    padding: 0 8px;
    cursor: pointer;
    color: var(--color-textMuted, #6c7086);
    min-width: 200px;
    max-width: 320px;
    width: 100%;
  }

  .omnibar-idle:hover {
    border-color: var(--color-accent, #89b4fa);
    color: var(--color-text, #cdd6f4);
  }

  .omnibar-placeholder {
    flex: 1;
    font-size: 11px;
    text-align: left;
  }

  .omnibar-hint {
    font-size: 10px;
    color: var(--color-textDim, #45475a);
    border: 1px solid var(--color-border, #313244);
    padding: 1px 4px;
    border-radius: 3px;
    font-family: inherit;
    white-space: nowrap;
  }

  .omnibar-active {
    position: absolute;
    top: 0;
    left: 50%;
    transform: translateX(-50%);
    width: 360px;
    z-index: 10000;
    background: var(--color-bgPanel, #1e1e2e);
    border: 1px solid var(--color-accent, #89b4fa);
    border-radius: 6px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.6);
  }

  .omnibar-input-row {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 8px;
    height: 32px;
    border-bottom: 1px solid var(--color-border, #313244);
  }

  .omnibar-icon {
    color: var(--color-accent, #89b4fa);
    flex-shrink: 0;
  }

  .omnibar-input {
    flex: 1;
    background: none;
    border: none;
    outline: none;
    color: var(--color-text, #cdd6f4);
    font-size: 12px;
    font-family: inherit;
  }

  .omnibar-results {
    list-style: none;
    margin: 0;
    padding: 4px 0;
    max-height: 280px;
    overflow-y: auto;
  }

  .omnibar-result {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 10px;
    cursor: pointer;
    font-size: 12px;
    color: var(--color-textMuted, #a6adc8);
  }

  .omnibar-result.selected,
  .omnibar-result:hover {
    background: var(--color-bgHover, #313244);
    color: var(--color-text, #cdd6f4);
  }

  .result-label {
    flex: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .result-meta {
    font-size: 10px;
    color: var(--color-textDim, #45475a);
    background: var(--color-bg, #181825);
    padding: 1px 5px;
    border-radius: 3px;
    white-space: nowrap;
  }

  .result-keybind {
    font-size: 10px;
    color: var(--color-textDim, #45475a);
    border: 1px solid var(--color-border, #313244);
    padding: 1px 4px;
    border-radius: 3px;
    font-family: inherit;
    white-space: nowrap;
  }

  .omnibar-backdrop {
    position: fixed;
    inset: 0;
    z-index: 9999;
  }
</style>
