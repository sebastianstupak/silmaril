# ECS Rendering + Interactive Gizmos Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extract shared Vulkan infrastructure into `engine/render-context`, refactor `engine/renderer` and `native_viewport.rs` to use it without duplication, wire a live ECS world into Tauri managed state, and render interactive transform gizmos (move/rotate/scale) for all scene entities in the viewport.

**Architecture:** A new `engine/render-context` crate owns all raw Vulkan plumbing (context, swapchain, buffers, pipelines). `engine/renderer` depends on it and adds ECS mesh rendering + a `FrameRecorder` hook for overlay injection. The editor's `native_viewport.rs` wraps `engine_renderer::Renderer` and injects grid/gizmo draw calls via `FrameRecorder`. A live `Arc<RwLock<World>>` in Tauri managed state is the single source of truth; a separate in-memory `SceneUndoStack` handles Ctrl+Z for world mutations without touching the existing `CommandProcessor`/TemplateState system.

**Tech Stack:** Rust, Ash (Vulkan), gpu-allocator, glam, naga (runtime GLSL→SPIR-V for gizmo shaders), Tauri 2, Svelte 5, engine-core ECS, engine-ops CommandProcessor.

**Spec:** `docs/superpowers/specs/2026-03-21-ecs-rendering-gizmos-design.md`

---

## File Map

**New crate — `engine/render-context/`:**
- `Cargo.toml` — deps: ash, gpu-allocator, glam, raw-window-handle, windows (cfg)
- `src/lib.rs` — re-exports all public types
- `src/context.rs` — `RenderContext` (moved from engine/renderer)
- `src/surface.rs` — `Surface` + `from_raw_handle` (moved + extended)
- `src/swapchain.rs` — `Swapchain` (moved)
- `src/depth.rs` — `DepthBuffer` (moved)
- `src/render_pass.rs` — `RenderPass` (moved)
- `src/framebuffer.rs` — `Framebuffers` (moved)
- `src/command.rs` — `CommandPool`/`CommandBuffers` (moved)
- `src/sync.rs` — `FrameSync` (moved)
- `src/buffer.rs` — `GpuBuffer`/`VertexBuffer`/`IndexBuffer`/`GpuMesh` (moved)
- `src/pipeline.rs` — `GraphicsPipeline`/`PipelineBuilder` (moved)
- `src/shader.rs` — `ShaderModule` (moved)
- `src/error.rs` — `RenderContextError` using `define_error!`

**Modified — `engine/renderer/src/`:**
- `Cargo.toml` — add `engine-render-context` dep; remove re-duplicated deps
- `lib.rs` — remove moved types; re-export `engine_render_context::*`
- `renderer.rs` — add `FrameRecorder`, `ViewportDescriptor`, `from_raw_handle`, updated `render_meshes(recorder, world, Option<assets>, viewports)`
- All other files — update `use crate::X` → `use engine_render_context::X`

**New files — `engine/editor/src-tauri/`:**
- `state/mod.rs` — re-exports
- `state/scene_world.rs` — `SceneWorldState(Arc<RwLock<World>>)`
- `state/scene_undo.rs` — `SceneUndoStack`, `SceneAction`, `SerializedTransform`
- `bridge/gizmo_commands.rs` — `gizmo_hit_test`, `gizmo_drag`, `gizmo_drag_end`, `set_gizmo_mode`
- `viewport/gizmo_pipeline.rs` — `GizmoPipeline` + procedural geometry
- `viewport/shaders/gizmo.vert` — GLSL vertex shader
- `viewport/shaders/gizmo.frag` — GLSL fragment shader

**Modified — `engine/editor/src-tauri/`:**
- `Cargo.toml` — add `engine-render-context` dep (Windows cfg)
- `lib.rs` — manage `SceneWorldState`, `SceneUndoStack`; register new commands
- `bridge/commands.rs` — `create_entity`, `delete_entity`, `set_component_field` write to world; `NativeViewportState` gains `drag_state` field
- `viewport/native_viewport.rs` — `ViewportRenderer` wraps `engine_renderer::Renderer`; gizmo pipeline integration
- `viewport/mod.rs` — expose `GizmoPipeline`

**Modified — frontend:**
- `src/lib/scene/state.ts` — listen to `entity-transform-changed` Tauri event
- `src/lib/components/ViewportOverlay.svelte` — gizmo mouse event handlers + `set_gizmo_mode` calls
- `src/lib/stores/undo-history.ts` — `sceneCanUndo`/`sceneCanRedo`; route Ctrl+Z by focus

---

## Phase 1: Extract `engine/render-context`

### Task 1: Scaffold the crate

**Files:**
- Create: `engine/render-context/Cargo.toml`
- Create: `engine/render-context/src/lib.rs`
- Create: `engine/render-context/src/error.rs`
- Modify: `Cargo.toml` (workspace root)

- [ ] **Step 1: Create directory**

```bash
mkdir -p engine/render-context/src
```

- [ ] **Step 2: Write `engine/render-context/Cargo.toml`**

```toml
[package]
name = "engine-render-context"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true

[dependencies]
ash = "0.38"
gpu-allocator = { version = "0.28", features = ["vulkan"] }
glam = { workspace = true }
raw-window-handle = "0.6"
tracing = "0.1"
thiserror = "2.0"
bytemuck = "1.14"
engine-core = { path = "../core" }   # for define_error! and error codes

[target.'cfg(windows)'.dependencies]
windows = { version = "0.61", features = [
    "Win32_System_LibraryLoader",
] }
```

- [ ] **Step 3: Write `engine/render-context/src/error.rs`**

```rust
use silmaril_core::define_error;
use silmaril_core::errors::{ErrorCode, ErrorSeverity};

define_error! {
    pub enum RenderContextError {
        VulkanInit { reason: String } = ErrorCode::InternalError, ErrorSeverity::Fatal,
        SwapchainOutOfDate = ErrorCode::InternalError, ErrorSeverity::Warning,
        DeviceLost = ErrorCode::InternalError, ErrorSeverity::Fatal,
    }
}
```

- [ ] **Step 4: Write stub `engine/render-context/src/lib.rs`**

```rust
pub mod error;
pub use error::RenderContextError;
```

- [ ] **Step 5: Add to workspace root `Cargo.toml`**

In the `members = [...]` array, add `"engine/render-context"` after `"engine/renderer"`.

- [ ] **Step 6: Verify it compiles**

```bash
cargo check -p engine-render-context
```

Expected: no errors.

- [ ] **Step 7: Commit**

```bash
git add engine/render-context/ Cargo.toml
git commit -m "feat(render-context): scaffold new crate"
```

---

### Task 2: Move Vulkan infrastructure files

Move the following files verbatim from `engine/renderer/src/` to `engine/render-context/src/`. Change only the `use crate::` prefixes to `use crate::` (they stay internal). Leave the originals in place temporarily as re-exports.

Files to copy: `context.rs`, `surface.rs`, `swapchain.rs`, `depth.rs`, `render_pass.rs`, `framebuffer.rs`, `command.rs`, `sync.rs`, `buffer.rs`, `pipeline.rs`, `shader.rs`, `window.rs`

**Files:**
- Create: `engine/render-context/src/{context,surface,swapchain,depth,render_pass,framebuffer,command,sync,buffer,pipeline,shader,window}.rs`

- [ ] **Step 1: Copy each file**

```bash
for f in context surface swapchain depth render_pass framebuffer command sync buffer pipeline shader window; do
  cp engine/renderer/src/${f}.rs engine/render-context/src/${f}.rs
done
```

- [ ] **Step 2: Update `engine/render-context/src/lib.rs` to declare all modules**

```rust
pub mod buffer;
pub mod command;
pub mod context;
pub mod depth;
pub mod error;
pub mod framebuffer;
pub mod pipeline;
pub mod render_pass;
pub mod shader;
pub mod surface;
pub mod swapchain;
pub mod sync;
pub mod window;

pub use error::RenderContextError;
// Re-export all public types
pub use buffer::{GpuBuffer, GpuMesh, IndexBuffer, VertexBuffer};
pub use command::{CommandBuffers, CommandPool};
pub use context::RenderContext;
pub use depth::DepthBuffer;
pub use framebuffer::Framebuffers;
pub use pipeline::{GraphicsPipeline, PipelineBuilder};
pub use render_pass::RenderPass;
pub use shader::ShaderModule;
pub use surface::Surface;
pub use swapchain::Swapchain;
pub use sync::FrameSync;
pub use window::Window;
```

- [ ] **Step 3: Fix any `use crate::error::` references in copied files**

In each copied file, change `use crate::error::RendererError` → `use crate::error::RenderContextError` (or whatever the error type is called). Also change `use crate::context::` → `use crate::context::` (no change needed if internal).

- [ ] **Step 4: Check render-context compiles**

```bash
cargo check -p engine-render-context
```

Expected: compiles. Fix any import errors (they'll be `use crate::X` that now need to reference sibling modules instead of renderer modules).

- [ ] **Step 5: Commit**

```bash
git add engine/render-context/src/
git commit -m "feat(render-context): move Vulkan infrastructure from engine-renderer"
```

---

### Task 3: Update `engine/renderer` to use `engine-render-context`

Remove the duplicated source files from `engine/renderer/src/` and replace them with `pub use` re-exports from `engine-render-context` so all downstream users continue to compile unchanged.

**Files:**
- Modify: `engine/renderer/Cargo.toml`
- Modify: `engine/renderer/src/lib.rs`
- Delete (replace with re-export stubs): all 12 moved `.rs` files

- [ ] **Step 1: Add `engine-render-context` dep to `engine/renderer/Cargo.toml`**

```toml
[dependencies]
engine-render-context = { path = "../render-context" }
# keep ash, gpu-allocator etc for the remaining renderer-specific code
```

- [ ] **Step 2: Replace each moved file with a re-export stub**

For example, replace `engine/renderer/src/context.rs` with:
```rust
pub use engine_render_context::RenderContext;
```

Do this for all 12 moved files. The `Renderer` struct in `renderer.rs` imports from these modules and will continue to work via the re-exports.

- [ ] **Step 3: Check engine-renderer still compiles**

```bash
cargo check -p engine-renderer
```

Expected: compiles with no errors.

- [ ] **Step 4: Run existing renderer tests**

```bash
cargo test -p engine-renderer
```

Expected: all tests pass (no behavioral change).

- [ ] **Step 5: Commit**

```bash
git add engine/renderer/
git commit -m "refactor(renderer): depend on engine-render-context, remove duplicated Vulkan files"
```

---

## Phase 2: Refactor `engine/renderer`

### Task 4: Add `FrameRecorder` and `from_raw_handle`

**Files:**
- Modify: `engine/renderer/src/renderer.rs`

- [ ] **Step 1: Add `ViewportDescriptor` and `FrameRecorder` structs**

In `renderer.rs`, add near the top:

```rust
use std::marker::PhantomData;

/// Describes one viewport sub-rect within the swapchain surface.
pub struct ViewportDescriptor {
    pub bounds: engine_render_context::Rect,  // { x: i32, y: i32, width: u32, height: u32 }
    pub view: glam::Mat4,
    pub proj: glam::Mat4,
}

/// Live handle to an in-progress frame. The render pass is open for the
/// entire lifetime of this value. Inject overlay draw calls via `command_buffer`.
///
/// Safety: !Send (command buffer must not cross thread boundaries mid-frame).
/// Consumed by `end_frame` which closes the render pass and submits.
pub struct FrameRecorder {
    pub command_buffer: ash::vk::CommandBuffer,
    pub(crate) image_index: u32,
    _not_send: PhantomData<*mut ()>,
}
```

Note: if `Rect` doesn't exist in `engine-render-context`, add it there:
```rust
// engine/render-context/src/lib.rs
#[derive(Clone, Copy, Debug)]
pub struct Rect { pub x: i32, pub y: i32, pub width: u32, pub height: u32 }
```

- [ ] **Step 2: Add `from_raw_handle` constructor to `Renderer`**

```rust
impl Renderer {
    /// Construct a renderer targeting an existing OS window handle.
    /// `hwnd` is the result of `WebviewWindow::hwnd().0` on Windows.
    /// The caller is responsible for keeping the window alive.
    #[cfg(windows)]
    pub fn from_raw_handle(
        hwnd: isize,
        width: u32,
        height: u32,
    ) -> Result<Self, RendererError> {
        // Mirror the existing Renderer::new() path but use Surface::from_raw_hwnd
        // instead of Surface::from_window.
        // engine/renderer/src/surface.rs already has from_raw_hwnd(hwnd: isize).
        let surface = engine_render_context::Surface::from_raw_hwnd(hwnd)?;
        // ... rest of init identical to Renderer::new()
        todo!("mirror Renderer::new() using surface from HWND")
    }
}
```

Fill in the full init by copying the relevant parts of `Renderer::new()` and replacing the window/surface creation call.

- [ ] **Step 3: Refactor `begin_frame` to return `FrameRecorder`**

Change the signature from private frame management to:
```rust
pub fn begin_frame(&mut self) -> Option<FrameRecorder> {
    // existing: wait fence, reset fence, acquire image, begin command buffer, begin render pass
    // return Some(FrameRecorder { command_buffer, image_index, _not_send: PhantomData })
    // return None if swapchain needs rebuild
}
```

- [ ] **Step 4: Add `end_frame` that consumes `FrameRecorder`**

```rust
pub fn end_frame(&mut self, recorder: FrameRecorder) {
    // end render pass, end command buffer, queue_submit, queue_present
    // mirrors existing render_frame() submit/present logic
}
```

- [ ] **Step 5: Check compiles**

```bash
cargo check -p engine-renderer
```

- [ ] **Step 6: Commit**

```bash
git add engine/renderer/src/renderer.rs
git commit -m "feat(renderer): add FrameRecorder, from_raw_handle, begin_frame/end_frame API"
```

---

### Task 5: Multi-viewport `render_meshes`

**Files:**
- Modify: `engine/renderer/src/renderer.rs`
- Modify: `engine/renderer/tests/` — update calls to `render_meshes`

- [ ] **Step 1: Update `render_meshes` signature**

Replace the old `render_meshes(&mut self, world, assets)` with:

```rust
pub fn render_meshes(
    &self,
    recorder: &FrameRecorder,
    world: &engine_core::World,
    assets: Option<&engine_assets::AssetManager>,
    viewports: &[ViewportDescriptor],
) {
    let Some(assets) = assets else { return; };  // deferred phase: no-op
    // existing: build render_queue from ECS queries
    // NEW: iterate viewports; for each, set viewport+scissor then issue draw calls
    for vp in viewports {
        // cmd_set_viewport(recorder.command_buffer, 0, &[...])
        // cmd_set_scissor(recorder.command_buffer, 0, &[...])
        // use vp.proj * vp.view as the VP matrix (column-vector convention: proj on left)
        for cmd in &self.render_queue {
            // cmd_push_constants, cmd_bind_vertex/index, cmd_draw_indexed
        }
    }
}
```

- [ ] **Step 2: Update existing tests that called the old signature**

Find all test call sites:
```bash
grep -r "render_meshes" engine/renderer/tests/
```

Update each to pass `recorder`, `None` (for assets, since tests likely don't have GPU), and an empty viewport slice, or add a `#[cfg(not(feature = "gpu-test"))]` skip.

- [ ] **Step 3: Run tests**

```bash
cargo test -p engine-renderer
```

Expected: passes.

- [ ] **Step 4: Commit**

```bash
git add engine/renderer/src/renderer.rs engine/renderer/tests/
git commit -m "feat(renderer): multi-viewport render_meshes with Option<assets> and FrameRecorder"
```

---

## Phase 3: Refactor `native_viewport.rs`

### Task 6: Replace `ViewportRenderer` with `engine_renderer::Renderer`

This is the largest single-file change. `ViewportRenderer` in `native_viewport.rs` currently contains ~600 lines of duplicated Vulkan setup. We replace the struct's internals while preserving all public behaviour.

**Files:**
- Modify: `engine/editor/src-tauri/viewport/native_viewport.rs`
- Modify: `engine/editor/Cargo.toml`

- [ ] **Step 1: Add `engine-render-context` to editor deps (Windows only)**

In `engine/editor/Cargo.toml`:
```toml
[target.'cfg(windows)'.dependencies]
engine-render-context = { path = "../render-context" }
# engine-renderer is already there
```

- [ ] **Step 2: Replace `ViewportRenderer` struct fields**

```rust
struct ViewportRenderer {
    renderer: engine_renderer::Renderer,
    grid_pipeline: GridPipeline,   // keep existing struct unchanged
    // gizmo_pipeline added in Phase 6
}
```

Remove all the duplicated Vulkan fields (`context`, `device`, `swapchain`, `render_pass`, etc.).

- [ ] **Step 3: Add getter methods to `Renderer` in `engine/renderer/src/renderer.rs`**

`GridPipeline` (and later `GizmoPipeline`) need access to the raw Vulkan handles owned by `Renderer`. Add these three methods:

```rust
impl Renderer {
    pub fn device(&self) -> &ash::Device {
        &self.context.device
    }

    pub fn render_pass(&self) -> vk::RenderPass {
        self.render_pass.handle()
    }

    pub fn extent(&self) -> vk::Extent2D {
        self.swapchain.extent()
    }

    pub fn context(&self) -> &engine_render_context::RenderContext {
        &self.context
    }
}
```

`context()` is needed by `GizmoPipeline::new` (Task 14/15), which takes `&RenderContext` to access both the device and the GPU allocator for buffer creation.

Run `cargo check -p engine-renderer` to confirm the methods compile.

- [ ] **Step 4: Update `ViewportRenderer::new(hwnd)`**

```rust
impl ViewportRenderer {
    fn new(hwnd: isize, width: u32, height: u32) -> Result<Self, ...> {
        let renderer = engine_renderer::Renderer::from_raw_handle(hwnd, width, height)?;
        // Rebuild grid_pipeline using the new getter methods
        let grid_pipeline = GridPipeline::new(renderer.device(), renderer.render_pass(), renderer.extent())?;
        Ok(Self { renderer, grid_pipeline })
    }
}
```

- [ ] **Step 5: Update `render_frame` to use `begin_frame`/`end_frame`**

```rust
fn render_frame(&mut self, viewports: &[(ViewportBounds, OrbitCamera, bool, bool)]) -> Result<(), ...> {
    let Some(recorder) = self.renderer.begin_frame() else { return Ok(()); };

    let vp_descs: Vec<_> = viewports.iter().map(|(b, cam, _, is_ortho)| ViewportDescriptor {
        bounds: Rect { x: b.x, y: b.y, width: b.width, height: b.height },
        view: cam.view_matrix(),
        proj: cam.projection(*is_ortho),
    }).collect();

    self.renderer.render_meshes(&recorder, &World::default(), None, &vp_descs);

    for (bounds, cam, grid_visible, is_ortho) in viewports {
        if *grid_visible {
            self.grid_pipeline.record(&recorder.command_buffer, bounds, cam, *is_ortho);
        }
    }

    self.renderer.end_frame(recorder);
    Ok(())
}
```

- [ ] **Step 6: Verify editor builds**

```bash
cargo check -p silmaril-editor
```

Expected: compiles. Fix any import errors.

- [ ] **Step 7: Run the editor and confirm the grid still renders**

```bash
cargo tauri dev -p silmaril-editor
```

Expected: editor opens, viewport shows the grid exactly as before. Camera orbit/pan/zoom still works.

- [ ] **Step 8: Commit**

```bash
git add engine/editor/
git commit -m "refactor(editor/viewport): ViewportRenderer wraps engine_renderer::Renderer, removes duplicated Vulkan code"
```

---

## Phase 4: ECS World in Tauri State

### Task 7: Add `SceneWorldState` managed state

**Files:**
- Create: `engine/editor/src-tauri/state/mod.rs`
- Create: `engine/editor/src-tauri/state/scene_world.rs`
- Modify: `engine/editor/src-tauri/lib.rs`

- [ ] **Step 1: Create `state/scene_world.rs`**

```rust
use std::sync::{Arc, RwLock};
use engine_core::World;

pub struct SceneWorldState(pub Arc<RwLock<World>>);

impl SceneWorldState {
    pub fn new() -> Self {
        Self(Arc::new(RwLock::new(World::new())))
    }
}
```

- [ ] **Step 2: Create `state/mod.rs`**

```rust
pub mod scene_world;
pub use scene_world::SceneWorldState;
```

- [ ] **Step 3: Manage state in `lib.rs`**

In `lib.rs`, in `tauri::Builder::default()`:
```rust
.manage(crate::state::SceneWorldState::new())
```

Add `mod state;` at the top of `lib.rs`.

- [ ] **Step 4: Pass world Arc to viewport render thread**

In the viewport initialization code (where the render thread is spawned), clone the `Arc<RwLock<World>>` and move it into the render thread closure. The thread currently uses `Arc<Mutex<HashMap<String, ViewportInstance>>>` — add the world Arc alongside it.

Update `ViewportRenderer::render_frame` signature to accept `&RwLock<World>`:
```rust
fn render_frame(
    &mut self,
    viewports: &[(ViewportBounds, OrbitCamera, bool, bool)],
    world: &RwLock<World>,
) -> Result<(), ...> {
    let world_read = world.read().unwrap();
    // pass &world_read to render_meshes
}
```

- [ ] **Step 5: Check compiles**

```bash
cargo check -p silmaril-editor
```

- [ ] **Step 6: Commit**

```bash
git add engine/editor/src-tauri/state/ engine/editor/src-tauri/lib.rs engine/editor/src-tauri/viewport/
git commit -m "feat(editor): add SceneWorldState managed state, wire to viewport render thread"
```

---

### Task 8: Wire `create_entity` and `delete_entity` to the world

**Files:**
- Modify: `engine/editor/src-tauri/bridge/commands.rs`

- [ ] **Step 1: Write test**

In `engine/editor/src-tauri/bridge/commands.rs` (or a test module):

```rust
#[cfg(test)]
mod tests {
    use engine_core::{World, components::Transform};
    use std::sync::{Arc, RwLock};

    #[test]
    fn create_entity_adds_transform_to_world() {
        let world = Arc::new(RwLock::new(World::new()));
        // simulate what create_entity_impl does:
        let id = {
            let mut w = world.write().unwrap();
            let e = w.spawn();
            w.add(e, Transform::default());
            e.id()
        };
        let w = world.read().unwrap();
        assert!(w.get::<Transform>(engine_core::Entity::from_id(id)).is_some());
    }
}
```

- [ ] **Step 2: Run test to verify it fails (function not yet extracted)**

```bash
cargo test -p silmaril-editor -- create_entity_adds_transform
```

- [ ] **Step 3: Implement `create_entity` writing to world**

In `commands.rs`, update the `create_entity` Tauri command:

```rust
#[tauri::command]
pub fn create_entity(
    name: Option<String>,
    world_state: tauri::State<'_, crate::state::SceneWorldState>,
    app: tauri::AppHandle,
) -> Result<u64, String> {
    use engine_core::components::Transform;
    let entity_id = {
        let mut world = world_state.0.write().map_err(|e| e.to_string())?;
        let entity = world.spawn();
        world.add(entity, Transform::default());
        entity.id()
    };
    let entity_name = name.unwrap_or_else(|| format!("Entity {entity_id}"));
    app.emit("entity-created", serde_json::json!({ "id": entity_id, "name": entity_name }))
        .map_err(|e| e.to_string())?;
    Ok(entity_id)
}
```

Similarly implement `delete_entity`:
```rust
#[tauri::command]
pub fn delete_entity(
    entity_id: u64,
    world_state: tauri::State<'_, crate::state::SceneWorldState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let mut world = world_state.0.write().map_err(|e| e.to_string())?;
    world.despawn(engine_core::Entity::from_id(entity_id));
    app.emit("entity-deleted", serde_json::json!({ "id": entity_id }))
        .map_err(|e| e.to_string())?;
    Ok(())
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test -p silmaril-editor -- create_entity
```

Expected: passes.

- [ ] **Step 5: Commit**

```bash
git add engine/editor/src-tauri/bridge/commands.rs
git commit -m "feat(editor): create_entity and delete_entity write to ECS world, emit Tauri events"
```

---

### Task 9: Wire `set_component_field` (transform) to world + emit events

**Files:**
- Modify: `engine/editor/src-tauri/bridge/commands.rs`

- [ ] **Step 1: Write test**

```rust
#[test]
fn set_component_field_updates_transform_position() {
    let world = Arc::new(RwLock::new(World::new()));
    let entity = { let mut w = world.write().unwrap(); w.spawn() };
    { let mut w = world.write().unwrap(); w.add(entity, Transform::default()); }

    // simulate set_component_field("position", "x", 5.0)
    {
        let mut w = world.write().unwrap();
        if let Some(t) = w.get_mut::<Transform>(entity) {
            t.position.x = 5.0;
        }
    }

    let w = world.read().unwrap();
    let t = w.get::<Transform>(entity).unwrap();
    assert_eq!(t.position.x, 5.0);
}
```

- [ ] **Step 2: Implement `set_component_field` writing to world**

The command receives `entity_id: u64`, `component: String`, `field: String`, `value: serde_json::Value`. For Transform fields:

```rust
#[tauri::command]
pub fn set_component_field(
    entity_id: u64,
    component: String,
    field: String,
    value: serde_json::Value,
    world_state: tauri::State<'_, crate::state::SceneWorldState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let entity = engine_core::Entity::from_id(entity_id);
    let val = value.as_f64().ok_or("value must be a float")? as f32;
    {
        let mut world = world_state.0.write().map_err(|e| e.to_string())?;
        match component.as_str() {
            "Transform" => {
                let t = world.get_mut::<engine_core::components::Transform>(entity)
                    .ok_or("entity has no Transform")?;
                match field.as_str() {
                    "position.x" => t.position.x = val,
                    "position.y" => t.position.y = val,
                    "position.z" => t.position.z = val,
                    "rotation.x" => t.rotation.x = val,
                    "rotation.y" => t.rotation.y = val,
                    "rotation.z" => t.rotation.z = val,
                    "rotation.w" => t.rotation.w = val,
                    "scale.x" => t.scale.x = val,
                    "scale.y" => t.scale.y = val,
                    "scale.z" => t.scale.z = val,
                    _ => return Err(format!("unknown Transform field: {field}")),
                }
                // Emit transform-changed event with full current transform
                let t = world.get::<engine_core::components::Transform>(entity).unwrap();
                let pos = [t.position.x, t.position.y, t.position.z];
                let rot = [t.rotation.x, t.rotation.y, t.rotation.z, t.rotation.w];
                let scl = [t.scale.x, t.scale.y, t.scale.z];
                app.emit("entity-transform-changed", serde_json::json!({
                    "id": entity_id, "position": pos, "rotation": rot, "scale": scl
                })).map_err(|e| e.to_string())?;
            }
            _ => { /* other components: no-op for now */ }
        }
    }
    Ok(())
}
```

- [ ] **Step 3: Run tests**

```bash
cargo test -p silmaril-editor -- set_component_field
```

- [ ] **Step 4: Commit**

```bash
git add engine/editor/src-tauri/bridge/commands.rs
git commit -m "feat(editor): set_component_field writes Transform to ECS world, emits entity-transform-changed"
```

---

### Task 10: Frontend listens to entity-transform-changed

**Files:**
- Modify: `engine/editor/src/lib/scene/state.ts`

- [ ] **Step 1: Add event listener in `state.ts`**

In the `onMount` or initialization block (the frontend already subscribes to other Tauri events):

```typescript
import { listen } from '@tauri-apps/api/event';

// Listen for transform changes from gizmo drags and IPC field edits
await listen<{ id: number; position: [number,number,number]; rotation: [number,number,number,number]; scale: [number,number,number] }>(
  'entity-transform-changed',
  (event) => {
    const { id, position, rotation, scale } = event.payload;
    _mutateEntity(id, (entity) => {
      entity.position = { x: position[0], y: position[1], z: position[2] };
      entity.rotation = { x: rotation[0], y: rotation[1], z: rotation[2], w: rotation[3] };
      entity.scale    = { x: scale[0],    y: scale[1],    z: scale[2] };
    });
  }
);
```

Where `_mutateEntity` is an existing helper (or add one) that updates the in-memory entity map and notifies subscribers.

Also add listeners for `entity-created` and `entity-deleted` (they may already exist; verify and add if not).

- [ ] **Step 2: Verify in dev mode**

Start `cargo tauri dev`. In the browser console, call:
```js
window.__TAURI__.core.invoke('create_entity', { name: 'TestBox' })
```
Expected: entity appears in the Hierarchy panel.

- [ ] **Step 3: Commit**

```bash
git add engine/editor/src/lib/scene/state.ts
git commit -m "feat(editor): frontend mirrors entity-created/deleted/transform-changed Tauri events"
```

---

## Phase 5: Scene Undo Stack

### Task 11: `SceneUndoStack` + `scene_undo`/`scene_redo` IPC

**Files:**
- Create: `engine/editor/src-tauri/state/scene_undo.rs`
- Modify: `engine/editor/src-tauri/state/mod.rs`
- Modify: `engine/editor/src-tauri/lib.rs`

- [ ] **Step 1: Write tests**

```rust
// In state/scene_undo.rs #[cfg(test)]
#[test]
fn undo_restores_previous_transform() {
    let mut stack = SceneUndoStack::new();
    stack.push(SceneAction::SetTransform {
        entity_id: 1,
        before: SerializedTransform { position: [0.0, 0.0, 0.0], rotation: [0.0,0.0,0.0,1.0], scale: [1.0,1.0,1.0] },
        after:  SerializedTransform { position: [5.0, 0.0, 0.0], rotation: [0.0,0.0,0.0,1.0], scale: [1.0,1.0,1.0] },
    });
    assert!(stack.can_undo());
    let action = stack.pop_undo().unwrap();
    assert!(!stack.can_undo());
    assert!(stack.can_redo());
    // action.before is [0,0,0]
    if let SceneAction::SetTransform { before, .. } = action {
        assert_eq!(before.position[0], 0.0);
    }
}

#[test]
fn push_clears_redo_stack() {
    let mut stack = SceneUndoStack::new();
    stack.push(SceneAction::SetTransform { entity_id: 1, before: Default::default(), after: Default::default() });
    stack.pop_undo();  // moves to redo
    stack.push(SceneAction::SetTransform { entity_id: 2, before: Default::default(), after: Default::default() });
    assert!(!stack.can_redo());  // redo was cleared
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test -p silmaril-editor -- undo_restores
```

Expected: `SceneUndoStack` not found.

- [ ] **Step 3: Implement `state/scene_undo.rs`**

```rust
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct SerializedTransform {
    pub position: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
}

pub enum SceneAction {
    SetTransform {
        entity_id: u64,
        before: SerializedTransform,
        after: SerializedTransform,
    },
}

pub struct SceneUndoStack {
    undo: Vec<SceneAction>,
    redo: Vec<SceneAction>,
}

impl SceneUndoStack {
    pub fn new() -> Self { Self { undo: vec![], redo: vec![] } }
    pub fn can_undo(&self) -> bool { !self.undo.is_empty() }
    pub fn can_redo(&self) -> bool { !self.redo.is_empty() }

    pub fn push(&mut self, action: SceneAction) {
        self.redo.clear();
        self.undo.push(action);
    }

    /// Returns the action to apply_inverse.
    pub fn pop_undo(&mut self) -> Option<SceneAction> {
        let action = self.undo.pop()?;
        // push inverse to redo (swap before/after)
        if let SceneAction::SetTransform { entity_id, before, after } = &action {
            self.redo.push(SceneAction::SetTransform {
                entity_id: *entity_id,
                before: after.clone(),
                after: before.clone(),
            });
        }
        Some(action)
    }

    /// Returns the action to re-apply.
    pub fn pop_redo(&mut self) -> Option<SceneAction> {
        let action = self.redo.pop()?;
        if let SceneAction::SetTransform { entity_id, before, after } = &action {
            self.undo.push(SceneAction::SetTransform {
                entity_id: *entity_id,
                before: after.clone(),
                after: before.clone(),
            });
        }
        Some(action)
    }
}
```

- [ ] **Step 4: Add to `state/mod.rs` and manage in `lib.rs`**

```rust
// state/mod.rs
pub mod scene_undo;
pub mod scene_world;
pub use scene_undo::{SceneAction, SceneUndoStack, SerializedTransform};
pub use scene_world::SceneWorldState;
```

```rust
// lib.rs — in tauri::Builder::manage()
.manage(std::sync::Mutex::new(crate::state::SceneUndoStack::new()))
```

- [ ] **Step 5: Add `scene_undo` and `scene_redo` IPC commands to `bridge/commands.rs`**

```rust
#[tauri::command]
pub fn scene_undo(
    undo_state: tauri::State<'_, std::sync::Mutex<crate::state::SceneUndoStack>>,
    world_state: tauri::State<'_, crate::state::SceneWorldState>,
    app: tauri::AppHandle,
) -> Result<serde_json::Value, String> {
    let action = {
        let mut stack = undo_state.lock().map_err(|e| e.to_string())?;
        stack.pop_undo()
    };
    let Some(action) = action else {
        return Ok(serde_json::json!({ "canUndo": false, "canRedo": false }));
    };
    apply_scene_action(&action, &world_state, &app)?;
    let stack = undo_state.lock().map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "canUndo": stack.can_undo(), "canRedo": stack.can_redo() }))
}

#[tauri::command]
pub fn scene_redo(/* same signature */) -> Result<serde_json::Value, String> { /* mirror */ }

fn apply_scene_action(
    action: &crate::state::SceneAction,
    world_state: &crate::state::SceneWorldState,
    app: &tauri::AppHandle,
) -> Result<(), String> {
    use crate::state::SceneAction;
    match action {
        SceneAction::SetTransform { entity_id, after, .. } => {
            let entity = engine_core::Entity::from_id(*entity_id);
            let mut world = world_state.0.write().map_err(|e| e.to_string())?;
            if let Some(t) = world.get_mut::<engine_core::components::Transform>(entity) {
                t.position = glam::Vec3::from(after.position);
                t.rotation = glam::Quat::from_array(after.rotation);
                t.scale    = glam::Vec3::from(after.scale);
            }
            app.emit("entity-transform-changed", serde_json::json!({
                "id": entity_id,
                "position": after.position,
                "rotation": after.rotation,
                "scale": after.scale,
            })).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
```

Register `scene_undo` and `scene_redo` in `lib.rs`'s `generate_handler!`.

- [ ] **Step 6: Run tests**

```bash
cargo test -p silmaril-editor -- undo_restores push_clears_redo
```

Expected: passes.

- [ ] **Step 7: Commit**

```bash
git add engine/editor/src-tauri/state/ engine/editor/src-tauri/bridge/commands.rs engine/editor/src-tauri/lib.rs
git commit -m "feat(editor): SceneUndoStack with scene_undo/scene_redo IPC commands"
```

---

### Task 12: Frontend Ctrl+Z routing

**Files:**
- Modify: `engine/editor/src/lib/stores/undo-history.ts`
- Modify: `engine/editor/src/App.svelte`

- [ ] **Step 1: Add `sceneUndo`/`sceneRedo` to `undo-history.ts`**

```typescript
import { invoke } from '@tauri-apps/api/core';

let _sceneCanUndo = false;
let _sceneCanRedo = false;

export function getSceneCanUndo(): boolean { return _sceneCanUndo; }
export function getSceneCanRedo(): boolean { return _sceneCanRedo; }

export async function sceneUndo(): Promise<void> {
  const result = await invoke<{ canUndo: boolean; canRedo: boolean }>('scene_undo');
  _sceneCanUndo = result.canUndo;
  _sceneCanRedo = result.canRedo;
  notify();
}

export async function sceneRedo(): Promise<void> {
  const result = await invoke<{ canUndo: boolean; canRedo: boolean }>('scene_redo');
  _sceneCanUndo = result.canUndo;
  _sceneCanRedo = result.canRedo;
  notify();
}
```

- [ ] **Step 2: Route Ctrl+Z in `App.svelte`**

In the existing `handleKeyDown` handler, when `Ctrl+Z` is pressed, check whether a viewport panel is the active focus:

```typescript
if (e.key === 'z' && e.ctrlKey && !e.shiftKey) {
    e.preventDefault();
    if (viewportHasFocus) {
        await sceneUndo();
    } else {
        await undo(); // existing template undo
    }
    return;
}
```

`viewportHasFocus` can be a simple boolean state set by `onmouseenter`/`onmouseleave` on the viewport panel.

- [ ] **Step 3: Verify**

In dev mode: create an entity, drag it (once gizmo is implemented), press Ctrl+Z. Entity should return to original position.

- [ ] **Step 4: Commit**

```bash
git add engine/editor/src/lib/stores/undo-history.ts engine/editor/src/App.svelte
git commit -m "feat(editor): route Ctrl+Z to scene_undo when viewport is focused"
```

---

## Phase 6: Gizmo Pipeline

### Task 13: Gizmo GLSL shaders

**Files:**
- Create: `engine/editor/src-tauri/viewport/shaders/gizmo.vert`
- Create: `engine/editor/src-tauri/viewport/shaders/gizmo.frag`

The editor compiles GLSL via `naga` at runtime (see `native_viewport.rs` for the existing grid shader pattern).

- [ ] **Step 1: Write `gizmo.vert`**

```glsl
#version 450

layout(location = 0) in vec3 inLocalPos;   // vertex position in gizmo-local space

layout(push_constant) uniform PushConstants {
    mat4  viewProj;
    vec3  gizmoOriginWorld;  // entity world position
    float _pad0;
    vec4  color;             // rgba
    float scale;             // dist * 0.15
    vec3  _pad1;
} pc;

void main() {
    float dist = max(length(pc.gizmoOriginWorld - vec3(0.0)), 0.1);
    // camera_pos not passed — scale is pre-computed CPU-side from camera distance
    vec3 worldPos = pc.gizmoOriginWorld + inLocalPos * pc.scale;
    gl_Position = pc.viewProj * vec4(worldPos, 1.0);
}
```

Note: `scale` is computed on the CPU as `dist * 0.15` before issuing push constants. This keeps the shader simple.

- [ ] **Step 2: Write `gizmo.frag`**

```glsl
#version 450

layout(push_constant) uniform PushConstants {
    mat4  viewProj;
    vec3  gizmoOriginWorld;
    float _pad0;
    vec4  color;
    float scale;
    vec3  _pad1;
} pc;

layout(location = 0) out vec4 outColor;

void main() {
    outColor = pc.color;
}
```

Push constant total: `mat4(64) + vec3+pad(16) + vec4(16) + float+pad(16)` = 112 bytes — within 128-byte limit.

- [ ] **Step 3: Verify naga can compile them**

Add a test in `gizmo_pipeline.rs` (Task 14) that calls the naga compilation path and asserts it succeeds. No standalone test here — defer to Task 14.

---

### Task 14: `GizmoPipeline` — procedural geometry + pipeline creation

**Files:**
- Create: `engine/editor/src-tauri/viewport/gizmo_pipeline.rs`
- Modify: `engine/editor/src-tauri/viewport/mod.rs`

- [ ] **Step 1: Write test for procedural geometry generation**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crosshair_generates_6_vertices() {
        let verts = generate_crosshair_vertices();
        assert_eq!(verts.len(), 6);
        // X axis: (1,0,0) and (-1,0,0) — or (1,0,0) and (0,0,0) etc.
        // Just check count for now
    }

    #[test]
    fn move_arrow_generates_nonzero_vertices() {
        let verts = generate_move_arrow_vertices(GizmoAxis::X);
        assert!(!verts.is_empty());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test -p silmaril-editor -- crosshair_generates_6_vertices
```

- [ ] **Step 3: Implement `gizmo_pipeline.rs`**

```rust
use ash::vk;
use engine_render_context::GraphicsPipeline;

pub struct GizmoPipeline {
    pipeline: GraphicsPipeline,
    // static vertex buffers
    crosshair_vertices: engine_render_context::VertexBuffer,  // 6 verts
    move_arrow_x: engine_render_context::VertexBuffer,
    move_arrow_y: engine_render_context::VertexBuffer,
    move_arrow_z: engine_render_context::VertexBuffer,
    rotate_ring_x: engine_render_context::VertexBuffer,
    rotate_ring_y: engine_render_context::VertexBuffer,
    rotate_ring_z: engine_render_context::VertexBuffer,
    scale_handle_x: engine_render_context::VertexBuffer,
    scale_handle_y: engine_render_context::VertexBuffer,
    scale_handle_z: engine_render_context::VertexBuffer,
}

/// Vertex for gizmo geometry: just a 3D position.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct GizmoVertex {
    pos: [f32; 3],
}

pub fn generate_crosshair_vertices() -> Vec<GizmoVertex> {
    // 3 axis lines, 2 verts each = 6 verts
    vec![
        GizmoVertex { pos: [0.0, 0.0, 0.0] }, GizmoVertex { pos: [1.0, 0.0, 0.0] }, // X
        GizmoVertex { pos: [0.0, 0.0, 0.0] }, GizmoVertex { pos: [0.0, 1.0, 0.0] }, // Y
        GizmoVertex { pos: [0.0, 0.0, 0.0] }, GizmoVertex { pos: [0.0, 0.0, 1.0] }, // Z
    ]
}

pub fn generate_move_arrow_vertices(axis: GizmoAxis) -> Vec<GizmoVertex> {
    // Shaft: cylinder approximated as 8 quad-strips + cone tip
    // For simplicity: just a line + cone (line list topology)
    // Line: origin to (0,0,0.8); cone: 8 triangles from (0,0,0.8) to tip at (0,0,1.0)
    // All in axis-local space; rotation applied via model matrix
    let mut verts = vec![];
    // shaft (line)
    verts.push(GizmoVertex { pos: [0.0, 0.0, 0.0] });
    verts.push(GizmoVertex { pos: [0.0, 0.0, 0.8] });
    // cone (triangle fan)
    let tip = [0.0f32, 0.0, 1.0];
    let r = 0.06f32;
    let n = 8usize;
    for i in 0..n {
        let a0 = (i as f32) * std::f32::consts::TAU / n as f32;
        let a1 = ((i + 1) as f32) * std::f32::consts::TAU / n as f32;
        verts.push(GizmoVertex { pos: [r * a0.cos(), r * a0.sin(), 0.8] });
        verts.push(GizmoVertex { pos: [r * a1.cos(), r * a1.sin(), 0.8] });
        verts.push(GizmoVertex { pos: tip });
    }
    verts
}

// ... generate_rotate_ring_vertices, generate_scale_handle_vertices similarly

impl GizmoPipeline {
    pub fn new(
        context: &engine_render_context::RenderContext,
        render_pass: vk::RenderPass,
    ) -> Result<Self, ...> {
        // 1. Compile gizmo.vert + gizmo.frag via naga (same pattern as grid shader in native_viewport.rs)
        // 2. Create VkPipeline with:
        //    - vertex input: binding 0, stride 12, one attrib (position, R32G32B32_SFLOAT @ 0)
        //    - topology: TRIANGLE_LIST (use LINE_LIST for crosshair/rings — or separate pipelines)
        //    - depth test: DISABLED
        //    - push constants: 112 bytes, vertex+fragment stage
        // 3. Upload procedural geometry to vertex buffers
        todo!()
    }

    pub fn record(
        &self,
        cmd: vk::CommandBuffer,
        world: &engine_core::World,
        viewports: &[(ViewportBounds, OrbitCamera, bool, bool)],
        selected_entity: Option<u64>,
        mode: GizmoMode,
        device: &ash::Device,
    ) {
        // For each entity with Transform:
        //   compute scale = camera_dist * 0.15
        //   issue crosshair draw (6 verts, LINE_LIST)
        // For selected entity:
        //   issue move/rotate/scale geometry depending on mode
        todo!()
    }
}
```

Note on topology: crosshairs and rotate rings use `LINE_LIST`/`LINE_STRIP`; move arrows and scale handles use `TRIANGLE_LIST`. You can use a single pipeline with `polygonMode = LINE` to draw triangles as wireframe (gizmos are always wireframe-ish), or create two pipelines. Simpler: use `LINE_LIST` throughout and represent all geometry as line segments.

- [ ] **Step 4: Run geometry tests**

```bash
cargo test -p silmaril-editor -- crosshair_generates_6_vertices move_arrow_generates
```

Expected: passes.

- [ ] **Step 5: Commit**

```bash
git add engine/editor/src-tauri/viewport/gizmo_pipeline.rs engine/editor/src-tauri/viewport/shaders/
git commit -m "feat(editor): GizmoPipeline with procedural geometry + GLSL shaders"
```

---

### Task 15: Integrate `GizmoPipeline` into `ViewportRenderer`

**Files:**
- Modify: `engine/editor/src-tauri/viewport/native_viewport.rs`

- [ ] **Step 1: Add `gizmo_pipeline` field**

```rust
struct ViewportRenderer {
    renderer: engine_renderer::Renderer,
    grid_pipeline: GridPipeline,
    gizmo_pipeline: GizmoPipeline,  // new
}
```

Initialize in `ViewportRenderer::new()`:
```rust
let gizmo_pipeline = GizmoPipeline::new(renderer.context(), renderer.render_pass())?;
```

- [ ] **Step 2: Call `gizmo_pipeline.record` in `render_frame`**

After the grid recording block:
```rust
self.gizmo_pipeline.record(
    recorder.command_buffer,
    &world_read,
    viewports,
    selected_entity,
    gizmo_mode,
    self.renderer.device(),
);
```

`selected_entity` and `gizmo_mode` come from the render thread's snapshot of `NativeViewportState` — add these to the instances snapshot.

- [ ] **Step 3: Build and run**

```bash
cargo tauri dev -p silmaril-editor
```

Expected: editor opens, crosshair axes appear at each entity's world position. No crash.

- [ ] **Step 4: Commit**

```bash
git add engine/editor/src-tauri/viewport/native_viewport.rs engine/editor/src-tauri/viewport/mod.rs
git commit -m "feat(editor): integrate GizmoPipeline into ViewportRenderer, entity crosshairs visible"
```

---

## Phase 7: Gizmo Interaction

### Task 16: `DragState` + `gizmo_hit_test`

**Files:**
- Create: `engine/editor/src-tauri/bridge/gizmo_commands.rs`
- Modify: `engine/editor/src-tauri/bridge/commands.rs` (add `drag_state` to `NativeViewportState`)
- Modify: `engine/editor/src-tauri/lib.rs` (register commands)

- [ ] **Step 1: Write ray-capsule intersection test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use glam::{Vec3, Mat4};

    #[test]
    fn ray_hits_x_axis_handle() {
        // Camera at (0, 0, -5) looking toward +Z
        // Entity at origin; X-axis handle runs from (0,0,0) to (1,0,0)
        // Ray through screen center hits the X handle
        let ray_origin = Vec3::new(0.5, 0.0, -5.0);
        let ray_dir = Vec3::new(0.0, 0.0, 1.0);
        let capsule_start = Vec3::new(0.0, 0.0, 0.0);
        let capsule_end   = Vec3::new(1.0, 0.0, 0.0);
        let radius = 0.1;
        assert!(ray_capsule_intersects(ray_origin, ray_dir, capsule_start, capsule_end, radius));
    }

    #[test]
    fn ray_misses_when_offset() {
        let ray_origin = Vec3::new(5.0, 5.0, -5.0);
        let ray_dir = Vec3::new(0.0, 0.0, 1.0);
        let capsule_start = Vec3::new(0.0, 0.0, 0.0);
        let capsule_end   = Vec3::new(1.0, 0.0, 0.0);
        assert!(!ray_capsule_intersects(ray_origin, ray_dir, capsule_start, capsule_end, 0.1));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test -p silmaril-editor -- ray_hits_x_axis
```

- [ ] **Step 3: Implement `ray_capsule_intersects`**

```rust
/// Returns true if a ray (origin, dir) passes within `radius` of the capsule segment [a, b].
pub fn ray_capsule_intersects(
    ray_origin: glam::Vec3,
    ray_dir: glam::Vec3,    // normalized
    cap_a: glam::Vec3,
    cap_b: glam::Vec3,
    radius: f32,
) -> bool {
    // Closest distance between two line segments (ray vs capsule axis)
    let ab = cap_b - cap_a;
    let ao = ray_origin - cap_a;
    let d = ray_dir;
    let denom = d.dot(d) * ab.dot(ab) - d.dot(ab).powi(2);
    if denom.abs() < 1e-6 { return false; } // parallel
    let t = (d.dot(ab) * ab.dot(ao) - ab.dot(ab) * d.dot(ao)) / denom;
    let s = (d.dot(ab) * d.dot(ao) - d.dot(d) * ab.dot(ao)) / denom;
    let s = s.clamp(0.0, 1.0);
    let closest_ray = ray_origin + d * t.max(0.0);
    let closest_cap = cap_a + ab * s;
    closest_ray.distance(closest_cap) <= radius
}
```

- [ ] **Step 4: Add `DragState` to `NativeViewportState`**

In `bridge/commands.rs`:

```rust
use std::sync::Mutex;

pub struct NativeViewportState {
    pub viewport: Mutex<crate::viewport::ViewportRegistry>,  // existing
    pub drag_state: Mutex<Option<DragState>>,                 // new
    pub gizmo_mode: std::sync::atomic::AtomicU8,              // 0=move, 1=rotate, 2=scale
}

pub struct DragState {
    pub entity_id: u64,
    pub viewport_id: String,
    pub axis: GizmoAxis,
    pub mode: GizmoMode,
    pub transform_before: crate::state::SerializedTransform,
    pub last_screen: (f32, f32),
    pub camera_view: glam::Mat4,
    pub camera_proj: glam::Mat4,
    pub camera_pos: glam::Vec3,
    pub gizmo_scale: f32,
}

#[derive(Clone, Copy, PartialEq)]
pub enum GizmoAxis { X, Y, Z, XY, XZ, YZ }

#[derive(Clone, Copy, PartialEq)]
pub enum GizmoMode { Move, Rotate, Scale }
```

- [ ] **Step 5: Implement `gizmo_hit_test` command**

```rust
// bridge/gizmo_commands.rs
#[tauri::command]
pub fn gizmo_hit_test(
    viewport_id: String,
    screen_x: f32,
    screen_y: f32,
    entity_id: u64,   // the currently selected entity to test gizmo handles against
    viewport_state: tauri::State<'_, NativeViewportState>,
    world_state: tauri::State<'_, crate::state::SceneWorldState>,
) -> Option<serde_json::Value> {
    // 1. Get entity transform to compute camera distance for screen-space size
    let entity = engine_core::Entity::from_id(entity_id);
    let entity_pos = {
        let world = world_state.0.read().ok()?;
        world.get::<engine_core::components::Transform>(entity)?.position
    };

    // 2. Snapshot camera from viewport instances
    let (view, proj, cam_pos, bounds, gizmo_scale) = {
        let vp = viewport_state.viewport.lock().ok()?;
        let inst = vp.get(&viewport_id)?;
        let cam_pos = inst.camera.eye();
        let dist = (cam_pos - entity_pos).length().max(0.1);
        let view = inst.camera.view_matrix();
        let proj = inst.camera.projection(inst.is_ortho);
        (view, proj, cam_pos, inst.bounds, dist * 0.15)
    };

    // 3. Unproject screen → ray
    let (ray_origin, ray_dir) = unproject_screen(screen_x, screen_y, &bounds, view, proj);
    // 4. Test ray against each axis handle (capsule at entity_pos, scaled by gizmo_scale)
    // ...
    // 5. On hit: store DragState and return { axis, mode }
    // On miss: return None
    None
}

fn unproject_screen(sx: f32, sy: f32, bounds: &ViewportBounds, view: glam::Mat4, proj: glam::Mat4) -> (glam::Vec3, glam::Vec3) {
    // NDC: x in [-1,1], y in [-1,1]
    let ndc_x = (sx - bounds.x as f32) / bounds.width as f32 * 2.0 - 1.0;
    let ndc_y = 1.0 - (sy - bounds.y as f32) / bounds.height as f32 * 2.0;
    let inv_vp = (proj * view).inverse();
    let near = inv_vp.project_point3(glam::Vec3::new(ndc_x, ndc_y, 0.0));
    let far  = inv_vp.project_point3(glam::Vec3::new(ndc_x, ndc_y, 1.0));
    let dir = (far - near).normalize();
    (near, dir)
}
```

- [ ] **Step 6: Run tests**

```bash
cargo test -p silmaril-editor -- ray_hits_x_axis ray_misses_when_offset
```

Expected: passes.

- [ ] **Step 7: Commit**

```bash
git add engine/editor/src-tauri/bridge/gizmo_commands.rs engine/editor/src-tauri/bridge/commands.rs
git commit -m "feat(editor): DragState, ray-capsule intersection, gizmo_hit_test command"
```

---

### Task 17: `gizmo_drag` (move mode)

**Files:**
- Modify: `engine/editor/src-tauri/bridge/gizmo_commands.rs`

- [ ] **Step 1: Write test**

```rust
#[test]
fn drag_on_x_axis_moves_entity_x() {
    // Given: entity at (0,0,0), camera at (0,0,-5), dragging right
    // Expect: entity moves in +X
    let before = glam::Vec3::ZERO;
    let axis_world = glam::Vec3::X;
    let screen_delta = (10.0_f32, 0.0_f32); // 10px right
    let screen_width = 800.0_f32;
    let delta = project_screen_delta_to_axis(screen_delta, axis_world, screen_width, 1.0);
    assert!(delta.x > 0.0, "should move in +X, got {:?}", delta);
    assert!(delta.y.abs() < 0.001);
    assert!(delta.z.abs() < 0.001);
}
```

- [ ] **Step 2: Implement `project_screen_delta_to_axis`**

```rust
/// Projects a screen-space pixel delta onto a world-space axis vector.
pub fn project_screen_delta_to_axis(
    screen_delta: (f32, f32),
    axis: glam::Vec3,
    screen_width: f32,
    world_scale: f32,
) -> glam::Vec3 {
    // Simple: map pixel delta to a world-unit delta proportional to screen size
    let magnitude = (screen_delta.0 * axis.x + screen_delta.1 * -axis.y)
        / screen_width * world_scale * 2.0;
    axis * magnitude
}
```

- [ ] **Step 3: Implement `gizmo_drag` command**

```rust
#[tauri::command]
pub fn gizmo_drag(
    viewport_id: String,
    screen_x: f32,
    screen_y: f32,
    viewport_state: tauri::State<'_, NativeViewportState>,
    world_state: tauri::State<'_, crate::state::SceneWorldState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let mut drag = viewport_state.drag_state.lock().map_err(|e| e.to_string())?;
    let Some(ref mut ds) = *drag else { return Ok(()); };
    if ds.viewport_id != viewport_id { return Ok(()); }

    let dx = screen_x - ds.last_screen.0;
    let dy = screen_y - ds.last_screen.1;
    ds.last_screen = (screen_x, screen_y);

    let axis_vec = match ds.axis {
        GizmoAxis::X => glam::Vec3::X,
        GizmoAxis::Y => glam::Vec3::Y,
        GizmoAxis::Z => glam::Vec3::Z,
        _ => return Ok(()),  // planar handles: TODO
    };

    match ds.mode {
        GizmoMode::Move => {
            let delta = project_screen_delta_to_axis((dx, dy), axis_vec, 1024.0, ds.gizmo_scale * 2.0);
            let entity = engine_core::Entity::from_id(ds.entity_id);
            let mut world = world_state.0.write().map_err(|e| e.to_string())?;
            if let Some(t) = world.get_mut::<engine_core::components::Transform>(entity) {
                t.position += delta;
                let pos = [t.position.x, t.position.y, t.position.z];
                let rot = [t.rotation.x, t.rotation.y, t.rotation.z, t.rotation.w];
                let scl = [t.scale.x, t.scale.y, t.scale.z];
                app.emit("entity-transform-changed", serde_json::json!({
                    "id": ds.entity_id, "position": pos, "rotation": rot, "scale": scl
                })).map_err(|e| e.to_string())?;
            }
        }
        GizmoMode::Rotate | GizmoMode::Scale => { /* Task 18 */ }
    }
    Ok(())
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test -p silmaril-editor -- drag_on_x_axis_moves_entity_x
```

Expected: passes.

- [ ] **Step 5: Commit**

```bash
git add engine/editor/src-tauri/bridge/gizmo_commands.rs
git commit -m "feat(editor): gizmo_drag move mode — projects screen delta onto world axis"
```

---

### Task 18: `gizmo_drag` rotate + scale + `gizmo_drag_end`

**Files:**
- Modify: `engine/editor/src-tauri/bridge/gizmo_commands.rs`

- [ ] **Step 1: Write rotate test**

```rust
#[test]
fn drag_on_y_axis_rotates_entity() {
    // Horizontal drag around Y axis should produce a Quat rotation
    let angle_rad = std::f32::consts::PI / 4.0; // 45 degrees
    let q = glam::Quat::from_axis_angle(glam::Vec3::Y, angle_rad);
    assert!((q.w - (std::f32::consts::PI / 8.0).cos()).abs() < 0.01);
}
```

- [ ] **Step 2: Add rotate case to `gizmo_drag`**

```rust
GizmoMode::Rotate => {
    let angle = (dx / 300.0) * std::f32::consts::TAU; // 300px = full rotation
    let rotation_delta = glam::Quat::from_axis_angle(axis_vec, angle);
    let entity = engine_core::Entity::from_id(ds.entity_id);
    let mut world = world_state.0.write().map_err(|e| e.to_string())?;
    if let Some(t) = world.get_mut::<engine_core::components::Transform>(entity) {
        t.rotation = (rotation_delta * t.rotation).normalize();
        // emit entity-transform-changed
    }
}
```

- [ ] **Step 3: Add scale case**

```rust
GizmoMode::Scale => {
    let factor = 1.0 + dx / 200.0;
    let entity = engine_core::Entity::from_id(ds.entity_id);
    let mut world = world_state.0.write().map_err(|e| e.to_string())?;
    if let Some(t) = world.get_mut::<engine_core::components::Transform>(entity) {
        match ds.axis {
            GizmoAxis::X  => t.scale.x *= factor,
            GizmoAxis::Y  => t.scale.y *= factor,
            GizmoAxis::Z  => t.scale.z *= factor,
            _ => { t.scale.x *= factor; t.scale.y *= factor; t.scale.z *= factor; } // uniform
        }
        // emit entity-transform-changed
    }
}
```

- [ ] **Step 4: Implement `gizmo_drag_end`**

```rust
#[tauri::command]
pub fn gizmo_drag_end(
    viewport_id: String,
    viewport_state: tauri::State<'_, NativeViewportState>,
    world_state: tauri::State<'_, crate::state::SceneWorldState>,
    undo_state: tauri::State<'_, std::sync::Mutex<crate::state::SceneUndoStack>>,
) -> Result<(), String> {
    let drag = viewport_state.drag_state.lock().map_err(|e| e.to_string())?.take();
    let Some(ds) = drag else { return Ok(()); };

    // Snapshot current (final) transform
    let entity = engine_core::Entity::from_id(ds.entity_id);
    let world = world_state.0.read().map_err(|e| e.to_string())?;
    let t = world.get::<engine_core::components::Transform>(entity)
        .ok_or("entity not found")?;
    let after = crate::state::SerializedTransform {
        position: [t.position.x, t.position.y, t.position.z],
        rotation: [t.rotation.x, t.rotation.y, t.rotation.z, t.rotation.w],
        scale:    [t.scale.x, t.scale.y, t.scale.z],
    };
    drop(world);

    // Push to undo stack
    undo_state.lock().map_err(|e| e.to_string())?.push(
        crate::state::SceneAction::SetTransform {
            entity_id: ds.entity_id,
            before: ds.transform_before,
            after,
        }
    );
    Ok(())
}
```

- [ ] **Step 5: Implement `set_gizmo_mode`**

```rust
#[tauri::command]
pub fn set_gizmo_mode(
    mode: String,
    viewport_state: tauri::State<'_, NativeViewportState>,
) -> Result<(), String> {
    let m = match mode.as_str() {
        "move"   => 0u8,
        "rotate" => 1u8,
        "scale"  => 2u8,
        other => return Err(format!("unknown mode: {other}")),
    };
    viewport_state.gizmo_mode.store(m, std::sync::atomic::Ordering::Relaxed);
    Ok(())
}
```

- [ ] **Step 6: Register all new gizmo commands in `lib.rs`**

In `generate_handler!`:
```rust
bridge::gizmo_commands::gizmo_hit_test,
bridge::gizmo_commands::gizmo_drag,
bridge::gizmo_commands::gizmo_drag_end,
bridge::gizmo_commands::set_gizmo_mode,
commands::scene_undo,
commands::scene_redo,
```

- [ ] **Step 7: Run all tests**

```bash
cargo test -p silmaril-editor
```

Expected: all pass.

- [ ] **Step 8: Commit**

```bash
git add engine/editor/src-tauri/bridge/gizmo_commands.rs engine/editor/src-tauri/lib.rs
git commit -m "feat(editor): gizmo_drag rotate/scale + gizmo_drag_end commits to SceneUndoStack"
```

---

### Task 19: Frontend gizmo mouse handlers

**Files:**
- Modify: `engine/editor/src/lib/components/ViewportOverlay.svelte`

- [ ] **Step 1: Add gizmo mouse handlers to `ViewportOverlay.svelte`**

The overlay already captures mouse events for camera orbit. Add gizmo mode alongside:

```typescript
import { invoke } from '@tauri-apps/api/core';

let isDraggingGizmo = false;

async function onMouseDown(e: MouseEvent) {
    if (!isTauri) return;
    const hit = await invoke<{ axis: string; mode: string } | null>(
        'gizmo_hit_test',
        { viewportId: props.viewportId, screenX: e.clientX, screenY: e.clientY }
    );
    if (hit) {
        isDraggingGizmo = true;
        e.stopPropagation(); // don't start camera orbit
    }
    // else: fall through to existing camera orbit handler
}

async function onMouseMove(e: MouseEvent) {
    if (isDraggingGizmo) {
        await invoke('gizmo_drag', {
            viewportId: props.viewportId,
            screenX: e.clientX,
            screenY: e.clientY,
        });
    }
}

async function onMouseUp(e: MouseEvent) {
    if (isDraggingGizmo) {
        isDraggingGizmo = false;
        await invoke('gizmo_drag_end', { viewportId: props.viewportId });
    }
}
```

Wire these to the overlay div's `onmousedown`, `onmousemove`, `onmouseup` events.

- [ ] **Step 2: Add `W`/`E`/`R` key handlers for gizmo mode switching**

In the viewport's keydown handler:
```typescript
if (e.key === 'w') invoke('set_gizmo_mode', { mode: 'move' });
if (e.key === 'e') invoke('set_gizmo_mode', { mode: 'rotate' });
if (e.key === 'r') invoke('set_gizmo_mode', { mode: 'scale' });
```

- [ ] **Step 3: Manual end-to-end test**

Start `cargo tauri dev`:
1. Create entity via Hierarchy panel `+` button
2. Entity crosshair appears in viewport at (0,0,0)
3. Press `W` → move gizmo arrows visible on selected entity
4. Click + drag X axis → entity moves in X; inspector shows updated position
5. Press Ctrl+Z → entity returns to original position
6. Press `E` → rotate rings; drag → entity rotates
7. Press `R` → scale handles; drag → entity scales

- [ ] **Step 4: Commit**

```bash
git add engine/editor/src/lib/components/ViewportOverlay.svelte
git commit -m "feat(editor): gizmo mouse handlers in ViewportOverlay — hit test, drag, drag end, mode keys"
```

---

### Task 20: Push to remote

- [ ] **Step 1: Run full test suite**

```bash
cargo test -p engine-render-context
cargo test -p engine-renderer
cargo test -p silmaril-editor
```

Expected: all pass.

- [ ] **Step 2: Push**

```bash
git push
```
