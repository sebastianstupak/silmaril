<script lang="ts">
  import { onMount } from 'svelte';
  import { getEditorState, openProjectDialog, openProject, scanProjectEntities, type EditorState } from './lib/api';
  import { t } from './lib/i18n';
  import { setLocale } from './lib/i18n';
  import SettingsDialog from './lib/components/SettingsDialog.svelte';
  import PopoutView from './lib/components/PopoutView.svelte';
  import ViewportOverlay from './lib/components/ViewportOverlay.svelte';
  import { themes, applyTheme } from './lib/theme/tokens';
  import { loadSettings, saveSettings, hydrateSettings, type EditorSettings } from './lib/stores/settings';
  import { setEntities, setSelectedEntityId } from './lib/stores/editor-context';
  import { logInfo, logWarn } from './lib/stores/console';
  import { loadSchemas } from './lib/inspector/schema-store';
  import DockContainer from './lib/docking/DockContainer.svelte';
  import DockSplitter from './lib/docking/DockSplitter.svelte';
  import DragOverlay from './lib/docking/DragOverlay.svelte';
  import TitleBar from './lib/components/TitleBar.svelte';
  import { loadLayout, saveLayout, defaultLayout, resizeSplit, cycleActiveTab, startDrag, endDrag, getDragState, dropPanel, loadSavedLayouts, saveSavedLayouts, hydrateLayout, hydrateSavedLayouts, type SavedLayout } from './lib/docking/store';
  import type { EditorLayout, LayoutNode } from './lib/docking/types';

  // Panel components (no-prop wrappers for docking)
  import HierarchyWrapper from './lib/docking/panels/HierarchyWrapper.svelte';
  import InspectorWrapper from './lib/docking/panels/InspectorWrapper.svelte';
  import ConsoleWrapper from './lib/docking/panels/ConsoleWrapper.svelte';
  import ViewportPanel from './lib/docking/panels/ViewportPanel.svelte';
  import ProfilerPanel from './lib/docking/panels/ProfilerPanel.svelte';
  import AssetsPanel from './lib/docking/panels/AssetsPanel.svelte';

  /** Detect if running inside Tauri or standalone browser */
  const isTauri = typeof window !== 'undefined' && !!(window as any).__TAURI_INTERNALS__;

  // Pop-out panel detection via query parameter
  const popoutPanel = new URLSearchParams(window.location.search).get('panel');

  // Viewport overlay detection — rendered in a separate transparent WebView2
  // that sits above the Vulkan child window in the sandwich architecture.
  const isViewportOverlay = new URLSearchParams(window.location.search).get('overlay') === 'viewport';

  let editorState: EditorState | null = $state(null);

  // Load settings once; reuse for both the reactive state and the initial bottomHeight.
  const _initial = loadSettings();
  let settings: EditorSettings = $state(_initial);
  let showSettings = $state(false);

  // Docking layout state
  let layout: EditorLayout = $state(loadLayout());
  let bottomHeight = $state(_initial.bottomPanelHeight);

  function _collectPanels(node: LayoutNode, out: Set<string>) {
    if (node.type === 'tabs') { for (const p of node.panels) out.add(p); }
    else { for (const c of node.children) _collectPanels(c, out); }
  }
  let activePanels = $derived.by(() => {
    const s = new Set<string>();
    _collectPanels(layout.root, s);
    _collectPanels(layout.bottomPanel, s);
    return s;
  });

  // ── Saved layouts ──────────────────────────────────────────────────────────
  let savedLayouts: SavedLayout[] = $state(loadSavedLayouts());
  let activeLayoutId: string | null = $state(savedLayouts[0]?.id ?? null);
  let isDirty = $derived.by(() => {
    if (!activeLayoutId) return false;
    const slot = savedLayouts.find(s => s.id === activeLayoutId);
    if (!slot) return false;
    return JSON.stringify(layout) !== JSON.stringify(slot.layout);
  });
  const MIN_BOTTOM = 150;
  const MAX_BOTTOM_RATIO = 0.6;

  const panelComponents: Record<string, any> = {
    hierarchy: HierarchyWrapper,
    viewport: ViewportPanel,
    inspector: InspectorWrapper,
    console: ConsoleWrapper,
    profiler: ProfilerPanel,
    assets: AssetsPanel,
  };

  async function handleOpenProject() {
    const path = await openProjectDialog();
    if (!path) return;

    try {
      editorState = await openProject(path);
      const entities = await scanProjectEntities(path);
      setEntities(entities);
      setSelectedEntityId(null);
      logInfo(`Project loaded: ${editorState.project_name}`);
      logInfo(`Found ${entities.length} entities in scene`);
    } catch {
      logWarn('Failed to open project');
    }
  }

  function handleLayoutChange(newLayout: EditorLayout) {
    layout = newLayout;
    saveLayout(layout);
  }

  function handleLayoutReset() {
    layout = JSON.parse(JSON.stringify(defaultLayout));
    saveLayout(layout);
  }

  // ── Saved layout handlers ──────────────────────────────────────────────────
  function applyLayout(id: string) {
    const slot = savedLayouts.find(s => s.id === id);
    if (!slot) return;
    layout = JSON.parse(JSON.stringify(slot.layout));
    activeLayoutId = id;
    saveLayout(layout);
  }

  function saveToSlot(id: string) {
    savedLayouts = savedLayouts.map(s =>
      s.id === id ? { ...s, layout: JSON.parse(JSON.stringify(layout)) } : s
    );
    saveSavedLayouts(savedLayouts);
  }

  function resetSlot(id: string) {
    const slot = savedLayouts.find(s => s.id === id);
    if (!slot) return;
    layout = JSON.parse(JSON.stringify(slot.layout));
    saveLayout(layout);
  }

  function renameSlot(id: string, name: string) {
    savedLayouts = savedLayouts.map(s => s.id === id ? { ...s, name } : s);
    saveSavedLayouts(savedLayouts);
  }

  function duplicateSlot(id: string) {
    const slot = savedLayouts.find(s => s.id === id);
    if (!slot) return;
    const newSlot: SavedLayout = {
      id: `layout-${Date.now()}`,
      name: `${slot.name} Copy`,
      layout: JSON.parse(JSON.stringify(slot.layout)),
    };
    savedLayouts = [...savedLayouts, newSlot];
    saveSavedLayouts(savedLayouts);
  }

  function deleteSlot(id: string) {
    if (savedLayouts.length <= 1) return;
    savedLayouts = savedLayouts.filter(s => s.id !== id);
    if (activeLayoutId === id) activeLayoutId = savedLayouts[0]?.id ?? null;
    saveSavedLayouts(savedLayouts);
  }

  function createLayout(name: string) {
    const newSlot: SavedLayout = {
      id: `layout-${Date.now()}`,
      name,
      layout: JSON.parse(JSON.stringify(layout)),
    };
    savedLayouts = [...savedLayouts, newSlot];
    activeLayoutId = newSlot.id;
    saveSavedLayouts(savedLayouts);
  }

  /** Add a panel back into the layout at the specified dock zone. */
  function addPanelToLayout(panelId: string, zone: string = 'center') {
    const newTab: import('./lib/docking/types').TabsNode = {
      type: 'tabs',
      activeTab: 0,
      panels: [panelId],
    };

    // Bottom zone → add to bottom panel
    if (zone === 'bottom') {
      const target = findFirstTabsNode(layout.bottomPanel);
      if (target) {
        target.panels.push(panelId);
        target.activeTab = target.panels.length - 1;
      } else {
        layout.bottomPanel = { type: 'tabs', activeTab: 0, panels: [panelId] };
      }
      layout = { ...layout };
      saveLayout(layout);
      return;
    }

    const root = layout.root;

    if (zone === 'center') {
      // Add as tab in the middle tabs node
      if (root.type === 'tabs') {
        root.panels.push(panelId);
        root.activeTab = root.panels.length - 1;
      } else {
        const allTabs = collectAllTabsNodes(root);
        const target = allTabs.length > 1 ? allTabs[Math.floor(allTabs.length / 2)] : allTabs[0];
        if (target) {
          target.panels.push(panelId);
          target.activeTab = target.panels.length - 1;
        }
      }
    } else if (zone === 'left' || zone === 'right') {
      if (root.type === 'tabs') {
        layout.root = {
          type: 'split',
          direction: 'horizontal',
          sizes: zone === 'left' ? [25, 75] : [75, 25],
          children: zone === 'left' ? [newTab, root] : [root, newTab],
        };
      } else {
        const newSize = 20;
        const scale = (100 - newSize) / 100;
        root.sizes = root.sizes.map(s => s * scale);
        if (zone === 'left') {
          root.children.unshift(newTab);
          root.sizes.unshift(newSize);
        } else {
          root.children.push(newTab);
          root.sizes.push(newSize);
        }
      }
    } else if (zone === 'top') {
      layout.root = {
        type: 'split',
        direction: 'vertical',
        sizes: [25, 75],
        children: [newTab, root],
      };
    }

    layout = { ...layout };
    saveLayout(layout);
  }

  function collectAllTabsNodes(node: import('./lib/docking/types').LayoutNode): import('./lib/docking/types').TabsNode[] {
    if (node.type === 'tabs') return [node];
    const result: import('./lib/docking/types').TabsNode[] = [];
    for (const child of node.children) {
      result.push(...collectAllTabsNodes(child));
    }
    return result;
  }

  function findFirstTabsNode(node: import('./lib/docking/types').LayoutNode): import('./lib/docking/types').TabsNode | null {
    if (node.type === 'tabs') return node;
    for (const child of node.children) {
      const found = findFirstTabsNode(child);
      if (found) return found;
    }
    return null;
  }

  function hasAnyPanels(node: import('./lib/docking/types').LayoutNode): boolean {
    if (node.type === 'tabs') return node.panels.length > 0;
    return node.children.some(hasAnyPanels);
  }

  function clampBottom(h: number): number {
    return Math.max(MIN_BOTTOM, Math.min(h, window.innerHeight * MAX_BOTTOM_RATIO));
  }

  function onResizeBottom(delta: number) {
    bottomHeight = clampBottom(bottomHeight - delta);
  }

  // Keyboard shortcuts: Ctrl+Tab (tab cycle) + layout keybinds (per-slot).
  $effect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if (e.key === 'Tab' && e.ctrlKey) {
        e.preventDefault();
        cycleActiveTab(e.shiftKey ? -1 : 1);
        return;
      }
      // Settings: Ctrl+,
      if (e.key === ',' && e.ctrlKey && !e.shiftKey && !e.altKey) {
        e.preventDefault();
        showSettings = true;
        return;
      }
      // Layout slot keybinds — format "ctrl+1", "ctrl+2", etc.
      if (e.ctrlKey && !e.shiftKey && !e.altKey && !e.metaKey) {
        const slot = savedLayouts.find(s => s.keybind === `ctrl+${e.key}`);
        if (slot) { e.preventDefault(); applyLayout(slot.id); }
      }
    }
    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  });

  // Apply theme/locale whenever settings change (main window reactive update).
  $effect(() => {
    applyTheme(themes[settings.theme] ?? themes.dark);
    document.documentElement.style.fontSize = `${settings.fontSize}px`;
    setLocale(settings.language);
  });

  // Persist settings on change (debounced) and broadcast to pop-out windows.
  let saveTimer: ReturnType<typeof setTimeout> | null = null;
  $effect(() => {
    // Access properties explicitly so the effect re-runs when any of these change.
    settings.theme; settings.fontSize; settings.language; settings.autoSave; settings.compactMenu;
    bottomHeight;
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(() => {
      saveSettings({ ...settings, bottomPanelHeight: bottomHeight });
    }, 300);
  });

  function handleSettingsChange(updated: EditorSettings) {
    settings = updated;
    // Broadcast directly here rather than inside $effect — the call is triggered
    // synchronously from the settings dialog callback so the values are always fresh.
    // AppHandle::emit() on the Rust side broadcasts to ALL open webviews.
    if (isTauri && !popoutPanel && !isViewportOverlay) {
      import('@tauri-apps/api/core').then(({ invoke }) => {
        invoke('broadcast_settings', {
          theme: updated.theme,
          fontSize: updated.fontSize,
          language: updated.language,
        }).catch((e) => console.error('[silmaril] broadcast_settings error:', e));
      });
    }
  }

  function updateLayoutKeybind(id: string, keybind: string | undefined) {
    savedLayouts = savedLayouts.map(s => s.id === id ? { ...s, keybind } : s);
    saveSavedLayouts(savedLayouts);
  }

  onMount(async () => {
    setLocale(settings.language);
    applyTheme(themes[settings.theme] ?? themes.dark);
    document.documentElement.style.fontSize = `${settings.fontSize}px`;
    logInfo('Silmaril Editor started');
    loadSchemas(); // fire-and-forget; store notifies subscribers when ready
    editorState = await getEditorState();

    // Hydrate from tauri-plugin-store (durable, OS app-data directory).
    // This runs after the initial render so the UI is not blocked.
    if (isTauri) {
      const [hydratedSettings, hydratedLayout, hydratedLayouts] = await Promise.all([
        hydrateSettings(),
        hydrateLayout(),
        hydrateSavedLayouts(),
      ]);
      settings = hydratedSettings;
      bottomHeight = hydratedSettings.bottomPanelHeight;
      if (hydratedLayout) layout = hydratedLayout;
      if (hydratedLayouts) savedLayouts = hydratedLayouts;
    }

    // Listen for events from pop-out windows
    if (isTauri && !popoutPanel) {
      try {
        const { listen } = await import('@tauri-apps/api/event');

        // Panel docked back — use the per-panel drop target tracked by DockDropZone,
        // falling back to the whole-window zone from Rust if no panel was hovered.
        await listen<{ panelId: string; zone?: string }>('dock-panel-back', (event) => {
          const { dropPath, dropZone, dropIsBottom } = getDragState();
          endDrag();
          if (dropPath !== null && dropZone !== null) {
            const newLayout = dropPanel(layout, event.payload.panelId, dropPath, dropZone, dropIsBottom);
            layout = newLayout;
            saveLayout(layout);
          } else {
            addPanelToLayout(event.payload.panelId, event.payload.zone ?? 'center');
          }
        });

        // Pop-out proximity — drive the shared drag store so DockDropZone
        // instances in each panel show their zones exactly like internal drag.
        await listen<{ near: boolean; panelId: string; relX: number; relY: number }>('popout-near', (event) => {
          const { near, panelId, relX, relY } = event.payload;
          if (near) {
            startDrag(panelId, relX * window.innerWidth, relY * window.innerHeight, true);
          } else {
            endDrag();
          }
        });
      } catch (e) {
        console.error('[silmaril] Failed to listen for pop-out events:', e);
      }
    }
  });
</script>

{#if isViewportOverlay}
  <ViewportOverlay />
{:else if popoutPanel}
  <PopoutView panelId={popoutPanel} />
{:else}
<main class="editor-shell">
  <!-- Title Bar (custom, replaces native OS decorations) -->
  {#if isTauri}
    <TitleBar
        {savedLayouts}
        {activeLayoutId}
        {isDirty}
        {activePanels}
        onApplyLayout={applyLayout}
        onSaveToSlot={saveToSlot}
        onResetSlot={resetSlot}
        onRenameSlot={renameSlot}
        onDuplicateSlot={duplicateSlot}
        onDeleteSlot={deleteSlot}
        onCreateLayout={createLayout}
        onAddPanel={(id) => addPanelToLayout(id)}
        onSettingsOpen={() => showSettings = true}
        onOpenProject={handleOpenProject}
        onLayoutReset={handleLayoutReset}
        compactMenu={settings.compactMenu}
      />
  {/if}

  <!-- Toolbar -->
  <div class="toolbar">
    <!-- Left: breadcrumb -->
    <div class="toolbar-left">
      <span class="breadcrumb">
        <span class="breadcrumb-segment">{editorState?.project_name ?? t('breadcrumb.no_project')}</span>
        <span class="breadcrumb-sep">
          <svg width="10" height="10" viewBox="0 0 16 16" fill="currentColor">
            <path d="M5.7 13.7l5-5a1 1 0 000-1.4l-5-5" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round"/>
          </svg>
        </span>
        <span class="breadcrumb-segment">{t('breadcrumb.no_scene')}</span>
      </span>
    </div>

    <!-- Center: transport controls -->
    <div class="toolbar-center">
      <div class="transport-group">
        <button class="toolbar-btn transport-btn transport-play" title={t('toolbar.play')}>
          <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
            <path d="M4 2l10 6-10 6V2z"/>
          </svg>
        </button>
        <button class="toolbar-btn transport-btn" title={t('toolbar.pause')}>
          <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
            <rect x="3" y="2" width="4" height="12"/>
            <rect x="9" y="2" width="4" height="12"/>
          </svg>
        </button>
        <button class="toolbar-btn transport-btn" title={t('toolbar.stop')}>
          <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
            <rect x="3" y="3" width="10" height="10"/>
          </svg>
        </button>
      </div>
    </div>

    <!-- Right: mode badge -->
    <div class="toolbar-right">
      <span class="mode-badge">{editorState?.mode ?? '...'}</span>
    </div>
  </div>

  <!-- Settings Dialog -->
  <SettingsDialog
    bind:open={showSettings}
    {settings}
    {savedLayouts}
    onSettingsChange={handleSettingsChange}
    onUpdateLayoutKeybind={updateLayoutKeybind}
  />

  <!-- Main content area: docked panels + bottom panel -->
  <div class="content-area">
    <div class="main-area">
      <DockContainer
        node={layout.root}
        {layout}
        {panelComponents}
        onLayoutChange={handleLayoutChange}
      />
    </div>

    {#if hasAnyPanels(layout.bottomPanel)}
      <DockSplitter direction="vertical" onResize={onResizeBottom} />

      <div class="bottom-bar" style="height: {bottomHeight}px">
        <DockContainer
          node={layout.bottomPanel}
          {layout}
          {panelComponents}
          onLayoutChange={handleLayoutChange}
          isBottomPanel={true}
        />
      </div>
    {/if}
  </div>

  <!-- Drag overlay (ghost tab + backdrop during panel drag) -->
  <DragOverlay />


  <!-- Status bar -->
  <div class="status-bar">
    <div class="status-left">
      <span class="status-item">{t('placeholder.select_entity')}</span>
    </div>
    <div class="status-center">
      <span class="status-item">{t('status.ready')}</span>
    </div>
    <div class="status-right">
      <span class="status-item">{t('status.fps')}: --</span>
      <span class="status-divider"></span>
      <span class="status-item">{t('status.memory')}: --</span>
    </div>
  </div>
</main>
{/if}

<style>
  .editor-shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: transparent;
    color: var(--color-text, #cccccc);
    font-family: var(--font-body, system-ui, -apple-system, sans-serif);
    /* Clip all child content (panels, title bar) to the OS window corner radius
       so solid backgrounds don't bleed into the transparent corner regions. */
    border-radius: 8px;
    overflow: hidden;
  }

  /* Toolbar */
  .toolbar {
    height: 38px;
    display: flex;
    align-items: center;
    padding: 0 12px;
    background: var(--color-bgHeader, #2d2d2d);
    border-bottom: 1px solid var(--color-border, #404040);
    gap: 8px;
    flex-shrink: 0;
  }
  .toolbar-left {
    flex: 1;
    display: flex;
    align-items: center;
    min-width: 0;
  }
  .toolbar-center {
    display: flex;
    align-items: center;
    flex-shrink: 0;
  }
  .toolbar-right {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
  }

  /* Breadcrumb */
  .breadcrumb {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 12px;
    color: var(--color-textMuted, #999);
    overflow: hidden;
  }
  .breadcrumb-segment {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .breadcrumb-sep {
    display: flex;
    align-items: center;
    color: var(--color-textDim, #666);
    flex-shrink: 0;
  }

  /* Transport controls */
  .transport-group {
    display: flex;
    align-items: center;
    gap: 0;
    border: 1px solid var(--color-border, #404040);
    border-radius: 5px;
    background: var(--color-bg, #1e1e1e);
    overflow: hidden;
  }
  .transport-btn {
    border-radius: 0;
    border: none;
    border-right: 1px solid var(--color-border, #404040);
    padding: 4px 8px;
  }
  .transport-btn:last-child {
    border-right: none;
  }
  .transport-play {
    padding: 4px 10px;
  }
  .transport-play:hover {
    color: var(--color-accent, #007acc);
    background: var(--color-bgPanel, #252525);
  }

  /* Mode badge */
  .mode-badge {
    padding: 2px 8px;
    background: var(--color-accent, #007acc);
    color: #fff;
    border-radius: 3px;
    font-size: 11px;
    text-transform: uppercase;
    font-weight: 600;
  }
  .toolbar-btn {
    background: none;
    border: 1px solid transparent;
    color: var(--color-textMuted, #999);
    cursor: pointer;
    padding: 4px;
    border-radius: 4px;
    display: flex;
    align-items: center;
  }
  .toolbar-btn:hover {
    color: var(--color-text, #ccc);
    background: var(--color-bgPanel, #252525);
    border-color: var(--color-border, #404040);
  }

  /* Layout */
  .content-area {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .main-area {
    flex: 1;
    display: flex;
    overflow: hidden;
  }
  .bottom-bar {
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }


  .status-bar {
    height: 24px;
    display: flex;
    align-items: center;
    padding: 0 12px;
    background: var(--color-bgHeader, #2d2d2d);
    border-top: 1px solid var(--color-border, #404040);
    font-size: 11px;
    color: var(--color-textDim, #666);
    flex-shrink: 0;
    gap: 8px;
  }
  .status-left {
    flex: 1;
    display: flex;
    align-items: center;
    min-width: 0;
    overflow: hidden;
  }
  .status-center {
    flex-shrink: 0;
    display: flex;
    align-items: center;
  }
  .status-right {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
  }
  .status-item {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .status-divider {
    width: 1px;
    height: 12px;
    background: var(--color-border, #404040);
    flex-shrink: 0;
  }
</style>
