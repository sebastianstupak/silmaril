<script lang="ts">
  import { onMount } from 'svelte';
  import InspectorPanel from '$lib/components/InspectorPanel.svelte';
  import {
    getEditorContext,
    getSelectedEntity,
    subscribeContext,
  } from '$lib/stores/editor-context';
  import type { EntityInfo } from '$lib/api';

  let entity: EntityInfo | null = $state(getSelectedEntity());

  onMount(() => {
    return subscribeContext(() => {
      entity = getSelectedEntity();
    });
  });
</script>

<InspectorPanel {entity} />
