<script lang="ts">
  import { getPanelTitle } from '$lib/contributions/registry';
  import { getDragState, subscribeDrag } from './store';
  import { onMount } from 'svelte';

  let active = $state(false);
  let mouseX = $state(0);
  let mouseY = $state(0);
  let panelId = $state('');

  onMount(() => {
    const unsub = subscribeDrag(() => {
      const s = getDragState();
      // Don't show ghost/backdrop during popout-window drags — only internal tab drags.
      active = s.active && !s.popout;
      mouseX = s.mouseX;
      mouseY = s.mouseY;
      panelId = s.panelId;
    });
    return unsub;
  });

  function getLabel(id: string): string {
    return getPanelTitle(id);
  }
</script>

{#if active}
  <!-- Ghost tab following cursor -->
  <div class="drag-ghost" style="left: {mouseX + 10}px; top: {mouseY - 15}px;">
    {getLabel(panelId)}
  </div>

  <!-- Full-screen overlay to capture mouse events everywhere -->
  <div class="drag-backdrop"></div>
{/if}

<style>
  .drag-ghost {
    position: fixed;
    z-index: 10000;
    padding: 4px 12px;
    background: var(--color-accent, #007acc);
    color: white;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    border-radius: 4px;
    pointer-events: none;
    box-shadow: 0 2px 8px rgba(0,0,0,0.5);
    white-space: nowrap;
  }
  .drag-backdrop {
    position: fixed;
    inset: 0;
    z-index: 9999;
    cursor: grabbing;
  }
</style>
