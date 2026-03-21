use crate::bridge::registry::{CommandSpec, EditorModule};

pub struct SceneModule;

impl EditorModule for SceneModule {
    fn id(&self) -> &str {
        "scene"
    }

    fn commands(&self) -> Vec<CommandSpec> {
        vec![
            CommandSpec {
                id: "scene.new_entity".into(),
                module_id: String::new(),
                label: "New Entity".into(),
                category: "Scene".into(),
                description: Some("Create a new entity in the scene".into()),
                keybind: None,
                args_schema: None,
                returns_data: false,
                non_undoable: false,
            },
            CommandSpec {
                id: "scene.delete_entity".into(),
                module_id: String::new(),
                label: "Delete Entity".into(),
                category: "Scene".into(),
                description: Some("Delete the selected entity from the scene".into()),
                keybind: Some("Delete".into()),
                args_schema: None,
                returns_data: false,
                non_undoable: false,
            },
            CommandSpec {
                id: "scene.duplicate_entity".into(),
                module_id: String::new(),
                label: "Duplicate Entity".into(),
                category: "Scene".into(),
                description: Some("Duplicate the selected entity".into()),
                keybind: Some("Ctrl+D".into()),
                args_schema: None,
                returns_data: false,
                non_undoable: false,
            },
            CommandSpec {
                id: "scene.focus_entity".into(),
                module_id: String::new(),
                label: "Focus Entity".into(),
                category: "Scene".into(),
                description: Some("Focus the viewport camera on the selected entity".into()),
                keybind: Some("F".into()),
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
        let module = SceneModule;
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
