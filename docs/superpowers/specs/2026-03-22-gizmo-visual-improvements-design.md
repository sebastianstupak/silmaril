# Gizmo Visual Improvements Design

> **Status:** Approved

---

## Goal

Fix the gizmo so it renders only for the selected entity, upgrade geometry to Unity-style with solid
cone/cube tips, and add hover highlighting (brighten the hovered axis colour).

## Architecture

All gizmo rendering lives in `engine/editor/src-tauri/viewport/gizmo_pipeline.rs`.
Shared state (selected entity, gizmo mode, hover) lives in `NativeViewportState`
(`engine/editor/src-tauri/bridge/commands.rs`) and is cloned into the render thread as `Arc<…>`.
The frontend drives hover via a new lightweight `gizmo_hover_test` IPC (read-only, no side
effects) called on mousemove — separate from the existing `gizmo_hit_test` which writes
`DragState` and is only called on mousedown.

**Tech stack:** Rust, Vulkan (Ash), Naga GLSL→SPIR-V, Tauri IPC, Svelte frontend.

---

## Section 1 — Bug Fix: Gizmo Only on Selected Entity

**Current behaviour:** `crosshair_buf` (three short axis lines drawn from an entity's origin) is
emitted for **every** entity in the world, regardless of selection. The mode-specific handles
(move arrows, rotate rings, scale cubes) are already guarded by `is_selected`.

**Fix:** Move the three crosshair `draw_buf` calls (one per axis, each with its own `first_vertex`,
`color`, and `vertex_count`) inside the `if is_selected {` block, immediately before the
`match mode { … }` block. All three calls must move — the X, Y, and Z axis draws are separate.
No new state or IPC needed.

```rust
// gizmo_pipeline.rs — record() (illustrative; move all three draw_buf calls)
if is_selected {
    // crosshair: three separate draw_buf calls for X, Y, Z
    draw_buf(cmd, device, &self.crosshair_buf, …, /*X color*/ …, 0, 2);
    draw_buf(cmd, device, &self.crosshair_buf, …, /*Y color*/ …, 2, 2);
    draw_buf(cmd, device, &self.crosshair_buf, …, /*Z color*/ …, 4, 2);

    match mode {
        GizmoMode::Move   => { … }
        GizmoMode::Rotate => { … }
        GizmoMode::Scale  => { … }
    }
}
```

---

## Section 2 — Solid Geometry Tips

**Current behaviour:** Move cone tips are approximated with 8 spoke lines + a ring (wireframe).
Scale cube tips are 12 edge lines (wireframe). Rotate rings are 32-segment line circles (keep as-is).

**Target:** Unity-style — solid filled cones on move arrows, solid filled cubes on scale handles.
Rotate rings stay as line circles (already readable; solid torus is expensive).

### 2a — Second pipeline: `GizmoSolidPipeline`

The existing `GizmoPipeline` uses `LINE_LIST` topology. Solid fills require `TRIANGLE_LIST`.
Add a `GizmoSolidPipeline` struct that is created alongside `GizmoPipeline` and recorded in
the same rendering pass.

**Pipeline layout:** `GizmoSolidPipeline::new()` creates its **own** `vk::PipelineLayout` with
the same push constant range (112 bytes, stages `VERTEX | FRAGMENT`). The layouts are
independently owned — no sharing of handles. Both are destroyed in their respective `Drop` impls.

`GizmoSolidPipeline` requires a `Drop` impl that destroys both its `vk::Pipeline` and its
`vk::PipelineLayout`, mirroring the existing `impl Drop for GizmoPipeline`.

Both pipelines share the same vertex and fragment shaders (position-only vertex, colour output
from push constants). The solid pipeline differs only in `primitive_topology = TRIANGLE_LIST`.
All other state (depth disabled, cull mode NONE, blending, dynamic viewport/scissor) is identical.

### 2b — Solid cone geometry (move tips)

Generator: `generate_move_cone_solid_vertices(axis: GizmoAxis) -> Vec<GizmoVertex>`

6-sided cone; base centred at 0.8 along the axis (shaft end), tip at 1.0:
- 6 side triangles: `(base_i, base_{i+1}, tip)` — drawn with back-face culling disabled
- 6 base triangles: `(center, base_i, base_{i+1})` (cap)
- Total: 12 triangles = 36 vertices per axis

Cone base radius: **0.06**, matching the current wireframe cone ring.

The existing `move_x/y/z_buf` (line shafts, 0.0 → 0.8 along axis) are kept.
The existing wireframe cone geometry in those buffers (spokes + ring) is **removed** — the solid
cone replaces it. The shaft line remains.

### 2c — Solid cube geometry (scale tips)

Generator: `generate_scale_cube_solid_vertices(axis: GizmoAxis) -> Vec<GizmoVertex>`

Axis-aligned cube centred at shaft end (0.85 along the axis), half-size **0.06** — matching the
existing wireframe cube (`HALF = 0.06` in `generate_scale_handle_vertices`):
- 6 faces × 2 triangles = 12 triangles = 36 vertices per axis

The existing `scale_x/y/z_buf` (line shaft, 0.0 → 0.85 along axis) are kept.
The existing wireframe cube edge geometry in those buffers is **removed** — the solid cube
replaces it. The shaft line remains.

### 2d — Hover state on `GizmoPipeline`

`GizmoPipeline` stores an `Arc<AtomicU8>` clone of the hover axis (see Section 3a) as a struct
field. This avoids threading it through every `record()` parameter:

```rust
pub struct GizmoPipeline {
    // … existing fields …
    hovered_gizmo_axis: Arc<AtomicU8>,
}
```

`GizmoSolidPipeline` also stores the same clone.
Both read it once at the top of `record()` to determine which axis colour to brighten.

### 2e — Storage

`GizmoPipeline` gains six additional `vk::Buffer` fields:
```
move_x_cone_solid_buf, move_y_cone_solid_buf, move_z_cone_solid_buf,
scale_x_cube_solid_buf, scale_y_cube_solid_buf, scale_z_cube_solid_buf,
```
All allocated and uploaded at pipeline creation time, same as existing buffers.

---

## Section 3 — Hover Effect: Brighten Hovered Axis

### 3a — Shared hover state

Add to `NativeViewportState`:

```rust
/// Hovered gizmo axis: 0=none, 1=X, 2=Y, 3=Z.
pub hovered_gizmo_axis: std::sync::Arc<std::sync::atomic::AtomicU8>,
```

Initialised to `0` in `Default`. Cloned into the render thread at viewport creation time (same
site where `gizmo_mode` and `selected_entity_id` are cloned), then passed into `GizmoPipeline::new()`
and `GizmoSolidPipeline::new()` as constructor arguments — stored as struct fields (Section 2d).

### 3b — New IPC: `gizmo_hover_test` (read-only)

The existing `gizmo_hit_test` writes a `DragState` side-effect and is called on **mousedown**
only. The new `gizmo_hover_test` is a pure ray-cast with no side effects, called on **mousemove**.

```rust
/// Read-only axis hit-test for hover highlighting. No DragState written.
#[tauri::command]
pub fn gizmo_hover_test(
    viewport_id: u64,
    screen_x: f32,
    screen_y: f32,
    viewport_state: tauri::State<'_, NativeViewportState>,
) -> Result<Option<String>, String> {
    // Perform the same ray-cast geometry as gizmo_hit_test
    // Return "x" | "y" | "z" | None
    // Do NOT write to drag_state
}
```

And the setter:

```rust
#[tauri::command]
pub fn set_hovered_gizmo_axis(
    axis: Option<String>,   // "x" | "y" | "z" | null/None
    viewport_state: tauri::State<'_, NativeViewportState>,
) -> Result<(), String> {
    let v = match axis.as_deref() {
        Some("x") => 1,
        Some("y") => 2,
        Some("z") => 3,
        _         => 0,  // unknown strings → no hover (silent, intentional)
    };
    viewport_state
        .hovered_gizmo_axis
        .store(v, std::sync::atomic::Ordering::Relaxed);
    Ok(())
}
```

The silent-map-to-zero for unknown strings is intentional for this transient visual hint;
it differs deliberately from `set_gizmo_mode`'s strict validation.
Both commands registered in the Tauri `invoke_handler`. Both are guarded by
`#[cfg(windows)]` in `gizmo_commands.rs` (consistent with existing gizmo commands).
The `NativeViewportState` field and `Default` impl change are platform-independent.

### 3c — Frontend: call from mousemove handler

In `ViewportPanel.svelte`, the existing `handleMouseMove` (lines ~373–451) does not currently
call any hit-test IPC. Add a hover path:

```ts
// Inside handleMouseMove, when not dragging and viewport is focused:
if (!isDraggingGizmo && isTauri) {
  const hit = await invoke<string | null>('gizmo_hover_test', {
    viewportId, screenX, screenY
  });
  await invoke('set_hovered_gizmo_axis', { axis: hit ?? null });
}
```

During an active drag (`isDraggingGizmo === true`) the hover path is skipped — the hovered axis
may be stale until drag ends. This is acceptable: drag state takes precedence over hover state.
On drag end (`handleMouseUp`), the next `mousemove` will correct the hover state automatically.

On `mouseleave` (line ~587, inside `onmouseleave`):
```ts
if (isTauri) await invoke('set_hovered_gizmo_axis', { axis: null });
```

`gizmo_hover_test` acquires read locks on the registry and world. Called at mousemove frequency
(~60 calls/sec while mouse moves) — acceptable; the read is lightweight and lock contention is
low since the render thread also holds the world read-lock for only a brief window.

### 3d — Render: colour brightening

Axis colours in `record()` match the **actual existing values** in the codebase:
```
X = [1.0, 0.2, 0.2, 1.0]   // red
Y = [0.2, 1.0, 0.2, 1.0]   // green
Z = [0.2, 0.4, 1.0, 1.0]   // blue (note: G channel is 0.4, not 0.2)
```

Add a helper (inline function in `gizmo_pipeline.rs`):
```rust
fn axis_color(axis: GizmoAxis, hovered: bool) -> [f32; 4] {
    let base: [f32; 4] = match axis {
        GizmoAxis::X => [1.0, 0.2, 0.2, 1.0],
        GizmoAxis::Y => [0.2, 1.0, 0.2, 1.0],
        GizmoAxis::Z => [0.2, 0.4, 1.0, 1.0],
        _            => [0.8, 0.8, 0.8, 1.0],
    };
    if hovered {
        [
            (base[0] + 0.35).min(1.0),
            (base[1] + 0.35).min(1.0),
            (base[2] + 0.35).min(1.0),
            1.0,
        ]
    } else {
        base
    }
}
```

Replace all hardcoded axis colour literals in `record()` with `axis_color(axis, is_hovered)`.
`is_hovered` is computed once at the top of `record()`:
```rust
let hover_raw = self.hovered_gizmo_axis.load(Ordering::Relaxed);
```
Then for each axis draw: `let is_hovered = hover_raw == axis_index_u8;`

---

## Section 4 — Data Flow Summary

```
Frontend mousemove (not dragging)
  → gizmo_hover_test IPC  → read-only ray-cast → returns axis string or null
  → set_hovered_gizmo_axis IPC → writes AtomicU8 in NativeViewportState

Frontend mouseleave
  → set_hovered_gizmo_axis(null) → writes 0

Frontend mousedown on gizmo
  → gizmo_hit_test IPC (unchanged) → writes DragState

Render thread (each frame)
  reads selected_entity_id (Mutex)
  reads gizmo_mode (AtomicU8)
  reads hovered_gizmo_axis (AtomicU8, from GizmoPipeline struct field)
  → record() draws only for selected entity
  → axis_color(axis, is_hovered) for all push constants
  → LINE_LIST pipeline: shafts + rotate rings
  → TRIANGLE_LIST pipeline: cone tips (move) + cube tips (scale)
```

---

## Section 5 — Error Handling

- `set_hovered_gizmo_axis`: unknown strings silently map to `0` — intentional for transient hover.
- `gizmo_hover_test`: same error surface as `gizmo_hit_test` (registry lock, world read-lock).
- `GizmoSolidPipeline::new()`: same error propagation as `GizmoPipeline::new()`.
- No new failure modes; all new shared state uses lock-free atomics.

---

## Section 6 — Testing

Inline `#[cfg(test)]` unit tests in `engine/editor/src-tauri/` (no Vulkan context required):

1. **`test_cone_vertex_count`** — Call `generate_move_cone_solid_vertices(GizmoAxis::X)`;
   assert `len() == 36` and all vertices are within the bounding box of the cone (shaft end to tip).

2. **`test_cube_vertex_count`** — Call `generate_scale_cube_solid_vertices(GizmoAxis::X)`;
   assert `len() == 36`.

3. **`test_axis_color_hover`** — Assert `axis_color(GizmoAxis::X, true)` returns values brighter
   than `axis_color(GizmoAxis::X, false)` on all RGB channels; assert clamping to `1.0`.

4. **`test_axis_color_z_channel`** — Assert `axis_color(GizmoAxis::Z, false)[1] == 0.4`
   (G channel) to guard the real Z colour against future incorrect "normalisation" to 0.2.

5. **`test_set_hovered_gizmo_axis_ipc`** — Unit test `set_hovered_gizmo_axis` handler directly;
   assert the `AtomicU8` is set to `1` for `"x"`, `2` for `"y"`, `3` for `"z"`, `0` for `null`.

The Section 1 crosshair fix (moving three draw calls inside an existing guard) is too simple to
warrant a Vulkan-context test; correctness is verified by code review.

---

## Files Touched

| File | Change |
|------|--------|
| `engine/editor/src-tauri/viewport/gizmo_pipeline.rs` | Move crosshair draws into `is_selected` block; remove wireframe cone/cube tip geometry; add solid geometry generators + `GizmoSolidPipeline` (own layout + Drop); store `Arc<AtomicU8>` for hover; add `axis_color()` helper |
| `engine/editor/src-tauri/bridge/commands.rs` | Add `hovered_gizmo_axis: Arc<AtomicU8>` to `NativeViewportState` + `Default`; clone Arc at `create_native_viewport` and pass into `NativeViewport::new()` |
| `engine/editor/src-tauri/viewport/native_viewport.rs` | Thread `hovered_gizmo_axis: Arc<AtomicU8>` through `NativeViewport::new()` → `start_rendering()` → `render_loop()` → `ViewportRenderer::new()` → `GizmoPipeline::new()` + `GizmoSolidPipeline::new()` |
| `engine/editor/src-tauri/bridge/gizmo_commands.rs` | Add `gizmo_hover_test` (read-only, uses `screen_x`/`screen_y` matching existing convention) and `set_hovered_gizmo_axis` Tauri commands |
| `engine/editor/src-tauri/main.rs` (or `lib.rs`) | Register `gizmo_hover_test` and `set_hovered_gizmo_axis` in `invoke_handler` |
| `engine/editor/src/lib/docking/panels/ViewportPanel.svelte` | Add hover path in `handleMouseMove` (calls `gizmo_hover_test` + `set_hovered_gizmo_axis`); clear hover in `onmouseleave` |
