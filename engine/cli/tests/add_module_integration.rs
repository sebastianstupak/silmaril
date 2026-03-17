use std::fs;
use std::sync::Mutex;
use tempfile::TempDir;

static CWD_LOCK: Mutex<()> = Mutex::new(());

fn make_project(dir: &TempDir) {
    fs::write(
        dir.path().join("game.toml"),
        "[project]\nname = \"test-game\"\n\n[modules]\n# modules\n\n[dev]\nserver_package = \"test-game-server\"\nclient_package = \"test-game-client\"\ndev_server_port = 9999\ndev_client_port = 9998\n",
    ).unwrap();
    fs::create_dir_all(dir.path().join("shared/src")).unwrap();
    fs::write(
        dir.path().join("shared/Cargo.toml"),
        "[package]\nname = \"test-game-shared\"\nversion = \"0.1.0\"\n\n[dependencies]\n",
    ).unwrap();
    fs::write(dir.path().join("shared/src/lib.rs"), "// shared lib\n").unwrap();
    fs::create_dir_all(dir.path().join("server/src")).unwrap();
    fs::write(
        dir.path().join("server/Cargo.toml"),
        "[package]\nname = \"test-game-server\"\nversion = \"0.1.0\"\n\n[dependencies]\n",
    ).unwrap();
    fs::write(dir.path().join("server/src/main.rs"), "fn main() {}\n").unwrap();
    fs::write(
        dir.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\n    \"shared\",\n    \"server\",\n]\n",
    ).unwrap();
}

#[test]
fn test_add_module_registry_shared() {
    let _lock = CWD_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    make_project(&dir);
    std::env::set_current_dir(dir.path()).unwrap();

    silm::commands::add::module::add_module(
        "combat", None, None, None, None, false,
        silm::commands::add::wiring::Target::Shared,
    ).unwrap();

    let cargo = fs::read_to_string(dir.path().join("shared/Cargo.toml")).unwrap();
    assert!(cargo.contains("silmaril-module-combat"), "dep not in shared/Cargo.toml");

    let lib = fs::read_to_string(dir.path().join("shared/src/lib.rs")).unwrap();
    assert!(lib.contains("// --- silmaril module: combat"), "wiring block missing");
    assert!(lib.contains("use silmaril_module_combat::CombatModule;"), "use statement missing");

    let game = fs::read_to_string(dir.path().join("game.toml")).unwrap();
    assert!(game.contains("combat ="), "game.toml entry missing");
    assert!(game.contains("source = \"registry\""), "source not registry");
}

#[test]
fn test_add_module_duplicate_rejected() {
    let _lock = CWD_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    make_project(&dir);
    std::env::set_current_dir(dir.path()).unwrap();

    silm::commands::add::module::add_module(
        "combat", None, None, None, None, false,
        silm::commands::add::wiring::Target::Shared,
    ).unwrap();

    let result = silm::commands::add::module::add_module(
        "combat", None, None, None, None, false,
        silm::commands::add::wiring::Target::Shared,
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("already installed"));
}

#[test]
fn test_add_module_git_tag() {
    let _lock = CWD_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    make_project(&dir);
    std::env::set_current_dir(dir.path()).unwrap();

    silm::commands::add::module::add_module(
        "combat",
        Some("https://github.com/org/combat"),
        Some("v1.0.0"),
        None,
        None,
        false,
        silm::commands::add::wiring::Target::Shared,
    ).unwrap();

    let cargo = fs::read_to_string(dir.path().join("shared/Cargo.toml")).unwrap();
    assert!(cargo.contains("git = \"https://github.com/org/combat\""));
    assert!(cargo.contains("tag = \"v1.0.0\""));

    let game = fs::read_to_string(dir.path().join("game.toml")).unwrap();
    assert!(game.contains("source = \"git\""));
    assert!(game.contains("tag = \"v1.0.0\""));
}

#[test]
fn test_add_module_path() {
    let _lock = CWD_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    make_project(&dir);

    let module_dir = TempDir::new().unwrap();
    fs::write(
        module_dir.path().join("Cargo.toml"),
        "[package]\nname = \"my-combat\"\nversion = \"1.0.0\"\n\n[dependencies]\n",
    ).unwrap();
    fs::create_dir_all(module_dir.path().join("src")).unwrap();
    fs::write(module_dir.path().join("src/lib.rs"), "").unwrap();

    std::env::set_current_dir(dir.path()).unwrap();

    silm::commands::add::module::add_module(
        "combat",
        None,
        None,
        None,
        Some(module_dir.path().to_str().unwrap()),
        false,
        silm::commands::add::wiring::Target::Shared,
    ).unwrap();

    let cargo = fs::read_to_string(dir.path().join("shared/Cargo.toml")).unwrap();
    assert!(cargo.contains("path = "), "no path dep");
    assert!(cargo.contains("my-combat"), "wrong crate name");

    let game = fs::read_to_string(dir.path().join("game.toml")).unwrap();
    assert!(game.contains("source = \"local\""));
}

#[test]
fn test_add_module_vendor() {
    let _lock = CWD_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    make_project(&dir);

    // Create a fake upstream module (just a directory with Cargo.toml)
    let upstream = TempDir::new().unwrap();
    fs::write(
        upstream.path().join("Cargo.toml"),
        "[package]\nname = \"silmaril-module-combat\"\nversion = \"1.0.0\"\n\n[dependencies]\n",
    ).unwrap();
    fs::create_dir_all(upstream.path().join("src")).unwrap();
    fs::write(upstream.path().join("src/lib.rs"), "pub struct CombatModule;\n").unwrap();

    std::env::set_current_dir(dir.path()).unwrap();

    silm::commands::add::module::add_module_vendor_from_path(
        "combat",
        upstream.path(),
        silm::commands::add::wiring::Target::Shared,
        &dir.path().to_path_buf(),
    ).unwrap();

    // modules/combat/ should exist with the vendored files
    assert!(dir.path().join("modules/combat/Cargo.toml").exists());
    assert!(dir.path().join("modules/combat/src/lib.rs").exists());

    // Root Cargo.toml should have workspace member
    let root_cargo = fs::read_to_string(dir.path().join("Cargo.toml")).unwrap();
    assert!(root_cargo.contains("modules/combat"), "workspace member missing");

    // shared/Cargo.toml should have path dep
    let shared_cargo = fs::read_to_string(dir.path().join("shared/Cargo.toml")).unwrap();
    assert!(shared_cargo.contains("path ="), "no path dep in shared/Cargo.toml");
    assert!(shared_cargo.contains("silmaril-module-combat"), "wrong crate name in dep");

    // shared/src/lib.rs should have wiring block
    let lib = fs::read_to_string(dir.path().join("shared/src/lib.rs")).unwrap();
    assert!(lib.contains("// --- silmaril module: combat"), "wiring block missing");

    // game.toml should have vendor entry
    let game = fs::read_to_string(dir.path().join("game.toml")).unwrap();
    assert!(game.contains("source = \"vendor\""), "source not vendor");
    assert!(game.contains("crate = \"silmaril-module-combat\""), "crate name not stored");
}

#[test]
fn test_module_list_empty() {
    let _lock = CWD_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    make_project(&dir);
    std::env::set_current_dir(dir.path()).unwrap();

    // Should not error on empty [modules]
    let result = silm::commands::module::list::list_modules(&dir.path().to_path_buf());
    assert!(result.is_ok());
}

#[test]
fn test_module_list_after_add() {
    let _lock = CWD_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    make_project(&dir);
    std::env::set_current_dir(dir.path()).unwrap();

    silm::commands::add::module::add_module(
        "combat", None, None, None, None, false,
        silm::commands::add::wiring::Target::Shared,
    ).unwrap();

    // list should not error even without Cargo.lock (resolves to "?")
    let result = silm::commands::module::list::list_modules(&dir.path().to_path_buf());
    assert!(result.is_ok());
}

#[test]
fn test_module_remove() {
    let _lock = CWD_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    make_project(&dir);
    std::env::set_current_dir(dir.path()).unwrap();

    silm::commands::add::module::add_module(
        "combat", None, None, None, None, false,
        silm::commands::add::wiring::Target::Shared,
    ).unwrap();

    silm::commands::module::remove::remove_module("combat", &dir.path().to_path_buf()).unwrap();

    let cargo = fs::read_to_string(dir.path().join("shared/Cargo.toml")).unwrap();
    assert!(!cargo.contains("silmaril-module-combat"), "dep still in Cargo.toml");

    let lib = fs::read_to_string(dir.path().join("shared/src/lib.rs")).unwrap();
    assert!(!lib.contains("// --- silmaril module: combat"), "wiring block still present");

    let game = fs::read_to_string(dir.path().join("game.toml")).unwrap();
    assert!(!game.contains("combat ="), "game.toml entry still present");
}

#[test]
fn test_module_remove_not_installed() {
    let _lock = CWD_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    make_project(&dir);
    std::env::set_current_dir(dir.path()).unwrap();

    let result = silm::commands::module::remove::remove_module("combat", &dir.path().to_path_buf());
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not installed"));
}

#[test]
fn test_wiring_block_idempotent() {
    let _lock = CWD_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    make_project(&dir);
    std::env::set_current_dir(dir.path()).unwrap();

    silm::commands::add::module::add_module(
        "combat", None, None, None, None, false,
        silm::commands::add::wiring::Target::Shared,
    ).unwrap();

    // Second add with different module, should not duplicate wiring for first
    silm::commands::add::module::add_module(
        "health", None, None, None, None, false,
        silm::commands::add::wiring::Target::Shared,
    ).unwrap();

    let lib = fs::read_to_string(dir.path().join("shared/src/lib.rs")).unwrap();
    let count = lib.matches("// --- silmaril module: combat").count();
    assert_eq!(count, 1, "wiring block should appear exactly once");
}

#[test]
fn test_add_module_server_target() {
    let _lock = CWD_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    make_project(&dir);
    std::env::set_current_dir(dir.path()).unwrap();

    silm::commands::add::module::add_module(
        "combat", None, None, None, None, false,
        silm::commands::add::wiring::Target::Server,
    ).unwrap();

    let cargo = fs::read_to_string(dir.path().join("server/Cargo.toml")).unwrap();
    assert!(cargo.contains("silmaril-module-combat"));

    let main_rs = fs::read_to_string(dir.path().join("server/src/main.rs")).unwrap();
    assert!(main_rs.contains("// --- silmaril module: combat"));

    let game = fs::read_to_string(dir.path().join("game.toml")).unwrap();
    assert!(game.contains("target = \"server\""));
}

#[test]
fn test_remove_preserves_adjacent_modules() {
    let _lock = CWD_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    make_project(&dir);
    std::env::set_current_dir(dir.path()).unwrap();

    silm::commands::add::module::add_module(
        "combat", None, None, None, None, false,
        silm::commands::add::wiring::Target::Shared,
    ).unwrap();
    silm::commands::add::module::add_module(
        "health", None, None, None, None, false,
        silm::commands::add::wiring::Target::Shared,
    ).unwrap();

    silm::commands::module::remove::remove_module("combat", &dir.path().to_path_buf()).unwrap();

    let lib = fs::read_to_string(dir.path().join("shared/src/lib.rs")).unwrap();
    assert!(!lib.contains("// --- silmaril module: combat"), "combat block still present");
    assert!(lib.contains("// --- silmaril module: health"), "health block removed incorrectly");

    let game = fs::read_to_string(dir.path().join("game.toml")).unwrap();
    assert!(!game.contains("combat ="));
    assert!(game.contains("health ="));
}

#[test]
fn test_git_rev_pinning() {
    let _lock = CWD_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    make_project(&dir);
    std::env::set_current_dir(dir.path()).unwrap();

    silm::commands::add::module::add_module(
        "combat",
        Some("https://github.com/org/combat"),
        None,
        Some("abc123f"),
        None,
        false,
        silm::commands::add::wiring::Target::Shared,
    ).unwrap();

    let cargo = fs::read_to_string(dir.path().join("shared/Cargo.toml")).unwrap();
    assert!(cargo.contains("rev = \"abc123f\""));
    assert!(!cargo.contains("tag ="));
}
