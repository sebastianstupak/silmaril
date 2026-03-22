<script lang="ts">
  import { t } from '$lib/i18n';
  import { getBasePanelId, createPanelInstance } from './types';
  import { getPanelTitle } from '$lib/contributions/registry';
  import { startDrag, updateDrag, endDrag, getDragState } from './store';
  import { popOutPanel } from '$lib/api';

  interface Props {
    panels: string[];
    activeTab: number;
    onTabSelect: (index: number) => void;
    onDrop: (panelId: string, zone: 'center') => void;
    onClose: (panelId: string) => void;
    onDuplicate?: (panelId: string, newPanelId: string) => void;
    onCloseOthers?: (panelId: string) => void;
    onCloseAll?: () => void;
    onPopOut?: (panelId: string) => void;
  }

  let {
    panels,
    activeTab,
    onTabSelect,
    onDrop,
    onClose,
    onDuplicate,
    onCloseOthers,
    onCloseAll,
    onPopOut,
  }: Props = $props();

  let contextMenu = $state<{ x: number; y: number; panelId: string } | null>(null);

  // ----------- Mouse-based drag (replaces HTML5 drag) -----------

  function handleTabMouseDown(e: MouseEvent, panelId: string) {
    if (e.button !== 0) return; // left button only
    // Don't start drag from close button
    if ((e.target as HTMLElement).closest('.dock-tab-close')) return;

    const startX = e.clientX;
    const startY = e.clientY;
    let dragging = false;

    function onMouseMove(ev: MouseEvent) {
      const dx = ev.clientX - startX;
      const dy = ev.clientY - startY;

      // Start drag after 5px threshold
      if (!dragging && Math.abs(dx) + Math.abs(dy) > 5) {
        dragging = true;
        startDrag(panelId, ev.clientX, ev.clientY);
      }

      if (dragging) {
        updateDrag(ev.clientX, ev.clientY);
      }
    }

    function onMouseUp(ev: MouseEvent) {
      window.removeEventListener('mousemove', onMouseMove);
      window.removeEventListener('mouseup', onMouseUp);

      if (dragging) {
        // Check if the mouse is outside the main editor area — pop out
        const editorBounds = document.querySelector('.editor-shell')?.getBoundingClientRect();
        if (editorBounds && (
          ev.clientX < editorBounds.left || ev.clientX > editorBounds.right ||
          ev.clientY < editorBounds.top || ev.clientY > editorBounds.bottom
        )) {
          popOutPanel(panelId, getPanelTitle(panelId), ev.screenX, ev.screenY);
          if (onPopOut) {
            onPopOut(panelId);
          }
          endDrag();
          return;
        }

        // Normal drop — the DockDropZone's mouseup handler will handle
        // zone-based drops. Fire endDrag to let listeners know the drag
        // completed at this position.
        endDrag();
      }
    }

    window.addEventListener('mousemove', onMouseMove);
    window.addEventListener('mouseup', onMouseUp);
  }

  // ----------- Context menu -----------

  function handleContextMenu(e: MouseEvent, panelId: string) {
    e.preventDefault();
    contextMenu = { x: e.clientX, y: e.clientY, panelId };

    // Close on next click anywhere
    function closeMenu() {
      contextMenu = null;
      window.removeEventListener('mousedown', closeMenu);
    }
    // Use setTimeout so the current event doesn't immediately close it
    setTimeout(() => {
      window.addEventListener('mousedown', closeMenu);
    }, 0);
  }

  function handleCtxClose() {
    if (!contextMenu) return;
    onClose(contextMenu.panelId);
    contextMenu = null;
  }

  function handleCtxCloseOthers() {
    if (!contextMenu) return;
    if (onCloseOthers) {
      onCloseOthers(contextMenu.panelId);
    }
    contextMenu = null;
  }

  function handleCtxCloseAll() {
    if (!contextMenu) return;
    if (onCloseAll) {
      onCloseAll();
    }
    contextMenu = null;
  }

  function handleCtxDuplicate() {
    if (!contextMenu) return;
    if (onDuplicate) {
      const baseId = getBasePanelId(contextMenu.panelId);
      const newId = createPanelInstance(baseId);
      onDuplicate(contextMenu.panelId, newId);
    }
    contextMenu = null;
  }

  function handleCtxPopOut() {
    if (!contextMenu) return;
    const pid = contextMenu.panelId;
    // Position the pop-out window near the context menu click
    popOutPanel(pid, getPanelTitle(pid), contextMenu.x + window.screenX, contextMenu.y + window.screenY);
    if (onPopOut) {
      onPopOut(pid);
    }
    contextMenu = null;
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="dock-tab-bar">
  {#each panels as panelId, i}
    <div
      class="dock-tab"
      class:active={i === activeTab}
      role="tab"
      tabindex="0"
      aria-selected={i === activeTab}
      onmousedown={(e) => handleTabMouseDown(e, panelId)}
      onclick={() => onTabSelect(i)}
      oncontextmenu={(e) => handleContextMenu(e, panelId)}
      onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') onTabSelect(i); }}
    >
      <span class="dock-tab-label">{getPanelTitle(panelId)}</span>
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

<!-- Context menu -->
{#if contextMenu}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="context-menu"
    style="left: {contextMenu.x}px; top: {contextMenu.y}px;"
    onmousedown={(e: MouseEvent) => e.stopPropagation()}
  >
    <button class="context-item" onclick={handleCtxClose}>
      {t('dock.close_tab')}
    </button>
    <button class="context-item" onclick={handleCtxCloseOthers} disabled={panels.length <= 1}>
      {t('dock.close_others')}
    </button>
    <button class="context-item" onclick={handleCtxCloseAll}>
      {t('dock.close_all')}
    </button>
    <div class="context-separator"></div>
    <button class="context-item" onclick={handleCtxDuplicate}>
      {t('dock.duplicate')}
    </button>
    <div class="context-separator"></div>
    <button class="context-item" onclick={handleCtxPopOut}>
      {t('dock.pop_out')}
    </button>
  </div>
{/if}

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

  /* Context menu */
  .context-menu {
    position: fixed;
    z-index: 10001;
    min-width: 160px;
    background: var(--color-bgHeader, #2d2d2d);
    border: 1px solid var(--color-border, #404040);
    border-radius: 4px;
    box-shadow: 0 4px 12px rgba(0,0,0,0.5);
    padding: 4px 0;
    display: flex;
    flex-direction: column;
  }
  .context-item {
    background: none;
    border: none;
    color: var(--color-text, #ccc);
    font-size: 12px;
    padding: 6px 16px;
    text-align: left;
    cursor: pointer;
    white-space: nowrap;
  }
  .context-item:hover:not(:disabled) {
    background: var(--color-accent, #007acc);
    color: white;
  }
  .context-item:disabled {
    color: var(--color-textDim, #666);
    cursor: default;
  }
  .context-separator {
    height: 1px;
    background: var(--color-border, #404040);
    margin: 4px 0;
  }
</style>
