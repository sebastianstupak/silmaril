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

/// Dispatch a command by id. Returns `Ok(Some(value))` for commands that produce data,
/// `Ok(None)` for fire-and-forget commands, or `Err` if the id is not in `RUST_HANDLED`.
///
/// At this stage all branches are stubs — actual dispatch is wired in Task 6 when the
/// full `Arc<Mutex<CommandRegistry>>` Tauri state is available.
pub fn run_command_inner(
    id: &str,
    _args: Option<serde_json::Value>,
) -> Result<Option<serde_json::Value>, String> {
    match id {
        "viewport.screenshot" => {
            // Stub — actual screenshot logic stays in existing Tauri command for now.
            // Will be wired properly in Task 6.
            Ok(None)
        }
        "template.open" | "template.close" | "template.execute"
        | "template.undo" | "template.redo" | "template.history" => {
            // Stub — actual template dispatch stays in existing Tauri commands for now.
            // Will be wired properly in Task 6.
            Ok(None)
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
        return run_command_inner(&id, args);
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
        // Every id in RUST_HANDLED must contain a dot (namespace.command format)
        for id in RUST_HANDLED {
            assert!(id.contains('.'), "Command id '{}' must be in namespace.command format", id);
        }
    }

    #[test]
    fn run_command_inner_handles_all_rust_handled_ids() {
        for id in RUST_HANDLED {
            let result = run_command_inner(id, None);
            assert!(result.is_ok(), "run_command_inner failed for '{}': {:?}", id, result);
        }
    }

    #[test]
    fn run_command_inner_errors_on_unknown_id() {
        let result = run_command_inner("nonexistent.command", None);
        assert!(result.is_err());
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
