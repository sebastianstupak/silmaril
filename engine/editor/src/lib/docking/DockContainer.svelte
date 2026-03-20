<script lang="ts">
  import type { LayoutNode, DropZone } from './types';
  import { getBasePanelId } from './types';
  import type { Component } from 'svelte';
  import DockContainer from './DockContainer.svelte';
  import DockTabBar from './DockTabBar.svelte';
  import DockSplitter from './DockSplitter.svelte';
  import DockDropZone from './DockDropZone.svelte';
  import { dropPanel, resizeSplit, setActiveTab, removePanelFromLayout, endDrag, getDragState, subscribeDrag, registerTabCycle } from './store';
  import { popOutPanel, setViewportVisible } from '$lib/api';
  import type { EditorLayout } from './types';
  import { onMount } from 'svelte';

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

  // Subscribe to drag state changes instead of using HTML5 drag events
  onMount(() => {
    let wasDragging = false;
    const unsub = subscribeDrag(() => {
      isDragging = getDragState().active;
      wasDragging = isDragging;
    });
    return unsub;
  });

  function handleResize(index: number, deltaPx: number) {
    if (!containerEl) return;
    const size = node.type === 'split' && node.direction === 'horizontal'
      ? containerEl.clientWidth
      : containerEl.clientHeight;
    const newLayout = resizeSplit(layout, path, index, deltaPx, size, isBottomPanel);
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

  function handleDuplicate(panelId: string, newPanelId: string) {
    // Add the new panel instance as a tab next to the original
    const newLayout = dropPanel(layout, newPanelId, path, 'center', isBottomPanel);
    onLayoutChange(newLayout);
  }

  function handleCloseOthers(panelId: string) {
    if (node.type !== 'tabs') return;
    let newLayout = layout;
    for (const p of node.panels) {
      if (p !== panelId) {
        newLayout = removePanelFromLayout(newLayout, p);
      }
    }
    onLayoutChange(newLayout);
  }

  function handleCloseAll() {
    if (node.type !== 'tabs') return;
    let newLayout = layout;
    for (const p of node.panels) {
      newLayout = removePanelFromLayout(newLayout, p);
    }
    onLayoutChange(newLayout);
  }

  function handlePopOut(panelId: string) {
    const newLayout = removePanelFromLayout(layout, panelId);
    onLayoutChange(newLayout);
  }

  // When focus enters this tabs container (or any descendant), register it as
  // the target for Ctrl+Tab cycling. The callback closes over $props() signals
  // so it always reads the current node/layout at call time (Svelte 5 runes).
  function handleContainerFocus() {
    if (node.type !== 'tabs') return;
    registerTabCycle((dir: number) => {
      const count = node.panels.length;
      if (count <= 1) return;
      const next = ((node.activeTab + dir) + count) % count;
      onLayoutChange(setActiveTab(layout, path, next, isBottomPanel));
    });
  }

  // Pause Vulkan rendering for hidden viewport tabs.
  // Viewport panel IDs follow the pattern 'viewport', 'viewport:2', etc.
  // The Rust registry key equals the panel ID, so we can call setViewportVisible
  // directly from here without needing an isActive prop on ViewportPanel.
  $effect(() => {
    if (node.type !== 'tabs') return;
    node.panels.forEach((panelId, i) => {
      if (getBasePanelId(panelId) === 'viewport') {
        setViewportVisible(panelId, i === node.activeTab);
      }
    });
  });

  /** Resolve panel component, supporting instance IDs like 'viewport:2' */
  function resolveComponent(id: string): Component | undefined {
    return panelComponents[id] ?? panelComponents[getBasePanelId(id)];
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
          {isBottomPanel}
        />
      </div>
    {/each}
  </div>
{:else if node.type === 'tabs'}
  <div class="dock-tabs" bind:this={containerEl} onfocusin={handleContainerFocus} onmousedown={handleContainerFocus}>
    <DockTabBar
      panels={node.panels}
      activeTab={node.activeTab}
      onTabSelect={handleTabSelect}
      onDrop={handleTabDrop}
      onClose={handleTabClose}
      onDuplicate={handleDuplicate}
      onCloseOthers={handleCloseOthers}
      onCloseAll={handleCloseAll}
      onPopOut={handlePopOut}
    />
    <div class="dock-tab-content">
      <!-- All panels stay mounted to preserve state across tab switches.
           CSS hides inactive ones; the active panel gets display:flex.
           Viewport panels need setViewportVisible() to pause GPU rendering
           when hidden — handled by the $effect below. -->
      {#each node.panels as panelId, i (panelId)}
        {@const Comp = resolveComponent(panelId)}
        <div class="dock-panel-slot" class:active={i === node.activeTab}>
          {#if Comp}
            <Comp {panelId} />
          {:else}
            <div class="dock-panel-placeholder">
              <span>{panelId}</span>
            </div>
          {/if}
        </div>
      {/each}
      {#if node.panels.length === 0}
        <div class="dock-panel-empty"></div>
      {/if}

      <!-- Drop zone overlay for side splits -->
      <DockDropZone onDrop={handleZoneDrop} {path} {isBottomPanel} />
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
    overflow: auto;
    position: relative;
    display: flex;
    flex-direction: column;
    min-height: 0;
    /* Transparent so Vulkan viewport can show through.
       Non-viewport panels set their own opaque background. */
    background: transparent;
  }
  /* Each panel slot is hidden by default; only the active one is shown.
     Keeping all panels mounted preserves their state across tab switches. */
  .dock-panel-slot {
    display: none;
    flex: 1;
    flex-direction: column;
    overflow: hidden;
    min-height: 0;
    min-width: 0;
  }
  .dock-panel-slot.active {
    display: flex;
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
