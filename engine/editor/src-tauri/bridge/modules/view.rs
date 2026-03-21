use crate::bridge::registry::{CommandSpec, EditorModule};

pub struct ViewModule;

impl EditorModule for ViewModule {
    fn id(&self) -> &str {
        "view"
    }

    fn commands(&self) -> Vec<CommandSpec> {
        vec![
            CommandSpec {
                id: "view.toggle_hierarchy".into(),
                module_id: String::new(),
                label: "Toggle Hierarchy".into(),
                category: "View".into(),
                description: Some("Show or hide the hierarchy panel".into()),
                keybind: None,
                args_schema: None,
                returns_data: false,
                undoable: false,
            },
            CommandSpec {
                id: "view.toggle_inspector".into(),
                module_id: String::new(),
                label: "Toggle Inspector".into(),
                category: "View".into(),
                description: Some("Show or hide the inspector panel".into()),
                keybind: None,
                args_schema: None,
                returns_data: false,
                undoable: false,
            },
            CommandSpec {
                id: "view.toggle_console".into(),
                module_id: String::new(),
                label: "Toggle Console".into(),
                category: "View".into(),
                description: Some("Show or hide the console panel".into()),
                keybind: None,
                args_schema: None,
                returns_data: false,
                undoable: false,
            },
            CommandSpec {
                id: "view.toggle_asset_browser".into(),
                module_id: String::new(),
                label: "Toggle Asset Browser".into(),
                category: "View".into(),
                description: Some("Show or hide the asset browser panel".into()),
                keybind: None,
                args_schema: None,
                returns_data: false,
                undoable: false,
            },
            CommandSpec {
                id: "view.zoom_in".into(),
                module_id: String::new(),
                label: "Zoom In".into(),
                category: "View".into(),
                description: Some("Zoom the viewport in".into()),
                keybind: Some("Ctrl+=".into()),
                args_schema: None,
                returns_data: false,
                undoable: false,
            },
            CommandSpec {
                id: "view.zoom_out".into(),
                module_id: String::new(),
                label: "Zoom Out".into(),
                category: "View".into(),
                description: Some("Zoom the viewport out".into()),
                keybind: Some("Ctrl+-".into()),
                args_schema: None,
                returns_data: false,
                undoable: false,
            },
            CommandSpec {
                id: "view.zoom_reset".into(),
                module_id: String::new(),
                label: "Zoom Reset".into(),
                category: "View".into(),
                description: Some("Reset the viewport zoom to default".into()),
                keybind: Some("Ctrl+0".into()),
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
        let module = ViewModule;
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
