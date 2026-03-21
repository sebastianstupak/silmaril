<!-- src/lib/inspector/widgets/StringField.svelte -->
<script lang="ts">
  let {
    label,
    value = '',
    onchange,
  }: {
    label: string;
    value?: string;
    onchange?: (v: string) => void;
  } = $props();
</script>

<div class="field-row">
  <label class="field-label">{label}</label>
  <input
    type="text"
    value={value}
    oninput={(e) => onchange?.((e.target as HTMLInputElement).value)}
    class="field-input"
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
  .field-input {
    flex: 1;
    background: var(--color-bgInput, #1a1a1a);
    border: 1px solid var(--color-border, #404040);
    border-radius: 3px;
    color: var(--color-text, #ccc);
    font-size: 11px;
    padding: 1px 4px;
  }
  .field-input:focus {
    outline: none;
    border-color: var(--color-accent, #007acc);
  }
</style>
