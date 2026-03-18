<script>
  // Auto-renders panel UI from module schema definitions
  export let schema = { title: '', sections: [] };
</script>

<div class="schema-panel">
  <h3>{schema.title}</h3>
  {#each schema.sections as section}
    <div class="section">
      <h4>{section.name}</h4>
      <!-- TODO: Render fields based on type (slider, dropdown, checkbox, etc.) -->
      <p class="placeholder">Schema rendering not yet implemented</p>
    </div>
  {/each}
</div>

<style>
  .schema-panel { padding: 8px; }
  .section { margin-bottom: 12px; }
  h3 { font-size: 14px; margin: 0 0 8px; color: #ccc; }
  h4 { font-size: 12px; margin: 0 0 4px; color: #999; }
  .placeholder { color: #666; font-style: italic; font-size: 12px; }
</style>
