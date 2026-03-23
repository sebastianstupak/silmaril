<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { getEditorState, openProjectDialog, openProject, scanProjectEntities, type EditorState } from './lib/api';
  import { t } from './lib/i18n';
  import { setLocale } from './lib/i18n';
  import SettingsDialog from './lib/components/SettingsDialog.svelte';
  import PopoutView from './lib/components/PopoutView.svelte';
  import ViewportOverlay from './lib/components/ViewportOverlay.svelte';
  import { themes, applyTheme } from './lib/theme/tokens';
  import { loadSettings, saveSettings, hydrateSettings, type EditorSettings } from './lib/stores/settings';
  import { hydrateRecentItems, addRecentItem, subscribeRecent, getRecentItems, type RecentItem } from './lib/stores/recent-items';
  import { setEntities, setSelectedEntityId } from './lib/stores/editor-context';
  import { setAssets, clearAssets, type AssetEntry } from './lib/stores/assets';
  import { scanAssets } from './lib/api';
  import { logInfo, logWarn } from './lib/stores/console';
  import { undo, redo, templateUndo, templateRedo, getCanUndo, getCanRedo, getViewportFocused, subscribeUndoHistory } from './lib/stores/undo-history';
  import { loadSchemas } from './lib/inspector/schema-store';
  import DockContainer from './lib/docking/DockContainer.svelte';
  import DockSplitter from './lib/docking/DockSplitter.svelte';
  import DragOverlay from './lib/docking/DragOverlay.svelte';
  import TitleBar from './lib/components/TitleBar.svelte';
  import ResizeHandles from './lib/components/ResizeHandles.svelte';
  import { loadLayout, saveLayout, defaultLayout, resizeSplit, cycleActiveTab, startDrag, endDrag, getDragState, dropPanel, loadSavedLayouts, saveSavedLayouts, hydrateLayout, hydrateSavedLayouts, type SavedLayout } from './lib/docking/store';
  import type { EditorLayout, LayoutNode } from './lib/docking/types';
  import { setLayout as setLayoutStore, subscribeLayout, getLayout as getLayoutStore } from './lib/stores/layout';
  import { registerCommand } from './lib/omnibar/registry';
  import { dispatchTemplateCommand } from './lib/template/commands';
  import { populateRegistry, listSpecs, dispatchCommand, setUndoVerifier } from './lib/dispatch';
  import { registerAllHandlers } from './lib/commands/index';
  import { initTauriListeners } from './lib/template/state';
  import AiPermissionDialog from './lib/components/AiPermissionDialog.svelte';
  import { aiServerRunning, aiServerPort, refreshAiServerStatus } from './lib/stores/ai-server';

  import { registerBuiltinPanels } from './lib/contributions/builtins';
  import { getPanelContributions, subscribePanelContributions } from './lib/contributions/registry';
  import type { PanelContribution } from './lib/contributions/registry';

  /** Detect if running inside Tauri or standalone browser */
  const isTauri = typeof window !== 'undefined' && !!(window as any).__TAURI_INTERNALS__;

  // Pop-out panel detection via query parameter
  const popoutPanel = new URLSearchParams(window.location.search).get('panel');

  // Viewport overlay detection — rendered in a separate transparent WebView2
  // that sits above the Vulkan child window in the sandwich architecture.
  const isViewportOverlay = new URLSearchParams(window.location.search).get('overlay') === 'viewport';

  let editorState: EditorState | null = $state(null);
  let omnibarOpen = $state(false);
  let canUndoState = $state(false);
  let canRedoState = $state(false);
  let recentItems = $state<RecentItem[]>([]);
  let unlistenCatalog: (() => void) | undefined;
  let unlistenAiCmd: (() => void) | undefined;
  let unlistenFullscreen: (() => void) | undefined;
  let _unsubPanels: (() => void) | undefined;

  // Load settings once; reuse for both the reactive state and the initial bottomHeight.
  const _initial = loadSettings();
  let settings: EditorSettings = $state(_initial);
  let showSettings = $state(false);

  // Docking layout state
  let layout: EditorLayout = $state(loadLayout());
  let bottomHeight = $state(_initial.bottomPanelHeight);

  // Keep the shared layout store in sync with App.svelte's authoritative state.
  // setLayout() is called here once on init and again inside handleLayoutChange().
  setLayoutStore(layout);

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

  // Call before first render so DockContainer can resolve components immediately
  registerBuiltinPanels();

  let panelContributions = $state<PanelContribution[]>(getPanelContributions());

  async function handleOpenProject() {
    const path = await openProjectDialog();
    if (!path) return;

    try {
      editorState = await openProject(path);
      addRecentItem({ label: editorState.project_name ?? path, path, itemType: 'project' });
      const entities = await scanProjectEntities(path);
      setEntities(entities);
      setSelectedEntityId(null);
      logInfo(`Project loaded: ${editorState.project_name}`);
      logInfo(`Found ${entities.length} entities in scene`);
      clearAssets();
      try {
        const raw = await scanAssets(path);
        setAssets(raw.map((a) => ({
          path: a.path,
          assetType: a.asset_type as AssetEntry['assetType'],
          filename: a.path.split(/[\\/]/).pop() ?? a.path,
        })));
      } catch {
        // non-fatal — assets panel will show empty
      }
    } catch {
      logWarn('Failed to open project');
    }
  }

  function handleLayoutChange(newLayout: EditorLayout) {
    layout = newLayout;
    setLayoutStore(layout);
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

  // Subscribe to undo/redo state changes.
  $effect(() => {
    return subscribeUndoHistory(() => {
      canUndoState = getCanUndo();
      canRedoState = getCanRedo();
    });
  });

  // React to layout changes made by view command handlers (e.g. toggle_hierarchy).
  // The store notifies when togglePanel() mutates the layout outside App.svelte.
  $effect(() => {
    return subscribeLayout(() => {
      const updated = getLayoutStore();
      if (updated !== null) {
        layout = updated;
        saveLayout(layout);
      }
    });
  });

  // Keyboard shortcuts: Ctrl+Tab (tab cycle) + dispatch layer + layout keybinds (per-slot).
  $effect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      // Ctrl+Tab: UI-only tab cycling (not a dispatchable command)
      if (e.key === 'Tab' && e.ctrlKey) {
        e.preventDefault();
        cycleActiveTab(e.shiftKey ? -1 : 1);
        return;
      }

      // Undo: Ctrl+Z
      if (e.key === 'z' && e.ctrlKey && !e.shiftKey && !e.altKey) {
        e.preventDefault();
        if (getViewportFocused()) {
          templateUndo();
        } else {
          undo();
        }
        return;
      }
      // Redo: Ctrl+Y or Ctrl+Shift+Z
      if ((e.key === 'y' && e.ctrlKey && !e.shiftKey && !e.altKey) ||
          (e.key === 'z' && e.ctrlKey && e.shiftKey && !e.altKey)) {
        e.preventDefault();
        if (getViewportFocused()) {
          templateRedo();
        } else {
          redo();
        }
        return;
      }
      // Ctrl+K: omnibar open (UI-only, not in command spec registry)
      if (e.key === 'k' && e.ctrlKey && !e.shiftKey && !e.altKey) {
        e.preventDefault();
        omnibarOpen = true;
        return;
      }

      // Ctrl+,: settings open (UI-only, not in command spec registry)
      if (e.key === ',' && e.ctrlKey && !e.shiftKey && !e.altKey) {
        e.preventDefault();
        showSettings = true;
        return;
      }

      // Route through the dispatch layer for all registered command keybinds.
      // Build a normalized keybind string (e.g. "Ctrl+Z", "Ctrl+Shift+Z", "Delete").
      const parts: string[] = [];
      if (e.ctrlKey) parts.push('Ctrl');
      if (e.shiftKey) parts.push('Shift');
      if (e.altKey) parts.push('Alt');
      // Normalize single-char keys to uppercase; keep special keys as-is.
      const keyName = e.key.length === 1 ? e.key.toUpperCase() : e.key;
      parts.push(keyName);
      const keybind = parts.join('+');

      const spec = listSpecs().find(s => s.keybind === keybind);
      if (spec) {
        e.preventDefault();
        dispatchCommand(spec.id).catch(console.error);
        return;
      }

      // Ctrl+Shift+Z: alternate redo keybind (no spec, falls through from above)
      if (e.key === 'z' && e.ctrlKey && e.shiftKey && !e.altKey) {
        e.preventDefault();
        redo();
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
    await initTauriListeners();

    // Set up fullscreen border-radius fix
    if (isTauri) {
      try {
        const tauriWindow = await import('@tauri-apps/api/window');
        const win = tauriWindow.getCurrentWindow();

        // Set initial state
        document.documentElement.dataset.fullscreen = String(await win.isFullscreen());

        // React to resize events (fullscreen toggle changes the window size)
        const unlisten = await win.onResized(async () => {
          document.documentElement.dataset.fullscreen = String(await win.isFullscreen());
        });

        // Store unlisten function for cleanup in onDestroy
        unlistenFullscreen = unlisten;
      } catch (e) {
        logWarn('Failed to set up fullscreen listener', { error: String(e) });
      }
    }

    _unsubPanels = subscribePanelContributions(() => {
      panelContributions = getPanelContributions();
    });
    setLocale(settings.language);
    applyTheme(themes[settings.theme] ?? themes.dark);
    document.documentElement.style.fontSize = `${settings.fontSize}px`;
    logInfo('Silmaril Editor started');
    refreshAiServerStatus(); // sync MCP badge with any previously running server

    // Wire the dispatch layer: register all TypeScript-side handlers first,
    // then populate the spec registry from Rust so keybind lookup works.
    registerAllHandlers();
    setUndoVerifier(() => getCanUndo());
    try {
      const { commands: bindingCommands } = await import('./lib/bindings');
      const specs = await bindingCommands.listCommands();
      populateRegistry(specs);
    } catch {
      // Non-fatal in browser/test environments where Tauri is not available.
    }

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

    await hydrateRecentItems();
    recentItems = getRecentItems();
    const _unsubRecent = subscribeRecent((items) => { recentItems = items; });

    // Register UI-only commands in the frontend registry
    registerCommand({
      id: 'edit.undo',
      label: 'Undo',
      category: 'Edit',
      keybind: 'Ctrl+Z',
      run: undo,
    });
    registerCommand({
      id: 'edit.redo',
      label: 'Redo',
      category: 'Edit',
      keybind: 'Ctrl+Y',
      run: redo,
    });
    registerCommand({
      id: 'ui.open_settings',
      label: 'Open Settings',
      category: 'Editor',
      keybind: 'Ctrl+,',
      run: () => { showSettings = true; },
    });
    registerCommand({
      id: 'ui.open_project',
      label: 'Open Project…',
      category: 'File',
      run: handleOpenProject,
    });
    registerCommand({
      id: 'ui.layout.reset',
      label: 'Reset Layout',
      category: 'Layout',
      run: handleLayoutReset,
    });

    // Listen for registry catalog updates from Rust (editor-catalog-updated).
    // This keeps the frontend spec registry in sync when modules are loaded at runtime.
    if (isTauri) {
      try {
        const { listen } = await import('@tauri-apps/api/event');
        unlistenCatalog = await listen<import('./lib/bindings').CommandSpec[]>('editor-catalog-updated', (event) => {
          populateRegistry(event.payload);
        });
      } catch {
        // Non-fatal in browser/test environments.
      }
    }

    // Listen for events from pop-out windows
    if (isTauri && !popoutPanel) {
      try {
        const { listen } = await import('@tauri-apps/api/event');
        const { invoke } = await import('@tauri-apps/api/core');

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
        // editor-run-command: routes Tauri backend commands to the scene
        if (!isViewportOverlay) {
          await listen<{ id: string }>('editor-run-command', (event) => {
            const { id } = event.payload;
            const mapping: Record<string, () => void> = {
              'editor.toggle_grid':       () => dispatchTemplateCommand('toggle_grid', {}),
              'editor.toggle_snap':       () => dispatchTemplateCommand('toggle_snap', {}),
              'editor.toggle_projection': () => dispatchTemplateCommand('toggle_projection', {}),
              'editor.new_template':      () => dispatchTemplateCommand('new_template', {}),
              'editor.reset_camera':      () => dispatchTemplateCommand('reset_camera', {}),
              'editor.set_tool.select':   () => dispatchTemplateCommand('set_tool', { tool: 'select' }),
              'editor.set_tool.move':     () => dispatchTemplateCommand('set_tool', { tool: 'move' }),
              'editor.set_tool.rotate':   () => dispatchTemplateCommand('set_tool', { tool: 'rotate' }),
              'editor.set_tool.scale':    () => dispatchTemplateCommand('set_tool', { tool: 'scale' }),
            };
            mapping[id]?.();
          });

          // editor-run-command-ai: routes MCP AI agent commands through the editor
          // dispatch layer and sends results back via ai_scene_response.
          unlistenAiCmd = await listen<{ id: string; args: unknown; request_id: string }>(
            'editor-run-command-ai',
            async (event) => {
              const { id, args, request_id } = event.payload;
              try {
                const result = await invoke<{ status: string; data?: unknown }>('run_command', { id, args: args ?? null });
                await invoke('ai_scene_response', { request_id, data: result?.data ?? null, error: null });
              } catch (e) {
                const errorMsg = e instanceof Error ? e.message : String(e);
                await invoke('ai_scene_response', { request_id, data: null, error: errorMsg });
              }
            }
          );
        }
      } catch (e) {
        console.error('[silmaril] Failed to listen for pop-out events:', e);
      }
    }
  });

  onDestroy(() => {
    unlistenCatalog?.();
    unlistenAiCmd?.();
    unlistenFullscreen?.();
    _unsubPanels?.();
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
        onUndo={undo}
        onRedo={redo}
        canUndo={canUndoState}
        canRedo={canRedoState}
        compactMenu={settings.compactMenu}
        bind:omnibarOpen
        projectPath={editorState?.project_path ?? null}
        {recentItems}
        onOmnibarOpen={() => { omnibarOpen = true; }}
        onOmnibarClose={() => { omnibarOpen = false; }}
        {panelContributions}
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
        <span class="breadcrumb-segment">{t('breadcrumb.no_template')}</span>
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
        onLayoutChange={handleLayoutChange}
      />
    </div>

    {#if hasAnyPanels(layout.bottomPanel)}
      <DockSplitter direction="vertical" onResize={onResizeBottom} />

      <div class="bottom-bar" style="height: {bottomHeight}px">
        <DockContainer
          node={layout.bottomPanel}
          {layout}
          onLayoutChange={handleLayoutChange}
          isBottomPanel={true}
        />
      </div>
    {/if}
  </div>

  <!-- Drag overlay (ghost tab + backdrop during panel drag) -->
  <DragOverlay />

  <!-- Invisible resize handles (frameless window — WebView2 blocks native NC hit-testing) -->
  <ResizeHandles />

  <!-- AI permission dialog — shown when an MCP agent requests a command permission -->
  <AiPermissionDialog />


  <!-- Status bar -->
  <div class="status-bar">
    <div class="status-left">
      <span class="status-item">{t('placeholder.select_entity')}</span>
    </div>
    <div class="status-center">
      <span class="status-item">{t('status.ready')}</span>
    </div>
    <div class="status-right">
      {#if $aiServerRunning}
        <button
          class="status-item status-mcp-badge"
          title="MCP server running — click to copy URL"
          onclick={() => navigator.clipboard.writeText(`http://localhost:${$aiServerPort}/mcp`)}
        >
          MCP :{$aiServerPort}
        </button>
        <span class="status-divider"></span>
      {/if}
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
    height: 42px;
    display: flex;
    align-items: center;
    padding: 0 16px;
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
    padding: 6px 10px;
  }
  .transport-btn:last-child {
    border-right: none;
  }
  .transport-play {
    padding: 6px 12px;
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
  .status-mcp-badge {
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    font-size: inherit;
    font-family: var(--font-mono, monospace);
    color: #4ade80;
  }
  .status-mcp-badge:hover {
    color: #86efac;
  }
</style>
