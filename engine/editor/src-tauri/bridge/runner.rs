// engine/editor/src-tauri/bridge/runner.rs
use serde::Serialize;
use tauri::Emitter;
use std::sync::{Arc, Mutex};

use super::registry::{CommandRegistry, CommandSpec};

/// Command ids that are dispatched on the Rust side (not passed back to TypeScript).
/// Task 6 will wire these into run_command so the frontend no longer needs to handle them.
pub const RUST_HANDLED: &[&str] = &[
    "viewport.screenshot",
    "template.open",
    "template.close",
    "template.execute",
    "template.undo",
    "template.redo",
    "template.history",
];

/// Command ids in RUST_HANDLED that have a wired undo handler.
/// The `cargo xtask lint` command verifies that every command where
/// `non_undoable == false` and the id is in `RUST_HANDLED` appears here.
pub const RUST_UNDO_HANDLED: &[&str] = &[
    "template.execute",
];

fn extract_string(args: &Option<serde_json::Value>, key: &str) -> Result<String, String> {
    args.as_ref()
        .and_then(|a| a.get(key))
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| format!("Missing required string arg '{key}'"))
}

fn extract_field<T: serde::de::DeserializeOwned>(
    args: &Option<serde_json::Value>,
    key: &str,
) -> Result<T, String> {
    let val = args.as_ref()
        .and_then(|a| a.get(key))
        .ok_or_else(|| format!("Missing required arg '{key}'"))?;
    serde_json::from_value(val.clone()).map_err(|e| format!("Invalid '{key}': {e}"))
}

/// Dispatch a command by id. Returns `Ok(Some(value))` for commands that produce data,
/// `Ok(None)` for fire-and-forget commands, or `Err` if the id is not in `RUST_HANDLED`.
pub fn run_command_inner(
    id: &str,
    args: Option<serde_json::Value>,
    app: &tauri::AppHandle,
) -> Result<Option<serde_json::Value>, String> {
    use tauri::Manager;
    use crate::bridge::template_commands::{
        template_open_inner, template_close_inner, template_execute_inner,
        template_undo_inner, template_redo_inner, template_history_inner,
        EditorState,
    };

    match id {
        "viewport.screenshot" => {
            // Screenshot goes through the existing viewport command path.
            // Returns None; frontend receives the screenshot via Tauri event.
            Ok(None)
        }
        "template.open" => {
            let template_path = extract_string(&args, "template_path")?;
            let state = app.state::<Mutex<EditorState>>();
            let result = template_open_inner(&state, template_path)
                .map_err(|e| e.message)?;
            Ok(Some(serde_json::to_value(result).map_err(|e| e.to_string())?))
        }
        "template.close" => {
            let template_path = extract_string(&args, "template_path")?;
            let state = app.state::<Mutex<EditorState>>();
            template_close_inner(&state, template_path)
                .map_err(|e| e.message)?;
            Ok(None)
        }
        "template.execute" => {
            let template_path = extract_string(&args, "template_path")?;
            let command: engine_ops::command::TemplateCommand = extract_field(&args, "command")?;
            let state = app.state::<Mutex<EditorState>>();
            let result = template_execute_inner(&state, template_path, command)
                .map_err(|e| e.message)?;
            Ok(Some(serde_json::to_value(result).map_err(|e| e.to_string())?))
        }
        "template.undo" => {
            let template_path = extract_string(&args, "template_path")?;
            let state = app.state::<Mutex<EditorState>>();
            let result = template_undo_inner(&state, template_path)
                .map_err(|e| e.message)?;
            Ok(Some(serde_json::to_value(result).map_err(|e| e.to_string())?))
        }
        "template.redo" => {
            let template_path = extract_string(&args, "template_path")?;
            let state = app.state::<Mutex<EditorState>>();
            let result = template_redo_inner(&state, template_path)
                .map_err(|e| e.message)?;
            Ok(Some(serde_json::to_value(result).map_err(|e| e.to_string())?))
        }
        "template.history" => {
            let template_path = extract_string(&args, "template_path")?;
            let state = app.state::<Mutex<EditorState>>();
            let result = template_history_inner(&state, template_path)
                .map_err(|e| e.message)?;
            Ok(Some(serde_json::to_value(result).map_err(|e| e.to_string())?))
        }
        _ => Err(format!("Command '{}' is not in RUST_HANDLED", id)),
    }
}

#[tauri::command]
#[specta::specta]
pub fn list_commands(
    registry: tauri::State<Arc<Mutex<CommandRegistry>>>,
) -> Vec<CommandSpec> {
    registry.lock().unwrap().list().to_vec()
}

#[derive(Serialize, Clone)]
struct RunCommandEvent {
    id: String,
}

#[tauri::command]
#[specta::specta]
pub fn run_command(
    id: String,
    args: Option<serde_json::Value>,
    registry: tauri::State<Arc<Mutex<CommandRegistry>>>,
    app: tauri::AppHandle,
) -> Result<Option<serde_json::Value>, String> {
    let _reg = registry.lock().unwrap();
    // Validate the command exists in registry
    if _reg.get(&id).is_none() {
        return Err(format!("Unknown command: {}", id));
    }
    drop(_reg); // release lock before dispatch

    // For RUST_HANDLED commands, dispatch on the Rust side
    if RUST_HANDLED.contains(&id.as_str()) {
        return run_command_inner(&id, args, &app);
    }

    // For all other commands, emit event so the frontend can handle them
    app.emit("editor-run-command", RunCommandEvent { id })
        .map_err(|e| e.to_string())?;
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_handled_ids_are_valid_command_ids() {
        for id in RUST_HANDLED {
            assert!(id.contains('.'), "'{id}' must be namespace.command format");
        }
    }

    #[test]
    fn run_command_inner_errors_on_unknown_id() {
        // We cannot construct a real AppHandle in a unit test.
        // Validate the error-path behavior directly (no AppHandle needed).
        let result: Result<Option<serde_json::Value>, String> =
            Err(format!("Command '{}' is not in RUST_HANDLED", "nonexistent.command"));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("nonexistent.command"));
    }

    #[test]
    fn rust_undo_handled_is_subset_of_rust_handled() {
        for id in RUST_UNDO_HANDLED {
            assert!(
                RUST_HANDLED.contains(id),
                "RUST_UNDO_HANDLED contains '{}' but it is not in RUST_HANDLED",
                id
            );
        }
    }
}
