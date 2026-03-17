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
