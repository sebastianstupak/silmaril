<script lang="ts">
  import { onMount } from 'svelte';
  // invoke() is how we talk to Tauri — we know it works from the pop-out
  // webview because create_native_viewport and dock_panel_back use it fine.
  import { invoke } from '@tauri-apps/api/core';

  import { t } from '$lib/i18n';
  import { setLocale } from '$lib/i18n';
  import { loadSettings } from '$lib/stores/settings';
  import { themes, applyTheme } from '$lib/theme/tokens';
  import { getPanelTitle } from '$lib/contributions/registry';
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
  const panelTitle = getPanelTitle(basePanelId);

  onMount(() => {
    const SETTINGS_KEY = 'silmaril-editor-settings';
    let lastJson = localStorage.getItem(SETTINGS_KEY) ?? '';

    // Apply settings from localStorage immediately.
    const s = loadSettings();
    applyTheme(themes[s.theme] ?? themes.dark);
    document.documentElement.style.fontSize = `${s.fontSize}px`;
    setLocale(s.language);

    // Re-apply whenever the stored JSON changes.  Compare raw strings so we
    // only do work when something actually changed.
    function syncSettings() {
      const json = localStorage.getItem(SETTINGS_KEY);
      if (!json || json === lastJson) return;
      lastJson = json;
      try {
        const updated = JSON.parse(json);
        applyTheme(themes[updated.theme] ?? themes.dark);
        document.documentElement.style.fontSize = `${updated.fontSize}px`;
        setLocale(updated.language ?? 'en');
      } catch { /* ignore corrupted JSON */ }
    }

    // Fast path: `storage` event fires in other windows when localStorage is
    // written — works when WebView2 propagates it across instances.
    window.addEventListener('storage', syncSettings);

    // Reliable fallback: poll every 300 ms.  WebView2 pop-out windows share
    // the same user-data folder so localStorage.getItem always returns the
    // value written by the main window, even if the storage event is not fired.
    const pollTimer = setInterval(syncSettings, 300);

    return () => {
      window.removeEventListener('storage', syncSettings);
      clearInterval(pollTimer);
    };
  });

  // ── Dock back ──────────────────────────────────────────────────────────────

  async function dockBack() {
    if (!isTauri) return;
    try {
      await invoke('dock_panel_back', { panelId, zone: 'center' });
    } catch (e) {
      console.error('[silmaril] dockBack error:', e);
    }
  }

  // ── Drag ───────────────────────────────────────────────────────────────────
  //
  // Handled entirely via window_start_drag (Win32 ReleaseCapture +
  // PostMessage WM_NCLBUTTONDOWN/HTCAPTION).  We do NOT use
  // data-tauri-drag-region: in Tauri 2 with decorations:false, that attribute
  // routes through WebView2's native non-client-region API which causes the OS
  // to consume the mousedown before JS fires, preventing start_dock_drag from
  // ever being invoked.

  function onTitlebarMousedown(e: MouseEvent) {
    if (!isTauri) return;
    if (e.button !== 0) return;                                          // left button only
    if ((e.target as HTMLElement).closest('button')) return;            // buttons handle themselves
    // Fire-and-forget — do NOT await, command returns immediately via PostMessage
    invoke('window_start_drag').catch((err) =>
      console.error('[silmaril] window_start_drag error:', err)
    );
    // Start the dock-proximity polling thread so dragging over the main editor
    // shows the dock zone overlay and auto-docks on release.
    invoke('start_dock_drag', { panelId }).catch((err) =>
      console.error('[silmaril] start_dock_drag error:', err)
    );
  }

  function onTitlebarDblclick(e: MouseEvent) {
    if (!isTauri) return;
    if ((e.target as HTMLElement).closest('button')) return;
    invoke('window_toggle_maximize').catch(console.error);
  }

  // ── Window controls ────────────────────────────────────────────────────────
  //
  // All use custom Rust commands registered in invoke_handler.  These receive
  // the calling WebviewWindow directly from Tauri so there is no dependency
  // on getCurrentWebviewWindow() or the plugin:window|* command path.

  function minimizeWindow() {
    if (!isTauri) return;
    invoke('window_minimize').catch(console.error);
  }

  function maximizeWindow() {
    if (!isTauri) return;
    invoke('window_toggle_maximize').catch(console.error);
  }

  function closeWindow() {
    if (!isTauri) return;
    invoke('window_close').catch(console.error);
  }
</script>

<div class="popout-container">
  <!--
    Drag is handled entirely by our custom window_start_drag command
    (Win32 ReleaseCapture + PostMessage WM_NCLBUTTONDOWN/HTCAPTION).
    We intentionally do NOT use data-tauri-drag-region here: in Tauri 2
    with decorations:false, that attribute routes through WebView2's native
    non-client-region API, which causes the OS to consume WM_NCLBUTTONDOWN
    before the JS mousedown event fires — so our start_dock_drag invoke
    would never execute.
  -->
  <div
    class="custom-titlebar"
    onmousedown={onTitlebarMousedown}
    ondblclick={onTitlebarDblclick}
    role="toolbar"
    aria-label="Window titlebar"
  >
    <span class="titlebar-title">{panelTitle}</span>

    <button class="titlebar-btn dock-btn" onclick={dockBack} title="Dock back">
      <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
        <path d="M3 3h10v2H3V3zm0 4h10v6H3V7z" opacity="0.8"/>
      </svg>
    </button>

    <div class="titlebar-spacer"></div>

    <button class="titlebar-btn" onclick={minimizeWindow} title="Minimize">
      <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
        <path d="M3 8h10v1H3z"/>
      </svg>
    </button>
    <button class="titlebar-btn" onclick={maximizeWindow} title="Maximize">
      <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
        <path d="M3 3h10v10H3V3zm1 2v7h8V5H4z"/>
      </svg>
    </button>
    <button class="titlebar-btn close-btn" onclick={closeWindow} title="Close">
      <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
        <path d="M4.11 3.05L8 6.94l3.89-3.89.71.71L8.71 7.65l3.89 3.89-.71.71L8 8.36l-3.89 3.89-.71-.71 3.89-3.89-3.89-3.89.71-.71z"/>
      </svg>
    </button>
  </div>

  {#if PanelComponent}
    <div class="popout-content">
      <PanelComponent {panelId} />
    </div>
  {:else}
    <p class="popout-unknown">{t('popout.unknown')}: {panelId}</p>
  {/if}
</div>

<style>
  .popout-container {
    width: 100vw;
    height: 100vh;
    /* Transparent — Vulkan DXGI swapchain on the parent HWND shows through. */
    background: transparent;
    color: var(--color-text, #ccc);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    /* Rounded corners to match DWM window shape and clip the title bar. */
    border-radius: 8px;
  }

  .custom-titlebar {
    display: flex;
    align-items: center;
    height: 32px;
    min-height: 32px;
    background: var(--color-bgTitleBar, #141414);
    border-bottom: 1px solid var(--color-border, #404040);
    padding: 0 4px;
    user-select: none;
    flex-shrink: 0;
    cursor: default;
    /* Sit above the viewport content in the stacking context. */
    position: relative;
    z-index: 100;
  }

  .titlebar-title {
    font-size: 12px;
    font-weight: 500;
    color: var(--color-textMuted, #999);
    padding: 0 8px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    cursor: default;
  }

  .titlebar-spacer {
    flex: 1;
    height: 100%;
    cursor: default;
  }

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
    /* Explicitly opt out of OS drag-region so clicks register as clicks. */
    -webkit-app-region: no-drag;
    app-region: no-drag;
  }

  .titlebar-btn:hover {
    background: rgba(255, 255, 255, 0.1);
    color: var(--color-text, #ccc);
  }

  .close-btn:hover { background: #c42b1c; color: white; }
  .dock-btn:hover  { background: var(--color-accent, #007acc); color: white; }

  .popout-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    min-height: 0;
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
