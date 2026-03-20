<script lang="ts">
  import { onMount } from 'svelte';
  import { t } from '$lib/i18n';
  import {
    getEditorContext,
    setSelectedEntityId,
    subscribeContext,
  } from '$lib/stores/editor-context';
  import {
    createNativeViewport,
    destroyNativeViewport,
    viewportCameraOrbit,
    viewportCameraPan,
    viewportCameraZoom,
    viewportCameraReset,
    viewportSetGridVisible,
    viewportCameraSetOrientation,
    viewportSetProjection,
  } from '$lib/api';
  import type { SceneTool, ProjectionMode } from '$lib/scene/state';
  import {
    createEntity,
    deleteEntity,
    duplicateEntity,
    translateEntity,
    rotateEntityBy,
    scaleEntityBy,
    focusEntity,
  } from '$lib/scene/commands';
  import { saveViewportSettings, loadViewportSettings } from '$lib/viewport-settings';

  const TOOL_KEYS: Record<string, SceneTool> = {
    q: 'select',
    w: 'move',
    e: 'rotate',
    r: 'scale',
  };

  // Dock panel ID passed from DockContainer — stable across remounts so the
  // Rust registry can preserve camera state on panel drag / tab switch.
  let { panelId = '' }: { panelId?: string } = $props();

  /** Detect if running inside Tauri or standalone browser */
  // Check at runtime, not module load — __TAURI_INTERNALS__ may not be set yet
  function checkIsTauri(): boolean {
    return typeof window !== 'undefined' && !!(window as any).__TAURI_INTERNALS__;
  }
  let isTauri = checkIsTauri();

  // Use the stable dock panel ID as the Rust registry key so camera state is
  // preserved across panel drag / tab switches.  Fall back to a random ID
  // only when no panelId is provided (e.g. pop-out or dev/browser mode).
  const viewportId = panelId || `vp-${Date.now()}-${Math.floor(Math.random() * 0xffff).toString(16)}`;

  let containerEl: HTMLDivElement | undefined = $state(undefined);
  let viewportWidth = $state(800);
  let viewportHeight = $state(600);
  let loading = $state(true);
  /** Set synchronously before calling createNativeViewport so cleanup can
   *  always call destroyNativeViewport — even if the async create hasn't
   *  resolved yet when the component unmounts (fast tab switch). */
  let viewportRegistered = false;
  /** Reactive flag for UI — set true once the async create resolves. */
  let nativeViewportCreated = $state(false);

  // Per-viewport UI state — fully local, NOT shared with other viewports.
  let activeTool: SceneTool = $state('select');
  let gridVisible = $state(true);
  let snapToGrid = $state(false);
  let projection: ProjectionMode = $state('perspective');
  let cameraYawRad = $state(0.0);         // radians, matches Rust OrbitCamera::yaw
  let cameraPitchRad = $state(Math.PI / 6); // radians, matches Rust OrbitCamera::pitch default

  // True once onMount has loaded saved settings — gates the $effect save below
  // so we don't overwrite localStorage with defaults on first render.
  let settingsLoaded = false;

  // Save to localStorage reactively on every settings change (not just on
  // cleanup) so the stored values are always current when the tab unmounts.
  $effect(() => {
    if (!settingsLoaded) return;
    saveViewportSettings(viewportId, {
      activeTool,
      gridVisible,
      snapToGrid,
      projection,
      cameraYawRad,
      cameraPitchRad,
    });
  });

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

  // Shared editor state (entities, selection — truly global across all panels)
  let entities = $state(getEditorContext().entities);
  let selectedEntityId: number | null = $state(getEditorContext().selectedEntityId);

  onMount(() => {
    // Subscribe only for shared state: entities and selection.
    const unsub = subscribeContext(() => {
      const ctx = getEditorContext();
      entities = ctx.entities;
      selectedEntityId = ctx.selectedEntityId;
    });

    // Observe container size
    if (containerEl) {
      /** Compute physical-pixel bounds of the viewport panel container. */
      function getPhysicalBounds(): { x: number; y: number; width: number; height: number } {
        const rect = containerEl!.getBoundingClientRect();
        const sf = window.devicePixelRatio || 1;
        return {
          x: Math.round(rect.left * sf),
          y: Math.round(rect.top * sf),
          width: Math.round(rect.width * sf),
          height: Math.max(1, Math.round(rect.height * sf)),
        };
      }

      const observer = new ResizeObserver((entries) => {
        for (const entry of entries) {
          viewportWidth = Math.round(entry.contentRect.width) || 800;
          viewportHeight = Math.round(entry.contentRect.height) || 600;
        }
        // Skip when hidden (display:none gives 0 bounds) — we don't want to
        // register the viewport with wrong dimensions. When the slot becomes
        // visible again the observer fires with correct bounds and we register.
        if (isTauri) {
          const b = getPhysicalBounds();
          if (b.width > 0 && b.height > 0) {
            viewportRegistered = true;
            createNativeViewport(viewportId, b.x, b.y, b.width, b.height);
          }
        }
      });
      observer.observe(containerEl);

      // Re-check at mount time in case __TAURI_INTERNALS__ wasn't ready at script eval
      isTauri = checkIsTauri();

      // Create (or update bounds of) this viewport instance in Tauri mode.
      // createNativeViewport is idempotent — safe to call on every mount
      // including remounts after panel drag to a new dock zone.
      if (isTauri) {
        const bounds = getPhysicalBounds();
        // Skip initial registration if panel is hidden (display:none → 0 bounds).
        // The ResizeObserver will register it when the slot becomes visible.
        if (bounds.width > 0 && bounds.height > 0) {
          viewportRegistered = true;
          createNativeViewport(viewportId, bounds.x, bounds.y, bounds.width, bounds.height).then(() => {
            nativeViewportCreated = true;
            loading = false;
            // Sync grid visibility and projection to Rust on mount — restores persisted state
            viewportSetGridVisible(viewportId, gridVisible);
            viewportSetProjection(viewportId, projection === 'ortho');
          }).catch((_e) => {
            loading = false;
          });
        } else {
          loading = false;
        }
      } else {
        loading = false;
      }

      // Restore persisted per-viewport settings from a previous session.
      const saved = loadViewportSettings(viewportId);
      if (saved) {
        if (saved.activeTool) activeTool = saved.activeTool as SceneTool;
        gridVisible = saved.gridVisible;
        snapToGrid = saved.snapToGrid;
        if (saved.projection === 'perspective' || saved.projection === 'ortho') {
          projection = saved.projection as ProjectionMode;
        }
        if (saved.cameraYawRad != null) {
          cameraYawRad = saved.cameraYawRad;
        }
        if (saved.cameraPitchRad != null) {
          cameraPitchRad = saved.cameraPitchRad;
        }
        cursor = cursorForTool(activeTool);
      }
      // Allow $effect to start saving now that defaults have been overwritten.
      settingsLoaded = true;

      return () => {
        unsub();
        observer.disconnect();
        // Remove this instance from the Rust registry on unmount.
        if (isTauri && viewportRegistered) {
          destroyNativeViewport(viewportId);
        }
      };
    }

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
  // Mouse event handlers
  // ---------------------------------------------------------------------------

  /** Handle mouse wheel for zoom. */
  function handleWheel(event: WheelEvent) {
    event.preventDefault();
    viewportCameraZoom(viewportId, -event.deltaY);
  }

  /** Start a drag interaction based on button / modifier.
   *
   *  Navigation (works regardless of active tool):
   *    Middle mouse drag        = Pan   (cursor: grabbing)
   *    Alt + Left mouse drag    = Orbit (cursor: all-scroll)
   *    Right mouse drag         = Orbit (cursor: all-scroll)
   *    Scroll wheel             = Zoom  (handled in handleWheel)
   *
   *  Tool interactions (Left mouse, no modifier):
   *    Q (Select)  : Left click         = select entity
   *    W (Move)    : Left click + drag  = move entity
   *    E (Rotate)  : Left click + drag  = rotate entity
   *    R (Scale)   : Left click + drag  = scale entity
   */
  function handleMouseDown(event: MouseEvent) {
    const tool = activeTool;

    // Middle mouse → pan
    if (event.button === 1) {
      event.preventDefault();
      startDrag(event, 'pan');
      return;
    }

    // Alt + right click → zoom (Unity style) — check before plain right-click
    if (event.button === 2 && event.altKey) {
      event.preventDefault();
      startDrag(event, 'zoom');
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

    // Ctrl + left drag → pan
    if (event.button === 0 && event.ctrlKey && !event.metaKey) {
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

    const dy = event.clientY - dragStartY;

    switch (dragMode) {
      case 'pan': {
        const rawDx = event.clientX - dragStartX;
        const rawDy = event.clientY - dragStartY;
        dragStartX = event.clientX;
        dragStartY = event.clientY;
        viewportCameraPan(viewportId, rawDx, rawDy);
        break;
      }

      case 'orbit': {
        const orbitDx = event.clientX - dragStartX;
        const orbitDy = event.clientY - dragStartY;
        dragStartX = event.clientX;
        dragStartY = event.clientY;
        // Match Rust sign convention: yaw -= dx * 0.005, pitch += dy * 0.005 (clamped)
        cameraYawRad -= orbitDx * 0.005;
        cameraPitchRad = Math.max(-1.5, Math.min(1.5, cameraPitchRad + orbitDy * 0.005));
        viewportCameraOrbit(viewportId, orbitDx, orbitDy);
        break;
      }

      case 'zoom': {
        dragStartX = event.clientX;
        dragStartY = event.clientY;
        viewportCameraZoom(viewportId, dy * -5);
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

  /** Toggle projection mode and sync to Rust. */
  function toggleProjection() {
    projection = projection === 'perspective' ? 'ortho' : 'perspective';
    viewportSetProjection(viewportId, projection === 'ortho');
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
      activeTool = toolKey;
      cursor = cursorForTool(toolKey);
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

    // P — toggle projection
    if (event.key.toLowerCase() === 'p' && !event.ctrlKey && !event.altKey) {
      event.preventDefault();
      toggleProjection();
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
    const PAN_STEP = 30;
    switch (event.key) {
      case 'ArrowLeft':
        event.preventDefault();
        viewportCameraPan(viewportId, -PAN_STEP, 0);
        return;
      case 'ArrowRight':
        event.preventDefault();
        viewportCameraPan(viewportId, PAN_STEP, 0);
        return;
      case 'ArrowUp':
        event.preventDefault();
        viewportCameraPan(viewportId, 0, -PAN_STEP);
        return;
      case 'ArrowDown':
        event.preventDefault();
        viewportCameraPan(viewportId, 0, PAN_STEP);
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
          onclick={(e: MouseEvent) => { e.stopPropagation(); activeTool = tool.key; cursor = cursorForTool(tool.key); }}
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
        onclick={(e: MouseEvent) => {
          e.stopPropagation();
          gridVisible = !gridVisible;
          viewportSetGridVisible(viewportId, gridVisible);
        }}
      >
        <span class="tool-icon">#</span>
      </button>
      <button
        class="tool-btn"
        class:active={snapToGrid}
        title={t('viewport.snap')}
        onclick={(e: MouseEvent) => { e.stopPropagation(); snapToGrid = !snapToGrid; }}
      >
        <span class="tool-icon">&#8982;</span>
      </button>
    </div>

    <div class="toolbar-separator"></div>

    <div class="toolbar-group">
      <button
        class="tool-btn"
        title={projection === 'ortho' ? 'Switch to Perspective' : 'Switch to Orthographic'}
        onclick={(e: MouseEvent) => { e.stopPropagation(); toggleProjection(); }}
      >
        <span class="tool-icon">{projection === 'ortho' ? '\u229E' : '\u25CE'}</span>
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

  <!-- No fallback content. In Tauri, the Vulkan child window renders behind
       this transparent area. In browser mode, it's just empty/transparent. -->

  <!-- Axis gizmo — 3D projected cube showing camera orientation -->
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div class="axis-gizmo" role="group" aria-label="Camera orientation gizmo">
  {#if true}
    {@const S = 18}
    {@const CX = 30}
    {@const CY = 30}
    {@const cy_rot = Math.cos(-cameraYawRad)}
    {@const sy_rot = Math.sin(-cameraYawRad)}
    {@const cp_rot = Math.cos(-cameraPitchRad)}
    {@const sp_rot = Math.sin(-cameraPitchRad)}
    {@const project = (vx: number, vy: number, vz: number) => {
      // Rotate around Y (-yaw)
      const rx = cy_rot * vx + sy_rot * vz;
      const ry = vy;
      const rz = -sy_rot * vx + cy_rot * vz;
      // Rotate around X (-pitch)
      const px = rx;
      const py = cp_rot * ry - sp_rot * rz;
      const pz = sp_rot * ry + cp_rot * rz;
      return { x: CX + px * S, y: CY - py * S, z: pz };
    }}
    {@const VERTS: [number,number,number][] = [
      [-1,-1,-1],[1,-1,-1],[1,1,-1],[-1,1,-1],
      [-1,-1, 1],[1,-1, 1],[1,1, 1],[-1,1, 1],
    ]}
    {@const FACES: { vi: number[]; label: string; color: string; snapYaw: number; snapPitch: number }[] = [
      { vi:[1,2,6,5], label:'X',  color:'#e06c75', snapYaw:-Math.PI/2, snapPitch:0      },
      { vi:[0,4,7,3], label:'-X', color:'#7a3040', snapYaw: Math.PI/2, snapPitch:0      },
      { vi:[3,2,6,7], label:'Y',  color:'#98c379', snapYaw:0,          snapPitch:-1.5   },
      { vi:[0,1,5,4], label:'-Y', color:'#3d6130', snapYaw:0,          snapPitch: 1.5   },
      { vi:[4,5,6,7], label:'Z',  color:'#61afef', snapYaw:0,          snapPitch:0      },
      { vi:[0,1,2,3], label:'-Z', color:'#2a4d7a', snapYaw:Math.PI,    snapPitch:0      },
    ]}
    {@const projected = VERTS.map(([x,y,z]) => project(x,y,z))}
    {@const sortedFaces = FACES.map(f => {
      const pts = f.vi.map(i => projected[i]);
      const centerZ = pts.reduce((s,p) => s + p.z, 0) / pts.length;
      const points = pts.map(p => `${p.x.toFixed(1)},${p.y.toFixed(1)}`).join(' ');
      // Centroid for label
      const lx = pts.reduce((s,p) => s + p.x, 0) / pts.length;
      const ly = pts.reduce((s,p) => s + p.y, 0) / pts.length;
      return { ...f, centerZ, points, lx, ly };
    }).sort((a,b) => a.centerZ - b.centerZ)}
    <svg width="60" height="60" viewBox="0 0 60 60">
      {#each sortedFaces as face}
        {@const opacity = face.centerZ > 0 ? 1.0 : 0.25}
        {@const snapYaw = face.snapYaw}
        {@const snapPitch = face.snapPitch}
        <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
        <polygon
          points={face.points}
          fill={face.color}
          fill-opacity={opacity * 0.85}
          stroke={face.color}
          stroke-width="0.5"
          stroke-opacity={opacity}
          style="cursor: pointer;"
          onclick={(e: MouseEvent) => {
            e.stopPropagation();
            cameraYawRad = snapYaw;
            cameraPitchRad = snapPitch;
            viewportCameraSetOrientation(viewportId, snapYaw, snapPitch);
          }}
        />
        <text
          x={face.lx}
          y={face.ly + 3.5}
          text-anchor="middle"
          fill="white"
          fill-opacity={opacity}
          font-size="8"
          font-family="sans-serif"
          font-weight="600"
          style="pointer-events: none; user-select: none;"
        >{face.label}</text>
      {/each}
    </svg>
  {/if}
  </div>

  <!-- HUD overlay -->
  <div class="viewport-hud">
    <span class="hud-tool" title={t(`tool.${activeTool}` as any)}>
      {activeTool.charAt(0).toUpperCase() + activeTool.slice(1)}
    </span>
    <span class="hud-separator">|</span>
    <span class="hud-projection">{projection === 'ortho' ? 'Ortho' : 'Persp'}</span>
    <button
      class="hud-btn"
      onclick={(e: MouseEvent) => {
        e.stopPropagation();
        cameraYawRad = 0.0;
        cameraPitchRad = Math.PI / 6;
        viewportCameraReset(viewportId);
      }}
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
    /* Transparent — Vulkan child window renders on top via WS_EX_TRANSPARENT
       (click-through) child-above-WebView2 approach. */
    background: transparent;
    outline: none;
    z-index: 0;
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
    pointer-events: auto;
    opacity: 0.7;
  }

  .axis-gizmo:hover {
    opacity: 1;
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

  .hud-projection {
    color: #98c379;
    font-weight: 500;
    font-size: 10px;
    text-transform: uppercase;
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
