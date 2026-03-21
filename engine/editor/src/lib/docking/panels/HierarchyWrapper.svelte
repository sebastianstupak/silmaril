<script lang="ts">
  import { onMount } from 'svelte';
  import HierarchyPanel from '$lib/components/HierarchyPanel.svelte';
  import {
    getEditorContext,
    getSelectedEntityId,
    setSelectedEntityId,
    subscribeContext,
  } from '$lib/stores/editor-context';
  import { setSelectedEntity } from '$lib/api';

  let entities = $state(getEditorContext().entities);
  let selectedId = $state(getEditorContext().selectedEntityId);

  onMount(() => {
    return subscribeContext(() => {
      const ctx = getEditorContext();
      entities = ctx.entities;
      selectedId = ctx.selectedEntityId;
    });
  });

  // Mirror selection to the Rust viewport renderer.
  $effect(() => {
    return subscribeContext(() => {
      setSelectedEntity(getSelectedEntityId()).catch((e) => {
        console.warn('[silmaril] setSelectedEntity failed:', e);
      });
    });
  });

  function handleSelect(id: number) {
    setSelectedEntityId(id);
  }
</script>

<div class="panel-opaque"><HierarchyPanel {entities} {selectedId} onSelect={handleSelect} /></div>
<style>.panel-opaque { width:100%; height:100%; background: var(--color-bgPanel, #252525); }</style>
