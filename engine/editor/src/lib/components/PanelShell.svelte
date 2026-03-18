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
    border: 1px solid #404040;
    background: #252525;
    border-radius: 4px;
    overflow: hidden;
  }
  .panel-header {
    padding: 6px 12px;
    background: #2d2d2d;
    border-bottom: 1px solid #404040;
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: #999;
  }
  .panel-content {
    flex: 1;
    padding: 8px;
    overflow: auto;
  }
</style>
