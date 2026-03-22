# Hierarchy Panel Improvements Design

> **Status:** Approved

---

## Goal

Three focused improvements to the hierarchy panel:
1. **Rename on name only** — double-click on the entity name text starts rename; double-click on
   the rest of the row triggers camera fly-to.
2. **Smooth camera fly-to** — double-clicking outside the name text smoothly animates the camera
   to orbit the entity.
3. **Drag-to-reparent** — dragging one entity row onto another reparents it, persisted to the ECS
   world.

## Architecture

Frontend: `engine/editor/src/lib/components/HierarchyPanel.svelte` (Svelte 5 runes).
Backend: `engine/editor/src-tauri/bridge/commands.rs` (new IPCs), `engine/editor/src-tauri/viewport/native_viewport.rs`
(camera animation), `engine/core/src/` (new `Parent` component).

**Tech stack:** Svelte 5, Rust, Tauri IPC, per-frame render loop.

---

## Section 1 — Rename Only on Name Text

**Current behaviour:** `ondblclick` on `<li class="entity-row">` fires `startRename()` for any
double-click anywhere on the row.

**Target:** Double-clicking `<span class="entity-name">` starts rename. Double-clicking elsewhere
on the row triggers camera fly-to (Section 2).

### 1a — Move dblclick handler to name span

Remove `ondblclick` from `<li class="entity-row">`.
Add it to `<span class="entity-name">`:

```svelte
<span
  class="entity-name"
  ondblclick={(e) => { e.stopPropagation(); startRename(entity.id, entity.name); }}
>
  {entity.name}
</span>
```

`stopPropagation()` prevents the event from bubbling to the row-level handler.

### 1b — Row-level dblclick → camera fly-to

```svelte
<li
  class="entity-row"
  ondblclick={() => focusEntity(entity.id)}
  ...
>
```

Event order: name span fires → `stopPropagation()` → row handler never fires.
Non-name areas don't stop propagation → row handler fires → fly-to.
`focusEntity()` is defined in Section 2d.

---

## Section 2 — Smooth Camera Fly-To

### 2a — Camera animation state on `ViewportInstance`

Add a sibling struct and a new field to `ViewportInstance`:

```rust
struct CameraAnimation {
    target_pos:  glam::Vec3,
    target_dist: f32,         // always 5.0 for hierarchy focus
}
```

```rust
// In ViewportInstance:
camera_anim: Option<CameraAnimation>,
```

### 2b — Per-frame lerp inside the lock

The render loop snapshots instance data by cloning under the instances lock.
The animation step must run **inside the same lock block**, mutating `inst.camera` before the
snapshot is taken:

```rust
let viewports: Vec<(ViewportBounds, OrbitCamera, bool, bool)> = {
    let mut lock = instances.lock().unwrap();   // mut needed to lerp in-place

    for inst in lock.values_mut().filter(|i| i.visible) {
        if let Some(ref anim) = inst.camera_anim {
            // Skip animation if a gizmo drag is active
            let dragging = drag_state.lock().ok()
                .map_or(false, |g| g.is_some());
            if !dragging {
                const LERP: f32 = 0.12;
                const EPS:  f32 = 0.001;
                inst.camera.target +=
                    (anim.target_pos - inst.camera.target) * LERP;
                inst.camera.distance +=
                    (anim.target_dist - inst.camera.distance) * LERP;
                let done =
                    (inst.camera.target - anim.target_pos).length() < EPS
                    && (inst.camera.distance - anim.target_dist).abs() < EPS;
                if done {
                    inst.camera.target   = anim.target_pos;
                    inst.camera.distance = anim.target_dist;
                    inst.camera_anim = None;
                }
            }
        }
    }

    lock.values()
        .filter(|i| i.visible)
        .map(|i| (i.bounds, i.camera.clone(), i.grid_visible, i.is_ortho))
        .collect()
};
```

**Drag guard threading:** `NativeViewportState.drag_state` is currently a plain `Mutex` — change
it to `Arc<Mutex<Option<DragState>>>`. Then thread it through three coordinated steps:

1. `create_native_viewport()` in `commands.rs` — clone the Arc and pass it as a constructor arg:
   ```rust
   let drag_state = viewport_state.drag_state.clone(); // Arc clone
   NativeViewport::new(hwnd, width, height, selected_entity_id, gizmo_mode, drag_state, …)
   ```
2. `NativeViewport::new()` signature gains `drag_state: Arc<Mutex<Option<DragState>>>`,
   stored as a field: `self.drag_state = drag_state;`
3. `start_rendering()` clones it before the thread spawn:
   ```rust
   let drag_state = self.drag_state.clone(); // into render_loop closure
   ```
`drag_state` is then available inside the render loop for the animation guard above.

Pitch and yaw are **not changed** — camera orientation is fully preserved.

### 2c — `start_camera_animation` method

On `NativeViewport` (uses `&self` + interior mutex, matching existing camera methods):

```rust
pub fn start_camera_animation(&self, id: &str, target_pos: glam::Vec3, target_dist: f32) {
    if let Ok(mut instances) = self.instances.lock() {
        if let Some(inst) = instances.get_mut(id) {
            inst.camera_anim = Some(CameraAnimation { target_pos, target_dist });
        }
    }
}
```

### 2d — New IPC: `focus_entity_animated`

```rust
#[tauri::command]
pub fn focus_entity_animated(
    entity_id: u64,
    viewport_state: tauri::State<'_, NativeViewportState>,
    world_state: tauri::State<'_, crate::state::SceneWorldState>,
) -> Result<(), String> {
    debug_assert!(entity_id <= u32::MAX as u64, "entity_id overflows u32");
    let entity = engine_core::Entity::new(entity_id as u32, 0);
    let pos = {
        let world = world_state.inner().0.read().map_err(|e| e.to_string())?;
        world
            .get::<engine_core::Transform>(entity)
            .map(|t| glam::Vec3::new(t.position.x, t.position.y, t.position.z))
            .unwrap_or(glam::Vec3::ZERO)
    };

    let registry = viewport_state.registry.lock().map_err(|e| e.to_string())?;
    for id in registry.hwnd_by_id.keys() {
        if let Some(vp) = registry.get_for_id(id) {
            vp.start_camera_animation(id, pos, 5.0);
        }
    }
    Ok(())
}
```

Registered in `invoke_handler` and in `bridge/modules/viewport.rs` command list,
following the existing `viewport.focus_entity` pattern:
```rust
CommandSpec {
    id: "viewport.focus_entity_animated",
    returns_data: false,
    non_undoable: true,
    args_schema: serde_json::json!({
        "type": "object",
        "properties": { "entityId": { "type": "number" } },
        "required": ["entityId"]
    }),
}
```

### 2e — Frontend

```ts
async function focusEntity(id: number): Promise<void> {
  if (!isTauri) return;
  await invoke('focus_entity_animated', { entityId: id });
}
```

---

## Section 3 — Drag-to-Reparent

### 3a — New `Parent` component in `engine-core`

Add to the component definitions module (e.g., `engine/core/src/components.rs`):

```rust
/// Marks an entity as a child of another entity.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Parent(pub u32);
```

`#[derive(Component)]` is required for ECS storage registration (consistent with `Transform`,
`Health`, `MeshRenderer`, etc.).

Re-export from `engine/core/src/lib.rs`:
```rust
pub use components::Parent;
```

### 3b — Update `get_scene_entities` to read `Parent`

When building `EntityInfo`:
```rust
let parent_id: Option<u64> = world
    .get::<engine_core::Parent>(entity)
    .map(|p| p.0 as u64);
EntityInfo { id, name, components, parent_id }
```

### 3c — Cycle detection helper (free function, testable)

```rust
/// Returns true if making `entity_id` a child of `new_parent_id` would create a cycle.
pub fn would_create_cycle(world: &World, entity_id: u32, new_parent_id: u32) -> bool {
    let mut current = new_parent_id;
    loop {
        if current == entity_id { return true; }
        match world.get::<engine_core::Parent>(Entity::new(current, 0)) {
            Some(p) => current = p.0,
            None    => return false,
        }
    }
}
```

Free function (not a method on the handler) so it can be unit-tested without Tauri state.

### 3d — New IPC: `reparent_entity`

```rust
#[tauri::command]
pub fn reparent_entity(
    entity_id: u64,
    new_parent_id: Option<u64>,
    world_state: tauri::State<'_, crate::state::SceneWorldState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    debug_assert!(entity_id <= u32::MAX as u64);
    let entity = engine_core::Entity::new(entity_id as u32, 0);
    let mut world = world_state.inner().0.write().map_err(|e| e.to_string())?;

    if !world.is_alive(entity) {
        return Err(format!("entity {entity_id} not found"));
    }

    match new_parent_id {
        Some(pid) => {
            debug_assert!(pid <= u32::MAX as u64);
            let parent = engine_core::Entity::new(pid as u32, 0);
            if !world.is_alive(parent) {
                return Err(format!("parent entity {pid} not found"));
            }
            if would_create_cycle(&world, entity_id as u32, pid as u32) {
                return Err("cycle detected".into());
            }
            world.add(entity, engine_core::Parent(pid as u32));
        }
        None => {
            world.remove::<engine_core::Parent>(entity);
        }
    }

    app.emit(
        "entity-reparented",
        serde_json::json!({ "entityId": entity_id, "newParentId": new_parent_id }),
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}
```

Registered in `invoke_handler`.

### 3e — Frontend: drag state variable and handlers

The browser security model prevents `getData()` in `dragover`. Use a module-level reactive
variable set in `ondragstart` instead:

```ts
// Module-level state (Svelte 5 rune):
let reparentTargetId  = $state<number | null>(null);
let draggedEntityId   = $state<number | null>(null);   // set in ondragstart, cleared on drop/end
```

```ts
const ENTITY_MIME = 'application/x-entity-id';

function startEntityDrag(e: DragEvent, id: number) {
  draggedEntityId = id;
  e.dataTransfer!.setData(ENTITY_MIME, String(id));
  e.dataTransfer!.effectAllowed = 'move';
}

function onEntityDragOver(e: DragEvent, target: EntityInfo) {
  if (!e.dataTransfer?.types.includes(ENTITY_MIME)) return; // mesh drag — ignore
  if (draggedEntityId === null) return;
  if (draggedEntityId === target.id) return;
  if (isDescendant(target.id, draggedEntityId)) return;     // cycle guard
  e.preventDefault();
  e.dataTransfer!.dropEffect = 'move';
  reparentTargetId = target.id;
}

async function onEntityDrop(e: DragEvent, newParentId: number) {
  if (!e.dataTransfer?.types.includes(ENTITY_MIME)) return;
  const entityId = draggedEntityId;
  draggedEntityId = null;
  reparentTargetId = null;
  if (entityId === null || entityId === newParentId) return;
  if (!isTauri) return;
  await invoke('reparent_entity', { entityId, newParentId });
}

/** Walk the parentId chain; returns true if ancestorId is above candidateId. */
function isDescendant(candidateId: number, ancestorId: number): boolean {
  let current: number | undefined = candidateId;
  while (current !== undefined) {
    const info = entities.find(e => e.id === current);
    if (!info) return false;
    if (info.parentId === ancestorId) return true;
    current = info.parentId;
  }
  return false;
}
```

Markup:
```svelte
<li
  class="entity-row"
  class:reparent-target={reparentTargetId === entity.id}
  draggable="true"
  ondragstart={(e) => startEntityDrag(e, entity.id)}
  ondragover={(e) => onEntityDragOver(e, entity)}
  ondragleave={() => { reparentTargetId = null; }}
  ondrop={(e) => onEntityDrop(e, entity.id)}
  ondragend={() => { draggedEntityId = null; reparentTargetId = null; }}
  ...
>
```

### 3f — Frontend: handle `entity-reparented` event

```ts
await listen<{ entityId: number; newParentId: number | null | undefined }>(
  'entity-reparented',
  ({ payload }) => {
    entities = entities.map(e =>
      e.id === payload.entityId
        ? { ...e, parentId: payload.newParentId ?? undefined }
        : e
    );
  }
);
```

`buildTree()` is `$derived` from `entities` → tree re-renders automatically.

### 3g — Visual: reparent drop target style

```css
.entity-row.reparent-target {
  outline: 2px dashed var(--accent-color, #7c5cbf);
  outline-offset: -2px;
}
```

Distinct colour from the mesh drop target (which uses a different class/style).

---

## Section 4 — Data Flow Summary

```
User double-clicks name span
  → startRename() + stopPropagation (row handler does not fire)

User double-clicks row padding/chevron/icon
  → focusEntity(id)
  → focus_entity_animated IPC → reads entity Transform
  → start_camera_animation() sets camera_anim on each viewport instance
  → render loop: lerp runs inside lock before snapshot, each frame until settled
  → if gizmo drag active, lerp is skipped each frame

User drags entity row
  → ondragstart: draggedEntityId = id, MIME type set
  → ondragover target row: checks draggedEntityId (not getData), guards cycle
  → ondrop: invoke reparent_entity IPC
  → reparent_entity: cycle check → add/remove Parent → emit entity-reparented
  → entity-reparented listener: updates entities[] → buildTree re-derives
```

---

## Section 5 — Error Handling

- `focus_entity_animated`: entity not in world → `Vec3::ZERO` fallback (matches `set_selected_entity`).
- `reparent_entity`: not found → `Err` propagated as rejected promise (logged to console).
- `reparent_entity`: cycle → `Err("cycle detected")` — no visual change; frontend `isDescendant` guard prevents most calls reaching this path.
- Mesh and entity drag: MIME type check in `ondragover` / `ondrop` prevents cross-interference.

---

## Section 6 — Testing

**Rust unit tests** (no Tauri context required):

1. **`test_parent_component_round_trip`** — `world.add(entity, Parent(5))`, read back, remove, assert absent.
2. **`test_cycle_detection_direct`** — A parent of B; `would_create_cycle(world, A, B)` → `true`.
3. **`test_cycle_detection_indirect`** — A→B→C chain; `would_create_cycle(world, A, C)` → `true`.
4. **`test_no_cycle`** — A→B chain; `would_create_cycle(world, C, B)` → `false`.
5. **`test_reparent_sets_parent_component`** — Call `world.add(entity, Parent(...))` + `world.remove::<Parent>` directly; assert world state. (Tests ECS behaviour, not the Tauri handler — handler depends on Tauri runtime.)

**Playwright E2E tests** (`engine/editor/e2e/editor.spec.ts`):

6. **`rename triggers on name span dblclick`** — Create entity, double-click `.entity-name` → rename input visible.
7. **`dblclick row padding does not open rename`** — Double-click area outside `.entity-name` → no rename input.

---

## Files Touched

| File | Change |
|------|--------|
| `engine/core/src/components.rs` (or equivalent) | Add `#[derive(Component)] pub struct Parent(pub u32)` |
| `engine/core/src/lib.rs` | `pub use components::Parent` re-export |
| `engine/editor/src-tauri/bridge/commands.rs` | Add `focus_entity_animated` IPC; add `reparent_entity` IPC + `would_create_cycle` free function; update `get_scene_entities` to read `Parent`; change `drag_state` field to `Arc<Mutex<Option<DragState>>>` |
| `engine/editor/src-tauri/viewport/native_viewport.rs` | Add `CameraAnimation` struct + `camera_anim` field on `ViewportInstance`; add per-frame lerp inside the instances lock block (before snapshot); add `start_camera_animation(&self, id, pos, dist)` method; accept `drag_state: Arc<Mutex<…>>` in `render_loop` signature; clone `drag_state` at viewport creation site |
| `engine/editor/src-tauri/bridge/modules/viewport.rs` | Register `focus_entity_animated` in command list |
| `engine/editor/src-tauri/main.rs` (or `lib.rs`) | Register `focus_entity_animated` and `reparent_entity` in `invoke_handler` |
| `engine/editor/src/lib/components/HierarchyPanel.svelte` | Move rename dblclick to name span; row dblclick → `focusEntity`; add `draggedEntityId` + `reparentTargetId` state; add drag handlers; add `isDescendant` helper; add `entity-reparented` event listener; add `.reparent-target` CSS |
