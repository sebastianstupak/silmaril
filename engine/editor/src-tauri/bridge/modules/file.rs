use crate::bridge::registry::{CommandSpec, EditorModule};

pub struct FileModule;

impl EditorModule for FileModule {
    fn id(&self) -> &str {
        "file"
    }

    fn commands(&self) -> Vec<CommandSpec> {
        vec![
            CommandSpec {
                id: "file.new_project".into(),
                module_id: String::new(),
                label: "New Project".into(),
                category: "File".into(),
                description: Some("Create a new project".into()),
                keybind: None,
                args_schema: None,
                returns_data: false,
                non_undoable: true,
            },
            CommandSpec {
                id: "file.open_project".into(),
                module_id: String::new(),
                label: "Open Project".into(),
                category: "File".into(),
                description: Some("Open an existing project".into()),
                keybind: None,
                args_schema: None,
                returns_data: false,
                non_undoable: true,
            },
            CommandSpec {
                id: "file.save_scene".into(),
                module_id: String::new(),
                label: "Save Scene".into(),
                category: "File".into(),
                description: Some("Save the current scene".into()),
                keybind: Some("Ctrl+S".into()),
                args_schema: None,
                returns_data: false,
                non_undoable: true,
            },
            CommandSpec {
                id: "file.save_scene_as".into(),
                module_id: String::new(),
                label: "Save Scene As".into(),
                category: "File".into(),
                description: Some("Save the current scene to a new location".into()),
                keybind: Some("Ctrl+Shift+S".into()),
                args_schema: None,
                returns_data: false,
                non_undoable: true,
            },
            CommandSpec {
                id: "file.open_scene".into(),
                module_id: String::new(),
                label: "Open Scene".into(),
                category: "File".into(),
                description: Some("Open a scene file".into()),
                keybind: None,
                args_schema: None,
                returns_data: false,
                non_undoable: true,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_have_correct_prefix() {
        let module = FileModule;
        let prefix = format!("{}.", module.id());
        for cmd in module.commands() {
            assert!(
                cmd.id.starts_with(&prefix),
                "Command '{}' has wrong prefix",
                cmd.id
            );
        }
    }
}
