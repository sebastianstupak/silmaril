<script lang="ts">
  import { t } from '$lib/i18n';
  import type { EntityInfo } from '$lib/api';

  let { entity = null }: { entity: EntityInfo | null } = $props();

  let collapsedSections: Record<string, boolean> = $state({});

  function toggleSection(name: string) {
    collapsedSections[name] = !collapsedSections[name];
  }
</script>

<div class="inspector">
  {#if !entity}
    <p class="inspector-empty">{t('inspector.no_selection')}</p>
  {:else}
    <div class="inspector-header">
      <span class="inspector-entity-icon">
        <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
          <path d="M8 1l6.5 3.75v6.5L8 15l-6.5-3.75v-6.5L8 1z" stroke="currentColor" stroke-width="1" fill="none"/>
        </svg>
      </span>
      <span class="inspector-entity-name">{entity.name}</span>
      <span class="inspector-entity-id">#{entity.id}</span>
    </div>

    <div class="inspector-section-label">{t('inspector.components')}</div>

    {#each entity.components as component (component)}
      <div class="component-section">
        <button
          class="component-header"
          onclick={() => toggleSection(component)}
          aria-expanded={!collapsedSections[component]}
        >
          <span class="component-chevron" class:collapsed={collapsedSections[component]}>
            <svg width="10" height="10" viewBox="0 0 16 16" fill="currentColor">
              <path d="M4 6l4 4 4-4" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </span>
          <span class="component-name">{component}</span>
        </button>
        {#if !collapsedSections[component]}
          <div class="component-body">
            <div class="component-field">
              <span class="field-label">type</span>
              <span class="field-value">{component}</span>
            </div>
          </div>
        {/if}
      </div>
    {/each}

    <button class="add-component-btn">
      + {t('inspector.add_component')}
    </button>
  {/if}
</div>

<style>
  .inspector {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow-y: auto;
  }

  .inspector-empty {
    color: var(--color-textDim, #666);
    font-style: italic;
    padding: 12px 8px;
    font-size: 12px;
    text-align: center;
  }

  .inspector-header {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px;
    border-bottom: 1px solid var(--color-border, #404040);
    flex-shrink: 0;
  }

  .inspector-entity-icon {
    display: flex;
    align-items: center;
    color: var(--color-accent, #007acc);
    flex-shrink: 0;
  }

  .inspector-entity-name {
    font-size: 13px;
    font-weight: 600;
    color: var(--color-text, #ccc);
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .inspector-entity-id {
    font-size: 10px;
    color: var(--color-textDim, #666);
    flex-shrink: 0;
  }

  .inspector-section-label {
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--color-textDim, #666);
    padding: 8px 8px 4px;
  }

  .component-section {
    border-bottom: 1px solid var(--color-border, #404040);
  }

  .component-header {
    display: flex;
    align-items: center;
    gap: 4px;
    width: 100%;
    padding: 6px 8px;
    background: var(--color-bgHeader, #2d2d2d);
    border: none;
    cursor: pointer;
    color: var(--color-text, #ccc);
    font-size: 12px;
    font-weight: 500;
    text-align: left;
  }

  .component-header:hover {
    background: var(--color-bg, #1e1e1e);
  }

  .component-chevron {
    display: flex;
    align-items: center;
    transition: transform 0.15s ease;
    color: var(--color-textMuted, #999);
  }

  .component-chevron.collapsed {
    transform: rotate(-90deg);
  }

  .component-name {
    flex: 1;
  }

  .component-body {
    padding: 4px 8px 8px 26px;
  }

  .component-field {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 2px 0;
    font-size: 11px;
  }

  .field-label {
    color: var(--color-textMuted, #999);
    min-width: 60px;
    flex-shrink: 0;
  }

  .field-value {
    color: var(--color-text, #ccc);
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .add-component-btn {
    margin: 8px;
    padding: 6px 12px;
    background: var(--color-bg, #1e1e1e);
    border: 1px dashed var(--color-border, #404040);
    border-radius: 4px;
    color: var(--color-textMuted, #999);
    font-size: 11px;
    cursor: pointer;
    text-align: center;
  }

  .add-component-btn:hover {
    border-color: var(--color-accent, #007acc);
    color: var(--color-accent, #007acc);
  }
</style>
