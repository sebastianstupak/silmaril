<!-- engine/editor/src/lib/components/FileTreeNode.svelte -->
<script lang="ts">
  import type { TreeNode } from '$lib/stores/file-explorer';
  import {
    expandDir,
    collapseDir,
    setSelected,
  } from '$lib/stores/file-explorer';
  import FileTreeNode from './FileTreeNode.svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { t } from '$lib/i18n';
  import { logError } from '$lib/stores/console';

  let {
    node,
    depth = 0,
    showIgnored = false,
    selected = null,
    gitStatus = {},
    expanded = new Set<string>(),
  }: {
    node: TreeNode;
    depth?: number;
    showIgnored?: boolean;
    selected?: string | null;
    gitStatus?: Record<string, string>;
    expanded?: Set<string>;
  } = $props();

  let isExpanded = $derived(expanded.has(node.path));
  let isSelected = $derived(selected === node.path);
  let status = $derived(gitStatus[node.path] ?? node.git_status ?? null);

  let renaming = $state(false);
  let newName = $state('');

  const GIT_COLORS: Record<string, string> = {
    modified: 'var(--color-warn)',
    untracked: 'var(--color-success)',
    deleted: 'var(--color-error)',
    staged: 'var(--color-info)',
  };

  function handleClick() {
    setSelected(node.path);
    if (node.kind === 'dir') {
      isExpanded ? collapseDir(node.path) : expandDir(node.path);
    } else {
      invoke('open_in_editor', { path: node.path }).catch((e: unknown) => {
        logError(`Could not open file: ${e}`);
      });
    }
  }

  async function startRename() {
    newName = node.name;
    renaming = true;
  }

  async function confirmRename() {
    if (!newName.trim() || newName === node.name) { renaming = false; return; }
    const dir = node.path.substring(0, node.path.length - node.name.length);
    const to = dir + newName;
    await invoke('rename_path', { from: node.path, to }).catch((e: unknown) => logError(String(e)));
    renaming = false;
  }

  function cancelRename() { renaming = false; }

  async function handleContextMenu(e: MouseEvent) {
    e.preventDefault();
    const action = await showContextMenu(node);
    if (!action) return;
    if (action === 'rename') startRename();
    if (action === 'delete') await invoke('delete_path', { path: node.path }).catch((e: unknown) => logError(String(e)));
    if (action === 'copy_path') navigator.clipboard.writeText(node.path);
    if (action === 'new_file') {
      const name = prompt(t('explorer.new_file'));
      if (name) await invoke('create_file', { path: node.path + '/' + name }).catch((e: unknown) => logError(String(e)));
    }
    if (action === 'new_folder') {
      const name = prompt(t('explorer.new_folder'));
      if (name) await invoke('create_dir', { path: node.path + '/' + name }).catch((e: unknown) => logError(String(e)));
    }
    if (action === 'reveal') {
      // Deferred: OS reveal requires a Tauri shell command — implement in follow-up
      logError('Reveal in Explorer not yet implemented');
    }
  }

  // Placeholder — replaced with a proper context menu component in a follow-up
  async function showContextMenu(_node: TreeNode): Promise<string | null> {
    const options = ['rename', 'delete', 'copy_path', 'reveal'];
    if (_node.kind === 'dir') options.unshift('new_file', 'new_folder');
    return prompt('Action: ' + options.join(', ')) ?? null;
  }

  let visibleChildren = $derived(
    node.children
      ? node.children.filter((c) => showIgnored || !c.ignored)
      : []
  );
</script>

{#if !node.ignored || showIgnored}
  <div
    class="tree-node"
    class:selected={isSelected}
    style:padding-left="{depth * 12 + 4}px"
    style:opacity={node.ignored ? 0.5 : 1}
    onclick={handleClick}
    oncontextmenu={handleContextMenu}
    role="treeitem"
    aria-selected={isSelected}
    aria-expanded={node.kind === 'dir' ? isExpanded : undefined}
  >
    <!-- Chevron for dirs -->
    <span class="chevron">
      {#if node.kind === 'dir'}
        {isExpanded ? '▼' : '▶'}
      {:else}
        &nbsp;
      {/if}
    </span>

    <!-- Icon -->
    <span class="icon">{node.kind === 'dir' ? '📁' : '📄'}</span>

    <!-- Name or inline rename input -->
    {#if renaming}
      <input
        class="rename-input"
        bind:value={newName}
        onkeydown={(e) => { if (e.key === 'Enter') confirmRename(); if (e.key === 'Escape') cancelRename(); }}
        onblur={cancelRename}
        autofocus
        onclick={(e) => e.stopPropagation()}
      />
    {:else}
      <span class="name">{node.name}</span>
    {/if}

    <!-- Git badge -->
    {#if status}
      <span class="git-badge" style:color={GIT_COLORS[status] ?? 'inherit'}>
        {status[0].toUpperCase()}
      </span>
    {/if}
  </div>

  <!-- Children (recursive) -->
  {#if node.kind === 'dir' && isExpanded && node.children !== null}
    {#each visibleChildren as child (child.path)}
      <FileTreeNode
        node={child}
        depth={depth + 1}
        {showIgnored}
        {selected}
        {gitStatus}
        {expanded}
      />
    {/each}
  {/if}
{/if}

<style>
  .tree-node {
    display: flex;
    align-items: center;
    gap: 4px;
    padding-top: 2px;
    padding-bottom: 2px;
    padding-right: 8px;
    cursor: pointer;
    user-select: none;
    border-radius: 3px;
    font-size: 13px;
    color: var(--color-text, #ccc);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .tree-node:hover { background: var(--color-bgHover, #2a2a2a); }
  .tree-node.selected { background: var(--color-bgSelected, #2a3f5f); color: #fff; }
  .chevron { font-size: 10px; width: 12px; flex-shrink: 0; color: var(--color-textMuted, #888); }
  .icon { font-size: 14px; flex-shrink: 0; }
  .name { overflow: hidden; text-overflow: ellipsis; flex: 1; }
  .git-badge { font-size: 11px; font-weight: 600; flex-shrink: 0; margin-left: auto; }
  .rename-input {
    flex: 1;
    background: var(--color-bgInput, #1a1a1a);
    border: 1px solid var(--color-accent, #4a9eff);
    color: var(--color-text, #ccc);
    font-size: 13px;
    padding: 0 4px;
    border-radius: 2px;
    outline: none;
  }
</style>
