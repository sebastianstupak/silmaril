<script lang="ts">
  import { onMount } from 'svelte';
  import { getEditorState, type EditorState } from './lib/api';
  import PanelShell from './lib/components/PanelShell.svelte';
  import ResizeHandle from './lib/components/ResizeHandle.svelte';
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
    leftWidth; rightWidth; bottomHeight; settings.theme; settings.fontSize;
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

  function setTheme(name: string) {
    settings.theme = name;
    applyTheme(themes[name]);
    saveSettings({ ...settings, theme: name });
  }

  onMount(async () => {
    applyTheme(themes[settings.theme] ?? themes.dark);
    document.documentElement.style.fontSize = `${settings.fontSize}px`;
    editorState = await getEditorState();
  });
</script>

<main class="editor-shell">
  <!-- Toolbar -->
  <div class="toolbar">
    <span class="title">Silmaril Editor</span>
    {#if editorState?.project_name}
      <span class="project-name">— {editorState.project_name}</span>
    {/if}
    <div class="toolbar-spacer"></div>
    <span class="mode-badge">{editorState?.mode ?? 'loading...'}</span>
    <button class="toolbar-btn" onclick={() => showSettings = !showSettings} title="Settings">
      <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
        <path d="M8 4.754a3.246 3.246 0 1 0 0 6.492 3.246 3.246 0 0 0 0-6.492zM5.754 8a2.246 2.246 0 1 1 4.492 0 2.246 2.246 0 0 1-4.492 0z"/>
        <path d="M9.796 1.343c-.527-1.79-3.065-1.79-3.592 0l-.094.319a.873.873 0 0 1-1.255.52l-.292-.16c-1.64-.892-3.433.902-2.54 2.541l.159.292a.873.873 0 0 1-.52 1.255l-.319.094c-1.79.527-1.79 3.065 0 3.592l.319.094a.873.873 0 0 1 .52 1.255l-.16.292c-.892 1.64.901 3.434 2.541 2.54l.292-.159a.873.873 0 0 1 1.255.52l.094.319c.527 1.79 3.065 1.79 3.592 0l.094-.319a.873.873 0 0 1 1.255-.52l.292.16c1.64.893 3.434-.902 2.54-2.541l-.159-.292a.873.873 0 0 1 .52-1.255l.319-.094c1.79-.527 1.79-3.065 0-3.592l-.319-.094a.873.873 0 0 1-.52-1.255l.16-.292c.893-1.64-.902-3.433-2.541-2.54l-.292.159a.873.873 0 0 1-1.255-.52l-.094-.319zm-2.633.283c.246-.835 1.428-.835 1.674 0l.094.319a1.873 1.873 0 0 0 2.693 1.115l.291-.16c.764-.415 1.6.42 1.184 1.185l-.159.292a1.873 1.873 0 0 0 1.116 2.692l.318.094c.835.246.835 1.428 0 1.674l-.319.094a1.873 1.873 0 0 0-1.115 2.693l.16.291c.415.764-.421 1.6-1.185 1.184l-.291-.159a1.873 1.873 0 0 0-2.693 1.116l-.094.318c-.246.835-1.428.835-1.674 0l-.094-.319a1.873 1.873 0 0 0-2.692-1.115l-.292.16c-.764.415-1.6-.421-1.184-1.185l.159-.291A1.873 1.873 0 0 0 1.945 8.93l-.319-.094c-.835-.246-.835-1.428 0-1.674l.319-.094A1.873 1.873 0 0 0 3.06 4.377l-.16-.292c-.415-.764.42-1.6 1.185-1.184l.292.159a1.873 1.873 0 0 0 2.692-1.116l.094-.318z"/>
      </svg>
    </button>
  </div>

  <!-- Settings dropdown -->
  {#if showSettings}
    <div class="settings-dropdown">
      <div class="settings-row">
        <label>Theme</label>
        <select value={settings.theme} onchange={(e) => setTheme((e.target as HTMLSelectElement).value)}>
          <option value="dark">Dark</option>
          <option value="light">Light</option>
        </select>
      </div>
      <div class="settings-row">
        <label>Font Size</label>
        <input
          type="range"
          min="10"
          max="18"
          value={settings.fontSize}
          oninput={(e) => {
            settings.fontSize = parseInt((e.target as HTMLInputElement).value);
            document.documentElement.style.fontSize = `${settings.fontSize}px`;
            saveSettings(settings);
          }}
        />
        <span>{settings.fontSize}px</span>
      </div>
    </div>
  {/if}

  <!-- Main area: left | resize | viewport | resize | right -->
  <div class="content-area">
    <div class="main-area">
      <div class="sidebar-left" style="width: {leftWidth}px">
        <PanelShell title="Hierarchy">
          <p class="placeholder">No project loaded</p>
        </PanelShell>
      </div>

      <ResizeHandle direction="horizontal" onResize={onResizeLeft} />

      <div class="viewport">
        <PanelShell title="Viewport">
          <div class="viewport-placeholder">
            <p>Vulkan viewport will render here</p>
          </div>
        </PanelShell>
      </div>

      <ResizeHandle direction="horizontal" onResize={onResizeRight} />

      <div class="sidebar-right" style="width: {rightWidth}px">
        <PanelShell title="Inspector">
          <p class="placeholder">Select an entity</p>
        </PanelShell>
      </div>
    </div>

    <ResizeHandle direction="vertical" onResize={onResizeBottom} />

    <!-- Bottom panel -->
    <div class="bottom-bar" style="height: {bottomHeight}px">
      <PanelShell title="Console">
        <p class="placeholder">No logs yet</p>
      </PanelShell>
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
    height: 40px;
    display: flex;
    align-items: center;
    padding: 0 12px;
    background: var(--color-bgHeader, #2d2d2d);
    border-bottom: 1px solid var(--color-border, #404040);
    gap: 8px;
    flex-shrink: 0;
  }
  .title { font-weight: 600; font-size: 14px; }
  .project-name { color: var(--color-textMuted, #999); font-size: 13px; }
  .toolbar-spacer { flex: 1; }
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

  /* Settings dropdown */
  .settings-dropdown {
    position: absolute;
    top: 40px;
    right: 12px;
    background: var(--color-bgPanel, #252525);
    border: 1px solid var(--color-border, #404040);
    border-radius: 6px;
    padding: 12px;
    z-index: 100;
    min-width: 220px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
  }
  .settings-row {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 8px;
  }
  .settings-row:last-child { margin-bottom: 0; }
  .settings-row label {
    font-size: 12px;
    color: var(--color-textMuted, #999);
    min-width: 70px;
  }
  .settings-row select, .settings-row input[type="range"] {
    flex: 1;
    background: var(--color-bgInput, #333);
    color: var(--color-text, #ccc);
    border: 1px solid var(--color-border, #404040);
    border-radius: 3px;
    padding: 2px 6px;
    font-size: 12px;
  }
  .settings-row span {
    font-size: 11px;
    color: var(--color-textDim, #666);
    min-width: 32px;
    text-align: right;
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
</style>
