<script lang="ts">
  import { onMount } from 'svelte';
  import { getEditorState, type EditorState } from './lib/api';
  import { t } from './lib/i18n';
  import { setLocale } from './lib/i18n';
  import PanelShell from './lib/components/PanelShell.svelte';
  import ResizeHandle from './lib/components/ResizeHandle.svelte';
  import MenuBar from './lib/components/MenuBar.svelte';
  import SettingsDialog from './lib/components/SettingsDialog.svelte';
  import { themes, applyTheme } from './lib/theme/tokens';
  import { loadSettings, saveSettings, type EditorSettings } from './lib/stores/settings';

  let editorState: EditorState | null = $state(null);
  let settings: EditorSettings = $state(loadSettings());
  let showSettings = $state(false);

  // Panel sizes (reactive, persisted)
  let leftWidth = $state(settings.leftPanelWidth);
  let rightWidth = $state(settings.rightPanelWidth);
  let bottomHeight = $state(settings.bottomPanelHeight);

  const MIN_PANEL = 120;
  const MIN_VIEWPORT = 200;

  function clampLeft(w: number) { return Math.max(MIN_PANEL, Math.min(w, window.innerWidth - rightWidth - MIN_VIEWPORT)); }
  function clampRight(w: number) { return Math.max(MIN_PANEL, Math.min(w, window.innerWidth - leftWidth - MIN_VIEWPORT)); }
  function clampBottom(h: number) { return Math.max(MIN_PANEL, Math.min(h, window.innerHeight - 240)); }

  function onResizeLeft(delta: number) { leftWidth = clampLeft(leftWidth + delta); }
  function onResizeRight(delta: number) { rightWidth = clampRight(rightWidth - delta); }
  function onResizeBottom(delta: number) { bottomHeight = clampBottom(bottomHeight - delta); }

  // Persist on change (debounced)
  let saveTimer: ReturnType<typeof setTimeout> | null = null;
  $effect(() => {
    // Track reactive values
    leftWidth; rightWidth; bottomHeight; settings.theme; settings.fontSize; settings.language; settings.autoSave;
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(() => {
      saveSettings({
        ...settings,
        leftPanelWidth: leftWidth,
        rightPanelWidth: rightWidth,
        bottomPanelHeight: bottomHeight,
      });
    }, 300);
  });

  function handleSettingsChange(updated: EditorSettings) {
    settings = updated;
  }

  onMount(async () => {
    setLocale(settings.language);
    applyTheme(themes[settings.theme] ?? themes.dark);
    document.documentElement.style.fontSize = `${settings.fontSize}px`;
    editorState = await getEditorState();
  });
</script>

<main class="editor-shell">
  <!-- Menu Bar -->
  <MenuBar onSettingsOpen={() => showSettings = true} />

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

    <!-- Right: mode + settings -->
    <div class="toolbar-right">
      <span class="mode-badge">{editorState?.mode ?? '...'}</span>
      <button class="toolbar-btn" onclick={() => showSettings = true} title={t('settings.title')}>
        <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
          <path d="M8 4.754a3.246 3.246 0 1 0 0 6.492 3.246 3.246 0 0 0 0-6.492zM5.754 8a2.246 2.246 0 1 1 4.492 0 2.246 2.246 0 0 1-4.492 0z"/>
          <path d="M9.796 1.343c-.527-1.79-3.065-1.79-3.592 0l-.094.319a.873.873 0 0 1-1.255.52l-.292-.16c-1.64-.892-3.433.902-2.54 2.541l.159.292a.873.873 0 0 1-.52 1.255l-.319.094c-1.79.527-1.79 3.065 0 3.592l.319.094a.873.873 0 0 1 .52 1.255l-.16.292c-.892 1.64.901 3.434 2.541 2.54l.292-.159a.873.873 0 0 1 1.255.52l.094.319c.527 1.79 3.065 1.79 3.592 0l.094-.319a.873.873 0 0 1 1.255-.52l.292.16c1.64.893 3.434-.902 2.54-2.541l-.159-.292a.873.873 0 0 1 .52-1.255l.319-.094c1.79-.527 1.79-3.065 0-3.592l-.319-.094a.873.873 0 0 1-.52-1.255l.16-.292c.893-1.64-.902-3.433-2.541-2.54l-.292.159a.873.873 0 0 1-1.255-.52l-.094-.319zm-2.633.283c.246-.835 1.428-.835 1.674 0l.094.319a1.873 1.873 0 0 0 2.693 1.115l.291-.16c.764-.415 1.6.42 1.184 1.185l-.159.292a1.873 1.873 0 0 0 1.116 2.692l.318.094c.835.246.835 1.428 0 1.674l-.319.094a1.873 1.873 0 0 0-1.115 2.693l.16.291c.415.764-.421 1.6-1.185 1.184l-.291-.159a1.873 1.873 0 0 0-2.693 1.116l-.094.318c-.246.835-1.428.835-1.674 0l-.094-.319a1.873 1.873 0 0 0-2.692-1.115l-.292.16c-.764.415-1.6-.421-1.184-1.185l.159-.291A1.873 1.873 0 0 0 1.945 8.93l-.319-.094c-.835-.246-.835-1.428 0-1.674l.319-.094A1.873 1.873 0 0 0 3.06 4.377l-.16-.292c-.415-.764.42-1.6 1.185-1.184l.292.159a1.873 1.873 0 0 0 2.692-1.116l.094-.318z"/>
        </svg>
      </button>
    </div>
  </div>

  <!-- Settings Dialog -->
  <SettingsDialog
    bind:open={showSettings}
    {settings}
    onSettingsChange={handleSettingsChange}
  />

  <!-- Main area: left | resize | viewport | resize | right -->
  <div class="content-area">
    <div class="main-area">
      <div class="sidebar-left" style="width: {leftWidth}px">
        <PanelShell title={t('panel.hierarchy')}>
          <p class="placeholder">{t('placeholder.no_project')}</p>
        </PanelShell>
      </div>

      <ResizeHandle direction="horizontal" onResize={onResizeLeft} />

      <div class="viewport">
        <PanelShell title={t('panel.viewport')}>
          <div class="viewport-placeholder">
            <p>{t('placeholder.viewport')}</p>
          </div>
        </PanelShell>
      </div>

      <ResizeHandle direction="horizontal" onResize={onResizeRight} />

      <div class="sidebar-right" style="width: {rightWidth}px">
        <PanelShell title={t('panel.inspector')}>
          <p class="placeholder">{t('placeholder.select_entity')}</p>
        </PanelShell>
      </div>
    </div>

    <ResizeHandle direction="vertical" onResize={onResizeBottom} />

    <!-- Bottom panel -->
    <div class="bottom-bar" style="height: {bottomHeight}px">
      <PanelShell title={t('panel.console')}>
        <p class="placeholder">{t('placeholder.no_logs')}</p>
      </PanelShell>
    </div>
  </div>

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

<style>
  .editor-shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--color-bg, #1e1e1e);
    color: var(--color-text, #cccccc);
    font-family: var(--font-body, system-ui, -apple-system, sans-serif);
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
  .sidebar-left, .sidebar-right {
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .viewport {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 200px;
    overflow: hidden;
  }
  .bottom-bar {
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .viewport-placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--color-textDim, #666);
  }
  .placeholder { color: var(--color-textDim, #666); font-style: italic; padding: 8px; }

  .sidebar-left :global(.panel),
  .sidebar-right :global(.panel),
  .viewport :global(.panel),
  .bottom-bar :global(.panel) {
    flex: 1;
  }

  /* Status bar */
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
