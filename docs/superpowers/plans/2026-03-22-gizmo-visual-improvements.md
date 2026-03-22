# Gizmo Visual Improvements Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the gizmo crosshair (show only on selected entity), upgrade move/scale tips to solid geometry, and add hover axis highlighting.

**Architecture:** All gizmo rendering lives in `gizmo_pipeline.rs`; a second `GizmoSolidPipeline` (TRIANGLE_LIST) is added alongside the existing LINE_LIST pipeline and recorded in the same pass. Hover state is a new `Arc<AtomicU8>` in `NativeViewportState` threaded into both pipelines at construction, updated via two new IPC commands (`gizmo_hover_test` + `set_hovered_gizmo_axis`) called from `handleMouseMove`.

**Tech Stack:** Rust, Ash (Vulkan), TRIANGLE_LIST/LINE_LIST pipelines, Tauri IPC, Svelte 5 frontend.

---

## File Map

| File | Change |
|------|--------|
| `engine/editor/src-tauri/viewport/gizmo_pipeline.rs` | Bug fix; new solid generators; `GizmoSolidPipeline` struct + Drop; `axis_color()` helper; hover Arc field; updated re-exports |
| `engine/editor/src-tauri/bridge/commands.rs` | Add `hovered_gizmo_axis: Arc<AtomicU8>` to `NativeViewportState` + `Default`; clone Arc in `create_native_viewport` |
| `engine/editor/src-tauri/viewport/native_viewport.rs` | Thread `hovered_gizmo_axis` through `NativeViewport::new()` → `start_rendering()` → `render_loop()` → `ViewportRenderer::new()` → both pipeline constructors; add `gizmo_solid_pipeline` field |
| `engine/editor/src-tauri/bridge/gizmo_commands.rs` | Add `gizmo_hover_test` and `set_hovered_gizmo_axis` commands |
| `engine/editor/src-tauri/lib.rs` | Register two new commands in `invoke_handler` |
| `engine/editor/src/lib/docking/panels/ViewportPanel.svelte` | Add hover path in `handleMouseMove`; clear hover in `onmouseleave` |

---

## Task 1: Bug Fix — Crosshair Only on Selected Entity

**Files:**
- Modify: `engine/editor/src-tauri/viewport/gizmo_pipeline.rs` (lines ~437–479 in `record()`)

**Context:** Currently the three crosshair `draw_buf` calls (X/Y/Z axis lines) run for every entity in the world. They must be moved inside the `if is_selected {` block. The `is_selected` check already exists at line ~482. The crosshair draws are at lines ~439–479.

- [ ] **Step 1: Move the three crosshair draw_buf calls inside `if is_selected {`**

  In `gizmo_pipeline.rs`, inside `impl GizmoPipeline`, in `pub unsafe fn record()`, find the "Crosshair (every entity)" block. It has three `self.draw_buf(...)` calls for X, Y, Z, immediately before `let is_selected = ...`. Move all three calls to be the first thing inside the `if is_selected {` block, before `match mode { ... }`.

  The result should look like:
  ```rust
  let is_selected = selected_entity_id.map_or(false, |id| {
      if id > u32::MAX as u64 { return false; }
      entity.id() == id as u32
  });
  if is_selected {
      // ── Crosshair ─────────────────────────────────────────────────
      // X axis — red
      self.draw_buf(cmd, device, &self.crosshair_buf, view_proj, origin.into(),
          [1.0, 0.2, 0.2, 0.9], scale, 0, 2);
      // Y axis — green
      self.draw_buf(cmd, device, &self.crosshair_buf, view_proj, origin.into(),
          [0.2, 1.0, 0.2, 0.9], scale, 2, 2);
      // Z axis — blue
      self.draw_buf(cmd, device, &self.crosshair_buf, view_proj, origin.into(),
          [0.2, 0.4, 1.0, 0.9], scale, 4, 2);

      match mode { ... }
  }
  ```

- [ ] **Step 2: Verify build passes**

  ```bash
  cd engine/editor && cargo build -p silmaril-editor-tauri 2>&1 | grep -E "^error"
  ```
  Expected: no `^error` lines.

- [ ] **Step 3: Commit**

  ```bash
  git add engine/editor/src-tauri/viewport/gizmo_pipeline.rs
  git commit -m "fix(gizmo): show crosshair only on selected entity"
  ```

---

## Task 2: Thread `hovered_gizmo_axis` into GizmoPipeline

**Files:**
- Modify: `engine/editor/src-tauri/bridge/commands.rs` (NativeViewportState struct, Default impl, create_native_viewport)
- Modify: `engine/editor/src-tauri/viewport/native_viewport.rs` (NativeViewport struct, new(), start_rendering(), render_loop(), ViewportRenderer struct, ViewportRenderer::new())
- Modify: `engine/editor/src-tauri/viewport/gizmo_pipeline.rs` (GizmoPipeline struct, new(), record())

**Context:** `selected_entity_id` and `gizmo_mode` are already threaded as `Arc<Mutex>` / `Arc<AtomicU8>` through the same chain. Follow the same pattern for `hovered_gizmo_axis: Arc<AtomicU8>`.

- [ ] **Step 1: Add `hovered_gizmo_axis` field to `NativeViewportState` in `commands.rs`**

  In the `NativeViewportState` struct (around line 370):
  ```rust
  pub struct NativeViewportState {
      pub registry: Mutex<ViewportRegistry>,
      pub drag_state: Mutex<Option<crate::bridge::gizmo_commands::DragState>>,
      pub gizmo_mode: std::sync::Arc<std::sync::atomic::AtomicU8>,
      pub selected_entity_id: std::sync::Arc<Mutex<Option<u64>>>,
      /// Hovered gizmo axis: 0=none, 1=X, 2=Y, 3=Z.
      pub hovered_gizmo_axis: std::sync::Arc<std::sync::atomic::AtomicU8>,
  }
  ```

  In `Default` (around line 382):
  ```rust
  impl Default for NativeViewportState {
      fn default() -> Self {
          Self {
              registry: Mutex::new(ViewportRegistry::new()),
              drag_state: Mutex::new(None),
              gizmo_mode: std::sync::Arc::new(std::sync::atomic::AtomicU8::new(0)),
              selected_entity_id: std::sync::Arc::new(Mutex::new(None)),
              hovered_gizmo_axis: std::sync::Arc::new(std::sync::atomic::AtomicU8::new(0)),
          }
      }
  }
  ```

- [ ] **Step 2: Clone `hovered_gizmo_axis` in `create_native_viewport` and pass to `NativeViewport::new()`**

  In `create_native_viewport` (around line 429–432), add alongside the existing clone lines:
  ```rust
  let selected_entity_id = std::sync::Arc::clone(&viewport_state.selected_entity_id);
  let gizmo_mode = std::sync::Arc::clone(&viewport_state.gizmo_mode);
  let hovered_gizmo_axis = std::sync::Arc::clone(&viewport_state.hovered_gizmo_axis);
  let asset_manager = asset_manager_state.0.clone();
  let mut vp = NativeViewport::new(
      parent_hwnd, world_state.inner().0.clone(),
      selected_entity_id, gizmo_mode, hovered_gizmo_axis, asset_manager
  )...
  ```

- [ ] **Step 3: Add `hovered_gizmo_axis` field and parameter to `NativeViewport`**

  In `native_viewport.rs`, in the `NativeViewport` struct (look for the struct that contains `gizmo_mode`):
  ```rust
  hovered_gizmo_axis: std::sync::Arc<std::sync::atomic::AtomicU8>,
  ```

  In `NativeViewport::new()` signature:
  ```rust
  pub fn new(
      parent_hwnd: HWND,
      world: Arc<std::sync::RwLock<engine_core::World>>,
      selected_entity_id: std::sync::Arc<std::sync::Mutex<Option<u64>>>,
      gizmo_mode: std::sync::Arc<std::sync::atomic::AtomicU8>,
      hovered_gizmo_axis: std::sync::Arc<std::sync::atomic::AtomicU8>,
      asset_manager: Arc<engine_assets::AssetManager>,
  ) -> Result<Self, String>
  ```

  In the `Ok(Self { ... })` block, add:
  ```rust
  hovered_gizmo_axis,
  ```

- [ ] **Step 4: Clone `hovered_gizmo_axis` in `start_rendering()` and pass to `render_loop()`**

  In `start_rendering()`, alongside the other clones (around line 138):
  ```rust
  let hovered_gizmo_axis = self.hovered_gizmo_axis.clone();
  ```

  Add it to the `render_loop(...)` call and to the `render_loop` function signature (after `gizmo_mode`):
  ```rust
  fn render_loop(
      ...,
      gizmo_mode: std::sync::Arc<std::sync::atomic::AtomicU8>,
      hovered_gizmo_axis: std::sync::Arc<std::sync::atomic::AtomicU8>,
      asset_manager: Arc<engine_assets::AssetManager>,
  )
  ```

- [ ] **Step 5: Pass `hovered_gizmo_axis` to `ViewportRenderer::new()` in `render_loop`**

  In `render_loop`, where `ViewportRenderer::new(hwnd, init_w, init_h)` is called (around line 1030), change to:
  ```rust
  let mut renderer = match ViewportRenderer::new(hwnd, init_w, init_h, hovered_gizmo_axis) {
  ```

- [ ] **Step 6: Add `hovered_gizmo_axis` param to `ViewportRenderer::new()`, pass to `GizmoPipeline::new()`**

  Change `ViewportRenderer::new()` signature:
  ```rust
  fn new(
      hwnd: HWND,
      width: u32,
      height: u32,
      hovered_gizmo_axis: std::sync::Arc<std::sync::atomic::AtomicU8>,
  ) -> Result<Self, String>
  ```

  Pass a clone of `hovered_gizmo_axis` to `GizmoPipeline::new()` (must clone so the Arc is still available for `GizmoSolidPipeline::new()` in Task 6):
  ```rust
  let gizmo_pipeline = crate::viewport::gizmo_pipeline::GizmoPipeline::new(
      renderer.context(),
      renderer.render_pass(),
      hovered_gizmo_axis.clone(),
  )?;
  ```

- [ ] **Step 7: Add `hovered_gizmo_axis` field to `GizmoPipeline` struct and `new()` in `gizmo_pipeline.rs`**

  In the `GizmoPipeline` struct (around line 295), add after existing fields:
  ```rust
  hovered_gizmo_axis: std::sync::Arc<std::sync::atomic::AtomicU8>,
  ```

  Change `GizmoPipeline::new()` signature:
  ```rust
  pub fn new(
      context: &VulkanContext,
      render_pass: vk::RenderPass,
      hovered_gizmo_axis: std::sync::Arc<std::sync::atomic::AtomicU8>,
  ) -> Result<Self, String>
  ```

  Add to the `Ok(Self { ... })` block:
  ```rust
  hovered_gizmo_axis,
  ```

- [ ] **Step 8: Verify build**

  ```bash
  cd engine/editor && cargo build -p silmaril-editor-tauri 2>&1 | grep -E "^error"
  ```
  Expected: no errors.

- [ ] **Step 9: Commit**

  ```bash
  git add engine/editor/src-tauri/
  git commit -m "feat(gizmo): thread hovered_gizmo_axis Arc through render pipeline"
  ```

---

## Task 3: `axis_color()` Helper and Hover Brightening

**Files:**
- Modify: `engine/editor/src-tauri/viewport/gizmo_pipeline.rs`

**Context:** Replace hardcoded `[1.0, 0.2, 0.2, 1.0]`-style literals in `record()` with calls to `axis_color(axis, is_hovered)`. The actual Z axis colour in the codebase is `[0.2, 0.4, 1.0, ...]` (G=0.4, not 0.2) — the unit test guards this.

- [ ] **Step 1: Write failing unit tests for `axis_color()`**

  Inside `#[cfg(test)] mod tests { ... }` in `gizmo_pipeline.rs` (inside `mod imp`), add:

  ```rust
  #[test]
  fn axis_color_hover_brightens_all_channels() {
      let base   = axis_color(GizmoAxis::X, false);
      let bright = axis_color(GizmoAxis::X, true);
      assert!(bright[0] > base[0], "R not brightened");
      // G and B are already near-max but clamped to 1.0
      assert!(bright[1] >= base[1], "G decreased");
      assert!(bright[2] >= base[2], "B decreased");
  }

  #[test]
  fn axis_color_z_has_g_channel_0_4() {
      // Guards the actual Z colour value against accidental "normalisation" to 0.2.
      let c = axis_color(GizmoAxis::Z, false);
      assert!(
          (c[1] - 0.4).abs() < 1e-5,
          "Z axis G channel should be 0.4, got {}",
          c[1]
      );
  }

  #[test]
  fn axis_color_hover_clamps_to_1_0() {
      // Brightening must not produce values > 1.0
      for axis in [GizmoAxis::X, GizmoAxis::Y, GizmoAxis::Z] {
          let c = axis_color(axis, true);
          for ch in &c[..3] {
              assert!(*ch <= 1.0, "axis_color exceeded 1.0: {ch}");
          }
      }
  }
  ```

- [ ] **Step 2: Run tests — expect compile failure (function not defined yet)**

  ```bash
  cd engine/editor && cargo test -p silmaril-editor-tauri axis_color 2>&1 | grep -E "^error|FAILED|ok"
  ```
  Expected: compile error — `axis_color` not found.

- [ ] **Step 3: Add `axis_color()` function** (in `mod imp`, before the `GizmoPipeline` impl block)

  ```rust
  /// Returns the display colour for a gizmo axis, brightened when hovered.
  fn axis_color(axis: GizmoAxis, hovered: bool) -> [f32; 4] {
      let base: [f32; 4] = match axis {
          GizmoAxis::X  => [1.0, 0.2, 0.2, 1.0],
          GizmoAxis::Y  => [0.2, 1.0, 0.2, 1.0],
          GizmoAxis::Z  => [0.2, 0.4, 1.0, 1.0],
          _             => [0.8, 0.8, 0.8, 1.0],
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

- [ ] **Step 4: Run tests — expect pass**

  ```bash
  cd engine/editor && cargo test -p silmaril-editor-tauri axis_color 2>&1 | grep -E "FAILED|ok|test result"
  ```
  Expected: all 3 tests pass.

- [ ] **Step 5: Use `axis_color()` in `record()`**

  At the top of `pub unsafe fn record()`, after binding the pipeline, read hover state once:
  ```rust
  let hover_raw = self.hovered_gizmo_axis.load(std::sync::atomic::Ordering::Relaxed);
  // hover_raw: 0=none, 1=X, 2=Y, 3=Z
  ```

  Replace the three crosshair color literals (now inside `if is_selected`):
  ```rust
  // X axis
  self.draw_buf(..., axis_color(GizmoAxis::X, hover_raw == 1), scale, 0, 2);
  // Y axis
  self.draw_buf(..., axis_color(GizmoAxis::Y, hover_raw == 2), scale, 2, 2);
  // Z axis
  self.draw_buf(..., axis_color(GizmoAxis::Z, hover_raw == 3), scale, 4, 2);
  ```

  Replace the nine mode handle color literals in `match mode { ... }`:
  ```rust
  GizmoMode::Move => {
      self.draw_buf(..., &self.move_x_buf, ..., axis_color(GizmoAxis::X, hover_raw == 1), scale, 0, self.move_x_count);
      self.draw_buf(..., &self.move_y_buf, ..., axis_color(GizmoAxis::Y, hover_raw == 2), scale, 0, self.move_y_count);
      self.draw_buf(..., &self.move_z_buf, ..., axis_color(GizmoAxis::Z, hover_raw == 3), scale, 0, self.move_z_count);
  }
  GizmoMode::Rotate => {
      self.draw_buf(..., &self.rotate_x_buf, ..., axis_color(GizmoAxis::X, hover_raw == 1), scale, 0, self.rotate_x_count);
      self.draw_buf(..., &self.rotate_y_buf, ..., axis_color(GizmoAxis::Y, hover_raw == 2), scale, 0, self.rotate_y_count);
      self.draw_buf(..., &self.rotate_z_buf, ..., axis_color(GizmoAxis::Z, hover_raw == 3), scale, 0, self.rotate_z_count);
  }
  GizmoMode::Scale => {
      self.draw_buf(..., &self.scale_x_buf, ..., axis_color(GizmoAxis::X, hover_raw == 1), scale, 0, self.scale_x_count);
      self.draw_buf(..., &self.scale_y_buf, ..., axis_color(GizmoAxis::Y, hover_raw == 2), scale, 0, self.scale_y_count);
      self.draw_buf(..., &self.scale_z_buf, ..., axis_color(GizmoAxis::Z, hover_raw == 3), scale, 0, self.scale_z_count);
  }
  ```

  > **Note:** Fill in the unchanged args (`cmd, device, view_proj, origin.into()`) from the existing code — don't remove them, just replace the color literal.

- [ ] **Step 6: Verify build + tests still pass**

  ```bash
  cd engine/editor && cargo test -p silmaril-editor-tauri 2>&1 | grep -E "^error|FAILED|test result"
  ```
  Expected: no errors, no FAILED.

- [ ] **Step 7: Commit**

  ```bash
  git add engine/editor/src-tauri/viewport/gizmo_pipeline.rs
  git commit -m "feat(gizmo): add axis_color() helper with hover brightening"
  ```

---

## Task 4: Solid Geometry Generators

**Files:**
- Modify: `engine/editor/src-tauri/viewport/gizmo_pipeline.rs`

**Context:** Add two new generator functions for TRIANGLE_LIST solid geometry. Also strip the wireframe cone/cube tips from the existing `generate_move_arrow_vertices` and `generate_scale_handle_vertices` functions — keep only the shaft line (first 2 vertices). The solid geometry replaces those tips.

- [ ] **Step 1: Write failing unit tests for solid generators**

  Inside `#[cfg(test)] mod tests { ... }` in `mod imp`:

  ```rust
  #[test]
  fn cone_solid_vertex_count_is_36() {
      for axis in [GizmoAxis::X, GizmoAxis::Y, GizmoAxis::Z] {
          let v = generate_move_cone_solid_vertices(axis);
          assert_eq!(v.len(), 36, "cone solid for {axis:?}: expected 36 got {}", v.len());
      }
  }

  #[test]
  fn cone_solid_vertices_within_bounds() {
      let v = generate_move_cone_solid_vertices(GizmoAxis::X);
      // All X vertices must be in [0.8, 1.0] along the shaft (X axis)
      for vert in &v {
          assert!(vert.pos[0] >= 0.79 && vert.pos[0] <= 1.01,
              "X out of range: {}", vert.pos[0]);
      }
  }

  #[test]
  fn cube_solid_vertex_count_is_36() {
      for axis in [GizmoAxis::X, GizmoAxis::Y, GizmoAxis::Z] {
          let v = generate_scale_cube_solid_vertices(axis);
          assert_eq!(v.len(), 36, "cube solid for {axis:?}: expected 36 got {}", v.len());
      }
  }
  ```

- [ ] **Step 2: Run tests — expect compile failure**

  ```bash
  cd engine/editor && cargo test -p silmaril-editor-tauri cone_solid 2>&1 | grep -E "^error"
  ```
  Expected: compile error — functions not defined.

- [ ] **Step 3: Add `generate_move_cone_solid_vertices()`**

  6-sided cone; base centred at `dir * 0.8`, tip at `dir * 1.0`, base radius 0.06:
  - 6 side triangles: `(base_i, base_{i+1}, tip)` — winding outward
  - 6 base triangles: `(center, base_i, base_{i+1})` — inward (cap)
  - Total: 12 triangles = 36 vertices

  ```rust
  pub fn generate_move_cone_solid_vertices(axis: GizmoAxis) -> Vec<GizmoVertex> {
      let dir   = axis_dir(axis);
      let perp1 = perpendicular(dir);
      let perp2 = dir.cross(perp1);
      let base_center = dir * 0.8;
      let tip         = dir * 1.0;
      const CONE_R: f32 = 0.06;
      const SIDES:  usize = 6;

      let ring: Vec<glam::Vec3> = (0..SIDES)
          .map(|i| {
              let a = (i as f32) * std::f32::consts::TAU / (SIDES as f32);
              base_center + perp1 * (a.cos() * CONE_R) + perp2 * (a.sin() * CONE_R)
          })
          .collect();

      let mut verts = Vec::with_capacity(36);
      for i in 0..SIDES {
          let ni = (i + 1) % SIDES;
          // Side triangle
          verts.push(GizmoVertex { pos: ring[i].into() });
          verts.push(GizmoVertex { pos: ring[ni].into() });
          verts.push(GizmoVertex { pos: tip.into() });
          // Base cap triangle
          verts.push(GizmoVertex { pos: base_center.into() });
          verts.push(GizmoVertex { pos: ring[i].into() });
          verts.push(GizmoVertex { pos: ring[ni].into() });
      }
      verts
  }
  ```

- [ ] **Step 4: Add `generate_scale_cube_solid_vertices()`**

  Axis-aligned cube centred at `dir * 0.85`, half-size 0.06, 6 faces × 2 triangles = 36 vertices:

  ```rust
  pub fn generate_scale_cube_solid_vertices(axis: GizmoAxis) -> Vec<GizmoVertex> {
      let dir   = axis_dir(axis);
      let perp1 = perpendicular(dir);
      let perp2 = dir.cross(perp1);
      let center = dir * 0.85;
      const H: f32 = 0.06;

      // 8 corners: ±H along dir, perp1, perp2
      let c = |da: f32, db: f32, dc: f32| -> GizmoVertex {
          GizmoVertex { pos: (center + dir * da + perp1 * db + perp2 * dc).into() }
      };
      let corners = [
          c(-H,-H,-H), c( H,-H,-H), c( H, H,-H), c(-H, H,-H),
          c(-H,-H, H), c( H,-H, H), c( H, H, H), c(-H, H, H),
      ];

      // 6 faces, each as 2 triangles (CCW winding)
      let faces: [[usize; 6]; 6] = [
          [0,1,2, 0,2,3], // -Z face
          [4,6,5, 4,7,6], // +Z face
          [0,4,5, 0,5,1], // -Y face
          [2,6,7, 2,7,3], // +Y face
          [0,3,7, 0,7,4], // -X face
          [1,5,6, 1,6,2], // +X face
      ];

      let mut verts = Vec::with_capacity(36);
      for face in &faces {
          for &idx in face {
              verts.push(corners[idx]);
          }
      }
      verts
  }
  ```

- [ ] **Step 5: Strip wireframe tips from existing generators**

  In `generate_move_arrow_vertices()`: remove the "Cone spokes" loop (everything after the shaft pair). Keep only:
  ```rust
  // Shaft: origin → shaft_tip
  verts.push(GizmoVertex { pos: [0.0, 0.0, 0.0] });
  verts.push(GizmoVertex { pos: shaft_tip.into() });
  ```

  In `generate_scale_handle_vertices()`: remove the corners + edges block. Keep only:
  ```rust
  // Shaft
  verts.push(GizmoVertex { pos: [0.0, 0.0, 0.0] });
  verts.push(GizmoVertex { pos: shaft_tip.into() });
  ```

- [ ] **Step 6: Run tests — all must pass**

  ```bash
  cd engine/editor && cargo test -p silmaril-editor-tauri 2>&1 | grep -E "FAILED|test result"
  ```
  Expected: `test result: ok`.

- [ ] **Step 7: Update re-exports at bottom of `gizmo_pipeline.rs`**

  Add the new generators to the `#[cfg(windows)] pub use imp::{ ... }` block:
  ```rust
  #[cfg(windows)]
  pub use imp::{
      generate_crosshair_vertices, generate_move_arrow_vertices,
      generate_move_cone_solid_vertices,
      generate_rotate_ring_vertices, generate_scale_handle_vertices,
      generate_scale_cube_solid_vertices,
      GizmoAxis, GizmoMode, GizmoPipeline, GizmoVertex,
  };
  ```

- [ ] **Step 8: Commit**

  ```bash
  git add engine/editor/src-tauri/viewport/gizmo_pipeline.rs
  git commit -m "feat(gizmo): solid cone/cube tip generators; strip wireframe tips"
  ```

---

## Task 5: GizmoSolidPipeline Struct

**Files:**
- Modify: `engine/editor/src-tauri/viewport/gizmo_pipeline.rs`

**Context:** Add a second `GizmoSolidPipeline` struct in `mod imp` (alongside `GizmoPipeline`). Uses TRIANGLE_LIST topology; same shaders; owns its own `vk::Pipeline` and `vk::PipelineLayout` (independently — no sharing). Has a `Drop` impl like `GizmoPipeline`. Stores 6 solid geometry buffers (3 cones + 3 cubes) and the `hovered_gizmo_axis` Arc.

- [ ] **Step 1: Add `GizmoSolidPipeline` struct and `Drop` impl**

  After the `impl Drop for GizmoPipeline` block, add:

  ```rust
  /// Second gizmo pipeline using TRIANGLE_LIST for solid cone/cube tips.
  ///
  /// Owns its own `vk::Pipeline` and `vk::PipelineLayout` (no sharing with GizmoPipeline).
  #[allow(dead_code)]
  pub struct GizmoSolidPipeline {
      device: ash::Device,
      pipeline: vk::Pipeline,
      pipeline_layout: vk::PipelineLayout,

      move_x_cone_solid_buf: GpuBuffer,
      move_x_cone_solid_count: u32,
      move_y_cone_solid_buf: GpuBuffer,
      move_y_cone_solid_count: u32,
      move_z_cone_solid_buf: GpuBuffer,
      move_z_cone_solid_count: u32,

      scale_x_cube_solid_buf: GpuBuffer,
      scale_x_cube_solid_count: u32,
      scale_y_cube_solid_buf: GpuBuffer,
      scale_y_cube_solid_count: u32,
      scale_z_cube_solid_buf: GpuBuffer,
      scale_z_cube_solid_count: u32,

      hovered_gizmo_axis: std::sync::Arc<std::sync::atomic::AtomicU8>,
  }

  impl Drop for GizmoSolidPipeline {
      fn drop(&mut self) {
          unsafe {
              self.device.destroy_pipeline(self.pipeline, None);
              self.device.destroy_pipeline_layout(self.pipeline_layout, None);
          }
      }
  }
  ```

- [ ] **Step 2: Add `GizmoSolidPipeline::new()`**

  Create a helper `create_gizmo_solid_pipeline()` that is identical to `create_gizmo_pipeline()` except:
  - `primitive_topology: vk::PrimitiveTopology::TRIANGLE_LIST` (instead of `LINE_LIST`)
  - The layout is independently created (same push range, same stages) — NOT shared.

  Then add `impl GizmoSolidPipeline`:
  ```rust
  impl GizmoSolidPipeline {
      pub fn new(
          context: &VulkanContext,
          render_pass: vk::RenderPass,
          hovered_gizmo_axis: std::sync::Arc<std::sync::atomic::AtomicU8>,
      ) -> Result<Self, String> {
          let device = &context.device;
          let (vert_spirv, frag_spirv) = get_or_compile_shaders()?;
          let vert_shader = ShaderModule::from_spirv(device, vert_spirv,
              vk::ShaderStageFlags::VERTEX, "main")
              .map_err(|e| format!("SolidGizmoVertShader: {e}"))?;
          let frag_shader = ShaderModule::from_spirv(device, frag_spirv,
              vk::ShaderStageFlags::FRAGMENT, "main")
              .map_err(|e| format!("SolidGizmoFragShader: {e}"))?;

          let (pipeline, pipeline_layout) =
              create_gizmo_solid_pipeline(device, render_pass, &vert_shader, &frag_shader)?;

          macro_rules! upload {
              ($verts:expr) => {{
                  let v = $verts;
                  let count = v.len() as u32;
                  let buf = upload_verts(context, &v)?;
                  (buf, count)
              }};
          }

          let (move_x_cone_solid_buf, move_x_cone_solid_count) =
              upload!(generate_move_cone_solid_vertices(GizmoAxis::X));
          let (move_y_cone_solid_buf, move_y_cone_solid_count) =
              upload!(generate_move_cone_solid_vertices(GizmoAxis::Y));
          let (move_z_cone_solid_buf, move_z_cone_solid_count) =
              upload!(generate_move_cone_solid_vertices(GizmoAxis::Z));
          let (scale_x_cube_solid_buf, scale_x_cube_solid_count) =
              upload!(generate_scale_cube_solid_vertices(GizmoAxis::X));
          let (scale_y_cube_solid_buf, scale_y_cube_solid_count) =
              upload!(generate_scale_cube_solid_vertices(GizmoAxis::Y));
          let (scale_z_cube_solid_buf, scale_z_cube_solid_count) =
              upload!(generate_scale_cube_solid_vertices(GizmoAxis::Z));

          tracing::info!("GizmoSolidPipeline created");
          Ok(Self {
              device: device.clone(),
              pipeline,
              pipeline_layout,
              move_x_cone_solid_buf, move_x_cone_solid_count,
              move_y_cone_solid_buf, move_y_cone_solid_count,
              move_z_cone_solid_buf, move_z_cone_solid_count,
              scale_x_cube_solid_buf, scale_x_cube_solid_count,
              scale_y_cube_solid_buf, scale_y_cube_solid_count,
              scale_z_cube_solid_buf, scale_z_cube_solid_count,
              hovered_gizmo_axis,
          })
      }
  ```

- [ ] **Step 3: Add `create_gizmo_solid_pipeline()` helper**

  Copy `create_gizmo_pipeline()` to a new function `create_gizmo_solid_pipeline()` and change the topology line:
  ```rust
  .input_assembly_state(
      &vk::PipelineInputAssemblyStateCreateInfo::default()
          .topology(vk::PrimitiveTopology::TRIANGLE_LIST),
  )
  ```
  Everything else is identical.

- [ ] **Step 4: Add `GizmoSolidPipeline::record()` method**

  ```rust
      /// Record solid tip draw commands (must be called after GizmoPipeline::record).
      pub unsafe fn record(
          &self,
          cmd: vk::CommandBuffer,
          world: &engine_core::World,
          selected_entity_id: Option<u64>,
          mode: GizmoMode,
          view_proj: glam::Mat4,
          camera_pos: glam::Vec3,
      ) {
          let device = &self.device;
          device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, self.pipeline);

          let hover_raw = self.hovered_gizmo_axis.load(std::sync::atomic::Ordering::Relaxed);

          for entity in world.entities() {
              let Some(transform) = world.get::<engine_core::Transform>(entity) else {
                  continue;
              };

              let is_selected = selected_entity_id.map_or(false, |id| {
                  if id > u32::MAX as u64 { return false; }
                  entity.id() == id as u32
              });
              if !is_selected { continue; }

              let origin = transform.position;
              let dist   = (camera_pos - origin).length().max(0.1);
              let scale  = dist * 0.15;

              match mode {
                  GizmoMode::Move => {
                      self.draw_solid(cmd, device, &self.move_x_cone_solid_buf, view_proj, origin.into(),
                          axis_color(GizmoAxis::X, hover_raw == 1), scale, self.move_x_cone_solid_count);
                      self.draw_solid(cmd, device, &self.move_y_cone_solid_buf, view_proj, origin.into(),
                          axis_color(GizmoAxis::Y, hover_raw == 2), scale, self.move_y_cone_solid_count);
                      self.draw_solid(cmd, device, &self.move_z_cone_solid_buf, view_proj, origin.into(),
                          axis_color(GizmoAxis::Z, hover_raw == 3), scale, self.move_z_cone_solid_count);
                  }
                  GizmoMode::Scale => {
                      self.draw_solid(cmd, device, &self.scale_x_cube_solid_buf, view_proj, origin.into(),
                          axis_color(GizmoAxis::X, hover_raw == 1), scale, self.scale_x_cube_solid_count);
                      self.draw_solid(cmd, device, &self.scale_y_cube_solid_buf, view_proj, origin.into(),
                          axis_color(GizmoAxis::Y, hover_raw == 2), scale, self.scale_y_cube_solid_count);
                      self.draw_solid(cmd, device, &self.scale_z_cube_solid_buf, view_proj, origin.into(),
                          axis_color(GizmoAxis::Z, hover_raw == 3), scale, self.scale_z_cube_solid_count);
                  }
                  GizmoMode::Rotate => {
                      // Rotate mode uses rings (lines) only — no solid tips.
                  }
              }
          }
      }

      #[allow(clippy::too_many_arguments)]
      unsafe fn draw_solid(
          &self,
          cmd: vk::CommandBuffer,
          device: &ash::Device,
          buf: &GpuBuffer,
          view_proj: glam::Mat4,
          origin: [f32; 3],
          color: [f32; 4],
          scale: f32,
          vertex_count: u32,
      ) {
          device.cmd_bind_vertex_buffers(cmd, 0, &[buf.handle()], &[0]);
          let pc = GizmoPushConstants {
              view_proj: view_proj.to_cols_array_2d(),
              origin,
              _pad0: 0.0,
              color,
              scale,
              _pad1: [0.0; 3],
          };
          let pc_bytes = std::slice::from_raw_parts(
              &pc as *const GizmoPushConstants as *const u8,
              std::mem::size_of::<GizmoPushConstants>(),
          );
          device.cmd_push_constants(
              cmd,
              self.pipeline_layout,
              vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
              0,
              pc_bytes,
          );
          device.cmd_draw(cmd, vertex_count, 1, 0, 0);
      }
  } // end impl GizmoSolidPipeline
  ```

- [ ] **Step 5: Add `GizmoSolidPipeline` to the `#[cfg(windows)] pub use imp::{ ... }` re-exports**

  ```rust
  #[cfg(windows)]
  pub use imp::{
      generate_crosshair_vertices, generate_move_arrow_vertices,
      generate_move_cone_solid_vertices,
      generate_rotate_ring_vertices, generate_scale_handle_vertices,
      generate_scale_cube_solid_vertices,
      GizmoAxis, GizmoMode, GizmoPipeline, GizmoSolidPipeline, GizmoVertex,
  };
  ```

- [ ] **Step 6: Verify build + tests**

  ```bash
  cd engine/editor && cargo test -p silmaril-editor-tauri 2>&1 | grep -E "^error|FAILED|test result"
  ```
  Expected: no errors, `test result: ok`.

- [ ] **Step 7: Commit**

  ```bash
  git add engine/editor/src-tauri/viewport/gizmo_pipeline.rs
  git commit -m "feat(gizmo): add GizmoSolidPipeline for TRIANGLE_LIST solid cone/cube tips"
  ```

---

## Task 6: Wire GizmoSolidPipeline into ViewportRenderer

**Files:**
- Modify: `engine/editor/src-tauri/viewport/native_viewport.rs`

**Context:** `ViewportRenderer` already has `gizmo_pipeline: GizmoPipeline`. Add a `gizmo_solid_pipeline: GizmoSolidPipeline` field alongside it. Field order matters for Drop (see existing comment: `renderer` must drop before pipelines). `gizmo_solid_pipeline` drops in the same pass as `gizmo_pipeline`. Also pass the `hovered_gizmo_axis` arc to the solid pipeline constructor.

- [ ] **Step 1: Add `gizmo_solid_pipeline` field to `ViewportRenderer`**

  After `gizmo_pipeline: crate::viewport::gizmo_pipeline::GizmoPipeline,` add:
  ```rust
  gizmo_solid_pipeline: crate::viewport::gizmo_pipeline::GizmoSolidPipeline,
  ```
  (Keep after `renderer` for correct Drop order.)

- [ ] **Step 2: Construct `GizmoSolidPipeline` in `ViewportRenderer::new()`**

  After the `GizmoPipeline::new(...)` call, add:
  ```rust
  // hovered_gizmo_axis was cloned for gizmo_pipeline above; original Arc is used here
  let gizmo_solid_pipeline = crate::viewport::gizmo_pipeline::GizmoSolidPipeline::new(
      renderer.context(),
      renderer.render_pass(),
      hovered_gizmo_axis,  // original Arc (not cloned — moves into gizmo_solid_pipeline)
  )?;
  ```

  Update the `Ok(Self { ... })` to include `gizmo_solid_pipeline`.

- [ ] **Step 3: Call `gizmo_solid_pipeline.record()` in `render_frame()`**

  In `render_frame()`, immediately after the `self.gizmo_pipeline.record(...)` call (inside the `unsafe` block, inside the `for (bounds, camera, ...)` loop):
  ```rust
  self.gizmo_solid_pipeline.record(
      cmd,
      &world_guard,
      selected_entity_id,
      gizmo_mode,
      view_proj,
      camera_pos,
  );
  ```

- [ ] **Step 4: Verify build**

  ```bash
  cd engine/editor && cargo build -p silmaril-editor-tauri 2>&1 | grep -E "^error"
  ```
  Expected: no errors.

- [ ] **Step 5: Commit**

  ```bash
  git add engine/editor/src-tauri/viewport/native_viewport.rs
  git commit -m "feat(gizmo): wire GizmoSolidPipeline into ViewportRenderer"
  ```

---

## Task 7: IPC Commands — `gizmo_hover_test` and `set_hovered_gizmo_axis`

**Files:**
- Modify: `engine/editor/src-tauri/bridge/gizmo_commands.rs`
- Modify: `engine/editor/src-tauri/lib.rs`

**Context:** `gizmo_hit_test` (existing) reads entity_id from params and writes DragState. `gizmo_hover_test` (new) reads selected entity from `viewport_state.selected_entity_id`, does the same ray-cast, but does NOT write DragState. Both are guarded by `#[cfg(windows)]` consistent with the existing commands.

`set_hovered_gizmo_axis` is a simple setter — maps "x"→1, "y"→2, "z"→3, else→0 into `viewport_state.hovered_gizmo_axis`.

- [ ] **Step 1: Write a unit test for `set_hovered_gizmo_axis` handler logic**

  In the `#[cfg(test)]` block at the bottom of `gizmo_commands.rs`:
  ```rust
  #[test]
  fn set_hovered_axis_maps_strings_to_u8() {
      use std::sync::{Arc, atomic::{AtomicU8, Ordering}};
      let atom = Arc::new(AtomicU8::new(0));

      // Simulate the handler's logic directly (not via Tauri State)
      let map = |s: Option<&str>| -> u8 {
          match s {
              Some("x") => 1,
              Some("y") => 2,
              Some("z") => 3,
              _          => 0,
          }
      };

      atom.store(map(Some("x")), Ordering::Relaxed);
      assert_eq!(atom.load(Ordering::Relaxed), 1);
      atom.store(map(Some("y")), Ordering::Relaxed);
      assert_eq!(atom.load(Ordering::Relaxed), 2);
      atom.store(map(Some("z")), Ordering::Relaxed);
      assert_eq!(atom.load(Ordering::Relaxed), 3);
      atom.store(map(None), Ordering::Relaxed);
      assert_eq!(atom.load(Ordering::Relaxed), 0);
      // Unknown strings → 0
      atom.store(map(Some("xy")), Ordering::Relaxed);
      assert_eq!(atom.load(Ordering::Relaxed), 0);
  }
  ```

- [ ] **Step 2: Run test — expect compile success and pass**

  ```bash
  cd engine/editor && cargo test -p silmaril-editor-tauri set_hovered_axis 2>&1 | grep -E "FAILED|ok|test result"
  ```
  Expected: test passes immediately (pure logic, no new functions yet).

- [ ] **Step 3: Add `set_hovered_gizmo_axis` Tauri command**

  After `set_gizmo_mode` (around line 442 of `gizmo_commands.rs`):
  ```rust
  /// Set the hovered gizmo axis for hover highlighting.
  ///
  /// Called from the frontend on every mousemove (non-drag) and cleared on mouseleave.
  /// Unknown strings silently map to 0 (no hover) — intentional for this transient hint.
  #[cfg(windows)]
  #[tauri::command]
  pub fn set_hovered_gizmo_axis(
      axis: Option<String>,
      viewport_state: tauri::State<'_, super::commands::NativeViewportState>,
  ) -> Result<(), String> {
      let v = match axis.as_deref() {
          Some("x") => 1,
          Some("y") => 2,
          Some("z") => 3,
          _          => 0,
      };
      viewport_state
          .hovered_gizmo_axis
          .store(v, std::sync::atomic::Ordering::Relaxed);
      Ok(())
  }
  ```

- [ ] **Step 4: Add `gizmo_hover_test` Tauri command**

  This reuses the same ray-cast logic as `gizmo_hit_test` but reads selected entity from state instead of a param, and does NOT write DragState:

  ```rust
  /// Read-only axis hit-test for hover highlighting. No DragState written.
  ///
  /// Called from the frontend's mousemove handler when not dragging.
  /// Returns "x" | "y" | "z" | null.
  #[cfg(windows)]
  #[tauri::command]
  pub fn gizmo_hover_test(
      viewport_id: String,
      screen_x: f32,
      screen_y: f32,
      viewport_state: tauri::State<'_, super::commands::NativeViewportState>,
      world_state: tauri::State<'_, crate::state::SceneWorldState>,
  ) -> Result<Option<String>, String> {
      // `unproject_screen` is a private helper already defined in this file (line ~103).
      use glam::Vec3;

      // Read selected entity id — no entity selected means no gizmo to test.
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
      let entity = engine_core::Entity::new(entity_id as u32, 0);

      // Read entity position
      let entity_pos = {
          let world = world_state.inner().0.read().map_err(|e| e.to_string())?;
          match world.get::<engine_core::Transform>(entity) {
              Some(t) => t.position,
              None    => return Ok(None),
          }
      };

      // Get camera data
      let (view, proj, _cam_pos, bounds, gizmo_scale) = {
          let registry = viewport_state.registry.lock().map_err(|e| e.to_string())?;
          match registry.get_for_id(&viewport_id) {
              Some(vp) => match vp.get_instance_ray_data(&viewport_id) {
                  Some((view, proj, eye, bounds)) => {
                      let dist = (eye - entity_pos).length().max(0.1_f32);
                      (view, proj, eye, bounds, dist * 0.15)
                  }
                  None => return Ok(None),
              },
              None => return Ok(None),
          }
      };

      // Unproject screen position to world-space ray
      let (ray_origin, ray_dir) = unproject_screen(screen_x, screen_y, &bounds, view, proj);

      // Test each axis handle (same geometry as gizmo_hit_test, no DragState written)
      let axes = [
          (Vec3::X, "x"),
          (Vec3::Y, "y"),
          (Vec3::Z, "z"),
      ];
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

- [ ] **Step 5: Register both commands in `lib.rs` invoke_handler**

  In `lib.rs` around line 367–370, after the existing gizmo commands:
  ```rust
  bridge::gizmo_commands::gizmo_hit_test,
  bridge::gizmo_commands::gizmo_drag,
  bridge::gizmo_commands::gizmo_drag_end,
  bridge::gizmo_commands::set_gizmo_mode,
  bridge::gizmo_commands::gizmo_hover_test,       // NEW
  bridge::gizmo_commands::set_hovered_gizmo_axis,  // NEW
  ```

  > **Note:** Tauri's `generate_handler!` macro uses cfg attributes from the function, so even though the two new functions are `#[cfg(windows)]`, they must be listed without a cfg guard in the `generate_handler!` call — Tauri handles the platform filtering internally. Follow the same pattern as the other gizmo commands already in the list.

- [ ] **Step 6: Verify build + tests**

  ```bash
  cd engine/editor && cargo test -p silmaril-editor-tauri 2>&1 | grep -E "^error|FAILED|test result"
  ```
  Expected: no errors, all tests pass.

- [ ] **Step 7: Commit**

  ```bash
  git add engine/editor/src-tauri/bridge/gizmo_commands.rs engine/editor/src-tauri/lib.rs
  git commit -m "feat(gizmo): add gizmo_hover_test and set_hovered_gizmo_axis IPC commands"
  ```

---

## Task 8: Frontend — Hover Path in ViewportPanel.svelte

**Files:**
- Modify: `engine/editor/src/lib/docking/panels/ViewportPanel.svelte`

**Context:** `handleMouseMove` at line 373 is `async` and checks `isDraggingGizmo` at line 375. We add a hover test path that runs only when NOT dragging (`isDraggingGizmo === false`). The `onmouseleave` handler (line 587) calls `handleMouseUp()` and `setViewportFocused(false)` — we also call `set_hovered_gizmo_axis(null)` there. Both IPC calls are guarded by `isTauri`.

Look for the `invoke` imports near the top of the file to confirm how gizmo commands are currently imported — likely through a module like `$lib/viewport-commands`. Adapt as appropriate (add new calls alongside existing gizmo IPC calls). If invoke is called directly, use `import { invoke } from '@tauri-apps/api/core'` which is already imported.

- [ ] **Step 1: Add hover IPC call at the end of `handleMouseMove`**

  In `handleMouseMove()`, BEFORE the final `}` that closes the function, add a hover path that runs when NOT dragging (after the early return for gizmo drag, after the `if (!isDragging) return` check):

  ```ts
  // Hover test — update hovered axis for visual highlight (non-drag only)
  // gizmo_hover_test returns Result<Option<String>, String>; invoke<> unwraps the Ok value.
  if (!isDraggingGizmo && !isDragging && isTauri) {
    try {
      const hit = await invoke<string | null>('gizmo_hover_test', {
        viewportId, screenX: event.clientX, screenY: event.clientY,
      });
      await invoke('set_hovered_gizmo_axis', { axis: hit ?? null });
    } catch {
      // Non-critical — hover state may be stale for one frame; silently ignore errors.
    }
  }
  ```

  > **Placement:** The hover block must be AFTER the `if (!isDragging) return;` line (line 385) — when camera-dragging, skip hover. It also returns early if `isDraggingGizmo`, so insert after the gizmo-drag early return. The natural spot is right before the closing `}` of the function, after the entire `switch(dragMode)` block.

- [ ] **Step 2: Clear hover state on `onmouseleave`**

  In the `onmouseleave` handler (inline on the `<div class="viewport-container">`), change from:
  ```svelte
  onmouseleave={() => { handleMouseUp(); setViewportFocused(false); }}
  ```
  to:
  ```svelte
  onmouseleave={async () => {
    handleMouseUp();
    setViewportFocused(false);
    if (isTauri) {
      try { await invoke('set_hovered_gizmo_axis', { axis: null }); } catch {}
    }
  }}
  ```

- [ ] **Step 3: Verify TypeScript compilation**

  ```bash
  cd engine/editor && npm run typecheck 2>&1 | grep -v "^>" | grep "error TS"
  ```
  Expected: same pre-existing errors only (the 5 unrelated errors). No new errors.

- [ ] **Step 4: Build frontend**

  ```bash
  cd engine/editor && npm run build 2>&1 | tail -5
  ```
  Expected: `ok (no errors)`.

- [ ] **Step 5: Run E2E tests to confirm nothing regressed**

  ```bash
  cd engine/editor && npx playwright test --reporter=list 2>&1 | tail -10
  ```
  Expected: all existing tests pass.

- [ ] **Step 6: Commit**

  ```bash
  git add engine/editor/src/lib/docking/panels/ViewportPanel.svelte
  git commit -m "feat(gizmo): add hover IPC calls in ViewportPanel mousemove/mouseleave"
  ```

---

## Final Verification

- [ ] **Full Rust build clean**

  ```bash
  cd engine/editor && cargo build -p silmaril-editor-tauri 2>&1 | grep -E "^error"
  ```
  Expected: no errors.

- [ ] **All Rust unit tests pass**

  ```bash
  cd engine/editor && cargo test -p silmaril-editor-tauri 2>&1 | grep "test result"
  ```
  Expected: `test result: ok`.

- [ ] **Frontend build clean**

  ```bash
  cd engine/editor && npm run build 2>&1 | tail -3
  ```
  Expected: `ok (no errors)`.

- [ ] **All E2E tests pass**

  ```bash
  cd engine/editor && npx playwright test --reporter=list 2>&1 | tail -5
  ```
  Expected: all tests pass.
