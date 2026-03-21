// engine/editor/src-tauri/bridge/runner.rs
// NOTE: This file is being incrementally replaced as part of the command-arch refactor.
// Task 6 will wire the full Tauri state and remove the legacy IPC commands below.
use serde::Serialize;
use tauri::Emitter;

use super::registry::{CommandRegistryState, CommandSpec};

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
pub fn list_commands(
    _registry: tauri::State<CommandRegistryState>,
) -> Vec<CommandSpec> {
    // CommandRegistryState is a temporary stub (Task 6 wires real state).
    // Return empty list until Task 5/6 complete the wiring.
    Vec::new()
}

#[derive(Serialize, Clone)]
struct RunCommandEvent {
    id: String,
}

#[tauri::command]
pub fn run_command(
    id: String,
    _registry: tauri::State<CommandRegistryState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    // CommandRegistryState is a temporary stub — always emit the event for now.
    // Task 6 will replace this with run_command_inner using Arc<Mutex<CommandRegistry>>.
    app.emit("editor-run-command", RunCommandEvent { id })
        .map_err(|e| e.to_string())?;

    Ok(())
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
}
