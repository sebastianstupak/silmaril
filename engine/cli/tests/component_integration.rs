use std::fs;
use std::sync::Mutex;
use tempfile::TempDir;

use silm::commands::add::wiring::Target;

// Serialize tests that call `env::set_current_dir` — that is process-global state.
static CWD_LOCK: Mutex<()> = Mutex::new(());

/// Helper to create a minimal project structure for testing
fn make_project(tmp: &TempDir) -> std::path::PathBuf {
    let root = tmp.path().to_path_buf();
    fs::write(root.join("game.toml"), "[game]\nname = \"test\"").unwrap();
    fs::create_dir_all(root.join("shared/src")).unwrap();
    fs::write(root.join("shared/src/lib.rs"), "").unwrap();
    fs::create_dir_all(root.join("client/src")).unwrap();
    fs::write(root.join("client/src/main.rs"), "").unwrap();
    fs::create_dir_all(root.join("server/src")).unwrap();
    fs::write(root.join("server/src/main.rs"), "").unwrap();
    root
}

#[test]
fn test_add_component_basic() {
    let tmp = TempDir::new().unwrap();
    let root = make_project(&tmp);

    let _guard = CWD_LOCK.lock().unwrap();
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&root).unwrap();

    let result = silm::commands::add::component::add_component(
        "Health",
        "current:f32,max:f32",
        Target::Shared,
        "health",
    );

    std::env::set_current_dir(&original_dir).unwrap();
    drop(_guard);

    assert!(result.is_ok(), "Component generation failed: {:?}", result);

    // Verify domain file created
    let domain_file = root.join("shared/src/health/mod.rs");
    assert!(domain_file.exists(), "Domain file not created: {}", domain_file.display());

    // Verify content
    let content = fs::read_to_string(&domain_file).unwrap();
    assert!(content.contains("pub struct Health"));
    assert!(content.contains("pub current: f32"));
    assert!(content.contains("pub max: f32"));
}

#[test]
fn test_add_component_client_target() {
    let tmp = TempDir::new().unwrap();
    let root = make_project(&tmp);

    let _guard = CWD_LOCK.lock().unwrap();
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&root).unwrap();

    let result = silm::commands::add::component::add_component(
        "CameraState",
        "fov:f32,zoom:f32",
        Target::Client,
        "camera",
    );

    std::env::set_current_dir(&original_dir).unwrap();
    drop(_guard);

    assert!(result.is_ok(), "Component generation failed: {:?}", result);

    let domain_file = root.join("client/src/camera/mod.rs");
    assert!(domain_file.exists());

    let content = fs::read_to_string(&domain_file).unwrap();
    assert!(content.contains("pub struct CameraState"));
}

#[test]
fn test_add_component_server_target() {
    let tmp = TempDir::new().unwrap();
    let root = make_project(&tmp);

    let _guard = CWD_LOCK.lock().unwrap();
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&root).unwrap();

    let result = silm::commands::add::component::add_component(
        "ServerState",
        "tick:u64",
        Target::Server,
        "state",
    );

    std::env::set_current_dir(&original_dir).unwrap();
    drop(_guard);

    assert!(result.is_ok(), "Component generation failed: {:?}", result);

    let domain_file = root.join("server/src/state/mod.rs");
    assert!(domain_file.exists());
}

#[test]
fn test_add_component_with_vec_type() {
    let tmp = TempDir::new().unwrap();
    let root = make_project(&tmp);

    let _guard = CWD_LOCK.lock().unwrap();
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&root).unwrap();

    let result = silm::commands::add::component::add_component(
        "Inventory",
        "items:Vec<Item>,capacity:usize",
        Target::Shared,
        "inventory",
    );

    std::env::set_current_dir(&original_dir).unwrap();
    drop(_guard);

    assert!(result.is_ok(), "Component generation failed: {:?}", result);

    let domain_file = root.join("shared/src/inventory/mod.rs");
    let content = fs::read_to_string(&domain_file).unwrap();
    assert!(content.contains("pub items: Vec<Item>"));
    assert!(content.contains("pub capacity: usize"));
    assert!(content.contains("items: Vec::new()"));
}

#[test]
fn test_add_component_with_array_type() {
    let tmp = TempDir::new().unwrap();
    let root = make_project(&tmp);

    let _guard = CWD_LOCK.lock().unwrap();
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&root).unwrap();

    let result = silm::commands::add::component::add_component(
        "Transform",
        "position:[f32;3],rotation:[f32;4],scale:[f32;3]",
        Target::Shared,
        "transform",
    );

    std::env::set_current_dir(&original_dir).unwrap();
    drop(_guard);

    assert!(result.is_ok(), "Component generation failed: {:?}", result);

    let domain_file = root.join("shared/src/transform/mod.rs");
    let content = fs::read_to_string(&domain_file).unwrap();
    assert!(content.contains("pub position: [f32;3]"));
    assert!(content.contains("position: [0.0, 0.0, 0.0]"));
}

#[test]
fn test_add_component_duplicate_error() {
    let tmp = TempDir::new().unwrap();
    let root = make_project(&tmp);

    let _guard = CWD_LOCK.lock().unwrap();
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&root).unwrap();

    // Create component first time
    let result1 = silm::commands::add::component::add_component(
        "Health",
        "hp:f32",
        Target::Shared,
        "health",
    );
    assert!(result1.is_ok());

    // Try to create again in same domain
    let result2 = silm::commands::add::component::add_component(
        "Health",
        "hp:f32",
        Target::Shared,
        "health",
    );

    std::env::set_current_dir(&original_dir).unwrap();
    drop(_guard);

    assert!(result2.is_err(), "Should fail when component already exists");
    assert!(result2.unwrap_err().to_string().contains("already exists"));
}

#[test]
fn test_add_component_invalid_name() {
    let tmp = TempDir::new().unwrap();
    let root = make_project(&tmp);

    let _guard = CWD_LOCK.lock().unwrap();
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&root).unwrap();

    // Lowercase name is invalid PascalCase
    let result = silm::commands::add::component::add_component(
        "health",
        "hp:f32",
        Target::Shared,
        "health",
    );

    std::env::set_current_dir(&original_dir).unwrap();
    drop(_guard);

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("must start with uppercase"));
}

#[test]
fn test_add_component_no_project_root() {
    let tmp = TempDir::new().unwrap();
    // No game.toml created

    let _guard = CWD_LOCK.lock().unwrap();
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(tmp.path()).unwrap();

    let result = silm::commands::add::component::add_component(
        "Health",
        "hp:f32",
        Target::Shared,
        "health",
    );

    std::env::set_current_dir(&original_dir).unwrap();
    drop(_guard);

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("game.toml"));
}

#[test]
fn test_add_component_empty_fields() {
    let tmp = TempDir::new().unwrap();
    let root = make_project(&tmp);

    let _guard = CWD_LOCK.lock().unwrap();
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&root).unwrap();

    let result = silm::commands::add::component::add_component(
        "Empty",
        "",
        Target::Shared,
        "empty",
    );

    std::env::set_current_dir(&original_dir).unwrap();
    drop(_guard);

    assert!(result.is_err());
}

#[test]
fn test_add_component_invalid_field_format() {
    let tmp = TempDir::new().unwrap();
    let root = make_project(&tmp);

    let _guard = CWD_LOCK.lock().unwrap();
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&root).unwrap();

    // Invalid format: no colon separator
    let result = silm::commands::add::component::add_component(
        "Health",
        "hp",
        Target::Shared,
        "health",
    );

    std::env::set_current_dir(&original_dir).unwrap();
    drop(_guard);

    assert!(result.is_err());
}

#[test]
fn test_add_component_wires_mod_declaration() {
    let tmp = TempDir::new().unwrap();
    let root = make_project(&tmp);

    let _guard = CWD_LOCK.lock().unwrap();
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&root).unwrap();

    let result = silm::commands::add::component::add_component(
        "Health",
        "hp:f32",
        Target::Shared,
        "health",
    );

    std::env::set_current_dir(&original_dir).unwrap();
    drop(_guard);

    assert!(result.is_ok());

    // Verify lib.rs was updated with module declaration
    let lib_rs = root.join("shared/src/lib.rs");
    let content = fs::read_to_string(&lib_rs).unwrap();
    assert!(content.contains("pub mod health;"), "lib.rs missing 'pub mod health;'");
}
