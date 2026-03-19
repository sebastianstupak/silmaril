<script lang="ts">
  import { onMount } from 'svelte';
  import { t } from '$lib/i18n';
  import { loadSettings } from '$lib/stores/settings';
  import { themes, applyTheme } from '$lib/theme/tokens';
  import { getPanelInfo } from '$lib/docking/types';
  import HierarchyWrapper from '$lib/docking/panels/HierarchyWrapper.svelte';
  import InspectorWrapper from '$lib/docking/panels/InspectorWrapper.svelte';
  import ConsoleWrapper from '$lib/docking/panels/ConsoleWrapper.svelte';
  import ViewportPanel from '$lib/docking/panels/ViewportPanel.svelte';
  import ProfilerPanel from '$lib/docking/panels/ProfilerPanel.svelte';
  import AssetsPanel from '$lib/docking/panels/AssetsPanel.svelte';
  import type { Component } from 'svelte';

  let { panelId }: { panelId: string } = $props();

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
  const info = getPanelInfo(basePanelId);
  const panelTitle = info ? t(info.titleKey) : panelId;

  let nearMainWindow = $state(false);
  let dragging = $state(false);
  let dragStartX = 0;
  let dragStartY = 0;
  let windowStartX = 0;
  let windowStartY = 0;

  onMount(() => {
    const settings = loadSettings();
    applyTheme(themes[settings.theme] ?? themes.dark);
    document.documentElement.style.fontSize = `${settings.fontSize}px`;
  });

  // ---- Title bar drag: moves window + detects dock proximity ----

  async function startTitleBarDrag(e: MouseEvent) {
    if (e.button !== 0) return;
    e.preventDefault();
    dragging = true;
    dragStartX = e.screenX;
    dragStartY = e.screenY;

    // Get current window position
    if (isTauri) {
      try {
        const { getCurrentWindow } = await import('@tauri-apps/api/window');
        const win = getCurrentWindow();
        const pos = await win.outerPosition();
        windowStartX = pos.x;
        windowStartY = pos.y;
      } catch { /* ignore */ }
    }

    function onMouseMove(ev: MouseEvent) {
      const dx = ev.screenX - dragStartX;
      const dy = ev.screenY - dragStartY;
      moveWindow(windowStartX + dx, windowStartY + dy);
      checkProximity(ev.screenX, ev.screenY);
    }

    async function onMouseUp() {
      window.removeEventListener('mousemove', onMouseMove);
      window.removeEventListener('mouseup', onMouseUp);
      dragging = false;

      if (nearMainWindow) {
        await dockBack();
      }
    }

    window.addEventListener('mousemove', onMouseMove);
    window.addEventListener('mouseup', onMouseUp);
  }

  async function moveWindow(x: number, y: number) {
    if (!isTauri) return;
    try {
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      const { LogicalPosition } = await import('@tauri-apps/api/dpi');
      await getCurrentWindow().setPosition(new LogicalPosition(x, y));
    } catch { /* ignore */ }
  }

  async function checkProximity(screenX: number, screenY: number) {
    if (!isTauri) return;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
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
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('dock_panel_back', { panelId });
    } catch (e) {
      console.error('[silmaril] dockBack error:', e);
    }
  }

  async function minimizeWindow() {
    if (!isTauri) return;
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    getCurrentWindow().minimize();
  }

  async function maximizeWindow() {
    if (!isTauri) return;
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    getCurrentWindow().toggleMaximize();
  }

  async function closeWindow() {
    if (!isTauri) return;
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    getCurrentWindow().close();
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="popout-container" class:near-dock={nearMainWindow}>
  <!-- Custom title bar — entire bar is draggable and handles docking -->
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    class="custom-titlebar"
    class:dragging
    onmousedown={startTitleBarDrag}
    ondblclick={maximizeWindow}
  >
    <!-- Panel icon + name -->
    <span class="titlebar-title">{panelTitle}</span>

    <!-- Dock Back button -->
    <button class="titlebar-btn dock-btn" onclick={(e) => { e.stopPropagation(); dockBack(); }} title="Dock back">
      <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
        <path d="M3 3h10v2H3V3zm0 4h10v6H3V7z" opacity="0.8"/>
      </svg>
    </button>

    <!-- Spacer -->
    <div class="titlebar-spacer"></div>

    <!-- Status during drag -->
    {#if nearMainWindow}
      <span class="dock-hint near">Release to dock</span>
    {:else if dragging}
      <span class="dock-hint">Drag over editor to dock</span>
    {/if}

    <!-- Window controls -->
    <button class="titlebar-btn" onclick={(e) => { e.stopPropagation(); minimizeWindow(); }} title="Minimize">
      <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor"><path d="M3 8h10v1H3z"/></svg>
    </button>
    <button class="titlebar-btn" onclick={(e) => { e.stopPropagation(); maximizeWindow(); }} title="Maximize">
      <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor"><path d="M3 3h10v10H3V3zm1 2v7h8V5H4z"/></svg>
    </button>
    <button class="titlebar-btn close-btn" onclick={(e) => { e.stopPropagation(); closeWindow(); }} title="Close">
      <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor"><path d="M4.11 3.05L8 6.94l3.89-3.89.71.71L8.71 7.65l3.89 3.89-.71.71L8 8.36l-3.89 3.89-.71-.71 3.89-3.89-3.89-3.89.71-.71z"/></svg>
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

  .custom-titlebar {
    display: flex;
    align-items: center;
    height: 32px;
    background: var(--color-bgHeader, #2d2d2d);
    border-bottom: 1px solid var(--color-border, #404040);
    padding: 0 4px;
    user-select: none;
    cursor: grab;
    flex-shrink: 0;
  }
  .custom-titlebar.dragging {
    cursor: grabbing;
    background: var(--color-bg, #1e1e1e);
  }

  .titlebar-title {
    font-size: 12px;
    font-weight: 500;
    color: var(--color-textMuted, #999);
    padding: 0 8px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .titlebar-spacer { flex: 1; }

  .titlebar-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 28px;
    background: none;
    border: none;
    color: var(--color-textMuted, #999);
    cursor: pointer;
    transition: background 0.1s;
  }
  .titlebar-btn:hover {
    background: rgba(255, 255, 255, 0.1);
    color: var(--color-text, #ccc);
  }
  .close-btn:hover { background: #e81123; color: white; }
  .dock-btn:hover { background: var(--color-accent, #007acc); color: white; }

  .dock-hint {
    font-size: 11px;
    color: var(--color-textMuted, #999);
    margin-right: 8px;
    white-space: nowrap;
  }
  .dock-hint.near {
    color: var(--color-accent, #007acc);
    font-weight: 600;
  }

  .near-dock {
    outline: 3px solid var(--color-accent, #007acc);
    outline-offset: -3px;
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
