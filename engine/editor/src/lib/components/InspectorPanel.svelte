<!-- src/lib/components/InspectorPanel.svelte -->
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { t } from '$lib/i18n';
  import type { SceneEntity } from '$lib/scene/state';
  import { subscribeSchemas, getSchemas } from '$lib/inspector/schema-store';
  import { setComponentField, addComponent, removeComponent } from '$lib/scene/commands';
  import type { ComponentSchemas } from '$lib/inspector/schema';
  import F32Field    from '$lib/inspector/widgets/F32Field.svelte';
  import BoolField   from '$lib/inspector/widgets/BoolField.svelte';
  import Vec3Field   from '$lib/inspector/widgets/Vec3Field.svelte';
  import EnumField   from '$lib/inspector/widgets/EnumField.svelte';
  import StringField from '$lib/inspector/widgets/StringField.svelte';
  import { assignMesh } from '$lib/api';
  import { getActiveTemplatePath } from '$lib/stores/undo-history';
  import { getAssets, subscribeAssets, type AssetEntry } from '$lib/stores/assets';

  let { entity = null }: { entity: SceneEntity | null } = $props();

  let schemas: ComponentSchemas = $state(getSchemas());
  let collapsedSections: Record<string, boolean> = $state({});
  let addingComponent = $state(false);
  let componentFilter = $state('');

  // Refresh schemas when store loads (schemas arrive async after startup)
  const unsub = subscribeSchemas(() => { schemas = getSchemas(); });
  onDestroy(unsub);

  // Mesh asset list for the MeshRenderer picker
  let meshAssets = $state<AssetEntry[]>(getAssets().filter((a) => a.assetType === 'mesh'));
  let unsubAssets: (() => void) | null = null;
  onMount(() => {
    unsubAssets = subscribeAssets((list) => {
      meshAssets = list.filter((a) => a.assetType === 'mesh');
    });
  });
  onDestroy(() => unsubAssets?.());

  async function onMeshChange(event: Event): Promise<void> {
    if (!entity) return;
    const meshPath = (event.target as HTMLSelectElement).value;
    const templatePath = getActiveTemplatePath();
    if (!templatePath) return;
    await assignMesh(entity.id, templatePath, meshPath);
  }

  function toggleSection(name: string) {
    collapsedSections[name] = !collapsedSections[name];
  }

  function handleFieldChange(componentName: string, fieldName: string, value: unknown) {
    if (!entity) return;
    setComponentField(entity.id, componentName, fieldName, value);
  }

  function fieldValue(componentName: string, fieldName: string): unknown {
    return entity?.componentValues?.[componentName]?.[fieldName];
  }
</script>

<div class="inspector">
  {#if !entity}
    <p class="inspector-empty">{t('inspector.no_selection')}</p>
  {:else}
    <!-- Header -->
    <div class="inspector-header">
      <span class="inspector-entity-icon">
        <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
          <path d="M8 1l6.5 3.75v6.5L8 15l-6.5-3.75v-6.5L8 1z" stroke="currentColor" stroke-width="1" fill="none"/>
        </svg>
      </span>
      <span class="inspector-entity-name">{entity.name}</span>
      <span class="inspector-entity-id">#{entity.id}</span>
    </div>

    <div class="inspector-section-label">{t('inspector.components')}</div>

    <!-- Components -->
    {#each entity.components as componentName (componentName)}
      {@const schema = schemas[componentName]}
      <div class="component-section">
        <!-- svelte-ignore a11y-no-static-element-interactions -->
        <div
          class="component-header"
          role="button"
          tabindex="0"
          onclick={() => toggleSection(componentName)}
          onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') toggleSection(componentName); }}
          aria-expanded={!collapsedSections[componentName]}
        >
          <span class="component-chevron" class:collapsed={collapsedSections[componentName]}>
            <svg width="10" height="10" viewBox="0 0 16 16" fill="currentColor">
              <path d="M4 6l4 4 4-4" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </span>
          <span class="component-name">{schema?.label ?? componentName}</span>
          {#if schema}
            <span class="component-category">{schema.category}</span>
          {/if}
          <button
            class="remove-component-btn"
            title="Remove {componentName}"
            onclick={(e) => { e.stopPropagation(); removeComponent(entity.id, componentName); }}
          >✕</button>
        </div>

        {#if !collapsedSections[componentName]}
          <div class="component-body">
            {#if componentName === 'MeshRenderer'}
              <!-- Mesh picker row -->
              <div class="field-row">
                <span class="field-label">Mesh</span>
                <select
                  class="mesh-select"
                  value={String(entity.componentValues?.['MeshRenderer']?.['mesh_path'] ?? 'builtin://cube')}
                  onchange={onMeshChange}
                >
                  <optgroup label="Primitives">
                    <option value="builtin://cube">Cube</option>
                    <option value="builtin://sphere">Sphere</option>
                    <option value="builtin://plane">Plane</option>
                    <option value="builtin://cylinder">Cylinder</option>
                    <option value="builtin://capsule">Capsule</option>
                  </optgroup>
                  {#if meshAssets.length > 0}
                    <optgroup label="Models">
                      {#each meshAssets as asset (asset.path)}
                        <option value={asset.path}>{asset.filename}</option>
                      {/each}
                    </optgroup>
                  {/if}
                </select>
              </div>
            {:else if schema}
              {#each schema.fields as field (field.name)}
                {@const ft = field.field_type}
                {@const val = fieldValue(componentName, field.name)}
                <div class="field-wrapper">
                  {#if ft.kind === 'f32'}
                    <F32Field
                      label={field.label}
                      value={val as number ?? 0}
                      min={ft.min}
                      max={ft.max}
                      step={ft.step ?? 0.1}
                      onchange={(v) => handleFieldChange(componentName, field.name, v)}
                    />
                  {:else if ft.kind === 'bool'}
                    <BoolField
                      label={field.label}
                      value={val as boolean ?? false}
                      onchange={(v) => handleFieldChange(componentName, field.name, v)}
                    />
                  {:else if ft.kind === 'vec3'}
                    <Vec3Field
                      label={field.label}
                      value={val as { x: number; y: number; z: number } ?? { x: 0, y: 0, z: 0 }}
                      onchange={(v) => handleFieldChange(componentName, field.name, v)}
                    />
                  {:else if ft.kind === 'enum'}
                    <EnumField
                      label={field.label}
                      value={val as string ?? ''}
                      options={ft.options}
                      onchange={(v) => handleFieldChange(componentName, field.name, v)}
                    />
                  {:else if ft.kind === 'string'}
                    <StringField
                      label={field.label}
                      value={val as string ?? ''}
                      onchange={(v) => handleFieldChange(componentName, field.name, v)}
                    />
                  {:else}
                    <div class="field-row">
                      <span class="field-label">{field.label}</span>
                      <span class="field-value">{JSON.stringify(val)}</span>
                    </div>
                  {/if}
                </div>
              {/each}
            {:else}
              <!-- No schema registered — show raw values if any -->
              {#each Object.entries(entity.componentValues?.[componentName] ?? {}) as [k, v]}
                <div class="field-row">
                  <span class="field-label">{k}</span>
                  <span class="field-value">{JSON.stringify(v)}</span>
                </div>
              {/each}
              {#if !entity.componentValues?.[componentName] || Object.keys(entity.componentValues[componentName]).length === 0}
                <div class="field-row">
                  <span class="field-label field-label--dim">no schema</span>
                </div>
              {/if}
            {/if}
          </div>
        {/if}
      </div>
    {/each}

    <!-- Add Component picker -->
    <div class="add-component-section">
      {#if !addingComponent}
        <button class="add-component-btn" onclick={() => { addingComponent = true; componentFilter = ''; }}>
          + Add Component…
        </button>
      {:else}
        <div class="component-picker">
          <input
            class="component-filter-input"
            type="text"
            placeholder="Filter components…"
            bind:value={componentFilter}
            autofocus
            onkeydown={(e) => { if (e.key === 'Escape') addingComponent = false; }}
          />
          <ul class="component-picker-list">
            {#each Object.values(schemas).filter(s =>
              !entity.components.includes(s.name) &&
              (componentFilter === '' || s.name.toLowerCase().includes(componentFilter.toLowerCase()))
            ) as schema (schema.name)}
              <li>
                <button
                  onclick={() => {
                    addComponent(entity.id, schema.name);
                    addingComponent = false;
                  }}
                >{schema.name}</button>
              </li>
            {/each}
          </ul>
        </div>
        <!-- backdrop to close picker -->
        <div
          class="picker-backdrop"
          role="none"
          onclick={() => { addingComponent = false; }}
        ></div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .inspector {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow-y: auto;
  }

  .inspector-empty {
    color: var(--color-textDim, #666);
    font-style: italic;
    padding: 12px 8px;
    font-size: 12px;
    text-align: center;
  }

  .inspector-header {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px;
    border-bottom: 1px solid var(--color-border, #404040);
    flex-shrink: 0;
  }

  .inspector-entity-icon {
    display: flex;
    align-items: center;
    color: var(--color-accent, #007acc);
    flex-shrink: 0;
  }

  .inspector-entity-name {
    font-size: 13px;
    font-weight: 600;
    color: var(--color-text, #ccc);
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .inspector-entity-id {
    font-size: 10px;
    color: var(--color-textDim, #666);
    flex-shrink: 0;
  }

  .inspector-section-label {
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--color-textDim, #666);
    padding: 8px 8px 4px;
  }

  .component-section {
    border-bottom: 1px solid var(--color-border, #404040);
  }

  .component-header {
    display: flex;
    align-items: center;
    gap: 4px;
    width: 100%;
    padding: 6px 8px;
    background: var(--color-bgHeader, #2d2d2d);
    border: none;
    cursor: pointer;
    color: var(--color-text, #ccc);
    font-size: 12px;
    font-weight: 500;
    text-align: left;
  }

  .component-header:hover {
    background: var(--color-bg, #1e1e1e);
  }

  .component-chevron {
    display: flex;
    align-items: center;
    transition: transform 0.15s ease;
    color: var(--color-textMuted, #999);
  }

  .component-chevron.collapsed {
    transform: rotate(-90deg);
  }

  .component-name {
    flex: 1;
  }

  .component-category {
    font-size: 10px;
    color: var(--color-textDim, #555);
    font-weight: 400;
  }

  .component-body {
    padding: 4px 8px 8px 8px;
  }

  .field-wrapper {
    padding: 1px 0;
  }

  .field-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 2px 0;
    font-size: 11px;
  }

  .field-label {
    color: var(--color-textMuted, #999);
    min-width: 70px;
    flex-shrink: 0;
  }

  .field-label--dim {
    font-style: italic;
    color: var(--color-textDim, #555);
  }

  .field-value {
    color: var(--color-text, #ccc);
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: monospace;
    font-size: 10px;
  }

  .remove-component-btn {
    all: unset;
    margin-left: auto;
    color: var(--color-textDim, #585b70);
    cursor: pointer;
    font-size: 11px;
    padding: 0 3px;
    border-radius: 2px;
    line-height: 1;
    flex-shrink: 0;
  }
  .remove-component-btn:hover { color: #f38ba8; }

  .add-component-section {
    padding: 4px 6px 6px;
    position: relative;
  }

  .add-component-btn {
    all: unset;
    display: block;
    width: 100%;
    background: none;
    border: 1px dashed var(--color-border, #45475a);
    color: var(--color-textDim, #6c7086);
    border-radius: 4px;
    padding: 5px 8px;
    cursor: pointer;
    font-size: 11px;
    text-align: left;
    box-sizing: border-box;
    font-family: inherit;
  }
  .add-component-btn:hover { border-color: var(--color-accent, #89b4fa); color: var(--color-text, #cdd6f4); }

  .component-picker {
    background: var(--color-bgPanel, #1e1e2e);
    border: 1px solid var(--color-accent, #89b4fa);
    border-radius: 4px;
    overflow: hidden;
    position: relative;
    z-index: 100;
  }

  .component-filter-input {
    width: 100%;
    background: var(--color-bg, #181825);
    border: none;
    border-bottom: 1px solid var(--color-border, #313244);
    padding: 5px 8px;
    font-size: 11px;
    color: var(--color-text, #cdd6f4);
    outline: none;
    box-sizing: border-box;
    font-family: inherit;
  }

  .component-picker-list {
    list-style: none;
    margin: 0;
    padding: 3px 0;
    max-height: 160px;
    overflow-y: auto;
  }

  .component-picker-list button {
    all: unset;
    display: block;
    width: 100%;
    padding: 4px 10px;
    font-size: 11px;
    color: var(--color-textMuted, #a6adc8);
    cursor: pointer;
    box-sizing: border-box;
  }
  .component-picker-list button:hover {
    background: var(--color-bgHover, #313244);
    color: var(--color-text, #cdd6f4);
  }

  .picker-backdrop { position: fixed; inset: 0; z-index: 99; }

  .mesh-select {
    flex: 1;
    background: var(--color-bg, #1e1e1e);
    border: 1px solid var(--color-border, #404040);
    border-radius: 3px;
    padding: 2px 4px;
    font-size: 11px;
    color: var(--color-text, #ccc);
    outline: none;
    font-family: inherit;
    min-width: 0;
  }
  .mesh-select:focus { border-color: var(--color-accent, #007acc); }
</style>
