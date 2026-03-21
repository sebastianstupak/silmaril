use crate::bridge::registry::{CommandSpec, EditorModule};

pub struct BuildModule;

impl EditorModule for BuildModule {
    fn id(&self) -> &str {
        "build"
    }

    fn commands(&self) -> Vec<CommandSpec> {
        vec![
            CommandSpec {
                id: "build.run".into(),
                module_id: String::new(),
                label: "Run Project".into(),
                category: "Build".into(),
                description: Some("Run the project in play mode".into()),
                keybind: None,
                args_schema: None,
                returns_data: false,
                undoable: false,
            },
            CommandSpec {
                id: "build.build".into(),
                module_id: String::new(),
                label: "Build Project".into(),
                category: "Build".into(),
                description: Some("Build the project for distribution".into()),
                keybind: None,
                args_schema: None,
                returns_data: false,
                undoable: false,
            },
            CommandSpec {
                id: "build.package".into(),
                module_id: String::new(),
                label: "Package Project".into(),
                category: "Build".into(),
                description: Some("Package the project into a distributable archive".into()),
                keybind: None,
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
        let module = BuildModule;
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
