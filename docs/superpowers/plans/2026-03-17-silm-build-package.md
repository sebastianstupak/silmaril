# silm build + silm package Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `silm build` and `silm package` commands that build game projects for multiple platforms and produce distributable zip archives.

**Architecture:** Thin wrapper around `cargo` (native), `cross` (cross-platform), and `trunk` (WASM). A `BuildRunner` trait abstracts subprocess execution so integration tests can capture commands without running them. `silm package` calls the build logic with `--release`, then assembles `dist/<platform>/` directories and creates versioned zips.

**Tech Stack:** Rust, clap (CLI), zip crate v2 (archives), anyhow (errors), tracing (logging)

**Spec:** `docs/superpowers/specs/2026-03-17-silm-build-package-design.md`

**Reviewer notes (must follow):**
- Use the `toml` crate (already a dependency) with `toml::Value` to parse `game.toml` sections instead of hand-rolled line-by-line parsing. Hand-rolled parsers break on multi-line arrays, trailing commas, inline comments, etc.
- `zip_filename` lives only in `package.rs`. Tests import from `silm::commands::build::package::zip_filename`.
- `host_target_triple()` uses `std::env::consts::{ARCH, OS}` at runtime — NOT `env!("TARGET")`.
- Tests using `std::env::set_var` / `remove_var` must use unique prefixed names (`_SILM_TEST_*`) and run with `-- --test-threads=1` or use the `serial_test` crate.
- Env var names in integration tests must be unique: use `_SILM_TEST_*` prefix to avoid collisions.
- `package.rs` must include a `// TODO(CLI.7): cargo-packager integration for AppImage/DMG/NSIS` comment at the zip-creation step.
- `walkdir` is already a dependency of the CLI crate — no `Cargo.toml` change needed for it.
- `build_platform` is private. Integration tests call `build_all_platforms` (public) for orchestration testing, or call `native::build_native` / `wasm::build_wasm` directly for lower-level tests.

---

## File Structure

### New files

| File | Responsibility |
|------|---------------|
| `engine/cli/src/commands/build/mod.rs` | `BuildCommand` + `PackageCommand` clap enums, `handle_build_command`, `handle_package_command`, `Platform` enum + mapping, `BuildRunner` trait, `RealRunner`, tool detection, game.toml `[build]` parsing |
| `engine/cli/src/commands/build/env.rs` | `.env` file parsing, `game.toml [build.env]` parsing, env layer merge |
| `engine/cli/src/commands/build/native.rs` | `build_native()` — cargo/cross invocation for server + client, Windows host detection |
| `engine/cli/src/commands/build/wasm.rs` | `build_wasm()` — trunk invocation for client |
| `engine/cli/src/commands/build/package.rs` | `assemble_dist()`, `create_zip()`, `generate_dockerfile()`, `copy_assets()`, zip filename construction |
| `engine/cli/tests/build_tests.rs` | Unit tests for all pure logic (env, platform, paths, Dockerfile, zip names) |
| `engine/cli/tests/build_integration_tests.rs` | Integration tests with `MockRunner` capturing commands |
| `scripts/e2e-tests/test-silm-build.sh` | E2E tests with real tools on a real `silm new` project |

### Modified files

| File | Change |
|------|--------|
| `engine/cli/src/commands/mod.rs` | Add `pub mod build;` |
| `engine/cli/src/main.rs` | Add `Commands::Build` and `Commands::Package` variants + dispatch |
| `engine/cli/src/templates/basic.rs` | Add `[build]` section to game.toml, add `client/index.html`, update `.gitignore` |
| `engine/cli/Cargo.toml` | Add `zip = "2"` dependency |

---

## Chunk 1: Foundation — env parsing, platform types, runner trait

### Task 1: Environment variable parsing and merge (`env.rs`)

**Files:**
- Create: `engine/cli/src/commands/build/env.rs`
- Create: `engine/cli/src/commands/build/mod.rs` (minimal — just `pub mod env;`)
- Modify: `engine/cli/src/commands/mod.rs` — add `pub mod build;`
- Test: `engine/cli/tests/build_tests.rs`

**Context:** This task implements the env layer merge described in the spec. The precedence is: shell env > `--env-file` > `.env` > `game.toml [build.env]`. The merge algorithm builds a HashMap from lowest to highest priority, then filters out keys already in the shell environment. All functions are pure (no subprocess calls), so they can be fully unit-tested.

- [ ] **Step 1: Create minimal mod.rs and wire into commands**

Create `engine/cli/src/commands/build/mod.rs`:
```rust
pub mod env;
```

Add to `engine/cli/src/commands/mod.rs` (line 3, after `pub mod module;`):
```rust
pub mod build;
```

- [ ] **Step 2: Write failing tests for .env parsing**

Create `engine/cli/tests/build_tests.rs`:
```rust
use silm::commands::build::env::{parse_env_file, merge_env, parse_build_section};
use std::collections::HashMap;

#[test]
fn test_parse_env_file_basic() {
    let content = "SERVER_ADDRESS=ws://localhost:7777\nSERVER_PORT=7777\n";
    let vars = parse_env_file(content);
    assert_eq!(vars.len(), 2);
    assert_eq!(vars[0], ("SERVER_ADDRESS".to_string(), "ws://localhost:7777".to_string()));
    assert_eq!(vars[1], ("SERVER_PORT".to_string(), "7777".to_string()));
}

#[test]
fn test_parse_env_file_comments_and_blanks() {
    let content = "# comment\n\nKEY=value\n  # indented comment\n";
    let vars = parse_env_file(content);
    assert_eq!(vars.len(), 1);
    assert_eq!(vars[0].0, "KEY");
}

#[test]
fn test_parse_env_file_blank_value() {
    let content = "EMPTY=\n";
    let vars = parse_env_file(content);
    assert_eq!(vars.len(), 1);
    assert_eq!(vars[0], ("EMPTY".to_string(), String::new()));
}

#[test]
fn test_parse_env_file_duplicate_last_wins() {
    let content = "KEY=first\nKEY=second\n";
    let vars = parse_env_file(content);
    // Both are returned; merge_env handles dedup
    assert_eq!(vars.len(), 2);
    assert_eq!(vars[1].1, "second");
}

#[test]
fn test_parse_env_file_no_equals_skipped() {
    let content = "INVALID_LINE\nKEY=value\n";
    let vars = parse_env_file(content);
    assert_eq!(vars.len(), 1);
    assert_eq!(vars[0].0, "KEY");
}
```

Run: `cargo test --package silm --test build_tests`
Expected: FAIL — `parse_env_file` not found

- [ ] **Step 3: Implement parse_env_file**

Create `engine/cli/src/commands/build/env.rs`:
```rust
use std::collections::HashMap;

/// Parse a .env file into (key, value) pairs.
/// Standard KEY=VALUE per line, # for comments, blank lines skipped.
/// Duplicate keys: last definition wins (handled by inserting into HashMap later).
/// Note: values are NOT trimmed (consistent with standard .env semantics).
pub fn parse_env_file(content: &str) -> Vec<(String, String)> {
    let mut result = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some(eq_pos) = trimmed.find('=') {
            let key = trimmed[..eq_pos].trim().to_string();
            let value = trimmed[eq_pos + 1..].to_string();
            if !key.is_empty() {
                result.push((key, value));
            }
        }
    }
    result
}

/// Parse game.toml [build.env] section into (key, value) pairs.
/// Uses the `toml` crate for robust TOML parsing.
pub fn parse_build_env(game_toml_content: &str) -> Vec<(String, String)> {
    let parsed: toml::Value = match game_toml_content.parse() {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let mut result = Vec::new();
    if let Some(env_table) = parsed.get("build").and_then(|b| b.get("env")).and_then(|e| e.as_table()) {
        for (key, value) in env_table {
            if let Some(s) = value.as_str() {
                result.push((key.clone(), s.to_string()));
            }
        }
    }
    result
}

/// Parse game.toml [build] section for platforms list.
/// Returns None if [build] section or platforms key is absent.
/// Uses the `toml` crate for robust TOML parsing (handles multi-line arrays, trailing commas, etc.).
pub fn parse_build_section(game_toml_content: &str) -> Option<Vec<String>> {
    let parsed: toml::Value = game_toml_content.parse().ok()?;
    let platforms = parsed.get("build")?.get("platforms")?.as_array()?;
    let result: Vec<String> = platforms
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect();
    if result.is_empty() { None } else { Some(result) }
}

/// Merge env layers: game.toml [build.env] < .env < --env-file.
/// Then filter out keys already set in the shell environment.
/// Returns the final HashMap to pass to the subprocess.
pub fn merge_env(
    build_env: &[(String, String)],
    dotenv: &[(String, String)],
    env_file: &[(String, String)],
) -> HashMap<String, String> {
    let mut merged = HashMap::new();

    // Lowest priority first: game.toml [build.env]
    for (k, v) in build_env {
        merged.insert(k.clone(), v.clone());
    }
    // .env overwrites [build.env]
    for (k, v) in dotenv {
        merged.insert(k.clone(), v.clone());
    }
    // --env-file overwrites .env
    for (k, v) in env_file {
        merged.insert(k.clone(), v.clone());
    }

    // Shell env wins: remove keys already set in the process environment
    merged.retain(|k, _| std::env::var(k).is_err());

    merged
}
```

- [ ] **Step 4: Run tests to verify parse_env_file passes**

Run: `cargo test --package silm --test build_tests`
Expected: PASS for parse_env_file tests, FAIL for others (not yet written)

- [ ] **Step 5: Write and run tests for parse_build_env and parse_build_section**

Add to `engine/cli/tests/build_tests.rs`:
```rust
#[test]
fn test_parse_build_env_basic() {
    let content = "[build.env]\nSERVER_ADDRESS = \"ws://localhost:7777\"\nSERVER_PORT = \"7777\"\n";
    let vars = silm::commands::build::env::parse_build_env(content);
    assert_eq!(vars.len(), 2);
    assert_eq!(vars[0], ("SERVER_ADDRESS".to_string(), "ws://localhost:7777".to_string()));
}

#[test]
fn test_parse_build_env_ignores_other_sections() {
    let content = "[project]\nname = \"test\"\n\n[build.env]\nKEY = \"val\"\n\n[modules]\n";
    let vars = silm::commands::build::env::parse_build_env(content);
    assert_eq!(vars.len(), 1);
    assert_eq!(vars[0].0, "KEY");
}

#[test]
fn test_parse_build_env_empty_when_absent() {
    let content = "[project]\nname = \"test\"\n";
    let vars = silm::commands::build::env::parse_build_env(content);
    assert!(vars.is_empty());
}

#[test]
fn test_parse_build_section_platforms() {
    let content = "[build]\nplatforms = [\"native\", \"wasm\"]\n";
    let platforms = parse_build_section(content).unwrap();
    assert_eq!(platforms, vec!["native", "wasm"]);
}

#[test]
fn test_parse_build_section_absent() {
    let content = "[project]\nname = \"test\"\n";
    assert!(parse_build_section(content).is_none());
}

#[test]
fn test_parse_build_section_many_platforms() {
    let content = "[build]\nplatforms = [\"windows-x86_64\", \"linux-x86_64\", \"linux-arm64\", \"wasm\"]\n";
    let platforms = parse_build_section(content).unwrap();
    assert_eq!(platforms.len(), 4);
    assert_eq!(platforms[0], "windows-x86_64");
}
```

Run: `cargo test --package silm --test build_tests`
Expected: PASS

- [ ] **Step 6: Write and run tests for merge_env**

Add to `engine/cli/tests/build_tests.rs`:
```rust
#[test]
fn test_merge_env_priority_order() {
    let build_env = vec![("KEY".into(), "from_build".into())];
    let dotenv = vec![("KEY".into(), "from_dotenv".into())];
    let env_file = vec![("KEY".into(), "from_env_file".into())];
    let merged = silm::commands::build::env::merge_env(&build_env, &dotenv, &env_file);
    assert_eq!(merged.get("KEY").unwrap(), "from_env_file");
}

#[test]
fn test_merge_env_build_env_lowest() {
    let build_env = vec![("A".into(), "build".into())];
    let dotenv = vec![("B".into(), "dotenv".into())];
    let env_file: Vec<(String, String)> = vec![];
    let merged = silm::commands::build::env::merge_env(&build_env, &dotenv, &env_file);
    assert_eq!(merged.get("A").unwrap(), "build");
    assert_eq!(merged.get("B").unwrap(), "dotenv");
}

// NOTE: This test uses set_var/remove_var which is unsafe in multi-threaded contexts.
// Run with: cargo test --package silm --test build_tests -- --test-threads=1
// Or add serial_test crate for #[serial] attribute.
#[test]
fn test_merge_env_shell_wins() {
    unsafe { std::env::set_var("_SILM_BUILD_TEST_SHELL_VAR", "shell_value"); }
    let build_env = vec![("_SILM_BUILD_TEST_SHELL_VAR".into(), "build_value".into())];
    let merged = silm::commands::build::env::merge_env(&build_env, &[], &[]);
    assert!(!merged.contains_key("_SILM_BUILD_TEST_SHELL_VAR"));
    unsafe { std::env::remove_var("_SILM_BUILD_TEST_SHELL_VAR"); }
}
```

Run: `cargo test --package silm --test build_tests`
Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add engine/cli/src/commands/build/mod.rs engine/cli/src/commands/build/env.rs engine/cli/src/commands/mod.rs engine/cli/tests/build_tests.rs
git commit -m "feat(cli): add env parsing and merge for silm build"
```

---

### Task 2: Platform types, mapping, and path helpers

**Files:**
- Modify: `engine/cli/src/commands/build/mod.rs` — add Platform enum, mappings, path helpers
- Test: `engine/cli/tests/build_tests.rs` — add platform tests

**Context:** The `Platform` enum maps user-facing platform strings (e.g., `"windows-x86_64"`) to Rust target triples and tool selection. The `BuildKind` indicates what to build (server+client, server-only, client-only). Windows host detection uses `std::env::consts::OS`. All functions are pure — no I/O.

**Reference:** Spec section "Platform Targets" and "Binary names".

- [ ] **Step 1: Write failing tests for platform mapping**

Add to `engine/cli/tests/build_tests.rs`:
```rust
use silm::commands::build::{Platform, BuildTool, BuildKind, platform_from_str, dist_dir_name};
use silm::commands::build::package::zip_filename;

#[test]
fn test_platform_native() {
    let p = platform_from_str("native").unwrap();
    assert_eq!(p.build_tool(), BuildTool::Cargo);
    assert_eq!(p.build_kind(), BuildKind::ServerAndClient);
    assert_eq!(dist_dir_name(&p), "native");
}

#[test]
fn test_platform_server() {
    let p = platform_from_str("server").unwrap();
    assert_eq!(p.build_tool(), BuildTool::Cargo);
    assert_eq!(p.build_kind(), BuildKind::ServerOnly);
    assert_eq!(dist_dir_name(&p), "server");
}

#[test]
fn test_platform_wasm() {
    let p = platform_from_str("wasm").unwrap();
    assert_eq!(p.build_tool(), BuildTool::Trunk);
    assert_eq!(p.build_kind(), BuildKind::ClientOnly);
    assert_eq!(dist_dir_name(&p), "wasm");
}

#[test]
fn test_platform_windows_x86_64_on_windows() {
    let p = platform_from_str("windows-x86_64").unwrap();
    if cfg!(windows) {
        assert_eq!(p.target_triple(), "x86_64-pc-windows-msvc");
        assert_eq!(p.build_tool(), BuildTool::Cargo);
    } else {
        assert_eq!(p.target_triple(), "x86_64-pc-windows-gnu");
        assert_eq!(p.build_tool(), BuildTool::Cross);
    }
}

#[test]
fn test_platform_linux_x86_64() {
    let p = platform_from_str("linux-x86_64").unwrap();
    assert_eq!(p.target_triple(), "x86_64-unknown-linux-gnu");
    assert_eq!(p.build_tool(), BuildTool::Cross);
}

#[test]
fn test_platform_linux_arm64() {
    let p = platform_from_str("linux-arm64").unwrap();
    assert_eq!(p.target_triple(), "aarch64-unknown-linux-gnu");
    assert_eq!(p.build_tool(), BuildTool::Cross);
}

#[test]
fn test_platform_macos_x86_64() {
    let p = platform_from_str("macos-x86_64").unwrap();
    assert_eq!(p.target_triple(), "x86_64-apple-darwin");
    assert!(p.is_experimental());
}

#[test]
fn test_platform_macos_arm64() {
    let p = platform_from_str("macos-arm64").unwrap();
    assert_eq!(p.target_triple(), "aarch64-apple-darwin");
    assert!(p.is_experimental());
}

#[test]
fn test_platform_unknown_errors() {
    let err = platform_from_str("darwin").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("darwin"), "error should contain the bad platform name");
    assert!(msg.contains("native"), "error should list known platforms");
}

#[test]
fn test_zip_filename() {
    assert_eq!(zip_filename("my-game", "0.1.0", "native"), "my-game-v0.1.0-native.zip");
    assert_eq!(zip_filename("game", "0.0.0", "wasm"), "game-v0.0.0-wasm.zip");
}

#[test]
fn test_dist_dir_name_all_platforms() {
    for name in &["native", "server", "windows-x86_64", "linux-x86_64", "linux-arm64", "macos-x86_64", "macos-arm64", "wasm"] {
        let p = platform_from_str(name).unwrap();
        assert_eq!(dist_dir_name(&p), *name);
    }
}
```

Run: `cargo test --package silm --test build_tests`
Expected: FAIL — types not found

- [ ] **Step 2: Implement Platform types in mod.rs**

Replace `engine/cli/src/commands/build/mod.rs` with:
```rust
pub mod env;

use anyhow::{bail, Result};

/// Which tool to use for building.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildTool {
    Cargo,
    Cross,
    Trunk,
}

/// What to build for a platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildKind {
    ServerAndClient,
    ServerOnly,
    ClientOnly,
}

/// A resolved build platform with all its metadata.
#[derive(Debug, Clone)]
pub struct Platform {
    name: String,
    target_triple: String,
    tool: BuildTool,
    kind: BuildKind,
    experimental: bool,
    uses_exe_extension: bool,
}

impl Platform {
    pub fn name(&self) -> &str { &self.name }
    pub fn target_triple(&self) -> &str { &self.target_triple }
    pub fn build_tool(&self) -> BuildTool { self.tool }
    pub fn build_kind(&self) -> BuildKind { self.kind }
    pub fn is_experimental(&self) -> bool { self.experimental }
    pub fn uses_exe_extension(&self) -> bool { self.uses_exe_extension }
}

const KNOWN_PLATFORMS: &[&str] = &[
    "native", "server", "windows-x86_64", "linux-x86_64", "linux-arm64",
    "macos-x86_64", "macos-arm64", "wasm",
];

/// Parse a platform string into a resolved Platform.
pub fn platform_from_str(name: &str) -> Result<Platform> {
    let is_windows_host = std::env::consts::OS == "windows";

    match name {
        "native" => Ok(Platform {
            name: "native".into(),
            target_triple: host_target_triple(),
            tool: BuildTool::Cargo,
            kind: BuildKind::ServerAndClient,
            experimental: false,
            uses_exe_extension: is_windows_host,
        }),
        "server" => Ok(Platform {
            name: "server".into(),
            target_triple: host_target_triple(),
            tool: BuildTool::Cargo,
            kind: BuildKind::ServerOnly,
            experimental: false,
            uses_exe_extension: is_windows_host,
        }),
        "windows-x86_64" => {
            if is_windows_host {
                Ok(Platform {
                    name: "windows-x86_64".into(),
                    target_triple: "x86_64-pc-windows-msvc".into(),
                    tool: BuildTool::Cargo,
                    kind: BuildKind::ServerAndClient,
                    experimental: false,
                    uses_exe_extension: true,
                })
            } else {
                Ok(Platform {
                    name: "windows-x86_64".into(),
                    target_triple: "x86_64-pc-windows-gnu".into(),
                    tool: BuildTool::Cross,
                    kind: BuildKind::ServerAndClient,
                    experimental: false,
                    uses_exe_extension: true,
                })
            }
        }
        "linux-x86_64" => Ok(Platform {
            name: "linux-x86_64".into(),
            target_triple: "x86_64-unknown-linux-gnu".into(),
            tool: BuildTool::Cross,
            kind: BuildKind::ServerAndClient,
            experimental: false,
            uses_exe_extension: false,
        }),
        "linux-arm64" => Ok(Platform {
            name: "linux-arm64".into(),
            target_triple: "aarch64-unknown-linux-gnu".into(),
            tool: BuildTool::Cross,
            kind: BuildKind::ServerAndClient,
            experimental: false,
            uses_exe_extension: false,
        }),
        "macos-x86_64" => Ok(Platform {
            name: "macos-x86_64".into(),
            target_triple: "x86_64-apple-darwin".into(),
            tool: BuildTool::Cross,
            kind: BuildKind::ServerAndClient,
            experimental: true,
            uses_exe_extension: false,
        }),
        "macos-arm64" => Ok(Platform {
            name: "macos-arm64".into(),
            target_triple: "aarch64-apple-darwin".into(),
            tool: BuildTool::Cross,
            kind: BuildKind::ServerAndClient,
            experimental: true,
            uses_exe_extension: false,
        }),
        "wasm" => Ok(Platform {
            name: "wasm".into(),
            target_triple: "wasm32-unknown-unknown".into(),
            tool: BuildTool::Trunk,
            kind: BuildKind::ClientOnly,
            experimental: false,
            uses_exe_extension: false,
        }),
        _ => bail!(
            "unknown platform '{}' — known platforms: {}",
            name,
            KNOWN_PLATFORMS.join(", ")
        ),
    }
}

/// Returns the dist directory name for a platform (just the platform key).
pub fn dist_dir_name(platform: &Platform) -> &str {
    platform.name()
}

fn host_target_triple() -> String {
    let arch = std::env::consts::ARCH;
    let os = std::env::consts::OS;
    match (arch, os) {
        ("x86_64", "windows") => "x86_64-pc-windows-msvc".into(),
        ("x86_64", "linux") => "x86_64-unknown-linux-gnu".into(),
        ("x86_64", "macos") => "x86_64-apple-darwin".into(),
        ("aarch64", "linux") => "aarch64-unknown-linux-gnu".into(),
        ("aarch64", "macos") => "aarch64-apple-darwin".into(),
        _ => format!("{}-unknown-{}-unknown", arch, os),
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test --package silm --test build_tests`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add engine/cli/src/commands/build/mod.rs engine/cli/tests/build_tests.rs
git commit -m "feat(cli): add Platform types and mapping for silm build"
```

---

### Task 3: BuildRunner trait, tool detection, and Dockerfile generation

**Files:**
- Modify: `engine/cli/src/commands/build/mod.rs` — add BuildRunner trait, RealRunner, tool detection
- Modify: `engine/cli/src/commands/build/env.rs` — no changes
- Create: `engine/cli/src/commands/build/package.rs` (partial — just Dockerfile + zip filename helpers)
- Test: `engine/cli/tests/build_tests.rs` — add tool detection and Dockerfile tests

**Context:** The `BuildRunner` trait abstracts subprocess execution. `RealRunner` calls `std::process::Command`. Tests use `MockRunner` which captures invocations. Tool detection checks PATH for `trunk`, `cross`, and whether Docker is running. `generate_dockerfile` produces the Dockerfile content from `[build.env]` entries.

- [ ] **Step 1: Write failing tests for Dockerfile generation**

Add to `engine/cli/tests/build_tests.rs`:
```rust
use silm::commands::build::package::generate_dockerfile;

#[test]
fn test_generate_dockerfile_basic() {
    let env_entries = vec![
        ("SERVER_PORT".to_string(), "7777".to_string()),
    ];
    let dockerfile = generate_dockerfile(&env_entries);
    assert!(dockerfile.contains("FROM debian:bookworm-slim"));
    assert!(dockerfile.contains("COPY server /usr/local/bin/server"));
    assert!(dockerfile.contains("EXPOSE 7777/udp"));
    assert!(dockerfile.contains("ENV SERVER_PORT=7777"));
    assert!(dockerfile.contains("ENTRYPOINT [\"/usr/local/bin/server\"]"));
}

#[test]
fn test_generate_dockerfile_multiple_env() {
    let env_entries = vec![
        ("SERVER_PORT".to_string(), "7777".to_string()),
        ("SERVER_ADDRESS".to_string(), "ws://localhost:7777".to_string()),
    ];
    let dockerfile = generate_dockerfile(&env_entries);
    assert!(dockerfile.contains("ENV SERVER_PORT=7777"));
    assert!(dockerfile.contains("ENV SERVER_ADDRESS=ws://localhost:7777"));
}

#[test]
fn test_generate_dockerfile_no_env() {
    let dockerfile = generate_dockerfile(&[]);
    assert!(dockerfile.contains("FROM debian:bookworm-slim"));
    assert!(dockerfile.contains("ENTRYPOINT"));
    assert!(!dockerfile.contains("ENV "));
}
```

Run: `cargo test --package silm --test build_tests`
Expected: FAIL — module not found

- [ ] **Step 2: Implement Dockerfile generation in package.rs**

Create `engine/cli/src/commands/build/package.rs`:
```rust
use std::path::Path;

/// Construct zip filename: <name>-v<version>-<platform>.zip
pub fn zip_filename(project_name: &str, version: &str, platform_name: &str) -> String {
    format!("{}-v{}-{}.zip", project_name, version, platform_name)
}

/// Generate Dockerfile content for the server platform.
/// Includes ENV lines for each entry from [build.env].
pub fn generate_dockerfile(env_entries: &[(String, String)]) -> String {
    let mut lines = Vec::new();
    lines.push("FROM debian:bookworm-slim".to_string());
    lines.push("COPY server /usr/local/bin/server".to_string());
    lines.push("EXPOSE 7777/udp".to_string());
    lines.push(String::new());

    if !env_entries.is_empty() {
        lines.push("# Override at runtime: docker run -e KEY=value ...".to_string());
        for (key, value) in env_entries {
            lines.push(format!("ENV {}={}", key, value));
        }
        lines.push(String::new());
    }

    lines.push("ENTRYPOINT [\"/usr/local/bin/server\"]".to_string());
    lines.push(String::new());
    lines.join("\n")
}

/// Construct zip filename: <name>-v<version>-<platform>.zip
pub fn zip_filename(project_name: &str, version: &str, platform_name: &str) -> String {
    format!("{}-v{}-{}.zip", project_name, version, platform_name)
}
```

Update `engine/cli/src/commands/build/mod.rs` to add:
```rust
pub mod package;
```

- [ ] **Step 3: Add BuildRunner trait and RealRunner to mod.rs**

Add to `engine/cli/src/commands/build/mod.rs`:
```rust
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

/// Trait abstracting subprocess execution for testability.
pub trait BuildRunner {
    fn run_command(
        &self,
        program: &str,
        args: &[String],
        env: &HashMap<String, String>,
        cwd: &Path,
    ) -> Result<()>;
}

/// Real runner that executes commands via std::process::Command.
pub struct RealRunner;

impl BuildRunner for RealRunner {
    fn run_command(
        &self,
        program: &str,
        args: &[String],
        env: &HashMap<String, String>,
        cwd: &Path,
    ) -> Result<()> {
        tracing::info!("[silm] running: {} {}", program, args.join(" "));
        let status = Command::new(program)
            .args(args)
            .envs(env)
            .current_dir(cwd)
            .status()
            .map_err(|e| anyhow::anyhow!("failed to execute '{}': {}", program, e))?;
        if !status.success() {
            bail!("'{}' exited with status: {}", program, status);
        }
        Ok(())
    }
}

/// Check if a tool is available on PATH by running `<tool> --version`.
pub fn check_tool(tool: &str) -> Result<()> {
    match Command::new(tool).arg("--version").output() {
        Ok(output) if output.status.success() => Ok(()),
        _ => bail!("'{}' not found — install: cargo install {}", tool, tool),
    }
}

/// Check if Docker is running by running `docker info`.
pub fn check_docker() -> Result<()> {
    match Command::new("docker").arg("info").output() {
        Ok(output) if output.status.success() => Ok(()),
        _ => bail!("Docker is not running — start Docker Desktop, then retry"),
    }
}
```

- [ ] **Step 4: Write tool detection tests**

Add to `engine/cli/tests/build_tests.rs`:
```rust
// Tool detection tests — these test the error message format.
// We can't reliably test presence/absence of tools in unit tests,
// so we test the error message construction instead.
// The actual tool detection is tested in E2E tests.

#[test]
fn test_check_tool_error_message_format() {
    // Use a tool name that definitely doesn't exist
    let result = silm::commands::build::check_tool("silm_nonexistent_tool_xyz");
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("silm_nonexistent_tool_xyz"));
    assert!(msg.contains("not found"));
    assert!(msg.contains("cargo install"));
}
```

Run: `cargo test --package silm --test build_tests`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add engine/cli/src/commands/build/mod.rs engine/cli/src/commands/build/package.rs engine/cli/tests/build_tests.rs
git commit -m "feat(cli): add BuildRunner trait, tool detection, and Dockerfile generation"
```

---

## Chunk 2: Build implementations — native, WASM, orchestration

### Task 4: Native build implementation (`native.rs`)

**Files:**
- Create: `engine/cli/src/commands/build/native.rs`
- Modify: `engine/cli/src/commands/build/mod.rs` — add `pub mod native;`
- Test: `engine/cli/tests/build_tests.rs` — verify command construction

**Context:** `build_native()` constructs `cargo build` or `cross build` commands for the server and/or client binaries. It uses the `BuildRunner` trait, so tests inject a `MockRunner` that captures command arguments. The function reads `server_package` and `client_package` from the caller (parsed from game.toml `[dev]`).

- [ ] **Step 1: Write failing tests with MockRunner**

Add to `engine/cli/tests/build_tests.rs`:
```rust
use silm::commands::build::{BuildRunner, BuildTool, BuildKind};
use silm::commands::build::native::build_native;
use std::cell::RefCell;
use std::path::PathBuf;

#[derive(Debug, Clone)]
struct CapturedCommand {
    program: String,
    args: Vec<String>,
    env: HashMap<String, String>,
}

struct MockRunner {
    commands: RefCell<Vec<CapturedCommand>>,
}

impl MockRunner {
    fn new() -> Self {
        Self { commands: RefCell::new(Vec::new()) }
    }
    fn captured(&self) -> Vec<CapturedCommand> {
        self.commands.borrow().clone()
    }
}

impl BuildRunner for MockRunner {
    fn run_command(
        &self,
        program: &str,
        args: &[String],
        env: &HashMap<String, String>,
        _cwd: &std::path::Path,
    ) -> anyhow::Result<()> {
        self.commands.borrow_mut().push(CapturedCommand {
            program: program.to_string(),
            args: args.to_vec(),
            env: env.clone(),
        });
        Ok(())
    }
}

#[test]
fn test_build_native_cargo_server_and_client() {
    let runner = MockRunner::new();
    let env = HashMap::new();
    let cwd = PathBuf::from("/tmp/project");

    build_native(
        &runner, &cwd, &env,
        "my-game-server", "my-game-client",
        BuildTool::Cargo, None, // no target triple override for native
        BuildKind::ServerAndClient,
        false, // not release
    ).unwrap();

    let cmds = runner.captured();
    assert_eq!(cmds.len(), 2);
    assert_eq!(cmds[0].program, "cargo");
    assert!(cmds[0].args.contains(&"--package".to_string()));
    assert!(cmds[0].args.contains(&"my-game-server".to_string()));
    assert!(cmds[0].args.contains(&"--bin".to_string()));
    assert!(cmds[0].args.contains(&"server".to_string()));
    assert_eq!(cmds[1].program, "cargo");
    assert!(cmds[1].args.contains(&"my-game-client".to_string()));
}

#[test]
fn test_build_native_server_only() {
    let runner = MockRunner::new();
    let env = HashMap::new();
    let cwd = PathBuf::from("/tmp/project");

    build_native(
        &runner, &cwd, &env,
        "my-game-server", "my-game-client",
        BuildTool::Cargo, None,
        BuildKind::ServerOnly,
        false,
    ).unwrap();

    let cmds = runner.captured();
    assert_eq!(cmds.len(), 1);
    assert!(cmds[0].args.contains(&"server".to_string()));
}

#[test]
fn test_build_native_release_flag() {
    let runner = MockRunner::new();
    let env = HashMap::new();
    let cwd = PathBuf::from("/tmp/project");

    build_native(
        &runner, &cwd, &env,
        "my-game-server", "my-game-client",
        BuildTool::Cargo, None,
        BuildKind::ServerAndClient,
        true, // release
    ).unwrap();

    let cmds = runner.captured();
    assert!(cmds[0].args.contains(&"--release".to_string()));
}

#[test]
fn test_build_native_cross_with_target() {
    let runner = MockRunner::new();
    let env = HashMap::new();
    let cwd = PathBuf::from("/tmp/project");

    build_native(
        &runner, &cwd, &env,
        "my-game-server", "my-game-client",
        BuildTool::Cross, Some("x86_64-pc-windows-gnu"),
        BuildKind::ServerAndClient,
        true,
    ).unwrap();

    let cmds = runner.captured();
    assert_eq!(cmds[0].program, "cross");
    assert!(cmds[0].args.contains(&"--target".to_string()));
    assert!(cmds[0].args.contains(&"x86_64-pc-windows-gnu".to_string()));
}
```

Run: `cargo test --package silm --test build_tests`
Expected: FAIL — `native` module not found

- [ ] **Step 2: Implement build_native**

Create `engine/cli/src/commands/build/native.rs`:
```rust
use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;

use super::{BuildRunner, BuildTool, BuildKind};

/// Build server and/or client using cargo or cross.
pub fn build_native(
    runner: &dyn BuildRunner,
    project_root: &Path,
    env: &HashMap<String, String>,
    server_package: &str,
    client_package: &str,
    tool: BuildTool,
    target_triple: Option<&str>,
    kind: BuildKind,
    release: bool,
) -> Result<()> {
    let program = match tool {
        BuildTool::Cargo => "cargo",
        BuildTool::Cross => "cross",
        BuildTool::Trunk => anyhow::bail!("trunk is not used for native builds"),
    };

    let build_binary = |pkg: &str, bin: &str| -> Result<()> {
        let mut args = vec!["build".to_string()];
        if let Some(triple) = target_triple {
            args.push("--target".to_string());
            args.push(triple.to_string());
        }
        args.push("--package".to_string());
        args.push(pkg.to_string());
        args.push("--bin".to_string());
        args.push(bin.to_string());
        if release {
            args.push("--release".to_string());
        }
        runner.run_command(program, &args, env, project_root)
    };

    match kind {
        BuildKind::ServerAndClient => {
            build_binary(server_package, "server")?;
            build_binary(client_package, "client")?;
        }
        BuildKind::ServerOnly => {
            build_binary(server_package, "server")?;
        }
        BuildKind::ClientOnly => {
            build_binary(client_package, "client")?;
        }
    }

    Ok(())
}
```

Add to `engine/cli/src/commands/build/mod.rs`:
```rust
pub mod native;
```

- [ ] **Step 3: Run tests**

Run: `cargo test --package silm --test build_tests`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add engine/cli/src/commands/build/native.rs engine/cli/src/commands/build/mod.rs engine/cli/tests/build_tests.rs
git commit -m "feat(cli): add native build implementation (cargo/cross)"
```

---

### Task 5: WASM build implementation (`wasm.rs`)

**Files:**
- Create: `engine/cli/src/commands/build/wasm.rs`
- Modify: `engine/cli/src/commands/build/mod.rs` — add `pub mod wasm;`
- Test: `engine/cli/tests/build_tests.rs`

**Context:** `build_wasm()` invokes `trunk build client/index.html --dist dist/wasm [--release]`. It uses the same `BuildRunner` trait as native builds.

- [ ] **Step 1: Write failing tests**

Add to `engine/cli/tests/build_tests.rs`:
```rust
use silm::commands::build::wasm::build_wasm;

#[test]
fn test_build_wasm_debug() {
    let runner = MockRunner::new();
    let env = HashMap::new();
    let cwd = PathBuf::from("/tmp/project");

    build_wasm(&runner, &cwd, &env, false).unwrap();

    let cmds = runner.captured();
    assert_eq!(cmds.len(), 1);
    assert_eq!(cmds[0].program, "trunk");
    assert!(cmds[0].args.contains(&"build".to_string()));
    assert!(cmds[0].args.contains(&"client/index.html".to_string()));
    assert!(cmds[0].args.contains(&"--dist".to_string()));
    assert!(!cmds[0].args.contains(&"--release".to_string()));
}

#[test]
fn test_build_wasm_release() {
    let runner = MockRunner::new();
    let env = HashMap::new();
    let cwd = PathBuf::from("/tmp/project");

    build_wasm(&runner, &cwd, &env, true).unwrap();

    let cmds = runner.captured();
    assert!(cmds[0].args.contains(&"--release".to_string()));
}
```

Run: `cargo test --package silm --test build_tests`
Expected: FAIL

- [ ] **Step 2: Implement build_wasm**

Create `engine/cli/src/commands/build/wasm.rs`:
```rust
use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;

use super::BuildRunner;

/// Build WASM client using trunk.
pub fn build_wasm(
    runner: &dyn BuildRunner,
    project_root: &Path,
    env: &HashMap<String, String>,
    release: bool,
) -> Result<()> {
    let mut args = vec![
        "build".to_string(),
        "client/index.html".to_string(),
        "--dist".to_string(),
        "dist/wasm".to_string(),
    ];
    if release {
        args.push("--release".to_string());
    }
    runner.run_command("trunk", &args, env, project_root)
}
```

Add `pub mod wasm;` to mod.rs.

- [ ] **Step 3: Run tests**

Run: `cargo test --package silm --test build_tests`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add engine/cli/src/commands/build/wasm.rs engine/cli/src/commands/build/mod.rs engine/cli/tests/build_tests.rs
git commit -m "feat(cli): add WASM build implementation (trunk)"
```

---

### Task 6: Build command orchestration (`handle_build_command`)

**Files:**
- Modify: `engine/cli/src/commands/build/mod.rs` — add clap enums, handle_build_command, game.toml parsing helpers
- Test: `engine/cli/tests/build_tests.rs`

**Context:** `handle_build_command` is the top-level entry point. It resolves platforms (from `--platform` flag or `game.toml [build]`), reads `[dev]` for package names, merges env vars, detects required tools, then dispatches to `build_native` or `build_wasm` for each platform. macOS failures are non-fatal; other failures are fatal.

**Reference:** Spec sections "Commands", "game.toml Integration", "Tool Detection".

- [ ] **Step 1: Add clap command enums to mod.rs**

Add to `engine/cli/src/commands/build/mod.rs`:
```rust
use clap::Args;

#[derive(Args, Debug)]
pub struct BuildCommand {
    /// Target platform (e.g., native, wasm, windows-x86_64, linux-x86_64)
    #[arg(long)]
    pub platform: Option<String>,

    /// Build in release mode (LTO, optimizations)
    #[arg(long)]
    pub release: bool,

    /// Load environment variables from file (default: .env)
    #[arg(long)]
    pub env_file: Option<String>,
}

#[derive(Args, Debug)]
pub struct PackageCommand {
    /// Target platform (e.g., native, wasm, windows-x86_64)
    #[arg(long)]
    pub platform: Option<String>,

    /// Output directory for zip files (default: project root)
    #[arg(long)]
    pub out_dir: Option<String>,
}
```

- [ ] **Step 2: Add game.toml [dev] parsing helper**

Add to `engine/cli/src/commands/build/mod.rs`:
Uses the `toml` crate (already a dependency) for robust parsing:
```rust
/// Parse [dev] section from game.toml for server_package and client_package.
/// Falls back to <project_name>-server / <project_name>-client if absent.
pub fn parse_dev_section(game_toml_content: &str, project_name: &str) -> (String, String) {
    let fallback_server = format!("{}-server", project_name);
    let fallback_client = format!("{}-client", project_name);

    let parsed: toml::Value = match game_toml_content.parse() {
        Ok(v) => v,
        Err(_) => return (fallback_server, fallback_client),
    };

    let dev = match parsed.get("dev") {
        Some(d) => d,
        None => return (fallback_server, fallback_client),
    };

    let server_pkg = dev.get("server_package")
        .and_then(|v| v.as_str())
        .map(String::from)
        .unwrap_or(fallback_server);
    let client_pkg = dev.get("client_package")
        .and_then(|v| v.as_str())
        .map(String::from)
        .unwrap_or(fallback_client);

    (server_pkg, client_pkg)
}

/// Parse [project] name from game.toml.
pub fn parse_project_name(game_toml_content: &str) -> Option<String> {
    let parsed: toml::Value = game_toml_content.parse().ok()?;
    parsed.get("project")?.get("name")?.as_str().map(String::from)
}

/// Parse [project] version from game.toml. Defaults to "0.0.0".
pub fn parse_project_version(game_toml_content: &str) -> String {
    let parsed: toml::Value = match game_toml_content.parse() {
        Ok(v) => v,
        Err(_) => return "0.0.0".to_string(),
    };
    parsed.get("project")
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .map(String::from)
        .unwrap_or_else(|| "0.0.0".to_string())
}
```

- [ ] **Step 3: Write tests for dev section parsing**

Add to `engine/cli/tests/build_tests.rs`:
```rust
use silm::commands::build::{parse_dev_section, parse_project_name, parse_project_version};

#[test]
fn test_parse_dev_section() {
    let content = "[dev]\nserver_package = \"my-server\"\nclient_package = \"my-client\"\n";
    let (s, c) = parse_dev_section(content, "fallback");
    assert_eq!(s, "my-server");
    assert_eq!(c, "my-client");
}

#[test]
fn test_parse_dev_section_fallback() {
    let content = "[project]\nname = \"test\"\n";
    let (s, c) = parse_dev_section(content, "my-game");
    assert_eq!(s, "my-game-server");
    assert_eq!(c, "my-game-client");
}

#[test]
fn test_parse_project_name() {
    let content = "[project]\nname = \"my-game\"\nversion = \"0.1.0\"\n";
    assert_eq!(parse_project_name(content).unwrap(), "my-game");
}

#[test]
fn test_parse_project_version() {
    let content = "[project]\nname = \"x\"\nversion = \"1.2.3\"\n";
    assert_eq!(parse_project_version(content), "1.2.3");
}

#[test]
fn test_parse_project_version_missing() {
    let content = "[project]\nname = \"x\"\n";
    assert_eq!(parse_project_version(content), "0.0.0");
}
```

Run: `cargo test --package silm --test build_tests`
Expected: PASS

- [ ] **Step 4: Implement handle_build_command**

Add to `engine/cli/src/commands/build/mod.rs`:
```rust
use std::fs;
use std::path::PathBuf;

/// Main entry point for `silm build`.
pub fn handle_build_command(cmd: BuildCommand, project_root: PathBuf) -> Result<()> {
    let game_toml_content = fs::read_to_string(project_root.join("game.toml"))?;

    let project_name = parse_project_name(&game_toml_content)
        .ok_or_else(|| anyhow::anyhow!("game.toml is missing [project] name"))?;

    // Resolve platforms
    let platform_names = if let Some(ref p) = cmd.platform {
        vec![p.clone()]
    } else {
        match env::parse_build_section(&game_toml_content) {
            Some(platforms) => platforms,
            None => bail!(
                "no platforms specified — add [build] platforms = [...] to game.toml, \
                 or use --platform <name>"
            ),
        }
    };

    // Parse [dev] for package names
    let (server_pkg, client_pkg) = parse_dev_section(&game_toml_content, &project_name);

    // Merge env vars
    let build_env_entries = env::parse_build_env(&game_toml_content);
    let dotenv_content = fs::read_to_string(project_root.join(".env")).unwrap_or_default();
    let dotenv_entries = env::parse_env_file(&dotenv_content);
    let env_file_entries = if let Some(ref path) = cmd.env_file {
        let content = fs::read_to_string(project_root.join(path))?;
        env::parse_env_file(&content)
    } else {
        Vec::new()
    };
    let merged_env = env::merge_env(&build_env_entries, &dotenv_entries, &env_file_entries);

    let runner = RealRunner;

    for name in &platform_names {
        let platform = platform_from_str(name)?;

        if platform.is_experimental() {
            tracing::warn!("[silm] platform '{}' is experimental — build may fail", name);
        }

        let result = build_platform(&runner, &project_root, &merged_env, &platform, &server_pkg, &client_pkg, cmd.release);

        if let Err(e) = result {
            if platform.is_experimental() {
                tracing::warn!("[silm] experimental platform '{}' failed: {}", name, e);
                continue;
            }
            return Err(e);
        }
    }

    tracing::info!("[silm] build complete");
    Ok(())
}

/// Build a single platform. Handles tool detection and dispatch.
fn build_platform(
    runner: &dyn BuildRunner,
    project_root: &Path,
    env: &HashMap<String, String>,
    platform: &Platform,
    server_pkg: &str,
    client_pkg: &str,
    release: bool,
) -> Result<()> {
    match platform.build_tool() {
        BuildTool::Trunk => {
            check_tool("trunk")?;
            if !project_root.join("client/index.html").exists() {
                bail!("WASM build requires client/index.html — not found");
            }
            wasm::build_wasm(runner, project_root, env, release)
        }
        BuildTool::Cross => {
            check_tool("cross")?;
            check_docker()?;
            if platform.name().starts_with("macos-") {
                if std::env::var("MACOS_SDK_URL").is_err() {
                    bail!(
                        "macOS cross-build requires MACOS_SDK_URL — \
                         see: https://github.com/cross-rs/cross/wiki/Recipes"
                    );
                }
            }
            native::build_native(
                runner, project_root, env,
                server_pkg, client_pkg,
                BuildTool::Cross, Some(platform.target_triple()),
                platform.build_kind(), release,
            )
        }
        BuildTool::Cargo => {
            let target = if platform.name() == "native" || platform.name() == "server" {
                None // host triple, no --target needed
            } else {
                Some(platform.target_triple())
            };
            native::build_native(
                runner, project_root, env,
                server_pkg, client_pkg,
                BuildTool::Cargo, target,
                platform.build_kind(), release,
            )
        }
    }
}
```

- [ ] **Step 5: Write orchestration tests**

Add to `engine/cli/tests/build_tests.rs`:
```rust
use silm::commands::build::build_all_platforms;
use tempfile::TempDir;

#[test]
fn test_orchestration_missing_build_no_platform_errors() {
    let game_toml = "[project]\nname = \"test\"\n[dev]\nserver_package = \"t-s\"\nclient_package = \"t-c\"\n";
    let platforms = silm::commands::build::env::parse_build_section(game_toml);
    assert!(platforms.is_none(), "should return None when [build] is absent");
}

#[test]
fn test_orchestration_native_build_dispatches() {
    let dir = TempDir::new().unwrap();
    let game_toml = "[project]\nname = \"test\"\n\n[dev]\nserver_package = \"t-s\"\nclient_package = \"t-c\"\n";
    fs::write(dir.path().join("game.toml"), game_toml).unwrap();

    let runner = MockRunner::new();
    build_all_platforms(
        &runner, dir.path(), game_toml,
        &["native".to_string()], false, None,
    ).unwrap();

    let cmds = runner.captured();
    assert_eq!(cmds.len(), 2, "should build server + client");
    assert_eq!(cmds[0].program, "cargo");
}

#[test]
fn test_orchestration_unknown_platform_errors() {
    let dir = TempDir::new().unwrap();
    let game_toml = "[project]\nname = \"test\"\n[dev]\nserver_package = \"t-s\"\nclient_package = \"t-c\"\n";
    fs::write(dir.path().join("game.toml"), game_toml).unwrap();

    let runner = MockRunner::new();
    let result = build_all_platforms(
        &runner, dir.path(), game_toml,
        &["darwin".to_string()], false, None,
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("darwin"));
}
```

- [ ] **Step 6: Run all tests**

Run: `cargo test --package silm --test build_tests`
Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add engine/cli/src/commands/build/mod.rs engine/cli/tests/build_tests.rs
git commit -m "feat(cli): add handle_build_command orchestration"
```

---

## Chunk 3: Package, CLI wiring, templates, and integration tests

### Task 7: Package command — dist assembly, zip, Dockerfile (`package.rs`)

**Files:**
- Modify: `engine/cli/src/commands/build/package.rs` — add dist assembly, zip, asset copy, handle_package_command
- Modify: `engine/cli/Cargo.toml` — add `zip = "2"` dependency
- Test: `engine/cli/tests/build_tests.rs`

**Context:** `handle_package_command` calls `handle_build_command` with `--release`, then for each platform: wipes and creates `dist/<platform>/`, copies binaries from `target/`, copies `assets/` if present, generates Dockerfile for `server` platform, and creates a zip archive. Uses the `zip` crate v2.

- [ ] **Step 1: Add zip dependency**

Add to `engine/cli/Cargo.toml` in `[dependencies]` section:
```toml
zip = "2"
```

- [ ] **Step 2: Write tests for create_zip and assemble_dist helpers**

Add to `engine/cli/tests/build_tests.rs`:
```rust
use silm::commands::build::package::{generate_dockerfile, create_zip, copy_assets};
use tempfile::TempDir;

#[test]
fn test_create_zip() {
    let dir = TempDir::new().unwrap();
    let src = dir.path().join("src_dir");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("file1.txt"), "hello").unwrap();
    fs::write(src.join("file2.txt"), "world").unwrap();

    let zip_path = dir.path().join("test.zip");
    create_zip(&src, &zip_path).unwrap();

    assert!(zip_path.exists());
    assert!(fs::metadata(&zip_path).unwrap().len() > 0);
}

#[test]
fn test_create_zip_nested_dirs() {
    let dir = TempDir::new().unwrap();
    let src = dir.path().join("src_dir");
    fs::create_dir_all(src.join("assets/textures")).unwrap();
    fs::write(src.join("assets/textures/player.png"), "fake png").unwrap();
    fs::write(src.join("server"), "fake binary").unwrap();

    let zip_path = dir.path().join("test.zip");
    create_zip(&src, &zip_path).unwrap();

    assert!(zip_path.exists());
}

#[test]
fn test_copy_assets_present() {
    let dir = TempDir::new().unwrap();
    let project = dir.path().join("project");
    fs::create_dir_all(project.join("assets/textures")).unwrap();
    fs::write(project.join("assets/textures/player.png"), "img").unwrap();
    fs::write(project.join("assets/config.ron"), "config").unwrap();

    let dest = dir.path().join("dist/native");
    fs::create_dir_all(&dest).unwrap();

    copy_assets(&project, &dest).unwrap();

    assert!(dest.join("assets/textures/player.png").exists());
    assert!(dest.join("assets/config.ron").exists());
}

#[test]
fn test_copy_assets_absent_no_error() {
    let dir = TempDir::new().unwrap();
    let project = dir.path().join("project");
    fs::create_dir_all(&project).unwrap();
    // No assets/ directory

    let dest = dir.path().join("dist/native");
    fs::create_dir_all(&dest).unwrap();

    // Should not error
    copy_assets(&project, &dest).unwrap();
}
```

Run: `cargo test --package silm --test build_tests`
Expected: FAIL — functions not found

- [ ] **Step 3: Implement create_zip, copy_assets in package.rs**

Update `engine/cli/src/commands/build/package.rs` — add `create_zip`, `copy_assets`, and assembly functions. The `generate_dockerfile` and `zip_filename` functions already exist from Task 3. Add the following:
```rust
use anyhow::Result;
use std::fs;
use std::io::Write;
use std::path::Path;
use walkdir::WalkDir;  // already a dependency of the CLI crate

/// Create a zip archive from a directory.
// TODO(CLI.7): cargo-packager integration for AppImage/DMG/NSIS
pub fn create_zip(source_dir: &Path, zip_path: &Path) -> Result<()> {
    let file = fs::File::create(zip_path)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    for entry in WalkDir::new(source_dir) {
        let entry = entry?;
        let path = entry.path();
        let relative = path.strip_prefix(source_dir)?;

        if relative.as_os_str().is_empty() {
            continue;
        }

        let name = relative.to_string_lossy().replace('\\', "/");

        if path.is_dir() {
            zip.add_directory(&name, options)?;
        } else {
            zip.start_file(&name, options)?;
            let data = fs::read(path)?;
            zip.write_all(&data)?;
        }
    }

    zip.finish()?;
    Ok(())
}

/// Copy assets/ from project root to dist/<platform>/assets/ if present.
/// Silently skips if assets/ does not exist.
pub fn copy_assets(project_root: &Path, dist_platform_dir: &Path) -> Result<()> {
    let assets_src = project_root.join("assets");
    if !assets_src.is_dir() {
        return Ok(());
    }

    let assets_dest = dist_platform_dir.join("assets");
    copy_dir_recursive(&assets_src, &assets_dest)?;
    Ok(())
}

fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<()> {
    fs::create_dir_all(dest)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dest_path)?;
        } else {
            fs::copy(&src_path, &dest_path)?;
        }
    }
    Ok(())
}

/// Assemble dist/<platform>/ for a single native/cross platform.
/// Copies binaries from target/ and assets/ from project root.
pub fn assemble_native_dist(
    project_root: &Path,
    platform_name: &str,
    target_triple: Option<&str>,
    server_binary: bool,
    client_binary: bool,
    exe_extension: bool,
) -> Result<std::path::PathBuf> {
    let dist_dir = project_root.join("dist").join(platform_name);

    // Wipe and recreate
    if dist_dir.exists() {
        fs::remove_dir_all(&dist_dir)?;
    }
    fs::create_dir_all(&dist_dir)?;

    let ext = if exe_extension { ".exe" } else { "" };

    // Determine binary source directory
    let target_base = project_root.join("target");
    let bin_dir = if let Some(triple) = target_triple {
        target_base.join(triple).join("release")
    } else {
        target_base.join("release")
    };

    if server_binary {
        let src = bin_dir.join(format!("server{}", ext));
        if src.exists() {
            fs::copy(&src, dist_dir.join(format!("server{}", ext)))?;
        } else {
            tracing::warn!("[silm] server binary not found at {}", src.display());
        }
    }

    if client_binary {
        let src = bin_dir.join(format!("client{}", ext));
        if src.exists() {
            fs::copy(&src, dist_dir.join(format!("client{}", ext)))?;
        } else {
            tracing::warn!("[silm] client binary not found at {}", src.display());
        }
    }

    copy_assets(project_root, &dist_dir)?;

    Ok(dist_dir)
}

/// Assemble dist/server/ with server binary + generated Dockerfile.
pub fn assemble_server_dist(
    project_root: &Path,
    env_entries: &[(String, String)],
    exe_extension: bool,
) -> Result<std::path::PathBuf> {
    let dist_dir = project_root.join("dist").join("server");

    if dist_dir.exists() {
        fs::remove_dir_all(&dist_dir)?;
    }
    fs::create_dir_all(&dist_dir)?;

    let ext = if exe_extension { ".exe" } else { "" };
    let bin_dir = project_root.join("target").join("release");
    let src = bin_dir.join(format!("server{}", ext));
    if src.exists() {
        fs::copy(&src, dist_dir.join(format!("server{}", ext)))?;
    }

    let dockerfile = generate_dockerfile(env_entries);
    fs::write(dist_dir.join("Dockerfile"), dockerfile)?;

    Ok(dist_dir)
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test --package silm --test build_tests`
Expected: PASS

- [ ] **Step 5: Implement handle_package_command in mod.rs**

Add to `engine/cli/src/commands/build/mod.rs`:
```rust
/// Main entry point for `silm package`.
pub fn handle_package_command(cmd: PackageCommand, project_root: PathBuf) -> Result<()> {
    let game_toml_content = fs::read_to_string(project_root.join("game.toml"))?;
    let project_name = parse_project_name(&game_toml_content)
        .ok_or_else(|| anyhow::anyhow!("game.toml is missing [project] name"))?;
    let version = parse_project_version(&game_toml_content);

    // Run release build first
    let build_cmd = BuildCommand {
        platform: cmd.platform.clone(),
        release: true,
        env_file: None,
    };
    handle_build_command(build_cmd, project_root.clone())?;

    // Resolve platforms
    let platform_names = if let Some(ref p) = cmd.platform {
        vec![p.clone()]
    } else {
        env::parse_build_section(&game_toml_content)
            .ok_or_else(|| anyhow::anyhow!("no platforms specified"))?
    };

    let build_env_entries = env::parse_build_env(&game_toml_content);
    let out_dir = cmd.out_dir
        .as_ref()
        .map(|d| project_root.join(d))
        .unwrap_or_else(|| project_root.clone());
    fs::create_dir_all(&out_dir)?;

    let is_windows_host = std::env::consts::OS == "windows";

    for name in &platform_names {
        let platform = platform_from_str(name)?;

        tracing::info!("[silm] packaging platform: {}", name);

        let dist_dir = match platform.build_tool() {
            BuildTool::Trunk => {
                // Trunk already outputs to dist/wasm via --dist
                let dir = project_root.join("dist").join("wasm");
                if !dir.exists() {
                    tracing::warn!("[silm] dist/wasm/ not found — was WASM build successful?");
                    continue;
                }
                dir
            }
            _ => {
                if platform.name() == "server" {
                    package::assemble_server_dist(
                        &project_root,
                        &build_env_entries,
                        is_windows_host,
                    )?
                } else {
                    let target_triple = if platform.name() == "native" {
                        None
                    } else {
                        Some(platform.target_triple())
                    };
                    package::assemble_native_dist(
                        &project_root,
                        platform.name(),
                        target_triple,
                        platform.build_kind() != BuildKind::ClientOnly,
                        platform.build_kind() != BuildKind::ServerOnly,
                        platform.uses_exe_extension(),
                    )?
                }
            }
        };

        // Create zip
        let zip_name = package::zip_filename(&project_name, &version, platform.name());
        let zip_path = out_dir.join(&zip_name);
        package::create_zip(&dist_dir, &zip_path)?;
        tracing::info!("[silm] created {}", zip_name);
    }

    tracing::info!("[silm] packaging complete");
    Ok(())
}
```

**Note:** `zip_filename` is defined only in `package.rs` (not `mod.rs`). Tests import it via `silm::commands::build::package::zip_filename`.

- [ ] **Step 6: Run all tests**

Run: `cargo test --package silm --test build_tests`
Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add engine/cli/src/commands/build/package.rs engine/cli/src/commands/build/mod.rs engine/cli/Cargo.toml engine/cli/tests/build_tests.rs
git commit -m "feat(cli): add silm package — dist assembly, zip creation, Dockerfile"
```

---

### Task 8: CLI registration in main.rs

**Files:**
- Modify: `engine/cli/src/main.rs` — add Build and Package commands
- Test: compile + `cargo test --package silm`

**Context:** Wire `BuildCommand` and `PackageCommand` into the top-level `Commands` enum and dispatch them in `main()`. Follow the same pattern as the `Module` command: find project root, then call handler.

- [ ] **Step 1: Add Build and Package to Commands enum**

In `engine/cli/src/main.rs`, add after the `Module` variant (line 53):
```rust
    /// Build the game for one or more target platforms
    Build {
        #[command(flatten)]
        command: commands::build::BuildCommand,
    },

    /// Package the game into distributable zip archives
    Package {
        #[command(flatten)]
        command: commands::build::PackageCommand,
    },
```

- [ ] **Step 2: Add dispatch in match block**

In the `match cli.command` block (after `Commands::Module` arm, around line 77):
```rust
        Commands::Build { command } => {
            let cwd = std::env::current_dir()?;
            let project_root = commands::add::wiring::find_project_root(&cwd)?;
            commands::build::handle_build_command(command, project_root)?;
        }
        Commands::Package { command } => {
            let cwd = std::env::current_dir()?;
            let project_root = commands::add::wiring::find_project_root(&cwd)?;
            commands::build::handle_package_command(command, project_root)?;
        }
```

- [ ] **Step 3: Verify compilation**

Run: `cargo build --package silm`
Expected: compiles without errors

- [ ] **Step 4: Verify help text**

Run: `cargo run --package silm -- --help`
Expected: `build` and `package` commands appear in the help output

Run: `cargo run --package silm -- build --help`
Expected: `--platform`, `--release`, `--env-file` flags shown

Run: `cargo run --package silm -- package --help`
Expected: `--platform`, `--out-dir` flags shown

- [ ] **Step 5: Commit**

```bash
git add engine/cli/src/main.rs
git commit -m "feat(cli): register silm build and silm package commands"
```

---

### Task 9: Template updates (`basic.rs`)

**Files:**
- Modify: `engine/cli/src/templates/basic.rs` — add `[build]` to game.toml, add `client/index.html`, update `.gitignore`
- Test: `engine/cli/tests/build_tests.rs` or existing template tests

**Context:** When `silm new` creates a project, the generated `game.toml` should include a `[build]` section with default platforms. A `client/index.html` file for Trunk should be generated. The `.gitignore` should exclude `dist/` and `*.zip`.

**Reference:** Spec section "Template Updates (`silm new`)".

- [ ] **Step 1: Write failing tests**

Add to `engine/cli/tests/build_tests.rs`:
```rust
use silm::templates::{Template, BasicTemplate};

#[test]
fn test_template_game_toml_has_build_section() {
    let t = BasicTemplate::new("my-game".to_string(), false);
    let files = t.files();
    let game_toml = files.iter().find(|f| f.path == "game.toml").unwrap();
    assert!(game_toml.content.contains("[build]"), "game.toml missing [build]");
    assert!(game_toml.content.contains("platforms = [\"native\", \"wasm\"]"));
}

#[test]
fn test_template_game_toml_has_build_env() {
    let t = BasicTemplate::new("my-game".to_string(), false);
    let files = t.files();
    let game_toml = files.iter().find(|f| f.path == "game.toml").unwrap();
    assert!(game_toml.content.contains("[build.env]"));
    assert!(game_toml.content.contains("SERVER_ADDRESS"));
    assert!(game_toml.content.contains("SERVER_PORT"));
}

#[test]
fn test_template_has_client_index_html() {
    let t = BasicTemplate::new("my-game".to_string(), false);
    let files = t.files();
    let index = files.iter().find(|f| f.path == "client/index.html");
    assert!(index.is_some(), "client/index.html not generated");
    let content = &index.unwrap().content;
    assert!(content.contains("data-trunk"), "index.html should use Trunk data-trunk directive");
    assert!(content.contains("id=\"silmaril\""), "canvas should have id=silmaril");
}

#[test]
fn test_template_gitignore_has_dist() {
    let t = BasicTemplate::new("my-game".to_string(), false);
    let files = t.files();
    let gitignore = files.iter().find(|f| f.path == ".gitignore").unwrap();
    assert!(gitignore.content.contains("dist/"), ".gitignore missing dist/");
    assert!(gitignore.content.contains("*.zip"), ".gitignore missing *.zip");
}
```

Run: `cargo test --package silm --test build_tests`
Expected: FAIL

- [ ] **Step 2: Update game_toml() in basic.rs**

In `engine/cli/src/templates/basic.rs`, update the `game_toml()` method. Add after the `[dev]` section (before the closing `"#`):
```toml

[build]
platforms = ["native", "wasm"]

[build.env]
SERVER_ADDRESS = "ws://localhost:7777"
SERVER_PORT = "7777"
```

The full `game_toml` format string should end with:
```rust
[dev]
server_package = "{name}-server"
client_package = "{name}-client"
server_port = 7777
dev_server_port = 9999
dev_client_port = 9998

[build]
platforms = ["native", "wasm"]

[build.env]
SERVER_ADDRESS = "ws://localhost:7777"
SERVER_PORT = "7777"
"#,
```

- [ ] **Step 3: Add client_index_html() method**

Add a new method to `BasicTemplate`:
```rust
    fn client_index_html(&self) -> TemplateFile {
        let content = r#"<!DOCTYPE html>
<html>
  <head>
    <meta charset="utf-8"/>
    <title>My Game</title>
  </head>
  <body>
    <canvas id="silmaril"></canvas>
    <link data-trunk rel="rust" data-wasm-opt="z"/>
  </body>
</html>
"#;
        TemplateFile::new("client/index.html", content)
    }
```

Add `self.client_index_html()` to the `files()` method's `vec![]`, after `self.client_main_rs()`:
```rust
            self.client_main_rs(),
            self.client_index_html(),
```

- [ ] **Step 4: Update gitignore() method**

Add `dist/` and `*.zip` lines to the `.gitignore` content:
```rust
    fn gitignore(&self) -> TemplateFile {
        let content = r#"/target
/Cargo.lock
*.pdb
*.swp
*.swo
.DS_Store
.vscode/
.idea/
*.log
profiling-output.log
pgo-data/
*.prof
dist/
*.zip
"#;
        TemplateFile::new(".gitignore", content)
    }
```

- [ ] **Step 5: Run tests**

Run: `cargo test --package silm --test build_tests`
Expected: PASS

Also run existing template tests to confirm no regressions:
Run: `cargo test --package silm -- templates::basic::tests`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add engine/cli/src/templates/basic.rs engine/cli/tests/build_tests.rs
git commit -m "feat(cli): update silm new template with [build] section and client/index.html"
```

---

### Task 10: Integration tests with MockRunner

**Files:**
- Create: `engine/cli/tests/build_integration_tests.rs`
- Test: Full integration scenarios with mock runner

**Context:** These tests create a real filesystem project structure (using `TempDir`) and call the build logic with a `MockRunner` that captures commands instead of executing them. This validates the full orchestration without requiring cargo/cross/trunk to be installed.

**Note:** The `MockRunner` and `CapturedCommand` types should be defined in the test file (not in the library) since they're test-only constructs. The `build_platform` function needs to be made `pub` in `mod.rs` so integration tests can call it, or tests should go through the public `handle_build_command` with a way to inject the runner.

**Design decision:** To make the code testable without exposing `MockRunner` in the library, add a `build_platform_with_runner` public function that accepts a `&dyn BuildRunner`. The `handle_build_command` function calls this internally with `RealRunner`. Integration tests call `build_platform_with_runner` directly.

- [ ] **Step 1: Add public build_all_platforms function to mod.rs**

```rust
/// Build all resolved platforms with a provided runner. Used by tests.
pub fn build_all_platforms(
    runner: &dyn BuildRunner,
    project_root: &Path,
    game_toml_content: &str,
    platform_names: &[String],
    release: bool,
    env_file_path: Option<&Path>,
) -> Result<()> {
    let project_name = parse_project_name(game_toml_content)
        .ok_or_else(|| anyhow::anyhow!("game.toml is missing [project] name"))?;
    let (server_pkg, client_pkg) = parse_dev_section(game_toml_content, &project_name);

    let build_env_entries = env::parse_build_env(game_toml_content);
    let dotenv_content = fs::read_to_string(project_root.join(".env")).unwrap_or_default();
    let dotenv_entries = env::parse_env_file(&dotenv_content);
    let env_file_entries = if let Some(path) = env_file_path {
        let content = fs::read_to_string(path)?;
        env::parse_env_file(&content)
    } else {
        Vec::new()
    };
    let merged_env = env::merge_env(&build_env_entries, &dotenv_entries, &env_file_entries);

    for name in platform_names {
        let platform = platform_from_str(name)?;
        if platform.is_experimental() {
            tracing::warn!("[silm] platform '{}' is experimental", name);
        }
        let result = build_platform(runner, project_root, &merged_env, &platform, &server_pkg, &client_pkg, release);
        if let Err(e) = result {
            if platform.is_experimental() {
                tracing::warn!("[silm] experimental platform '{}' failed: {}", name, e);
                continue;
            }
            return Err(e);
        }
    }
    Ok(())
}
```

Update `handle_build_command` to call `build_all_platforms` internally with `RealRunner`.

- [ ] **Step 2: Write integration tests**

Create `engine/cli/tests/build_integration_tests.rs`:
```rust
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

use silm::commands::build::{
    BuildRunner, BuildTool, BuildKind, platform_from_str,
    build_all_platforms, parse_dev_section, parse_project_name,
};
use silm::commands::build::env::{parse_env_file, parse_build_env, merge_env, parse_build_section};
use silm::commands::build::package::{generate_dockerfile, create_zip, copy_assets, assemble_native_dist, assemble_server_dist};

#[derive(Debug, Clone)]
struct CapturedCommand {
    program: String,
    args: Vec<String>,
    env: HashMap<String, String>,
}

struct MockRunner {
    commands: RefCell<Vec<CapturedCommand>>,
}

impl MockRunner {
    fn new() -> Self {
        Self { commands: RefCell::new(Vec::new()) }
    }
    fn captured(&self) -> Vec<CapturedCommand> {
        self.commands.borrow().clone()
    }
}

impl BuildRunner for MockRunner {
    fn run_command(
        &self,
        program: &str,
        args: &[String],
        env: &HashMap<String, String>,
        _cwd: &Path,
    ) -> anyhow::Result<()> {
        self.commands.borrow_mut().push(CapturedCommand {
            program: program.to_string(),
            args: args.to_vec(),
            env: env.clone(),
        });
        Ok(())
    }
}

fn make_project(dir: &TempDir) -> String {
    let game_toml = r#"[project]
name = "test-game"
version = "0.1.0"

[dev]
server_package = "test-game-server"
client_package = "test-game-client"
server_port = 7777

[build]
platforms = ["native", "wasm"]

[build.env]
SERVER_ADDRESS = "ws://localhost:7777"
SERVER_PORT = "7777"

[modules]
"#;
    fs::write(dir.path().join("game.toml"), game_toml).unwrap();
    fs::create_dir_all(dir.path().join("client")).unwrap();
    fs::write(dir.path().join("client/index.html"), "<html></html>").unwrap();
    game_toml.to_string()
}

#[test]
fn test_build_native_captures_cargo() {
    let dir = TempDir::new().unwrap();
    let game_toml = make_project(&dir);
    let runner = MockRunner::new();

    build_all_platforms(
        &runner, dir.path(), &game_toml,
        &["native".to_string()], false, None,
    ).unwrap();

    let cmds = runner.captured();
    assert_eq!(cmds.len(), 2, "native should build server + client");
    assert_eq!(cmds[0].program, "cargo");
    assert!(cmds[0].args.contains(&"test-game-server".to_string()));
    assert!(cmds[0].args.contains(&"server".to_string()));
    assert_eq!(cmds[1].program, "cargo");
    assert!(cmds[1].args.contains(&"test-game-client".to_string()));
}

#[test]
fn test_build_native_release_flag() {
    let dir = TempDir::new().unwrap();
    let game_toml = make_project(&dir);
    let runner = MockRunner::new();

    build_all_platforms(
        &runner, dir.path(), &game_toml,
        &["native".to_string()], true, None,
    ).unwrap();

    let cmds = runner.captured();
    assert!(cmds[0].args.contains(&"--release".to_string()));
}

#[test]
fn test_build_wasm_captures_trunk() {
    let dir = TempDir::new().unwrap();
    let game_toml = make_project(&dir);
    let runner = MockRunner::new();

    // Note: this will fail at tool detection (trunk not found) in the real flow.
    // For this test, build_all_platforms should skip tool detection or we need
    // to test at a lower level. Let's test the wasm module directly.
    silm::commands::build::wasm::build_wasm(&runner, dir.path(), &HashMap::new(), false).unwrap();

    let cmds = runner.captured();
    assert_eq!(cmds[0].program, "trunk");
    assert!(cmds[0].args.contains(&"client/index.html".to_string()));
    assert!(cmds[0].args.contains(&"--dist".to_string()));
    assert!(cmds[0].args.contains(&"dist/wasm".to_string()));
}

#[test]
fn test_build_wasm_release() {
    let dir = TempDir::new().unwrap();
    let _game_toml = make_project(&dir);
    let runner = MockRunner::new();

    silm::commands::build::wasm::build_wasm(&runner, dir.path(), &HashMap::new(), true).unwrap();

    let cmds = runner.captured();
    assert!(cmds[0].args.contains(&"--release".to_string()));
}

#[test]
fn test_env_vars_in_subprocess() {
    let dir = TempDir::new().unwrap();
    let game_toml = make_project(&dir);
    fs::write(dir.path().join(".env"), "_SILM_TEST_BUILD_VAR=hello\n").unwrap();
    let runner = MockRunner::new();

    build_all_platforms(
        &runner, dir.path(), &game_toml,
        &["native".to_string()], false, None,
    ).unwrap();

    let cmds = runner.captured();
    assert_eq!(cmds[0].env.get("_SILM_TEST_BUILD_VAR").map(String::as_str), Some("hello"));
}

#[test]
fn test_build_env_from_game_toml() {
    let dir = TempDir::new().unwrap();
    let game_toml = make_project(&dir);
    // No .env file
    let runner = MockRunner::new();

    build_all_platforms(
        &runner, dir.path(), &game_toml,
        &["native".to_string()], false, None,
    ).unwrap();

    let cmds = runner.captured();
    // [build.env] vars should be present
    assert_eq!(cmds[0].env.get("SERVER_PORT").map(String::as_str), Some("7777"));
}

#[test]
fn test_env_file_overrides_dotenv() {
    let dir = TempDir::new().unwrap();
    let game_toml = make_project(&dir);
    fs::write(dir.path().join(".env"), "_SILM_TEST_PRIORITY=from_dotenv\n").unwrap();
    fs::write(dir.path().join("prod.env"), "_SILM_TEST_PRIORITY=from_env_file\n").unwrap();
    let runner = MockRunner::new();

    build_all_platforms(
        &runner, dir.path(), &game_toml,
        &["native".to_string()], false,
        Some(&dir.path().join("prod.env")),
    ).unwrap();

    let cmds = runner.captured();
    assert_eq!(cmds[0].env.get("_SILM_TEST_PRIORITY").map(String::as_str), Some("from_env_file"));
}

#[test]
fn test_missing_build_section_no_platform_flag_errors() {
    let dir = TempDir::new().unwrap();
    let game_toml = "[project]\nname = \"test\"\n[dev]\nserver_package = \"t-s\"\nclient_package = \"t-c\"\n";
    fs::write(dir.path().join("game.toml"), game_toml).unwrap();

    let platforms = parse_build_section(game_toml);
    assert!(platforms.is_none(), "should have no platforms when [build] is absent");
}

#[test]
fn test_unknown_platform_errors() {
    let result = platform_from_str("darwin");
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("darwin"));
    assert!(msg.contains("native"));
}

#[test]
fn test_package_assembles_dist() {
    let dir = TempDir::new().unwrap();
    let project = dir.path();

    // Create fake release binaries
    fs::create_dir_all(project.join("target/release")).unwrap();
    fs::write(project.join("target/release/server"), "fake server").unwrap();
    fs::write(project.join("target/release/client"), "fake client").unwrap();

    // Create assets
    fs::create_dir_all(project.join("assets")).unwrap();
    fs::write(project.join("assets/texture.png"), "fake png").unwrap();

    let dist_dir = assemble_native_dist(
        project, "native", None, true, true, false,
    ).unwrap();

    assert!(dist_dir.join("server").exists());
    assert!(dist_dir.join("client").exists());
    assert!(dist_dir.join("assets/texture.png").exists());
}

#[test]
fn test_package_server_has_dockerfile() {
    let dir = TempDir::new().unwrap();
    let project = dir.path();

    fs::create_dir_all(project.join("target/release")).unwrap();
    fs::write(project.join("target/release/server"), "fake server").unwrap();

    let env_entries = vec![("SERVER_PORT".to_string(), "7777".to_string())];
    let dist_dir = assemble_server_dist(project, &env_entries, false).unwrap();

    assert!(dist_dir.join("server").exists());
    assert!(dist_dir.join("Dockerfile").exists());
    let dockerfile = fs::read_to_string(dist_dir.join("Dockerfile")).unwrap();
    assert!(dockerfile.contains("ENV SERVER_PORT=7777"));
}

#[test]
fn test_package_creates_zip() {
    let dir = TempDir::new().unwrap();
    let src = dir.path().join("dist/native");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("server"), "binary").unwrap();

    let zip_path = dir.path().join("test-game-v0.1.0-native.zip");
    create_zip(&src, &zip_path).unwrap();

    assert!(zip_path.exists());
    assert!(fs::metadata(&zip_path).unwrap().len() > 0);
}

#[test]
fn test_assets_absent_no_error() {
    let dir = TempDir::new().unwrap();
    let project = dir.path();

    fs::create_dir_all(project.join("target/release")).unwrap();
    fs::write(project.join("target/release/server"), "fake").unwrap();

    // No assets/ directory — should succeed
    let dist = assemble_native_dist(project, "native", None, true, false, false).unwrap();
    assert!(dist.exists());
    assert!(!dist.join("assets").exists());
}
```

- [ ] **Step 3: Run integration tests**

Run: `cargo test --package silm --test build_integration_tests`
Expected: PASS

- [ ] **Step 4: Run all tests**

Run: `cargo test --package silm`
Expected: PASS (all existing tests + new tests)

- [ ] **Step 5: Commit**

```bash
git add engine/cli/tests/build_integration_tests.rs engine/cli/src/commands/build/mod.rs
git commit -m "feat(cli): add integration tests for silm build/package"
```

---

### Task 11: E2E test script

**Files:**
- Create: `scripts/e2e-tests/test-silm-build.sh`

**Context:** Real tool invocations on a real `silm new` project. Tests skip gracefully if tools (trunk, cross, Docker) are absent. Requires `silm` binary to be built first.

- [ ] **Step 1: Create E2E test script**

Create `scripts/e2e-tests/test-silm-build.sh`:
```bash
#!/bin/bash
set -euo pipefail

# E2E tests for silm build and silm package
# Skips tests gracefully when tools are not available

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
SILM="$REPO_ROOT/target/debug/silm"

# Build silm CLI first
echo "=== Building silm CLI ==="
cargo build --package silm

TMPDIR=$(mktemp -d)
trap "rm -rf $TMPDIR" EXIT

echo "=== Creating test project ==="
cd "$TMPDIR"
"$SILM" new test-game --local
cd test-game

PASS=0
SKIP=0
FAIL=0

run_test() {
    local name="$1"
    shift
    echo "--- TEST: $name ---"
    if "$@"; then
        echo "  PASS: $name"
        PASS=$((PASS + 1))
    else
        echo "  FAIL: $name"
        FAIL=$((FAIL + 1))
    fi
}

skip_test() {
    local name="$1"
    local reason="$2"
    echo "--- SKIP: $name ($reason) ---"
    SKIP=$((SKIP + 1))
}

# Test 1: silm build --platform native
run_test "build native debug" "$SILM" build --platform native
if [ -f target/debug/server ] || [ -f target/debug/server.exe ]; then
    run_test "native debug binary exists" true
else
    run_test "native debug binary exists" false
fi

# Test 2: silm build --platform native --release
run_test "build native release" "$SILM" build --platform native --release
if [ -f target/release/server ] || [ -f target/release/server.exe ]; then
    run_test "native release binary exists" true
else
    run_test "native release binary exists" false
fi

# Test 3: silm package --platform native
run_test "package native" "$SILM" package --platform native
if [ -d dist/native ] && ls test-game-v*-native.zip 1>/dev/null 2>&1; then
    run_test "native dist and zip exist" true
else
    run_test "native dist and zip exist" false
fi

# Test 4: silm package --platform server
run_test "package server" "$SILM" package --platform server
if [ -d dist/server ] && [ -f dist/server/Dockerfile ]; then
    run_test "server dist has Dockerfile" true
else
    run_test "server dist has Dockerfile" false
fi

# Test 5: WASM (skip if trunk not available)
if command -v trunk &>/dev/null; then
    run_test "build wasm" "$SILM" build --platform wasm
else
    skip_test "build wasm" "trunk not installed"
fi

# Test 6: Cross (skip if docker not running)
if docker info &>/dev/null 2>&1 && command -v cross &>/dev/null; then
    run_test "build linux-x86_64" "$SILM" build --platform linux-x86_64 --release
else
    skip_test "build linux-x86_64" "docker or cross not available"
fi

echo ""
echo "=== Results ==="
echo "  PASS: $PASS"
echo "  SKIP: $SKIP"
echo "  FAIL: $FAIL"

if [ $FAIL -gt 0 ]; then
    exit 1
fi
```

- [ ] **Step 2: Make executable**

```bash
chmod +x scripts/e2e-tests/test-silm-build.sh
```

- [ ] **Step 3: Commit**

```bash
git add scripts/e2e-tests/test-silm-build.sh
git commit -m "feat(cli): add E2E test script for silm build/package"
```

---

### Task 12: Final verification and cleanup

**Files:**
- All files from Tasks 1-11

- [ ] **Step 1: Run full test suite**

```bash
cargo test --package silm
```
Expected: All tests pass

- [ ] **Step 2: Run clippy**

```bash
cargo clippy --package silm -- -D warnings
```
Expected: No warnings

- [ ] **Step 3: Run formatter**

```bash
cargo fmt --package silm -- --check
```
Expected: No formatting issues

- [ ] **Step 4: Verify help text**

```bash
cargo run --package silm -- build --help
cargo run --package silm -- package --help
```
Expected: Clean help output with all flags documented

- [ ] **Step 5: Final commit (if any cleanup needed)**

```bash
git add -A
git commit -m "chore(cli): final cleanup for silm build/package"
```

---

## Summary

| Task | Description | Files | Tests |
|------|-------------|-------|-------|
| 1 | Env parsing + merge | `env.rs`, `mod.rs` (minimal) | 11 unit tests |
| 2 | Platform types + mapping | `mod.rs` | 12 unit tests |
| 3 | BuildRunner trait + tool detection + Dockerfile | `mod.rs`, `package.rs` (partial) | 4 unit tests |
| 4 | Native build (cargo/cross) | `native.rs` | 4 unit tests |
| 5 | WASM build (trunk) | `wasm.rs` | 2 unit tests |
| 6 | Build orchestration | `mod.rs` (handle_build_command) | 5 unit tests |
| 7 | Package command (dist, zip, Dockerfile) | `package.rs`, `Cargo.toml` | 4 unit tests |
| 8 | CLI registration | `main.rs` | Compile check |
| 9 | Template updates | `basic.rs` | 4 unit tests |
| 10 | Integration tests | `build_integration_tests.rs` | 12 integration tests |
| 11 | E2E test script | `test-silm-build.sh` | 6 E2E tests |
| 12 | Final verification | All | Full suite |
