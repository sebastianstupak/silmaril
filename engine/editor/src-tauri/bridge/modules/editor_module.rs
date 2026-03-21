use crate::bridge::registry::{CommandSpec, EditorModule};

/// Read-only editor query commands exposed to MCP agents.
pub struct EditorCoreModule;

impl EditorModule for EditorCoreModule {
    fn id(&self) -> &str {
        "editor"
    }

    fn commands(&self) -> Vec<CommandSpec> {
        vec![
            CommandSpec {
                id: "editor.get_scene_state".into(),
                module_id: String::new(),
                label: "Get Scene State".into(),
                category: "Editor".into(),
                description: Some("Return the full scene state as JSON".into()),
                keybind: None,
                args_schema: None,
                returns_data: true,
                non_undoable: true,
            },
            CommandSpec {
                id: "editor.get_entity".into(),
                module_id: String::new(),
                label: "Get Entity".into(),
                category: "Editor".into(),
                description: Some("Return a single entity's full state by id".into()),
                keybind: None,
                args_schema: Some(serde_json::json!({
                    "type": "object",
                    "required": ["id"],
                    "properties": {
                        "id": { "type": "integer", "description": "Entity id" }
                    }
                })),
                returns_data: true,
                non_undoable: true,
            },
            CommandSpec {
                id: "editor.list_assets".into(),
                module_id: String::new(),
                label: "List Assets".into(),
                category: "Editor".into(),
                description: Some("Return a list of all project assets".into()),
                keybind: None,
                args_schema: None,
                returns_data: true,
                non_undoable: true,
            },
            CommandSpec {
                id: "editor.get_project_info".into(),
                module_id: String::new(),
                label: "Get Project Info".into(),
                category: "Editor".into(),
                description: Some("Return project metadata (name, path, version)".into()),
                keybind: None,
                args_schema: None,
                returns_data: true,
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
        let module = EditorCoreModule;
        let prefix = format!("{}.", module.id());
        for cmd in module.commands() {
            assert!(cmd.id.starts_with(&prefix), "Command '{}' has wrong prefix", cmd.id);
        }
    }
}
