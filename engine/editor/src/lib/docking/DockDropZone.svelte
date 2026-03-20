<script lang="ts">
  import type { DropZone } from './types';
  import { getDragState, subscribeDrag, updateDropTarget } from './store';
  import { onMount } from 'svelte';

  interface Props {
    onDrop: (zone: DropZone) => void;
    path?: number[];
    isBottomPanel?: boolean;
  }

  let { onDrop, path = [], isBottomPanel = false }: Props = $props();

  let isDragging = $state(false);
  let mouseX = $state(0);
  let mouseY = $state(0);
  let hoveredZone: DropZone | null = $state(null);
  let overlayEl: HTMLDivElement | undefined = $state(undefined);

  onMount(() => {
    const unsub = subscribeDrag(() => {
      const s = getDragState();
      const wasDragging = isDragging;
      isDragging = s.active;
      mouseX = s.mouseX;
      mouseY = s.mouseY;

      if (s.active) {
        const next = hitTestZone(s.mouseX, s.mouseY);
        hoveredZone = next;
        // Report current hover target to the store so dock-panel-back can use it.
        // Only update when this panel IS hovered — don't clear when it isn't,
        // because other DockDropZone instances also call this and would race.
        if (s.popout && next !== null) {
          updateDropTarget(path, next, isBottomPanel);
        }
      } else {
        hoveredZone = null;
      }
    });

    // Listen for mouseup to detect drop while dragging.
    // Skip popout-window drags — those are dropped via the 'dock-panel-back' Tauri event.
    function handleMouseUp(_ev: MouseEvent) {
      const state = getDragState();
      if (!state.active || state.popout) return;
      if (!overlayEl) return;

      const zone = hitTestZone(state.mouseX, state.mouseY);
      if (zone) {
        onDrop(zone);
      }
    }

    window.addEventListener('mouseup', handleMouseUp);

    return () => {
      unsub();
      window.removeEventListener('mouseup', handleMouseUp);
    };
  });

  function hitTestZone(x: number, y: number): DropZone | null {
    if (!overlayEl) return null;
    const rect = overlayEl.getBoundingClientRect();

    const relX = (x - rect.left) / rect.width;
    const relY = (y - rect.top) / rect.height;

    if (relX < 0 || relX > 1 || relY < 0 || relY > 1) return null;

    // Edge zones (20% from each edge)
    if (relX < 0.2) return 'left';
    if (relX > 0.8) return 'right';
    if (relY < 0.2) return 'top';
    if (relY > 0.8) return 'bottom';
    return 'center';
  }
</script>

<!-- Always rendered so overlayEl is always bound (needed for getBoundingClientRect in hit-tests).
     Zones are hidden via CSS when not dragging. -->
<div class="dock-drop-overlay" class:active={isDragging} bind:this={overlayEl}>
  <div class="dock-drop-zone zone-left" class:hovered={hoveredZone === 'left'}>
    <svg width="24" height="24" viewBox="0 0 24 24" fill="white" opacity="0.8"><path d="M14 7l-5 5 5 5V7z"/></svg>
  </div>
  <div class="dock-drop-zone zone-right" class:hovered={hoveredZone === 'right'}>
    <svg width="24" height="24" viewBox="0 0 24 24" fill="white" opacity="0.8"><path d="M10 17l5-5-5-5v10z"/></svg>
  </div>
  <div class="dock-drop-zone zone-top" class:hovered={hoveredZone === 'top'}>
    <svg width="24" height="24" viewBox="0 0 24 24" fill="white" opacity="0.8"><path d="M7 14l5-5 5 5H7z"/></svg>
  </div>
  <div class="dock-drop-zone zone-bottom" class:hovered={hoveredZone === 'bottom'}>
    <svg width="24" height="24" viewBox="0 0 24 24" fill="white" opacity="0.8"><path d="M7 10l5 5 5-5H7z"/></svg>
  </div>
  <div class="dock-drop-zone zone-center" class:hovered={hoveredZone === 'center'}>
    <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.5" opacity="0.8"><rect x="4" y="4" width="16" height="16" rx="2"/><line x1="4" y1="10" x2="20" y2="10"/></svg>
  </div>
</div>

<style>
  .dock-drop-overlay {
    position: absolute;
    inset: 0;
    z-index: 99999;
    /* Must be above everything in the panel (viewport toolbar z-index: 10,
       drag overlay z-index: 10000, etc.) */
    pointer-events: none;
  }
  /* Zones are invisible when not dragging; the overlay div itself stays mounted
     so getBoundingClientRect() is always available for hit-testing. */
  .dock-drop-overlay:not(.active) .dock-drop-zone {
    display: none;
  }
  .dock-drop-zone {
    position: absolute;
    pointer-events: none;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.15s, opacity 0.15s;
    opacity: 0.3;
    background: rgba(0, 122, 204, 0.05);
    border-radius: 4px;
  }
  .dock-drop-zone.hovered {
    opacity: 1;
  }
  .zone-left {
    left: 0;
    top: 20%;
    width: 25%;
    height: 60%;
  }
  .zone-right {
    right: 0;
    top: 20%;
    width: 25%;
    height: 60%;
  }
  .zone-top {
    top: 0;
    left: 20%;
    width: 60%;
    height: 25%;
  }
  .zone-bottom {
    bottom: 0;
    left: 20%;
    width: 60%;
    height: 25%;
  }
  .zone-center {
    top: 25%;
    left: 25%;
    width: 50%;
    height: 50%;
  }
  .dock-drop-zone.hovered {
    background: color-mix(in srgb, var(--color-accent, #007acc) 30%, transparent);
    outline: 2px solid var(--color-accent, #007acc);
    outline-offset: -2px;
  }
</style>
