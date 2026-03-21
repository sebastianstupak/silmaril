<!-- engine/editor/src/lib/docking/panels/AssetsPanel.svelte -->
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { getAssets, subscribeAssets, type AssetEntry } from '$lib/stores/assets';
  import { fuzzyScore } from '$lib/omnibar/fuzzy';

  type Group = 'texture' | 'mesh' | 'audio' | 'config' | 'unknown';
  const GROUP_LABELS: Record<Group, string> = {
    texture: 'Textures',
    mesh: 'Meshes',
    audio: 'Audio',
    config: 'Config',
    unknown: 'Other',
  };
  const GROUP_ORDER: Group[] = ['texture', 'mesh', 'audio', 'config', 'unknown'];

  let assets = $state<AssetEntry[]>(getAssets());
  let filter = $state('');
  let collapsed = $state<Set<Group>>(new Set());
  let toast = $state('');
  let toastTimer: ReturnType<typeof setTimeout> | null = null;

  let unsub: (() => void) | null = null;
  onMount(() => {
    unsub = subscribeAssets((list) => { assets = list; });
  });
  onDestroy(() => unsub?.());

  let filtered = $derived(
    filter
      ? assets.filter((a) => fuzzyScore(a.filename, filter) >= 0)
      : assets,
  );

  function grouped(type: Group): AssetEntry[] {
    return filtered.filter((a) => a.assetType === type);
  }

  function toggleGroup(g: Group) {
    const next = new Set(collapsed);
    if (next.has(g)) next.delete(g); else next.add(g);
    collapsed = next;
  }

  async function copyPath(path: string) {
    await navigator.clipboard.writeText(path);
    if (toastTimer) clearTimeout(toastTimer);
    toast = 'Path copied';
    toastTimer = setTimeout(() => { toast = ''; }, 1500);
  }
</script>

<div class="assets-panel">
  {#if assets.length === 0}
    <p class="assets-empty">Open a project to browse assets</p>
  {:else}
    <div class="assets-header">
      <input
        class="assets-filter"
        type="text"
        placeholder="Filter assets…"
        bind:value={filter}
      />
    </div>

    <div class="assets-list">
      {#each GROUP_ORDER as group}
        {@const items = grouped(group)}
        {#if items.length > 0}
          <div class="group">
            <button class="group-header" onclick={() => toggleGroup(group)}>
              <span class="group-chevron">{collapsed.has(group) ? '▸' : '▾'}</span>
              <span class="group-label">{GROUP_LABELS[group]}</span>
              <span class="group-count">{items.length}</span>
            </button>
            {#if !collapsed.has(group)}
              {#each items as asset (asset.path)}
                <button
                  class="asset-row"
                  onclick={() => copyPath(asset.path)}
                  title={asset.path}
                >
                  {asset.filename}
                </button>
              {/each}
            {/if}
          </div>
        {/if}
      {/each}
    </div>
  {/if}

  {#if toast}
    <div class="toast">{toast}</div>
  {/if}
</div>

<style>
  .assets-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
    position: relative;
    background: var(--color-bgPanel, #252525);
  }

  .assets-empty {
    color: var(--color-textDim, #666);
    font-style: italic;
    padding: 12px 8px;
    font-size: 12px;
    text-align: center;
  }

  .assets-header {
    padding: 4px 8px;
    border-bottom: 1px solid var(--color-border, #404040);
    flex-shrink: 0;
  }

  .assets-filter {
    width: 100%;
    box-sizing: border-box;
    background: var(--color-bg, #1e1e1e);
    border: 1px solid var(--color-border, #404040);
    border-radius: 3px;
    padding: 3px 6px;
    font-size: 11px;
    color: var(--color-text, #ccc);
    outline: none;
  }

  .assets-filter:focus { border-color: var(--color-accent, #007acc); }

  .assets-list {
    flex: 1;
    overflow-y: auto;
    padding: 2px 0;
  }

  .group-header {
    all: unset;
    display: flex;
    align-items: center;
    gap: 4px;
    width: 100%;
    padding: 3px 8px;
    font-size: 10px;
    color: var(--color-textDim, #666);
    letter-spacing: 0.05em;
    cursor: pointer;
    box-sizing: border-box;
  }

  .group-header:hover { color: var(--color-text, #ccc); }

  .group-chevron { font-size: 9px; width: 10px; }
  .group-label { flex: 1; text-transform: uppercase; }
  .group-count {
    background: var(--color-bg, #1e1e1e);
    padding: 0 4px;
    border-radius: 3px;
    font-size: 10px;
  }

  .asset-row {
    all: unset;
    display: block;
    width: 100%;
    padding: 2px 8px 2px 22px;
    font-size: 11px;
    color: var(--color-textMuted, #999);
    cursor: pointer;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    box-sizing: border-box;
  }

  .asset-row:hover {
    background: var(--color-bgHeader, #2d2d2d);
    color: var(--color-text, #ccc);
  }

  .toast {
    position: absolute;
    bottom: 8px;
    left: 50%;
    transform: translateX(-50%);
    background: var(--color-accent, #007acc);
    color: #fff;
    font-size: 11px;
    padding: 4px 10px;
    border-radius: 4px;
    pointer-events: none;
    white-space: nowrap;
  }
</style>
