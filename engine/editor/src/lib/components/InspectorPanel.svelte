<!-- src/lib/components/InspectorPanel.svelte -->
<script lang="ts">
  import { onDestroy } from 'svelte';
  import { t } from '$lib/i18n';
  import type { SceneEntity } from '$lib/scene/state';
  import { subscribeSchemas, getSchemas } from '$lib/inspector/schema-store';
  import { setComponentField } from '$lib/scene/commands';
  import type { ComponentSchemas } from '$lib/inspector/schema';
  import F32Field    from '$lib/inspector/widgets/F32Field.svelte';
  import BoolField   from '$lib/inspector/widgets/BoolField.svelte';
  import Vec3Field   from '$lib/inspector/widgets/Vec3Field.svelte';
  import EnumField   from '$lib/inspector/widgets/EnumField.svelte';
  import StringField from '$lib/inspector/widgets/StringField.svelte';

  let { entity = null }: { entity: SceneEntity | null } = $props();

  let schemas: ComponentSchemas = $state(getSchemas());
  let collapsedSections: Record<string, boolean> = $state({});

  // Refresh schemas when store loads (schemas arrive async after startup)
  const unsub = subscribeSchemas(() => { schemas = getSchemas(); });
  onDestroy(unsub);

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
        <button
          class="component-header"
          onclick={() => toggleSection(componentName)}
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
        </button>

        {#if !collapsedSections[componentName]}
          <div class="component-body">
            {#if schema}
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

    <button class="add-component-btn">+ {t('inspector.add_component')}</button>
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

  .add-component-btn {
    margin: 8px;
    padding: 6px 12px;
    background: var(--color-bg, #1e1e1e);
    border: 1px dashed var(--color-border, #404040);
    border-radius: 4px;
    color: var(--color-textMuted, #999);
    font-size: 11px;
    cursor: pointer;
    text-align: center;
  }

  .add-component-btn:hover {
    border-color: var(--color-accent, #007acc);
    color: var(--color-accent, #007acc);
  }
</style>
