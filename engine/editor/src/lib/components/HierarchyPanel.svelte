<script lang="ts">
  import { t } from '$lib/i18n';
  import type { EntityInfo } from '$lib/api';

  let { entities = [], selectedId = null, onSelect }: {
    entities: EntityInfo[];
    selectedId: number | null;
    onSelect: (id: number) => void;
  } = $props();

  let filter = $state('');

  let filtered = $derived(
    filter
      ? entities.filter(e => e.name.toLowerCase().includes(filter.toLowerCase()))
      : entities
  );
</script>

<div class="hierarchy">
  <div class="hierarchy-search">
    <svg class="search-icon" width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
      <path d="M11.742 10.344a6.5 6.5 0 1 0-1.397 1.398h-.001l3.85 3.85a1 1 0 0 0 1.415-1.414l-3.85-3.85zm-5.242.156a5 5 0 1 1 0-10 5 5 0 0 1 0 10z"/>
    </svg>
    <input
      type="text"
      class="search-input"
      placeholder={t('hierarchy.search')}
      bind:value={filter}
    />
  </div>

  {#if entities.length === 0}
    <p class="hierarchy-empty">{t('placeholder.no_project')}</p>
  {:else if filtered.length === 0}
    <p class="hierarchy-empty">{t('hierarchy.empty')}</p>
  {:else}
    <div class="hierarchy-count">
      {t('hierarchy.count').replace('{count}', String(filtered.length))}
    </div>
    <ul class="entity-list" role="listbox" aria-label={t('panel.hierarchy')}>
      {#each filtered as entity (entity.id)}
        <li
          class="entity-row"
          class:selected={selectedId === entity.id}
          role="option"
          aria-selected={selectedId === entity.id}
          onclick={() => onSelect(entity.id)}
          onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') onSelect(entity.id); }}
          tabindex="0"
        >
          <span class="entity-chevron">
            <svg width="10" height="10" viewBox="0 0 16 16" fill="currentColor">
              <path d="M6 4l4 4-4 4" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </span>
          <span class="entity-icon">
            <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
              <path d="M8 1l6.5 3.75v6.5L8 15l-6.5-3.75v-6.5L8 1z" stroke="currentColor" stroke-width="1" fill="none"/>
            </svg>
          </span>
          <span class="entity-name">{entity.name}</span>
          <span class="entity-component-count" title={entity.components.join(', ')}>
            {entity.components.length}
          </span>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .hierarchy {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .hierarchy-search {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 8px;
    border-bottom: 1px solid var(--color-border, #404040);
    flex-shrink: 0;
  }

  .search-icon {
    color: var(--color-textDim, #666);
    flex-shrink: 0;
  }

  .search-input {
    flex: 1;
    background: var(--color-bg, #1e1e1e);
    border: 1px solid var(--color-border, #404040);
    border-radius: 3px;
    padding: 3px 6px;
    font-size: 11px;
    color: var(--color-text, #ccc);
    outline: none;
  }

  .search-input:focus {
    border-color: var(--color-accent, #007acc);
  }

  .search-input::placeholder {
    color: var(--color-textDim, #666);
  }

  .hierarchy-empty {
    color: var(--color-textDim, #666);
    font-style: italic;
    padding: 12px 8px;
    font-size: 12px;
    text-align: center;
  }

  .hierarchy-count {
    font-size: 10px;
    color: var(--color-textDim, #666);
    padding: 2px 8px;
    flex-shrink: 0;
  }

  .entity-list {
    list-style: none;
    margin: 0;
    padding: 0;
    overflow-y: auto;
    flex: 1;
  }

  .entity-row {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 3px 8px;
    cursor: pointer;
    font-size: 12px;
    color: var(--color-text, #ccc);
    border: 1px solid transparent;
    user-select: none;
  }

  .entity-row:hover {
    background: var(--color-bgHeader, #2d2d2d);
  }

  .entity-row.selected {
    background: var(--color-accent, #007acc);
    color: #fff;
  }

  .entity-row:focus-visible {
    outline: 1px solid var(--color-accent, #007acc);
    outline-offset: -1px;
  }

  .entity-chevron {
    display: flex;
    align-items: center;
    color: var(--color-textDim, #666);
    flex-shrink: 0;
    width: 14px;
  }

  .entity-icon {
    display: flex;
    align-items: center;
    color: var(--color-textMuted, #999);
    flex-shrink: 0;
  }

  .entity-row.selected .entity-icon,
  .entity-row.selected .entity-chevron {
    color: rgba(255, 255, 255, 0.7);
  }

  .entity-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .entity-component-count {
    font-size: 10px;
    color: var(--color-textDim, #666);
    background: var(--color-bg, #1e1e1e);
    padding: 0 4px;
    border-radius: 3px;
    flex-shrink: 0;
  }

  .entity-row.selected .entity-component-count {
    background: rgba(255, 255, 255, 0.15);
    color: rgba(255, 255, 255, 0.8);
  }
</style>
