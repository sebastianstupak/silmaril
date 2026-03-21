use crate::bridge::registry::{CommandSpec, EditorModule};

pub struct TemplateModule;

impl EditorModule for TemplateModule {
    fn id(&self) -> &str {
        "template"
    }

    fn commands(&self) -> Vec<CommandSpec> {
        vec![
            CommandSpec {
                id: "template.open".into(),
                module_id: String::new(),
                label: "Open Template".into(),
                category: "Template".into(),
                description: Some("Open a template file for editing".into()),
                keybind: None,
                args_schema: None,
                returns_data: false,
                non_undoable: true,
            },
            CommandSpec {
                id: "template.close".into(),
                module_id: String::new(),
                label: "Close Template".into(),
                category: "Template".into(),
                description: Some("Close the currently open template".into()),
                keybind: None,
                args_schema: None,
                returns_data: false,
                non_undoable: true,
            },
            CommandSpec {
                id: "template.execute".into(),
                module_id: String::new(),
                label: "Execute Template".into(),
                category: "Template".into(),
                description: Some("Execute the current template".into()),
                keybind: None,
                args_schema: None,
                returns_data: false,
                non_undoable: false,
            },
            CommandSpec {
                id: "template.undo".into(),
                module_id: String::new(),
                label: "Undo Template Action".into(),
                category: "Template".into(),
                description: Some("Undo the last template action".into()),
                keybind: None,
                args_schema: None,
                returns_data: false,
                non_undoable: true,
            },
            CommandSpec {
                id: "template.redo".into(),
                module_id: String::new(),
                label: "Redo Template Action".into(),
                category: "Template".into(),
                description: Some("Redo the last undone template action".into()),
                keybind: None,
                args_schema: None,
                returns_data: false,
                non_undoable: true,
            },
            CommandSpec {
                id: "template.history".into(),
                module_id: String::new(),
                label: "Template History".into(),
                category: "Template".into(),
                description: Some("View the template action history".into()),
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
        let module = TemplateModule;
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
