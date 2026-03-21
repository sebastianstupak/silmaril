<!-- engine/editor/src/lib/components/HierarchyPanel.svelte -->
<script lang="ts">
  import { t } from '$lib/i18n';
  import type { EntityInfo } from '$lib/api';
  import {
    createEntity,
    deleteEntity,
    duplicateEntity,
    renameEntity,
    selectEntity,
  } from '$lib/scene/commands';

  let { entities = [], selectedId = null, onSelect }: {
    entities: EntityInfo[];
    selectedId: number | null;
    onSelect: (id: number) => void;
  } = $props();

  let filter = $state('');
  let hoveredId = $state<number | null>(null);
  let renamingId = $state<number | null>(null);
  let renameValue = $state('');
  let contextMenu = $state<{ x: number; y: number; entityId: number } | null>(null);

  let filtered = $derived(
    filter
      ? entities.filter((e) => e.name.toLowerCase().includes(filter.toLowerCase()))
      : entities,
  );

  function handleNew() {
    const e = createEntity();
    onSelect(e.id);
  }

  function startRename(id: number, currentName: string) {
    renamingId = id;
    renameValue = currentName;
    contextMenu = null;
  }

  function commitRename() {
    if (renamingId !== null && renameValue.trim()) {
      renameEntity(renamingId, renameValue.trim());
    }
    renamingId = null;
    renameValue = '';
  }

  function cancelRename() {
    renamingId = null;
    renameValue = '';
  }

  function openContextMenu(e: MouseEvent, entityId: number) {
    e.preventDefault();
    contextMenu = { x: e.clientX, y: e.clientY, entityId };
  }

  function closeContextMenu() {
    contextMenu = null;
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Delete' && selectedId !== null && renamingId === null) {
      e.preventDefault();
      deleteEntity(selectedId);
    }
    if (e.key === 'Escape') cancelRename();
  }
</script>

<!-- svelte-ignore a11y-no-static-element-interactions -->
<div class="hierarchy" onkeydown={onKeydown} tabindex="-1" role="tree">

  <div class="hierarchy-header">
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
    <button class="add-btn" onclick={handleNew} title="New Entity">+</button>
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
          tabindex="0"
          onmouseenter={() => { hoveredId = entity.id; }}
          onmouseleave={() => { if (!contextMenu) hoveredId = null; }}
          onclick={() => {
            if (renamingId !== entity.id) {
              selectEntity(entity.id);
              onSelect(entity.id);
            }
          }}
          ondblclick={() => startRename(entity.id, entity.name)}
          oncontextmenu={(e) => openContextMenu(e, entity.id)}
          onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { selectEntity(entity.id); onSelect(entity.id); } }}
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

          {#if renamingId === entity.id}
            <!-- svelte-ignore a11y-autofocus -->
            <input
              class="rename-input"
              bind:value={renameValue}
              autofocus
              onblur={commitRename}
              onkeydown={(e) => {
                if (e.key === 'Enter') { e.preventDefault(); commitRename(); }
                if (e.key === 'Escape') { e.preventDefault(); cancelRename(); }
                e.stopPropagation();
              }}
              onclick={(e) => e.stopPropagation()}
            />
          {:else}
            <span class="entity-name">{entity.name}</span>
          {/if}

          {#if hoveredId === entity.id && renamingId !== entity.id}
            <span class="entity-actions">
              <button
                class="action-btn"
                title="Duplicate"
                onclick={(e) => { e.stopPropagation(); duplicateEntity(entity.id); }}
              >⧉</button>
              <button
                class="action-btn"
                title="Add Child"
                onclick={(e) => { e.stopPropagation(); const c = createEntity(); onSelect(c.id); }}
              >+</button>
              <button
                class="action-btn delete-btn"
                title="Delete"
                onclick={(e) => { e.stopPropagation(); deleteEntity(entity.id); }}
              >✕</button>
            </span>
          {:else if renamingId !== entity.id}
            <span class="entity-component-count" title={entity.components.join(', ')}>
              {entity.components.length}
            </span>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}

  {#if contextMenu}
    <div
      class="context-menu"
      style="left: {contextMenu.x}px; top: {contextMenu.y}px;"
      role="menu"
    >
      {@const cid = contextMenu.entityId}
      {@const cname = entities.find((e) => e.id === cid)?.name ?? ''}
      <button role="menuitem" onclick={() => { startRename(cid, cname); }}>Rename</button>
      <button role="menuitem" onclick={() => { duplicateEntity(cid); closeContextMenu(); }}>Duplicate</button>
      <button role="menuitem" onclick={() => { const c = createEntity(); onSelect(c.id); closeContextMenu(); }}>Add Child</button>
      <hr />
      <button role="menuitem" class="danger" onclick={() => { deleteEntity(cid); closeContextMenu(); }}>Delete</button>
    </div>
    <!-- svelte-ignore a11y-no-static-element-interactions -->
    <div class="context-backdrop" role="none" onclick={closeContextMenu}></div>
  {/if}
</div>

<style>
  .hierarchy {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
    outline: none;
    position: relative;
  }

  .hierarchy-header {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 8px;
    border-bottom: 1px solid var(--color-border, #404040);
    flex-shrink: 0;
  }

  .hierarchy-search {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: 1;
    min-width: 0;
  }

  .search-icon { color: var(--color-textDim, #666); flex-shrink: 0; }

  .search-input {
    flex: 1;
    background: var(--color-bg, #1e1e1e);
    border: 1px solid var(--color-border, #404040);
    border-radius: 3px;
    padding: 3px 6px;
    font-size: 11px;
    color: var(--color-text, #ccc);
    outline: none;
    min-width: 0;
  }

  .search-input:focus { border-color: var(--color-accent, #007acc); }
  .search-input::placeholder { color: var(--color-textDim, #666); }

  .add-btn {
    all: unset;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    border-radius: 3px;
    font-size: 16px;
    color: var(--color-textMuted, #999);
    cursor: pointer;
    flex-shrink: 0;
    line-height: 1;
  }

  .add-btn:hover { background: var(--color-bgHeader, #2d2d2d); color: var(--color-text, #ccc); }

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

  .entity-list { list-style: none; margin: 0; padding: 0; overflow-y: auto; flex: 1; }

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

  .entity-row:hover { background: var(--color-bgHeader, #2d2d2d); }
  .entity-row.selected { background: var(--color-accent, #007acc); color: #fff; }
  .entity-row:focus-visible { outline: 1px solid var(--color-accent, #007acc); outline-offset: -1px; }

  .entity-chevron { display: flex; align-items: center; color: var(--color-textDim, #666); flex-shrink: 0; width: 14px; }
  .entity-icon { display: flex; align-items: center; color: var(--color-textMuted, #999); flex-shrink: 0; }
  .entity-row.selected .entity-icon, .entity-row.selected .entity-chevron { color: rgba(255,255,255,0.7); }

  .entity-name { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

  .rename-input {
    flex: 1;
    background: var(--color-bg, #1e1e1e);
    border: 1px solid var(--color-accent, #007acc);
    border-radius: 2px;
    padding: 1px 4px;
    font-size: 12px;
    color: var(--color-text, #ccc);
    outline: none;
    font-family: inherit;
    min-width: 0;
  }

  .entity-component-count {
    font-size: 10px;
    color: var(--color-textDim, #666);
    background: var(--color-bg, #1e1e1e);
    padding: 0 4px;
    border-radius: 3px;
    flex-shrink: 0;
  }
  .entity-row.selected .entity-component-count { background: rgba(255,255,255,0.15); color: rgba(255,255,255,0.8); }

  .entity-actions {
    display: flex;
    gap: 1px;
    margin-left: auto;
    flex-shrink: 0;
  }

  .action-btn {
    all: unset;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    border-radius: 2px;
    font-size: 11px;
    color: var(--color-textDim, #666);
    cursor: pointer;
  }

  .action-btn:hover { background: rgba(255,255,255,0.1); color: var(--color-text, #ccc); }
  .delete-btn:hover { color: #f38ba8; }

  /* Context menu */
  .context-menu {
    position: fixed;
    background: var(--color-bgPanel, #252525);
    border: 1px solid var(--color-border, #404040);
    border-radius: 5px;
    padding: 4px;
    min-width: 140px;
    box-shadow: 0 4px 16px rgba(0,0,0,0.5);
    z-index: 10000;
    display: flex;
    flex-direction: column;
  }

  .context-menu button {
    all: unset;
    display: block;
    width: 100%;
    padding: 5px 10px;
    font-size: 12px;
    color: var(--color-text, #ccc);
    cursor: pointer;
    border-radius: 3px;
    box-sizing: border-box;
  }

  .context-menu button:hover { background: var(--color-bgHeader, #2d2d2d); }
  .context-menu button.danger { color: #f38ba8; }
  .context-menu hr { border: none; border-top: 1px solid var(--color-border, #404040); margin: 3px 0; }

  .context-backdrop { position: fixed; inset: 0; z-index: 9999; }
</style>
