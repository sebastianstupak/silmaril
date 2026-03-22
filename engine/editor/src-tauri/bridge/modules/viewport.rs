use crate::bridge::registry::{CommandSpec, EditorModule};

pub struct ViewportModule;

impl EditorModule for ViewportModule {
    fn id(&self) -> &str {
        "viewport"
    }

    fn commands(&self) -> Vec<CommandSpec> {
        vec![
            CommandSpec {
                id: "viewport.screenshot".into(), module_id: String::new(),
                label: "Take Screenshot".into(), category: "Viewport".into(),
                description: Some("Capture a screenshot of the current viewport".into()),
                keybind: None, args_schema: None, returns_data: true, non_undoable: true,
            },
            CommandSpec {
                id: "viewport.orbit".into(), module_id: String::new(),
                label: "Orbit Camera".into(), category: "Viewport".into(),
                description: Some("Orbit the viewport camera by delta angles".into()),
                keybind: None,
                args_schema: Some(serde_json::json!({
                    "type": "object", "required": ["dx", "dy"],
                    "properties": {
                        "dx": { "type": "number", "description": "Horizontal delta (degrees)" },
                        "dy": { "type": "number", "description": "Vertical delta (degrees)" }
                    }
                })),
                returns_data: false, non_undoable: true,
            },
            CommandSpec {
                id: "viewport.pan".into(), module_id: String::new(),
                label: "Pan Camera".into(), category: "Viewport".into(),
                description: Some("Pan the viewport camera".into()),
                keybind: None,
                args_schema: Some(serde_json::json!({
                    "type": "object", "required": ["dx", "dy"],
                    "properties": {
                        "dx": { "type": "number" }, "dy": { "type": "number" }
                    }
                })),
                returns_data: false, non_undoable: true,
            },
            CommandSpec {
                id: "viewport.zoom".into(), module_id: String::new(),
                label: "Zoom Camera".into(), category: "Viewport".into(),
                description: Some("Zoom the viewport camera".into()),
                keybind: None,
                args_schema: Some(serde_json::json!({
                    "type": "object", "required": ["delta"],
                    "properties": { "delta": { "type": "number" } }
                })),
                returns_data: false, non_undoable: true,
            },
            CommandSpec {
                id: "viewport.set_projection".into(), module_id: String::new(),
                label: "Set Projection".into(), category: "Viewport".into(),
                description: Some("Switch between perspective and orthographic projection".into()),
                keybind: None,
                args_schema: Some(serde_json::json!({
                    "type": "object", "required": ["mode"],
                    "properties": { "mode": { "type": "string", "enum": ["perspective", "ortho"] } }
                })),
                returns_data: false, non_undoable: true,
            },
            CommandSpec {
                id: "viewport.reset_camera".into(), module_id: String::new(),
                label: "Reset Camera".into(), category: "Viewport".into(),
                description: Some("Reset the camera to the default position".into()),
                keybind: None, args_schema: None, returns_data: false, non_undoable: true,
            },
            CommandSpec {
                id: "viewport.set_grid_visible".into(), module_id: String::new(),
                label: "Set Grid Visible".into(), category: "Viewport".into(),
                description: Some("Show or hide the viewport grid".into()),
                keybind: None,
                args_schema: Some(serde_json::json!({
                    "type": "object", "required": ["visible"],
                    "properties": { "visible": { "type": "boolean" } }
                })),
                returns_data: false, non_undoable: true,
            },
            CommandSpec {
                id: "viewport.focus_entity".into(), module_id: String::new(),
                label: "Focus Entity".into(), category: "Viewport".into(),
                description: Some("Frame the viewport camera on an entity".into()),
                keybind: Some("F".into()),
                args_schema: Some(serde_json::json!({
                    "type": "object", "required": ["id"],
                    "properties": { "id": { "type": "integer" } }
                })),
                returns_data: false, non_undoable: true,
            },
            CommandSpec {
                id: "viewport.focus_entity_animated".into(), module_id: String::new(),
                label: "Focus Entity (Animated)".into(), category: "Viewport".into(),
                description: Some(
                    "Smoothly animate the viewport camera to orbit the selected entity".into()
                ),
                keybind: None,
                args_schema: Some(serde_json::json!({
                    "type": "object",
                    "properties": { "entityId": { "type": "number" } },
                    "required": ["entityId"]
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
        let module = ViewportModule;
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
