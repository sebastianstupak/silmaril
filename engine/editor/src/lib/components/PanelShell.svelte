<script lang="ts">
  import type { Snippet } from 'svelte';

  let { title = 'Panel', children }: { title?: string; children?: Snippet } = $props();
</script>

<div class="panel">
  <div class="panel-header">
    <span>{title}</span>
  </div>
  <div class="panel-content">
    {#if children}
      {@render children()}
    {/if}
  </div>
</div>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    background: var(--color-bgPanel, #252525);
    overflow: hidden;
  }
  .panel-header {
    padding: 6px 12px;
    background: var(--color-bgHeader, #2d2d2d);
    border-bottom: 1px solid var(--color-border, #404040);
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--color-textMuted, #999);
    flex-shrink: 0;
    user-select: none;
  }
  .panel-content {
    flex: 1;
    padding: 8px;
    overflow: auto;
  }
</style>
