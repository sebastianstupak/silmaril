// engine/editor/src-tauri/bridge/runner.rs
// NOTE: This file is being incrementally replaced as part of the command-arch refactor.
// Task 5 will rewrite this file completely. The stubs below keep compilation working
// while registry.rs is updated (EditorCommand removed; CommandRegistryState is now a stub).
use serde::Serialize;
use tauri::Emitter;

use super::registry::{CommandRegistryState, CommandSpec};

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
    // Task 5 will replace this with run_command_inner using Arc<Mutex<CommandRegistry>>.
    app.emit("editor-run-command", RunCommandEvent { id })
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    // Old tests removed — they referenced EditorCommand which no longer exists.
    // New runner tests will be added in Task 5.
}
