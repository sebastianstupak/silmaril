<script lang="ts">
  import type { Snippet } from 'svelte';
  import { t } from '../i18n';

  let { title = 'Panel', children }: { title?: string; children?: Snippet } = $props();

  let menuOpen = $state(false);
</script>

<div class="panel">
  <div class="panel-header">
    <span class="panel-icon-slot"></span>
    <span class="panel-title">{title}</span>
    <button
      class="panel-menu-btn"
      title={t('panel.menu')}
      onclick={() => menuOpen = !menuOpen}
    >
      <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
        <circle cx="8" cy="3" r="1.5"/>
        <circle cx="8" cy="8" r="1.5"/>
        <circle cx="8" cy="13" r="1.5"/>
      </svg>
    </button>
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
    padding: 8px 12px;
    background: var(--color-bgHeader, #2d2d2d);
    border-bottom: 1px solid var(--color-border, #404040);
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--color-textMuted, #999);
    flex-shrink: 0;
    user-select: none;
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .panel-icon-slot {
    width: 14px;
    height: 14px;
    flex-shrink: 0;
  }
  .panel-title {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .panel-menu-btn {
    background: none;
    border: 1px solid transparent;
    color: var(--color-textDim, #666);
    cursor: pointer;
    padding: 2px;
    border-radius: 3px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    opacity: 0;
    transition: opacity 0.15s ease;
  }
  .panel-header:hover .panel-menu-btn {
    opacity: 1;
  }
  .panel-menu-btn:hover {
    color: var(--color-text, #ccc);
    background: var(--color-bgPanel, #252525);
    border-color: var(--color-border, #404040);
  }
  .panel-content {
    flex: 1;
    padding: 8px;
    overflow: auto;
  }
</style>
