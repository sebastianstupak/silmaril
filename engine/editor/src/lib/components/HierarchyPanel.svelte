<!-- engine/editor/src/lib/components/HierarchyPanel.svelte -->
<script lang="ts">
  import { t } from '$lib/i18n';
  import type { EntityInfo } from '$lib/api';
  import { assignMesh } from '$lib/api';
  import { invoke } from '@tauri-apps/api/core';
  import {
    createEntity,
    createEntityChild,
    deleteEntity,
    duplicateEntity,
    renameEntity,
  } from '$lib/template/commands';
  import { getActiveTemplatePath } from '$lib/stores/undo-history';

  let { entities = [], selectedId = null, onSelect }: {
    entities: EntityInfo[];
    selectedId: number | null;
    onSelect: (id: number) => void;
  } = $props();

  const isTauri = typeof window !== 'undefined' && !!(window as any).__TAURI_INTERNALS__;

  let filter = $state('');
  let hoveredId = $state<number | null>(null);
  let renamingId = $state<number | null>(null);
  let renameValue = $state('');
  let contextMenu = $state<{ x: number; y: number; entityId: number } | null>(null);
  // Track expanded state per entity id
  let expanded = $state<Set<number>>(new Set());

  // Build a tree structure from flat entity list
  type TreeNode = { entity: EntityInfo; children: TreeNode[] };

  function buildTree(list: EntityInfo[]): TreeNode[] {
    const byId = new Map<number, TreeNode>();
    for (const e of list) byId.set(e.id, { entity: e, children: [] });
    const roots: TreeNode[] = [];
    for (const e of list) {
      const node = byId.get(e.id)!;
      if (e.parentId != null && byId.has(e.parentId)) {
        byId.get(e.parentId)!.children.push(node);
      } else {
        roots.push(node);
      }
    }
    return roots;
  }

  function filterTree(nodes: TreeNode[], q: string): TreeNode[] {
    if (!q) return nodes;
    const result: TreeNode[] = [];
    for (const n of nodes) {
      const childMatches = filterTree(n.children, q);
      if (n.entity.name.toLowerCase().includes(q.toLowerCase()) || childMatches.length > 0) {
        result.push({ entity: n.entity, children: childMatches });
      }
    }
    return result;
  }

  let tree = $derived(filterTree(buildTree(entities), filter));

  // Auto-expand parents of nodes when filter is active
  $effect(() => {
    if (filter) {
      const toExpand = new Set<number>();
      function collectParents(nodes: TreeNode[]) {
        for (const n of nodes) {
          if (n.children.length > 0) {
            toExpand.add(n.entity.id);
            collectParents(n.children);
          }
        }
      }
      collectParents(tree);
      expanded = toExpand;
    }
  });

  function toggleExpand(id: number, e: Event) {
    e.stopPropagation();
    const next = new Set(expanded);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    expanded = next;
  }

  function handleNew() {
    const e = createEntity();
    onSelect(e.id);
  }

  function handleAddChild(parentId: number) {
    createEntityChild(parentId).then((id) => {
      expanded = new Set([...expanded, parentId]);
      onSelect(id);
    });
    contextMenu = null;
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

  async function focusEntity(id: number): Promise<void> {
    if (!isTauri) return;
    await invoke('focus_entity_animated', { entityId: id });
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

  // Count total visible entities for the count line
  function countNodes(nodes: TreeNode[]): number {
    return nodes.reduce((acc, n) => acc + 1 + countNodes(n.children), 0);
  }

  // ---------------------------------------------------------------------------
  // Mesh drag-drop
  // ---------------------------------------------------------------------------

  let dropTargetId = $state<number | null>(null);

  const ENTITY_MIME = 'application/x-entity-id';
  let draggedEntityId  = $state<number | null>(null);
  let reparentTargetId = $state<number | null>(null);

  function onDragOver(event: DragEvent, entityId: number): void {
    if (event.dataTransfer?.types.includes('application/x-mesh-path')) {
      event.preventDefault();
      if (event.dataTransfer) event.dataTransfer.dropEffect = 'copy';
      dropTargetId = entityId;
    }
  }

  function onDragLeave(entityId: number): void {
    if (dropTargetId === entityId) dropTargetId = null;
  }

  async function onDropMesh(event: DragEvent, entityId: number): Promise<void> {
    event.preventDefault();
    dropTargetId = null;
    const meshPath = event.dataTransfer?.getData('application/x-mesh-path');
    if (!meshPath) return;
    const templatePath = getActiveTemplatePath();
    if (!templatePath) return;
    await assignMesh(entityId, templatePath, meshPath);
  }

  function startEntityDrag(e: DragEvent, id: number): void {
    draggedEntityId = id;
    e.dataTransfer!.setData(ENTITY_MIME, String(id));
    e.dataTransfer!.effectAllowed = 'move';
  }

  function onEntityDragOver(e: DragEvent, target: EntityInfo): void {
    if (!e.dataTransfer?.types.includes(ENTITY_MIME)) return;
    if (draggedEntityId === null) return;
    if (draggedEntityId === target.id) return;
    if (isDescendant(target.id, draggedEntityId)) return;
    e.preventDefault();
    e.dataTransfer!.dropEffect = 'move';
    reparentTargetId = target.id;
  }

  async function onEntityDrop(e: DragEvent, newParentId: number): Promise<void> {
    if (!e.dataTransfer?.types.includes(ENTITY_MIME)) return;
    const entityId = draggedEntityId;
    draggedEntityId   = null;
    reparentTargetId  = null;
    if (entityId === null || entityId === newParentId) return;
    if (!isTauri) return;
    await invoke('reparent_entity', { entityId, newParentId });
  }

  function isDescendant(candidateId: number, ancestorId: number): boolean {
    let current: number | undefined = candidateId;
    while (current !== undefined) {
      const info = entities.find((e) => e.id === current);
      if (!info) return false;
      if (info.parentId === ancestorId) return true;
      current = info.parentId;
    }
    return false;
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
  {:else if tree.length === 0}
    <p class="hierarchy-empty">{t('hierarchy.empty')}</p>
  {:else}
    <div class="hierarchy-count">
      {t('hierarchy.count').replace('{count}', String(countNodes(tree)))}
    </div>
    <ul class="entity-list" role="listbox" aria-label={t('panel.hierarchy')}>
      {#snippet entityRow(node: TreeNode, depth: number)}
        {@const entity = node.entity}
        {@const hasChildren = node.children.length > 0}
        {@const isExpanded = expanded.has(entity.id)}
        <li
          class="entity-row"
          class:selected={selectedId === entity.id}
          class:drop-target={dropTargetId === entity.id}
          class:reparent-target={reparentTargetId === entity.id}
          role="option"
          aria-selected={selectedId === entity.id}
          tabindex="0"
          draggable="true"
          style="padding-left: {8 + depth * 14}px"
          onmouseenter={() => { hoveredId = entity.id; }}
          onmouseleave={() => { if (!contextMenu) hoveredId = null; }}
          onclick={() => {
            if (renamingId !== entity.id) {
              onSelect(entity.id);
            }
          }}
          ondblclick={() => focusEntity(entity.id)}
          oncontextmenu={(e) => openContextMenu(e, entity.id)}
          onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { onSelect(entity.id); } }}
          ondragstart={(e) => startEntityDrag(e, entity.id)}
          ondragover={(e) => { onDragOver(e, entity.id); onEntityDragOver(e, entity); }}
          ondragleave={() => { onDragLeave(entity.id); reparentTargetId = null; }}
          ondrop={(e) => {
            onDropMesh(e, entity.id);      // handles mesh drags; no-op if MIME is not mesh
            onEntityDrop(e, entity.id);    // handles entity drags; no-op if MIME is not entity
          }}
          ondragend={() => { draggedEntityId = null; reparentTargetId = null; }}
        >
          <!-- Expand/collapse chevron -->
          <span
            class="entity-chevron"
            class:has-children={hasChildren}
            class:expanded={isExpanded}
            onclick={hasChildren ? (e) => toggleExpand(entity.id, e) : undefined}
            role={hasChildren ? 'button' : undefined}
          >
            {#if hasChildren}
              <svg width="10" height="10" viewBox="0 0 16 16" fill="currentColor">
                <path d="M6 4l4 4-4 4" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
            {/if}
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
            <span
              class="entity-name"
              ondblclick={(e) => { e.stopPropagation(); startRename(entity.id, entity.name); }}
            >{entity.name}</span>
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
                onclick={(e) => { e.stopPropagation(); handleAddChild(entity.id); }}
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
        {#if hasChildren && isExpanded}
          {#each node.children as child (child.entity.id)}
            {@render entityRow(child, depth + 1)}
          {/each}
        {/if}
      {/snippet}

      {#each tree as node (node.entity.id)}
        {@render entityRow(node, 0)}
      {/each}
    </ul>
  {/if}

  {#if contextMenu}
    {@const cid = contextMenu.entityId}
    {@const cname = entities.find((e) => e.id === cid)?.name ?? ''}
    <div
      class="context-menu"
      style="left: {contextMenu.x}px; top: {contextMenu.y}px;"
      role="menu"
    >
      <button role="menuitem" onclick={() => { startRename(cid, cname); }}>Rename</button>
      <button role="menuitem" onclick={() => { duplicateEntity(cid); closeContextMenu(); }}>Duplicate</button>
      <button role="menuitem" onclick={() => { handleAddChild(cid); }}>Add Child</button>
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
    padding: 3px 8px; /* left padding overridden inline for depth */
    cursor: pointer;
    font-size: 12px;
    color: var(--color-text, #ccc);
    border: 1px solid transparent;
    user-select: none;
  }

  .entity-row:hover { background: var(--color-bgHeader, #2d2d2d); }
  .entity-row.selected { background: var(--color-accent, #007acc); color: #fff; }
  .entity-row:focus-visible { outline: 1px solid var(--color-accent, #007acc); outline-offset: -1px; }
  .entity-row.drop-target { outline: 1px dashed var(--color-accent, #007acc); outline-offset: -1px; background: rgba(0, 122, 204, 0.15); }
  .entity-row.reparent-target {
    outline: 2px dashed var(--accent-color, #7c5cbf);
    outline-offset: -2px;
  }

  .entity-chevron {
    display: flex;
    align-items: center;
    color: var(--color-textDim, #666);
    flex-shrink: 0;
    width: 14px;
    cursor: default;
    transition: transform 0.1s ease;
  }
  .entity-chevron.has-children { cursor: pointer; }
  .entity-chevron.has-children.expanded svg { transform: rotate(90deg); }
  .entity-chevron svg { transition: transform 0.1s ease; }
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
