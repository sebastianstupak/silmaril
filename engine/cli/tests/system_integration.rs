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
fn test_system_generation_creates_domain_file() {
    let tmp = TempDir::new().unwrap();
    let root = make_project(&tmp);

    let _guard = CWD_LOCK.lock().unwrap();
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&root).unwrap();

    let result = silm::commands::add::system::add_system(
        "health_regen",
        "mut:Health,RegenerationRate",
        Target::Shared,
        "health",
    );

    std::env::set_current_dir(&original_dir).unwrap();
    drop(_guard);

    assert!(result.is_ok(), "System generation failed: {:?}", result);

    let domain_file = root.join("shared/src/health/mod.rs");
    assert!(domain_file.exists(), "Domain file not created: {}", domain_file.display());
}

#[test]
fn test_generated_system_code_structure() {
    let tmp = TempDir::new().unwrap();
    let root = make_project(&tmp);

    let _guard = CWD_LOCK.lock().unwrap();
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&root).unwrap();

    let result = silm::commands::add::system::add_system(
        "health_regen",
        "mut:Health,RegenerationRate",
        Target::Shared,
        "health",
    );

    std::env::set_current_dir(&original_dir).unwrap();
    drop(_guard);

    assert!(result.is_ok(), "System generation failed: {:?}", result);

    let domain_file = root.join("shared/src/health/mod.rs");
    let content = fs::read_to_string(&domain_file).unwrap();

    // Verify imports
    assert!(content.contains("use engine_core::ecs::World"));

    // Verify function signature with _system suffix and dt parameter
    assert!(content.contains("pub fn health_regen_system(world: &mut World, dt: f32)"));

    // Verify query
    assert!(content.contains("world.query_mut::<(&mut Health, &RegenerationRate)>()"));

    // Verify tests
    assert!(content.contains("#[cfg(test)]"));
    assert!(content.contains("mod health_regen_system_tests {"));
    assert!(content.contains("fn test_health_regen_system()"));
}

#[test]
fn test_system_generation_invalid_name() {
    let tmp = TempDir::new().unwrap();
    let root = make_project(&tmp);

    let _guard = CWD_LOCK.lock().unwrap();
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&root).unwrap();

    // PascalCase is invalid for system names (must be snake_case)
    let result = silm::commands::add::system::add_system(
        "HealthRegen",
        "mut:Health",
        Target::Shared,
        "health",
    );

    std::env::set_current_dir(&original_dir).unwrap();
    drop(_guard);

    assert!(result.is_err());
}

#[test]
fn test_system_generation_invalid_query_old_syntax() {
    let tmp = TempDir::new().unwrap();
    let root = make_project(&tmp);

    let _guard = CWD_LOCK.lock().unwrap();
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&root).unwrap();

    // Old & syntax is invalid
    let result = silm::commands::add::system::add_system(
        "health_regen",
        "&Health",
        Target::Shared,
        "health",
    );

    std::env::set_current_dir(&original_dir).unwrap();
    drop(_guard);

    assert!(result.is_err());
}

#[test]
fn test_system_generation_server_target() {
    let tmp = TempDir::new().unwrap();
    let root = make_project(&tmp);

    let _guard = CWD_LOCK.lock().unwrap();
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&root).unwrap();

    let result = silm::commands::add::system::add_system(
        "physics_step",
        "mut:Transform,Velocity",
        Target::Server,
        "physics",
    );

    std::env::set_current_dir(&original_dir).unwrap();
    drop(_guard);

    assert!(result.is_ok(), "System generation failed: {:?}", result);

    let domain_file = root.join("server/src/physics/mod.rs");
    assert!(domain_file.exists());
}

#[test]
fn test_system_duplicate_error() {
    let tmp = TempDir::new().unwrap();
    let root = make_project(&tmp);

    let _guard = CWD_LOCK.lock().unwrap();
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&root).unwrap();

    // Create first time
    let result1 = silm::commands::add::system::add_system(
        "health_regen",
        "mut:Health",
        Target::Shared,
        "health",
    );
    assert!(result1.is_ok());

    // Try again in same domain
    let result2 = silm::commands::add::system::add_system(
        "health_regen",
        "mut:Health",
        Target::Shared,
        "health",
    );

    std::env::set_current_dir(&original_dir).unwrap();
    drop(_guard);

    assert!(result2.is_err(), "Should fail when system already exists");
    assert!(result2.unwrap_err().to_string().contains("already exists"));
}

#[test]
fn test_system_wires_mod_declaration() {
    let tmp = TempDir::new().unwrap();
    let root = make_project(&tmp);

    let _guard = CWD_LOCK.lock().unwrap();
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&root).unwrap();

    let result = silm::commands::add::system::add_system(
        "health_regen",
        "mut:Health",
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
