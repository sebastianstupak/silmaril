//! End-to-end CLI tests for the `silm` binary.
//!
//! These tests exercise the **real** CLI binary (`silm new`, `silm add`,
//! `silm build`, `silm package`) against the filesystem and `cargo`.
//! Every test is marked `#[ignore]` because it involves real compilation.
//!
//! Run with:
//! ```bash
//! cargo test --package silm --test e2e_cli_tests -- --ignored --test-threads=1
//! ```

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

// ============================================================================
// Helpers
// ============================================================================

fn silm_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_silm"))
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("cannot find repo root")
}

/// Create a test project adjacent to the repo root using `silm new --local`.
/// Returns `(project_dir, CleanupGuard)`. The guard removes the directory on
/// drop so cleanup happens even when a test panics.
fn create_test_project(name: &str) -> (PathBuf, CleanupGuard) {
    let parent = repo_root().parent().unwrap().to_path_buf();
    let project_dir = parent.join(name);

    // Remove leftovers from a previous (interrupted) run.
    if project_dir.exists() {
        fs::remove_dir_all(&project_dir).ok();
    }

    let output = Command::new(silm_bin())
        .args(["new", name, "--local"])
        .current_dir(&parent)
        .output()
        .expect("failed to run silm new");

    assert!(
        output.status.success(),
        "silm new failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    assert!(project_dir.exists(), "project dir not created at {:?}", project_dir);

    (project_dir.clone(), CleanupGuard(project_dir))
}

struct CleanupGuard(PathBuf);

impl Drop for CleanupGuard {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).ok();
    }
}

/// Run `silm <args>` inside `project_dir` and return raw `Output`.
fn run_silm(project_dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new(silm_bin())
        .args(args)
        .current_dir(project_dir)
        .output()
        .expect("failed to run silm")
}

/// Run `silm <args>` and assert success. Returns stdout as `String`.
fn run_silm_ok(project_dir: &Path, args: &[&str]) -> String {
    let output = run_silm(project_dir, args);
    assert!(
        output.status.success(),
        "silm {} failed:\nstdout: {}\nstderr: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    String::from_utf8_lossy(&output.stdout).to_string()
}

/// Run `cargo <args>` inside `project_dir` and return raw `Output`.
fn run_cargo(project_dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new("cargo")
        .args(args)
        .current_dir(project_dir)
        .output()
        .expect("failed to run cargo")
}

/// Check whether a binary exists at `path` (trying both bare and `.exe`).
fn binary_exists(path: &Path) -> bool {
    path.exists() || path.with_extension("exe").exists()
}

// ============================================================================
// Build / Package tests (replacing test-silm-build.sh)
// ============================================================================

#[test]
#[ignore]
fn e2e_build_native_debug() {
    let (dir, _guard) = create_test_project("e2e-build-debug");
    run_silm_ok(&dir, &["build", "--platform", "native"]);

    let server = binary_exists(&dir.join("target/debug/server"));
    let client = binary_exists(&dir.join("target/debug/client"));
    assert!(server, "server binary not found in target/debug/");
    assert!(client, "client binary not found in target/debug/");
}

#[test]
#[ignore]
fn e2e_build_native_release() {
    let (dir, _guard) = create_test_project("e2e-build-release");
    run_silm_ok(&dir, &["build", "--platform", "native", "--release"]);

    let server = binary_exists(&dir.join("target/release/server"));
    let client = binary_exists(&dir.join("target/release/client"));
    assert!(server, "server binary not found in target/release/");
    assert!(client, "client binary not found in target/release/");
}

#[test]
#[ignore]
fn e2e_package_native() {
    let (dir, _guard) = create_test_project("e2e-pkg-native");
    run_silm_ok(&dir, &["package", "--platform", "native"]);

    assert!(dir.join("dist/native").is_dir(), "dist/native/ not created");

    // The zip is placed at {project_root}/{name}-v{version}-native.zip
    let has_zip = fs::read_dir(&dir).unwrap().any(|e| {
        e.ok()
            .map(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                name.contains("-native.zip")
            })
            .unwrap_or(false)
    });
    assert!(has_zip, "native zip not found in project root");
}

#[test]
#[ignore]
fn e2e_package_server_has_dockerfile() {
    let (dir, _guard) = create_test_project("e2e-pkg-server");
    run_silm_ok(&dir, &["package", "--platform", "server"]);

    assert!(dir.join("dist/server").is_dir(), "dist/server/ not created");
    assert!(
        dir.join("dist/server/Dockerfile").exists(),
        "Dockerfile not found in dist/server/"
    );

    let dockerfile = fs::read_to_string(dir.join("dist/server/Dockerfile")).unwrap();
    assert!(dockerfile.contains("ENTRYPOINT"), "Dockerfile missing ENTRYPOINT");
}

#[test]
#[ignore]
fn e2e_build_no_platform_uses_game_toml() {
    let (dir, _guard) = create_test_project("e2e-build-default");

    // `silm build` without --platform should read [build].platforms from game.toml.
    // The template sets platforms = ["native", "wasm"]; wasm will fail without trunk,
    // but the command should NOT complain about missing platforms.
    let output = run_silm(&dir, &["build"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("no platforms specified"),
        "should have read platforms from game.toml"
    );
}

// ============================================================================
// Add Component / System tests (replacing test-silm-add.sh)
// ============================================================================

#[test]
#[ignore]
fn e2e_add_component_shared() {
    let (dir, _guard) = create_test_project("e2e-add-comp");

    run_silm_ok(
        &dir,
        &[
            "add", "component", "Health", "--shared", "--domain", "health", "--fields",
            "current:f32,max:f32",
        ],
    );

    // Verify file contents
    let mod_file = dir.join("shared/src/health/mod.rs");
    assert!(mod_file.exists(), "health/mod.rs not created");
    let content = fs::read_to_string(&mod_file).unwrap();
    assert!(content.contains("pub struct Health"), "Health struct missing");
    assert!(content.contains("pub current: f32"), "current field missing");
    assert!(content.contains("pub max: f32"), "max field missing");

    // Verify lib.rs wiring
    let lib = fs::read_to_string(dir.join("shared/src/lib.rs")).unwrap();
    assert!(lib.contains("pub mod health;"), "lib.rs not wired");

    // Verify it compiles
    let output = run_cargo(&dir, &["check", "--package", "e2e-add-comp-shared"]);
    assert!(
        output.status.success(),
        "shared crate failed to compile after adding component:\n{}",
        String::from_utf8_lossy(&output.stderr),
    );
}

#[test]
#[ignore]
fn e2e_add_system_shared() {
    let (dir, _guard) = create_test_project("e2e-add-sys");

    // Add component first (system needs something to query)
    run_silm_ok(
        &dir,
        &[
            "add", "component", "Health", "--shared", "--domain", "health", "--fields",
            "current:f32,max:f32",
        ],
    );

    // Add system
    run_silm_ok(
        &dir,
        &[
            "add", "system", "health_regen", "--shared", "--domain", "health", "--query",
            "mut:Health",
        ],
    );

    let content = fs::read_to_string(dir.join("shared/src/health/mod.rs")).unwrap();
    assert!(
        content.contains("pub fn health_regen_system("),
        "system function missing"
    );

    // Verify compiles
    let output = run_cargo(&dir, &["check", "--package", "e2e-add-sys-shared"]);
    assert!(
        output.status.success(),
        "shared crate failed to compile after adding system:\n{}",
        String::from_utf8_lossy(&output.stderr),
    );
}

#[test]
#[ignore]
fn e2e_add_multiple_domains() {
    let (dir, _guard) = create_test_project("e2e-add-multi");

    // Health domain
    run_silm_ok(
        &dir,
        &[
            "add", "component", "Health", "--shared", "--domain", "health", "--fields",
            "current:f32,max:f32",
        ],
    );
    // Movement domain
    run_silm_ok(
        &dir,
        &[
            "add", "component", "Velocity", "--shared", "--domain", "movement", "--fields",
            "x:f32,y:f32,z:f32",
        ],
    );

    let lib = fs::read_to_string(dir.join("shared/src/lib.rs")).unwrap();
    assert!(lib.contains("pub mod health;"), "health not wired");
    assert!(lib.contains("pub mod movement;"), "movement not wired");

    let output = run_cargo(&dir, &["check", "--package", "e2e-add-multi-shared"]);
    assert!(
        output.status.success(),
        "shared crate failed to compile with multiple domains:\n{}",
        String::from_utf8_lossy(&output.stderr),
    );
}

#[test]
#[ignore]
fn e2e_add_generated_tests_pass() {
    let (dir, _guard) = create_test_project("e2e-add-tests");

    run_silm_ok(
        &dir,
        &[
            "add", "component", "Health", "--shared", "--domain", "health", "--fields",
            "current:f32,max:f32",
        ],
    );
    run_silm_ok(
        &dir,
        &[
            "add", "system", "health_regen", "--shared", "--domain", "health", "--query",
            "mut:Health",
        ],
    );

    // Run generated tests
    let output = run_cargo(&dir, &["test", "--package", "e2e-add-tests-shared"]);
    assert!(
        output.status.success(),
        "generated tests failed:\n{}",
        String::from_utf8_lossy(&output.stderr),
    );
}

#[test]
#[ignore]
fn e2e_add_duplicate_component_rejected() {
    let (dir, _guard) = create_test_project("e2e-add-dup");

    run_silm_ok(
        &dir,
        &[
            "add", "component", "Health", "--shared", "--domain", "health", "--fields",
            "current:f32",
        ],
    );

    // Second add should fail
    let output = run_silm(
        &dir,
        &[
            "add", "component", "Health", "--shared", "--domain", "health", "--fields", "hp:f32",
        ],
    );
    assert!(
        !output.status.success(),
        "duplicate component should have been rejected"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("already exists"),
        "error should mention 'already exists', got: {}",
        stderr,
    );
}

#[test]
#[ignore]
fn e2e_add_duplicate_system_rejected() {
    let (dir, _guard) = create_test_project("e2e-add-dupsys");

    run_silm_ok(
        &dir,
        &[
            "add", "component", "Health", "--shared", "--domain", "health", "--fields",
            "current:f32",
        ],
    );
    run_silm_ok(
        &dir,
        &[
            "add", "system", "health_regen", "--shared", "--domain", "health", "--query",
            "mut:Health",
        ],
    );

    // Second add of same system should fail
    let output = run_silm(
        &dir,
        &[
            "add", "system", "health_regen", "--shared", "--domain", "health", "--query",
            "mut:Health",
        ],
    );
    assert!(
        !output.status.success(),
        "duplicate system should have been rejected"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("already exists"),
        "error should mention 'already exists', got: {}",
        stderr,
    );
}

#[test]
#[ignore]
fn e2e_add_server_component() {
    let (dir, _guard) = create_test_project("e2e-add-srv");

    run_silm_ok(
        &dir,
        &[
            "add", "component", "Damage", "--server", "--domain", "combat", "--fields",
            "amount:f32,source_entity:u64",
        ],
    );

    let mod_file = dir.join("server/src/combat/mod.rs");
    assert!(mod_file.exists(), "server/combat/mod.rs not created");
    let content = fs::read_to_string(&mod_file).unwrap();
    assert!(content.contains("pub struct Damage"), "Damage struct missing");

    let main_rs = fs::read_to_string(dir.join("server/src/main.rs")).unwrap();
    assert!(
        main_rs.contains("pub mod combat;"),
        "pub mod combat; not in server/main.rs"
    );
}

#[test]
#[ignore]
fn e2e_add_two_components_same_domain() {
    let (dir, _guard) = create_test_project("e2e-add-same");

    run_silm_ok(
        &dir,
        &[
            "add", "component", "Health", "--shared", "--domain", "health", "--fields",
            "current:f32,max:f32",
        ],
    );
    run_silm_ok(
        &dir,
        &[
            "add", "component", "Stamina", "--shared", "--domain", "health", "--fields",
            "current:f32,max:f32,regen_rate:f32",
        ],
    );

    let content = fs::read_to_string(dir.join("shared/src/health/mod.rs")).unwrap();
    assert!(content.contains("pub struct Health"), "Health struct missing");
    assert!(content.contains("pub struct Stamina"), "Stamina struct missing");

    // lib.rs should wire the module exactly once
    let lib = fs::read_to_string(dir.join("shared/src/lib.rs")).unwrap();
    assert_eq!(
        lib.matches("pub mod health;").count(),
        1,
        "pub mod health; should appear exactly once"
    );

    let output = run_cargo(&dir, &["check", "--package", "e2e-add-same-shared"]);
    assert!(
        output.status.success(),
        "shared crate failed to compile with two components in same domain:\n{}",
        String::from_utf8_lossy(&output.stderr),
    );
}

#[test]
#[ignore]
fn e2e_build_missing_game_toml_errors() {
    // Run silm build from a directory that has no game.toml
    let tmp = tempfile::TempDir::new().unwrap();
    let output = run_silm(tmp.path(), &["build", "--platform", "native"]);
    assert!(!output.status.success(), "should fail without game.toml");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("game.toml"),
        "error should mention game.toml, got: {}",
        stderr,
    );
}

#[test]
#[ignore]
fn e2e_add_missing_game_toml_errors() {
    // Run silm add from a directory that has no game.toml
    let tmp = tempfile::TempDir::new().unwrap();
    let output = run_silm(
        tmp.path(),
        &[
            "add", "component", "Foo", "--shared", "--domain", "test", "--fields", "x:f32",
        ],
    );
    assert!(!output.status.success(), "should fail without game.toml");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.to_lowercase().contains("game.toml"),
        "error should mention game.toml, got: {}",
        stderr,
    );
}

#[test]
#[ignore]
fn e2e_multi_component_system_query() {
    let (dir, _guard) = create_test_project("e2e-add-mcq");

    // Add two components in the same domain
    run_silm_ok(
        &dir,
        &[
            "add", "component", "Health", "--shared", "--domain", "health", "--fields",
            "current:f32,max:f32",
        ],
    );
    run_silm_ok(
        &dir,
        &[
            "add", "component", "Stamina", "--shared", "--domain", "health", "--fields",
            "current:f32,max:f32",
        ],
    );

    // System that queries both
    run_silm_ok(
        &dir,
        &[
            "add", "system", "stamina_drain", "--shared", "--domain", "health", "--query",
            "mut:Health,mut:Stamina",
        ],
    );

    let content = fs::read_to_string(dir.join("shared/src/health/mod.rs")).unwrap();
    assert!(
        content.contains("pub fn stamina_drain_system("),
        "stamina_drain_system missing"
    );
    assert!(content.contains("&mut Health"), "&mut Health not in query");
    assert!(
        content.contains("&mut Stamina"),
        "&mut Stamina not in query"
    );

    // Verify compiles
    let output = run_cargo(&dir, &["check", "--package", "e2e-add-mcq-shared"]);
    assert!(
        output.status.success(),
        "shared crate failed to compile with multi-component system:\n{}",
        String::from_utf8_lossy(&output.stderr),
    );
}
