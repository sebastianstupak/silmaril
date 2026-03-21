use crate::bridge::registry::{CommandSpec, EditorModule};

pub struct SceneModule;

impl EditorModule for SceneModule {
    fn id(&self) -> &str {
        "scene"
    }

    fn commands(&self) -> Vec<CommandSpec> {
        vec![
            CommandSpec {
                id: "scene.get_state".into(), module_id: String::new(),
                label: "Get Scene State".into(), category: "Scene".into(),
                description: Some("Return the full scene state snapshot".into()),
                keybind: None, args_schema: None, returns_data: true, non_undoable: true,
            },
            CommandSpec {
                id: "scene.create_entity".into(), module_id: String::new(),
                label: "Create Entity".into(), category: "Scene".into(),
                description: Some("Create a new entity with an optional name".into()),
                keybind: None,
                args_schema: Some(serde_json::json!({
                    "type": "object",
                    "properties": { "name": { "type": "string" } }
                })),
                returns_data: false, non_undoable: false,
            },
            CommandSpec {
                id: "scene.delete_entity".into(), module_id: String::new(),
                label: "Delete Entity".into(), category: "Scene".into(),
                description: Some("Delete an entity by id".into()),
                keybind: Some("Delete".into()),
                args_schema: Some(serde_json::json!({
                    "type": "object", "required": ["id"],
                    "properties": { "id": { "type": "integer" } }
                })),
                returns_data: false, non_undoable: false,
            },
            CommandSpec {
                id: "scene.rename_entity".into(), module_id: String::new(),
                label: "Rename Entity".into(), category: "Scene".into(),
                description: Some("Rename an entity by id".into()),
                keybind: None,
                args_schema: Some(serde_json::json!({
                    "type": "object", "required": ["id", "name"],
                    "properties": {
                        "id": { "type": "integer" },
                        "name": { "type": "string" }
                    }
                })),
                returns_data: false, non_undoable: false,
            },
            CommandSpec {
                id: "scene.duplicate_entity".into(), module_id: String::new(),
                label: "Duplicate Entity".into(), category: "Scene".into(),
                description: Some("Duplicate an entity by id".into()),
                keybind: Some("Ctrl+D".into()),
                args_schema: Some(serde_json::json!({
                    "type": "object", "required": ["id"],
                    "properties": { "id": { "type": "integer" } }
                })),
                returns_data: false, non_undoable: false,
            },
            CommandSpec {
                id: "scene.add_component".into(), module_id: String::new(),
                label: "Add Component".into(), category: "Scene".into(),
                description: Some("Add a component to an entity".into()),
                keybind: None,
                args_schema: Some(serde_json::json!({
                    "type": "object", "required": ["id", "component"],
                    "properties": {
                        "id": { "type": "integer" },
                        "component": { "type": "string", "description": "Component type name" }
                    }
                })),
                returns_data: false, non_undoable: false,
            },
            CommandSpec {
                id: "scene.remove_component".into(), module_id: String::new(),
                label: "Remove Component".into(), category: "Scene".into(),
                description: Some("Remove a component from an entity".into()),
                keybind: None,
                args_schema: Some(serde_json::json!({
                    "type": "object", "required": ["id", "component"],
                    "properties": {
                        "id": { "type": "integer" },
                        "component": { "type": "string" }
                    }
                })),
                returns_data: false, non_undoable: false,
            },
            CommandSpec {
                id: "scene.set_component_field".into(), module_id: String::new(),
                label: "Set Component Field".into(), category: "Scene".into(),
                description: Some("Set a field on a component".into()),
                keybind: None,
                args_schema: Some(serde_json::json!({
                    "type": "object", "required": ["id", "component", "field", "value"],
                    "properties": {
                        "id": { "type": "integer" },
                        "component": { "type": "string" },
                        "field": { "type": "string" },
                        "value": {}
                    }
                })),
                returns_data: false, non_undoable: false,
            },
            CommandSpec {
                id: "scene.select_entity".into(), module_id: String::new(),
                label: "Select Entity".into(), category: "Scene".into(),
                description: Some("Select an entity by id, or deselect with null".into()),
                keybind: None,
                args_schema: Some(serde_json::json!({
                    "type": "object",
                    "properties": { "id": { "type": ["integer", "null"] } }
                })),
                returns_data: false, non_undoable: true,
            },
            CommandSpec {
                id: "scene.move_entity".into(), module_id: String::new(),
                label: "Move Entity".into(), category: "Scene".into(),
                description: Some("Set entity position".into()),
                keybind: None,
                args_schema: Some(serde_json::json!({
                    "type": "object", "required": ["id", "x", "y", "z"],
                    "properties": {
                        "id": { "type": "integer" },
                        "x": { "type": "number" }, "y": { "type": "number" }, "z": { "type": "number" }
                    }
                })),
                returns_data: false, non_undoable: false,
            },
            CommandSpec {
                id: "scene.rotate_entity".into(), module_id: String::new(),
                label: "Rotate Entity".into(), category: "Scene".into(),
                description: Some("Set entity rotation (Euler angles, degrees)".into()),
                keybind: None,
                args_schema: Some(serde_json::json!({
                    "type": "object", "required": ["id", "rx", "ry", "rz"],
                    "properties": {
                        "id": { "type": "integer" },
                        "rx": { "type": "number" }, "ry": { "type": "number" }, "rz": { "type": "number" }
                    }
                })),
                returns_data: false, non_undoable: false,
            },
            CommandSpec {
                id: "scene.scale_entity".into(), module_id: String::new(),
                label: "Scale Entity".into(), category: "Scene".into(),
                description: Some("Set entity scale".into()),
                keybind: None,
                args_schema: Some(serde_json::json!({
                    "type": "object", "required": ["id", "sx", "sy", "sz"],
                    "properties": {
                        "id": { "type": "integer" },
                        "sx": { "type": "number" }, "sy": { "type": "number" }, "sz": { "type": "number" }
                    }
                })),
                returns_data: false, non_undoable: false,
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
