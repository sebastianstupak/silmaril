use crate::bridge::{modules::*, registry::CommandRegistry, runner::{RUST_HANDLED, RUST_UNDO_HANDLED}};

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
    reg.register_module(&EditorCoreModule);
    reg.register_module(&ProjectModule);
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

#[test]
fn lint_undo_coverage() {
    let reg = build_full_registry();
    let needs_undo = reg.requires_undo_handler();

    let mut missing: Vec<String> = Vec::new();
    for cmd in &needs_undo {
        if RUST_HANDLED.contains(&cmd.id.as_str()) {
            // This command is Rust-handled and expects an undo handler.
            // It must be listed in RUST_UNDO_HANDLED.
            if !RUST_UNDO_HANDLED.contains(&cmd.id.as_str()) {
                missing.push(cmd.id.clone());
            }
        }
    }

    assert!(
        missing.is_empty(),
        "Commands with non_undoable=false in RUST_HANDLED are missing from RUST_UNDO_HANDLED:\n  {}\n\
         Either add them to RUST_UNDO_HANDLED after wiring undo, or set non_undoable=true if intentional.",
        missing.join("\n  ")
    );
}

#[test]
fn registry_watch_fires_on_module_registration() {
    let (mut reg, mut rx) = CommandRegistry::new();

    // Initial state: no change yet
    assert!(!rx.has_changed().unwrap());

    // After registering a module, the watch should have a new value
    reg.register_module(&FileModule);

    assert!(rx.has_changed().unwrap());
    let specs = rx.borrow_and_update();
    assert!(!specs.is_empty());
}
