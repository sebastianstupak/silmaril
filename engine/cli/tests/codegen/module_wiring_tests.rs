use silm::codegen::module_wiring::*;

#[test]
fn test_generate_wiring_block() {
    let block = generate_wiring_block("combat", "silmaril-module-combat", "1.2.3", "CombatModule", "CombatModule::new()");
    assert!(block.contains("// --- silmaril module: combat (silmaril-module-combat v1.2.3) ---"));
    assert!(block.contains("use silmaril_module_combat::CombatModule;"));
    assert!(block.contains("// TODO: register → world.add_module(CombatModule::new());"));
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
    let content = "// --- silmaril module: combat (silmaril-module-combat v1.2.3) ---\nuse silmaril_module_combat::CombatModule;\n// TODO: register → world.add_module(CombatModule::new());\n\nfn main() {}\n";
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
