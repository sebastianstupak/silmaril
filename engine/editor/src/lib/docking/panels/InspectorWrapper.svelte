<script lang="ts">
  import { onMount } from 'svelte';
  import InspectorPanel from '$lib/components/InspectorPanel.svelte';
  import {
    getEditorContext,
    getSelectedEntity,
    subscribeContext,
  } from '$lib/stores/editor-context';
  import type { SceneEntity } from '$lib/scene/state';

  let entity: SceneEntity | null = $state(getSelectedEntity());

  onMount(() => {
    return subscribeContext(() => {
      entity = getSelectedEntity();
    });
  });
</script>

<div class="panel-opaque"><InspectorPanel {entity} /></div>
<style>.panel-opaque { width:100%; height:100%; background: var(--color-bgPanel, #252525); }</style>
