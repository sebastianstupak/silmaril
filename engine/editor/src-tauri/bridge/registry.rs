// engine/editor/src-tauri/bridge/registry.rs
use serde::Serialize;
use tokio::sync::watch;

/// A specta-compatible wrapper that generates `unknown` in TypeScript.
/// Used for `args_schema` which is an arbitrary JSON Schema value.
struct SpecUnknown;

impl specta::Type for SpecUnknown {
    fn inline(_t: &mut specta::TypeCollection, _generics: specta::Generics) -> specta::DataType {
        specta::DataType::Unknown
    }
}

/// Full descriptor for a single editor command.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct CommandSpec {
    pub id: String,
    pub module_id: String,
    pub label: String,
    pub category: String,
    pub description: Option<String>,
    pub keybind: Option<String>,
    #[specta(type = Option<SpecUnknown>)]
    pub args_schema: Option<serde_json::Value>,
    pub returns_data: bool,
    pub undoable: bool,
}

/// Implemented by every subsystem that exposes commands to the editor.
pub trait EditorModule: Send + Sync {
    fn id(&self) -> &str;
    fn commands(&self) -> Vec<CommandSpec>;
}

pub struct CommandRegistry {
    commands: Vec<CommandSpec>,
    module_index: std::collections::HashMap<String, Vec<usize>>,
    registry_tx: watch::Sender<Vec<CommandSpec>>,
}

impl CommandRegistry {
    /// Returns the registry and a watch receiver for live catalog updates.
    pub fn new() -> (Self, watch::Receiver<Vec<CommandSpec>>) {
        let (tx, rx) = watch::channel(Vec::new());
        let registry = Self {
            commands: Vec::new(),
            module_index: std::collections::HashMap::new(),
            registry_tx: tx,
        };
        (registry, rx)
    }

    /// Register all commands from a module. Sets `module_id` on each spec.
    /// Panics in debug builds if any command id doesn't start with `module.id() + "."`.
    pub fn register_module(&mut self, module: &dyn EditorModule) {
        let prefix = format!("{}.", module.id());
        assert!(
            !self.module_index.contains_key(module.id()),
            "Module '{}' has already been registered",
            module.id()
        );
        let mut specs = module.commands();
        assert!(
            !specs.is_empty(),
            "Module '{}' registered with no commands",
            module.id()
        );
        let base_idx = self.commands.len();
        let mut indices = Vec::with_capacity(specs.len());
        for (i, spec) in specs.iter_mut().enumerate() {
            assert!(
                spec.id.starts_with(&prefix),
                "Command '{}' in module '{}' must start with '{}'",
                spec.id,
                module.id(),
                prefix
            );
            spec.module_id = module.id().to_string();
            indices.push(base_idx + i);
        }
        self.commands.extend(specs);
        self.module_index.insert(module.id().to_string(), indices);
        let _ = self.registry_tx.send(self.commands.clone());
    }

    pub fn list(&self) -> &[CommandSpec] {
        &self.commands
    }

    pub fn get(&self, id: &str) -> Option<&CommandSpec> {
        self.commands.iter().find(|c| c.id == id)
    }

    pub fn by_module(&self, module_id: &str) -> Vec<&CommandSpec> {
        match self.module_index.get(module_id) {
            Some(indices) => indices.iter().map(|&i| &self.commands[i]).collect(),
            None => Vec::new(),
        }
    }

    pub fn get_by_keybind(&self, keybind: &str) -> Option<&CommandSpec> {
        self.commands
            .iter()
            .find(|c| c.keybind.as_deref() == Some(keybind))
    }

    pub fn undoable_commands(&self) -> Vec<&CommandSpec> {
        self.commands.iter().filter(|c| c.undoable).collect()
    }
}

// Temporary stub — CommandRegistryState is removed from the new design.
// Task 6 will replace .manage(CommandRegistryState::new()) with
// Arc<Mutex<CommandRegistry>> stored directly in Tauri managed state.
// This stub exists only so lib.rs and runner.rs compile during the
// incremental refactor (Tasks 3–5 will clean up the remaining usages).
#[allow(dead_code)]
pub struct CommandRegistryState;

#[allow(dead_code)]
impl CommandRegistryState {
    pub fn new() -> Self {
        CommandRegistryState
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestModule;
    impl EditorModule for TestModule {
        fn id(&self) -> &str {
            "test"
        }
        fn commands(&self) -> Vec<CommandSpec> {
            vec![CommandSpec {
                id: "test.do_thing".into(),
                module_id: String::new(), // set by register_module
                label: "Do Thing".into(),
                category: "test".into(),
                description: None,
                keybind: Some("Ctrl+T".into()),
                args_schema: None,
                returns_data: false,
                undoable: false,
            }]
        }
    }

    #[test]
    fn register_module_sets_module_id() {
        let (mut registry, _rx) = CommandRegistry::new();
        registry.register_module(&TestModule);
        let cmd = registry.get("test.do_thing").unwrap();
        assert_eq!(cmd.module_id, "test");
    }

    #[test]
    fn get_by_keybind_works() {
        let (mut registry, _rx) = CommandRegistry::new();
        registry.register_module(&TestModule);
        let cmd = registry.get_by_keybind("Ctrl+T").unwrap();
        assert_eq!(cmd.id, "test.do_thing");
    }

    #[test]
    fn by_module_returns_module_commands() {
        let (mut registry, _rx) = CommandRegistry::new();
        registry.register_module(&TestModule);
        let cmds = registry.by_module("test");
        assert_eq!(cmds.len(), 1);
    }

    #[test]
    fn register_module_panics_on_bad_prefix() {
        struct BadModule;
        impl EditorModule for BadModule {
            fn id(&self) -> &str {
                "good"
            }
            fn commands(&self) -> Vec<CommandSpec> {
                vec![CommandSpec {
                    id: "bad.prefix".into(), // wrong namespace
                    module_id: String::new(),
                    label: "X".into(),
                    category: "x".into(),
                    description: None,
                    keybind: None,
                    args_schema: None,
                    returns_data: false,
                    undoable: false,
                }]
            }
        }
        let result = std::panic::catch_unwind(|| {
            let (mut registry, _rx) = CommandRegistry::new();
            registry.register_module(&BadModule);
        });
        assert!(result.is_err(), "should panic on bad prefix");
    }

    #[test]
    #[should_panic(expected = "has already been registered")]
    fn register_module_panics_on_double_registration() {
        struct Mod;
        impl EditorModule for Mod {
            fn id(&self) -> &str { "foo" }
            fn commands(&self) -> Vec<CommandSpec> {
                vec![CommandSpec { id: "foo.bar".into(), module_id: String::new(), label: "Bar".into(), category: "General".into(), description: None, keybind: None, args_schema: None, returns_data: false, undoable: false }]
            }
        }
        let (mut reg, _rx) = CommandRegistry::new();
        reg.register_module(&Mod);
        reg.register_module(&Mod); // should panic
    }

    #[test]
    fn watch_receiver_receives_update() {
        let (mut registry, rx) = CommandRegistry::new();
        // Initially empty
        assert!(rx.borrow().is_empty());
        registry.register_module(&TestModule);
        // After registration, receiver sees the new snapshot
        assert_eq!(rx.borrow().len(), 1);
    }

    #[test]
    fn undoable_commands_are_scene_and_template_execute() {
        use crate::bridge::modules::*;
        let (mut reg, _rx) = CommandRegistry::new();
        reg.register_module(&SceneModule);
        reg.register_module(&TemplateModule);
        reg.register_module(&EditModule);
        reg.register_module(&FileModule);

        let undoable: Vec<&str> = reg.undoable_commands().iter().map(|c| c.id.as_str()).collect();

        // Scene mutations must be undoable
        assert!(undoable.contains(&"scene.new_entity"));
        assert!(undoable.contains(&"scene.delete_entity"));
        assert!(undoable.contains(&"scene.duplicate_entity"));
        assert!(undoable.contains(&"template.execute"));

        // These must NOT be undoable
        assert!(!undoable.contains(&"scene.focus_entity"));
        assert!(!undoable.contains(&"edit.undo"));
        assert!(!undoable.contains(&"edit.redo"));
        assert!(!undoable.contains(&"file.save_scene"));
    }
}
