use crate::bridge::registry::{CommandSpec, EditorModule};

pub struct AssetModule;

impl EditorModule for AssetModule {
    fn id(&self) -> &str {
        "asset"
    }

    fn commands(&self) -> Vec<CommandSpec> {
        vec![
            CommandSpec {
                id: "asset.import".into(),
                module_id: String::new(),
                label: "Import Asset".into(),
                category: "Asset".into(),
                description: Some("Import an external asset into the project".into()),
                keybind: None,
                args_schema: None,
                returns_data: false,
                undoable: false,
            },
            CommandSpec {
                id: "asset.refresh".into(),
                module_id: String::new(),
                label: "Refresh Assets".into(),
                category: "Asset".into(),
                description: Some("Refresh the asset database".into()),
                keybind: None,
                args_schema: None,
                returns_data: false,
                undoable: false,
            },
            CommandSpec {
                id: "asset.scan".into(),
                module_id: String::new(),
                label: "Scan Assets".into(),
                category: "Asset".into(),
                description: Some("Scan the project directory for assets".into()),
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
        let module = AssetModule;
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
