# Viewport UI Polish Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Modernise the Vulkan viewport UI — Lucide icon toolbar with bits-ui tooltips, working orthographic camera, gizmo top-right, min panel size, remove broken zoom %, HUD polish.

**Architecture:** Six sequential tasks: (1-2) Rust backend for ortho projection, (3) TS API + settings cleanup, (4) Svelte logic — cameraZoom removal + projection wiring, (5) Svelte toolbar icons + tooltips, (6) Svelte gizmo + HUD + min size. Tasks 1-3 are independent of each other but tasks 4-6 all touch ViewportPanel.svelte and must be sequential.

**Tech Stack:** Rust (glam `Mat4::orthographic_rh`), Tauri 2 commands, `@lucide/svelte` (already installed), `bits-ui` Tooltip (already installed), Svelte 5 `$state`/`$effect`, plain CSS custom properties.

**Spec:** `docs/superpowers/specs/2026-03-20-viewport-ui-polish-design.md`

---

## File Map

| File | Changes |
|------|---------|
| `engine/editor/src-tauri/viewport/native_viewport.rs` | Add `is_ortho` to `ViewportInstance`, update `view_projection`, add `set_projection`, update 4 snapshot sites, non-Windows stub |
| `engine/editor/src-tauri/bridge/commands.rs` | Add `viewport_set_projection` command |
| `engine/editor/src-tauri/lib.rs` | Register `viewport_set_projection` in `generate_handler!` |
| `engine/editor/src/lib/api.ts` | Add `viewportSetProjection` wrapper |
| `engine/editor/src/lib/viewport-settings.ts` | Remove `cameraZoom?: number` field |
| `engine/editor/src/lib/viewport-settings.test.ts` | Fix `'persp'` → `'perspective'` in two test fixtures |
| `engine/editor/src/lib/docking/panels/ViewportPanel.svelte` | All UI + logic changes (tasks 4-6) |

---

## Task 1: Rust — Orthographic camera in native_viewport.rs

**Files:**
- Modify: `engine/editor/src-tauri/viewport/native_viewport.rs`

**Context:** All changes are inside `#[cfg(windows)] mod platform { ... }`. The non-Windows stub block is at the bottom of the file (line ~1033). The render pipeline snapshot is at line ~827. Tests are in `#[cfg(test)] mod tests` inside `mod platform`.

- [ ] **Step 1: Add `is_ortho: bool` to `ViewportInstance` (line ~61)**

Replace:
```rust
#[derive(Clone)]
struct ViewportInstance {
    bounds: ViewportBounds,
    camera: OrbitCamera,
    visible: bool,
    grid_visible: bool,
}

impl ViewportInstance {
    fn new(bounds: ViewportBounds) -> Self {
        Self { bounds, camera: OrbitCamera::default(), visible: true, grid_visible: true }
    }
}
```
With:
```rust
#[derive(Clone)]
struct ViewportInstance {
    bounds: ViewportBounds,
    camera: OrbitCamera,
    visible: bool,
    grid_visible: bool,
    is_ortho: bool,
}

impl ViewportInstance {
    fn new(bounds: ViewportBounds) -> Self {
        Self { bounds, camera: OrbitCamera::default(), visible: true, grid_visible: true, is_ortho: false }
    }
}
```

- [ ] **Step 2: Update `view_projection` to accept `is_ortho` (line ~297)**

Replace:
```rust
fn view_projection(&self, aspect: f32) -> Mat4 {
    let view = Mat4::look_at_rh(self.eye(), self.target, Vec3::Y);
    let proj = Mat4::perspective_rh(self.fov_y, aspect, self.near, self.far);
    proj * view
}
```
With:
```rust
fn view_projection(&self, aspect: f32, is_ortho: bool) -> Mat4 {
    let view = Mat4::look_at_rh(self.eye(), self.target, Vec3::Y);
    if is_ortho {
        // half-extent matches what perspective renders at the focus point —
        // so toggling persp↔ortho doesn't cause a visual jump.
        let half_h = self.distance * (self.fov_y * 0.5).tan();
        let half_w = half_h * aspect;
        let proj = Mat4::orthographic_rh(-half_w, half_w, -half_h, half_h, self.near, self.far * 2.0);
        proj * view
    } else {
        let proj = Mat4::perspective_rh(self.fov_y, aspect, self.near, self.far);
        proj * view
    }
}
```

- [ ] **Step 3: Add `set_projection` method to `NativeViewport` (after `set_grid_visible`, line ~205)**

```rust
pub fn set_projection(&self, id: &str, is_ortho: bool) {
    if let Ok(mut instances) = self.instances.lock() {
        if let Some(inst) = instances.get_mut(id) {
            inst.is_ortho = is_ortho;
        }
    }
}
```

- [ ] **Step 4: Update the render loop snapshot tuple — 4 sites (line ~827)**

Site 1 — snapshot type declaration (line ~827):
```rust
// OLD
let viewports: Vec<(ViewportBounds, OrbitCamera, bool)> = {
// NEW
let viewports: Vec<(ViewportBounds, OrbitCamera, bool, bool)> = {
```

Site 2 — `.map()` closure (line ~831):
```rust
// OLD
.map(|i| (i.bounds, i.camera.clone(), i.grid_visible))
// NEW
.map(|i| (i.bounds, i.camera.clone(), i.grid_visible, i.is_ortho))
```

Site 3 — `render_frame` function signature (line ~557):
```rust
// OLD
fn render_frame(&mut self, viewports: &[(ViewportBounds, OrbitCamera, bool)]) -> Result<bool, String> {
// NEW
fn render_frame(&mut self, viewports: &[(ViewportBounds, OrbitCamera, bool, bool)]) -> Result<bool, String> {
```

Site 4 — `record_frame` function signature (line ~611):
```rust
// OLD
unsafe fn record_frame(
    &self,
    cmd: vk::CommandBuffer,
    image_index: usize,
    viewports: &[(ViewportBounds, OrbitCamera, bool)],
) -> Result<(), String> {
// NEW
unsafe fn record_frame(
    &self,
    cmd: vk::CommandBuffer,
    image_index: usize,
    viewports: &[(ViewportBounds, OrbitCamera, bool, bool)],
) -> Result<(), String> {
```

- [ ] **Step 5: Update the `record_frame` for-loop destructure and `view_projection` call (line ~633)**

Replace:
```rust
for (bounds, camera, grid_visible) in viewports {
    // ...
    let pc = GridPushConstants {
        view_proj: camera.view_projection(aspect).to_cols_array(),
```
With:
```rust
for (bounds, camera, grid_visible, is_ortho) in viewports {
    // ...
    let pc = GridPushConstants {
        view_proj: camera.view_projection(aspect, *is_ortho).to_cols_array(),
```

- [ ] **Step 6: Add `set_projection` no-op to the non-Windows stub (line ~1051, after `camera_set_orientation`)**

```rust
pub fn set_projection(&self, _id: &str, _is_ortho: bool) {}
```

- [ ] **Step 7: Add unit tests for orthographic projection (inside `#[cfg(test)] mod tests`)**

```rust
#[test]
fn view_projection_perspective_returns_finite_values() {
    let cam = OrbitCamera::default();
    let mat = cam.view_projection(16.0 / 9.0, false);
    assert!(mat.to_cols_array().iter().all(|v| v.is_finite()));
}

#[test]
fn view_projection_ortho_returns_finite_values() {
    let cam = OrbitCamera::default();
    let mat = cam.view_projection(16.0 / 9.0, true);
    assert!(mat.to_cols_array().iter().all(|v| v.is_finite()));
}

#[test]
fn view_projection_ortho_differs_from_perspective() {
    let cam = OrbitCamera::default();
    let persp = cam.view_projection(1.0, false).to_cols_array();
    let ortho = cam.view_projection(1.0, true).to_cols_array();
    // Matrices must be different — ortho has no perspective divide
    assert!(persp != ortho, "ortho and perspective matrices must differ");
}

#[test]
fn viewport_instance_defaults_to_perspective() {
    let inst = ViewportInstance::new(ViewportBounds { x: 0, y: 0, width: 800, height: 600 });
    assert!(!inst.is_ortho, "new instance should default to perspective");
}
```

- [ ] **Step 8: Compile**

```bash
cd engine/editor && cargo build --package silmaril-editor 2>&1 | tail -20
```
Expected: no errors.

- [ ] **Step 9: Run Rust tests**

```bash
cargo test --package silmaril-editor --lib 2>&1 | tail -10
```
Expected: all tests pass (now 17 passing).

- [ ] **Step 10: Commit**

```bash
git add engine/editor/src-tauri/viewport/native_viewport.rs
git commit -m "feat(editor): add orthographic camera projection to Vulkan viewport"
```

---

## Task 2: Rust — viewport_set_projection Tauri command

**Files:**
- Modify: `engine/editor/src-tauri/bridge/commands.rs`
- Modify: `engine/editor/src-tauri/lib.rs`

**Context:** The last viewport command in `commands.rs` is `viewport_camera_set_orientation` (around line 713). The `generate_handler!` list in `lib.rs` ends with `commands::broadcast_settings` at line 223.

- [ ] **Step 1: Add the command to `commands.rs` (after `viewport_camera_set_orientation`)**

```rust
/// Switch between perspective and orthographic projection for a viewport instance.
#[tauri::command]
pub fn viewport_set_projection(
    viewport_state: tauri::State<NativeViewportState>,
    viewport_id: String,
    is_ortho: bool,
) -> Result<(), String> {
    let registry = viewport_state.0.lock().unwrap();
    if let Some(vp) = registry.get_for_id(&viewport_id) {
        vp.set_projection(&viewport_id, is_ortho);
    }
    Ok(())
}
```

- [ ] **Step 2: Register in `lib.rs` (add after `viewport_camera_set_orientation` on line ~214)**

```rust
commands::viewport_set_projection,
```

- [ ] **Step 3: Compile**

```bash
cd engine/editor && cargo build --package silmaril-editor 2>&1 | tail -10
```
Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add engine/editor/src-tauri/bridge/commands.rs engine/editor/src-tauri/lib.rs
git commit -m "feat(editor): add viewport_set_projection Tauri command"
```

---

## Task 3: TypeScript — API wrapper + settings cleanup

**Files:**
- Modify: `engine/editor/src/lib/api.ts`
- Modify: `engine/editor/src/lib/viewport-settings.ts`
- Modify: `engine/editor/src/lib/viewport-settings.test.ts`

- [ ] **Step 1: Add `viewportSetProjection` to `api.ts` (after `viewportCameraSetOrientation`, line ~188)**

```typescript
/** Switch between perspective (isOrtho=false) and orthographic (isOrtho=true)
 *  projection for a specific viewport instance. */
export async function viewportSetProjection(viewportId: string, isOrtho: boolean): Promise<void> {
  if (!isTauri) return;
  return tauriInvoke<void>('viewport_set_projection', { viewportId, isOrtho });
}
```

- [ ] **Step 2: Remove `cameraZoom?: number` from `ViewportUISettings` in `viewport-settings.ts`**

Replace:
```typescript
export interface ViewportUISettings {
  activeTool: string;
  gridVisible: boolean;
  snapToGrid: boolean;
  projection: string;
  cameraZoom?: number;
  cameraYawRad?: number;
  cameraPitchRad?: number;
}
```
With:
```typescript
export interface ViewportUISettings {
  activeTool: string;
  gridVisible: boolean;
  snapToGrid: boolean;
  projection: string;
  cameraYawRad?: number;
  cameraPitchRad?: number;
}
```

- [ ] **Step 3: Fix `'persp'` → `'perspective'` in two test fixtures in `viewport-settings.test.ts`**

In the `'round-trips cameraYawRad and cameraPitchRad'` test (line ~69), change:
```typescript
projection: 'persp',
```
To:
```typescript
projection: 'perspective',
```

In the `'loads settings without cameraYawRad/cameraPitchRad gracefully'` test (line ~84), change:
```typescript
projection: 'persp',
```
To:
```typescript
projection: 'perspective',
```

- [ ] **Step 4: Run TypeScript tests**

```bash
cd engine/editor && npx vitest run 2>&1 | tail -10
```
Expected: all tests pass (34 passing).

- [ ] **Step 5: Commit**

```bash
git add engine/editor/src/lib/api.ts engine/editor/src/lib/viewport-settings.ts engine/editor/src/lib/viewport-settings.test.ts
git commit -m "feat(editor): add viewportSetProjection API, remove cameraZoom settings field"
```

---

## Task 4: Svelte — cameraZoom removal + projection logic fix + P shortcut + Rust wiring

**Files:**
- Modify: `engine/editor/src/lib/docking/panels/ViewportPanel.svelte`

**Context:** This task handles all logic/state changes before touching the UI. Do not change any HTML/CSS yet.

- [ ] **Step 1: Add `viewportSetProjection` to the import from `$lib/api` (line ~10)**

In the existing `import { ..., viewportCameraSetOrientation } from '$lib/api';` block, add `viewportSetProjection`:

```typescript
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
```

- [ ] **Step 2: Remove `cameraZoom` state variable (line ~72)**

Delete this line:
```typescript
let cameraZoom = $state(1);
```

- [ ] **Step 3: Remove `cameraZoom` from the `$effect` save (line ~82)**

Remove `cameraZoom,` from the `saveViewportSettings` call object:
```typescript
// REMOVE this line inside the saveViewportSettings({...}) call:
cameraZoom,
```

- [ ] **Step 4: Remove `cameraZoom` restore from `onMount` (line ~191)**

Delete these lines:
```typescript
if (saved.cameraZoom != null && saved.cameraZoom > 0) {
  cameraZoom = saved.cameraZoom;
}
```

- [ ] **Step 5: Fix the projection type — change initial value from `'persp'` to `'perspective'` (line ~69)**

Replace:
```typescript
let projection: ProjectionMode = $state('persp');
```
With:
```typescript
let projection: ProjectionMode = $state('perspective');
```

- [ ] **Step 6: Fix the projection guard in the settings restore (line ~188)**

Replace:
```typescript
if (saved.projection === 'persp' || saved.projection === 'ortho') {
  projection = saved.projection;
}
```
With:
```typescript
if (saved.projection === 'perspective' || saved.projection === 'ortho') {
  projection = saved.projection as ProjectionMode;
}
```

- [ ] **Step 7: Remove `cameraZoom` assignments in `handleWheel` (line ~254)**

In `handleWheel`, remove:
```typescript
const delta = event.deltaY > 0 ? -0.1 : 0.1;
cameraZoom = Math.max(0.01, cameraZoom + delta);
```
Keep only:
```typescript
viewportCameraZoom(viewportId, -event.deltaY);
```

(Keep the `event.preventDefault()` line.)

- [ ] **Step 8: Remove `cameraZoom` assignment in the zoom drag case of `handleMouseMove` (line ~361)**

In the `'zoom'` case, remove:
```typescript
cameraZoom = Math.max(0.01, cameraZoom + (-dy * 0.005));
```
Keep only:
```typescript
dragStartX = event.clientX;
dragStartY = event.clientY;
viewportCameraZoom(viewportId, dy * -5);
```

- [ ] **Step 9: Remove `cameraZoom = 1` from the reset handler (line ~663)**

In the HUD reset button `onclick`, remove:
```typescript
cameraZoom = 1;
```

- [ ] **Step 10: Add projection toggle function and wire to Rust**

After the existing `handleContextMenu` function, add:

```typescript
/** Toggle projection mode and sync to Rust. */
function toggleProjection() {
  projection = projection === 'perspective' ? 'ortho' : 'perspective';
  viewportSetProjection(viewportId, projection === 'ortho');
}
```

- [ ] **Step 11: Sync projection on mount (in the `createNativeViewport.then()` callback, line ~171)**

Add `viewportSetProjection` call after `viewportSetGridVisible`:
```typescript
createNativeViewport(viewportId, bounds.x, bounds.y, bounds.width, bounds.height).then(() => {
  nativeViewportCreated = true;
  loading = false;
  viewportSetGridVisible(viewportId, gridVisible);
  viewportSetProjection(viewportId, projection === 'ortho');
}).catch((_e) => {
```

- [ ] **Step 12: Add `P` key shortcut for projection toggle in `handleKeyDown`**

After the `'f'` (focus) handler, add:
```typescript
// P — toggle projection
if (event.key.toLowerCase() === 'p' && !event.ctrlKey && !event.altKey) {
  event.preventDefault();
  toggleProjection();
  return;
}
```

- [ ] **Step 13: Update the toolbar projection button `onclick` to use `toggleProjection()`**

Find the existing projection button `onclick`:
```typescript
onclick={(e: MouseEvent) => { e.stopPropagation(); projection = projection === 'ortho' ? 'persp' : 'ortho'; }}
```
Replace with:
```typescript
onclick={(e: MouseEvent) => { e.stopPropagation(); toggleProjection(); }}
```

- [ ] **Step 14: TypeScript compile check**

```bash
cd engine/editor && npx tsc --noEmit 2>&1 | head -30
```
Expected: no errors.

- [ ] **Step 15: Commit**

```bash
git add engine/editor/src/lib/docking/panels/ViewportPanel.svelte
git commit -m "feat(editor): wire orthographic projection toggle to Rust, remove cameraZoom"
```

---

## Task 5: Svelte — Toolbar icons with bits-ui tooltips

**Files:**
- Modify: `engine/editor/src/lib/docking/panels/ViewportPanel.svelte`

**Context:** `@lucide/svelte` and `bits-ui` are already installed. The toolbar is the `<div class="viewport-toolbar">` block at line ~500. This task replaces text-char buttons with icon buttons + tooltips. Use `@lucide/svelte` (Svelte 5 compatible) not `lucide-svelte`.

- [ ] **Step 1: Add icon, tooltip, and Component imports at the top of the `<script>` block**

After the existing `import { saveViewportSettings, loadViewportSettings } from '$lib/viewport-settings';` line, add all three imports:

```typescript
import type { Component } from 'svelte';
import {
  MousePointer2, Move, RotateCw, Maximize2,
  Grid2X2, Magnet, Video, ScanLine,
  CirclePlus, RotateCcw,
} from '@lucide/svelte';
import { Tooltip } from 'bits-ui';
```

Note: `bits-ui` v1.0.0-next only exports from its root (`'bits-ui'`), not subpaths. Use `<Tooltip.Root>`, `<Tooltip.Trigger>`, `<Tooltip.Content>`, `<Tooltip.Provider>`.

- [ ] **Step 2: Replace the `tools` array with icon-aware version (line ~475)**

Replace:
```typescript
const tools: { key: SceneTool; label: string; shortcut: string }[] = [
  { key: 'select', label: 'tool.select', shortcut: 'Q' },
  { key: 'move',   label: 'tool.move',   shortcut: 'W' },
  { key: 'rotate', label: 'tool.rotate', shortcut: 'E' },
  { key: 'scale',  label: 'tool.scale',  shortcut: 'R' },
];
```
With:
```typescript
const tools: { key: SceneTool; label: string; shortcut: string; Icon: Component }[] = [
  { key: 'select', label: 'Select',  shortcut: 'Q', Icon: MousePointer2 },
  { key: 'move',   label: 'Move',    shortcut: 'W', Icon: Move },
  { key: 'rotate', label: 'Rotate',  shortcut: 'E', Icon: RotateCw },
  { key: 'scale',  label: 'Scale',   shortcut: 'R', Icon: Maximize2 },
];
```

- [ ] **Step 3: Replace the entire `<div class="viewport-toolbar">` block**

Replace the existing toolbar HTML:
```html
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
```

With:
```html
<Tooltip.Provider delayDuration={400} closeDelay={0}>
<div class="viewport-toolbar">
  <!-- Transform tools -->
  <div class="toolbar-group">
    {#each tools as tool}
      <Tooltip.Root>
        <Tooltip.Trigger>
          {#snippet child({ props })}
            <button
              {...props}
              class="tool-btn"
              class:active={activeTool === tool.key}
              onclick={(e: MouseEvent) => { e.stopPropagation(); activeTool = tool.key; cursor = cursorForTool(tool.key); }}
            >
              <tool.Icon width={14} height={14} />
            </button>
          {/snippet}
        </Tooltip.Trigger>
        <Tooltip.Content class="tooltip-content">
          {tool.label} <span class="tooltip-shortcut">{tool.shortcut}</span>
        </Tooltip.Content>
      </Tooltip.Root>
    {/each}
  </div>

  <div class="toolbar-separator"></div>

  <!-- Grid / Snap -->
  <div class="toolbar-group">
    <Tooltip.Root>
      <Tooltip.Trigger>
        {#snippet child({ props })}
          <button
            {...props}
            class="tool-btn"
            class:active={gridVisible}
            onclick={(e: MouseEvent) => {
              e.stopPropagation();
              gridVisible = !gridVisible;
              viewportSetGridVisible(viewportId, gridVisible);
            }}
          >
            <Grid2X2 width={14} height={14} />
          </button>
        {/snippet}
      </Tooltip.Trigger>
      <Tooltip.Content class="tooltip-content">Grid</Tooltip.Content>
    </Tooltip.Root>

    <Tooltip.Root>
      <Tooltip.Trigger>
        {#snippet child({ props })}
          <button
            {...props}
            class="tool-btn"
            class:active={snapToGrid}
            onclick={(e: MouseEvent) => { e.stopPropagation(); snapToGrid = !snapToGrid; }}
          >
            <Magnet width={14} height={14} />
          </button>
        {/snippet}
      </Tooltip.Trigger>
      <Tooltip.Content class="tooltip-content">Snap to Grid</Tooltip.Content>
    </Tooltip.Root>
  </div>

  <div class="toolbar-separator"></div>

  <!-- Projection toggle -->
  <div class="toolbar-group">
    <Tooltip.Root>
      <Tooltip.Trigger>
        {#snippet child({ props })}
          <button
            {...props}
            class="tool-btn"
            class:active={projection === 'ortho'}
            onclick={(e: MouseEvent) => { e.stopPropagation(); toggleProjection(); }}
          >
            {#if projection === 'ortho'}
              <ScanLine width={14} height={14} />
            {:else}
              <Video width={14} height={14} />
            {/if}
          </button>
        {/snippet}
      </Tooltip.Trigger>
      <Tooltip.Content class="tooltip-content">
        {projection === 'ortho' ? 'Orthographic' : 'Perspective'} <span class="tooltip-shortcut">P</span>
      </Tooltip.Content>
    </Tooltip.Root>
  </div>

  <div class="toolbar-separator"></div>

  <!-- Add entity -->
  <div class="toolbar-group">
    <Tooltip.Root>
      <Tooltip.Trigger>
        {#snippet child({ props })}
          <button
            {...props}
            class="tool-btn"
            onclick={(e: MouseEvent) => { e.stopPropagation(); createEntity(); }}
          >
            <CirclePlus width={14} height={14} />
          </button>
        {/snippet}
      </Tooltip.Trigger>
      <Tooltip.Content class="tooltip-content">Add Entity</Tooltip.Content>
    </Tooltip.Root>
  </div>
</div>
</Tooltip.Provider>
```

- [ ] **Step 4: Update toolbar CSS — replace `.tool-btn` and `.tool-icon` styles**

Replace the existing `.tool-btn` and `.tool-icon` CSS blocks:
```css
.tool-btn {
  background: none;
  border: 1px solid transparent;
  border-radius: 4px;
  color: #777;
  padding: 0;
  cursor: pointer;
  line-height: 1;
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.tool-btn:hover {
  color: #ccc;
  border-color: rgba(255, 255, 255, 0.12);
  background: rgba(255, 255, 255, 0.06);
}

.tool-btn.active {
  color: #61afef;
  border-color: rgba(97, 175, 239, 0.4);
  background: rgba(97, 175, 239, 0.12);
}
```

Remove (delete) the `.tool-icon` block entirely.

Add tooltip styles:
```css
:global(.tooltip-content) {
  background: rgba(20, 20, 20, 0.95);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 4px;
  color: #ccc;
  font-size: 11px;
  padding: 4px 8px;
  pointer-events: none;
  white-space: nowrap;
  z-index: 100;
}

.tooltip-shortcut {
  color: #555;
  margin-left: 4px;
  font-family: monospace;
}
```

- [ ] **Step 5: Run TypeScript compile check**

```bash
cd engine/editor && npx tsc --noEmit 2>&1 | head -30
```
Expected: no errors.

- [ ] **Step 6: Commit**

```bash
git add engine/editor/src/lib/docking/panels/ViewportPanel.svelte
git commit -m "feat(editor): replace toolbar text buttons with Lucide icons + bits-ui tooltips"
```

---

## Task 6: Svelte — Gizmo repositioning + HUD polish + min panel size

**Files:**
- Modify: `engine/editor/src/lib/docking/panels/ViewportPanel.svelte`

- [ ] **Step 1: Grow the axis gizmo SVG and move it to top:8px**

In the gizmo block (line ~570), update the `{@const}` declarations:
```svelte
{@const S = 24}
{@const CX = 40}
{@const CY = 40}
```
(Was `S = 18`, `CX = 30`, `CY = 30`.)

Change the SVG element:
```html
<svg width="80" height="80" viewBox="0 0 80 80">
```
(Was `width="60" height="60" viewBox="0 0 60 60"`.)

Change the text font-size inside the SVG from `font-size="8"` to `font-size="10"` and `stroke-width` from `"0.5"` to `"0.8"`.

- [ ] **Step 2: Update gizmo CSS position and opacity**

Replace:
```css
.axis-gizmo {
  position: absolute;
  top: 40px;
  right: 8px;
  pointer-events: auto;
  opacity: 0.7;
}
```
With:
```css
.axis-gizmo {
  position: absolute;
  top: 8px;
  right: 8px;
  pointer-events: auto;
  opacity: 0.85;
}
```

- [ ] **Step 3: Replace the HUD — remove zoom %, add active tool icon, polish styling**

Replace the entire `<!-- HUD overlay -->` section:
```html
<!-- HUD overlay -->
<div class="viewport-hud">
  <span class="hud-tool" title={t(`tool.${activeTool}` as any)}>
    {activeTool.charAt(0).toUpperCase() + activeTool.slice(1)}
  </span>
  <span class="hud-separator">|</span>
  <span class="hud-projection">{projection === 'ortho' ? 'Ortho' : 'Persp'}</span>
  <span class="hud-separator">|</span>
  <span class="hud-zoom" title={t('viewport.zoom')}>
    {Math.round(cameraZoom * 100)}%
  </span>
  <button
    class="hud-btn"
    onclick={(e: MouseEvent) => {
      e.stopPropagation();
      cameraZoom = 1;
      cameraYawRad = 0.0;
      cameraPitchRad = Math.PI / 6;
      viewportCameraReset(viewportId);
    }}
    title={t('viewport.reset_camera')}
  >
    &#8634;
  </button>
</div>
```
With:
```html
<!-- HUD overlay -->
<div class="viewport-hud">
  <span class="hud-tool">
    {#if activeTool === 'select'}<MousePointer2 width={12} height={12} />
    {:else if activeTool === 'move'}<Move width={12} height={12} />
    {:else if activeTool === 'rotate'}<RotateCw width={12} height={12} />
    {:else if activeTool === 'scale'}<Maximize2 width={12} height={12} />
    {/if}
    <span class="hud-tool-name">{activeTool.charAt(0).toUpperCase() + activeTool.slice(1)}</span>
  </span>
  <span class="hud-separator">|</span>
  <span class="hud-projection">{projection === 'ortho' ? 'Ortho' : 'Persp'}</span>
  <span class="hud-separator">|</span>
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
    <RotateCcw width={12} height={12} />
  </button>
</div>
```

- [ ] **Step 4: Update HUD CSS — remove zoom styles, add tool icon flex, polish**

Replace the existing `.viewport-hud`, `.hud-tool`, `.hud-separator`, `.hud-projection`, `.hud-zoom`, `.hud-btn` blocks with:
```css
.viewport-hud {
  position: absolute;
  bottom: 8px;
  right: 8px;
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 10px;
  background: rgba(0, 0, 0, 0.6);
  backdrop-filter: blur(4px);
  border: 1px solid rgba(255, 255, 255, 0.06);
  border-radius: 4px;
  font-size: 11px;
  color: #aaa;
  pointer-events: auto;
}

.hud-tool {
  display: flex;
  align-items: center;
  gap: 4px;
  color: #ccc;
  font-weight: 500;
}

.hud-tool-name {
  font-size: 11px;
}

.hud-separator {
  color: #333;
}

.hud-projection {
  color: #98c379;
  font-weight: 500;
  font-size: 10px;
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.hud-btn {
  background: none;
  border: none;
  color: #666;
  padding: 0;
  cursor: pointer;
  display: flex;
  align-items: center;
  line-height: 1;
}

.hud-btn:hover {
  color: #ccc;
}
```

- [ ] **Step 5: Add min-width and min-height to `.viewport-container`**

Add to the existing `.viewport-container` block:
```css
min-width: 240px;
min-height: 160px;
```

- [ ] **Step 6: Run TypeScript compile + Vitest**

```bash
cd engine/editor && npx tsc --noEmit 2>&1 | head -20
npx vitest run 2>&1 | tail -10
```
Expected: no type errors, all 34 tests pass.

- [ ] **Step 7: Commit**

```bash
git add engine/editor/src/lib/docking/panels/ViewportPanel.svelte
git commit -m "feat(editor): gizmo top-right 80px, HUD polish with icons, min panel size 240×160"
```
