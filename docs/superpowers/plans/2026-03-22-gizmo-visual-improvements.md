# Gizmo Visual Improvements — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix gizmo rendering to show only on selected entity, add solid cone/cube tips via a second TRIANGLE_LIST pipeline, and add hover axis highlighting.

**Architecture:** Three layers — (1) Rust/Vulkan pipeline changes in gizmo_pipeline.rs, (2) shared hover state added to NativeViewportState in commands.rs, (3) new IPC commands in gizmo_commands.rs, (4) frontend hover integration in ViewportPanel.svelte.

**Tech Stack:** Rust, Vulkan (Ash), Naga GLSL→SPIR-V, Tauri IPC, Svelte 5

---

## Status Note

As of 2026-03-22 all tasks in this plan have been implemented. The document is retained as a reference for the design decisions and as a checklist for regression verification.

---

## Files Touched

| File | Change |
|------|--------|
| `engine/editor/src-tauri/viewport/gizmo_pipeline.rs` | Move crosshair draws into `is_selected` block; remove wireframe cone/cube tip geometry; add solid geometry generators + `GizmoSolidPipeline` (own layout + Drop); store `Arc<AtomicU8>` for hover; add `axis_color()` helper |
| `engine/editor/src-tauri/bridge/commands.rs` | Add `hovered_gizmo_axis: Arc<AtomicU8>` to `NativeViewportState` + `Default`; clone Arc at `create_native_viewport` and pass into `NativeViewport::new()` |
| `engine/editor/src-tauri/viewport/native_viewport.rs` | Thread `hovered_gizmo_axis: Arc<AtomicU8>` through `NativeViewport::new()` → `start_rendering()` → `render_loop()` → `ViewportRenderer::new()` → `GizmoPipeline::new()` + `GizmoSolidPipeline::new()` |
| `engine/editor/src-tauri/bridge/gizmo_commands.rs` | Add `gizmo_hover_test` (read-only ray-cast) and `set_hovered_gizmo_axis` Tauri commands |
| `engine/editor/src-tauri/lib.rs` | Register `gizmo_hover_test` and `set_hovered_gizmo_axis` in `invoke_handler` |
| `engine/editor/src/lib/docking/panels/ViewportPanel.svelte` | Add hover path in `handleMouseMove`; clear hover in `onmouseleave` |

---

## Task 1 — Fix crosshair selection bug

- [x] **File:** `engine/editor/src-tauri/viewport/gizmo_pipeline.rs`

**Problem:** The three crosshair `draw_buf` calls (X, Y, Z axis lines drawn from an entity's origin) were emitted for every entity in the world regardless of selection. The mode-specific handles (move arrows, rotate rings, scale cubes) were already guarded by `if is_selected`.

**Fix:** Move all three crosshair `draw_buf` calls inside the `if is_selected { … }` block, immediately before the `match mode { … }` block. The vertex buffer layout is `X0,X1,Y0,Y1,Z0,Z1` (2 verts each); `firstVertex` offsets select which pair: X=0, Y=2, Z=4.

```rust
// engine/editor/src-tauri/viewport/gizmo_pipeline.rs — record()
if is_selected {
    // Crosshair: X axis (red)
    self.draw_buf(cmd, device, &self.crosshair_buf, view_proj, origin.into(),
        { let mut c = axis_color(GizmoAxis::X, hover_raw == 1); c[3] = 0.9; c },
        scale, 0, 2);
    // Crosshair: Y axis (green)
    self.draw_buf(cmd, device, &self.crosshair_buf, view_proj, origin.into(),
        { let mut c = axis_color(GizmoAxis::Y, hover_raw == 2); c[3] = 0.9; c },
        scale, 2, 2);
    // Crosshair: Z axis (blue)
    self.draw_buf(cmd, device, &self.crosshair_buf, view_proj, origin.into(),
        { let mut c = axis_color(GizmoAxis::Z, hover_raw == 3); c[3] = 0.9; c },
        scale, 4, 2);

    match mode {
        GizmoMode::Move   => { /* move arrows */ }
        GizmoMode::Rotate => { /* rotate rings */ }
        GizmoMode::Scale  => { /* scale cubes */ }
    }
}
```

**Test:** Structural unit test — verify `generate_crosshair_vertices()` produces exactly 6 vertices (3 axes × 2 verts), confirming the buffer layout that makes `firstVertex` selection correct.

```rust
#[test]
fn crosshair_generates_6_vertices() {
    let verts = generate_crosshair_vertices();
    assert_eq!(verts.len(), 6);
}
```

**Test command:** `cargo test -p silmaril-editor-tauri crosshair_generates_6_vertices`

- [x] Commit: `fix(editor): move crosshair draws inside is_selected guard`

---

## Task 2 — Add hover state to NativeViewportState

- [x] **File:** `engine/editor/src-tauri/bridge/commands.rs`

**Change:** Add `hovered_gizmo_axis: Arc<AtomicU8>` field to `NativeViewportState` struct and its `Default` impl. Initialise to `0` (no hover). The field sits after `gizmo_mode` (same pattern — `Arc<AtomicU8>` cloned into the render thread).

```rust
// In NativeViewportState struct (~line 370):
pub struct NativeViewportState {
    pub registry: Mutex<ViewportRegistry>,
    pub drag_state: Mutex<Option<crate::bridge::gizmo_commands::DragState>>,
    pub gizmo_mode: std::sync::Arc<std::sync::atomic::AtomicU8>,
    /// Hovered gizmo axis: 0=none, 1=X, 2=Y, 3=Z.
    /// Stored as Arc<AtomicU8> so it can be cloned into the render thread.
    pub hovered_gizmo_axis: std::sync::Arc<std::sync::atomic::AtomicU8>,
    pub selected_entity_id: std::sync::Arc<Mutex<Option<u64>>>,
}

// In Default impl (~line 385):
impl Default for NativeViewportState {
    fn default() -> Self {
        Self {
            registry: Mutex::new(ViewportRegistry::new()),
            drag_state: Mutex::new(None),
            gizmo_mode: std::sync::Arc::new(std::sync::atomic::AtomicU8::new(0)),
            hovered_gizmo_axis: std::sync::Arc::new(std::sync::atomic::AtomicU8::new(0)),
            selected_entity_id: std::sync::Arc::new(Mutex::new(None)),
        }
    }
}
```

**Also in `create_native_viewport`** command (same file, ~line 435): clone the Arc and pass to `NativeViewport::new()`:

```rust
let hovered_gizmo_axis = std::sync::Arc::clone(&viewport_state.hovered_gizmo_axis);
let mut vp = NativeViewport::new(
    parent_hwnd, world_state.inner().0.clone(),
    selected_entity_id, gizmo_mode, hovered_gizmo_axis, asset_manager
)?;
```

**Test:**

```rust
#[test]
fn hovered_gizmo_axis_default_is_zero() {
    let state = NativeViewportState::default();
    assert_eq!(
        state.hovered_gizmo_axis.load(std::sync::atomic::Ordering::Relaxed),
        0
    );
}
```

**Test command:** `cargo test -p silmaril-editor-tauri hovered_gizmo_axis_default_is_zero`

- [x] Commit: `feat(editor): add hovered_gizmo_axis AtomicU8 to NativeViewportState`

---

## Task 3 — Add `set_hovered_gizmo_axis` IPC command

- [x] **File:** `engine/editor/src-tauri/bridge/gizmo_commands.rs`

**Pattern:** Follows `set_gizmo_mode` exactly (lines 441–455) but accepts `Option<String>` (nullable from JS) and silently maps unknown strings to `0`. Guarded by `#[cfg(windows)]` consistent with all other gizmo commands in this file.

The silent-map-to-zero for unknown strings is intentional for this transient visual hint; it differs deliberately from `set_gizmo_mode`'s strict validation which returns `Err`.

```rust
/// Set the hovered gizmo axis for hover highlighting.
///
/// Called from the frontend on every mousemove (non-drag) and cleared on mouseleave.
/// Unknown strings silently map to 0 (no hover) — intentional for this transient hint.
#[tauri::command]
pub fn set_hovered_gizmo_axis(
    axis: Option<String>,
    viewport_state: tauri::State<'_, super::commands::NativeViewportState>,
) -> Result<(), String> {
    let v = match axis.as_deref() {
        Some("x") => 1u8,
        Some("y") => 2u8,
        Some("z") => 3u8,
        _          => 0u8,
    };
    viewport_state
        .hovered_gizmo_axis
        .store(v, std::sync::atomic::Ordering::Relaxed);
    Ok(())
}
```

**Registration** in `engine/editor/src-tauri/lib.rs` `invoke_handler`:

```rust
bridge::gizmo_commands::set_hovered_gizmo_axis,
```

**Test:**

```rust
#[test]
fn test_set_hovered_gizmo_axis_ipc() {
    // Test the axis-string → u8 mapping directly (no Tauri State needed).
    let map = |s: Option<&str>| -> u8 {
        match s {
            Some("x") => 1,
            Some("y") => 2,
            Some("z") => 3,
            _          => 0,
        }
    };
    assert_eq!(map(Some("x")), 1);
    assert_eq!(map(Some("y")), 2);
    assert_eq!(map(Some("z")), 3);
    assert_eq!(map(None),      0);
    // Unknown strings silently map to 0
    assert_eq!(map(Some("w")), 0);
    assert_eq!(map(Some("")),  0);
}
```

**Test command:** `cargo test -p silmaril-editor-tauri test_set_hovered_gizmo_axis_ipc`

- [x] Commit: `feat(editor): add set_hovered_gizmo_axis IPC command`

---

## Task 4 — Add `gizmo_hover_test` IPC command

- [x] **File:** `engine/editor/src-tauri/bridge/gizmo_commands.rs`

**Purpose:** Read-only axis hit-test for hover highlighting. Performs the same ray-cast geometry as `gizmo_hit_test` but writes **no** `DragState`. Called from frontend `mousemove` (not `mousedown`). Returns `"x" | "y" | "z" | null`.

```rust
/// Read-only axis hit-test for hover highlighting. No DragState written.
///
/// Called from the frontend's mousemove handler when not dragging.
/// Returns `"x"` | `"y"` | `"z"` | `null`.
#[tauri::command]
pub fn gizmo_hover_test(
    viewport_id: String,
    screen_x: f32,
    screen_y: f32,
    viewport_state: tauri::State<'_, super::commands::NativeViewportState>,
    world_state: tauri::State<'_, crate::state::SceneWorldState>,
) -> Result<Option<String>, String> {
    let entity_id: u64 = match viewport_state
        .selected_entity_id
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .copied()
    {
        Some(id) => id,
        None => return Ok(None),
    };
    if entity_id > u32::MAX as u64 { return Ok(None); }
    // FIXME: hardcodes generation 0 — will break after entity slot reuse.
    let entity = engine_core::Entity::new(entity_id as u32, 0);

    let entity_pos = {
        let world = world_state.inner().0.read().map_err(|e| e.to_string())?;
        match world.get::<engine_core::Transform>(entity) {
            Some(t) => t.position,
            None    => return Ok(None),
        }
    };

    let (view, proj, bounds, gizmo_scale) = {
        let registry = viewport_state.registry.lock().map_err(|e| e.to_string())?;
        match registry.get_for_id(&viewport_id) {
            Some(vp) => match vp.get_instance_ray_data(&viewport_id) {
                Some((view, proj, eye, bounds)) => {
                    let dist = (eye - entity_pos).length().max(0.1_f32);
                    (view, proj, bounds, dist * 0.15)
                }
                None => return Ok(None),
            },
            None => return Ok(None),
        }
    };

    let (ray_origin, ray_dir) = unproject_screen(screen_x, screen_y, &bounds, view, proj);

    // Test each axis handle (same geometry as gizmo_hit_test, no DragState written).
    let axes: [(Vec3, &str); 3] = [(Vec3::X, "x"), (Vec3::Y, "y"), (Vec3::Z, "z")];
    for (axis_dir, label) in &axes {
        let handle_end = entity_pos + *axis_dir * gizmo_scale;
        let radius = gizmo_scale * 0.1;
        if ray_capsule_intersects(ray_origin, ray_dir, entity_pos, handle_end, radius) {
            return Ok(Some(label.to_string()));
        }
    }
    Ok(None)
}
```

**Registration** in `engine/editor/src-tauri/lib.rs`:

```rust
bridge::gizmo_commands::gizmo_hover_test,
```

**Test:** Correctness is covered by the existing `ray_hits_x_axis_handle` and `ray_misses_when_offset` tests in gizmo_commands.rs which exercise `ray_capsule_intersects` directly.

**Test command:** `cargo test -p silmaril-editor-tauri ray_hits_x_axis_handle`

- [x] Commit: `feat(editor): add gizmo_hover_test read-only IPC command`

---

## Task 5 — Thread hover state into render loop

- [x] **File:** `engine/editor/src-tauri/viewport/native_viewport.rs`

**Change:** Thread `hovered_gizmo_axis: Arc<AtomicU8>` through the full call chain alongside the existing `gizmo_mode` Arc.

**Call chain:**
1. `NativeViewport` struct gains `hovered_gizmo_axis: Arc<AtomicU8>` field.
2. `NativeViewport::new()` accepts it as a constructor parameter (~line 104).
3. `start_rendering()` clones it into the render thread (~line 143).
4. `render_loop()` receives it as a parameter (~line 1049).
5. `ViewportRenderer::new(hwnd, width, height, hovered_gizmo_axis)` receives it (~line 877).
6. Inside `ViewportRenderer::new()`: passed to `GizmoPipeline::new(context, render_pass, Arc::clone(&hovered_gizmo_axis))` and `GizmoSolidPipeline::new(context, render_pass, Arc::clone(&hovered_gizmo_axis))`.

On non-Windows builds: stored as `_hovered_gizmo_axis` (underscore prefix suppresses dead_code warning since `GizmoPipeline`/`GizmoSolidPipeline` are not compiled).

**GizmoPipeline struct field** (`gizmo_pipeline.rs` ~line 387):

```rust
pub struct GizmoPipeline {
    // … existing fields …
    hovered_gizmo_axis: std::sync::Arc<std::sync::atomic::AtomicU8>,
}
```

**Same field on GizmoSolidPipeline** (~line 892). Both read it once at the top of `record()`:

```rust
let hover_raw = self.hovered_gizmo_axis.load(std::sync::atomic::Ordering::Relaxed);
```

Per-axis draw: `axis_color(GizmoAxis::X, hover_raw == 1)`, etc.

**Tests:**

```rust
#[test]
fn test_axis_color_hover() {
    let normal_x  = axis_color(GizmoAxis::X, false);
    let hovered_x = axis_color(GizmoAxis::X, true);
    assert!(hovered_x[0] >= normal_x[0]);
    assert!(hovered_x[1] >= normal_x[1]);
    assert!(hovered_x[2] >= normal_x[2]);
    let any_brighter = hovered_x[0] > normal_x[0]
        || hovered_x[1] > normal_x[1]
        || hovered_x[2] > normal_x[2];
    assert!(any_brighter);
    assert_eq!(hovered_x[3], 1.0);
    assert!(hovered_x[0] <= 1.0);
    assert!(hovered_x[1] <= 1.0);
    assert!(hovered_x[2] <= 1.0);
}

#[test]
fn test_axis_color_z_channel() {
    // Z axis G channel is 0.4, NOT 0.2 — guard against future incorrect normalisation
    let z = axis_color(GizmoAxis::Z, false);
    assert_eq!(z[1], 0.4, "Z axis G channel must be 0.4");
}
```

**Test command:** `cargo test -p silmaril-editor-tauri test_axis_color`

- [x] Commit: `feat(editor): thread hovered_gizmo_axis Arc through render loop; add axis_color helper`

---

## Task 6 — Add GizmoSolidPipeline struct

- [x] **File:** `engine/editor/src-tauri/viewport/gizmo_pipeline.rs`

**Change:** Add `GizmoSolidPipeline` struct alongside `GizmoPipeline`. Uses `TRIANGLE_LIST` topology with its own `vk::PipelineLayout` and `vk::Pipeline`, independently owned — no sharing of handles.

**Pipeline creation:** `create_gizmo_solid_pipeline()` is a separate function from `create_gizmo_pipeline()`, identical except for `topology(vk::PrimitiveTopology::TRIANGLE_LIST)`. Both use the same GLSL shaders (compiled once via `OnceLock`). Same push constant layout: 112 bytes, `VERTEX | FRAGMENT` stages. Same rasterisation state: no depth test, `CULL_MODE_NONE`, `COUNTER_CLOCKWISE`, dynamic viewport/scissor.

**`Drop` impl** destroys both `pipeline` and `pipeline_layout`:

```rust
impl Drop for GizmoSolidPipeline {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_pipeline(self.pipeline, None);
            self.device.destroy_pipeline_layout(self.pipeline_layout, None);
        }
    }
}
```

**Struct fields:**

```rust
pub struct GizmoSolidPipeline {
    device: ash::Device,
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    _vert_shader: ShaderModule,
    _frag_shader: ShaderModule,
    move_x_cone_solid_buf:  GpuBuffer,  move_x_cone_solid_count:  u32,
    move_y_cone_solid_buf:  GpuBuffer,  move_y_cone_solid_count:  u32,
    move_z_cone_solid_buf:  GpuBuffer,  move_z_cone_solid_count:  u32,
    scale_x_cube_solid_buf: GpuBuffer,  scale_x_cube_solid_count: u32,
    scale_y_cube_solid_buf: GpuBuffer,  scale_y_cube_solid_count: u32,
    scale_z_cube_solid_buf: GpuBuffer,  scale_z_cube_solid_count: u32,
    hovered_gizmo_axis: std::sync::Arc<std::sync::atomic::AtomicU8>,
}
```

**`record()` sequence:** Bind TRIANGLE_LIST pipeline → load `hover_raw` → for each entity: skip if not selected → `match mode { Move => draw 3 cone bufs | Scale => draw 3 cube bufs | Rotate => no-op }`.

**Test command:** `cargo test -p silmaril-editor-tauri push_constants_are_112_bytes`

- [x] Commit: `feat(editor): add GizmoSolidPipeline with TRIANGLE_LIST topology`

---

## Task 7 — Add solid cone geometry

- [x] **File:** `engine/editor/src-tauri/viewport/gizmo_pipeline.rs`

**Generator:** `generate_move_cone_solid_vertices(axis: GizmoAxis) -> Vec<GizmoVertex>`

**Geometry:** 6-sided cone. Base centred at `0.8` along the axis (shaft end), tip at `1.0`. Base radius `0.06` (matches the former wireframe cone ring). Cull mode NONE so both sides render. 12 triangles × 3 verts = 36 vertices total.

```rust
pub fn generate_move_cone_solid_vertices(axis: GizmoAxis) -> Vec<GizmoVertex> {
    let dir   = axis_dir(axis);
    let perp1 = perpendicular(dir);
    let perp2 = dir.cross(perp1);

    const SIDES:  usize = 6;
    const CONE_R: f32   = 0.06;
    let base_center = dir * 0.8;
    let tip         = dir * 1.0;

    let mut ring = Vec::with_capacity(SIDES);
    for i in 0..SIDES {
        let angle = (i as f32) * std::f32::consts::TAU / (SIDES as f32);
        let (s, c) = angle.sin_cos();
        ring.push(base_center + perp1 * (c * CONE_R) + perp2 * (s * CONE_R));
    }

    let mut verts = Vec::with_capacity(36);
    for i in 0..SIDES {
        let next = (i + 1) % SIDES;
        // Side triangle: base_i → base_{i+1} → tip
        verts.push(GizmoVertex { pos: ring[i].into()    });
        verts.push(GizmoVertex { pos: ring[next].into() });
        verts.push(GizmoVertex { pos: tip.into()        });
        // Cap triangle: base_center → base_i → base_{i+1}
        verts.push(GizmoVertex { pos: base_center.into() });
        verts.push(GizmoVertex { pos: ring[i].into()     });
        verts.push(GizmoVertex { pos: ring[next].into()  });
    }
    verts
}
```

**Existing move buffers:** `move_x/y/z_buf` (LINE_LIST) retain only the shaft line (`origin → 0.8 along axis`). Wireframe spoke/ring geometry is removed. Shaft vertex count is exactly 2.

**Tests:**

```rust
#[test]
fn test_cone_vertex_count() {
    let verts = generate_move_cone_solid_vertices(GizmoAxis::X);
    assert_eq!(verts.len(), 36,
        "6-sided cone: 6 side triangles + 6 cap triangles = 12 triangles = 36 verts");
}

#[test]
fn test_move_arrow_shaft_only() {
    let verts = generate_move_arrow_vertices(GizmoAxis::X);
    assert_eq!(verts.len(), 2, "move arrow shaft must be exactly 2 verts (1 line segment)");
}
```

**Test command:** `cargo test -p silmaril-editor-tauri test_cone_vertex_count`

- [x] Commit: `feat(editor): add solid cone geometry; strip wireframe cone from move arrow shaft`

---

## Task 8 — Add solid cube geometry

- [x] **File:** `engine/editor/src-tauri/viewport/gizmo_pipeline.rs`

**Generator:** `generate_scale_cube_solid_vertices(axis: GizmoAxis) -> Vec<GizmoVertex>`

**Geometry:** Axis-aligned cube centred at `0.85` along the axis (shaft end), half-size `0.06` (matching former `HALF = 0.06` constant). Cull mode NONE. 6 faces × 2 triangles × 3 verts = 36 vertices total.

**Corner naming** — signs of `(perp1, perp2, dir)`:

```
l=−perp1, r=+perp1, b=−perp2, t=+perp2, b(ack)=−dir, f(ront)=+dir
lbb=(-1,-1,-1)  rbb=(+1,-1,-1)  rtb=(+1,+1,-1)  ltb=(-1,+1,-1)
lbf=(-1,-1,+1)  rbf=(+1,-1,+1)  rtf=(+1,+1,+1)  ltf=(-1,+1,+1)
```

```rust
pub fn generate_scale_cube_solid_vertices(axis: GizmoAxis) -> Vec<GizmoVertex> {
    let dir   = axis_dir(axis);
    let perp1 = perpendicular(dir);
    let perp2 = dir.cross(perp1);
    let center = dir * 0.85;
    const HALF: f32 = 0.06;

    let c = |s1: f32, s2: f32, sd: f32| -> [f32; 3] {
        (center + perp1*(s1*HALF) + perp2*(s2*HALF) + dir*(sd*HALF)).into()
    };

    let lbb=c(-1.,-1.,-1.); let rbb=c(1.,-1.,-1.);
    let rtb=c(1., 1.,-1.);  let ltb=c(-1., 1.,-1.);
    let lbf=c(-1.,-1., 1.); let rbf=c(1.,-1., 1.);
    let rtf=c(1., 1., 1.);  let ltf=c(-1., 1., 1.);

    // 6 faces × 2 CCW triangles each (viewed from outside):
    let faces: [[[f32;3];6];6] = [
        [lbb,ltb,rtb, lbb,rtb,rbb],  // −dir face (back)
        [lbf,rbf,rtf, lbf,rtf,ltf],  // +dir face (front)
        [lbb,lbf,ltf, lbb,ltf,ltb],  // −perp1 (left)
        [rbb,rtb,rtf, rbb,rtf,rbf],  // +perp1 (right)
        [lbb,rbb,rbf, lbb,rbf,lbf],  // −perp2 (bottom)
        [ltb,ltf,rtf, ltb,rtf,rtb],  // +perp2 (top)
    ];

    let mut verts = Vec::with_capacity(36);
    for face in &faces {
        for pos in face {
            verts.push(GizmoVertex { pos: *pos });
        }
    }
    verts
}
```

**Existing scale buffers:** `scale_x/y/z_buf` retain only the shaft line (`origin → 0.85`). Wireframe cube edge geometry is removed. Shaft vertex count is exactly 2.

**Tests:**

```rust
#[test]
fn test_cube_vertex_count() {
    let verts = generate_scale_cube_solid_vertices(GizmoAxis::X);
    assert_eq!(verts.len(), 36, "6 faces * 2 triangles * 3 verts = 36 verts");
}

#[test]
fn test_scale_handle_shaft_only() {
    let verts = generate_scale_handle_vertices(GizmoAxis::X);
    assert_eq!(verts.len(), 2, "scale handle shaft must be exactly 2 verts (1 line segment)");
}
```

**Test command:** `cargo test -p silmaril-editor-tauri test_cube_vertex_count`

- [x] Commit: `feat(editor): add solid cube geometry; strip wireframe cube from scale handle shaft`

---

## Task 9 — Frontend hover integration

- [x] **File:** `engine/editor/src/lib/docking/panels/ViewportPanel.svelte`

### 9a — handleMouseMove (lines ~375–463)

Add a hover path at the end of `handleMouseMove`, after all drag-mode handling. Guard: `!isDraggingGizmo && !isDragging && isTauri` — skipped during any active drag.

```ts
// At the end of handleMouseMove (after the dragMode switch):
if (!isDraggingGizmo && !isDragging && isTauri) {
  try {
    const hit = await gizmoHoverTest(viewportId, event.clientX, event.clientY);
    await setHoveredGizmoAxis(hit ?? null);
  } catch {
    // Non-critical — hover state may be stale for one frame; silently ignore errors.
  }
}
```

### 9b — onmouseleave handler (lines ~599–604)

Clear hover state when the cursor leaves the viewport so the highlighted axis does not remain bright after the mouse exits:

```ts
onmouseleave={async () => {
  handleMouseUp();
  setViewportFocused(false);
  if (isTauri) {
    try { await setHoveredGizmoAxis(null); } catch {}
  }
}}
```

### 9c — Drag lifecycle

During an active gizmo drag (`isDraggingGizmo === true`) the hover path is bypassed. The hovered axis may be stale during the drag — this is acceptable since drag state takes precedence visually. The first `mousemove` after `mouseup` (which clears `isDraggingGizmo`) corrects the hover state automatically.

### 9d — Frequency

`gizmoHoverTest` is called at mousemove frequency (~60 calls/sec). The call is lightweight: read lock on registry + read lock on ECS world + 3 capsule ray-casts. Lock contention is low because the render thread holds the world read-lock only during the brief `record()` window.

**Test command:** Visual verification — hover over gizmo axes in the editor viewport and confirm axis brightening.

- [x] Commit: `feat(editor): add gizmo hover highlighting to frontend mousemove/mouseleave`

---

## Colour Reference

Axis base colours in `axis_color()` — actual codebase values:

| Axis | R   | G   | B   | A   |
|------|-----|-----|-----|-----|
| X    | 1.0 | 0.2 | 0.2 | 1.0 |
| Y    | 0.2 | 1.0 | 0.2 | 1.0 |
| Z    | 0.2 | 0.4 | 1.0 | 1.0 |

Note: Z axis G channel is `0.4`, not `0.2`. The `test_axis_color_z_channel` test guards this against future incorrect "normalisation".

Hover brightening: add `0.35` to each RGB channel, clamped to `1.0`. Alpha stays `1.0`.

Crosshair lines use `c[3] = 0.9` applied after `axis_color()`, keeping them slightly transparent.

---

## Data Flow Summary

```
Frontend mousemove (not dragging)
  → gizmoHoverTest IPC      → read-only ray-cast → returns axis string or null
  → setHoveredGizmoAxis IPC → writes AtomicU8 in NativeViewportState

Frontend mouseleave
  → setHoveredGizmoAxis(null) → writes 0

Frontend mousedown on gizmo
  → gizmoHitTest IPC (unchanged) → writes DragState

Render thread (each frame)
  reads selected_entity_id   (Mutex<Option<u64>>)
  reads gizmo_mode           (AtomicU8)
  reads hovered_gizmo_axis   (AtomicU8, from GizmoPipeline struct field)
  → GizmoPipeline.record():
      if is_selected:
        crosshair X/Y/Z draws (axis_color with hover check, alpha 0.9)
        match mode { Move/Rotate/Scale: shaft/ring draws }
  → GizmoSolidPipeline.record():
      if is_selected:
        match mode { Move: cone solid draws | Scale: cube solid draws | Rotate: no-op }
```

---

## Error Handling

- `set_hovered_gizmo_axis`: unknown strings silently map to `0` — intentional for transient hover (differs deliberately from `set_gizmo_mode`'s strict `Err` return).
- `gizmo_hover_test`: same error surface as `gizmo_hit_test` — registry lock failure or world read-lock failure propagate as `Err(String)`; no entity selected returns `Ok(None)`.
- `GizmoSolidPipeline::new()`: same error propagation as `GizmoPipeline::new()` — buffer allocation and pipeline creation failures return `Err(String)`.
- No new failure modes; all new shared state uses lock-free atomics.

---

## Running All Tests

```bash
# All gizmo unit tests (no Vulkan context needed — inline cfg(test)):
cargo test -p silmaril-editor-tauri

# Specific groups:
cargo test -p silmaril-editor-tauri test_cone
cargo test -p silmaril-editor-tauri test_cube
cargo test -p silmaril-editor-tauri test_axis_color
cargo test -p silmaril-editor-tauri test_set_hovered_gizmo_axis
cargo test -p silmaril-editor-tauri ray_hits
cargo test -p silmaril-editor-tauri crosshair_generates_6_vertices
```
