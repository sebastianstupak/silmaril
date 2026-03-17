//! Integration tests for `silm add component` and `silm add system`.
//!
//! Each test creates a minimal project in a temp dir, runs the add function,
//! and asserts the generated file content and wiring.

use silm::commands::add::wiring::Target;
use std::fs;
use tempfile::TempDir;

// Serialize access to set_current_dir since it's process-global
static CWD_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn make_project(tmp: &TempDir) -> std::path::PathBuf {
    let root = tmp.path().to_path_buf();
    fs::write(root.join("game.toml"), "[game]\nname=\"test\"\n").unwrap();
    fs::create_dir_all(root.join("shared/src")).unwrap();
    fs::write(root.join("shared/src/lib.rs"), "// shared lib\n").unwrap();
    fs::create_dir_all(root.join("server/src")).unwrap();
    fs::write(root.join("server/src/main.rs"), "fn main() {}\n").unwrap();
    fs::create_dir_all(root.join("client/src")).unwrap();
    fs::write(root.join("client/src/main.rs"), "fn main() {}\n").unwrap();
    root
}

#[test]
fn test_add_component_creates_domain_file() {
    let _lock = CWD_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    let root = make_project(&tmp);
    std::env::set_current_dir(&root).unwrap();

    silm::commands::add::component::add_component(
        "Health",
        "current:f32,max:f32",
        Target::Shared,
        "health",
    )
    .unwrap();

    let domain_file = root.join("shared/src/health/mod.rs");
    assert!(domain_file.exists(), "domain file should be created");
    let content = fs::read_to_string(&domain_file).unwrap();
    assert!(content.contains("pub struct Health {"));
    assert!(content.contains("pub current: f32,"));
    assert!(content.contains("pub max: f32,"));
    assert!(content.contains("Component, Debug, Clone, PartialEq, Serialize, Deserialize"));
    assert!(content.contains("mod health_tests {"));
}

#[test]
fn test_add_component_wires_lib_rs() {
    let _lock = CWD_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    let root = make_project(&tmp);
    std::env::set_current_dir(&root).unwrap();

    silm::commands::add::component::add_component(
        "Health",
        "current:f32",
        Target::Shared,
        "health",
    )
    .unwrap();

    let lib = fs::read_to_string(root.join("shared/src/lib.rs")).unwrap();
    assert!(lib.contains("pub mod health;"));
}

#[test]
fn test_add_component_wires_main_rs_for_server() {
    let _lock = CWD_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    let root = make_project(&tmp);
    std::env::set_current_dir(&root).unwrap();

    silm::commands::add::component::add_component(
        "Damage",
        "amount:f32",
        Target::Server,
        "combat",
    )
    .unwrap();

    let main = fs::read_to_string(root.join("server/src/main.rs")).unwrap();
    assert!(main.contains("pub mod combat;"));
}

#[test]
fn test_add_component_duplicate_rejected() {
    let _lock = CWD_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    let root = make_project(&tmp);
    std::env::set_current_dir(&root).unwrap();

    silm::commands::add::component::add_component(
        "Health",
        "current:f32",
        Target::Shared,
        "health",
    )
    .unwrap();
    let result = silm::commands::add::component::add_component(
        "Health",
        "max:f32",
        Target::Shared,
        "health",
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("already exists"));
}

#[test]
fn test_add_two_components_same_domain() {
    let _lock = CWD_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    let root = make_project(&tmp);
    std::env::set_current_dir(&root).unwrap();

    silm::commands::add::component::add_component(
        "Health",
        "current:f32,max:f32",
        Target::Shared,
        "health",
    )
    .unwrap();
    silm::commands::add::component::add_component(
        "MaxHealth",
        "value:f32",
        Target::Shared,
        "health",
    )
    .unwrap();

    let content = fs::read_to_string(root.join("shared/src/health/mod.rs")).unwrap();
    assert!(content.contains("pub struct Health {"));
    assert!(content.contains("pub struct MaxHealth {"));

    // lib.rs wired only once
    let lib = fs::read_to_string(root.join("shared/src/lib.rs")).unwrap();
    assert_eq!(lib.matches("pub mod health;").count(), 1);
}

#[test]
fn test_add_system_creates_domain_file() {
    let _lock = CWD_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    let root = make_project(&tmp);
    std::env::set_current_dir(&root).unwrap();

    silm::commands::add::system::add_system(
        "health_regen",
        "mut:Health,RegenerationRate",
        Target::Shared,
        "health",
    )
    .unwrap();

    let content = fs::read_to_string(root.join("shared/src/health/mod.rs")).unwrap();
    assert!(content.contains("pub fn health_regen_system("));
    assert!(content.contains("dt: f32"));
    assert!(content.contains("mod health_regen_system_tests {"));
    assert!(content.contains("// To register: app.add_system(health_regen_system)"));
}

#[test]
fn test_add_component_then_system_same_domain() {
    let _lock = CWD_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    let root = make_project(&tmp);
    std::env::set_current_dir(&root).unwrap();

    silm::commands::add::component::add_component(
        "Health",
        "current:f32,max:f32",
        Target::Shared,
        "health",
    )
    .unwrap();
    silm::commands::add::system::add_system(
        "health_regen",
        "mut:Health,RegenerationRate",
        Target::Shared,
        "health",
    )
    .unwrap();

    let content = fs::read_to_string(root.join("shared/src/health/mod.rs")).unwrap();
    assert!(content.contains("pub struct Health {"));
    assert!(content.contains("pub fn health_regen_system("));

    // lib.rs wired exactly once
    let lib = fs::read_to_string(root.join("shared/src/lib.rs")).unwrap();
    assert_eq!(lib.matches("pub mod health;").count(), 1);
}

#[test]
fn test_add_system_duplicate_rejected() {
    let _lock = CWD_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    let root = make_project(&tmp);
    std::env::set_current_dir(&root).unwrap();

    silm::commands::add::system::add_system(
        "health_regen",
        "mut:Health",
        Target::Shared,
        "health",
    )
    .unwrap();
    let result = silm::commands::add::system::add_system(
        "health_regen",
        "mut:Health",
        Target::Shared,
        "health",
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("already exists"));
}

#[test]
fn test_missing_crate_dir_errors_clearly() {
    let _lock = CWD_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    let root = make_project(&tmp);
    std::env::set_current_dir(&root).unwrap();

    fs::remove_dir_all(root.join("server")).unwrap();
    let result = silm::commands::add::component::add_component(
        "Health",
        "hp:f32",
        Target::Server,
        "health",
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("server/"));
}
