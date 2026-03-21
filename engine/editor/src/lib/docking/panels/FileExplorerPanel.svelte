<!-- engine/editor/src/lib/docking/panels/FileExplorerPanel.svelte -->
<script lang="ts">
  import type { FileExplorerState } from '$lib/stores/file-explorer';
  import { toggleShowIgnored, refreshTree } from '$lib/stores/file-explorer';
  import FileTreeNode from '$lib/components/FileTreeNode.svelte';
  import { t } from '$lib/i18n';

  let { state }: { state: FileExplorerState } = $props();

  let visibleRoots = $derived(
    state.nodes.filter((n) => state.showIgnored || !n.ignored)
  );
</script>

<div class="file-explorer">
  <!-- Header -->
  <div class="panel-header">
    <span class="panel-title">{t('panel.file_explorer')}</span>
    <div class="header-actions">
      <button
        class="icon-btn"
        title={t('explorer.show_ignored')}
        class:active={state.showIgnored}
        onclick={toggleShowIgnored}
        aria-label={t('explorer.show_ignored')}
        aria-pressed={state.showIgnored}
      >
        👁
      </button>
      <button
        class="icon-btn"
        title={t('explorer.refresh')}
        onclick={refreshTree}
        aria-label={t('explorer.refresh')}
        disabled={state.loading}
      >
        ↻
      </button>
    </div>
  </div>

  <!-- Error bar -->
  {#if state.error}
    <div class="error-bar" role="alert">
      {t('explorer.error')}: {state.error}
    </div>
  {/if}

  <!-- Tree -->
  <!-- Store always creates new state objects (spread assign), so $derived recomputes correctly on reference change -->
  <div class="tree-scroll" role="tree" aria-label={t('panel.file_explorer')} aria-busy={state.loading}>
    {#if state.loading}
      <div class="status-msg">{t('explorer.loading')}</div>
    {:else if !state.root}
      <div class="status-msg">{t('placeholder.no_project')}</div>
    {:else if visibleRoots.length === 0}
      <div class="status-msg">{t('explorer.empty')}</div>
    {:else}
      {#each visibleRoots as node (node.path)}
        <FileTreeNode
          {node}
          depth={0}
          showIgnored={state.showIgnored}
          selected={state.selected}
          gitStatus={state.gitStatus}
          expanded={state.expanded}
        />
      {/each}
    {/if}
  </div>
</div>

<style>
  .file-explorer {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
    background: var(--color-bgPanel, #1e1e1e);
  }
  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 4px 8px;
    border-bottom: 1px solid var(--color-border, #333);
    flex-shrink: 0;
  }
  .panel-title {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--color-textMuted, #888);
  }
  .header-actions { display: flex; gap: 4px; }
  .icon-btn {
    background: none;
    border: none;
    cursor: pointer;
    padding: 2px 4px;
    border-radius: 3px;
    color: var(--color-textMuted, #888);
    font-size: 14px;
    line-height: 1;
  }
  .icon-btn:hover { background: var(--color-bgHover, #2a2a2a); color: var(--color-text, #ccc); }
  .icon-btn.active { color: var(--color-accent, #4a9eff); }
  .icon-btn:disabled { opacity: 0.4; cursor: not-allowed; }
  .error-bar {
    background: var(--color-error, #c0392b);
    color: #fff;
    font-size: 12px;
    padding: 4px 8px;
    flex-shrink: 0;
  }
  .tree-scroll {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
    padding: 4px 0;
  }
  .status-msg {
    padding: 8px;
    color: var(--color-textMuted, #888);
    font-size: 12px;
  }
</style>
