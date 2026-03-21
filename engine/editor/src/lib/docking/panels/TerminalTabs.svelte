<!-- engine/editor/src/lib/docking/panels/TerminalTabs.svelte -->
<script lang="ts">
  import { t } from '$lib/i18n';
  import type { TerminalTab } from '$lib/stores/terminal';

  let {
    tabs,
    activeTabId,
    onNewTab,
    onCloseTab,
    onSelectTab,
  }: {
    tabs: TerminalTab[];
    activeTabId: string | null;
    onNewTab: () => void;
    onCloseTab: (id: string) => void;
    onSelectTab: (id: string) => void;
  } = $props();

  /** Last non-exited tab cannot be closed */
  function canClose(tab: TerminalTab): boolean {
    if (tab.exited) return true;
    const liveCount = tabs.filter(t => !t.exited).length;
    return liveCount > 1;
  }
</script>

<div class="tab-bar" role="tablist">
  {#each tabs as tab (tab.id)}
    <button
      class="tab"
      class:active={tab.id === activeTabId}
      class:exited={tab.exited}
      role="tab"
      aria-selected={tab.id === activeTabId}
      onclick={() => onSelectTab(tab.id)}
    >
      <span class="tab-label">{tab.title}</span>
      {#if canClose(tab)}
        <span
          class="tab-close"
          role="button"
          aria-label={t('terminal.close_tab')}
          tabindex="0"
          onclick={e => { e.stopPropagation(); onCloseTab(tab.id); }}
          onkeydown={e => { if (e.key === 'Enter' || e.key === ' ') { e.stopPropagation(); onCloseTab(tab.id); } }}
        >×</span>
      {/if}
    </button>
  {/each}
  <button class="tab-new" aria-label={t('terminal.new_tab')} onclick={onNewTab}>+</button>
</div>

<style>
  .tab-bar {
    display: flex;
    align-items: stretch;
    background: var(--color-bgPanel, #1e1e1e);
    border-bottom: 1px solid var(--color-border, #333);
    overflow-x: auto;
    height: 32px;
    flex-shrink: 0;
  }
  .tab {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 0 10px;
    font-size: 12px;
    color: var(--color-text, #ccc);
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    cursor: pointer;
    white-space: nowrap;
    flex-shrink: 0;
  }
  .tab:hover { background: var(--color-bgHover, #2a2a2a); }
  .tab.active { border-bottom-color: var(--color-accent, #569cd6); color: #fff; }
  .tab.exited { opacity: 0.5; }
  .tab-close {
    opacity: 0.6;
    font-size: 14px;
    line-height: 1;
    padding: 0 2px;
    border-radius: 2px;
    cursor: pointer;
  }
  .tab-close:hover { opacity: 1; background: var(--color-bgHover, #2a2a2a); }
  .tab-new {
    padding: 0 10px;
    font-size: 16px;
    color: var(--color-text, #ccc);
    background: transparent;
    border: none;
    cursor: pointer;
    align-self: center;
  }
  .tab-new:hover { color: #fff; }
</style>
