<script lang="ts">
  import type { LayoutNode, DropZone } from './types';
  import type { Component } from 'svelte';
  import DockContainer from './DockContainer.svelte';
  import DockTabBar from './DockTabBar.svelte';
  import DockSplitter from './DockSplitter.svelte';
  import DockDropZone from './DockDropZone.svelte';
  import { dropPanel, resizeSplit, setActiveTab, removePanelFromLayout, endDrag, getDragState } from './store';
  import type { EditorLayout } from './types';

  interface Props {
    node: LayoutNode;
    layout: EditorLayout;
    path?: number[];
    panelComponents: Record<string, Component>;
    onLayoutChange: (layout: EditorLayout) => void;
    isBottomPanel?: boolean;
  }

  let {
    node,
    layout,
    path = [],
    panelComponents,
    onLayoutChange,
    isBottomPanel = false,
  }: Props = $props();

  let containerEl: HTMLDivElement | undefined = $state(undefined);
  let isDragging = $state(false);

  $effect(() => {
    function onDragStart() { isDragging = true; }
    function onDragEnd() { isDragging = false; }

    window.addEventListener('dragstart', onDragStart);
    window.addEventListener('dragend', onDragEnd);
    // Also listen for drop to clear state in case dragend doesn't fire
    window.addEventListener('drop', onDragEnd);

    return () => {
      window.removeEventListener('dragstart', onDragStart);
      window.removeEventListener('dragend', onDragEnd);
      window.removeEventListener('drop', onDragEnd);
    };
  });

  function handleResize(index: number, deltaPx: number) {
    if (!containerEl) return;
    const size = node.type === 'split' && node.direction === 'horizontal'
      ? containerEl.clientWidth
      : containerEl.clientHeight;
    const newLayout = resizeSplit(layout, path, index, deltaPx, size);
    onLayoutChange(newLayout);
  }

  function handleTabSelect(index: number) {
    const newLayout = setActiveTab(layout, path, index, isBottomPanel);
    onLayoutChange(newLayout);
  }

  function handleTabDrop(panelId: string, _zone: 'center') {
    const newLayout = dropPanel(layout, panelId, path, 'center', isBottomPanel);
    onLayoutChange(newLayout);
    endDrag();
  }

  function handleZoneDrop(zone: DropZone) {
    const state = getDragState();
    if (!state.active) return;
    const newLayout = dropPanel(layout, state.panelId, path, zone, isBottomPanel);
    onLayoutChange(newLayout);
    endDrag();
  }

  function handleTabClose(panelId: string) {
    const newLayout = removePanelFromLayout(layout, panelId);
    onLayoutChange(newLayout);
  }
</script>

{#if node.type === 'split'}
  <div
    class="dock-split"
    class:horizontal={node.direction === 'horizontal'}
    class:vertical={node.direction === 'vertical'}
    bind:this={containerEl}
  >
    {#each node.children as child, i}
      {#if i > 0}
        <DockSplitter
          direction={node.direction}
          onResize={(delta) => handleResize(i, delta)}
        />
      {/if}
      <div class="dock-child" style="flex: {node.sizes[i]} 0 0%">
        <DockContainer
          node={child}
          {layout}
          path={[...path, i]}
          {panelComponents}
          {onLayoutChange}
        />
      </div>
    {/each}
  </div>
{:else if node.type === 'tabs'}
  <div class="dock-tabs" bind:this={containerEl}>
    <DockTabBar
      panels={node.panels}
      activeTab={node.activeTab}
      onTabSelect={handleTabSelect}
      onDrop={handleTabDrop}
      onClose={handleTabClose}
    />
    <div class="dock-tab-content">
      {#if node.panels[node.activeTab]}
        {@const Comp = panelComponents[node.panels[node.activeTab]]}
        {#if Comp}
          <Comp />
        {:else}
          <div class="dock-panel-placeholder">
            <span>{node.panels[node.activeTab]}</span>
          </div>
        {/if}
      {:else}
        <div class="dock-panel-empty"></div>
      {/if}

      <!-- Drop zone overlay for side splits -->
      <DockDropZone {isDragging} onDrop={handleZoneDrop} />
    </div>
  </div>
{/if}

<style>
  .dock-split {
    display: flex;
    flex: 1;
    overflow: hidden;
  }
  .dock-split.horizontal {
    flex-direction: row;
  }
  .dock-split.vertical {
    flex-direction: column;
  }
  .dock-child {
    display: flex;
    overflow: hidden;
    min-width: 0;
    min-height: 0;
  }
  .dock-tabs {
    display: flex;
    flex-direction: column;
    flex: 1;
    overflow: hidden;
    min-width: 0;
    min-height: 0;
  }
  .dock-tab-content {
    flex: 1;
    overflow: hidden;
    position: relative;
    display: flex;
    flex-direction: column;
    background: var(--color-bgPanel, #252525);
  }
  .dock-panel-placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--color-textDim, #666);
    font-style: italic;
  }
  .dock-panel-empty {
    flex: 1;
  }
</style>
