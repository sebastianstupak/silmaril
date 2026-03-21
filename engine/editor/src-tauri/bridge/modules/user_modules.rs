use crate::bridge::registry::{CommandSpec, EditorModule};

/// Placeholder for user-defined commands registered at runtime.
pub struct UserCommandsModule;

impl EditorModule for UserCommandsModule {
    fn id(&self) -> &str {
        "user"
    }

    fn commands(&self) -> Vec<CommandSpec> {
        vec![CommandSpec {
            id: "user.placeholder".into(),
            module_id: String::new(),
            label: "User Command Placeholder".into(),
            category: "User".into(),
            description: Some("Placeholder for user-defined commands".into()),
            keybind: None,
            args_schema: None,
            returns_data: false,
            non_undoable: true,
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_have_correct_prefix() {
        let module = UserCommandsModule;
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
