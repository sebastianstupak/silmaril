// engine/editor/src-tauri/bridge/registry.rs
use serde::Serialize;
use std::sync::Mutex;

#[derive(Serialize, Clone)]
pub struct EditorCommand {
    pub id: String,
    pub label: String,
    pub category: String,
    pub keybind: Option<String>,
    pub description: Option<String>,
}

impl EditorCommand {
    pub fn new(id: &str, label: &str, category: &str) -> Self {
        EditorCommand {
            id: id.to_string(),
            label: label.to_string(),
            category: category.to_string(),
            keybind: None,
            description: None,
        }
    }

    pub fn with_keybind(mut self, kb: &str) -> Self {
        self.keybind = Some(kb.to_string());
        self
    }
}

pub struct CommandRegistry {
    commands: Vec<EditorCommand>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        CommandRegistry { commands: Vec::new() }
    }

    pub fn register(&mut self, cmd: EditorCommand) {
        if let Some(pos) = self.commands.iter().position(|c| c.id == cmd.id) {
            self.commands[pos] = cmd;
        } else {
            self.commands.push(cmd);
        }
    }

    pub fn list(&self) -> Vec<EditorCommand> {
        self.commands.clone()
    }
}

/// Tauri managed state wrapper.
pub struct CommandRegistryState(pub Mutex<CommandRegistry>);

impl CommandRegistryState {
    pub fn new() -> Self {
        CommandRegistryState(Mutex::new(CommandRegistry::new()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_list() {
        let mut reg = CommandRegistry::new();
        reg.register(EditorCommand::new("editor.test", "Test", "View"));
        let list = reg.list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "editor.test");
    }

    #[test]
    fn register_overwrites_same_id() {
        let mut reg = CommandRegistry::new();
        reg.register(EditorCommand::new("editor.test", "First", "View"));
        reg.register(EditorCommand::new("editor.test", "Second", "View"));
        let list = reg.list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].label, "Second");
    }

    #[test]
    fn keybind_builder() {
        let cmd = EditorCommand::new("editor.grid", "Toggle Grid", "View")
            .with_keybind("Ctrl+G");
        assert_eq!(cmd.keybind.as_deref(), Some("Ctrl+G"));
    }
}
