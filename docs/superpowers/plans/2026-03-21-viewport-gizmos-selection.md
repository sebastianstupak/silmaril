# Viewport Gizmos & Selection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire entity selection from the hierarchy to the gizmo renderer, unify gizmo drag undo through the template `CommandProcessor`, and remove the parallel `SceneUndoStack`.

**Architecture:** `selectedEntityId` from `editor-context` flows to `NativeViewportState` via a new Tauri command; the render thread reads it each frame via an `Arc<Mutex<Option<u64>>>` cloned at spawn. `gizmo_drag_end` records transforms through `template_execute_inner` instead of the hand-rolled `SceneUndoStack`. Template execute/undo/redo Tauri wrappers gain an ECS sync step so the live world stays in sync after every command. `SceneUndoStack`, `SceneAction`, and `SerializedTransform` are deleted.

**Tech Stack:** Rust / Tauri 2, `engine_ops::CommandProcessor`, `engine_core::World`, Svelte 5, TypeScript, Vitest.

**Spec:** `docs/superpowers/specs/2026-03-21-viewport-gizmos-selection-design.md`

---

## File Map

### Rust — Modified

| File | Change |
|------|--------|
| `engine/editor/src-tauri/bridge/commands.rs` | `NativeViewportState`: add `selected_entity_id: Arc<Mutex<Option<u64>>>`, change `gizmo_mode: AtomicU8` → `Arc<AtomicU8>`; add `set_selected_entity` command; remove `scene_undo`, `scene_redo`, `apply_scene_action` |
| `engine/editor/src-tauri/bridge/gizmo_commands.rs` | Remove `transform_before` from `DragState`; rewrite `gizmo_drag_end` (remove `undo_state`, add `template_path`/`editor_state`/`world_state`/`app`) |
| `engine/editor/src-tauri/bridge/template_commands.rs` | Add `sync_transform_to_ecs`, `sync_all_transforms` private helpers; update `template_execute`, `template_undo`, `template_redo` wrappers to accept `world_state`+`app` and call sync helpers |
| `engine/editor/src-tauri/viewport/native_viewport.rs` | `NativeViewport` gains `selected_entity_id`+`gizmo_mode` Arc fields; `new()` accepts them; `start_rendering()` clones into thread; `render_loop` + `render_frame` signatures updated |
| `engine/editor/src-tauri/state/mod.rs` | Remove `pub mod scene_undo` and its re-exports |
| `engine/editor/src-tauri/lib.rs` | Remove `.manage(Mutex::new(SceneUndoStack::new()))`; register `set_selected_entity`; remove `scene_undo`/`scene_redo` from `invoke_handler!` |

### Rust — Deleted

| File | Reason |
|------|--------|
| `engine/editor/src-tauri/state/scene_undo.rs` | Replaced by `CommandProcessor` |

### TypeScript — Modified

| File | Change |
|------|--------|
| `engine/editor/src/lib/api.ts` | Add `setSelectedEntity(id: number \| null)`; update `gizmoDragEnd` to accept `templatePath: string`; remove `sceneUndo`/`sceneRedo` exports |
| `engine/editor/src/lib/stores/editor-context.ts` | Add `getSelectedEntityId()` export |
| `engine/editor/src/lib/stores/undo-history.ts` | Rewrite `sceneUndo`/`sceneRedo` to call `template.undo`/`template.redo`; remove `_sceneCanUndo`/`_sceneCanRedo`/`getSceneCanUndo()`/`getSceneCanRedo()` |
| `engine/editor/src/lib/stores/undo-history.test.ts` | Remove stale `$lib/api` mock entries for `sceneUndo`/`sceneRedo`; add tests for new `sceneUndo`/`sceneRedo` behavior |
| `engine/editor/src/lib/docking/panels/HierarchyWrapper.svelte` | Add `$effect` that calls `setSelectedEntity` on selection change |
| `engine/editor/src/lib/docking/panels/ViewportPanel.svelte` | Import `getActiveTemplatePath`; pass it to `gizmoDragEnd` |

---

## Task 1: `set_selected_entity` Tauri command

**Files:**
- Modify: `engine/editor/src-tauri/bridge/commands.rs`
- Modify: `engine/editor/src-tauri/lib.rs`

### Background

`NativeViewportState` currently has `gizmo_mode: AtomicU8`. The render thread needs to also read `selected_entity_id`. Both values must be shared via `Arc` so they can be cloned into the render thread at spawn time (Task 2). We change `gizmo_mode` to `Arc<AtomicU8>` now so both changes land together. All existing `.load()` / `.store()` calls on `gizmo_mode` continue to work unchanged via `Deref`.

- [ ] **Step 1: Write the failing test in `commands.rs`**

Add at the bottom of `commands.rs` (inside the existing `#[cfg(test)] mod tests { ... }` block or create one):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn selected_entity_id_default_is_none() {
        let state = NativeViewportState::new();
        assert_eq!(*state.selected_entity_id.lock().unwrap(), None);
    }

    #[test]
    fn selected_entity_id_can_be_set_and_cleared() {
        let state = NativeViewportState::new();
        *state.selected_entity_id.lock().unwrap() = Some(42);
        assert_eq!(*state.selected_entity_id.lock().unwrap(), Some(42));
        *state.selected_entity_id.lock().unwrap() = None;
        assert_eq!(*state.selected_entity_id.lock().unwrap(), None);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test -p silmaril-editor selected_entity_id
```

Expected: compile error — `selected_entity_id` does not exist on `NativeViewportState`.

- [ ] **Step 3: Update `NativeViewportState` in `commands.rs`**

Find the `NativeViewportState` struct (currently around line 322). Change it from:

```rust
pub struct NativeViewportState {
    pub registry: Mutex<ViewportRegistry>,
    pub drag_state: Mutex<Option<crate::bridge::gizmo_commands::DragState>>,
    pub gizmo_mode: std::sync::atomic::AtomicU8,
}

impl Default for NativeViewportState {
    fn default() -> Self {
        Self {
            registry: Mutex::new(ViewportRegistry::new()),
            drag_state: Mutex::new(None),
            gizmo_mode: std::sync::atomic::AtomicU8::new(0),
        }
    }
}
```

To:

```rust
pub struct NativeViewportState {
    pub registry: Mutex<ViewportRegistry>,
    pub drag_state: Mutex<Option<crate::bridge::gizmo_commands::DragState>>,
    /// Current gizmo mode: 0 = Move, 1 = Rotate, 2 = Scale.
    /// Stored as `Arc<AtomicU8>` so it can be cloned into the render thread.
    pub gizmo_mode: std::sync::Arc<std::sync::atomic::AtomicU8>,
    /// The entity currently selected in the hierarchy, or `None`.
    /// Stored as `Arc<Mutex<Option<u64>>>` so the render thread can read it.
    pub selected_entity_id: std::sync::Arc<Mutex<Option<u64>>>,
}

impl Default for NativeViewportState {
    fn default() -> Self {
        Self {
            registry: Mutex::new(ViewportRegistry::new()),
            drag_state: Mutex::new(None),
            gizmo_mode: std::sync::Arc::new(std::sync::atomic::AtomicU8::new(0)),
            selected_entity_id: std::sync::Arc::new(Mutex::new(None)),
        }
    }
}

// Preserve the existing `new()` constructor — `lib.rs` calls it.
impl NativeViewportState {
    pub fn new() -> Self {
        Self::default()
    }
}
```

- [ ] **Step 4: Add `set_selected_entity` command to `commands.rs`**

Find `pub fn set_gizmo_mode` (in `gizmo_commands.rs`) — the new command goes in `commands.rs` at the end of the IPC section (near line 1230, before `scene_undo`). Add:

```rust
/// Update which entity is selected in the viewport gizmo renderer.
///
/// Called by the frontend whenever `selectedEntityId` changes in the hierarchy.
/// Pass `None` to deselect.
#[tauri::command]
pub fn set_selected_entity(
    entity_id: Option<u64>,
    viewport_state: tauri::State<'_, NativeViewportState>,
) -> Result<(), String> {
    *viewport_state
        .selected_entity_id
        .lock()
        .map_err(|e| e.to_string())? = entity_id;
    Ok(())
}
```

- [ ] **Step 5: Register the command in `lib.rs`**

In `lib.rs`, find the `invoke_handler!` block. Add `commands::set_selected_entity` to the list alongside the other viewport commands.

- [ ] **Step 6: Run the tests to verify they pass**

```bash
cargo test -p silmaril-editor selected_entity_id
```

Expected: both tests pass.

- [ ] **Step 7: Commit**

```bash
git add engine/editor/src-tauri/bridge/commands.rs engine/editor/src-tauri/lib.rs
git commit -m "feat(editor): add selected_entity_id to NativeViewportState, add set_selected_entity command"
```

---

## Task 2: Render thread wiring

**Files:**
- Modify: `engine/editor/src-tauri/viewport/native_viewport.rs`
- Modify: `engine/editor/src-tauri/bridge/commands.rs`

### Background

`NativeViewport::new()` (Windows implementation, inside `#[cfg(windows)] mod imp { ... }`) currently takes `(parent_hwnd, world)`. We add the two new Arc values so `start_rendering()` can clone them into the render thread. `render_loop` and `render_frame` receive them and read the selected entity + gizmo mode each frame.

`create_native_viewport` in `commands.rs` calls `NativeViewport::new()` — we update that call too.

The non-Windows stub `impl NativeViewport` (at the bottom of the file, around line 1200) also gets the updated `new()` signature so it compiles.

- [ ] **Step 1: Write the failing test for render thread wiring in `native_viewport.rs`**

Add to the existing `#[cfg(test)] mod tests` block inside the `#[cfg(windows)] mod imp { ... }` section:

```rust
#[test]
fn render_loop_reads_selected_entity_from_arc() {
    use std::sync::{Arc, Mutex};
    let selected = Arc::new(Mutex::new(Some(7u64)));
    let val = selected.lock().ok().and_then(|g| *g);
    assert_eq!(val, Some(7));
    *selected.lock().unwrap() = None;
    let val = selected.lock().ok().and_then(|g| *g);
    assert_eq!(val, None);
}
```

This tests the Arc read pattern used in `render_loop`; it doesn't need a real Vulkan context.

- [ ] **Step 2: Run test to verify it compiles and passes** (it should pass immediately since it's testing only Arc semantics)

```bash
cargo test -p silmaril-editor render_loop_reads_selected_entity
```

- [ ] **Step 3: Update `NativeViewport` struct fields (Windows impl)**

Find the `NativeViewport` struct inside `#[cfg(windows)] mod imp`. Add two fields after `world`:

```rust
selected_entity_id: std::sync::Arc<std::sync::Mutex<Option<u64>>>,
gizmo_mode: std::sync::Arc<std::sync::atomic::AtomicU8>,
```

- [ ] **Step 4: Update `NativeViewport::new()` (Windows)**

Change the signature and body of `pub fn new(...)`:

```rust
pub fn new(
    parent_hwnd: HWND,
    world: Arc<std::sync::RwLock<engine_core::World>>,
    selected_entity_id: std::sync::Arc<std::sync::Mutex<Option<u64>>>,
    gizmo_mode: std::sync::Arc<std::sync::atomic::AtomicU8>,
) -> Result<Self, String> {
    tracing::info!(hwnd = ?parent_hwnd, "NativeViewport created for window");
    Ok(Self {
        parent_hwnd: SendHwnd(parent_hwnd),
        instances: Arc::new(Mutex::new(HashMap::new())),
        renderer_thread: None,
        should_stop: Arc::new(AtomicBool::new(false)),
        render_active: Arc::new(AtomicBool::new(true)),
        world,
        selected_entity_id,
        gizmo_mode,
    })
}
```

- [ ] **Step 5: Update `start_rendering()` to clone the new Arcs into the thread**

In `start_rendering()`, after the existing clones, add:

```rust
let selected_entity_id = self.selected_entity_id.clone();
let gizmo_mode = self.gizmo_mode.clone();
```

And update the `render_loop` call to pass them:

```rust
render_loop(hwnd, should_stop, render_active, instances, world, selected_entity_id, gizmo_mode);
```

- [ ] **Step 6: Update `render_loop` signature**

Change `fn render_loop(...)` to:

```rust
fn render_loop(
    hwnd: HWND,
    should_stop: Arc<AtomicBool>,
    render_active: Arc<AtomicBool>,
    instances: Arc<Mutex<HashMap<String, ViewportInstance>>>,
    world: Arc<std::sync::RwLock<engine_core::World>>,
    selected_entity_id: std::sync::Arc<std::sync::Mutex<Option<u64>>>,
    gizmo_mode: std::sync::Arc<std::sync::atomic::AtomicU8>,
) {
```

And inside the loop, before calling `renderer.render_frame(...)`, read the Arc values:

```rust
let selected_id = selected_entity_id.lock().ok().and_then(|g| *g);
let gizmo_mode_val = match gizmo_mode.load(std::sync::atomic::Ordering::Relaxed) {
    1 => crate::viewport::gizmo_pipeline::GizmoMode::Rotate,
    2 => crate::viewport::gizmo_pipeline::GizmoMode::Scale,
    _ => crate::viewport::gizmo_pipeline::GizmoMode::Move,
};
if let Err(e) = renderer.render_frame(&viewports, &world, selected_id, gizmo_mode_val) {
    tracing::error!(error = %e, "render_frame failed");
    std::thread::sleep(std::time::Duration::from_millis(100));
}
```

- [ ] **Step 7: Update `render_frame` signature and body**

Change `fn render_frame(...)` to accept and use the new params:

```rust
fn render_frame(
    &mut self,
    viewports: &[(ViewportBounds, OrbitCamera, bool, bool)],
    _world: &std::sync::RwLock<engine_core::World>,
    selected_entity_id: Option<u64>,
    gizmo_mode: crate::viewport::gizmo_pipeline::GizmoMode,
) -> Result<(), String> {
```

Find the `gizmo_pipeline.record(...)` call (currently at line ~899) which has the `// TODO` comment. Replace the hardcoded values:

```rust
// Before (remove these two lines):
None, // TODO(Task 19): read from NativeViewportState.gizmo_mode and selected entity
crate::viewport::gizmo_pipeline::GizmoMode::Move, // stub

// After:
selected_entity_id,
gizmo_mode,
```

- [ ] **Step 8: Update the non-Windows stub `impl NativeViewport`**

At the bottom of the file (around line 1200), find:

```rust
pub fn new(_parent_hwnd: isize) -> Result<Self, String> {
    Err("Native viewport not yet implemented for this platform".into())
}
```

Change to:

```rust
pub fn new(
    _parent_hwnd: isize,
    _world: std::sync::Arc<std::sync::RwLock<engine_core::World>>,
    _selected_entity_id: std::sync::Arc<std::sync::Mutex<Option<u64>>>,
    _gizmo_mode: std::sync::Arc<std::sync::atomic::AtomicU8>,
) -> Result<Self, String> {
    Err("Native viewport not yet implemented for this platform".into())
}
```

- [ ] **Step 9: Update `create_native_viewport` in `commands.rs`**

Find the `NativeViewport::new(parent_hwnd, world_state.inner().0.clone())` call inside `create_native_viewport`. Update it to pass the Arc clones from `viewport_state`:

```rust
let selected_entity_id = std::sync::Arc::clone(&viewport_state.selected_entity_id);
let gizmo_mode = std::sync::Arc::clone(&viewport_state.gizmo_mode);
let mut vp = NativeViewport::new(
    parent_hwnd,
    world_state.inner().0.clone(),
    selected_entity_id,
    gizmo_mode,
).map_err(|e| { ... })?;
```

- [ ] **Step 10: Verify it compiles**

```bash
cargo build -p silmaril-editor 2>&1 | head -40
```

Expected: no errors (warnings about unused params on the stub are OK).

- [ ] **Step 11: Commit**

```bash
git add engine/editor/src-tauri/viewport/native_viewport.rs engine/editor/src-tauri/bridge/commands.rs
git commit -m "feat(editor): wire selected_entity_id and gizmo_mode Arcs into render thread"
```

---

## Task 3: `sync_transform_to_ecs` helper

**Files:**
- Modify: `engine/editor/src-tauri/bridge/template_commands.rs`

### Background

`sync_transform_to_ecs` reads a Transform from `TemplateState` (Vec-based, not HashMap) and writes it to the live ECS world, then emits `entity-transform-changed`.

**Transform JSON format** (used by both `gizmo_drag_end` when building the `SetComponent` payload and by this reader):

```json
{"position": {"x": 1.0, "y": 2.0, "z": 3.0},
 "rotation": {"x": 0.0, "y": 0.0, "z": 0.0, "w": 1.0},
 "scale":    {"x": 1.0, "y": 1.0, "z": 1.0}}
```

To make `sync_transform_to_ecs` testable without a `tauri::AppHandle`, we split it into:
1. `extract_transform_from_template(entity_id, template_state)` — pure function, returns parsed Vec3/Quat/Vec3
2. `sync_transform_to_ecs(entity_id, template_state, world_state, app)` — calls (1), writes to ECS, emits event

- [ ] **Step 1: Write the failing tests**

Add to `template_commands.rs` (new `#[cfg(test)] mod tests` section):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use engine_ops::template::{TemplateComponent, TemplateEntity, TemplateState};
    use serde_json::json;

    fn make_template_with_transform(entity_id: u64, px: f32, py: f32, pz: f32) -> TemplateState {
        // TemplateComponent stores `data` as a serde_json::Value (json_as_string serde handles
        // serialization to/from YAML). Pass the object directly.
        let component = TemplateComponent {
            type_name: "Transform".to_string(),
            data: json!({
                "position": {"x": px, "y": py, "z": pz},
                "rotation": {"x": 0.0, "y": 0.0, "z": 0.0, "w": 1.0},
                "scale":    {"x": 1.0, "y": 1.0, "z": 1.0}
            }),
        };
        let entity = TemplateEntity {
            id: entity_id,
            name: Some("TestEntity".to_string()),
            components: vec![component],
        };
        let mut state = TemplateState::default();
        state.entities.push(entity);
        state
    }

    #[test]
    fn extract_transform_finds_entity_by_vec_iteration() {
        let ts = make_template_with_transform(1, 5.0, 6.0, 7.0);
        let result = extract_transform_from_template(1, &ts);
        assert!(result.is_some(), "should find entity 1");
        let (pos, _rot, _scl) = result.unwrap();
        assert!((pos.x - 5.0).abs() < 1e-4);
        assert!((pos.y - 6.0).abs() < 1e-4);
        assert!((pos.z - 7.0).abs() < 1e-4);
    }

    #[test]
    fn extract_transform_absent_entity_returns_none() {
        let ts = make_template_with_transform(1, 0.0, 0.0, 0.0);
        let result = extract_transform_from_template(99, &ts);
        assert!(result.is_none(), "entity 99 doesn't exist");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test -p silmaril-editor extract_transform
```

Expected: compile error — `extract_transform_from_template` not defined yet.

- [ ] **Step 3: Add the imports to `template_commands.rs`**

At the top of `template_commands.rs`, add after existing use statements:

```rust
use engine_core::{Entity, Transform};
use engine_ops::template::TemplateState;
use glam::{Quat, Vec3};
```

- [ ] **Step 4: Add `extract_transform_from_template` pure helper**

Add before the `#[cfg(test)]` block:

```rust
/// Extract Transform data from `TemplateState` for the given entity.
///
/// Returns `(position, rotation, scale)` parsed from the stored JSON, or
/// `None` if the entity is absent or has no Transform component.
///
/// `TemplateState.entities` is a `Vec`, not a `HashMap` — use `iter().find()`.
fn extract_transform_from_template(
    entity_id: u64,
    template_state: &TemplateState,
) -> Option<(Vec3, Quat, Vec3)> {
    let data = template_state
        .entities
        .iter()
        .find(|e| e.id == entity_id)
        .and_then(|e| e.components.iter().find(|c| c.type_name == "Transform"))
        .map(|c| &c.data)?;

    let get_f32 = |obj: &serde_json::Value, key: &str| -> f32 {
        obj.get(key).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32
    };

    let pos = data.get("position").unwrap_or(&serde_json::Value::Null);
    let rot = data.get("rotation").unwrap_or(&serde_json::Value::Null);
    let scl = data.get("scale").unwrap_or(&serde_json::Value::Null);

    Some((
        Vec3::new(get_f32(pos, "x"), get_f32(pos, "y"), get_f32(pos, "z")),
        Quat::from_xyzw(
            get_f32(rot, "x"),
            get_f32(rot, "y"),
            get_f32(rot, "z"),
            get_f32(rot, "w"),
        )
        .normalize(),
        Vec3::new(get_f32(scl, "x"), get_f32(scl, "y"), get_f32(scl, "z")),
    ))
}
```

- [ ] **Step 5: Run tests to verify `extract_transform_from_template` passes**

```bash
cargo test -p silmaril-editor extract_transform
```

Expected: both tests pass.

- [ ] **Step 6: Add `sync_transform_to_ecs` and `sync_all_transforms`**

Add after `extract_transform_from_template`:

```rust
/// Write the Transform for `entity_id` from `TemplateState` to the live ECS world
/// and emit `entity-transform-changed`.
///
/// Non-fatal if the entity has no Transform in `TemplateState` — logs a warning
/// and returns `Ok(())`.  Returns `Err` only if `entity_id > u32::MAX` or a
/// world-lock error occurs.
pub(crate) fn sync_transform_to_ecs(
    entity_id: u64,
    template_state: &TemplateState,
    world_state: &crate::state::SceneWorldState,
    app: &tauri::AppHandle,
) -> Result<(), String> {
    use tauri::Emitter;

    if entity_id > u32::MAX as u64 {
        return Err(format!("entity_id {entity_id} exceeds u32::MAX"));
    }

    let Some((pos, rot, scl)) = extract_transform_from_template(entity_id, template_state) else {
        tracing::warn!(entity_id, "sync_transform_to_ecs: no Transform in TemplateState");
        return Ok(()); // non-fatal
    };

    // FIXME: hardcodes generation 0 — will break after entity slot reuse.
    let entity = Entity::new(entity_id as u32, 0);
    {
        let mut world = world_state.0.write().map_err(|e| e.to_string())?;
        if let Some(t) = world.get_mut::<Transform>(entity) {
            t.position = pos;
            t.rotation = rot;
            t.scale = scl;
        } else {
            tracing::warn!(entity_id, "sync_transform_to_ecs: entity not in ECS world");
            return Ok(()); // non-fatal
        }
    }

    app.emit(
        "entity-transform-changed",
        serde_json::json!({
            "id": entity_id,
            "position": {"x": pos.x, "y": pos.y, "z": pos.z},
            "rotation": {"x": rot.x, "y": rot.y, "z": rot.z, "w": rot.w},
            "scale":    {"x": scl.x, "y": scl.y, "z": scl.z},
        }),
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Sync ECS world for every entity in `template_state` that has a Transform.
///
/// Called after template undo/redo since the affected entity_id is not
/// returned by `CommandProcessor::undo()`. Syncing all is safe (idempotent)
/// and correct for the typical small templates used in the editor.
pub(crate) fn sync_all_transforms(
    template_state: &TemplateState,
    world_state: &crate::state::SceneWorldState,
    app: &tauri::AppHandle,
) -> Result<(), String> {
    for entity in &template_state.entities {
        if entity.components.iter().any(|c| c.type_name == "Transform") {
            sync_transform_to_ecs(entity.id, template_state, world_state, app)?;
        }
    }
    Ok(())
}
```

- [ ] **Step 7: Check it compiles**

```bash
cargo build -p silmaril-editor 2>&1 | head -40
```

- [ ] **Step 8: Commit**

```bash
git add engine/editor/src-tauri/bridge/template_commands.rs
git commit -m "feat(editor): add sync_transform_to_ecs and sync_all_transforms helpers"
```

---

## Task 4: Wire ECS sync into template execute/undo/redo wrappers

**Files:**
- Modify: `engine/editor/src-tauri/bridge/template_commands.rs`

### Background

The `#[tauri::command]` wrappers (`template_execute`, `template_undo`, `template_redo`) need two new Tauri State params: `world_state` and `app`. After the inner call succeeds:
- `template_execute`: if `command` is `SetComponent { type_name: "Transform", .. }`, call `sync_transform_to_ecs`
- `template_undo` / `template_redo`: if something was undone/redone (result is `Some`), call `sync_all_transforms`

- [ ] **Step 1: Write the failing test**

Add to the `#[cfg(test)] mod tests` in `template_commands.rs`:

```rust
#[test]
fn extract_transform_scale_parsed_correctly() {
    let ts = make_template_with_transform(1, 0.0, 0.0, 0.0);
    // Override scale
    let data = json!({
        "position": {"x": 0.0, "y": 0.0, "z": 0.0},
        "rotation": {"x": 0.0, "y": 0.0, "z": 0.0, "w": 1.0},
        "scale":    {"x": 2.0, "y": 3.0, "z": 4.0}
    });
    let mut ts2 = TemplateState::default();
    ts2.entities.push(TemplateEntity {
        id: 1,
        name: None,
        components: vec![TemplateComponent { type_name: "Transform".to_string(), data }],
    });
    let (_, _, scl) = extract_transform_from_template(1, &ts2).unwrap();
    assert!((scl.x - 2.0).abs() < 1e-4);
    assert!((scl.y - 3.0).abs() < 1e-4);
    assert!((scl.z - 4.0).abs() < 1e-4);
}
```

- [ ] **Step 2: Run test to verify it passes** (tests logic already implemented in Task 3)

```bash
cargo test -p silmaril-editor extract_transform_scale
```

Expected: passes immediately since `extract_transform_from_template` is already implemented.

- [ ] **Step 3: Update `template_execute` wrapper**

Change the wrapper signature and body:

```rust
#[tauri::command]
pub fn template_execute(
    state: State<'_, Mutex<EditorState>>,
    template_path: String,
    command: TemplateCommand,
    world_state: State<'_, crate::state::SceneWorldState>,
    app: tauri::AppHandle,
) -> Result<CommandResult, IpcError> {
    // Extract entity_id before moving command into inner (TemplateCommand is Clone).
    let transform_entity_id = match &command {
        TemplateCommand::SetComponent { id, type_name, .. } if type_name == "Transform" => {
            Some(*id)
        }
        _ => None,
    };

    let result = template_execute_inner(&state, template_path, command)?;

    if let Some(entity_id) = transform_entity_id {
        sync_transform_to_ecs(entity_id, &result.new_state, &world_state, &app)
            .map_err(|e| IpcError { code: 0, message: e })?;
    }

    Ok(result)
}
```

- [ ] **Step 4: Update `template_undo` wrapper**

```rust
#[tauri::command]
pub fn template_undo(
    state: State<'_, Mutex<EditorState>>,
    template_path: String,
    world_state: State<'_, crate::state::SceneWorldState>,
    app: tauri::AppHandle,
) -> Result<Option<ActionId>, IpcError> {
    let result = template_undo_inner(&state, template_path.clone())?;
    if result.is_some() {
        let guard = state.lock().unwrap();
        let path = std::path::PathBuf::from(&template_path);
        if let Some(proc) = guard.processors.get(&path) {
            sync_all_transforms(proc.state_ref(), &world_state, &app)
                .map_err(|e| IpcError { code: 0, message: e })?;
        }
    }
    Ok(result)
}
```

- [ ] **Step 5: Update `template_redo` wrapper** (same pattern as undo)

```rust
#[tauri::command]
pub fn template_redo(
    state: State<'_, Mutex<EditorState>>,
    template_path: String,
    world_state: State<'_, crate::state::SceneWorldState>,
    app: tauri::AppHandle,
) -> Result<Option<ActionId>, IpcError> {
    let result = template_redo_inner(&state, template_path.clone())?;
    if result.is_some() {
        let guard = state.lock().unwrap();
        let path = std::path::PathBuf::from(&template_path);
        if let Some(proc) = guard.processors.get(&path) {
            sync_all_transforms(proc.state_ref(), &world_state, &app)
                .map_err(|e| IpcError { code: 0, message: e })?;
        }
    }
    Ok(result)
}
```

- [ ] **Step 6: Verify it compiles**

```bash
cargo build -p silmaril-editor 2>&1 | head -40
```

- [ ] **Step 7: Commit**

```bash
git add engine/editor/src-tauri/bridge/template_commands.rs
git commit -m "feat(editor): wire ECS sync into template execute/undo/redo Tauri wrappers"
```

---

## Task 5: Rewrite `gizmo_drag_end`

**Files:**
- Modify: `engine/editor/src-tauri/bridge/gizmo_commands.rs`

### Background

`gizmo_drag_end` currently records to `SceneUndoStack`. We replace that with `template_execute_inner` + `sync_transform_to_ecs`.

Two preparatory changes:
1. Remove `transform_before: crate::state::SerializedTransform` from `DragState` — it was only needed by `SceneAction::SetTransform { before }`. `CommandProcessor` captures before-state internally.
2. Remove the `transform_before` initialisation in `gizmo_hit_test`.

The after-state is the final ECS transform at drag-end time. We build it as a JSON object in the format `sync_transform_to_ecs` expects.

- [ ] **Step 1: Write the failing tests**

Append to the existing `#[cfg(test)] mod tests` in `gizmo_commands.rs`:

```rust
#[test]
fn gizmo_drag_end_empty_path_returns_ok_without_template() {
    // When template_path is empty, gizmo_drag_end_logic returns Ok
    // (ECS already has the final state from drag; only undo history is skipped).
    let after_json = serde_json::json!({
        "position": {"x": 1.0, "y": 0.0, "z": 0.0},
        "rotation": {"x": 0.0, "y": 0.0, "z": 0.0, "w": 1.0},
        "scale":    {"x": 1.0, "y": 1.0, "z": 1.0}
    });
    let result = maybe_record_gizmo_drag("", 1, after_json, &std::sync::Mutex::new(
        crate::bridge::template_commands::EditorState::new()
    ));
    assert!(result.is_ok());
}

#[test]
fn gizmo_drag_end_valid_path_records_to_command_processor() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Create a minimal template YAML
    let mut f = NamedTempFile::new().unwrap();
    writeln!(f, "entities:\n  - id: 1\n    name: Cube\n    components:\n      - type_name: Transform\n        data: '{{\"position\":{{\"x\":0,\"y\":0,\"z\":0}},\"rotation\":{{\"x\":0,\"y\":0,\"z\":0,\"w\":1}},\"scale\":{{\"x\":1,\"y\":1,\"z\":1}}}}'").unwrap();
    let path = f.path().to_str().unwrap().to_string();

    let editor_state = std::sync::Mutex::new(crate::bridge::template_commands::EditorState::new());
    // Open the template so the processor is registered
    crate::bridge::template_commands::template_open_inner(&editor_state, path.clone()).unwrap();

    let after_json = serde_json::json!({
        "position": {"x": 5.0, "y": 0.0, "z": 0.0},
        "rotation": {"x": 0.0, "y": 0.0, "z": 0.0, "w": 1.0},
        "scale":    {"x": 1.0, "y": 1.0, "z": 1.0}
    });
    let result = maybe_record_gizmo_drag(&path, 1, after_json, &editor_state);
    assert!(result.is_ok(), "result: {:?}", result);

    // Verify the action was recorded (history should have one entry)
    let history = crate::bridge::template_commands::template_history_inner(&editor_state, path).unwrap();
    assert_eq!(history.len(), 1, "should have 1 undo entry");
}
```

Note: this test needs `tempfile` crate. Check if it's already a dev-dependency:

```bash
grep "tempfile" engine/editor/Cargo.toml
```

If not present, add to `[dev-dependencies]` in `engine/editor/Cargo.toml`:

```toml
[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test -p silmaril-editor maybe_record_gizmo_drag
```

Expected: compile error — `maybe_record_gizmo_drag` not defined.

- [ ] **Step 3: Remove `transform_before` from `DragState`**

In `gizmo_commands.rs`, remove the `transform_before` field from `DragState`:

```rust
// Remove this field:
/// Transform of the entity at the moment the drag started (for undo).
pub transform_before: crate::state::SerializedTransform,
```

- [ ] **Step 4: Remove `transform_before` initialization in `gizmo_hit_test`**

Find the block in `gizmo_hit_test` that builds `transform_before`:

```rust
// Remove this entire block:
let transform_before = {
    let world = world_state.inner().0.read().ok()?;
    let t = world.get::<engine_core::Transform>(entity)?;
    crate::state::SerializedTransform {
        position: [t.position.x, t.position.y, t.position.z],
        rotation: [t.rotation.x, t.rotation.y, t.rotation.z, t.rotation.w],
        scale: [t.scale.x, t.scale.y, t.scale.z],
    }
};
```

And remove `transform_before` from the `DragState { ... }` struct literal inside `gizmo_hit_test`.

- [ ] **Step 5: Add `maybe_record_gizmo_drag` inner helper**

Add after the existing `pub fn gizmo_drag(...)` command, before `pub fn gizmo_drag_end`:

```rust
/// Inner logic of `gizmo_drag_end` that can be tested without Tauri State.
///
/// Records the final transform in the template's CommandProcessor.
/// If `template_path` is empty, logs a warning and returns `Ok(())` — the ECS
/// already has the final state from the drag; only undo history is skipped.
pub fn maybe_record_gizmo_drag(
    template_path: &str,
    entity_id: u64,
    after_json: serde_json::Value,
    editor_state: &std::sync::Mutex<crate::bridge::template_commands::EditorState>,
) -> Result<(), String> {
    if template_path.is_empty() {
        tracing::warn!("gizmo_drag_end: no active template, undo history not recorded");
        return Ok(());
    }
    crate::bridge::template_commands::template_execute_inner(
        editor_state,
        template_path.to_string(),
        engine_ops::command::TemplateCommand::SetComponent {
            id: entity_id,
            type_name: "Transform".to_string(),
            data: after_json,
        },
    )
    .map_err(|e| e.message)?;
    Ok(())
}
```

- [ ] **Step 6: Rewrite `gizmo_drag_end` Tauri command**

Replace the existing `gizmo_drag_end` command body entirely:

```rust
/// Finalise an active drag: clear [`DragState`], record the final transform as
/// a `SetComponent{Transform}` in the template CommandProcessor, and sync the
/// ECS world so the inspector stays in sync.
#[tauri::command]
pub fn gizmo_drag_end(
    viewport_id: String,
    template_path: String,
    viewport_state: tauri::State<'_, super::commands::NativeViewportState>,
    editor_state: tauri::State<'_, std::sync::Mutex<crate::bridge::template_commands::EditorState>>,
    world_state: tauri::State<'_, crate::state::SceneWorldState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let drag = {
        let mut lock = viewport_state.drag_state.lock().map_err(|e| e.to_string())?;
        lock.take()
    };
    let Some(ds) = drag else {
        return Ok(());
    };
    if ds.viewport_id != viewport_id {
        *viewport_state.drag_state.lock().map_err(|e| e.to_string())? = Some(ds);
        return Ok(());
    }

    debug_assert!(ds.entity_id <= u32::MAX as u64, "entity_id truncation");
    // FIXME: hardcodes generation 0 — will break after entity slot reuse.
    let entity = engine_core::Entity::new(ds.entity_id as u32, 0);

    // Read final transform from ECS (set during drag).
    let after_json = {
        let world = world_state.0.read().map_err(|e| e.to_string())?;
        let t = world
            .get::<engine_core::Transform>(entity)
            .ok_or_else(|| format!("Entity {} not found", ds.entity_id))?;
        serde_json::json!({
            "position": {"x": t.position.x, "y": t.position.y, "z": t.position.z},
            "rotation": {"x": t.rotation.x, "y": t.rotation.y, "z": t.rotation.z, "w": t.rotation.w},
            "scale":    {"x": t.scale.x,    "y": t.scale.y,    "z": t.scale.z},
        })
    };

    maybe_record_gizmo_drag(&template_path, ds.entity_id, after_json.clone(), &editor_state)?;

    // Sync TemplateState → ECS (emit entity-transform-changed so inspector stays live).
    if !template_path.is_empty() {
        let guard = editor_state.lock().map_err(|e| e.to_string())?;
        let path = std::path::PathBuf::from(&template_path);
        if let Some(proc) = guard.processors.get(&path) {
            crate::bridge::template_commands::sync_transform_to_ecs(
                ds.entity_id,
                proc.state_ref(),
                &world_state,
                &app,
            )?;
        }
    }

    tracing::debug!(entity_id = ds.entity_id, template_path = %template_path, "gizmo_drag_end: SetComponent recorded");
    Ok(())
}
```

- [ ] **Step 7: Run the tests**

```bash
cargo test -p silmaril-editor maybe_record_gizmo_drag gizmo_drag_end
```

Expected: both tests pass.

- [ ] **Step 8: Verify the whole crate still compiles**

```bash
cargo build -p silmaril-editor 2>&1 | head -40
```

- [ ] **Step 9: Commit**

```bash
git add engine/editor/src-tauri/bridge/gizmo_commands.rs engine/editor/Cargo.toml
git commit -m "feat(editor): rewrite gizmo_drag_end to record via template CommandProcessor"
```

---

## Task 6: Delete `SceneUndoStack`

**Files:**
- Delete: `engine/editor/src-tauri/state/scene_undo.rs`
- Modify: `engine/editor/src-tauri/state/mod.rs`
- Modify: `engine/editor/src-tauri/bridge/commands.rs`
- Modify: `engine/editor/src-tauri/lib.rs`

### Background

`SceneUndoStack`, `SceneAction`, `SerializedTransform`, `scene_undo`, `scene_redo`, and `apply_scene_action` are all dead code after Tasks 3–5. We delete the file and all references.

- [ ] **Step 1: Delete `state/scene_undo.rs`**

```bash
rm engine/editor/src-tauri/state/scene_undo.rs
```

- [ ] **Step 2: Update `state/mod.rs`**

Remove `pub mod scene_undo;` and the re-export line. The file should become:

```rust
pub mod editor;
pub mod scene_world;

pub use editor::{EditorMode, EditorState};
pub use scene_world::SceneWorldState;
```

- [ ] **Step 3: Remove `scene_undo`/`scene_redo`/`apply_scene_action` from `commands.rs`**

Find and delete the three functions (lines ~1230–1327):
- `pub fn scene_undo(...)` and its body
- `pub fn scene_redo(...)` and its body
- `fn apply_scene_action(...)` and its body
- The `// Scene undo / redo IPC` section comment

Also delete any `use crate::state::SceneAction` or similar imports that are now dead.

- [ ] **Step 4: Remove managed state and invoke_handler entries from `lib.rs`**

In `lib.rs`:
1. Remove `.manage(std::sync::Mutex::new(crate::state::SceneUndoStack::new()))` from the builder chain
2. Remove `commands::scene_undo,` from `invoke_handler!`
3. Remove `commands::scene_redo,` from `invoke_handler!`

- [ ] **Step 5: Verify it compiles with no errors**

```bash
cargo build -p silmaril-editor 2>&1 | head -40
```

Expected: no errors. If there are dangling references to `SerializedTransform`, `SceneAction`, or `SceneUndoStack`, find and remove them.

- [ ] **Step 6: Run all Rust tests**

```bash
cargo test -p silmaril-editor 2>&1 | tail -20
```

Expected: all tests pass.

- [ ] **Step 7: Commit**

```bash
git add engine/editor/src-tauri/state/ engine/editor/src-tauri/bridge/commands.rs engine/editor/src-tauri/lib.rs
git commit -m "feat(editor): delete SceneUndoStack; scene undo now routed through template CommandProcessor"
```

---

## Task 7: Frontend — selection wiring

**Files:**
- Modify: `engine/editor/src/lib/stores/editor-context.ts`
- Modify: `engine/editor/src/lib/api.ts`
- Modify: `engine/editor/src/lib/docking/panels/HierarchyWrapper.svelte`

### Background

When the user clicks an entity in the hierarchy, `setSelectedEntityId(id)` is called (already exists). We need to mirror this to the Rust side via `set_selected_entity`. We add:
1. `getSelectedEntityId()` to `editor-context.ts` (convenience wrapper)
2. `setSelectedEntity(id)` to `api.ts` (IPC wrapper)
3. A `$effect` in `HierarchyWrapper.svelte` that calls `setSelectedEntity` on every selection change

- [ ] **Step 1: Write the failing TS test**

Create `engine/editor/src/lib/stores/editor-context.test.ts`:

```typescript
import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('$lib/scene/state', () => ({
  getSceneState: vi.fn(() => ({ entities: [], selectedEntityId: null })),
  getSelectedEntity: vi.fn(() => null),
  subscribeScene: vi.fn(() => () => {}),
}));
vi.mock('$lib/scene/commands', () => ({
  selectEntity: vi.fn(),
  populateFromScan: vi.fn(),
}));

describe('editor-context — getSelectedEntityId', () => {
  it('returns null when nothing is selected', async () => {
    const { getSelectedEntityId } = await import('$lib/stores/editor-context');
    expect(getSelectedEntityId()).toBeNull();
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd engine/editor && npm run test -- src/lib/stores/editor-context.test.ts
```

Expected: compile/import error — `getSelectedEntityId` not exported.

- [ ] **Step 3: Add `getSelectedEntityId` to `editor-context.ts`**

Add after the existing `getEditorContext()` function:

```typescript
/** Get the id of the currently selected entity (or null). */
export function getSelectedEntityId(): number | null {
  return getSceneState().selectedEntityId;
}
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cd engine/editor && npm run test -- src/lib/stores/editor-context.test.ts
```

- [ ] **Step 5: Add `setSelectedEntity` to `api.ts`**

Find where `gizmoDragEnd` is defined (~line 341). Add before it:

```typescript
/** Mirror the selected entity to the Rust viewport renderer. */
export async function setSelectedEntity(entityId: number | null): Promise<void> {
  return tauriInvoke<void>('set_selected_entity', { entityId });
}
```

Also add a browser mock inside `browserMock`:

```typescript
case 'set_selected_entity':
  return undefined as T;
```

- [ ] **Step 6: Add `$effect` to `HierarchyWrapper.svelte`**

The current file uses `onMount` for subscriptions. Add a `$effect` using the Svelte 5 pattern alongside the existing code.

Update the `<script lang="ts">` block:

```typescript
<script lang="ts">
  import { onMount } from 'svelte';
  import HierarchyPanel from '$lib/components/HierarchyPanel.svelte';
  import {
    getEditorContext,
    getSelectedEntityId,
    setSelectedEntityId,
    subscribeContext,
  } from '$lib/stores/editor-context';
  import { setSelectedEntity } from '$lib/api';

  let entities = $state(getEditorContext().entities);
  let selectedId = $state(getEditorContext().selectedEntityId);

  onMount(() => {
    return subscribeContext(() => {
      const ctx = getEditorContext();
      entities = ctx.entities;
      selectedId = ctx.selectedEntityId;
    });
  });

  // Mirror selection to the Rust viewport renderer.
  $effect(() => {
    return subscribeContext(() => {
      setSelectedEntity(getSelectedEntityId()).catch((e) => {
        console.warn('[silmaril] setSelectedEntity failed:', e);
      });
    });
  });

  function handleSelect(id: number) {
    setSelectedEntityId(id);
  }
</script>
```

- [ ] **Step 7: Run TS tests**

```bash
cd engine/editor && npm run test -- src/lib/stores/editor-context.test.ts
```

Expected: passes.

- [ ] **Step 8: Commit**

```bash
git add engine/editor/src/lib/stores/editor-context.ts engine/editor/src/lib/api.ts engine/editor/src/lib/docking/panels/HierarchyWrapper.svelte
git commit -m "feat(editor): wire selectedEntityId to set_selected_entity IPC on selection change"
```

---

## Task 8: Frontend — undo routing + drag end path

**Files:**
- Modify: `engine/editor/src/lib/stores/undo-history.ts`
- Modify: `engine/editor/src/lib/stores/undo-history.test.ts`
- Modify: `engine/editor/src/lib/api.ts`
- Modify: `engine/editor/src/lib/docking/panels/ViewportPanel.svelte`

### Background

`sceneUndo()` and `sceneRedo()` currently call `scene_undo`/`scene_redo` IPC (now deleted). They must now call `template.undo`/`template.redo` with the active template path, mirroring the existing `undo()`/`redo()` functions but with the inflight guard preserved. Remove dead `_sceneCanUndo`/`_sceneCanRedo` state and their getters.

`gizmoDragEnd` in `api.ts` needs a `templatePath: string` parameter. `ViewportPanel.svelte` imports `getActiveTemplatePath` and passes it.

`undo-history.test.ts` currently mocks `sceneUndo`/`sceneRedo` from `$lib/api` — remove that stale mock since those imports no longer exist.

- [ ] **Step 1: Write the failing tests for `sceneUndo`/`sceneRedo`**

Add to `undo-history.test.ts`:

```typescript
describe('undo-history — sceneUndo()', () => {
  beforeEach(() => {
    vi.resetModules();
    setupMocks();
  });
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('sceneUndo is a no-op when no template is active', async () => {
    const store = await loadStore();
    await store.sceneUndo();
    // No call to template.undo expected since no template active
    const undoCalls = mockRunCommand.mock.calls.filter(
      (call) => call[0] === 'template.undo'
    );
    expect(undoCalls).toHaveLength(0);
  });

  it('sceneUndo calls template.undo with active template path', async () => {
    const store = await loadStore();
    mockRunCommand.mockResolvedValueOnce(ok([]));       // setActiveTemplate → history
    await store.setActiveTemplate('/tmp/hero.yaml');
    mockRunCommand.mockResolvedValueOnce(ok(null));     // template.undo → null (nothing to undo)
    mockRunCommand.mockResolvedValueOnce(ok([]));       // _refreshState → history
    await store.sceneUndo();
    expect(mockRunCommand).toHaveBeenCalledWith('template.undo', { template_path: '/tmp/hero.yaml' });
  });

  it('sceneUndo calls _refreshState after (notifies subscribers)', async () => {
    const store = await loadStore();
    mockRunCommand.mockResolvedValueOnce(ok([]));
    await store.setActiveTemplate('/tmp/hero.yaml');
    const listener = vi.fn();
    store.subscribeUndoHistory(listener);
    listener.mockClear();
    mockRunCommand.mockResolvedValueOnce(ok(null));
    mockRunCommand.mockResolvedValueOnce(ok([]));
    await store.sceneUndo();
    expect(listener).toHaveBeenCalled(); // _refreshState triggers notify
  });
});

describe('undo-history — sceneRedo()', () => {
  beforeEach(() => {
    vi.resetModules();
    setupMocks();
  });
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('sceneRedo is a no-op when no template is active', async () => {
    const store = await loadStore();
    await store.sceneRedo();
    const redoCalls = mockRunCommand.mock.calls.filter(
      (call) => call[0] === 'template.redo'
    );
    expect(redoCalls).toHaveLength(0);
  });

  it('sceneRedo calls template.redo with active template path', async () => {
    const store = await loadStore();
    mockRunCommand.mockResolvedValueOnce(ok([]));
    await store.setActiveTemplate('/tmp/hero.yaml');
    mockRunCommand.mockResolvedValueOnce(ok(null));
    mockRunCommand.mockResolvedValueOnce(ok([]));
    await store.sceneRedo();
    expect(mockRunCommand).toHaveBeenCalledWith('template.redo', { template_path: '/tmp/hero.yaml' });
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd engine/editor && npm run test -- src/lib/stores/undo-history.test.ts 2>&1 | tail -30
```

Expected: some tests fail because `sceneUndo` still calls old `sceneUndo` from api.ts.

- [ ] **Step 3: Update `undo-history.ts` — remove dead scene undo state and rewrite functions**

In `undo-history.ts`:

a) Remove these imports:
```typescript
import { sceneUndo as _sceneUndo, sceneRedo as _sceneRedo } from '$lib/api';
```

b) Remove these module-level variables:
```typescript
// Remove:
let _sceneCanUndo = false;
let _sceneCanRedo = false;
```

c) Remove these exported functions:
```typescript
// Remove:
export function getSceneCanUndo(): boolean { return _sceneCanUndo; }
export function getSceneCanRedo(): boolean { return _sceneCanRedo; }
```

d) Replace `sceneUndo()` and `sceneRedo()` with:

```typescript
/** Undo the last transform change, routed through the active template. */
export async function sceneUndo(): Promise<void> {
  if (_sceneUndoRedoInFlight) return;
  const path = getActiveTemplatePath();
  if (!path) return;
  _sceneUndoRedoInFlight = true;
  try {
    await commands.runCommand('template.undo', { template_path: path });
    await _refreshState();
  } catch (e) {
    logError(`Scene undo failed: ${e}`);
  } finally {
    _sceneUndoRedoInFlight = false;
  }
}

/** Redo the last undone transform change, routed through the active template. */
export async function sceneRedo(): Promise<void> {
  if (_sceneUndoRedoInFlight) return;
  const path = getActiveTemplatePath();
  if (!path) return;
  _sceneUndoRedoInFlight = true;
  try {
    await commands.runCommand('template.redo', { template_path: path });
    await _refreshState();
  } catch (e) {
    logError(`Scene redo failed: ${e}`);
  } finally {
    _sceneUndoRedoInFlight = false;
  }
}
```

- [ ] **Step 4: Update `undo-history.test.ts` — remove stale mock**

In the `setupMocks()` function, find and remove:

```typescript
// Remove this entire doMock block:
vi.doMock('$lib/api', () => ({
  sceneUndo: vi.fn().mockResolvedValue({ canUndo: false, canRedo: false }),
  sceneRedo: vi.fn().mockResolvedValue({ canUndo: false, canRedo: false }),
}));
```

- [ ] **Step 5: Run undo-history tests to verify they pass**

```bash
cd engine/editor && npm run test -- src/lib/stores/undo-history.test.ts
```

Expected: all tests pass (including the new `sceneUndo`/`sceneRedo` tests and all existing tests).

- [ ] **Step 6: Update `api.ts` — remove `sceneUndo`/`sceneRedo`, update `gizmoDragEnd`**

a) Remove these two functions:
```typescript
// Remove:
export async function sceneUndo(): Promise<UndoRedoState> { ... }
export async function sceneRedo(): Promise<UndoRedoState> { ... }
// Remove the UndoRedoState interface if it's only used by these two functions
```

b) Update `gizmoDragEnd` signature:
```typescript
// Before:
export async function gizmoDragEnd(viewportId: string): Promise<void> {
  return tauriInvoke<void>('gizmo_drag_end', { viewportId });
}

// After:
export async function gizmoDragEnd(viewportId: string, templatePath: string): Promise<void> {
  return tauriInvoke<void>('gizmo_drag_end', { viewportId, templatePath });
}
```

c) Add browser mock for `gizmo_drag_end` if not already present:
```typescript
case 'gizmo_drag_end':
  return undefined as T;
```

- [ ] **Step 7: Update `ViewportPanel.svelte`**

Find the import of `gizmoDragEnd`:
```typescript
import { gizmoDragEnd, ... } from '$lib/api';
```

Add import of `getActiveTemplatePath`:
```typescript
import { getActiveTemplatePath } from '$lib/stores/undo-history';
```

There are **two** call sites for `gizmoDragEnd` in `ViewportPanel.svelte`. Update both to pass the template path:

**Call site 1** — inside `handleWindowPointerUp` (a `window.addEventListener('pointerup', ...)` handler in `onMount`, around line 226):

```typescript
// Before:
gizmoDragEnd(viewportId).catch(err => console.error('gizmo_drag_end failed:', err));
// After:
const path = getActiveTemplatePath() ?? '';
gizmoDragEnd(viewportId, path).catch(err => console.error('gizmo_drag_end failed:', err));
```

**Call site 2** — inside `handleMouseUp` (the component's own mouse handler, around line 457):

```typescript
// Before:
await gizmoDragEnd(viewportId);
// After:
const path = getActiveTemplatePath() ?? '';
await gizmoDragEnd(viewportId, path);
```

- [ ] **Step 8: Run all frontend tests**

```bash
cd engine/editor && npm run test
```

Expected: all tests pass.

- [ ] **Step 9: Run all Rust tests one final time**

```bash
cargo test -p silmaril-editor
```

Expected: all tests pass.

- [ ] **Step 10: Commit**

```bash
git add engine/editor/src/lib/stores/undo-history.ts engine/editor/src/lib/stores/undo-history.test.ts engine/editor/src/lib/api.ts engine/editor/src/lib/docking/panels/ViewportPanel.svelte
git commit -m "feat(editor): route sceneUndo/Redo through template pipeline; pass templatePath to gizmoDragEnd"
```

---

## Final Verification

After all 8 tasks are complete, run the full test suite:

```bash
# Rust
cargo test -p silmaril-editor

# TypeScript
cd engine/editor && npm run test
```

Both should be green. Then invoke `superpowers:finishing-a-development-branch` to complete the work.
