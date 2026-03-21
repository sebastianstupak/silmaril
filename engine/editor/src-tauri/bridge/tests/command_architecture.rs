use crate::bridge::{modules::*, registry::CommandRegistry, runner::RUST_HANDLED};

fn build_full_registry() -> CommandRegistry {
    let (mut reg, _rx) = CommandRegistry::new();
    reg.register_module(&FileModule);
    reg.register_module(&EditModule);
    reg.register_module(&ViewModule);
    reg.register_module(&SceneModule);
    reg.register_module(&AssetModule);
    reg.register_module(&BuildModule);
    reg.register_module(&ViewportModule);
    reg.register_module(&TemplateModule);
    reg.register_module(&UserCommandsModule);
    reg
}

#[test]
fn all_commands_have_correct_namespace_prefix() {
    let reg = build_full_registry();
    for cmd in reg.list() {
        let prefix = format!("{}.", cmd.module_id);
        assert!(
            cmd.id.starts_with(&prefix),
            "Command '{}' has module_id '{}' but id does not start with '{}'",
            cmd.id, cmd.module_id, prefix
        );
    }
}

#[test]
fn all_keybinds_are_unique() {
    let reg = build_full_registry();
    let mut seen: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
    for cmd in reg.list() {
        if let Some(kb) = cmd.keybind.as_deref() {
            if let Some(existing_id) = seen.insert(kb, cmd.id.as_str()) {
                panic!(
                    "Keybind '{}' is used by both '{}' and '{}'",
                    kb, existing_id, cmd.id
                );
            }
        }
    }
}

#[test]
fn command_manifest_snapshot() {
    let reg = build_full_registry();
    let commands: Vec<_> = reg.list().iter().map(|c| {
        serde_json::json!({
            "id": c.id,
            "module_id": c.module_id,
            "label": c.label,
            "category": c.category,
            "keybind": c.keybind,
            "returns_data": c.returns_data,
            "non_undoable": c.non_undoable,
        })
    }).collect();
    insta::assert_json_snapshot!("command_manifest", commands);
}

#[test]
fn rust_handled_commands_exist_in_registry() {
    let reg = build_full_registry();
    for id in RUST_HANDLED {
        assert!(
            reg.get(id).is_some(),
            "RUST_HANDLED contains '{}' but it is not in the registry",
            id
        );
    }
}
