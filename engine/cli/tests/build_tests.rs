//! Comprehensive tests for the build command layers.

use silm::commands::build::env::{merge_env, parse_build_env, parse_build_section, parse_env_file};
use silm::commands::build::native::build_native;
use silm::commands::build::package::{generate_dockerfile, zip_filename};
use silm::commands::build::wasm::build_wasm;
use silm::commands::build::{
    build_all_platforms, check_tool, dist_dir_name, host_target_triple, parse_dev_section,
    parse_project_name, parse_project_version, platform_from_str, BuildKind, BuildRunner,
    BuildTool, KNOWN_PLATFORMS,
};

use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

// ============================================================================
// MockRunner for testing
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
    /// If set, the runner will return this error on the next call.
    fail_on_program: Option<String>,
}

impl MockRunner {
    fn new() -> (Self, Arc<Mutex<Vec<CapturedCommand>>>) {
        let commands = Arc::new(Mutex::new(Vec::new()));
        let runner = Self {
            commands: Arc::clone(&commands),
            fail_on_program: None,
        };
        (runner, commands)
    }

    fn new_failing(program: &str) -> (Self, Arc<Mutex<Vec<CapturedCommand>>>) {
        let commands = Arc::new(Mutex::new(Vec::new()));
        let runner = Self {
            commands: Arc::clone(&commands),
            fail_on_program: Some(program.to_string()),
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
        if let Some(ref fail_prog) = self.fail_on_program {
            if program == fail_prog {
                anyhow::bail!("{program} failed");
            }
        }
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
// parse_env_file
// ============================================================================

#[test]
fn test_parse_env_file_basic() {
    let content = "FOO=bar\nBAZ=qux";
    let result = parse_env_file(content);
    assert_eq!(
        result,
        vec![
            ("FOO".into(), "bar".into()),
            ("BAZ".into(), "qux".into()),
        ]
    );
}

#[test]
fn test_parse_env_file_comments_and_blanks() {
    let content = "# a comment\n\nKEY=value\n  \n# another comment\nKEY2=val2\n";
    let result = parse_env_file(content);
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].0, "KEY");
    assert_eq!(result[1].0, "KEY2");
}

#[test]
fn test_parse_env_file_blank_value() {
    let content = "EMPTY=";
    let result = parse_env_file(content);
    assert_eq!(result, vec![("EMPTY".into(), "".into())]);
}

#[test]
fn test_parse_env_file_duplicate_keys() {
    let content = "DUP=first\nDUP=second";
    let result = parse_env_file(content);
    // Both appear; merge_env will handle dedup via HashMap insertion order
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].1, "first");
    assert_eq!(result[1].1, "second");
}

#[test]
fn test_parse_env_file_no_equals_lines_skipped() {
    let content = "GOOD=yes\nno-equals-here\nALSO_GOOD=yep";
    let result = parse_env_file(content);
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].0, "GOOD");
    assert_eq!(result[1].0, "ALSO_GOOD");
}

#[test]
fn test_parse_env_file_value_with_equals() {
    let content = "URL=https://example.com?a=1&b=2";
    let result = parse_env_file(content);
    assert_eq!(result[0].1, "https://example.com?a=1&b=2");
}

#[test]
fn test_parse_env_file_value_not_trimmed() {
    let content = "SPACED=  hello  ";
    let result = parse_env_file(content);
    assert_eq!(result[0].1, "  hello  ");
}

// ============================================================================
// parse_build_env
// ============================================================================

#[test]
fn test_parse_build_env_basic() {
    let content = r#"
[build.env]
RUST_LOG = "debug"
SERVER_PORT = "7777"
"#;
    let result = parse_build_env(content);
    assert!(result.iter().any(|(k, v)| k == "RUST_LOG" && v == "debug"));
    assert!(result
        .iter()
        .any(|(k, v)| k == "SERVER_PORT" && v == "7777"));
}

#[test]
fn test_parse_build_env_ignores_other_sections() {
    let content = r#"
[package]
name = "my-game"

[build.env]
ONLY_THIS = "yes"

[other]
not_this = "no"
"#;
    let result = parse_build_env(content);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].0, "ONLY_THIS");
}

#[test]
fn test_parse_build_env_empty_when_absent() {
    let content = r#"
[package]
name = "my-game"
"#;
    let result = parse_build_env(content);
    assert!(result.is_empty());
}

#[test]
fn test_parse_build_env_skips_non_string_values() {
    let content = r#"
[build.env]
GOOD = "yes"
PORT = 8080
ENABLED = true
"#;
    let result = parse_build_env(content);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].0, "GOOD");
}

// ============================================================================
// parse_build_section
// ============================================================================

#[test]
fn test_parse_build_section_platforms_list() {
    let content = r#"
[build]
platforms = ["native", "linux-x86_64", "wasm"]
"#;
    let result = parse_build_section(content).unwrap();
    assert_eq!(result, vec!["native", "linux-x86_64", "wasm"]);
}

#[test]
fn test_parse_build_section_absent() {
    let content = r#"
[package]
name = "my-game"
"#;
    assert!(parse_build_section(content).is_none());
}

#[test]
fn test_parse_build_section_many_platforms() {
    let content = r#"
[build]
platforms = ["native", "server", "windows-x86_64", "linux-x86_64", "linux-arm64", "macos-x86_64", "macos-arm64", "wasm"]
"#;
    let result = parse_build_section(content).unwrap();
    assert_eq!(result.len(), 8);
}

#[test]
fn test_parse_build_section_no_platforms_key() {
    let content = r#"
[build]
optimization = "release"
"#;
    assert!(parse_build_section(content).is_none());
}

#[test]
fn test_parse_build_section_empty_array_returns_none() {
    let content = r#"
[build]
platforms = []
"#;
    assert!(parse_build_section(content).is_none());
}

// ============================================================================
// merge_env
// ============================================================================

#[test]
fn test_merge_env_priority_order() {
    let build_env = vec![("KEY".into(), "from_build".into())];
    let dotenv = vec![("KEY".into(), "from_dotenv".into())];
    let env_file = vec![("KEY".into(), "from_file".into())];

    let result = merge_env(&build_env, &dotenv, &env_file);
    assert_eq!(result.get("KEY").unwrap(), "from_file");
}

#[test]
fn test_merge_env_build_env_lowest() {
    let build_env = vec![("A".into(), "build".into())];
    let dotenv = vec![("A".into(), "dotenv".into())];
    let env_file: Vec<(String, String)> = vec![];

    let result = merge_env(&build_env, &dotenv, &env_file);
    assert_eq!(result.get("A").unwrap(), "dotenv");
}

#[test]
fn test_merge_env_disjoint_keys() {
    let build_env = vec![("A".into(), "1".into())];
    let dotenv = vec![("B".into(), "2".into())];
    let env_file = vec![("C".into(), "3".into())];

    let result = merge_env(&build_env, &dotenv, &env_file);
    assert_eq!(result.len(), 3);
    assert_eq!(result["A"], "1");
    assert_eq!(result["B"], "2");
    assert_eq!(result["C"], "3");
}

#[test]
fn test_merge_env_shell_wins() {
    // Use a unique key unlikely to exist in the real environment
    let key = "_SILM_BUILD_TEST_SHELL_WINS";
    unsafe {
        std::env::set_var(key, "from_shell");
    }

    let build_env = vec![(key.into(), "from_build".into())];
    let dotenv: Vec<(String, String)> = vec![];
    let env_file: Vec<(String, String)> = vec![];

    let result = merge_env(&build_env, &dotenv, &env_file);
    // Shell var exists, so the key should be filtered out
    assert!(!result.contains_key(key));

    // Clean up
    unsafe {
        std::env::remove_var(key);
    }
}

// ============================================================================
// platform_from_str
// ============================================================================

#[test]
fn test_platform_from_str_native() {
    let p = platform_from_str("native").unwrap();
    assert_eq!(p.name(), "native");
    assert_eq!(p.build_tool(), BuildTool::Cargo);
    assert_eq!(p.build_kind(), BuildKind::ServerAndClient);
    assert!(!p.is_experimental());
    // target triple should match host
    assert_eq!(p.target_triple(), host_target_triple());
}

#[test]
fn test_platform_from_str_server() {
    let p = platform_from_str("server").unwrap();
    assert_eq!(p.name(), "server");
    assert_eq!(p.build_kind(), BuildKind::ServerOnly);
    assert_eq!(p.build_tool(), BuildTool::Cargo);
}

#[test]
fn test_platform_from_str_windows_x86_64() {
    let p = platform_from_str("windows-x86_64").unwrap();
    assert_eq!(p.name(), "windows-x86_64");
    assert!(p.uses_exe_extension());
    assert!(!p.is_experimental());
    // Tool depends on host
    if std::env::consts::OS == "windows" {
        assert_eq!(p.build_tool(), BuildTool::Cargo);
        assert_eq!(p.target_triple(), "x86_64-pc-windows-msvc");
    } else {
        assert_eq!(p.build_tool(), BuildTool::Cross);
        assert_eq!(p.target_triple(), "x86_64-pc-windows-gnu");
    }
}

#[test]
fn test_platform_from_str_linux_x86_64() {
    let p = platform_from_str("linux-x86_64").unwrap();
    assert_eq!(p.target_triple(), "x86_64-unknown-linux-gnu");
    assert_eq!(p.build_tool(), BuildTool::Cross);
    assert!(!p.uses_exe_extension());
}

#[test]
fn test_platform_from_str_linux_arm64() {
    let p = platform_from_str("linux-arm64").unwrap();
    assert_eq!(p.target_triple(), "aarch64-unknown-linux-gnu");
    assert_eq!(p.build_tool(), BuildTool::Cross);
}

#[test]
fn test_platform_from_str_macos_x86_64() {
    let p = platform_from_str("macos-x86_64").unwrap();
    assert_eq!(p.target_triple(), "x86_64-apple-darwin");
    assert!(p.is_experimental());
}

#[test]
fn test_platform_from_str_macos_arm64() {
    let p = platform_from_str("macos-arm64").unwrap();
    assert_eq!(p.target_triple(), "aarch64-apple-darwin");
    assert!(p.is_experimental());
}

#[test]
fn test_platform_from_str_wasm() {
    let p = platform_from_str("wasm").unwrap();
    assert_eq!(p.target_triple(), "wasm32-unknown-unknown");
    assert_eq!(p.build_tool(), BuildTool::Trunk);
    assert_eq!(p.build_kind(), BuildKind::ClientOnly);
    assert!(!p.uses_exe_extension());
}

#[test]
fn test_platform_from_str_unknown() {
    let result = platform_from_str("playstation-5");
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("Unknown platform"));
    assert!(msg.contains("playstation-5"));
}

#[test]
fn test_all_known_platforms_resolve() {
    for name in KNOWN_PLATFORMS {
        let result = platform_from_str(name);
        assert!(result.is_ok(), "Failed to resolve platform: {name}");
    }
}

// ============================================================================
// dist_dir_name
// ============================================================================

#[test]
fn test_dist_dir_name_all_platforms() {
    for name in KNOWN_PLATFORMS {
        let p = platform_from_str(name).unwrap();
        assert_eq!(dist_dir_name(&p), *name);
    }
}

// ============================================================================
// host_target_triple
// ============================================================================

#[test]
fn test_host_target_triple_not_empty() {
    let triple = host_target_triple();
    assert!(!triple.is_empty());
    // Should contain arch and os info
    assert!(triple.contains('-'));
}

// ============================================================================
// zip_filename
// ============================================================================

#[test]
fn test_zip_filename_basic() {
    assert_eq!(
        zip_filename("my-game", "1.0.0", "linux-x86_64"),
        "my-game-v1.0.0-linux-x86_64.zip"
    );
}

#[test]
fn test_zip_filename_various() {
    assert_eq!(
        zip_filename("silmaril", "0.2.3", "wasm"),
        "silmaril-v0.2.3-wasm.zip"
    );
}

// ============================================================================
// generate_dockerfile
// ============================================================================

#[test]
fn test_generate_dockerfile_basic() {
    let env = vec![("RUST_LOG".into(), "info".into())];
    let df = generate_dockerfile(&env);
    assert!(df.contains("FROM debian:bookworm-slim"));
    assert!(df.contains("COPY server /usr/local/bin/server"));
    assert!(df.contains("EXPOSE 7777/udp"));
    assert!(df.contains("# Override at runtime: docker run -e KEY=value ..."));
    assert!(df.contains("ENV RUST_LOG=info"));
    assert!(df.contains("ENTRYPOINT [\"/usr/local/bin/server\"]"));
}

#[test]
fn test_generate_dockerfile_multiple_env() {
    let env = vec![
        ("A".into(), "1".into()),
        ("B".into(), "2".into()),
        ("C".into(), "3".into()),
    ];
    let df = generate_dockerfile(&env);
    assert!(df.contains("# Override at runtime: docker run -e KEY=value ..."));
    assert!(df.contains("ENV A=1"));
    assert!(df.contains("ENV B=2"));
    assert!(df.contains("ENV C=3"));
}

#[test]
fn test_generate_dockerfile_no_env() {
    let df = generate_dockerfile(&[]);
    assert!(df.contains("FROM debian:bookworm-slim"));
    assert!(df.contains("ENTRYPOINT"));
    assert!(!df.contains("ENV "));
    assert!(!df.contains("# Override at runtime"));
}

#[test]
fn test_generate_dockerfile_has_blank_lines() {
    let env = vec![("KEY".into(), "val".into())];
    let df = generate_dockerfile(&env);
    // Should have blank lines between sections
    assert!(df.contains("\n\n"));
}

// ============================================================================
// check_tool (error path)
// ============================================================================

#[test]
fn test_check_tool_nonexistent() {
    let result = check_tool("__silm_nonexistent_tool_xyz__");
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("not found"));
    assert!(msg.contains("cargo install"));
}

// ============================================================================
// build_native tests (Task 4)
// ============================================================================

#[test]
fn test_native_cargo_server_and_client() {
    let (runner, commands) = MockRunner::new();
    let env = HashMap::new();
    let root = PathBuf::from("/project");

    build_native(
        &runner,
        &root,
        &env,
        "my-game-server",
        "my-game-client",
        BuildTool::Cargo,
        None,
        BuildKind::ServerAndClient,
        false,
    )
    .unwrap();

    let cmds = commands.lock().unwrap();
    assert_eq!(cmds.len(), 2);

    // Server build
    assert_eq!(cmds[0].program, "cargo");
    assert!(cmds[0].args.contains(&"--package".to_string()));
    assert!(cmds[0].args.contains(&"my-game-server".to_string()));
    assert!(cmds[0].args.contains(&"--bin".to_string()));
    assert!(cmds[0].args.contains(&"server".to_string()));
    assert!(!cmds[0].args.contains(&"--release".to_string()));

    // Client build
    assert_eq!(cmds[1].program, "cargo");
    assert!(cmds[1].args.contains(&"my-game-client".to_string()));
    assert!(cmds[1].args.contains(&"client".to_string()));
}

#[test]
fn test_native_server_only() {
    let (runner, commands) = MockRunner::new();
    let env = HashMap::new();
    let root = PathBuf::from("/project");

    build_native(
        &runner,
        &root,
        &env,
        "my-server",
        "my-client",
        BuildTool::Cargo,
        None,
        BuildKind::ServerOnly,
        false,
    )
    .unwrap();

    let cmds = commands.lock().unwrap();
    assert_eq!(cmds.len(), 1);
    assert!(cmds[0].args.contains(&"my-server".to_string()));
    assert!(cmds[0].args.contains(&"server".to_string()));
}

#[test]
fn test_native_client_only() {
    let (runner, commands) = MockRunner::new();
    let env = HashMap::new();
    let root = PathBuf::from("/project");

    build_native(
        &runner,
        &root,
        &env,
        "my-server",
        "my-client",
        BuildTool::Cargo,
        None,
        BuildKind::ClientOnly,
        false,
    )
    .unwrap();

    let cmds = commands.lock().unwrap();
    assert_eq!(cmds.len(), 1);
    assert!(cmds[0].args.contains(&"my-client".to_string()));
    assert!(cmds[0].args.contains(&"client".to_string()));
}

#[test]
fn test_native_release_flag() {
    let (runner, commands) = MockRunner::new();
    let env = HashMap::new();
    let root = PathBuf::from("/project");

    build_native(
        &runner,
        &root,
        &env,
        "srv",
        "cli",
        BuildTool::Cargo,
        None,
        BuildKind::ServerOnly,
        true,
    )
    .unwrap();

    let cmds = commands.lock().unwrap();
    assert!(cmds[0].args.contains(&"--release".to_string()));
}

#[test]
fn test_native_cross_with_target_triple() {
    let (runner, commands) = MockRunner::new();
    let env = HashMap::new();
    let root = PathBuf::from("/project");

    build_native(
        &runner,
        &root,
        &env,
        "srv",
        "cli",
        BuildTool::Cross,
        Some("x86_64-unknown-linux-gnu"),
        BuildKind::ServerAndClient,
        true,
    )
    .unwrap();

    let cmds = commands.lock().unwrap();
    assert_eq!(cmds.len(), 2);
    for cmd in cmds.iter() {
        assert_eq!(cmd.program, "cross");
        assert!(cmd.args.contains(&"--target".to_string()));
        assert!(cmd.args.contains(&"x86_64-unknown-linux-gnu".to_string()));
        assert!(cmd.args.contains(&"--release".to_string()));
    }
}

#[test]
fn test_native_trunk_is_error() {
    let (runner, _) = MockRunner::new();
    let env = HashMap::new();
    let root = PathBuf::from("/project");

    let result = build_native(
        &runner,
        &root,
        &env,
        "srv",
        "cli",
        BuildTool::Trunk,
        None,
        BuildKind::ClientOnly,
        false,
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Trunk"));
}

// ============================================================================
// build_wasm tests (Task 5)
// ============================================================================

#[test]
fn test_wasm_debug_mode() {
    let (runner, commands) = MockRunner::new();
    let env = HashMap::new();
    let root = PathBuf::from("/project");

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

#[test]
fn test_wasm_release_mode() {
    let (runner, commands) = MockRunner::new();
    let env = HashMap::new();
    let root = PathBuf::from("/project");

    build_wasm(&runner, &root, &env, true).unwrap();

    let cmds = commands.lock().unwrap();
    assert_eq!(cmds.len(), 1);
    assert!(cmds[0].args.contains(&"--release".to_string()));
}

// ============================================================================
// game.toml parsing helpers (Task 6)
// ============================================================================

#[test]
fn test_parse_dev_section_present() {
    let content = r#"
[project]
name = "my-game"

[dev]
server_package = "custom-server"
client_package = "custom-client"
"#;
    let (srv, cli) = parse_dev_section(content, "my-game");
    assert_eq!(srv, "custom-server");
    assert_eq!(cli, "custom-client");
}

#[test]
fn test_parse_dev_section_fallback() {
    let content = r#"
[project]
name = "my-game"
"#;
    let (srv, cli) = parse_dev_section(content, "my-game");
    assert_eq!(srv, "my-game-server");
    assert_eq!(cli, "my-game-client");
}

#[test]
fn test_parse_dev_section_partial_fallback() {
    let content = r#"
[dev]
server_package = "custom-server"
"#;
    let (srv, cli) = parse_dev_section(content, "cool-game");
    assert_eq!(srv, "custom-server");
    assert_eq!(cli, "cool-game-client");
}

#[test]
fn test_parse_project_name_present() {
    let content = r#"
[project]
name = "silmaril"
"#;
    assert_eq!(parse_project_name(content), Some("silmaril".into()));
}

#[test]
fn test_parse_project_name_absent() {
    let content = r#"
[build]
platforms = ["native"]
"#;
    assert_eq!(parse_project_name(content), None);
}

#[test]
fn test_parse_project_version_present() {
    let content = r#"
[project]
version = "1.2.3"
"#;
    assert_eq!(parse_project_version(content), "1.2.3");
}

#[test]
fn test_parse_project_version_missing() {
    let content = r#"
[project]
name = "my-game"
"#;
    assert_eq!(parse_project_version(content), "0.0.0");
}

#[test]
fn test_parse_project_version_invalid_toml() {
    assert_eq!(parse_project_version("not valid toml {{{{"), "0.0.0");
}

// ============================================================================
// build_all_platforms orchestration (Task 6)
// ============================================================================

#[test]
fn test_build_all_platforms_native() {
    let (runner, commands) = MockRunner::new();
    let root = PathBuf::from("/project");
    let game_toml = r#"
[project]
name = "my-game"

[dev]
server_package = "my-server"
client_package = "my-client"
"#;

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
    // native = ServerAndClient = 2 commands (server + client)
    assert_eq!(cmds.len(), 2);
    assert_eq!(cmds[0].program, "cargo");
    assert!(cmds[0].args.contains(&"my-server".to_string()));
    assert!(cmds[1].args.contains(&"my-client".to_string()));
}

#[test]
fn test_build_all_platforms_wasm() {
    let (runner, commands) = MockRunner::new();
    let root = PathBuf::from("/project");
    let game_toml = r#"
[project]
name = "my-game"
"#;

    build_all_platforms(
        &runner,
        &root,
        game_toml,
        &["wasm".into()],
        true,
        None,
        true,
    )
    .unwrap();

    let cmds = commands.lock().unwrap();
    assert_eq!(cmds.len(), 1);
    assert_eq!(cmds[0].program, "trunk");
    assert!(cmds[0].args.contains(&"--release".to_string()));
}

#[test]
fn test_build_all_platforms_unknown_platform_errors() {
    let (runner, _) = MockRunner::new();
    let root = PathBuf::from("/project");
    let game_toml = r#"
[project]
name = "my-game"
"#;

    let result = build_all_platforms(
        &runner,
        &root,
        game_toml,
        &["playstation-5".into()],
        false,
        None,
        true,
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Unknown platform"));
}

#[test]
fn test_build_all_platforms_experimental_failure_nonfatal() {
    // Use a runner that fails on "cross" (used by macos platforms)
    let (runner, _commands) = MockRunner::new_failing("cross");
    let root = PathBuf::from("/project");
    let game_toml = r#"
[project]
name = "my-game"
"#;

    // macos-arm64 is experimental, so failure should be non-fatal
    let result = build_all_platforms(
        &runner,
        &root,
        game_toml,
        &["macos-arm64".into()],
        false,
        None,
        true,
    );
    assert!(result.is_ok());
}

#[test]
fn test_build_all_platforms_non_experimental_failure_is_fatal() {
    let (runner, _commands) = MockRunner::new_failing("cross");
    let root = PathBuf::from("/project");
    let game_toml = r#"
[project]
name = "my-game"
"#;

    // linux-x86_64 uses cross and is not experimental
    let result = build_all_platforms(
        &runner,
        &root,
        game_toml,
        &["linux-x86_64".into()],
        false,
        None,
        true,
    );
    assert!(result.is_err());
}

#[test]
fn test_build_all_platforms_server_only() {
    let (runner, commands) = MockRunner::new();
    let root = PathBuf::from("/project");
    let game_toml = r#"
[project]
name = "my-game"
"#;

    build_all_platforms(
        &runner,
        &root,
        game_toml,
        &["server".into()],
        false,
        None,
        true,
    )
    .unwrap();

    let cmds = commands.lock().unwrap();
    assert_eq!(cmds.len(), 1);
    assert!(cmds[0].args.contains(&"server".to_string()));
}

#[test]
fn test_build_all_platforms_multiple() {
    let (runner, commands) = MockRunner::new();
    let root = PathBuf::from("/project");
    let game_toml = r#"
[project]
name = "my-game"
"#;

    build_all_platforms(
        &runner,
        &root,
        game_toml,
        &["native".into(), "wasm".into()],
        false,
        None,
        true,
    )
    .unwrap();

    let cmds = commands.lock().unwrap();
    // native = 2 (server+client) + wasm = 1 = 3 total
    assert_eq!(cmds.len(), 3);
}

// ============================================================================
// Pre-flight checks (Issues 1-3)
// ============================================================================

#[test]
fn test_preflight_wasm_missing_index_html() {
    // Use a temp dir that does NOT contain client/index.html
    let dir = tempfile::tempdir().unwrap();
    let (runner, _) = MockRunner::new();
    let game_toml = r#"
[project]
name = "my-game"
"#;

    let result = build_all_platforms(
        &runner,
        dir.path(),
        game_toml,
        &["wasm".into()],
        false,
        None,
        false, // DO run preflight
    );
    // trunk is not installed, so we may get a tool error first;
    // but if trunk happens to be installed, we'd get the index.html error.
    // Either way, the build should fail.
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    // Should mention either trunk not found or index.html not found
    assert!(
        msg.contains("trunk") || msg.contains("index.html"),
        "Unexpected error: {msg}"
    );
}

#[test]
fn test_preflight_cross_tool_not_found() {
    // linux-x86_64 uses cross. If cross is not installed, preflight should fail.
    let dir = tempfile::tempdir().unwrap();
    let (runner, _) = MockRunner::new();
    let game_toml = r#"
[project]
name = "my-game"
"#;

    let result = build_all_platforms(
        &runner,
        dir.path(),
        game_toml,
        &["linux-x86_64".into()],
        false,
        None,
        false, // DO run preflight
    );
    // cross is almost certainly not installed in CI/dev
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("cross") || msg.contains("Docker"),
        "Unexpected error: {msg}"
    );
}

// ============================================================================
// Issue 4: missing project name should error
// ============================================================================

#[test]
fn test_build_all_platforms_missing_project_name_errors() {
    let (runner, _) = MockRunner::new();
    let root = PathBuf::from("/project");
    let game_toml = r#"
[build]
platforms = ["native"]
"#;

    let result = build_all_platforms(
        &runner,
        &root,
        game_toml,
        &["native".into()],
        false,
        None,
        true,
    );
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("missing [project] name"), "Unexpected error: {msg}");
}

// ============================================================================
// Issue 5: no platforms should error (tested via handle_build_command indirectly,
// but we can test parse_build_section returning None + build_all_platforms requiring name)
// ============================================================================

#[test]
fn test_build_all_platforms_empty_platform_list_succeeds() {
    // build_all_platforms with empty list is a no-op (the error is in handle_build_command)
    let (runner, commands) = MockRunner::new();
    let root = PathBuf::from("/project");
    let game_toml = r#"
[project]
name = "my-game"
"#;

    let result = build_all_platforms(
        &runner,
        &root,
        game_toml,
        &[],
        false,
        None,
        true,
    );
    assert!(result.is_ok());
    let cmds = commands.lock().unwrap();
    assert_eq!(cmds.len(), 0);
}
