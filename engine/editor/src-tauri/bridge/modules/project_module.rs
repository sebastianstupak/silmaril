use crate::bridge::registry::{CommandSpec, EditorModule};

/// Project-level build/codegen commands exposed to MCP agents.
pub struct ProjectModule;

impl EditorModule for ProjectModule {
    fn id(&self) -> &str { "project" }

    fn commands(&self) -> Vec<CommandSpec> {
        vec![
            CommandSpec {
                id: "project.build".into(), module_id: String::new(),
                label: "Build Project".into(), category: "Build".into(),
                description: Some("Build the project for a target platform".into()),
                keybind: None,
                args_schema: Some(serde_json::json!({
                    "type": "object", "required": ["platform"],
                    "properties": { "platform": { "type": "string" } }
                })),
                returns_data: false, non_undoable: true,
            },
            CommandSpec {
                id: "project.add_module".into(), module_id: String::new(),
                label: "Add Module".into(), category: "Build".into(),
                description: Some("Add an engine module to the project".into()),
                keybind: None,
                args_schema: Some(serde_json::json!({
                    "type": "object", "required": ["name"],
                    "properties": { "name": { "type": "string" } }
                })),
                returns_data: false, non_undoable: true,
            },
            CommandSpec {
                id: "project.list_modules".into(), module_id: String::new(),
                label: "List Modules".into(), category: "Build".into(),
                description: Some("Return all installed engine modules".into()),
                keybind: None, args_schema: None, returns_data: true, non_undoable: true,
            },
            CommandSpec {
                id: "project.generate_component".into(), module_id: String::new(),
                label: "Generate Component".into(), category: "Build".into(),
                description: Some("Scaffold a new ECS component".into()),
                keybind: None,
                args_schema: Some(serde_json::json!({
                    "type": "object", "required": ["name"],
                    "properties": {
                        "name": { "type": "string" },
                        "fields": { "type": "array", "items": { "type": "object" } }
                    }
                })),
                returns_data: false, non_undoable: true,
            },
            CommandSpec {
                id: "project.generate_system".into(), module_id: String::new(),
                label: "Generate System".into(), category: "Build".into(),
                description: Some("Scaffold a new ECS system".into()),
                keybind: None,
                args_schema: Some(serde_json::json!({
                    "type": "object", "required": ["name"],
                    "properties": { "name": { "type": "string" } }
                })),
                returns_data: false, non_undoable: true,
            },
            CommandSpec {
                id: "project.run".into(), module_id: String::new(),
                label: "Run Command".into(), category: "Build".into(),
                description: Some("Run any registered engine-ops command by id. Permission for the target command's category is verified before execution.".into()),
                keybind: None,
                args_schema: Some(serde_json::json!({
                    "type": "object", "required": ["id"],
                    "properties": { "id": { "type": "string", "description": "engine-ops command id" } }
                })),
                returns_data: false, non_undoable: true,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_have_correct_prefix() {
        let module = ProjectModule;
        let prefix = format!("{}.", module.id());
        for cmd in module.commands() {
            assert!(cmd.id.starts_with(&prefix), "Command '{}' has wrong prefix", cmd.id);
        }
    }
}
