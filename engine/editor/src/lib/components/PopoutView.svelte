<script lang="ts">
  import { onMount } from 'svelte';
  import { t } from '$lib/i18n';
  import { loadSettings } from '$lib/stores/settings';
  import { themes, applyTheme } from '$lib/theme/tokens';
  import HierarchyWrapper from '$lib/docking/panels/HierarchyWrapper.svelte';
  import InspectorWrapper from '$lib/docking/panels/InspectorWrapper.svelte';
  import ConsoleWrapper from '$lib/docking/panels/ConsoleWrapper.svelte';
  import ViewportPanel from '$lib/docking/panels/ViewportPanel.svelte';
  import ProfilerPanel from '$lib/docking/panels/ProfilerPanel.svelte';
  import AssetsPanel from '$lib/docking/panels/AssetsPanel.svelte';
  import type { Component } from 'svelte';

  let { panelId }: { panelId: string } = $props();

  /** Detect if running inside Tauri or standalone browser */
  const isTauri = typeof window !== 'undefined' && !!(window as any).__TAURI_INTERNALS__;

  const panels: Record<string, Component> = {
    hierarchy: HierarchyWrapper,
    inspector: InspectorWrapper,
    console: ConsoleWrapper,
    viewport: ViewportPanel,
    profiler: ProfilerPanel,
    assets: AssetsPanel,
  };

  const basePanelId = panelId.split(':')[0];
  const PanelComponent = panels[basePanelId];

  onMount(() => {
    // Apply theme so the pop-out window matches the main editor
    const settings = loadSettings();
    applyTheme(themes[settings.theme] ?? themes.dark);
    document.documentElement.style.fontSize = `${settings.fontSize}px`;
  });

  async function dockBack() {
    if (!isTauri) return;
    try {
      const { emit } = await import('@tauri-apps/api/event');
      await emit('dock-panel-back', { panelId });
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      getCurrentWindow().close();
    } catch (e) {
      console.error('[silmaril] dockBack error:', e);
    }
  }
</script>

<div class="popout-container">
  <div class="popout-toolbar">
    <button class="dock-back-btn" onclick={dockBack} title={t('popout.dock_back')}>
      {t('popout.dock_back')}
    </button>
  </div>
  {#if PanelComponent}
    <div class="popout-content">
      <PanelComponent />
    </div>
  {:else}
    <p class="popout-unknown">{t('popout.unknown')}: {panelId}</p>
  {/if}
</div>

<style>
  .popout-container {
    width: 100vw;
    height: 100vh;
    background: var(--color-bg, #1e1e1e);
    color: var(--color-text, #ccc);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .popout-toolbar {
    display: flex;
    align-items: center;
    padding: 4px 8px;
    background: var(--color-bgHeader, #2d2d2d);
    border-bottom: 1px solid var(--color-border, #404040);
    flex-shrink: 0;
  }

  .dock-back-btn {
    background: none;
    border: 1px solid var(--color-border, #404040);
    border-radius: 4px;
    color: var(--color-text, #ccc);
    font-size: 11px;
    padding: 3px 10px;
    cursor: pointer;
    transition: background 0.1s, color 0.1s;
  }

  .dock-back-btn:hover {
    background: var(--color-accent, #007acc);
    color: #fff;
    border-color: var(--color-accent, #007acc);
  }

  .popout-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .popout-unknown {
    display: flex;
    align-items: center;
    justify-content: center;
    flex: 1;
    color: var(--color-textDim, #666);
    font-style: italic;
  }
</style>
