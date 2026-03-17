//! Integration tests for silm build/package commands.
//!
//! These tests create real filesystem project structures and exercise the
//! build and package logic through a MockRunner that captures commands
//! instead of spawning real processes.

use silm::commands::build::env::parse_build_section;
use silm::commands::build::package::{
    assemble_native_dist, assemble_server_dist, create_zip,
};
use silm::commands::build::wasm::build_wasm;
use silm::commands::build::{build_all_platforms, platform_from_str, BuildRunner};

use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

// ============================================================================
// MockRunner + CapturedCommand
// ============================================================================

/// A captured command invocation for test assertions.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct CapturedCommand {
    program: String,
    args: Vec<String>,
    env: HashMap<String, String>,
    cwd: PathBuf,
}

/// Mock build runner that records commands instead of executing them.
struct MockRunner {
    commands: Arc<Mutex<Vec<CapturedCommand>>>,
}

impl MockRunner {
    fn new() -> (Self, Arc<Mutex<Vec<CapturedCommand>>>) {
        let commands = Arc::new(Mutex::new(Vec::new()));
        let runner = Self {
            commands: Arc::clone(&commands),
        };
        (runner, commands)
    }
}

impl BuildRunner for MockRunner {
    fn run_command(
        &self,
        program: &str,
        args: &[String],
        env: &HashMap<String, String>,
        cwd: &Path,
    ) -> Result<()> {
        self.commands.lock().unwrap().push(CapturedCommand {
            program: program.to_string(),
            args: args.to_vec(),
            env: env.clone(),
            cwd: cwd.to_path_buf(),
        });
        Ok(())
    }
}

// ============================================================================
// Helper: create a project directory with game.toml and client/index.html
// ============================================================================

/// Creates a temporary project directory containing a game.toml with
/// [project], [dev], [build], [build.env], and [modules] sections,
/// plus a client/index.html file.  Returns (tempdir, project_root, game_toml_content).
fn make_project() -> (tempfile::TempDir, PathBuf, String) {
    let dir = tempfile::tempdir().expect("failed to create tempdir");
    let root = dir.path().join("my-game");
    std::fs::create_dir_all(root.join("client")).unwrap();

    let game_toml = r#"[project]
name = "my-game"
version = "0.1.0"

[dev]
server_package = "my-game-server"
client_package = "my-game-client"

[build]
platforms = ["native", "wasm"]

[build.env]
_SILM_TEST_BUILD_VAR = "from_toml"
SERVER_ADDRESS = "127.0.0.1:7777"

[modules]
"#
    .to_string();

    std::fs::write(root.join("game.toml"), &game_toml).unwrap();

    let index_html = r#"<!DOCTYPE html>
<html>
<head><link data-trunk rel="rust" data-wasm-opt="z" /></head>
<body><canvas id="silmaril"></canvas></body>
</html>"#;
    std::fs::write(root.join("client").join("index.html"), index_html).unwrap();

    (dir, root, game_toml)
}

// ============================================================================
// Test 1: test_build_native_captures_cargo
// ============================================================================

#[test]
fn test_build_native_captures_cargo() {
    let (_dir, root, game_toml) = make_project();
    let (runner, commands) = MockRunner::new();

    build_all_platforms(
        &runner,
        &root,
        &game_toml,
        &["native".into()],
        false,
        None,
        true, // skip_preflight
    )
    .unwrap();

    let cmds = commands.lock().unwrap();
    // native = ServerAndClient = 2 cargo commands (server + client)
    assert_eq!(cmds.len(), 2, "expected 2 cargo commands, got {}", cmds.len());

    assert_eq!(cmds[0].program, "cargo");
    assert!(cmds[0].args.contains(&"my-game-server".to_string()));
    assert!(cmds[0].args.contains(&"--package".to_string()));
    assert!(cmds[0].args.contains(&"server".to_string()));

    assert_eq!(cmds[1].program, "cargo");
    assert!(cmds[1].args.contains(&"my-game-client".to_string()));
    assert!(cmds[1].args.contains(&"client".to_string()));
}

// ============================================================================
// Test 2: test_build_native_release_flag
// ============================================================================

#[test]
fn test_build_native_release_flag() {
    let (_dir, root, game_toml) = make_project();
    let (runner, commands) = MockRunner::new();

    build_all_platforms(
        &runner,
        &root,
        &game_toml,
        &["native".into()],
        true, // release
        None,
        true,
    )
    .unwrap();

    let cmds = commands.lock().unwrap();
    assert_eq!(cmds.len(), 2);
    for cmd in cmds.iter() {
        assert!(
            cmd.args.contains(&"--release".to_string()),
            "expected --release in args: {:?}",
            cmd.args
        );
    }
}

// ============================================================================
// Test 3: test_build_wasm_captures_trunk (call build_wasm directly)
// ============================================================================

#[test]
fn test_build_wasm_captures_trunk() {
    let (_dir, root, _game_toml) = make_project();
    let (runner, commands) = MockRunner::new();
    let env = HashMap::new();

    build_wasm(&runner, &root, &env, false).unwrap();

    let cmds = commands.lock().unwrap();
    assert_eq!(cmds.len(), 1);
    assert_eq!(cmds[0].program, "trunk");
    assert_eq!(cmds[0].args[0], "build");
    assert_eq!(cmds[0].args[1], "client/index.html");
    assert_eq!(cmds[0].args[2], "--dist");
    assert_eq!(cmds[0].args[3], "dist/wasm");
    assert!(!cmds[0].args.contains(&"--release".to_string()));
}

// ============================================================================
// Test 4: test_build_wasm_release (call build_wasm directly)
// ============================================================================

#[test]
fn test_build_wasm_release() {
    let (_dir, root, _game_toml) = make_project();
    let (runner, commands) = MockRunner::new();
    let env = HashMap::new();

    build_wasm(&runner, &root, &env, true).unwrap();

    let cmds = commands.lock().unwrap();
    assert_eq!(cmds.len(), 1);
    assert_eq!(cmds[0].program, "trunk");
    assert!(cmds[0].args.contains(&"--release".to_string()));
}

// ============================================================================
// Test 5: test_env_vars_in_subprocess
// ============================================================================

#[test]
fn test_env_vars_in_subprocess() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("env-project");
    std::fs::create_dir_all(root.join("client")).unwrap();

    // Create .env file with a test variable
    std::fs::write(root.join(".env"), "_SILM_TEST_BUILD_VAR=from_dotenv\n").unwrap();
    std::fs::write(
        root.join("client").join("index.html"),
        "<html></html>",
    )
    .unwrap();

    let game_toml = r#"[project]
name = "env-test"

[dev]
server_package = "env-test-server"
client_package = "env-test-client"
"#;
    std::fs::write(root.join("game.toml"), game_toml).unwrap();

    let (runner, commands) = MockRunner::new();

    build_all_platforms(
        &runner,
        &root,
        game_toml,
        &["native".into()],
        false,
        None,
        true,
    )
    .unwrap();

    let cmds = commands.lock().unwrap();
    assert!(!cmds.is_empty());

    // The .env var should appear in the captured command env
    // (unless the shell already has it set, in which case merge_env filters it out)
    // Use a key unlikely to exist in the real shell environment.
    if std::env::var("_SILM_TEST_BUILD_VAR").is_err() {
        assert_eq!(
            cmds[0].env.get("_SILM_TEST_BUILD_VAR").map(|s| s.as_str()),
            Some("from_dotenv"),
            "expected _SILM_TEST_BUILD_VAR=from_dotenv in captured env"
        );
    }
}

// ============================================================================
// Test 6: test_build_env_from_game_toml
// ============================================================================

#[test]
fn test_build_env_from_game_toml() {
    let (_dir, root, game_toml) = make_project();
    let (runner, commands) = MockRunner::new();

    // No .env file in this project (make_project doesn't create one)
    assert!(!root.join(".env").exists());

    build_all_platforms(
        &runner,
        &root,
        &game_toml,
        &["native".into()],
        false,
        None,
        true,
    )
    .unwrap();

    let cmds = commands.lock().unwrap();
    assert!(!cmds.is_empty());

    // [build.env] vars should be present (unless shadowed by shell)
    if std::env::var("_SILM_TEST_BUILD_VAR").is_err() {
        assert_eq!(
            cmds[0].env.get("_SILM_TEST_BUILD_VAR").map(|s| s.as_str()),
            Some("from_toml"),
        );
    }
    if std::env::var("SERVER_ADDRESS").is_err() {
        assert_eq!(
            cmds[0].env.get("SERVER_ADDRESS").map(|s| s.as_str()),
            Some("127.0.0.1:7777"),
        );
    }
}

// ============================================================================
// Test 7: test_env_file_overrides_dotenv
// ============================================================================

#[test]
fn test_env_file_overrides_dotenv() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("priority-project");
    std::fs::create_dir_all(root.join("client")).unwrap();

    // .env with lower priority value
    std::fs::write(root.join(".env"), "_SILM_TEST_PRIORITY=from_dotenv\n").unwrap();

    // explicit --env-file with higher priority value
    let env_file = dir.path().join("override.env");
    std::fs::write(&env_file, "_SILM_TEST_PRIORITY=from_env_file\n").unwrap();

    std::fs::write(
        root.join("client").join("index.html"),
        "<html></html>",
    )
    .unwrap();

    let game_toml = r#"[project]
name = "priority-test"
"#;
    std::fs::write(root.join("game.toml"), game_toml).unwrap();

    let (runner, commands) = MockRunner::new();

    build_all_platforms(
        &runner,
        &root,
        game_toml,
        &["native".into()],
        false,
        Some(env_file.as_path()),
        true,
    )
    .unwrap();

    let cmds = commands.lock().unwrap();
    assert!(!cmds.is_empty());

    // --env-file should beat .env
    if std::env::var("_SILM_TEST_PRIORITY").is_err() {
        assert_eq!(
            cmds[0].env.get("_SILM_TEST_PRIORITY").map(|s| s.as_str()),
            Some("from_env_file"),
            "--env-file should override .env"
        );
    }
}

// ============================================================================
// Test 8: test_missing_build_section_no_platform_errors
// ============================================================================

#[test]
fn test_missing_build_section_no_platform_errors() {
    let content = r#"[project]
name = "no-build"
"#;
    let result = parse_build_section(content);
    assert!(
        result.is_none(),
        "parse_build_section should return None when [build] is absent"
    );
}

// ============================================================================
// Test 9: test_unknown_platform_errors
// ============================================================================

#[test]
fn test_unknown_platform_errors() {
    let result = platform_from_str("darwin");
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("Unknown platform") && msg.contains("darwin"),
        "expected error mentioning 'Unknown platform' and 'darwin', got: {msg}"
    );
}

// ============================================================================
// Test 10: test_package_assembles_dist
// ============================================================================

#[test]
fn test_package_assembles_dist() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("pkg-project");
    let release = root.join("target").join("release");
    std::fs::create_dir_all(&release).unwrap();

    // Create fake binaries
    std::fs::write(release.join("server"), "fake server binary").unwrap();
    std::fs::write(release.join("client"), "fake client binary").unwrap();

    // Create assets
    let assets = root.join("assets");
    std::fs::create_dir_all(&assets).unwrap();
    std::fs::write(assets.join("texture.png"), "fake png data").unwrap();

    let dist_dir = assemble_native_dist(&root, "native", None, true, true, false).unwrap();

    assert!(dist_dir.is_dir(), "dist directory should exist");
    assert!(dist_dir.join("server").is_file(), "server binary should be in dist");
    assert!(dist_dir.join("client").is_file(), "client binary should be in dist");
    assert!(
        dist_dir.join("assets").join("texture.png").is_file(),
        "assets should be copied into dist"
    );
}

// ============================================================================
// Test 11: test_package_server_has_dockerfile
// ============================================================================

#[test]
fn test_package_server_has_dockerfile() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("server-pkg");
    let release = root.join("target").join("release");
    std::fs::create_dir_all(&release).unwrap();

    std::fs::write(release.join("server"), "fake server").unwrap();

    let env = vec![("GAME_PORT".into(), "7777".into())];
    let dist_dir = assemble_server_dist(&root, &env, false).unwrap();

    assert!(dist_dir.join("Dockerfile").is_file(), "Dockerfile should exist");
    assert!(dist_dir.join("server").is_file(), "server binary should be in dist");

    let dockerfile = std::fs::read_to_string(dist_dir.join("Dockerfile")).unwrap();
    assert!(dockerfile.contains("FROM debian:bookworm-slim"));
    assert!(dockerfile.contains("ENTRYPOINT"));
    assert!(dockerfile.contains("ENV GAME_PORT=7777"));
}

// ============================================================================
// Test 12: test_package_creates_zip
// ============================================================================

#[test]
fn test_package_creates_zip() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("zip-source");
    std::fs::create_dir_all(source.join("subdir")).unwrap();
    std::fs::write(source.join("binary"), "fake binary content").unwrap();
    std::fs::write(source.join("subdir").join("data.txt"), "some data").unwrap();

    let zip_path = dir.path().join("game-v0.1.0-native.zip");
    create_zip(&source, &zip_path).unwrap();

    assert!(zip_path.exists(), "zip file should be created");
    let metadata = std::fs::metadata(&zip_path).unwrap();
    assert!(metadata.len() > 0, "zip file should not be empty");

    // Verify the zip is valid by opening it
    let file = std::fs::File::open(&zip_path).unwrap();
    let archive = zip::ZipArchive::new(file).unwrap();
    let names: Vec<String> = (0..archive.len())
        .map(|i| archive.name_for_index(i).unwrap().to_string())
        .collect();
    assert!(
        names.iter().any(|n| n == "binary"),
        "zip should contain 'binary', got: {:?}",
        names
    );
    assert!(
        names.iter().any(|n| n == "subdir/data.txt"),
        "zip should contain 'subdir/data.txt', got: {:?}",
        names
    );
}

// ============================================================================
// Test 13: test_assets_absent_no_error
// ============================================================================

#[test]
fn test_assets_absent_no_error() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("no-assets");
    let release = root.join("target").join("release");
    std::fs::create_dir_all(&release).unwrap();

    std::fs::write(release.join("server"), "fake server").unwrap();
    std::fs::write(release.join("client"), "fake client").unwrap();

    // No assets/ directory at all
    let dist_dir = assemble_native_dist(&root, "native", None, true, true, false).unwrap();

    assert!(dist_dir.join("server").is_file());
    assert!(dist_dir.join("client").is_file());
    assert!(
        !dist_dir.join("assets").exists(),
        "assets dir should not exist when source has no assets"
    );
}
