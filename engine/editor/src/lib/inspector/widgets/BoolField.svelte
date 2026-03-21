<!-- src/lib/inspector/widgets/BoolField.svelte -->
<script lang="ts">
  let {
    label,
    value = false,
    onchange,
  }: {
    label: string;
    value?: boolean;
    onchange?: (v: boolean) => void;
  } = $props();
</script>

<div class="field-row">
  <label class="field-label">{label}</label>
  <input
    type="checkbox"
    checked={value}
    onchange={(e) => onchange?.((e.target as HTMLInputElement).checked)}
    class="field-checkbox"
  />
</div>

<style>
  .field-row {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 2px 0;
    font-size: 11px;
  }
  .field-label {
    color: var(--color-textMuted, #999);
    min-width: 70px;
    flex-shrink: 0;
  }
  .field-checkbox {
    accent-color: var(--color-accent, #007acc);
    cursor: pointer;
  }
</style>
