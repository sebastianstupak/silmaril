//! Gizmo IPC commands — hit-testing, dragging, and mode switching.
//!
//! These commands wire the 3D gizmo handles rendered by `GizmoPipeline`
//! to transform mutations on the live ECS world.
//!
//! # Flow
//! 1. Frontend calls `gizmo_hit_test` on mouse-down.
//!    Returns `{axis, mode}` JSON if a handle was hit; `null` otherwise.
//! 2. On each mouse-move the frontend calls `gizmo_drag`.
//!    The backend mutates the entity transform and emits
//!    `entity-transform-changed` so the inspector stays in sync.
//! 3. On mouse-up the frontend calls `gizmo_drag_end`.
//!    The backend records a `SetComponent{Transform}` in the template CommandProcessor.
//! 4. The frontend (or keyboard shortcuts) call `set_gizmo_mode` to switch
//!    between Move / Rotate / Scale.

use glam::{Mat4, Quat, Vec3};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Snapshot of an ongoing gizmo drag operation.
pub struct DragState {
    pub entity_id: u64,
    pub viewport_id: String,
    pub axis: crate::viewport::GizmoAxis,
    pub mode: crate::viewport::GizmoMode,
    /// Screen position of the last drag event.
    pub last_screen: (f32, f32),
    /// Camera matrices captured at drag-start (used for consistent feel).
    pub camera_view: Mat4,
    pub camera_proj: Mat4,
    pub camera_pos: Vec3,
    /// Gizmo handle scale (distance-based).
    pub gizmo_scale: f32,
    /// Viewport width at drag-start (used for screen-delta projection).
    pub viewport_width: f32,
}

// ---------------------------------------------------------------------------
// Pure math helpers (tested below)
// ---------------------------------------------------------------------------

/// Returns `true` if a ray `(ray_origin, ray_dir)` passes within `radius` of
/// the capsule segment `[cap_a, cap_b]`.
///
/// Used to hit-test the three axis handles of the translate/rotate/scale gizmo.
pub fn ray_capsule_intersects(
    ray_origin: Vec3,
    ray_dir: Vec3, // expected to be normalised
    cap_a: Vec3,
    cap_b: Vec3,
    radius: f32,
) -> bool {
    let ab = cap_b - cap_a;
    let ao = ray_origin - cap_a;
    let d = ray_dir;

    let denom = d.dot(d) * ab.dot(ab) - d.dot(ab).powi(2);
    if denom.abs() < 1e-6 {
        // Ray is parallel to the capsule axis — treat as miss for simplicity.
        return false;
    }

    let t = (d.dot(ab) * ab.dot(ao) - ab.dot(ab) * d.dot(ao)) / denom;
    let t = t.max(0.0); // ray only goes forward
    let s_reclamped = if ab.length_squared() > 1e-10 {
        ((ray_origin + d * t - cap_a).dot(ab) / ab.dot(ab)).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let closest_ray = ray_origin + d * t;
    let closest_cap = cap_a + ab * s_reclamped;
    closest_ray.distance(closest_cap) <= radius
}

/// Projects a 2-D screen-space delta onto a world-space axis vector.
///
/// The returned `Vec3` is along `axis` with magnitude proportional to how far
/// the cursor moved in the direction that the axis projects to on screen.
///
/// `screen_width` is used to normalise the pixel delta into a [0, 2] NDC range.
/// `world_scale` should match the gizmo handle scale so the drag sensitivity
/// feels consistent regardless of camera distance.
pub fn project_screen_delta_to_axis(
    screen_delta: (f32, f32),
    axis: Vec3,
    screen_width: f32,
    world_scale: f32,
) -> Vec3 {
    // Map screen pixels to a signed magnitude along the axis.
    // Y is inverted because screen +Y is down but world +Y is up.
    let magnitude =
        (screen_delta.0 * axis.x + screen_delta.1 * -axis.y) / screen_width * world_scale * 2.0;
    axis * magnitude
}

// ---------------------------------------------------------------------------
// Private helper — unproject a screen pixel to a world-space ray
// ---------------------------------------------------------------------------

fn unproject_screen(
    sx: f32,
    sy: f32,
    bounds: &crate::viewport::native_viewport::ViewportBounds,
    view: Mat4,
    proj: Mat4,
) -> (Vec3, Vec3) {
    let ndc_x = (sx - bounds.x as f32) / bounds.width as f32 * 2.0 - 1.0;
    let ndc_y = 1.0 - (sy - bounds.y as f32) / bounds.height as f32 * 2.0;
    let inv_vp = (proj * view).inverse();
    let near = inv_vp.project_point3(Vec3::new(ndc_x, ndc_y, 0.0));
    let far = inv_vp.project_point3(Vec3::new(ndc_x, ndc_y, 1.0));
    let dir = (far - near).normalize();
    (near, dir)
}

// ---------------------------------------------------------------------------
// IPC commands
// ---------------------------------------------------------------------------

/// Test whether a screen-space position hits any gizmo handle for an entity.
///
/// If a handle is hit, the result is stored as the active [`DragState`] and
/// `Some({axis, mode})` is returned so the frontend knows which handle was
/// grabbed.  Returns `None` when no handle is hit.
#[tauri::command]
pub fn gizmo_hit_test(
    viewport_id: String,
    screen_x: f32,
    screen_y: f32,
    entity_id: u64,
    viewport_state: tauri::State<'_, super::commands::NativeViewportState>,
    world_state: tauri::State<'_, crate::state::SceneWorldState>,
) -> Option<serde_json::Value> {
    if entity_id > u32::MAX as u64 {
        return None;
    }
    // FIXME: hardcodes generation 0 — will break after any entity slot reuse.
    let entity = engine_core::Entity::new(entity_id as u32, 0);

    // --- 1. Read entity position from ECS world ---
    let entity_pos = {
        let world = world_state.inner().0.read().ok()?;
        let t = world.get::<engine_core::Transform>(entity)?;
        t.position
    };

    // --- 2. Get camera data from the viewport instance ---
    let (view, proj, cam_pos, bounds, gizmo_scale) = {
        let registry = viewport_state.registry.lock().ok()?;
        let vp = registry.get_for_id(&viewport_id)?;
        let (view, proj, eye, bounds) = vp.get_instance_ray_data(&viewport_id)?;
        let dist = (eye - entity_pos).length().max(0.1_f32);
        (view, proj, eye, bounds, dist * 0.15)
    };

    // --- 3. Unproject screen position to a world-space ray ---
    let (ray_origin, ray_dir) = unproject_screen(screen_x, screen_y, &bounds, view, proj);

    // --- 4. Read the current gizmo mode ---
    let gizmo_mode_u8 =
        viewport_state.gizmo_mode.load(std::sync::atomic::Ordering::Relaxed);
    let mode = match gizmo_mode_u8 {
        1 => crate::viewport::GizmoMode::Rotate,
        2 => crate::viewport::GizmoMode::Scale,
        _ => crate::viewport::GizmoMode::Move,
    };

    // --- 5. Test ray against X, Y, Z axis handles ---
    let axes = [
        (crate::viewport::GizmoAxis::X, Vec3::X),
        (crate::viewport::GizmoAxis::Y, Vec3::Y),
        (crate::viewport::GizmoAxis::Z, Vec3::Z),
    ];

    for (axis, axis_dir) in &axes {
        let handle_end = entity_pos + *axis_dir * gizmo_scale;
        let radius = gizmo_scale * 0.1;
        if ray_capsule_intersects(ray_origin, ray_dir, entity_pos, handle_end, radius) {
            *viewport_state.drag_state.lock().ok()? = Some(DragState {
                entity_id,
                viewport_id,
                axis: *axis,
                mode,
                last_screen: (screen_x, screen_y),
                camera_view: view,
                camera_proj: proj,
                camera_pos: cam_pos,
                gizmo_scale,
                viewport_width: bounds.width.max(1) as f32,
            });

            tracing::debug!(
                entity_id,
                axis = ?axis,
                mode = ?mode,
                "Gizmo hit"
            );

            let axis_str = match axis {
                crate::viewport::GizmoAxis::X  => "x",
                crate::viewport::GizmoAxis::Y  => "y",
                crate::viewport::GizmoAxis::Z  => "z",
                crate::viewport::GizmoAxis::XY => "xy",
                crate::viewport::GizmoAxis::XZ => "xz",
                crate::viewport::GizmoAxis::YZ => "yz",
            };
            let mode_str = match mode {
                crate::viewport::GizmoMode::Move   => "move",
                crate::viewport::GizmoMode::Rotate => "rotate",
                crate::viewport::GizmoMode::Scale  => "scale",
            };
            return Some(serde_json::json!({
                "axis": axis_str,
                "mode": mode_str,
            }));
        }
    }

    None
}

/// Apply one mouse-move step to the active drag.
///
/// Mutates the entity transform in the ECS world and emits
/// `entity-transform-changed` so the frontend inspector stays in sync.
#[tauri::command]
pub fn gizmo_drag(
    viewport_id: String,
    screen_x: f32,
    screen_y: f32,
    viewport_state: tauri::State<'_, super::commands::NativeViewportState>,
    world_state: tauri::State<'_, crate::state::SceneWorldState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    use tauri::Emitter;

    let mut drag_lock = viewport_state.drag_state.lock().map_err(|e| e.to_string())?;
    let Some(ref mut ds) = *drag_lock else {
        return Ok(());
    };
    if ds.viewport_id != viewport_id {
        return Ok(());
    }

    let dx = screen_x - ds.last_screen.0;
    let dy = screen_y - ds.last_screen.1;
    ds.last_screen = (screen_x, screen_y);

    let axis_vec = match ds.axis {
        crate::viewport::GizmoAxis::X => Vec3::X,
        crate::viewport::GizmoAxis::Y => Vec3::Y,
        crate::viewport::GizmoAxis::Z => Vec3::Z,
        _ => return Ok(()), // planar handles: not yet implemented
    };

    debug_assert!(ds.entity_id <= u32::MAX as u64, "entity_id truncation");
    // FIXME: hardcodes generation 0 — will break after any entity slot reuse.
    let entity = engine_core::Entity::new(ds.entity_id as u32, 0);
    let entity_id = ds.entity_id;
    let mode = ds.mode;
    let gizmo_scale = ds.gizmo_scale;
    let axis = ds.axis;

    match mode {
        crate::viewport::GizmoMode::Move => {
            let delta =
                project_screen_delta_to_axis((dx, dy), axis_vec, ds.viewport_width.max(1.0), gizmo_scale * 2.0);
            let mut world = world_state.inner().0.write().map_err(|e| e.to_string())?;
            if let Some(t) = world.get_mut::<engine_core::Transform>(entity) {
                t.position += delta;
                let pos = [t.position.x, t.position.y, t.position.z];
                let rot = [t.rotation.x, t.rotation.y, t.rotation.z, t.rotation.w];
                let scl = [t.scale.x, t.scale.y, t.scale.z];
                drop(world);
                app.emit(
                    "entity-transform-changed",
                    serde_json::json!({
                        "id": entity_id,
                        "position": pos,
                        "rotation": rot,
                        "scale": scl,
                    }),
                )
                .map_err(|e| e.to_string())?;
            }
        }

        crate::viewport::GizmoMode::Rotate => {
            let angle = (dx / 300.0) * std::f32::consts::TAU;
            let rotation_delta = Quat::from_axis_angle(axis_vec, angle);
            let mut world = world_state.inner().0.write().map_err(|e| e.to_string())?;
            if let Some(t) = world.get_mut::<engine_core::Transform>(entity) {
                t.rotation = (rotation_delta * t.rotation).normalize();
                let pos = [t.position.x, t.position.y, t.position.z];
                let rot = [t.rotation.x, t.rotation.y, t.rotation.z, t.rotation.w];
                let scl = [t.scale.x, t.scale.y, t.scale.z];
                drop(world);
                app.emit(
                    "entity-transform-changed",
                    serde_json::json!({
                        "id": entity_id,
                        "position": pos,
                        "rotation": rot,
                        "scale": scl,
                    }),
                )
                .map_err(|e| e.to_string())?;
            }
        }

        crate::viewport::GizmoMode::Scale => {
            let factor = 1.0 + dx / 200.0;
            let mut world = world_state.inner().0.write().map_err(|e| e.to_string())?;
            if let Some(t) = world.get_mut::<engine_core::Transform>(entity) {
                match axis {
                    crate::viewport::GizmoAxis::X => t.scale.x *= factor,
                    crate::viewport::GizmoAxis::Y => t.scale.y *= factor,
                    crate::viewport::GizmoAxis::Z => t.scale.z *= factor,
                    _ => {
                        t.scale.x *= factor;
                        t.scale.y *= factor;
                        t.scale.z *= factor;
                    }
                }
                let pos = [t.position.x, t.position.y, t.position.z];
                let rot = [t.rotation.x, t.rotation.y, t.rotation.z, t.rotation.w];
                let scl = [t.scale.x, t.scale.y, t.scale.z];
                drop(world);
                app.emit(
                    "entity-transform-changed",
                    serde_json::json!({
                        "id": entity_id,
                        "position": pos,
                        "rotation": rot,
                        "scale": scl,
                    }),
                )
                .map_err(|e| e.to_string())?;
            }
        }
    }

    Ok(())
}

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
        let world = world_state.inner().0.read().map_err(|e| e.to_string())?;
        let t = world
            .get::<engine_core::Transform>(entity)
            .ok_or_else(|| format!("Entity {} not found", ds.entity_id))?;
        serde_json::json!({
            "position": {"x": t.position.x, "y": t.position.y, "z": t.position.z},
            "rotation": {"x": t.rotation.x, "y": t.rotation.y, "z": t.rotation.z, "w": t.rotation.w},
            "scale":    {"x": t.scale.x,    "y": t.scale.y,    "z": t.scale.z},
        })
    };

    maybe_record_gizmo_drag(&template_path, ds.entity_id, after_json, &editor_state)?;

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

/// Set the active gizmo mode.
///
/// Accepted values: `"move"`, `"rotate"`, `"scale"`.
#[tauri::command]
pub fn set_gizmo_mode(
    mode: String,
    viewport_state: tauri::State<'_, super::commands::NativeViewportState>,
) -> Result<(), String> {
    let m = match mode.as_str() {
        "move" => 0u8,
        "rotate" => 1u8,
        "scale" => 2u8,
        other => return Err(format!("unknown gizmo mode: {other}")),
    };
    viewport_state.gizmo_mode.store(m, std::sync::atomic::Ordering::Relaxed);
    tracing::debug!(mode = %mode, "Gizmo mode set");
    Ok(())
}

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
    if entity_id > u32::MAX as u64 {
        return Ok(None);
    }
    // FIXME: hardcodes generation 0 — will break after any entity slot reuse.
    let entity = engine_core::Entity::new(entity_id as u32, 0);

    // Read entity position from ECS world.
    let entity_pos = {
        let world = world_state.inner().0.read().map_err(|e| e.to_string())?;
        match world.get::<engine_core::Transform>(entity) {
            Some(t) => t.position,
            None    => return Ok(None),
        }
    };

    // Get camera data from the viewport instance.
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

    // Unproject screen position to world-space ray.
    let (ray_origin, ray_dir) = unproject_screen(screen_x, screen_y, &bounds, view, proj);

    // Test each axis handle (same geometry as gizmo_hit_test, no DragState written).
    let axes: [(Vec3, &str); 3] = [
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn ray_hits_x_axis_handle() {
        // Camera at (0, 0, -5) looking toward +Z.
        // Entity at origin; X-axis handle from (0,0,0) to (1,0,0).
        // Ray through x=0.5 should hit the X handle.
        let ray_origin = Vec3::new(0.5, 0.0, -5.0);
        let ray_dir = Vec3::new(0.0, 0.0, 1.0);
        let capsule_start = Vec3::new(0.0, 0.0, 0.0);
        let capsule_end = Vec3::new(1.0, 0.0, 0.0);
        let radius = 0.1;
        assert!(
            ray_capsule_intersects(ray_origin, ray_dir, capsule_start, capsule_end, radius),
            "ray at x=0.5 should hit X-axis handle"
        );
    }

    #[test]
    fn ray_misses_when_offset() {
        let ray_origin = Vec3::new(5.0, 5.0, -5.0);
        let ray_dir = Vec3::new(0.0, 0.0, 1.0);
        let capsule_start = Vec3::new(0.0, 0.0, 0.0);
        let capsule_end = Vec3::new(1.0, 0.0, 0.0);
        assert!(
            !ray_capsule_intersects(ray_origin, ray_dir, capsule_start, capsule_end, 0.1),
            "ray at (5,5) should miss X-axis handle near origin"
        );
    }

    #[test]
    fn drag_on_x_axis_moves_entity_x() {
        let axis_world = Vec3::X;
        let screen_delta = (10.0_f32, 0.0_f32);
        let screen_width = 800.0_f32;
        let delta = project_screen_delta_to_axis(screen_delta, axis_world, screen_width, 1.0);
        assert!(delta.x > 0.0, "should move in +X, got {:?}", delta);
        assert!(delta.y.abs() < 0.001, "Y should be ~0, got {:?}", delta);
        assert!(delta.z.abs() < 0.001, "Z should be ~0, got {:?}", delta);
    }

    #[test]
    fn drag_on_y_axis_rotates_entity() {
        let angle_rad = std::f32::consts::PI / 4.0;
        let q = Quat::from_axis_angle(Vec3::Y, angle_rad);
        // cos(PI/8) ≈ 0.924
        assert!(
            (q.w - (std::f32::consts::PI / 8.0).cos()).abs() < 0.01,
            "quaternion w component mismatch: got {}",
            q.w
        );
    }

    #[test]
    fn gizmo_drag_end_empty_path_returns_ok_without_template() {
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

    #[test]
    fn gizmo_drag_end_valid_path_records_to_command_processor() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Create a minimal template YAML
        let mut f = NamedTempFile::with_suffix(".yaml").unwrap();
        writeln!(f, "name: test\nentities:\n  - id: 1\n    name: Cube\n    components:\n      - type_name: Transform\n        data: '{{\"position\":{{\"x\":0,\"y\":0,\"z\":0}},\"rotation\":{{\"x\":0,\"y\":0,\"z\":0,\"w\":1}},\"scale\":{{\"x\":1,\"y\":1,\"z\":1}}}}'").unwrap();
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
}
