# Hierarchy Panel Improvements — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Three hierarchy improvements — rename triggers only on name text double-click, smooth animated camera fly-to on row double-click, and drag-to-reparent saved to the ECS world.

**Architecture:** Backend adds Parent ECS component, reparent_entity IPC, and focus_entity_animated IPC with per-frame camera lerp inside the render loop lock. Frontend adds split double-click targets, drag handlers, and event listeners.

**Tech Stack:** Rust, ECS (engine-core), Tauri IPC, Svelte 5 (runes), CSS

---

## Files Touched

| File | Change |
|------|--------|
| `engine/core/src/gameplay.rs` | Add `Parent` component struct + `impl Component` |
| `engine/core/src/lib.rs` | `pub use gameplay::Parent` re-export |
| `engine/editor/src-tauri/bridge/commands.rs` | Add `would_create_cycle` free fn; add `reparent_entity` IPC; add `focus_entity_animated` IPC; change `drag_state` field type; add `drag_state` Arc clone in `create_native_viewport` |
| `engine/editor/src-tauri/viewport/native_viewport.rs` | Add `CameraAnimation` struct + `camera_anim` field on `ViewportInstance`; per-frame lerp in render loop inside lock block before snapshot; add `start_camera_animation` method; add `drag_state` field + parameter threading |
| `engine/editor/src-tauri/bridge/modules/viewport.rs` | Add `focus_entity_animated` `CommandSpec` entry |
| `engine/editor/src-tauri/lib.rs` | Register `focus_entity_animated` and `reparent_entity` in `invoke_handler` |
| `engine/editor/src/lib/components/HierarchyPanel.svelte` | Split dblclick targets; add `focusEntity`; add drag state + handlers; add `isDescendant`; add `entity-reparented` listener; add `.reparent-target` CSS |
| `engine/editor/e2e/editor.spec.ts` | Two Playwright E2E tests for dblclick behaviour |

---

## Task 1 — Add Parent component to engine-core

**Crate:** `engine/core`

### 1a — Add the component

In `engine/core/src/gameplay.rs`, append after the `Health` impl block (before the `#[cfg(test)]` section):

```rust
/// Marks an entity as a child of another entity.
///
/// # Example
/// ```
/// use engine_core::{World, Entity, Parent};
///
/// let mut world = World::new();
/// let parent = world.spawn();
/// let child  = world.spawn();
/// world.add(child, Parent(parent.id()));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Parent(pub u32);

impl Component for Parent {}
```

`Parent` does not need `Serialize`/`Deserialize` yet — neither `Health` nor `Velocity` carry those derives in this crate. Add them when a serialization task requires it.

### 1b — Re-export from lib.rs

In `engine/core/src/lib.rs`, extend the `pub use gameplay::` line:

```rust
// Before:
pub use gameplay::Health;

// After:
pub use gameplay::{Health, Parent};
```

### 1c — Write the unit test

Add to the existing `#[cfg(test)] mod tests` block inside `engine/core/src/gameplay.rs`:

```rust
#[test]
fn test_parent_component_round_trip() {
    use crate::ecs::World;

    let mut world = World::new();
    let child  = world.spawn();
    let parent = world.spawn();

    // Add
    world.add(child, Parent(parent.id()));
    let got = world.get::<Parent>(child).expect("Parent should be present");
    assert_eq!(got.0, parent.id());

    // Remove
    world.remove::<Parent>(child);
    assert!(world.get::<Parent>(child).is_none(), "Parent should be absent after remove");
}
```

### 1d — Verify and commit

- [ ] Run `cargo test` from `engine/core/`
- [ ] Commit: `feat(core): add Parent component for entity hierarchy`

---

## Task 2 — Update get_scene_entities to read Parent

**File:** `engine/editor/src-tauri/bridge/commands.rs`

### Context

`EntityInfo` already has `pub parent_id: Option<u64>` (line 22) and the field exists in `scan_project_entities`. There is no `get_scene_entities` IPC today — entities are tracked in frontend state. The `create_entity_child` command (around line 164) already sets `parent_id` in the emitted JSON event. The place to update is the `EntityInfo` construction in `scan_project_entities` and in any future snapshot read of the ECS world.

When a world read is added (or if `scan_project_entities` is wired to the ECS world), the pattern to use is:

```rust
let parent_id: Option<u64> = world
    .get::<engine_core::Parent>(entity)
    .map(|p| p.0 as u64);

EntityInfo { id, name, components, parent_id }
```

### 2a — Update scan_project_entities (defensive)

In `commands.rs`, inside `scan_project_entities`, the `.map(|(i, name)| EntityInfo { ... parent_id: None })` closure already sets `parent_id: None`. This is correct for file-scan mode. Add a comment to flag the ECS read path:

```rust
// TODO(hierarchy): when this path queries the live ECS world instead of the
// filesystem, read `world.get::<engine_core::Parent>(entity).map(|p| p.0 as u64)` here.
parent_id: None,
```

### 2b — Write a unit test for the ECS read pattern

Add to the `#[cfg(test)] mod tests` block inside `engine/core/src/gameplay.rs` (or in a new `commands` test module if it already exists in `engine/editor/src-tauri/bridge/commands.rs`):

```rust
#[test]
fn test_parent_read_from_world() {
    use engine_core::{World, Entity, Parent};

    let mut world = World::new();
    let parent = world.spawn();
    let child  = world.spawn();
    world.add(child, Parent(parent.id()));

    let parent_id: Option<u64> = world
        .get::<Parent>(child)
        .map(|p| p.0 as u64);

    assert_eq!(parent_id, Some(parent.id() as u64));

    // Entity without Parent returns None
    let orphan = world.spawn();
    assert!(world.get::<Parent>(orphan).is_none());
}
```

Place this test alongside `test_parent_component_round_trip` in `engine/core/src/gameplay.rs`.

### 2c — Verify and commit

- [ ] Run `cargo test` from `engine/core/`
- [ ] Commit: `feat(core): test Parent ECS read pattern for hierarchy serialization`

---

## Task 3 — Add would_create_cycle + reparent_entity IPC

**File:** `engine/editor/src-tauri/bridge/commands.rs`

### 3a — Free function would_create_cycle

Add directly above the `reparent_entity` command definition (not inside any `impl`). The function takes a `&World` reference so it can be unit-tested without Tauri state.

```rust
/// Returns `true` if making `entity_id` a child of `new_parent_id` would
/// create a cycle in the parent-child hierarchy.
///
/// Walks the parent chain of `new_parent_id` upward; if it reaches
/// `entity_id`, a cycle would result.
pub fn would_create_cycle(
    world: &engine_core::World,
    entity_id: u32,
    new_parent_id: u32,
) -> bool {
    let mut current = new_parent_id;
    loop {
        if current == entity_id {
            return true;
        }
        match world.get::<engine_core::Parent>(engine_core::Entity::new(current, 0)) {
            Some(p) => current = p.0,
            None    => return false,
        }
    }
}
```

### 3b — reparent_entity IPC command

Add after `would_create_cycle`:

```rust
/// Reparent an entity in the live ECS world.
///
/// Pass `new_parent_id: None` to make the entity a root (removes the `Parent`
/// component). Emits `entity-reparented` so the frontend updates its tree.
#[tauri::command]
pub fn reparent_entity(
    entity_id: u64,
    new_parent_id: Option<u64>,
    world_state: tauri::State<'_, crate::state::SceneWorldState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    use tauri::Emitter;
    debug_assert!(entity_id <= u32::MAX as u64, "entity_id overflows u32");

    let entity = engine_core::Entity::new(entity_id as u32, 0);
    let mut world = world_state.inner().0.write().map_err(|e| e.to_string())?;

    if !world.is_alive(entity) {
        return Err(format!("entity {entity_id} not found"));
    }

    match new_parent_id {
        Some(pid) => {
            debug_assert!(pid <= u32::MAX as u64, "new_parent_id overflows u32");
            let parent_entity = engine_core::Entity::new(pid as u32, 0);
            if !world.is_alive(parent_entity) {
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

    tracing::debug!(entity_id, new_parent_id = ?new_parent_id, "reparent_entity");

    app.emit(
        "entity-reparented",
        serde_json::json!({ "entityId": entity_id, "newParentId": new_parent_id }),
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}
```

### 3c — Register reparent_entity in invoke_handler

In `engine/editor/src-tauri/lib.rs`, add to the `tauri::generate_handler![...]` list, after `commands::assign_mesh`:

```rust
commands::reparent_entity,
```

### 3d — Write unit tests

Add a `#[cfg(test)] mod tests` block in `commands.rs` (or extend the existing one):

```rust
#[cfg(test)]
mod hierarchy_tests {
    use super::would_create_cycle;
    use engine_core::{World, Entity, Parent};

    #[test]
    fn test_cycle_detection_direct() {
        // A is parent of B; making A a child of B would cycle.
        let mut world = World::new();
        let a = world.spawn(); // id = first allocated
        let b = world.spawn();
        world.add(b, Parent(a.id())); // B's parent is A
        // would_create_cycle(world, A, B): walking from B finds A → cycle
        assert!(would_create_cycle(&world, a.id(), b.id()));
    }

    #[test]
    fn test_cycle_detection_indirect() {
        // Chain: A → B → C (C's parent is B, B's parent is A).
        // Making A a child of C would cycle: C → B → A → C.
        let mut world = World::new();
        let a = world.spawn();
        let b = world.spawn();
        let c = world.spawn();
        world.add(b, Parent(a.id())); // B's parent is A
        world.add(c, Parent(b.id())); // C's parent is B
        assert!(would_create_cycle(&world, a.id(), c.id()));
    }

    #[test]
    fn test_no_cycle() {
        // A → B chain; making C a child of B is fine.
        let mut world = World::new();
        let a = world.spawn();
        let b = world.spawn();
        let c = world.spawn();
        world.add(b, Parent(a.id()));
        assert!(!would_create_cycle(&world, c.id(), b.id()));
    }

    #[test]
    fn test_reparent_sets_parent_component() {
        // Simulate what reparent_entity does to the ECS world.
        let mut world = World::new();
        let parent = world.spawn();
        let child  = world.spawn();

        // Set parent
        world.add(child, Parent(parent.id()));
        assert_eq!(
            world.get::<Parent>(child).map(|p| p.0),
            Some(parent.id())
        );

        // Remove parent (make root)
        world.remove::<Parent>(child);
        assert!(world.get::<Parent>(child).is_none());
    }
}
```

**Note:** These tests exercise the free `would_create_cycle` function and direct ECS mutations. They do not require a Tauri runtime. Run them with `cargo test` from `engine/editor/src-tauri/` (or the workspace root).

### 3e — Verify and commit

- [ ] Run `cargo test` from `engine/editor/src-tauri/`
- [ ] Commit: `feat(editor): add reparent_entity IPC + would_create_cycle cycle detection`

---

## Task 4 — Add CameraAnimation + camera lerp in render loop

**File:** `engine/editor/src-tauri/viewport/native_viewport.rs`

This task has four coordinated sub-steps that must be applied together or the code will not compile.

### 4a — Add CameraAnimation struct and camera_anim field

`ViewportInstance` is defined at around line 61. Add the sibling struct immediately before it (inside the `#[cfg(windows)] mod inner { ... }` block if one exists, or directly in the module):

```rust
/// Describes a smooth camera fly-to animation targeting a world-space position
/// and a zoom distance. The render loop lerps toward these values each frame.
struct CameraAnimation {
    target_pos:  glam::Vec3,
    /// Orbit distance to animate toward. Always `5.0` for hierarchy focus.
    target_dist: f32,
}
```

Add `camera_anim: Option<CameraAnimation>` to `ViewportInstance`:

```rust
// Before:
#[derive(Clone)]
struct ViewportInstance {
    bounds: ViewportBounds,
    camera: OrbitCamera,
    visible: bool,
    grid_visible: bool,
    is_ortho: bool,
}

// After:
#[derive(Clone)]
struct ViewportInstance {
    bounds: ViewportBounds,
    camera: OrbitCamera,
    visible: bool,
    grid_visible: bool,
    is_ortho: bool,
    camera_anim: Option<CameraAnimation>,
}
```

`CameraAnimation` does not need `Clone` because it lives only inside `ViewportInstance::camera_anim` and is consumed frame-by-frame; the outer `#[derive(Clone)]` on `ViewportInstance` requires it, however, so add `#[derive(Clone)]` to `CameraAnimation` as well.

Update `ViewportInstance::new` to initialise the field:

```rust
impl ViewportInstance {
    fn new(bounds: ViewportBounds) -> Self {
        Self {
            bounds,
            camera: OrbitCamera::default(),
            visible: true,
            grid_visible: true,
            is_ortho: false,
            camera_anim: None,   // ← new
        }
    }
}
```

### 4b — Change drag_state in NativeViewportState to Arc<Mutex<...>>

**File:** `engine/editor/src-tauri/bridge/commands.rs`, `NativeViewportState` struct (around line 370).

The current type is `Mutex<Option<DragState>>`. Change it to `Arc<Mutex<Option<DragState>>>` so it can be cloned into the render thread:

```rust
// Before:
pub drag_state: Mutex<Option<crate::bridge::gizmo_commands::DragState>>,

// After:
pub drag_state: std::sync::Arc<Mutex<Option<crate::bridge::gizmo_commands::DragState>>>,
```

Update `Default` impl accordingly:

```rust
// Before:
drag_state: Mutex::new(None),

// After:
drag_state: std::sync::Arc::new(Mutex::new(None)),
```

Any site in `gizmo_commands.rs` that calls `viewport_state.drag_state.lock()` continues to work unchanged because `Arc<Mutex<T>>` derefs to `Mutex<T>` for `.lock()`.

### 4c — Thread drag_state through NativeViewport::new + start_rendering + render_loop

**Step 1 — create_native_viewport in commands.rs** (around line 437): clone the Arc before calling `NativeViewport::new`:

```rust
// Add this line before the NativeViewport::new call:
let drag_state = std::sync::Arc::clone(&viewport_state.drag_state);

// Pass drag_state as a new argument:
let mut vp = NativeViewport::new(
    parent_hwnd,
    world_state.inner().0.clone(),
    selected_entity_id,
    gizmo_mode,
    hovered_gizmo_axis,
    asset_manager,
    drag_state,          // ← new
).map_err(|e| { ... })?;
```

**Step 2 — NativeViewport struct** (around line 87): add the field:

```rust
/// Active gizmo drag state, shared with the render thread for animation guard.
drag_state: std::sync::Arc<Mutex<Option<crate::bridge::gizmo_commands::DragState>>>,
```

**Step 3 — NativeViewport::new signature** (around line 110): add parameter and store it:

```rust
pub fn new(
    parent_hwnd: HWND,
    world: Arc<std::sync::RwLock<engine_core::World>>,
    selected_entity_id: std::sync::Arc<std::sync::Mutex<Option<u64>>>,
    gizmo_mode: std::sync::Arc<std::sync::atomic::AtomicU8>,
    hovered_gizmo_axis: std::sync::Arc<std::sync::atomic::AtomicU8>,
    asset_manager: Arc<engine_assets::AssetManager>,
    drag_state: std::sync::Arc<Mutex<Option<crate::bridge::gizmo_commands::DragState>>>,
) -> Result<Self, String> {
    Ok(Self {
        // ... existing fields ...
        drag_state,   // ← new
    })
}
```

**Step 4 — start_rendering** (around line 134): clone before the thread spawn, then pass to `render_loop`:

```rust
let drag_state = self.drag_state.clone();   // ← new clone

let handle = std::thread::Builder::new()
    .name("viewport-render".into())
    .spawn(move || {
        // ...
        render_loop(
            hwnd, should_stop, render_active, instances, world,
            screenshot_slot, selected_entity_id, gizmo_mode,
            hovered_gizmo_axis, asset_manager,
            drag_state,   // ← new argument
        );
    })
```

**Step 5 — render_loop signature** (around line 1040): add the parameter:

```rust
fn render_loop(
    hwnd: HWND,
    should_stop: Arc<AtomicBool>,
    render_active: Arc<AtomicBool>,
    instances: Arc<Mutex<HashMap<String, ViewportInstance>>>,
    world: Arc<std::sync::RwLock<engine_core::World>>,
    screenshot_slot: Arc<Mutex<Option<std::sync::mpsc::SyncSender<Result<Vec<u8>, String>>>>>,
    selected_entity_id: std::sync::Arc<std::sync::Mutex<Option<u64>>>,
    gizmo_mode: std::sync::Arc<std::sync::atomic::AtomicU8>,
    hovered_gizmo_axis: std::sync::Arc<std::sync::atomic::AtomicU8>,
    asset_manager: Arc<engine_assets::AssetManager>,
    drag_state: std::sync::Arc<Mutex<Option<crate::bridge::gizmo_commands::DragState>>>,
) {
```

### 4d — Per-frame lerp INSIDE the instances lock block BEFORE the snapshot clone

**Why this must be inside the lock:** The render loop snapshots instance data by cloning `i.camera` values under the lock. Any mutation to `inst.camera` made *outside* the lock — or *after* the clone — will be silently discarded for that frame. The animation must mutate `inst.camera` and only *then* clone it, which requires both actions to happen inside the same `instances.lock()` block.

Replace the existing snapshot block (around line 1074):

```rust
// Before:
let viewports: Vec<(ViewportBounds, OrbitCamera, bool, bool)> = {
    let lock = instances.lock().unwrap();
    lock.values()
        .filter(|i| i.visible)
        .map(|i| (i.bounds, i.camera.clone(), i.grid_visible, i.is_ortho))
        .collect()
};

// After:
let viewports: Vec<(ViewportBounds, OrbitCamera, bool, bool)> = {
    let mut lock = instances.lock().unwrap();  // mut: needed to lerp in-place

    for inst in lock.values_mut().filter(|i| i.visible) {
        if let Some(ref anim) = inst.camera_anim {
            // Skip lerp while a gizmo drag is active (drag owns the camera).
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

    // Snapshot *after* lerp mutations so each frame renders the animated state.
    lock.values()
        .filter(|i| i.visible)
        .map(|i| (i.bounds, i.camera.clone(), i.grid_visible, i.is_ortho))
        .collect()
};
```

Pitch and yaw are intentionally not modified — camera orientation is fully preserved through the fly-to.

### 4e — Add start_camera_animation method on NativeViewport

Add alongside the existing `camera_focus` method (around line 268):

```rust
/// Begin a smooth animated camera fly-to for the named viewport instance.
///
/// Each render frame lerps `camera.target` toward `target_pos` and
/// `camera.distance` toward `target_dist` at a fixed rate (LERP = 0.12).
/// The animation stops when both values are within 0.001 of the targets.
pub fn start_camera_animation(
    &self,
    id: &str,
    target_pos: glam::Vec3,
    target_dist: f32,
) {
    if let Ok(mut instances) = self.instances.lock() {
        if let Some(inst) = instances.get_mut(id) {
            inst.camera_anim = Some(CameraAnimation { target_pos, target_dist });
        }
    }
}
```

### 4f — Verify and commit

- [ ] Run `cargo build` from `engine/editor/src-tauri/` (full test requires a display; build check is sufficient)
- [ ] Commit: `feat(viewport): add CameraAnimation + per-frame lerp + drag_state Arc threading`

---

## Task 5 — Add focus_entity_animated IPC

**Files:** `engine/editor/src-tauri/bridge/commands.rs`, `engine/editor/src-tauri/bridge/modules/viewport.rs`, `engine/editor/src-tauri/lib.rs`

### 5a — The IPC command

Add to `commands.rs`, near `set_selected_entity` (around line 940):

```rust
/// Smoothly animate every active viewport camera to orbit the given entity.
///
/// Reads the entity's `Transform` position from the ECS world. Falls back to
/// `Vec3::ZERO` if the entity has no `Transform` (matches `set_selected_entity`
/// behaviour). Camera orientation (yaw/pitch) is preserved.
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
    let ids: Vec<String> = registry.hwnd_by_id.keys().cloned().collect();
    for id in &ids {
        if let Some(vp) = registry.get_for_id(id) {
            vp.start_camera_animation(id, pos, 5.0);
        }
    }

    tracing::debug!(entity_id, ?pos, "focus_entity_animated");
    Ok(())
}
```

### 5b — Register in invoke_handler

In `engine/editor/src-tauri/lib.rs`, add after `commands::assign_mesh` (and alongside `commands::reparent_entity` from Task 3):

```rust
commands::focus_entity_animated,
commands::reparent_entity,
```

### 5c — Add CommandSpec to viewport module

In `engine/editor/src-tauri/bridge/modules/viewport.rs`, add inside the `vec![...]` returned by `commands()`, after the existing `viewport.focus_entity` entry:

```rust
CommandSpec {
    id: "viewport.focus_entity_animated".into(), module_id: String::new(),
    label: "Focus Entity (Animated)".into(), category: "Viewport".into(),
    description: Some(
        "Smoothly animate the viewport camera to orbit the selected entity".into()
    ),
    keybind: None,
    args_schema: Some(serde_json::json!({
        "type": "object",
        "properties": { "entityId": { "type": "number" } },
        "required": ["entityId"]
    })),
    returns_data: false,
    non_undoable: true,
},
```

### 5d — Verify and commit

- [ ] Run `cargo build` from `engine/editor/src-tauri/`
- [ ] Run `cargo test` from `engine/editor/src-tauri/bridge/modules/` to confirm `commands_have_correct_prefix` still passes
- [ ] Commit: `feat(editor): add focus_entity_animated IPC for smooth camera fly-to`

---

## Task 6 — Frontend: rename on name span only + focusEntity on row dblclick

**File:** `engine/editor/src/lib/components/HierarchyPanel.svelte`

### 6a — Add focusEntity function

Add to the `<script>` block, after the `cancelRename` function (around line 116):

```ts
import { isTauri } from '@tauri-apps/api/core';
// (or use the existing isTauri guard pattern from the codebase)

async function focusEntity(id: number): Promise<void> {
  if (!isTauri) return;
  const { invoke } = await import('@tauri-apps/api/core');
  await invoke('focus_entity_animated', { entityId: id });
}
```

Check whether `isTauri` and `invoke` are already imported at the top of the file (they are likely imported elsewhere in the project — use the same pattern as `setSelectedEntity` in other commands files).

### 6b — Move ondblclick from li to span + add row-level ondblclick

Locate the `<li class="entity-row" ... ondblclick={() => startRename(entity.id, entity.name)} ...>` (line 215 in the current file).

**Change 1:** Remove `ondblclick` from the `<li>` element and replace it with a fly-to handler:

```svelte
<li
  class="entity-row"
  ...
  ondblclick={() => focusEntity(entity.id)}
  ...
>
```

**Change 2:** Locate `<span class="entity-name">{entity.name}</span>` (line 257 in the current file). Wrap the name text in an `ondblclick` handler that stops propagation so the row-level handler does not also fire:

```svelte
{:else}
  <span
    class="entity-name"
    ondblclick={(e) => { e.stopPropagation(); startRename(entity.id, entity.name); }}
  >{entity.name}</span>
{/if}
```

**Event flow:** User double-clicks the name span → `startRename` fires → `stopPropagation` prevents the row `ondblclick` from receiving the event. User double-clicks the chevron, icon, padding, or component count → no `stopPropagation` → row `ondblclick` fires → `focusEntity`.

### 6c — Verify and commit

- [ ] Run `npm run dev` from `engine/editor/` and manually verify:
  - Double-clicking the entity name text opens the rename input
  - Double-clicking the row padding/chevron/icon does not open the rename input
- [ ] Commit: `feat(editor): split rename/fly-to dblclick — name span renames, row padding focuses`

---

## Task 7 — Frontend: drag-to-reparent

**File:** `engine/editor/src/lib/components/HierarchyPanel.svelte`

### 7a — Add drag state variables

Add to the `<script>` block, after the existing `let dropTargetId` declaration (around line 144):

```ts
// Entity drag-to-reparent state
const ENTITY_MIME = 'application/x-entity-id';
let draggedEntityId  = $state<number | null>(null);
let reparentTargetId = $state<number | null>(null);
```

### 7b — Add drag handlers

Add after the `onDropMesh` function (around line 166):

```ts
function startEntityDrag(e: DragEvent, id: number): void {
  draggedEntityId = id;
  e.dataTransfer!.setData(ENTITY_MIME, String(id));
  e.dataTransfer!.effectAllowed = 'move';
}

function onEntityDragOver(e: DragEvent, target: EntityInfo): void {
  // Ignore mesh drags — only handle entity drags.
  if (!e.dataTransfer?.types.includes(ENTITY_MIME)) return;
  if (draggedEntityId === null) return;
  if (draggedEntityId === target.id) return;
  if (isDescendant(target.id, draggedEntityId)) return; // frontend cycle guard
  e.preventDefault();
  e.dataTransfer!.dropEffect = 'move';
  reparentTargetId = target.id;
}

async function onEntityDrop(e: DragEvent, newParentId: number): Promise<void> {
  if (!e.dataTransfer?.types.includes(ENTITY_MIME)) return;
  const entityId = draggedEntityId;
  draggedEntityId   = null;
  reparentTargetId  = null;
  if (entityId === null || entityId === newParentId) return;
  if (!isTauri) return;
  const { invoke } = await import('@tauri-apps/api/core');
  await invoke('reparent_entity', { entityId, newParentId });
}

/**
 * Walk the parentId chain upward from `candidateId`.
 * Returns true if `ancestorId` appears anywhere in that chain.
 * Used as a frontend cycle guard before calling reparent_entity IPC.
 */
function isDescendant(candidateId: number, ancestorId: number): boolean {
  let current: number | undefined = candidateId;
  while (current !== undefined) {
    const info = entities.find((e) => e.id === current);
    if (!info) return false;
    if (info.parentId === ancestorId) return true;
    current = info.parentId;
  }
  return false;
}
```

**Key design note:** `onEntityDragOver` reads `draggedEntityId` from reactive state, NOT from `e.dataTransfer.getData()`. The browser security model prohibits `getData()` during `dragover`; only `types` is available. The module-level `$state` variable bridges this gap.

### 7c — Update the entity-row markup

Add drag attributes to the `<li class="entity-row">` element. The existing `ondragover` and `ondrop` handlers are for mesh drop — they must coexist. Update the element:

```svelte
<li
  class="entity-row"
  class:selected={selectedId === entity.id}
  class:drop-target={dropTargetId === entity.id}
  class:reparent-target={reparentTargetId === entity.id}
  role="option"
  aria-selected={selectedId === entity.id}
  tabindex="0"
  style="padding-left: {8 + depth * 14}px"
  draggable="true"
  onmouseenter={() => { hoveredId = entity.id; }}
  onmouseleave={() => { if (!contextMenu) hoveredId = null; }}
  onclick={() => {
    if (renamingId !== entity.id) {
      onSelect(entity.id);
    }
  }}
  ondblclick={() => focusEntity(entity.id)}
  oncontextmenu={(e) => openContextMenu(e, entity.id)}
  onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { onSelect(entity.id); } }}
  ondragstart={(e) => startEntityDrag(e, entity.id)}
  ondragover={(e) => {
    onDragOver(e, entity.id);       // mesh handler (existing)
    onEntityDragOver(e, entity);    // entity handler (new)
  }}
  ondragleave={() => {
    onDragLeave(entity.id);         // mesh handler (existing)
    if (reparentTargetId === entity.id) reparentTargetId = null;
  }}
  ondrop={(e) => {
    onDropMesh(e, entity.id);       // mesh handler (existing)
    onEntityDrop(e, entity.id);     // entity handler (new)
  }}
  ondragend={() => { draggedEntityId = null; reparentTargetId = null; }}
>
```

### 7d — Add .reparent-target CSS

Add to the `<style>` block, after the existing `.entity-row.drop-target` rule (around line 409):

```css
.entity-row.reparent-target {
  outline: 2px dashed var(--accent-color, #7c5cbf);
  outline-offset: -2px;
}
```

The dashed purple outline is visually distinct from the solid blue `.drop-target` outline used by mesh drops.

### 7e — Verify and commit

- [ ] Run `npm run dev` from `engine/editor/` and manually verify drag cursor appears over rows
- [ ] Commit: `feat(editor): drag-to-reparent entity rows with cycle guard and drop highlight`

---

## Task 8 — Frontend: entity-reparented event listener

**File:** `engine/editor/src/lib/components/HierarchyPanel.svelte`

### 8a — Add the listen call

The `entities` prop comes from the parent (`let { entities = [], ... } = $props()`). To update `entities` in response to a backend event the component needs writable local state. Check whether `entities` is already writable local state or a prop — if it is a prop, the listener should live in the parent that owns the entities array, not in `HierarchyPanel.svelte`. Follow the existing pattern used by the `entity-created` and `entity-deleted` event listeners in the parent component.

In the component that owns the canonical `entities` array (likely `App.svelte` or the scene store), add:

```ts
import { listen } from '@tauri-apps/api/event';
import { isTauri } from '@tauri-apps/api/core';

// In onMount or a $effect:
if (isTauri) {
  const unlisten = await listen<{ entityId: number; newParentId: number | null | undefined }>(
    'entity-reparented',
    ({ payload }) => {
      entities = entities.map((e) =>
        e.id === payload.entityId
          ? { ...e, parentId: payload.newParentId ?? undefined }
          : e
      );
    }
  );
  onDestroy(unlisten);
}
```

`buildTree()` in `HierarchyPanel.svelte` is `$derived` from `entities`, so the tree re-renders automatically when `entities` changes.

If `HierarchyPanel.svelte` accepts `entities` as a mutable `$bindable` prop or owns the array itself, place the listener directly in its `<script>` block using the same pattern.

### 8b — Verify and commit

- [ ] Verify in `npm run dev`: drag-reparenting an entity updates the tree without a full refresh
- [ ] Commit: `feat(editor): listen for entity-reparented event to update hierarchy tree`

---

## Task 9 — Playwright E2E tests

**File:** `engine/editor/e2e/editor.spec.ts`

### 9a — Add selectors

The `SEL` constant at the top of `editor.spec.ts` already has `entityName: '.entity-name'`. No new selectors are needed.

### 9b — Add the two tests

Add after the last existing test in the file:

```ts
// ---------------------------------------------------------------------------
// Hierarchy double-click behaviour
// ---------------------------------------------------------------------------

test('rename triggers on name span dblclick', async ({ page }) => {
  await page.goto('http://localhost:4173');

  // Create an entity so the hierarchy is not empty.
  const row = await createEntity(page);

  // Double-click the entity name span specifically.
  const nameSpan = row.locator(SEL.entityName);
  await nameSpan.dblclick();

  // The rename input should appear.
  await expect(row.locator(SEL.renameInput)).toBeVisible();
});

test('dblclick row padding does not open rename', async ({ page }) => {
  await page.goto('http://localhost:4173');

  const row = await createEntity(page);

  // Double-click the far left edge of the row (chevron/padding area), NOT the name.
  // The bounding box offset (x: 4) lands on the chevron span, before the name text.
  const box = await row.boundingBox();
  if (!box) throw new Error('entity row has no bounding box');
  await page.mouse.dblclick(box.x + 4, box.y + box.height / 2);

  // The rename input must NOT appear.
  await expect(row.locator(SEL.renameInput)).not.toBeVisible();
});
```

### 9c — Verify and commit

- [ ] Run `npm run preview` from `engine/editor/` then `npx playwright test e2e/editor.spec.ts` (or the project's equivalent E2E command)
- [ ] Commit: `test(editor): add E2E tests for split rename/fly-to dblclick behaviour`

---

## Execution Order and Dependencies

```
Task 1 (Parent component) ──► Task 2 (ECS read pattern)
                                     │
Task 3 (cycle detection + reparent)◄─┘
       │
       ▼
Task 4 (CameraAnimation + lerp)
       │
       ▼
Task 5 (focus_entity_animated IPC) ──► Task 6 (frontend dblclick split)
                                              │
Task 7 (drag-to-reparent frontend) ◄──────────┘
       │
       ▼
Task 8 (entity-reparented listener)
       │
       ▼
Task 9 (E2E tests)
```

Tasks 1–5 are backend-only and can be reviewed as a single PR. Tasks 6–9 are frontend and depend on Tasks 3 and 5 being merged first (or stubbed behind `isTauri` guards).

---

## Error Handling Summary

| Scenario | Behaviour |
|----------|-----------|
| `focus_entity_animated` — entity has no Transform | Falls back to `Vec3::ZERO`; camera animates to scene origin |
| `reparent_entity` — entity not alive | Returns `Err`; frontend receives rejected promise; logged to console |
| `reparent_entity` — cycle detected | Returns `Err("cycle detected")`; frontend `isDescendant` guard prevents most calls reaching this path |
| Mesh drag and entity drag coexist on same row | MIME type check (`application/x-entity-id` vs `application/x-mesh-path`) prevents cross-interference in `ondragover` and `ondrop` |
| Gizmo drag active during camera animation | Lerp is skipped each frame; animation resumes immediately when drag ends |
