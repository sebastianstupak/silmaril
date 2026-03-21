<!-- src/lib/inspector/widgets/F32Field.svelte -->
<script lang="ts">
  let {
    label,
    value = 0,
    min,
    max,
    step = 0.1,
    onchange,
  }: {
    label: string;
    value?: number;
    min?: number;
    max?: number;
    step?: number;
    onchange?: (v: number) => void;
  } = $props();

  function handleInput(e: Event) {
    const v = parseFloat((e.target as HTMLInputElement).value);
    if (!isNaN(v)) onchange?.(v);
  }
</script>

<div class="field-row">
  <label class="field-label">{label}</label>
  <div class="field-controls">
    {#if min !== undefined && max !== undefined}
      <input
        type="range"
        {min}
        {max}
        {step}
        value={value}
        oninput={handleInput}
        class="field-slider"
      />
    {/if}
    <input
      type="number"
      value={value}
      {step}
      oninput={handleInput}
      class="field-number"
    />
  </div>
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
  .field-controls {
    display: flex;
    align-items: center;
    gap: 4px;
    flex: 1;
    min-width: 0;
  }
  .field-slider {
    flex: 1;
    min-width: 0;
    accent-color: var(--color-accent, #007acc);
  }
  .field-number {
    width: 52px;
    flex-shrink: 0;
    background: var(--color-bgInput, #1a1a1a);
    border: 1px solid var(--color-border, #404040);
    border-radius: 3px;
    color: var(--color-text, #ccc);
    font-size: 11px;
    padding: 1px 4px;
    text-align: right;
  }
  .field-number:focus {
    outline: none;
    border-color: var(--color-accent, #007acc);
  }
</style>
