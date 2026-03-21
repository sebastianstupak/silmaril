use crate::bridge::registry::{CommandSpec, EditorModule};

pub struct ViewportModule;

impl EditorModule for ViewportModule {
    fn id(&self) -> &str {
        "viewport"
    }

    fn commands(&self) -> Vec<CommandSpec> {
        vec![
            CommandSpec {
                id: "viewport.screenshot".into(),
                module_id: String::new(),
                label: "Take Screenshot".into(),
                category: "Viewport".into(),
                description: Some("Capture a screenshot of the current viewport".into()),
                keybind: None,
                args_schema: None,
                returns_data: true,
                non_undoable: true,
            },
            CommandSpec {
                id: "viewport.toggle_grid".into(),
                module_id: String::new(),
                label: "Toggle Grid".into(),
                category: "Viewport".into(),
                description: Some("Show or hide the viewport grid".into()),
                keybind: None,
                args_schema: None,
                returns_data: false,
                non_undoable: true,
            },
            CommandSpec {
                id: "viewport.toggle_gizmos".into(),
                module_id: String::new(),
                label: "Toggle Gizmos".into(),
                category: "Viewport".into(),
                description: Some("Show or hide viewport gizmos".into()),
                keybind: None,
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
