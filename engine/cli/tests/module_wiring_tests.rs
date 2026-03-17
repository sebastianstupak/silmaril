use silm::codegen::module_wiring::*;

#[test]
fn test_generate_wiring_block() {
    let block = generate_wiring_block("combat", "silmaril-module-combat", "1.2.3", "CombatModule", "CombatModule::new()");
    assert!(block.contains("// --- silmaril module: combat (silmaril-module-combat v1.2.3) ---"));
    assert!(block.contains("use silmaril_module_combat::CombatModule;"));
    assert!(block.contains("// TODO: register \u{2192} world.add_module(CombatModule::new());"));
}

#[test]
fn test_has_wiring_block_found() {
    let content = "// --- silmaril module: combat (silmaril-module-combat v1.2.3) ---\nuse silmaril_module_combat::CombatModule;\n";
    assert!(has_wiring_block(content, "combat"));
}

#[test]
fn test_has_wiring_block_not_found() {
    let content = "// some other code\n";
    assert!(!has_wiring_block(content, "combat"));
}

#[test]
fn test_remove_wiring_block_single() {
    let content = "// --- silmaril module: combat (silmaril-module-combat v1.2.3) ---\nuse silmaril_module_combat::CombatModule;\n// TODO: register \u{2192} world.add_module(CombatModule::new());\n\nfn main() {}\n";
    let result = remove_wiring_block(content, "combat");
    assert!(!result.contains("// --- silmaril module: combat"));
    assert!(!result.contains("CombatModule"));
    assert!(result.contains("fn main() {}"));
}

#[test]
fn test_remove_wiring_block_adjacent_blocks() {
    let content = "// --- silmaril module: combat (silmaril-module-combat v1.0.0) ---\nuse combat;\n// --- silmaril module: health (silmaril-module-health v1.0.0) ---\nuse health;\n";
    let result = remove_wiring_block(content, "combat");
    assert!(!result.contains("use combat;"));
    assert!(result.contains("// --- silmaril module: health ("));
    assert!(result.contains("use health;"));
}

#[test]
fn test_parse_cargo_lock_version_found() {
    let lock = "[[package]]\nname = \"silmaril-module-combat\"\nversion = \"1.2.3\"\nsource = \"registry+...\"\n";
    assert_eq!(parse_cargo_lock_version(lock, "silmaril-module-combat"), Some("1.2.3".to_string()));
}

#[test]
fn test_parse_cargo_lock_version_not_found() {
    let lock = "[[package]]\nname = \"some-other-crate\"\nversion = \"1.0.0\"\n";
    assert_eq!(parse_cargo_lock_version(lock, "silmaril-module-combat"), None);
}

#[test]
fn test_module_type_from_name() {
    assert_eq!(module_type_from_name("combat"), "CombatModule");
    assert_eq!(module_type_from_name("health_regen"), "HealthRegenModule");
    assert_eq!(module_type_from_name("my_module"), "MyModuleModule");
}

#[test]
fn test_crate_name_from_module_name() {
    assert_eq!(crate_name_from_module_name("combat"), "silmaril-module-combat");
    assert_eq!(crate_name_from_module_name("health_regen"), "silmaril-module-health-regen");
}

#[test]
fn test_read_module_metadata_found() {
    let cargo_toml = r#"
[package]
name = "my-combat"
version = "1.0.0"

[package.metadata.silmaril]
module_type = "MyCombatModule"
target = "server"
init = "MyCombatModule::new()"
"#;
    let meta = parse_module_metadata(cargo_toml).unwrap();
    assert_eq!(meta.module_type, "MyCombatModule");
    assert_eq!(meta.target, "server");
    assert_eq!(meta.init, "MyCombatModule::new()");
}

#[test]
fn test_read_module_metadata_absent() {
    let cargo_toml = "[package]\nname = \"my-combat\"\nversion = \"1.0.0\"\n";
    assert!(parse_module_metadata(cargo_toml).is_none());
}

// game.toml helpers
use silm::commands::add::module::{
    game_toml_has_module, append_module_to_game_toml, remove_module_from_game_toml,
    cargo_toml_has_dep, append_dep_to_cargo_toml, remove_dep_from_cargo_toml,
    add_workspace_member, remove_workspace_member,
};

#[test]
fn test_game_toml_has_module_found() {
    let content = "[modules]\ncombat = { source = \"registry\", version = \"^1.0.0\", target = \"shared\" }\n";
    assert!(game_toml_has_module(content, "combat"));
}

#[test]
fn test_game_toml_has_module_not_found() {
    let content = "[modules]\n# empty\n";
    assert!(!game_toml_has_module(content, "combat"));
}

#[test]
fn test_append_module_to_game_toml_registry() {
    let content = "[project]\nname = \"test\"\n\n[modules]\n# modules here\n\n[dev]\n";
    let result = append_module_to_game_toml(content, "combat",
        "source = \"registry\", version = \"^1.2.0\", target = \"shared\"");
    assert!(result.contains("combat = { source = \"registry\", version = \"^1.2.0\", target = \"shared\" }"));
    assert!(result.contains("[dev]"));
}

#[test]
fn test_remove_module_from_game_toml() {
    let content = "[modules]\ncombat = { source = \"registry\", version = \"^1.0.0\", target = \"shared\" }\nhealth = { source = \"registry\", version = \"^1.0.0\", target = \"shared\" }\n";
    let result = remove_module_from_game_toml(content, "combat");
    assert!(!result.contains("combat ="));
    assert!(result.contains("health ="));
}

#[test]
fn test_cargo_has_dep_found() {
    let content = "[dependencies]\nsome-crate = \"1.0\"\n";
    assert!(cargo_toml_has_dep(content, "some-crate"));
}

#[test]
fn test_cargo_has_dep_not_found() {
    let content = "[dependencies]\n# empty\n";
    assert!(!cargo_toml_has_dep(content, "some-crate"));
}

#[test]
fn test_cargo_append_dep() {
    let content = "[package]\nname = \"foo\"\n\n[dependencies]\n";
    let result = append_dep_to_cargo_toml(content, "combat", "\"^1.0\"");
    assert!(result.contains("combat = \"^1.0\""));
    assert!(result.contains("[package]"));
}

#[test]
fn test_cargo_remove_dep_scoped() {
    // Should only remove from [dependencies], not [dev-dependencies]
    let content = "[dependencies]\ncombat = \"^1.0\"\nhealth = \"^1.0\"\n\n[dev-dependencies]\ncombat = \"^1.0\"\n";
    let result = remove_dep_from_cargo_toml(content, "combat");
    assert!(!result.contains("[dependencies]\ncombat"), "combat still in [dependencies]");
    assert!(result.contains("health ="), "health was incorrectly removed");
    assert!(result.contains("[dev-dependencies]\ncombat"), "combat incorrectly removed from [dev-dependencies]");
}

#[test]
fn test_add_workspace_member() {
    let content = "[workspace]\nmembers = [\n    \"shared\",\n]\n";
    let result = add_workspace_member(content, "modules/combat");
    assert!(result.contains("\"modules/combat\""));
    assert!(result.contains("\"shared\""));
}

#[test]
fn test_remove_workspace_member() {
    let content = "[workspace]\nmembers = [\n    \"shared\",\n    \"modules/combat\",\n]\n";
    let result = remove_workspace_member(content, "modules/combat");
    assert!(!result.contains("modules/combat"));
    assert!(result.contains("\"shared\""));
}
