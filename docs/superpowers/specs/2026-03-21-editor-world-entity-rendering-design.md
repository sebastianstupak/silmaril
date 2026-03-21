# EditorWorld & Entity Rendering — Design Spec

**Date:** 2026-03-21
**Scope:** Live ECS World in the editor viewport — commands flow through CommandProcessor → TemplateState → YAML → EditorWorld → render thread → entities visible as shaded cubes in real time. Supports UI, CLI, and AI agent workflows identically.

---

## 1. Goal

When any actor (editor UI, CLI, AI agent) issues a `TemplateCommand`, the change is persisted to YAML **and** immediately visible in the Vulkan viewport — entities appear as shaded cubes, updating in real time after every execute/undo/redo.

At Play time, the live `EditorWorld` is snapshotted and handed to the game loop. No re-parsing, no extra serialization step.

---

## 2. Architecture

```
UI / CLI / AI Agent
      ↓  TemplateCommand (identical path for all three)
CommandProcessor  (engine/ops — unchanged, stays pure)
      ├─→ TemplateState  (in-memory)
      ├─→ YAML file      (persistence)
      └─→ returns CommandResult { new_state }
                ↓
      Tauri IPC handler  (template_commands.rs)
                ↓
      EditorWorld::rebuild_from_template(&new_state)
                ↓
      Arc<RwLock<EditorWorld>>   ← written here
                ↑
      Render thread reads each frame via try_read()
      query::<(&Transform, &MeshRenderer)>()
                ↓
      ViewportRenderer draws shaded cubes
```

`CommandProcessor` has no editor coupling — the rebuild is the IPC handler's responsibility, one line after the command returns.

---

## 3. EditorWorld

**File:** `engine/editor/src-tauri/world/editor_world.rs`

```rust
pub struct EditorWorld {
    world: engine_core::World,
}

impl EditorWorld {
    /// Empty world (no entities).
    pub fn empty() -> Self;

    /// Rebuild from a TemplateState snapshot.
    /// Called after every execute / undo / redo / open.
    pub fn rebuild_from_template(state: &TemplateState) -> Self;

    /// Borrow the inner World (render thread reads).
    pub fn world(&self) -> &engine_core::World;

    /// Consume EditorWorld and return the inner World (play mode — ownership transfer).
    pub fn into_world(self) -> engine_core::World;
}
```

### rebuild_from_template logic

**Component registration:** `engine_core::World::add()` panics if the component type was not previously registered. `rebuild_from_template` must call `world.register::<Transform>()` and `world.register::<MeshRenderer>()` before any `world.add()` call.

For each `TemplateEntity` in `state.entities`:

1. Spawn a new `Entity` in the World.
2. Scan its `components` for `type_name == "Transform"`:
   - If found: deserialize `data` as `engine_core::Transform` (position/rotation/scale from JSON); add to World.
   - If not found: add `Transform::default()` (identity — entity appears at world origin, always visible).
3. Add `MeshRenderer { mesh_id: CUBE_MESH_ID, visible: true }` to every entity.

`CUBE_MESH_ID` is a fixed `u64` constant (`pub const CUBE_MESH_ID: u64 = 0`). `MeshRenderer.mesh_id` is typed `u64` — no `AssetId` involved here. The viewport renderer special-cases `mesh_id == CUBE_MESH_ID` to draw from the pre-uploaded cube buffers rather than a dynamic GPU cache lookup.

### Why rebuild rather than delta-apply

Rebuild is O(n) over entity count. For editor scenes (< 10 000 entities) this is imperceptibly fast (< 1 ms). Delta application is an optimization deferred until profiling proves it necessary.

---

## 4. EditorState — shared state

**File:** `engine/editor/src-tauri/bridge/template_commands.rs`

```rust
pub struct EditorState {
    pub processors: HashMap<PathBuf, CommandProcessor>,
    pub editor_world: Arc<RwLock<EditorWorld>>,   // shared with render thread
    pub selected_entity: Arc<AtomicU64>,           // u64::MAX = no selection
}
```

Each mutating IPC handler rebuilds the EditorWorld after the mutation. The pattern differs per handler because their return types differ:

```rust
// template_execute — CommandResult contains new_state directly
let result = proc.execute(command)?;
*guard.editor_world.write() = EditorWorld::rebuild_from_template(&result.new_state);
Ok(result)

// template_open — returns TemplateState; use proc.state_ref() after load
let proc = CommandProcessor::load(&path)?;
let ts = proc.state_ref().clone();
guard.processors.insert(path.clone(), proc);
*guard.editor_world.write() = EditorWorld::rebuild_from_template(&ts);
Ok(ts)

// template_undo — returns Option<ActionId>; re-read state after mutation
let action_id = proc.undo()?;
let ts = proc.state_ref().clone();
*guard.editor_world.write() = EditorWorld::rebuild_from_template(&ts);
Ok(action_id)

// template_redo — same pattern as undo
let action_id = proc.redo()?;
let ts = proc.state_ref().clone();
*guard.editor_world.write() = EditorWorld::rebuild_from_template(&ts);
Ok(action_id)
```

`template_close` clears the world: `*guard.editor_world.write() = EditorWorld::empty()`.

### Selection sync

A new Tauri command `viewport_set_selected_entity(id: u64)` stores `id` into `selected_entity` via `AtomicU64::store`. The frontend calls this whenever `selectedEntityId` changes in `editor-context.ts`. The render thread reads it with `Ordering::Relaxed` — no lock needed, one atomic load per frame.

---

## 5. Render thread integration

`NativeViewport` receives clones of both Arcs at construction and captures them in the render thread closure.

### Per-frame entity snapshot

```rust
let entity_snapshots: Vec<EntitySnapshot> = {
    match editor_world.try_read() {
        Ok(ew) => {
            let selected = selected_entity.load(Ordering::Relaxed);
            // engine_core query yields (Entity, (&A, &B)) — nested tuple, not flat
            ew.world()
              .query::<(&Transform, &MeshRenderer)>()
              .filter(|(_, (_, mr))| mr.is_visible())
              .map(|(entity, (transform, _))| EntitySnapshot {
                  model_matrix: transform.to_matrix(),  // engine_math::Transform::to_matrix()
                  // entity.id() is u32; cast to u64 safe. Sentinel (-1i64 as u64 = u64::MAX)
                  // can never equal a real entity id (u32::MAX casts to 0x00000000FFFFFFFF).
                  selected: entity.id() as u64 == selected,
              })
              .collect()
        }
        // Write in progress — skip entities this frame; grid still draws
        Err(_) => vec![],
    }
};
```

### Updated NativeViewport and render_loop signatures

`NativeViewport` gains two new fields:
```rust
pub struct NativeViewport {
    // ... existing fields ...
    editor_world: Arc<RwLock<EditorWorld>>,
    selected_entity: Arc<AtomicU64>,
}
```

`NativeViewport::new(hwnd, editor_world: Arc<RwLock<EditorWorld>>, selected_entity: Arc<AtomicU64>) -> Self`

`start_rendering()` passes clones into the render thread:
```rust
let ew = Arc::clone(&self.editor_world);
let sel = Arc::clone(&self.selected_entity);
std::thread::Builder::new()
    .name("viewport-render".into())
    .spawn(move || render_loop(hwnd, should_stop, render_active, instances, ew, sel))
```

`render_loop` signature:
```rust
fn render_loop(
    hwnd: HWND,
    should_stop: Arc<AtomicBool>,
    render_active: Arc<AtomicBool>,
    instances: Arc<Mutex<HashMap<String, ViewportInstance>>>,
    editor_world: Arc<RwLock<EditorWorld>>,
    selected_entity: Arc<AtomicU64>,
)
```

### Updated record_frame / render_frame signatures

Entity snapshots collected in `render_loop` are passed down through the call chain:

```rust
// render_loop calls:
renderer.render_frame(&viewports, &entity_snapshots)

// ViewportRenderer::render_frame:
fn render_frame(
    &mut self,
    viewports: &[(ViewportBounds, OrbitCamera, bool, bool)],
    entity_snapshots: &[EntitySnapshot],
) -> Result<(), String>

// which calls:
self.record_frame(cmd, image_index, viewports, entity_snapshots)

fn record_frame(
    &self,
    cmd: vk::CommandBuffer,
    image_index: usize,
    viewports: &[(ViewportBounds, OrbitCamera, bool, bool)],
    entity_snapshots: &[EntitySnapshot],
) -> Result<(), String>
```

Entity snapshots are drawn once per viewport instance (each instance applies its own camera VP matrix).

`try_read()` ensures the render thread never blocks. Worst case: one frame with no entity cubes while a command is being processed. Grid and camera remain fully responsive.

---

## 6. Entity rendering in ViewportRenderer

### Shaders

Two new GLSL shaders compiled at build time alongside the grid shaders:

**`entity.vert.glsl`**
- Inputs: `layout(location=0) in vec3 position`, `layout(location=1) in vec3 normal`
- Push constants (80 bytes): `mat4 mvp` (bytes 0–63) + `vec4 color` (bytes 64–79)
- Outputs: clip-space position, world normal, color to fragment stage

**`entity.frag.glsl`**
- Lambert diffuse with fixed directional light `normalize(vec3(1, 2, 3))`
- `ambient = 0.25`, `diffuse = max(dot(normal, light), 0.0) * 0.75`
- Output: `vec4(color.rgb * (ambient + diffuse), 1.0)`

### Colors

| State | Color |
|-------|-------|
| Unselected | `vec4(0.55, 0.55, 0.60, 1.0)` — neutral gray-blue |
| Selected | `vec4(0.25, 0.60, 1.0, 1.0)` — accent blue |

### Entity pipeline depth state

The entity pipeline uses:
- `depth_test_enable(true)` with `LESS` compare — entities occlude each other correctly
- `depth_write_enable(true)` — entities write depth so the grid (drawn first, `depth_write_enable(false)`) is correctly overdrawn by entity cubes

Draw order per viewport: grid first, then entities. Grid's transparent areas remain visible through entity-free regions.

### ViewportRenderer additions

```rust
struct ViewportRenderer {
    // ... existing fields ...
    entity_pipeline:        vk::Pipeline,
    entity_pipeline_layout: vk::PipelineLayout,
    cube_vertex_buffer:     GpuBuffer,
    cube_index_buffer:      GpuBuffer,
    cube_index_count:       u32,
    _entity_vert_shader:    ShaderModule,
    _entity_frag_shader:    ShaderModule,
}
```

Cube mesh is built from `engine_assets::MeshData::cube()` and uploaded once during `ViewportRenderer::new()`.

### Draw call (inside record_frame, after grid)

```
for each viewport instance:
    set viewport + scissor (existing)
    draw grid (existing)
    if entity_snapshots not empty:
        bind entity_pipeline
        for each EntitySnapshot:
            push_constants: mvp = camera.view_projection(aspect) * snapshot.model_matrix
                            color = selected ? SELECTED_COLOR : UNSELECTED_COLOR
            draw_indexed(cube_index_count, 1, 0, 0, 0)
```

---

## 7. Play mode

```rust
fn start_play(template_path: &Path, processors: &HashMap<PathBuf, CommandProcessor>) -> PlaySession {
    // Re-parse from TemplateState to build play World (avoids requiring World: Clone)
    let state = processors[template_path].current_state();
    let play_world = EditorWorld::rebuild_from_template(&state).into_world();
    PlaySession::new(play_world) // attach game systems, start loop
}
// On Stop: editor_world Arc is unchanged; render thread resumes immediately
```

**Note on `World::Clone`:** `engine_core::World` contains `Box<dyn ComponentStorage>` trait objects which are not `Clone` without explicit implementation. Rather than requiring `World: Clone`, play mode rebuilds from `TemplateState` (fast: same O(n) rebuild as after a command). `CommandProcessor` exposes `current_state() -> &TemplateState` for this purpose. This avoids coupling `World` to `Clone` semantics.

The editor world is never mutated during play. Stopping play requires no re-parse on the editor side — the editor `Arc<RwLock<EditorWorld>>` was untouched throughout play.

---

## 8. Frontend changes

**`engine/editor/src/lib/api.ts`**
```typescript
export async function viewportSetSelectedEntity(id: number | null): Promise<void>
// Passes id as a number, or -1 for no selection.
// Rust side: #[tauri::command] fn viewport_set_selected_entity(id: i64)
//   -> selected_entity.store(if id < 0 { u64::MAX } else { id as u64 }, Ordering::Relaxed)
// Using i64 avoids JS u64 truncation (Number.MAX_SAFE_INTEGER = 2^53-1 < u64::MAX).
// Sentinel -1 is passed as JS -1, never truncated. u32::MAX entity ids fit in i64 safely.
```

**`engine/editor/src/lib/stores/editor-context.ts`**
- After `setSelectedEntityId(id)`: call `viewportSetSelectedEntity(id)` (fire-and-forget).

---

## 9. File map

### Arc creation discipline

`Arc<RwLock<EditorWorld>>` and `Arc<AtomicU64>` are created **once** in `lib.rs` before either `NativeViewport` or `EditorState` is constructed. Each receives a `Arc::clone`. Neither creates the Arc themselves.

```rust
// lib.rs — app setup
let editor_world = Arc::new(RwLock::new(EditorWorld::empty()));
let selected_entity = Arc::new(AtomicU64::new(u64::MAX));

app_builder
    .manage(Mutex::new(EditorState {
        processors: HashMap::new(),
        editor_world: Arc::clone(&editor_world),
        selected_entity: Arc::clone(&selected_entity),
    }))
    // NativeViewportState constructed with both Arcs:
    .manage(NativeViewportState::new(editor_world, selected_entity))
```

### New files
| File | Purpose |
|------|---------|
| `engine/editor/src-tauri/world/editor_world.rs` | EditorWorld struct + rebuild_from_template |
| `engine/editor/src-tauri/viewport/shaders/entity.vert.glsl` | Entity vertex shader |
| `engine/editor/src-tauri/viewport/shaders/entity.frag.glsl` | Entity fragment shader |

### Modified files
| File | Change |
|------|--------|
| `engine/editor/Cargo.toml` | Add `engine-assets` dependency if not already present (needed for `MeshData::cube()`) |
| `engine/editor/src-tauri/world/mod.rs` | pub mod editor_world; re-export EditorWorld |
| `engine/editor/src-tauri/bridge/template_commands.rs` | EditorState gains editor_world + selected_entity; handlers rebuild after command |
| `engine/editor/src-tauri/bridge/commands.rs` | Add viewport_set_selected_entity command |
| `engine/editor/src-tauri/viewport/native_viewport.rs` | ViewportRenderer gains entity pipeline + cube buffers; render loop snapshots EditorWorld |
| `engine/editor/src-tauri/lib.rs` | Create Arc<RwLock<EditorWorld>> + Arc<AtomicU64>; wire into NativeViewport + EditorState |
| `engine/editor/src/lib/api.ts` | viewportSetSelectedEntity wrapper |
| `engine/editor/src/lib/stores/editor-context.ts` | Call viewportSetSelectedEntity on selection change |

---

## 10. Testing

### Unit tests (inline `#[cfg(test)]` in `editor_world.rs`, per codebase convention)
- `rebuild_empty_template` → world has 0 entities
- `rebuild_single_entity_no_transform` → world has 1 entity with identity Transform
- `rebuild_single_entity_with_transform` → world has 1 entity with correct position/rotation/scale
- `rebuild_multiple_entities` → world entity count matches template entity count
- `rebuild_replaces_previous_world` → rebuild twice, second state is authoritative

### Integration tests
- Execute `CreateEntity` command → `editor_world` has 1 entity
- Execute `CreateEntity` twice → 2 entities
- Undo → 1 entity
- Undo again → 0 entities
- `template_close` → 0 entities

### TypeScript unit tests
- `viewportSetSelectedEntity(42)` → invokes correct Tauri command with correct args
- `viewportSetSelectedEntity(null)` → invokes with sentinel value
