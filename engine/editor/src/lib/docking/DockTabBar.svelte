<script lang="ts">
  import { t } from '$lib/i18n';
  import { getPanelInfo } from './types';
  import { getDragState, startDrag, endDrag } from './store';

  interface Props {
    panels: string[];
    activeTab: number;
    onTabSelect: (index: number) => void;
    onDrop: (panelId: string, zone: 'center') => void;
    onClose: (panelId: string) => void;
  }

  let { panels, activeTab, onTabSelect, onDrop, onClose }: Props = $props();

  let dragOver = $state(false);

  function handleDragStart(e: DragEvent, panelId: string) {
    if (!e.dataTransfer) return;
    e.dataTransfer.setData('text/plain', panelId);
    e.dataTransfer.effectAllowed = 'move';
    startDrag(panelId);
  }

  function handleDragEnd() {
    endDrag();
  }

  function handleDragOver(e: DragEvent) {
    e.preventDefault();
    if (e.dataTransfer) e.dataTransfer.dropEffect = 'move';
    dragOver = true;
  }

  function handleDragLeave() {
    dragOver = false;
  }

  function handleDropOnBar(e: DragEvent) {
    e.preventDefault();
    dragOver = false;
    const panelId = e.dataTransfer?.getData('text/plain');
    if (panelId) {
      onDrop(panelId, 'center');
    }
    endDrag();
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="dock-tab-bar"
  class:drag-over={dragOver}
  ondragover={handleDragOver}
  ondragleave={handleDragLeave}
  ondrop={handleDropOnBar}
>
  {#each panels as panelId, i}
    {@const info = getPanelInfo(panelId)}
    <div
      class="dock-tab"
      class:active={i === activeTab}
      draggable="true"
      role="tab"
      tabindex="0"
      aria-selected={i === activeTab}
      ondragstart={(e) => handleDragStart(e, panelId)}
      ondragend={handleDragEnd}
      onclick={() => onTabSelect(i)}
      onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') onTabSelect(i); }}
    >
      <span class="dock-tab-label">{info ? t(info.titleKey) : panelId}</span>
      {#if panels.length > 1}
        <button
          class="dock-tab-close"
          title={t('dock.close_tab')}
          onclick={(e: MouseEvent) => { e.stopPropagation(); onClose(panelId); }}
        >
          <svg width="10" height="10" viewBox="0 0 16 16" fill="currentColor">
            <path d="M4.11 3.05L8 6.94l3.89-3.89a.75.75 0 111.06 1.06L9.06 8l3.89 3.89a.75.75 0 11-1.06 1.06L8 9.06l-3.89 3.89a.75.75 0 01-1.06-1.06L6.94 8 3.05 4.11a.75.75 0 011.06-1.06z"/>
          </svg>
        </button>
      {/if}
    </div>
  {/each}
</div>

<style>
  .dock-tab-bar {
    display: flex;
    align-items: stretch;
    background: var(--color-bgHeader, #2d2d2d);
    border-bottom: 1px solid var(--color-border, #404040);
    flex-shrink: 0;
    min-height: 32px;
    overflow-x: auto;
    overflow-y: hidden;
  }
  .dock-tab-bar.drag-over {
    background: var(--color-bgPanel, #252525);
    outline: 1px dashed var(--color-accent, #007acc);
    outline-offset: -1px;
  }
  .dock-tab {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 0 12px;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--color-textMuted, #999);
    cursor: pointer;
    user-select: none;
    border-right: 1px solid var(--color-border, #404040);
    white-space: nowrap;
    transition: color 0.1s, background 0.1s;
  }
  .dock-tab:hover {
    color: var(--color-text, #ccc);
    background: var(--color-bgPanel, #252525);
  }
  .dock-tab.active {
    color: var(--color-text, #ccc);
    background: var(--color-bgPanel, #252525);
    border-bottom: 2px solid var(--color-accent, #007acc);
  }
  .dock-tab-label {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .dock-tab-close {
    background: none;
    border: none;
    color: var(--color-textDim, #666);
    cursor: pointer;
    padding: 2px;
    border-radius: 3px;
    display: flex;
    align-items: center;
    justify-content: center;
    opacity: 0;
    transition: opacity 0.1s;
  }
  .dock-tab:hover .dock-tab-close {
    opacity: 1;
  }
  .dock-tab-close:hover {
    color: var(--color-text, #ccc);
    background: var(--color-bg, #1e1e1e);
  }
</style>
