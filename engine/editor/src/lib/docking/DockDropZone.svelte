<script lang="ts">
  import { t } from '$lib/i18n';
  import type { DropZone } from './types';

  interface Props {
    isDragging: boolean;
    onDrop: (zone: DropZone) => void;
  }

  let { isDragging, onDrop }: Props = $props();

  let hoveredZone: DropZone | null = $state(null);

  function handleDragOver(e: DragEvent, zone: DropZone) {
    e.preventDefault();
    e.stopPropagation();
    if (e.dataTransfer) e.dataTransfer.dropEffect = 'move';
    hoveredZone = zone;
  }

  function handleDragLeave(zone: DropZone) {
    if (hoveredZone === zone) hoveredZone = null;
  }

  function handleDrop(e: DragEvent, zone: DropZone) {
    e.preventDefault();
    e.stopPropagation();
    hoveredZone = null;
    onDrop(zone);
  }

  const zones: DropZone[] = ['left', 'right', 'top', 'bottom', 'center'];
</script>

{#if isDragging}
  <div class="dock-drop-overlay">
    {#each zones as zone}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="dock-drop-zone zone-{zone}"
        class:hovered={hoveredZone === zone}
        ondragover={(e) => handleDragOver(e, zone)}
        ondragleave={() => handleDragLeave(zone)}
        ondrop={(e) => handleDrop(e, zone)}
      >
        <div class="zone-indicator">
          {#if zone === 'center'}
            <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor" opacity="0.7">
              <rect x="2" y="2" width="12" height="12" rx="1" fill="none" stroke="currentColor" stroke-width="1.5"/>
            </svg>
          {/if}
        </div>
      </div>
    {/each}
  </div>
{/if}

<style>
  .dock-drop-overlay {
    position: absolute;
    inset: 0;
    z-index: 100;
    pointer-events: none;
  }
  .dock-drop-zone {
    position: absolute;
    pointer-events: all;
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
    opacity: 0;
    transition: opacity 0.1s;
  }
  .dock-drop-zone.hovered .zone-indicator {
    opacity: 1;
  }
</style>
