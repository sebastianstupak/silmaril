//! Tauri IPC handlers for template CRUD and undo/redo.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use engine_core::{Entity, Transform};
use engine_ops::command::TemplateCommand;
use engine_ops::ipc::{ActionId, ActionSummary, CommandResult, IpcError};
use engine_ops::processor::CommandProcessor;
use engine_ops::template::TemplateState;
use glam::{Quat, Vec3};
use tauri::State;

/// Global map of open template files → CommandProcessor.
pub struct EditorState {
    pub processors: HashMap<PathBuf, CommandProcessor>,
}

impl EditorState {
    pub fn new() -> Self {
        Self { processors: HashMap::new() }
    }
}

fn get_processor<'a>(
    map: &'a mut HashMap<PathBuf, CommandProcessor>,
    template_path: &str,
) -> Result<&'a mut CommandProcessor, IpcError> {
    let path = PathBuf::from(template_path);
    map.get_mut(&path).ok_or_else(|| IpcError {
        code: engine_core::error::ErrorCode::TemplateNoTemplateOpen as u32,
        message: format!("Template not open: {template_path}"),
    })
}

/// Opens a template file and registers its [`CommandProcessor`].
pub fn template_open_inner(
    state: &Mutex<EditorState>,
    template_path: String,
) -> Result<TemplateState, IpcError> {
    let path = PathBuf::from(&template_path);
    let processor = CommandProcessor::load(path.clone()).map_err(IpcError::from)?;
    let result = processor.state_ref().clone();
    state.lock().unwrap().processors.insert(path, processor);
    Ok(result)
}

#[tauri::command]
pub fn template_open(
    state: State<'_, Mutex<EditorState>>,
    template_path: String,
) -> Result<TemplateState, IpcError> {
    template_open_inner(&state, template_path)
}

/// Closes a template and removes its processor from the active set.
pub fn template_close_inner(
    state: &Mutex<EditorState>,
    template_path: String,
) -> Result<(), IpcError> {
    let path = PathBuf::from(&template_path);
    state.lock().unwrap().processors.remove(&path);
    Ok(())
}

#[tauri::command]
pub fn template_close(
    state: State<'_, Mutex<EditorState>>,
    template_path: String,
) -> Result<(), IpcError> {
    template_close_inner(&state, template_path)
}

/// Executes a [`TemplateCommand`] and records it in the undo history.
pub fn template_execute_inner(
    state: &Mutex<EditorState>,
    template_path: String,
    command: TemplateCommand,
) -> Result<CommandResult, IpcError> {
    let mut guard = state.lock().unwrap();
    let proc = get_processor(&mut guard.processors, &template_path)?;
    proc.execute(command).map_err(IpcError::from)
}

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
            .map_err(|e| IpcError {
                code: engine_core::error::ErrorCode::TemplateNoTemplateOpen as u32,
                message: e,
            })?;
    }

    Ok(result)
}

/// Undoes the last command on the given template, returning the undone [`ActionId`].
pub fn template_undo_inner(
    state: &Mutex<EditorState>,
    template_path: String,
) -> Result<Option<ActionId>, IpcError> {
    let mut guard = state.lock().unwrap();
    let proc = get_processor(&mut guard.processors, &template_path)?;
    proc.undo().map_err(IpcError::from)
}

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
                .map_err(|e| IpcError {
                    code: engine_core::error::ErrorCode::TemplateNoTemplateOpen as u32,
                    message: e,
                })?;
        }
    }
    Ok(result)
}

/// Redoes the last undone command on the given template, returning the redone [`ActionId`].
pub fn template_redo_inner(
    state: &Mutex<EditorState>,
    template_path: String,
) -> Result<Option<ActionId>, IpcError> {
    let mut guard = state.lock().unwrap();
    let proc = get_processor(&mut guard.processors, &template_path)?;
    proc.redo().map_err(IpcError::from)
}

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
                .map_err(|e| IpcError {
                    code: engine_core::error::ErrorCode::TemplateNoTemplateOpen as u32,
                    message: e,
                })?;
        }
    }
    Ok(result)
}

/// Returns a summary of all recorded actions for the given template.
pub fn template_history_inner(
    state: &Mutex<EditorState>,
    template_path: String,
) -> Result<Vec<ActionSummary>, IpcError> {
    let guard = state.lock().unwrap();
    let path = PathBuf::from(&template_path);
    let proc = guard.processors.get(&path).ok_or_else(|| IpcError {
        code: engine_core::error::ErrorCode::TemplateNoTemplateOpen as u32,
        message: format!("Template not open: {template_path}"),
    })?;
    Ok(proc.history_summaries())
}

#[tauri::command]
pub fn template_history(
    state: State<'_, Mutex<EditorState>>,
    template_path: String,
) -> Result<Vec<ActionSummary>, IpcError> {
    template_history_inner(&state, template_path)
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use engine_ops::template::{TemplateComponent, TemplateEntity, TemplateState};
    use serde_json::json;

    fn make_template_with_transform(entity_id: u64, px: f32, py: f32, pz: f32) -> TemplateState {
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
        let mut state = TemplateState::new("test");
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

    #[test]
    fn extract_transform_scale_parsed_correctly() {
        let data = json!({
            "position": {"x": 0.0, "y": 0.0, "z": 0.0},
            "rotation": {"x": 0.0, "y": 0.0, "z": 0.0, "w": 1.0},
            "scale":    {"x": 2.0, "y": 3.0, "z": 4.0}
        });
        let mut ts = TemplateState::new("test");
        ts.entities.push(TemplateEntity {
            id: 1,
            name: None,
            components: vec![TemplateComponent { type_name: "Transform".to_string(), data }],
        });
        let (_, _, scl) = extract_transform_from_template(1, &ts).unwrap();
        assert!((scl.x - 2.0).abs() < 1e-4);
        assert!((scl.y - 3.0).abs() < 1e-4);
        assert!((scl.z - 4.0).abs() < 1e-4);
    }
}
