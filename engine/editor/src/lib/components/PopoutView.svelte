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

  let nearMainWindow = $state(false);

  onMount(() => {
    // Apply theme so the pop-out window matches the main editor
    const settings = loadSettings();
    applyTheme(themes[settings.theme] ?? themes.dark);
    document.documentElement.style.fontSize = `${settings.fontSize}px`;

    // Dock detection is handled by the drag handle (grab dots icon in toolbar)
  });

  // Drag handle state — user drags the handle to dock the panel back
  let dragHandleActive = $state(false);

  /** Start dragging the dock handle — tracks mouse globally */
  function startDragHandle(e: MouseEvent) {
    if (e.button !== 0) return;
    e.preventDefault();
    dragHandleActive = true;

    // Track mouse position to detect when it enters the main window
    function onMouseMove(ev: MouseEvent) {
      // screenX/screenY gives us the global position
      checkMainWindowProximity(ev.screenX, ev.screenY);
    }

    async function onMouseUp(ev: MouseEvent) {
      window.removeEventListener('mousemove', onMouseMove);
      window.removeEventListener('mouseup', onMouseUp);
      dragHandleActive = false;

      if (nearMainWindow) {
        await dockBack();
      }
    }

    window.addEventListener('mousemove', onMouseMove);
    window.addEventListener('mouseup', onMouseUp);
  }

  async function checkMainWindowProximity(screenX: number, screenY: number) {
    if (!isTauri) return;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      // Use a small rect around the cursor as the "pop-out bounds"
      const result = await invoke<{ near: boolean }>('check_dock_proximity', {
        popoutX: screenX - 50,
        popoutY: screenY - 50,
        popoutWidth: 100,
        popoutHeight: 100,
      });
      nearMainWindow = result.near;
    } catch { /* ignore */ }
  }

  async function dockBack() {
    if (!isTauri) return;
    try {
      // Call Rust command — it emits event to main window and closes this one
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('dock_panel_back', { panelId });
    } catch (e) {
      console.error('[silmaril] dockBack error:', e);
    }
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="popout-container" class:near-dock={nearMainWindow}>
  <div class="popout-toolbar">
    <!-- Drag handle: grab this and drag over main editor to dock back -->
    <div
      class="drag-dock-handle"
      class:dragging={dragHandleActive}
      onmousedown={startDragHandle}
      title="Drag to dock back into editor"
    >
      <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor" opacity="0.6">
        <rect x="3" y="3" width="4" height="2" rx="0.5"/>
        <rect x="9" y="3" width="4" height="2" rx="0.5"/>
        <rect x="3" y="7" width="4" height="2" rx="0.5"/>
        <rect x="9" y="7" width="4" height="2" rx="0.5"/>
        <rect x="3" y="11" width="4" height="2" rx="0.5"/>
        <rect x="9" y="11" width="4" height="2" rx="0.5"/>
      </svg>
    </div>
    <button class="dock-back-btn" onclick={dockBack} title={t('popout.dock_back')}>
      {t('popout.dock_back')}
    </button>
    {#if nearMainWindow}
      <span class="dock-hint">Release to dock</span>
    {:else if dragHandleActive}
      <span class="dock-hint drag-active">Drag over editor to dock</span>
    {/if}
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

  .drag-dock-handle {
    cursor: grab;
    padding: 4px 6px;
    border-radius: 4px;
    display: flex;
    align-items: center;
    transition: background 0.1s;
  }
  .drag-dock-handle:hover {
    background: rgba(255, 255, 255, 0.1);
  }
  .drag-dock-handle.dragging {
    cursor: grabbing;
    background: var(--color-accent, #007acc);
  }
  .drag-dock-handle.dragging svg {
    opacity: 1;
  }

  .dock-hint.drag-active {
    color: var(--color-textMuted, #999);
  }

  .near-dock {
    outline: 2px solid var(--color-accent, #007acc);
    outline-offset: -2px;
  }

  .dock-hint {
    color: var(--color-accent, #007acc);
    font-size: 11px;
    font-weight: 600;
    margin-left: 8px;
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
