<script lang="ts">
  import type { DropZone } from './types';
  import { getDragState, subscribeDrag } from './store';
  import { onMount } from 'svelte';

  interface Props {
    onDrop: (zone: DropZone) => void;
  }

  let { onDrop }: Props = $props();

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

      if (s.active && overlayEl) {
        hoveredZone = hitTestZone(s.mouseX, s.mouseY);
      } else {
        // Drag ended — if we had a hovered zone, execute the drop
        if (wasDragging && hoveredZone && s.panelId === '' && !s.active) {
          // endDrag was called — but we need the panelId from before.
          // We handle drop in the mouseup path instead (see below).
        }
        hoveredZone = null;
      }
    });

    // Listen for mouseup to detect drop while dragging
    function handleMouseUp(_ev: MouseEvent) {
      const state = getDragState();
      if (!state.active) return;
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

{#if isDragging}
  <div class="dock-drop-overlay" bind:this={overlayEl}>
    <!-- Visual zone indicators -->
    <div class="dock-drop-zone zone-left" class:hovered={hoveredZone === 'left'}></div>
    <div class="dock-drop-zone zone-right" class:hovered={hoveredZone === 'right'}></div>
    <div class="dock-drop-zone zone-top" class:hovered={hoveredZone === 'top'}></div>
    <div class="dock-drop-zone zone-bottom" class:hovered={hoveredZone === 'bottom'}></div>
    <div class="dock-drop-zone zone-center" class:hovered={hoveredZone === 'center'}>
      {#if hoveredZone === 'center'}
        <div class="zone-indicator">
          <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor" opacity="0.7">
            <rect x="2" y="2" width="12" height="12" rx="1" fill="none" stroke="currentColor" stroke-width="1.5"/>
          </svg>
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .dock-drop-overlay {
    position: absolute;
    inset: 0;
    z-index: 100;
    /* Allow mouse events to pass through to backdrop for cursor,
       but the overlay itself needs to be measurable for hit-testing */
    pointer-events: none;
  }
  .dock-drop-zone {
    position: absolute;
    pointer-events: none;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.1s;
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
    border-radius: 4px;
  }
  .zone-indicator {
    opacity: 1;
  }
</style>
