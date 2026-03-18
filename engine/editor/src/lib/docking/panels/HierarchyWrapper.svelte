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

<HierarchyPanel {entities} {selectedId} onSelect={handleSelect} />
