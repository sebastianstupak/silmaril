<!-- engine/editor/src/lib/docking/panels/TerminalPanel.svelte -->
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { t } from '$lib/i18n';
  import type { TerminalState } from '$lib/stores/terminal';
  import { drainTerminalData, setActiveTab } from '$lib/stores/terminal';
  import TerminalTabs from './TerminalTabs.svelte';

  let { state: termState, onNewTab, onCloseTab }: {
    state: TerminalState;
    onNewTab: () => void;
    onCloseTab: (id: string) => void;
  } = $props();

  // Map of tabId → { terminal, fitAddon, containerEl }
  let xtermInstances = new Map<string, { term: any; fit: any; el: HTMLDivElement }>();
  let containerRef = $state<HTMLDivElement | null>(null);
  let resizeObserver: ResizeObserver | null = null;
  let loadError = $state<string | null>(null);

  // xterm.js is imported dynamically to avoid SSR issues
  let XTermModule: { Terminal: any; FitAddon: any } | null = null;

  const XTERM_THEME = {
    background: '#1e1e1e',
    foreground: '#d4d4d4',
    cursor: '#d4d4d4',
    black: '#1e1e1e', red: '#cc3e28', green: '#57a64a', yellow: '#d7ba7d',
    blue: '#569cd6', magenta: '#c586c0', cyan: '#9cdcfe', white: '#d4d4d4',
    brightBlack: '#666666', brightRed: '#f44747', brightGreen: '#b5cea8',
    brightYellow: '#dcdcaa', brightBlue: '#4ec9b0', brightMagenta: '#d670d6',
    brightCyan: '#87d5f5', brightWhite: '#ffffff',
  };

  onMount(async () => {
    try {
      const [xtermPkg, fitPkg] = await Promise.all([
        import('@xterm/xterm'),
        import('@xterm/addon-fit'),
      ]);
      XTermModule = { Terminal: xtermPkg.Terminal, FitAddon: fitPkg.FitAddon };
    } catch (e) {
      loadError = `Failed to load xterm.js: ${e}`;
      return;
    }

    // Create terminals for any tabs that already exist
    for (const tab of termState.tabs) {
      if (!xtermInstances.has(tab.id)) {
        createTerminal(tab.id);
      }
    }

    resizeObserver = new ResizeObserver(() => fitAll());
    if (containerRef) resizeObserver.observe(containerRef);
  });

  onDestroy(() => {
    resizeObserver?.disconnect();
    for (const [, inst] of xtermInstances) {
      inst.term.dispose();
    }
    xtermInstances.clear();
  });

  function createTerminal(tabId: string): void {
    if (!XTermModule || !containerRef) return;

    const el = document.createElement('div');
    el.style.cssText = 'position:absolute;inset:0;display:none;';
    containerRef.appendChild(el);

    const term = new XTermModule.Terminal({
      theme: XTERM_THEME,
      fontFamily: 'Consolas, "Courier New", monospace',
      fontSize: 13,
      cursorBlink: true,
    });
    const fit = new XTermModule.FitAddon();
    term.loadAddon(fit);
    term.open(el);
    fit.fit();

    term.onData((data: string) => {
      invoke('terminal_write', { tabId, data }).catch(() => {});
    });

    xtermInstances.set(tabId, { term, fit, el });
    updateVisibility();
  }

  function updateVisibility(): void {
    for (const [id, inst] of xtermInstances) {
      inst.el.style.display = id === termState.activeTabId ? 'block' : 'none';
    }
  }

  function fitAll(): void {
    for (const [tabId, inst] of xtermInstances) {
      try {
        inst.fit.fit();
        invoke('terminal_resize', {
          tabId,
          cols: inst.term.cols,
          rows: inst.term.rows,
        }).catch(() => {});
      } catch { /* ignore fit errors */ }
    }
  }

  // Reactive: when termState.tabs changes, create terminals for new tabs
  $effect(() => {
    if (!XTermModule) return;
    for (const tab of termState.tabs) {
      if (!xtermInstances.has(tab.id)) {
        createTerminal(tab.id);
      }
    }
    updateVisibility();
  });

  // Reactive: drain pending data and write to appropriate terminal.
  // IMPORTANT: drainTerminalData() must NOT call notify() — this effect
  // mutates the store's pendingData Map directly without triggering reactivity,
  // which avoids an infinite effect loop. See terminal.ts drainTerminalData().
  $effect(() => {
    for (const tab of termState.tabs) {
      const data = drainTerminalData(tab.id);
      if (data) {
        xtermInstances.get(tab.id)?.term.write(data);
      }
    }
  });

  // Reactive: update visibility when active tab changes
  $effect(() => {
    void termState.activeTabId; // track dependency
    updateVisibility();
  });
</script>

<div class="terminal-panel">
  {#if termState.tabs.length === 0}
    <div class="placeholder">{t('terminal.opening')}</div>
  {:else}
    <TerminalTabs
      tabs={termState.tabs}
      activeTabId={termState.activeTabId}
      {onNewTab}
      onCloseTab={onCloseTab}
      onSelectTab={id => setActiveTab(id)}
    />
    {#if loadError}
      <div class="load-error">{loadError}</div>
    {:else}
      <div class="xterm-container" bind:this={containerRef}></div>
    {/if}
  {/if}
</div>

<style>
  .terminal-panel {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    background: #1e1e1e;
  }
  .xterm-container {
    flex: 1;
    position: relative;
    overflow: hidden;
  }
  .placeholder, .load-error {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--color-textMuted, #666);
    font-size: 13px;
  }
  .load-error { color: var(--color-error, #f44747); }
</style>
