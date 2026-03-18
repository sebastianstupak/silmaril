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
    translateEntity,
    rotateEntityBy,
    scaleEntityBy,
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

  // --- Drag / interaction state ---
  type DragMode =
    | 'none'
    | 'pan'
    | 'orbit'
    | 'zoom'
    | 'move_entity'
    | 'rotate_entity'
    | 'scale_entity';

  let isDragging = $state(false);
  let dragMode: DragMode = $state('none');
  let dragStartX = 0;
  let dragStartY = 0;

  /** Cursor CSS value — reactive, recalculated on every relevant state change. */
  let cursor = $state('default');

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

      // Update cursor when tool changes (and we are not dragging)
      if (!isDragging) {
        cursor = cursorForTool(activeTool);
      }

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

  // ---------------------------------------------------------------------------
  // Cursor helpers
  // ---------------------------------------------------------------------------

  /** Map a tool name to its resting (non-drag) cursor. */
  function cursorForTool(tool: SceneTool): string {
    switch (tool) {
      case 'select': return 'default';
      case 'move':   return 'move';
      case 'rotate': return 'crosshair';
      case 'scale':  return 'nwse-resize';
      default:       return 'default';
    }
  }

  /** Map a drag mode to the cursor shown while dragging. */
  function cursorForDrag(mode: DragMode): string {
    switch (mode) {
      case 'pan':            return 'grabbing';
      case 'orbit':          return 'all-scroll';
      case 'zoom':           return 'ns-resize';
      case 'move_entity':    return 'move';
      case 'rotate_entity':  return 'crosshair';
      case 'scale_entity':   return 'nwse-resize';
      default:               return 'default';
    }
  }

  // ---------------------------------------------------------------------------
  // Viewport entity list
  // ---------------------------------------------------------------------------

  /** Build the viewport entity list from editor context entities.
   *  Converts 3D world positions to normalised viewport coords (0–1)
   *  using the scene camera offset and zoom. */
  function buildViewportEntities(): ViewportEntity[] {
    const scene = getSceneState();
    return scene.entities.map((e, i) => {
      const cam = scene.camera;
      const worldX = e.position.x - cam.position.x;
      const worldY = e.position.z - cam.position.z; // Z is "forward" in top-down
      const scale = cam.zoom * 50; // pixels-per-world-unit
      const screenX = 0.5 + (worldX * scale) / viewportWidth;
      const screenY = 0.5 + (worldY * scale) / viewportHeight;

      return {
        id: e.id,
        name: e.name,
        x: Math.max(0.05, Math.min(0.95, screenX)),
        y: Math.max(0.05, Math.min(0.95, screenY)),
        color: ENTITY_COLORS[i % ENTITY_COLORS.length],
      };
    });
  }

  // ---------------------------------------------------------------------------
  // Frame rendering
  // ---------------------------------------------------------------------------

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
        tool: activeTool,
      });
      svgContent = svg;
      loading = false;
    } catch {
      loading = false;
    }
  }

  // ---------------------------------------------------------------------------
  // Mouse event handlers
  // ---------------------------------------------------------------------------

  /** Handle click on the viewport to pick entities. */
  async function handleClick(event: MouseEvent) {
    // Don't pick if we just finished a drag
    if (isDragging) return;
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

  /** Start a drag interaction based on button / modifier. */
  function handleMouseDown(event: MouseEvent) {
    const tool = getSceneState().activeTool;

    // Middle mouse → pan
    if (event.button === 1) {
      event.preventDefault();
      startDrag(event, 'pan');
      return;
    }

    // Right mouse → orbit
    if (event.button === 2) {
      event.preventDefault();
      startDrag(event, 'orbit');
      return;
    }

    // Alt + left click → orbit (Unity style)
    if (event.button === 0 && event.altKey) {
      event.preventDefault();
      startDrag(event, 'orbit');
      return;
    }

    // Alt + right click → zoom (Unity style)
    if (event.button === 2 && event.altKey) {
      event.preventDefault();
      startDrag(event, 'zoom');
      return;
    }

    // Ctrl + left drag → pan
    if (event.button === 0 && event.ctrlKey) {
      event.preventDefault();
      startDrag(event, 'pan');
      return;
    }

    // Left click with manipulation tool on selected entity → entity drag
    if (event.button === 0 && tool !== 'select' && selectedEntityId != null) {
      event.preventDefault();
      const mode: DragMode =
        tool === 'move' ? 'move_entity' :
        tool === 'rotate' ? 'rotate_entity' : 'scale_entity';
      startDrag(event, mode);
      return;
    }

    // Left click with select tool → handled by handleClick
  }

  function startDrag(event: MouseEvent, mode: DragMode) {
    isDragging = true;
    dragMode = mode;
    dragStartX = event.clientX;
    dragStartY = event.clientY;
    cursor = cursorForDrag(mode);
  }

  function handleMouseMove(event: MouseEvent) {
    if (!isDragging) return;

    const dx = event.clientX - dragStartX;
    const dy = event.clientY - dragStartY;

    switch (dragMode) {
      case 'pan': {
        const panDx = (event.clientX - dragStartX) / (camera.zoom * 50);
        const panDy = -(event.clientY - dragStartY) / (camera.zoom * 50);
        dragStartX = event.clientX;
        dragStartY = event.clientY;
        panCamera(panDx, panDy);
        break;
      }

      case 'orbit': {
        const orbitDx = (event.clientX - dragStartX);
        const orbitDy = (event.clientY - dragStartY);
        dragStartX = event.clientX;
        dragStartY = event.clientY;
        orbitCamera(orbitDx * 0.5, orbitDy * 0.5);
        break;
      }

      case 'zoom': {
        const zoomDelta = -dy * 0.005;
        dragStartX = event.clientX;
        dragStartY = event.clientY;
        zoomCamera(zoomDelta);
        break;
      }

      case 'move_entity': {
        if (selectedEntityId != null) {
          const moveDx = (event.clientX - dragStartX) * 0.02;
          const moveDy = -(event.clientY - dragStartY) * 0.02;
          dragStartX = event.clientX;
          dragStartY = event.clientY;
          translateEntity(selectedEntityId, moveDx, moveDy, 0);
        }
        break;
      }

      case 'rotate_entity': {
        if (selectedEntityId != null) {
          const rotDx = (event.clientX - dragStartX) * 0.5;
          const rotDy = -(event.clientY - dragStartY) * 0.5;
          dragStartX = event.clientX;
          dragStartY = event.clientY;
          rotateEntityBy(selectedEntityId, rotDy, rotDx, 0);
        }
        break;
      }

      case 'scale_entity': {
        if (selectedEntityId != null) {
          const scaleDelta = (event.clientX - dragStartX) * 0.005;
          dragStartX = event.clientX;
          dragStartY = event.clientY;
          const factor = 1 + scaleDelta;
          scaleEntityBy(selectedEntityId, factor, factor, factor);
        }
        break;
      }
    }
  }

  function handleMouseUp() {
    if (isDragging) {
      isDragging = false;
      dragMode = 'none';
      cursor = cursorForTool(activeTool);
    }
  }

  /** Prevent context menu so right-click drag works for orbiting. */
  function handleContextMenu(event: MouseEvent) {
    event.preventDefault();
  }

  // ---------------------------------------------------------------------------
  // Keyboard shortcuts
  // ---------------------------------------------------------------------------

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
  style:cursor={cursor}
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

  <!-- Drag-mode indicator (visible during drag operations) -->
  {#if isDragging}
    <div class="drag-indicator" aria-hidden="true">
      {#if dragMode === 'pan'}Pan
      {:else if dragMode === 'orbit'}Orbit
      {:else if dragMode === 'zoom'}Zoom
      {:else if dragMode === 'move_entity'}Move
      {:else if dragMode === 'rotate_entity'}Rotate
      {:else if dragMode === 'scale_entity'}Scale
      {/if}
    </div>
  {/if}
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

  /* Drag-mode indicator */
  .drag-indicator {
    position: absolute;
    bottom: 8px;
    left: 8px;
    padding: 3px 10px;
    background: rgba(0, 0, 0, 0.65);
    border-radius: 4px;
    font-size: 11px;
    color: #61afef;
    font-weight: 500;
    pointer-events: none;
  }
</style>
