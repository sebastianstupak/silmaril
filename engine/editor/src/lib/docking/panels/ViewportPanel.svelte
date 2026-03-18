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
  import {
    getSceneState,
    subscribeScene,
    type SceneTool,
  } from '$lib/scene/state';
  import {
    selectEntity,
    createEntity,
    deleteEntity,
    duplicateEntity,
    moveEntity,
    panCamera,
    orbitCamera,
    zoomCamera,
    focusEntity,
    resetCamera,
    setActiveTool,
    toggleGrid,
    toggleSnapToGrid,
  } from '$lib/scene/commands';

  // Entity colour palette — matches the Rust side
  const ENTITY_COLORS = [
    '#e06c75', '#61afef', '#98c379', '#e5c07b',
    '#c678dd', '#56b6c2', '#d19a66', '#be5046',
  ];

  const TOOL_KEYS: Record<string, SceneTool> = {
    q: 'select',
    w: 'move',
    e: 'rotate',
    r: 'scale',
  };

  let containerEl: HTMLDivElement | undefined = $state(undefined);
  let viewportWidth = $state(800);
  let viewportHeight = $state(600);
  let svgContent = $state('');
  let loading = $state(true);

  // Camera state (viewport-local, synced from scene state)
  let camera: ViewportCamera = $state({ offset_x: 0, offset_y: 0, zoom: 1 });

  // Interaction state
  let isPanning = $state(false);
  let isOrbiting = $state(false);
  let panStartX = 0;
  let panStartY = 0;
  let panStartOffsetX = 0;
  let panStartOffsetY = 0;

  // Scene state mirror
  let entities = $state(getEditorContext().entities);
  let selectedEntityId: number | null = $state(getEditorContext().selectedEntityId);
  let activeTool: SceneTool = $state(getSceneState().activeTool);
  let gridVisible = $state(getSceneState().gridVisible);
  let snapToGrid = $state(getSceneState().snapToGrid);

  onMount(() => {
    const unsub = subscribeScene(() => {
      const ctx = getEditorContext();
      const scene = getSceneState();
      entities = ctx.entities;
      selectedEntityId = ctx.selectedEntityId;
      activeTool = scene.activeTool;
      gridVisible = scene.gridVisible;
      snapToGrid = scene.snapToGrid;

      // Sync camera from scene state to viewport camera
      camera = {
        offset_x: scene.camera.position.x * 50,
        offset_y: -scene.camera.position.y * 50,
        zoom: scene.camera.zoom,
      };
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
    if (isPanning || isOrbiting) return;
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

    selectEntity(entityId);
  }

  /** Handle mouse wheel for zoom. */
  function handleWheel(event: WheelEvent) {
    event.preventDefault();
    const delta = event.deltaY > 0 ? -0.1 : 0.1;
    zoomCamera(delta);
  }

  /** Start panning or orbiting based on button / modifier. */
  function handleMouseDown(event: MouseEvent) {
    // Middle mouse → pan
    if (event.button === 1) {
      event.preventDefault();
      isPanning = true;
      panStartX = event.clientX;
      panStartY = event.clientY;
      panStartOffsetX = camera.offset_x;
      panStartOffsetY = camera.offset_y;
      return;
    }

    // Right mouse drag → orbit
    if (event.button === 2) {
      event.preventDefault();
      isOrbiting = true;
      panStartX = event.clientX;
      panStartY = event.clientY;
      return;
    }

    // Alt + left drag → orbit
    if (event.button === 0 && event.altKey) {
      event.preventDefault();
      isOrbiting = true;
      panStartX = event.clientX;
      panStartY = event.clientY;
      return;
    }

    // Ctrl + left drag → pan
    if (event.button === 0 && event.ctrlKey) {
      event.preventDefault();
      isPanning = true;
      panStartX = event.clientX;
      panStartY = event.clientY;
      panStartOffsetX = camera.offset_x;
      panStartOffsetY = camera.offset_y;
    }
  }

  function handleMouseMove(event: MouseEvent) {
    if (isPanning) {
      const dx = (event.clientX - panStartX) / camera.zoom;
      const dy = (event.clientY - panStartY) / camera.zoom;
      camera = { ...camera, offset_x: panStartOffsetX + dx, offset_y: panStartOffsetY + dy };
      requestFrame();
      return;
    }

    if (isOrbiting) {
      const dx = (event.clientX - panStartX) * 0.5;
      const dy = (event.clientY - panStartY) * 0.5;
      panStartX = event.clientX;
      panStartY = event.clientY;
      orbitCamera(dx, dy);
    }
  }

  function handleMouseUp() {
    isPanning = false;
    isOrbiting = false;
  }

  /** Prevent context menu so right-click drag works for orbiting. */
  function handleContextMenu(event: MouseEvent) {
    event.preventDefault();
  }

  /** Handle keyboard shortcuts when viewport is focused. */
  function handleKeyDown(event: KeyboardEvent) {
    // Tool switching: Q/W/E/R
    const toolKey = TOOL_KEYS[event.key.toLowerCase()];
    if (toolKey && !event.ctrlKey && !event.altKey && !event.metaKey) {
      event.preventDefault();
      setActiveTool(toolKey);
      return;
    }

    // F — focus selected entity
    if (event.key.toLowerCase() === 'f' && !event.ctrlKey) {
      event.preventDefault();
      if (selectedEntityId != null) {
        focusEntity(selectedEntityId);
      }
      return;
    }

    // Delete / Backspace — delete selected entity
    if ((event.key === 'Delete' || event.key === 'Backspace') && selectedEntityId != null) {
      event.preventDefault();
      deleteEntity(selectedEntityId);
      return;
    }

    // Ctrl+D — duplicate selected entity
    if (event.key.toLowerCase() === 'd' && event.ctrlKey && selectedEntityId != null) {
      event.preventDefault();
      duplicateEntity(selectedEntityId);
      return;
    }

    // Arrow keys — pan camera
    const PAN_STEP = 0.5;
    switch (event.key) {
      case 'ArrowLeft':
        event.preventDefault();
        panCamera(-PAN_STEP, 0);
        return;
      case 'ArrowRight':
        event.preventDefault();
        panCamera(PAN_STEP, 0);
        return;
      case 'ArrowUp':
        event.preventDefault();
        panCamera(0, PAN_STEP);
        return;
      case 'ArrowDown':
        event.preventDefault();
        panCamera(0, -PAN_STEP);
        return;
    }
  }

  /** Get cursor style based on active tool. */
  function getCursor(): string {
    if (isPanning) return 'grabbing';
    if (isOrbiting) return 'grab';
    switch (activeTool) {
      case 'select': return 'crosshair';
      case 'move': return 'move';
      case 'rotate': return 'alias';
      case 'scale': return 'nwse-resize';
      default: return 'crosshair';
    }
  }

  /** Tool button data. */
  const tools: { key: SceneTool; label: string; shortcut: string }[] = [
    { key: 'select', label: 'tool.select', shortcut: 'Q' },
    { key: 'move',   label: 'tool.move',   shortcut: 'W' },
    { key: 'rotate', label: 'tool.rotate', shortcut: 'E' },
    { key: 'scale',  label: 'tool.scale',  shortcut: 'R' },
  ];
</script>

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_noninteractive_element_interactions a11y_no_noninteractive_tabindex -->
<div
  class="viewport-container"
  bind:this={containerEl}
  role="application"
  aria-label={t('panel.viewport')}
  tabindex="0"
  style:cursor={getCursor()}
  onclick={handleClick}
  onwheel={handleWheel}
  onmousedown={handleMouseDown}
  onmousemove={handleMouseMove}
  onmouseup={handleMouseUp}
  onmouseleave={handleMouseUp}
  oncontextmenu={handleContextMenu}
  onkeydown={handleKeyDown}
>
  <!-- Toolbar -->
  <div class="viewport-toolbar">
    <div class="toolbar-group">
      {#each tools as tool}
        <button
          class="tool-btn"
          class:active={activeTool === tool.key}
          title={t(tool.label)}
          onclick={(e: MouseEvent) => { e.stopPropagation(); setActiveTool(tool.key); }}
        >
          <span class="tool-icon">{tool.shortcut}</span>
        </button>
      {/each}
    </div>

    <div class="toolbar-separator"></div>

    <div class="toolbar-group">
      <button
        class="tool-btn"
        class:active={gridVisible}
        title={t('viewport.grid')}
        onclick={(e: MouseEvent) => { e.stopPropagation(); toggleGrid(); }}
      >
        <span class="tool-icon">#</span>
      </button>
      <button
        class="tool-btn"
        class:active={snapToGrid}
        title={t('viewport.snap')}
        onclick={(e: MouseEvent) => { e.stopPropagation(); toggleSnapToGrid(); }}
      >
        <span class="tool-icon">&#8982;</span>
      </button>
    </div>

    <div class="toolbar-separator"></div>

    <div class="toolbar-group">
      <button
        class="tool-btn"
        title={t('scene.create_entity')}
        onclick={(e: MouseEvent) => { e.stopPropagation(); createEntity(); }}
      >
        <span class="tool-icon">+</span>
      </button>
    </div>
  </div>

  <!-- SVG viewport -->
  {#if loading}
    <div class="viewport-loading">
      <p>{t('viewport.loading')}</p>
    </div>
  {:else}
    <!-- eslint-disable-next-line svelte/no-at-html-tags -->
    {@html svgContent}
  {/if}

  <!-- Axis gizmo (camera orientation indicator) -->
  <div class="axis-gizmo" aria-hidden="true">
    <svg width="60" height="60" viewBox="0 0 60 60">
      <line x1="30" y1="30" x2="50" y2="30" stroke="#e06c75" stroke-width="2"/>
      <text x="52" y="34" fill="#e06c75" font-size="10" font-family="sans-serif">X</text>
      <line x1="30" y1="30" x2="30" y2="10" stroke="#98c379" stroke-width="2"/>
      <text x="27" y="8" fill="#98c379" font-size="10" font-family="sans-serif">Y</text>
      <line x1="30" y1="30" x2="16" y2="42" stroke="#61afef" stroke-width="2"/>
      <text x="8" y="48" fill="#61afef" font-size="10" font-family="sans-serif">Z</text>
    </svg>
  </div>

  <!-- HUD overlay -->
  <div class="viewport-hud">
    <span class="hud-tool" title={t(`tool.${activeTool}` as any)}>
      {activeTool.charAt(0).toUpperCase() + activeTool.slice(1)}
    </span>
    <span class="hud-separator">|</span>
    <span class="hud-zoom" title={t('viewport.zoom')}>
      {Math.round(camera.zoom * 100)}%
    </span>
    <button
      class="hud-btn"
      onclick={(e: MouseEvent) => { e.stopPropagation(); resetCamera(); }}
      title={t('viewport.reset_camera')}
    >
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
    user-select: none;
    background: #1a1a2e;
    outline: none;
  }

  .viewport-container:focus-visible {
    outline: 1px solid var(--color-accent, #61afef);
    outline-offset: -1px;
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

  /* Toolbar */
  .viewport-toolbar {
    position: absolute;
    top: 6px;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 2px 6px;
    background: rgba(0, 0, 0, 0.7);
    border-radius: 5px;
    z-index: 10;
    pointer-events: auto;
  }

  .toolbar-group {
    display: flex;
    gap: 1px;
  }

  .toolbar-separator {
    width: 1px;
    height: 18px;
    background: #444;
    margin: 0 4px;
  }

  .tool-btn {
    background: none;
    border: 1px solid transparent;
    border-radius: 3px;
    color: #999;
    font-size: 12px;
    font-family: monospace;
    padding: 2px 6px;
    cursor: pointer;
    line-height: 1.2;
    min-width: 24px;
    text-align: center;
  }

  .tool-btn:hover {
    color: #fff;
    border-color: #555;
    background: rgba(255, 255, 255, 0.05);
  }

  .tool-btn.active {
    color: #61afef;
    border-color: #61afef;
    background: rgba(97, 175, 239, 0.1);
  }

  .tool-icon {
    display: inline-block;
  }

  /* Axis gizmo */
  .axis-gizmo {
    position: absolute;
    top: 40px;
    right: 8px;
    pointer-events: none;
    opacity: 0.7;
  }

  /* HUD */
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

  .hud-tool {
    color: #61afef;
    font-weight: 500;
  }

  .hud-separator {
    color: #444;
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
