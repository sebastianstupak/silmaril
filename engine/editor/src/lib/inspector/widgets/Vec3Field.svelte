<!-- src/lib/inspector/widgets/Vec3Field.svelte -->
<script lang="ts">
  import F32Field from './F32Field.svelte';

  let {
    label,
    value = { x: 0, y: 0, z: 0 },
    onchange,
  }: {
    label: string;
    value?: { x: number; y: number; z: number };
    onchange?: (v: { x: number; y: number; z: number }) => void;
  } = $props();

  function update(axis: 'x' | 'y' | 'z', v: number) {
    onchange?.({ ...value, [axis]: v });
  }
</script>

<div class="vec3-group">
  <div class="vec3-label">{label}</div>
  <div class="vec3-axes">
    <F32Field label="X" value={value.x} onchange={(v) => update('x', v)} />
    <F32Field label="Y" value={value.y} onchange={(v) => update('y', v)} />
    <F32Field label="Z" value={value.z} onchange={(v) => update('z', v)} />
  </div>
</div>

<style>
  .vec3-group {
    padding: 2px 0 4px;
  }
  .vec3-label {
    font-size: 11px;
    color: var(--color-textMuted, #999);
    margin-bottom: 2px;
  }
  .vec3-axes {
    display: flex;
    flex-direction: column;
    gap: 1px;
    padding-left: 8px;
  }
</style>
