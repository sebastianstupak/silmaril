<script lang="ts">
  import { onMount } from 'svelte';
  import HierarchyPanel from '$lib/components/HierarchyPanel.svelte';
  import {
    getEditorContext,
    setSelectedEntityId,
    subscribeContext,
  } from '$lib/stores/editor-context';

  let entities = $state(getEditorContext().entities);
  let selectedId = $state(getEditorContext().selectedEntityId);

  onMount(() => {
    return subscribeContext(() => {
      const ctx = getEditorContext();
      entities = ctx.entities;
      selectedId = ctx.selectedEntityId;
    });
  });

  function handleSelect(id: number) {
    setSelectedEntityId(id);
  }
</script>

<div class="panel-opaque"><HierarchyPanel {entities} {selectedId} onSelect={handleSelect} /></div>
<style>.panel-opaque { width:100%; height:100%; background: var(--color-bgPanel, #252525); }</style>
