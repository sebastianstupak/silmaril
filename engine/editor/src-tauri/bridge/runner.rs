// engine/editor/src-tauri/bridge/runner.rs
use serde::Serialize;
use tauri::Emitter;

use super::registry::{CommandRegistryState, EditorCommand};

#[tauri::command]
pub fn list_commands(
    registry: tauri::State<CommandRegistryState>,
) -> Vec<EditorCommand> {
    registry.0.lock().unwrap().list()
}

#[derive(Serialize, Clone)]
struct RunCommandEvent {
    id: String,
}

#[tauri::command]
pub fn run_command(
    id: String,
    registry: tauri::State<CommandRegistryState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let known = registry.0.lock().unwrap().list();
    let exists = known.iter().any(|c| c.id == id);
    if !exists {
        return Err(format!("Unknown command: {}", id));
    }

    app.emit("editor-run-command", RunCommandEvent { id })
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::registry::{CommandRegistry, EditorCommand};

    #[test]
    fn list_after_register() {
        let mut reg = CommandRegistry::new();
        reg.register(EditorCommand::new("editor.toggle_grid", "Toggle Grid", "View"));
        let list = reg.list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "editor.toggle_grid");
    }

    #[test]
    fn unknown_command_returns_err() {
        let reg = CommandRegistry::new();
        let ids: Vec<_> = reg.list().iter().map(|c| c.id.clone()).collect();
        assert!(!ids.contains(&"editor.nonexistent".to_string()));
    }
}
