# ECS Rendering + Interactive Gizmos — Design Spec

**Date:** 2026-03-21
**Scope:** Refactor renderer into `engine/render-context` + `engine/renderer`, wire ECS World into Tauri, render entity gizmos in the viewport, interactive transform gizmo (move/rotate/scale) with undo/redo.

---

## 1. Goals

- Entities are visible in the editor viewport as axis crosshairs (no mesh rendering yet)
- Selected entity shows a full interactive transform gizmo (move/rotate/scale)
- Gizmo drag operations are undoable via Ctrl+Z
- Rust ECS world is the single source of truth; frontend mirrors it via Tauri events
- Renderer codebase has zero Vulkan duplication between editor and game paths

## 2. Non-Goals (deferred)

- Actual mesh rendering in the viewport (entities render as gizmos only for now)
- Entity picking by clicking on the entity body (only gizmo handles are hit-testable)
- Multi-window viewport panels (single viewport only in this phase)

---

## 3. Crate Structure

### 3.1 New crate: `engine/render-context`

All raw Vulkan plumbing with no game or editor logic:

```
RenderContext     — VkInstance, physical/logical device, queues, gpu-allocator
Surface           — VkSurfaceKHR wrapper
  ├── Surface::from_raw_handle(hwnd: isize)  ← editor: external HWND as isize (Windows only, #[cfg(windows)])
  └── Surface::from_window(window)           ← game: creates OS window internally
Swapchain         — images, views, extent, recreation
DepthBuffer       — depth image + view
RenderPass        — single pass, color + depth attachments
Framebuffers      — one per swapchain image
CommandPool / CommandBuffers
FrameSync         — per-frame fence + semaphores (2 frames in flight)
GpuBuffer / VertexBuffer / IndexBuffer / GpuMesh
GraphicsPipeline + PipelineBuilder
ShaderModule
```

Moved from `engine/renderer` with no behavioral change. The crate has no knowledge of ECS, assets, or the editor.

### 3.2 `engine/renderer` (refactored)

Depends on `engine/render-context`. Adds ECS mesh logic and the `FrameRecorder` overlay hook.

**HWND API:** `from_raw_handle` accepts `isize` (not the `windows` crate `HWND`) to keep the public API platform-agnostic. The `#[cfg(windows)]` gate lives inside the implementation, consistent with how `Surface::from_raw_hwnd` works today. This satisfies CLAUDE.md rule 3 (no `#[cfg]` in business logic signatures).

```rust
pub struct Renderer { /* render-context types + mesh_pipeline + gpu_cache */ }

pub struct ViewportDescriptor {
    pub bounds: Rect,   // sub-rectangle within the swapchain surface
    pub view: Mat4,
    pub proj: Mat4,
}

/// Returned by begin_frame(). Exposes command buffer for overlay injection.
///
/// # Safety contract
/// - The render pass is begun inside `begin_frame` and remains open for the
///   entire lifetime of `FrameRecorder`. Callers MUST NOT call
///   `cmd_end_render_pass` or `end_command_buffer` directly.
/// - `end_frame` calls `cmd_end_render_pass` then `end_command_buffer` before
///   consuming the recorder.
/// - `FrameRecorder` is NOT `Send`. It must not be stored across frame boundaries
///   or outlive the call to `end_frame`. `end_frame` consumes it by value to
///   enforce this at the type level.
/// - With 2 frames in flight, `begin_frame` waits on the per-frame fence before
///   returning, guaranteeing the GPU has finished with this command buffer.
pub struct FrameRecorder {
    pub command_buffer: vk::CommandBuffer,  // record overlay draw calls here
    image_index: u32,                       // private, used by end_frame
    _not_send: PhantomData<*mut ()>,        // vk::CommandBuffer is Send by itself; this makes FrameRecorder !Send on stable Rust
}

impl Renderer {
    /// Editor constructor: accepts an existing OS window handle as isize.
    /// The caller (native_viewport.rs) obtains this from `WebviewWindow::hwnd().0`.
    #[cfg(windows)]
    pub fn from_raw_handle(hwnd: isize, width: u32, height: u32) -> Result<Self, RendererError>;

    /// Game constructor: creates a new OS window.
    pub fn new(config: WindowConfig) -> Result<Self, RendererError>;

    pub fn notify_resize(&mut self, width: u32, height: u32);

    /// Returns None if the swapchain is being rebuilt (caller should skip the frame).
    pub fn begin_frame(&mut self) -> Option<FrameRecorder>;

    /// Draws ECS entities with mesh components. Pass `None` for assets when mesh
    /// rendering is deferred (gizmo-only phase) — the method is a no-op in that case.
    pub fn render_meshes(
        &self,
        recorder: &FrameRecorder,
        world: &World,
        assets: Option<&AssetManager>,
        viewports: &[ViewportDescriptor],
    );

    /// Ends the render pass, submits the command buffer, and presents.
    /// Consumes `recorder` to enforce the single-frame lifetime invariant.
    pub fn end_frame(&mut self, recorder: FrameRecorder);
}
```

**Breaking change to existing `render_meshes`:** The current signature is
`render_meshes(&mut self, world: &World, assets: &AssetManager) -> Result<(), RendererError>`.
The new signature changes `&mut self` → `&self`, adds `recorder` and `viewports` parameters,
wraps `assets` in `Option`, and removes the `Result` (errors handled via tracing + skipped frames).
The existing renderer tests that call `render_meshes` must be updated as part of this task.

### 3.3 `native_viewport.rs` (refactored)

```rust
struct ViewportRenderer {
    renderer: engine_renderer::Renderer,   // ECS mesh rendering + frame management
    grid_pipeline: GridPipeline,           // editor overlay (logic unchanged)
    gizmo_pipeline: GizmoPipeline,         // new
}

impl ViewportRenderer {
    fn render_frame(
        &mut self,
        instances: &[(ViewportBounds, OrbitCamera, bool, bool)],
        world: &RwLockReadGuard<World>,
        selected_entity: Option<EntityId>,
        gizmo_mode: GizmoMode,
    ) {
        let Some(recorder) = self.renderer.begin_frame() else { return; };
        // render pass is now open and remains open until end_frame

        let vp_descs = instances.iter().map(|(b, cam, _, ortho)| ViewportDescriptor {
            bounds: *b,
            view: cam.view_matrix(),
            proj: cam.projection(*ortho),
        }).collect::<Vec<_>>();

        // assets: None = mesh rendering deferred; method is a no-op
        self.renderer.render_meshes(&recorder, world, None, &vp_descs);

        for (bounds, cam, grid_visible, is_ortho) in instances {
            if *grid_visible {
                self.grid_pipeline.record(&recorder.command_buffer, bounds, cam, *is_ortho);
            }
        }

        self.gizmo_pipeline.record(
            &recorder.command_buffer,
            world,
            instances,
            selected_entity,
            gizmo_mode,
        );

        self.renderer.end_frame(recorder);
        // render pass ended and command buffer submitted inside end_frame
    }
}
```

---

## 4. ECS World in Tauri

### 4.1 Managed state

```rust
pub struct SceneWorldState(pub Arc<RwLock<engine_core::World>>);
```

Added to `tauri::Builder::manage()`. `RwLock` chosen because the render thread holds a read lock at 60fps while IPC writes are infrequent.

### 4.2 IPC commands that write to the world

| Command | World mutation |
|---------|---------------|
| `create_entity` | `world.spawn()` + add `Transform::default()` |
| `delete_entity` | `world.despawn(id)` |
| `set_component_field` (transform fields) | `world.write().get_mut::<Transform>(id)` → set field → emit event |
| `gizmo_drag_end` | Reads final transform → pushes `SceneAction::SetTransform` to `SceneUndoStack` |

After every write, Rust emits a Tauri event:

```
entity-created              { id: u64, name: String }
entity-deleted              { id: u64 }
entity-transform-changed    { id: u64, position: [f32;3], rotation: [f32;4], scale: [f32;3] }
```

Frontend listens to these events and updates its scene state — no polling.

### 4.3 Render thread access

The render thread (already running at 60fps) holds `Arc<RwLock<World>>`. Each frame:

```rust
let world = scene_world.read();
renderer.render_meshes(&recorder, &world, None, &vp_descs);
gizmo_pipeline.record(&recorder.command_buffer, &world, ...);
drop(world); // release read lock before end_frame
renderer.end_frame(recorder);
```

---

## 5. Gizmo Rendering

### 5.1 Visual tiers

- **All entities (unselected):** 6-vertex axis crosshair at world position (X=red, Y=green, Z=blue, length ~0.3m)
- **Selected entity:** full transform gizmo — move arrows, rotate rings, scale handles in XYZ colors

### 5.2 Gizmo pipeline

- Separate `VkPipeline` and `VkPipelineLayout` from both the mesh pipeline and grid pipeline
- Vertex shader: vertex position + push constants `(mvp: mat4, color: vec4, scale: f32)` = 84 bytes total
- Fragment shader: flat color, no lighting
- **Depth test: disabled** — gizmos always render on top
- Geometry generated procedurally at startup: cylinder+cone (move), circle line strip (rotate), cylinder+cube (scale). Uploaded as static buffers, reused every frame.

**Push constant budget note:** The grid pipeline uses 80 bytes; the gizmo pipeline uses 84 bytes. Both are within the Vulkan-guaranteed minimum of 128 bytes. They use separate `VkPipelineLayout` instances so their budgets do not share.

### 5.3 Constant screen-space size

```glsl
// vertex shader
float dist = max(length(camera_pos_world - gizmo_origin_world), 0.1); // clamp prevents degenerate case
vec3 offset = vertex_local_pos * dist * 0.15;
gl_Position = view_proj * vec4(gizmo_origin_world + offset, 1.0);
```

The `max(..., 0.1)` clamp prevents the gizmo from collapsing to a point when the camera is coincident with the entity. The `0.15` scale factor is a tunable constant.

### 5.4 Draw cost

- N entities × 1 draw call (crosshair, 6 verts each) — negligible
- Selected entity: ~10 draw calls for full gizmo handles

---

## 6. Gizmo Interaction

### 6.1 IPC surface (JS → Rust)

```typescript
gizmo_hit_test(viewport_id: string, screen_x: number, screen_y: number)
  → { axis: "x"|"y"|"z"|"xy"|"xz"|"yz", mode: "move"|"rotate"|"scale" } | null

gizmo_drag(viewport_id: string, screen_x: number, screen_y: number)
  → void  // world updated + entity-transform-changed event emitted

gizmo_drag_end(viewport_id: string)
  → void  // EditorAction::SetTransform committed to CommandProcessor
```

JS captures `mousedown`/`mousemove`/`mouseup` on the viewport overlay — same pattern as the existing camera orbit handler in `ViewportOverlay.svelte`.

### 6.2 DragState storage and threading

`DragState` is stored in `NativeViewportState` (the Tauri managed state struct already used by all viewport IPC commands), behind its existing `Mutex`. This is separate from the per-viewport `instances` mutex inside the render thread.

```rust
// in NativeViewportState (bridge/commands.rs)
pub struct NativeViewportState {
    pub viewport: Mutex<NativeViewport>,
    pub drag_state: Mutex<Option<DragState>>,  // new field
}

pub struct DragState {
    pub entity_id: EntityId,
    pub viewport_id: String,
    pub axis: GizmoAxis,
    pub mode: GizmoMode,
    pub transform_before: Transform,    // snapshot at drag start, used for undo
    pub last_screen_pos: (f32, f32),    // for delta computation in gizmo_drag
    pub camera_snapshot: CameraMatrices, // view + proj + camera world pos at drag start
}

pub struct CameraMatrices {
    pub view: Mat4,
    pub proj: Mat4,
    pub camera_pos_world: Vec3,
}
```

`gizmo_hit_test` reads the camera matrices from the `ViewportInstance` (via the instances lock, briefly), stores a snapshot in `DragState`, then releases the instances lock. Subsequent `gizmo_drag` calls use the snapshotted matrices — no further instances lock contention during drag.

### 6.3 Raycasting (Rust)

`gizmo_hit_test`:
1. Lock `instances` briefly → snapshot camera view+proj+pos for the given `viewport_id` → release lock
2. Unproject `(screen_x, screen_y)` → world-space ray
3. Test ray against each gizmo axis handle (approximated as a capsule) for the selected entity
4. Return closest hit axis+mode, or `null` if no hit
5. On hit: populate `DragState` with entity, axis, mode, `transform_before`, `camera_snapshot`, `last_screen_pos`

### 6.4 Drag accumulation (Rust)

Each `gizmo_drag(screen_x, screen_y)` call:
1. Lock `drag_state` → compute `(dx, dy)` from `last_screen_pos` → update `last_screen_pos`
2. **Move:** project screen delta onto axis vector using `camera_snapshot` → translate entity in world
3. **Rotate:** accumulate screen-space angle delta → rotate entity around axis
4. **Scale:** drag distance → scale entity along axis (or uniform if center handle)
5. Write new `Transform` to `scene_world` (write lock, brief)
6. Emit `entity-transform-changed` → frontend mirrors live

`gizmo_drag_end`:
1. Read final `Transform` from world
2. Push `EditorAction::SetTransform { entity_id, before: drag_state.transform_before, after: final }` to `CommandProcessor`
3. Clear `drag_state`

### 6.5 Undo/redo integration

**Separate in-memory undo stack for live-world operations.**

`CommandProcessor` in `engine/ops` operates exclusively on `TemplateState` (YAML files on disk) and has no access to the runtime ECS world. Adding ECS world access to `CommandProcessor` would violate its single responsibility and require threading `Arc<RwLock<World>>` through every call site. Instead, a parallel in-memory `SceneUndoStack` is introduced in Tauri managed state:

```rust
// engine/editor/src-tauri/state/scene_undo.rs
pub struct SceneUndoStack {
    undo: Vec<SceneAction>,
    redo: Vec<SceneAction>,
}

pub enum SceneAction {
    SetTransform {
        entity_id: u64,
        before: SerializedTransform,  // { position: [f32;3], rotation: [f32;4], scale: [f32;3] }
        after: SerializedTransform,
    },
    // future: CreateEntity, DeleteEntity, etc.
}
```

`SceneUndoStack` is **in-memory only** — it is never written to disk and is cleared on project close/open. This avoids the `.undo.json` mixed-history problem entirely.

**Two new IPC commands:**

```
scene_undo()  → pops from SceneUndoStack.undo, applies inverse to world, emits entity-transform-changed
scene_redo()  → pops from SceneUndoStack.redo, re-applies to world, emits entity-transform-changed
```

**Sources that push to `SceneUndoStack`:**
- `gizmo_drag_end` — one `SetTransform` action per completed drag
- `set_component_field` for transform fields — one `SetTransform` action per field edit

**Frontend Ctrl+Z routing:**

The existing `template_undo`/`template_redo` binding stays for template operations. The viewport panel uses `scene_undo`/`scene_redo` when focused. The frontend `undo-history.ts` store gains a `sceneCanUndo`/`sceneCanRedo` flag returned by `scene_undo`/`scene_redo` alongside the existing template undo state. TitleBar undo/redo buttons call `scene_undo` when a viewport panel is active, `template_undo` otherwise.

### 6.6 Gizmo mode switching

Frontend sends `set_gizmo_mode("move"|"rotate"|"scale")` IPC — or triggered by `W`/`E`/`R` keybinds already wired in the editor. Stores the mode in `NativeViewportState`. The render loop reads this field to select which gizmo geometry to draw.

---

## 7. File Summary

**New files:**
- `engine/render-context/` — new crate (Vulkan plumbing moved from `engine/renderer`)
- `engine/editor/src-tauri/bridge/gizmo_commands.rs` — `gizmo_hit_test`, `gizmo_drag`, `gizmo_drag_end`, `set_gizmo_mode`
- `engine/editor/src-tauri/viewport/gizmo_pipeline.rs` — `GizmoPipeline` struct + procedural geometry generation
- `engine/editor/src-tauri/state/scene_undo.rs` — `SceneUndoStack`, `SceneAction`, `SerializedTransform`

**Modified files:**
- `engine/renderer/src/` — depends on `render-context`; `FrameRecorder`, `from_raw_handle`, multi-viewport `render_meshes` with `Option<&AssetManager>`; existing renderer tests updated for new signature
- `engine/editor/src-tauri/viewport/native_viewport.rs` — `ViewportRenderer` wraps `engine_renderer::Renderer`; duplicated Vulkan setup removed
- `engine/editor/src-tauri/lib.rs` — manage `SceneWorldState`; register gizmo commands
- `engine/editor/src-tauri/bridge/commands.rs` — `create_entity`, `delete_entity`, `set_component_field` write to world; `NativeViewportState` gains `drag_state` field
- `engine/ops/src/` — `EditorAction::SetTransform` variant + apply/apply_inverse impl
- `engine/editor/src/lib/scene/state.ts` — listen to Tauri events for entity-transform-changed
- `engine/editor/src/lib/components/ViewportOverlay.svelte` — gizmo mousedown/mousemove/mouseup handlers
- `engine/editor/src/lib/stores/undo-history.ts` — add `sceneCanUndo`/`sceneCanRedo` state; route Ctrl+Z to `scene_undo` when viewport is focused

---

## 8. Testing

- Unit: `EditorAction::SetTransform` apply + apply_inverse roundtrip in `engine/ops` tests
- Unit: `gizmo_hit_test` ray-capsule intersection (parametric test cases, no GPU needed)
- Unit: `Surface::from_raw_handle` + `Renderer::from_raw_handle` Windows integration test (`#[cfg(windows)]`)
- Updated: existing `engine/renderer` tests — update calls to `render_meshes` for new signature
- Manual: drag each axis in each mode; Ctrl+Z restores original transform
- Manual: multiple entities in viewport; correct crosshairs at each position
- Manual: camera coincident with entity — gizmo stays visible (min-distance clamp works)
