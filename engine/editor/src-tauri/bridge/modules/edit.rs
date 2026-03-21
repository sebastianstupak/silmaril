use crate::bridge::registry::{CommandSpec, EditorModule};

pub struct EditModule;

impl EditorModule for EditModule {
    fn id(&self) -> &str {
        "edit"
    }

    fn commands(&self) -> Vec<CommandSpec> {
        vec![
            CommandSpec {
                id: "edit.undo".into(),
                module_id: String::new(),
                label: "Undo".into(),
                category: "Edit".into(),
                description: Some("Undo the last action".into()),
                keybind: Some("Ctrl+Z".into()),
                args_schema: None,
                returns_data: false,
                undoable: false,
            },
            CommandSpec {
                id: "edit.redo".into(),
                module_id: String::new(),
                label: "Redo".into(),
                category: "Edit".into(),
                description: Some("Redo the last undone action".into()),
                keybind: Some("Ctrl+Y".into()),
                args_schema: None,
                returns_data: false,
                undoable: false,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_have_correct_prefix() {
        let module = EditModule;
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
