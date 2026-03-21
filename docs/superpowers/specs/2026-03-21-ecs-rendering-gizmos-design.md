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
  ├── Surface::from_hwnd(hwnd)       ← editor: external OS handle
  └── Surface::from_window(window)   ← game: creates OS window internally
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

Depends on `engine/render-context`. Adds ECS mesh logic and the `FrameRecorder` overlay hook:

```rust
pub struct Renderer { /* render-context types + mesh_pipeline + gpu_cache */ }

pub struct ViewportDescriptor {
    pub bounds: Rect,       // sub-rectangle within the swapchain surface
    pub view: Mat4,
    pub proj: Mat4,
}

/// Returned by begin_frame(). Exposes command buffer for overlay injection.
pub struct FrameRecorder {
    pub command_buffer: vk::CommandBuffer,  // write overlays directly here
    image_index: u32,                       // private
}

impl Renderer {
    pub fn from_hwnd(hwnd: HWND, width: u32, height: u32) -> Result<Self>;  // editor
    pub fn new(config: WindowConfig) -> Result<Self>;                        // game

    pub fn notify_resize(&mut self, width: u32, height: u32);
    pub fn begin_frame(&mut self) -> Option<FrameRecorder>;   // None = swapchain rebuilding
    pub fn render_meshes(
        &self,
        recorder: &FrameRecorder,
        world: &World,
        assets: &AssetManager,
        viewports: &[ViewportDescriptor],
    );
    pub fn end_frame(&mut self, recorder: FrameRecorder);
}
```

The game binary calls `new()`. The editor calls `from_hwnd()`. Both use the same `begin_frame → render_meshes → end_frame` loop.

### 3.3 `native_viewport.rs` (refactored)

```rust
struct ViewportRenderer {
    renderer: engine_renderer::Renderer,   // ECS mesh rendering
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

        let vp_descs = instances.iter().map(|(b, cam, _, ortho)| ViewportDescriptor {
            bounds: *b,
            view: cam.view_matrix(),
            proj: cam.projection(*ortho),
        }).collect::<Vec<_>>();

        self.renderer.render_meshes(&recorder, world, &NO_ASSETS, &vp_descs);

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
| `set_component_field` (transform fields) | `world.get_mut::<Transform>(id)` → set field |
| `gizmo_drag_end` | Commits `TransformAction` to `CommandProcessor` |

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
renderer.render_meshes(&recorder, &world, &assets, &vp_descs);
gizmo_pipeline.record(&cmd, &world, ...);
```

---

## 5. Gizmo Rendering

### 5.1 Visual tiers

- **All entities (unselected):** 6-vertex axis crosshair at world position (X=red, Y=green, Z=blue, length ~0.3m)
- **Selected entity:** full transform gizmo — move arrows, rotate rings, scale handles in XYZ colors

### 5.2 Gizmo pipeline

- Separate `VkPipeline` from the mesh pipeline
- Vertex shader: vertex position + push constants `(mvp: mat4, color: vec4, scale: f32)`
- Fragment shader: flat color, no lighting
- **Depth test: disabled** — gizmos always render on top
- Geometry generated procedurally at startup: cylinder+cone (move), circle strip (rotate), cylinder+cube (scale). Uploaded as static buffers, reused every frame.

### 5.3 Constant screen-space size

```glsl
// vertex shader
float dist = length(camera_pos_world - gizmo_origin_world);
vec3 offset = vertex_local_pos * dist * 0.15;
gl_Position = view_proj * vec4(gizmo_origin_world + offset, 1.0);
```

Keeps gizmos the same apparent size regardless of camera distance.

### 5.4 Draw cost

- N entities × 1 draw call (crosshair, 6 verts each) — negligible
- Selected entity: ~10 draw calls for full gizmo handles

---

## 6. Gizmo Interaction

### 6.1 IPC surface (JS → Rust)

```typescript
// Rust side: three new Tauri commands
gizmo_hit_test(viewport_id: string, screen_x: f32, screen_y: f32)
  → { hit: bool, axis: "x"|"y"|"z"|"xy"|"xz"|"yz", mode: "move"|"rotate"|"scale" } | null

gizmo_drag(viewport_id: string, screen_x: f32, screen_y: f32)
  → void  (world updated + entity-transform-changed event emitted)

gizmo_drag_end(viewport_id: string)
  → void  (TransformAction committed to CommandProcessor)
```

JS captures `mousedown`/`mousemove`/`mouseup` on the viewport overlay — same pattern as the existing camera orbit handler.

### 6.2 Raycasting (Rust)

`gizmo_hit_test`:
1. Unproject screen position → world-space ray using current camera matrices
2. Test ray against each gizmo axis handle approximated as a capsule
3. Returns closest hit or null

### 6.3 Drag accumulation (Rust)

Drag state held in `NativeViewport` (not ECS):

```rust
struct DragState {
    entity_id: EntityId,
    axis: GizmoAxis,
    mode: GizmoMode,
    transform_before: Transform,  // snapshot at drag start, used for undo
}
```

- **Move:** project screen delta onto axis vector in world space → translate entity
- **Rotate:** accumulate screen-space angle delta → rotate around axis
- **Scale:** drag distance → scale along axis (or uniform if center handle)

Each `gizmo_drag` call:
1. Updates `Transform` in the ECS world directly (no undo entry — avoids spamming the undo stack)
2. Emits `entity-transform-changed` → frontend mirrors live

`gizmo_drag_end`:
1. Reads final `Transform` from world
2. Pushes `TransformAction { entity_id, before: drag_state.transform_before, after: final }` onto `CommandProcessor`
3. Clears `DragState`

### 6.4 Gizmo mode switching

Frontend sends `set_gizmo_mode("move"|"rotate"|"scale")` IPC — or triggered by `W`/`E`/`R` keybinds already wired in the editor. Updates a field in `NativeViewportState`, no world mutation. The render loop reads this field to know which gizmo geometry to draw.

---

## 7. Undo/Redo Integration

### 7.1 New action type

```rust
pub struct TransformAction {
    pub entity_id: EntityId,
    pub before: Transform,
    pub after: Transform,
}

impl Action for TransformAction {
    fn execute(&self, world: &mut World) { set_transform(world, self.entity_id, self.after); }
    fn undo(&self, world: &mut World)    { set_transform(world, self.entity_id, self.before); }
}
```

### 7.2 Sources of `TransformAction`

- **Gizmo drag:** pushed by `gizmo_drag_end` (one action per completed drag)
- **Inspector field edit:** `set_component_field` for transform fields pushes a `TransformAction` wrapping the before/after values

### 7.3 Undo flow

Ctrl+Z → existing `template_undo` IPC → `CommandProcessor::undo()` → `TransformAction::undo()` → world updated → `entity-transform-changed` event → frontend mirrors.

No changes to the undo-history store, TitleBar, or keybind wiring.

---

## 8. File Summary

**New files:**
- `engine/render-context/` — new crate (code moved from `engine/renderer`)
- `engine/editor/src-tauri/bridge/gizmo_commands.rs` — gizmo IPC commands
- `engine/editor/src-tauri/viewport/gizmo_pipeline.rs` — GizmoPipeline struct
- `engine/editor/src-tauri/state/scene_world.rs` — SceneWorldState wrapper

**Modified files:**
- `engine/renderer/src/` — refactored to depend on render-context; add `FrameRecorder`, `from_hwnd`, multi-viewport `render_meshes`
- `engine/editor/src-tauri/viewport/native_viewport.rs` — use `engine_renderer::Renderer`; remove duplicated Vulkan setup
- `engine/editor/src-tauri/lib.rs` — manage `SceneWorldState`; register gizmo commands
- `engine/editor/src-tauri/bridge/commands.rs` — `create_entity`, `delete_entity`, `set_component_field` write to world
- `engine/editor/src/lib/scene/state.ts` — listen to Tauri events instead of managing own state
- `engine/editor/src/lib/components/ViewportOverlay.svelte` — add gizmo mousedown/mousemove/mouseup handlers

---

## 9. Testing

- Unit: `TransformAction` execute/undo roundtrip
- Unit: `gizmo_hit_test` ray-capsule intersection (parametric test cases)
- Unit: `Surface::from_hwnd` + `Renderer::from_hwnd` integration (Windows only, behind `#[cfg(windows)]`)
- Manual: drag each axis in each mode; Ctrl+Z restores original transform
- Manual: multiple entities in viewport; correct crosshairs at each position
- Existing tests remain green (renderer crate tests pass after move to render-context)
