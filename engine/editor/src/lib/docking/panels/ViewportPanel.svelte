<script lang="ts">
  import { onMount } from 'svelte';
  import { t } from '$lib/i18n';
  import {
    getEditorContext,
    setSelectedEntityId,
    subscribeContext,
  } from '$lib/stores/editor-context';
  import {
    getViewportFrame,
    pickViewportEntity,
    type ViewportEntity,
    type ViewportCamera,
  } from '$lib/api';

  // Entity colour palette — matches the Rust side
  const ENTITY_COLORS = [
    '#e06c75', '#61afef', '#98c379', '#e5c07b',
    '#c678dd', '#56b6c2', '#d19a66', '#be5046',
  ];

  let containerEl: HTMLDivElement | undefined = $state(undefined);
  let viewportWidth = $state(800);
  let viewportHeight = $state(600);
  let svgContent = $state('');
  let loading = $state(true);

  // Camera state
  let camera: ViewportCamera = $state({ offset_x: 0, offset_y: 0, zoom: 1 });
  let isPanning = $state(false);
  let panStartX = 0;
  let panStartY = 0;
  let panStartOffsetX = 0;
  let panStartOffsetY = 0;

  // Entity data pulled from editor context
  let entities = $state(getEditorContext().entities);
  let selectedEntityId: number | null = $state(getEditorContext().selectedEntityId);

  onMount(() => {
    const unsub = subscribeContext(() => {
      const ctx = getEditorContext();
      entities = ctx.entities;
      selectedEntityId = ctx.selectedEntityId;
      requestFrame();
    });

    // Observe container size
    if (containerEl) {
      const observer = new ResizeObserver((entries) => {
        for (const entry of entries) {
          viewportWidth = Math.round(entry.contentRect.width) || 800;
          viewportHeight = Math.round(entry.contentRect.height) || 600;
        }
        requestFrame();
      });
      observer.observe(containerEl);

      // Initial frame
      requestFrame();

      return () => {
        unsub();
        observer.disconnect();
      };
    }

    requestFrame();
    return unsub;
  });

  /** Build the viewport entity list from editor context entities. */
  function buildViewportEntities(): ViewportEntity[] {
    return entities.map((e, i) => ({
      id: e.id,
      name: e.name,
      // Distribute entities in a circle pattern for visual interest
      x: 0.5 + 0.3 * Math.cos((2 * Math.PI * i) / Math.max(entities.length, 1)),
      y: 0.5 + 0.3 * Math.sin((2 * Math.PI * i) / Math.max(entities.length, 1)),
      color: ENTITY_COLORS[i % ENTITY_COLORS.length],
    }));
  }

  /** Request a new SVG frame from the backend. */
  async function requestFrame() {
    try {
      const viewEntities = buildViewportEntities();
      const svg = await getViewportFrame({
        width: viewportWidth,
        height: viewportHeight,
        selected_entity_id: selectedEntityId,
        camera,
        entities: viewEntities,
      });
      svgContent = svg;
      loading = false;
    } catch {
      loading = false;
    }
  }

  /** Handle click on the viewport to pick entities. */
  async function handleClick(event: MouseEvent) {
    if (isPanning) return;
    const rect = containerEl?.getBoundingClientRect();
    if (!rect) return;

    const clickX = event.clientX - rect.left;
    const clickY = event.clientY - rect.top;

    const viewEntities = buildViewportEntities();
    const entityId = await pickViewportEntity({
      click_x: clickX,
      click_y: clickY,
      width: viewportWidth,
      height: viewportHeight,
      entities: viewEntities,
      camera,
    });

    setSelectedEntityId(entityId);
  }

  /** Handle mouse wheel for zoom. */
  function handleWheel(event: WheelEvent) {
    event.preventDefault();
    const zoomFactor = event.deltaY > 0 ? 0.9 : 1.1;
    camera = { ...camera, zoom: Math.max(0.1, Math.min(10, camera.zoom * zoomFactor)) };
    requestFrame();
  }

  /** Start panning on middle-click or ctrl+left-click. */
  function handleMouseDown(event: MouseEvent) {
    if (event.button === 1 || (event.button === 0 && event.ctrlKey)) {
      event.preventDefault();
      isPanning = true;
      panStartX = event.clientX;
      panStartY = event.clientY;
      panStartOffsetX = camera.offset_x;
      panStartOffsetY = camera.offset_y;
    }
  }

  function handleMouseMove(event: MouseEvent) {
    if (!isPanning) return;
    const dx = (event.clientX - panStartX) / camera.zoom;
    const dy = (event.clientY - panStartY) / camera.zoom;
    camera = { ...camera, offset_x: panStartOffsetX + dx, offset_y: panStartOffsetY + dy };
    requestFrame();
  }

  function handleMouseUp() {
    isPanning = false;
  }

  function resetCamera() {
    camera = { offset_x: 0, offset_y: 0, zoom: 1 };
    requestFrame();
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_noninteractive_element_interactions -->
<div
  class="viewport-container"
  bind:this={containerEl}
  role="application"
  aria-label={t('panel.viewport')}
  tabindex="-1"
  onclick={handleClick}
  onwheel={handleWheel}
  onmousedown={handleMouseDown}
  onmousemove={handleMouseMove}
  onmouseup={handleMouseUp}
  onmouseleave={handleMouseUp}
>
  {#if loading}
    <div class="viewport-loading">
      <p>{t('viewport.loading')}</p>
    </div>
  {:else}
    <!-- eslint-disable-next-line svelte/no-at-html-tags -->
    {@html svgContent}
  {/if}

  <!-- HUD overlay -->
  <div class="viewport-hud">
    <span class="hud-zoom" title={t('viewport.zoom')}>
      {Math.round(camera.zoom * 100)}%
    </span>
    <button class="hud-btn" onclick={resetCamera} title={t('viewport.reset_camera')}>
      &#8634;
    </button>
  </div>
</div>

<style>
  .viewport-container {
    position: relative;
    width: 100%;
    height: 100%;
    overflow: hidden;
    cursor: crosshair;
    user-select: none;
    background: #1a1a2e;
  }

  .viewport-container :global(svg) {
    display: block;
    width: 100%;
    height: 100%;
  }

  .viewport-loading {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--color-textDim, #666);
  }

  .viewport-hud {
    position: absolute;
    bottom: 8px;
    right: 8px;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 3px 8px;
    background: rgba(0, 0, 0, 0.55);
    border-radius: 4px;
    font-size: 11px;
    color: #aaa;
    pointer-events: auto;
  }

  .hud-zoom {
    min-width: 36px;
    text-align: center;
  }

  .hud-btn {
    background: none;
    border: 1px solid #555;
    border-radius: 3px;
    color: #aaa;
    font-size: 13px;
    padding: 1px 5px;
    cursor: pointer;
    line-height: 1;
  }

  .hud-btn:hover {
    color: #fff;
    border-color: #888;
  }
</style>
