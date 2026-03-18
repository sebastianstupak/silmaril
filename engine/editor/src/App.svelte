<script lang="ts">
  import { onMount } from 'svelte';
  import { getEditorState, type EditorState } from './lib/api';
  import PanelShell from './lib/components/PanelShell.svelte';

  let editorState: EditorState | null = $state(null);

  onMount(async () => {
    editorState = await getEditorState();
  });
</script>

<main class="editor-shell">
  <div class="toolbar">
    <span class="title">Silmaril Editor</span>
    {#if editorState?.project_name}
      <span class="project-name">— {editorState.project_name}</span>
    {/if}
    <div class="toolbar-spacer"></div>
    <span class="mode-badge">{editorState?.mode ?? 'loading...'}</span>
  </div>

  <div class="main-area">
    <div class="sidebar-left">
      <PanelShell title="Hierarchy">
        <p class="placeholder">No project loaded</p>
      </PanelShell>
    </div>

    <div class="viewport">
      <PanelShell title="Viewport">
        <div class="viewport-placeholder">
          <p>Vulkan viewport will render here</p>
        </div>
      </PanelShell>
    </div>

    <div class="sidebar-right">
      <PanelShell title="Inspector">
        <p class="placeholder">Select an entity</p>
      </PanelShell>
    </div>
  </div>

  <div class="bottom-bar">
    <PanelShell title="Console">
      <p class="placeholder">No logs yet</p>
    </PanelShell>
  </div>
</main>

<style>
  .editor-shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: #1e1e1e;
    color: #cccccc;
    font-family: system-ui, -apple-system, sans-serif;
    font-size: 13px;
  }
  .toolbar {
    height: 40px;
    display: flex;
    align-items: center;
    padding: 0 16px;
    background: #2d2d2d;
    border-bottom: 1px solid #404040;
    gap: 8px;
  }
  .title { font-weight: 600; font-size: 14px; }
  .project-name { color: #999; font-size: 13px; }
  .toolbar-spacer { flex: 1; }
  .mode-badge {
    padding: 2px 8px;
    background: #007acc;
    border-radius: 3px;
    font-size: 11px;
    text-transform: uppercase;
    font-weight: 600;
  }
  .main-area {
    flex: 1;
    display: flex;
    overflow: hidden;
  }
  .sidebar-left { width: 250px; border-right: 1px solid #404040; }
  .sidebar-right { width: 300px; border-left: 1px solid #404040; }
  .viewport { flex: 1; }
  .bottom-bar { height: 200px; border-top: 1px solid #404040; }
  .viewport-placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: #666;
  }
  .placeholder { color: #666; font-style: italic; padding: 8px; }

  .sidebar-left, .sidebar-right, .viewport, .bottom-bar {
    display: flex;
    flex-direction: column;
  }
  .sidebar-left :global(.panel),
  .sidebar-right :global(.panel),
  .viewport :global(.panel),
  .bottom-bar :global(.panel) {
    flex: 1;
  }
</style>
