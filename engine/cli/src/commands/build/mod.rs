//! Build command types, platform mapping, and runner abstraction.
// Tasks 4+ will use these types; remove when fully wired
#![allow(dead_code)]

pub mod env;
pub mod native;
pub mod package;
pub mod wasm;

use anyhow::{bail, Result};
use clap::Args;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{info, warn};

// ============================================================================
// Build Tool / Kind enums
// ============================================================================

/// Which build tool to use for a given platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildTool {
    Cargo,
    Cross,
    Trunk,
}

/// What binaries to produce.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildKind {
    ServerAndClient,
    ServerOnly,
    ClientOnly,
}

// ============================================================================
// Platform
// ============================================================================

/// A build target platform with all its resolved properties.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Platform {
    name: String,
    target_triple: String,
    tool: BuildTool,
    kind: BuildKind,
    experimental: bool,
    uses_exe_extension: bool,
}

impl Platform {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn target_triple(&self) -> &str {
        &self.target_triple
    }

    pub fn build_tool(&self) -> BuildTool {
        self.tool
    }

    pub fn build_kind(&self) -> BuildKind {
        self.kind
    }

    pub fn is_experimental(&self) -> bool {
        self.experimental
    }

    pub fn uses_exe_extension(&self) -> bool {
        self.uses_exe_extension
    }
}

/// All known platform name strings.
pub const KNOWN_PLATFORMS: &[&str] = &[
    "native",
    "server",
    "windows-x86_64",
    "linux-x86_64",
    "linux-arm64",
    "macos-x86_64",
    "macos-arm64",
    "wasm",
];

/// Returns the host target triple based on runtime arch and OS.
pub fn host_target_triple() -> String {
    let arch = std::env::consts::ARCH;
    let os = std::env::consts::OS;

    match (arch, os) {
        ("x86_64", "windows") => "x86_64-pc-windows-msvc".to_string(),
        ("x86_64", "linux") => "x86_64-unknown-linux-gnu".to_string(),
        ("x86_64", "macos") => "x86_64-apple-darwin".to_string(),
        ("aarch64", "linux") => "aarch64-unknown-linux-gnu".to_string(),
        ("aarch64", "macos") => "aarch64-apple-darwin".to_string(),
        _ => format!("{arch}-unknown-{os}"),
    }
}

fn is_windows_host() -> bool {
    std::env::consts::OS == "windows"
}

/// Resolve a platform name to a fully populated [`Platform`].
pub fn platform_from_str(name: &str) -> Result<Platform> {
    let on_windows = is_windows_host();

    match name {
        "native" => Ok(Platform {
            name: "native".into(),
            target_triple: host_target_triple(),
            tool: BuildTool::Cargo,
            kind: BuildKind::ServerAndClient,
            experimental: false,
            uses_exe_extension: on_windows,
        }),
        "server" => Ok(Platform {
            name: "server".into(),
            target_triple: host_target_triple(),
            tool: BuildTool::Cargo,
            kind: BuildKind::ServerOnly,
            experimental: false,
            uses_exe_extension: on_windows,
        }),
        "windows-x86_64" => Ok(Platform {
            name: "windows-x86_64".into(),
            target_triple: if on_windows {
                "x86_64-pc-windows-msvc".into()
            } else {
                "x86_64-pc-windows-gnu".into()
            },
            tool: if on_windows {
                BuildTool::Cargo
            } else {
                BuildTool::Cross
            },
            kind: BuildKind::ServerAndClient,
            experimental: false,
            uses_exe_extension: true,
        }),
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
        unknown => bail!("Unknown platform: '{unknown}'. Known platforms: {}", KNOWN_PLATFORMS.join(", ")),
    }
}

/// Returns the distribution directory name for a platform (same as platform name).
pub fn dist_dir_name(platform: &Platform) -> &str {
    &platform.name
}

// ============================================================================
// BuildRunner trait + RealRunner
// ============================================================================

/// Abstraction over running external commands, enabling test mocking.
pub trait BuildRunner {
    fn run_command(
        &self,
        program: &str,
        args: &[String],
        env: &HashMap<String, String>,
        cwd: &Path,
    ) -> Result<()>;
}

/// Real implementation that spawns OS processes.
pub struct RealRunner;

impl BuildRunner for RealRunner {
    fn run_command(
        &self,
        program: &str,
        args: &[String],
        env: &HashMap<String, String>,
        cwd: &Path,
    ) -> Result<()> {
        info!("[silm] running: {} {}", program, args.join(" "));
        let status = Command::new(program)
            .args(args)
            .envs(env)
            .current_dir(cwd)
            .status()?;

        if !status.success() {
            bail!(
                "{} exited with status {}",
                program,
                status.code().unwrap_or(-1)
            );
        }
        Ok(())
    }
}

/// Check that a CLI tool is available by running `tool --version`.
pub fn check_tool(tool: &str) -> Result<()> {
    let result = Command::new(tool).arg("--version").output();
    match result {
        Ok(output) if output.status.success() => Ok(()),
        _ => bail!("{tool} not found — install: cargo install {tool}"),
    }
}

/// Check that Docker is running by executing `docker info`.
pub fn check_docker() -> Result<()> {
    let result = Command::new("docker").arg("info").output();
    match result {
        Ok(output) if output.status.success() => Ok(()),
        _ => bail!("Docker is not running — start Docker Desktop, then retry"),
    }
}

// ============================================================================
// Clap command structs
// ============================================================================

/// Build the game for one or more target platforms.
#[derive(Args, Debug)]
pub struct BuildCommand {
    /// Target platform (e.g. native, wasm, linux-x86_64). Defaults to game.toml [build] platforms.
    #[arg(long)]
    pub platform: Option<String>,

    /// Build in release mode with optimizations.
    #[arg(long)]
    pub release: bool,

    /// Path to an additional .env file whose variables are passed to builds.
    #[arg(long)]
    pub env_file: Option<String>,
}

/// Package built artefacts for distribution.
#[derive(Args, Debug)]
pub struct PackageCommand {
    /// Target platform to package (defaults to game.toml [build] platforms).
    #[arg(long)]
    pub platform: Option<String>,

    /// Output directory for packaged artefacts.
    #[arg(long)]
    pub out_dir: Option<String>,
}

// ============================================================================
// game.toml parsing helpers
// ============================================================================

/// Parse `[dev]` section from game.toml, returning `(server_package, client_package)`.
///
/// Falls back to `"{project_name}-server"` / `"{project_name}-client"` when keys are absent.
pub fn parse_dev_section(game_toml_content: &str, project_name: &str) -> (String, String) {
    let table: toml::Value = match game_toml_content.parse() {
        Ok(v) => v,
        Err(_) => {
            return (
                format!("{project_name}-server"),
                format!("{project_name}-client"),
            )
        }
    };

    let dev = table.get("dev");

    let server_package = dev
        .and_then(|d| d.get("server_package"))
        .and_then(|v| v.as_str())
        .map(String::from)
        .unwrap_or_else(|| format!("{project_name}-server"));

    let client_package = dev
        .and_then(|d| d.get("client_package"))
        .and_then(|v| v.as_str())
        .map(String::from)
        .unwrap_or_else(|| format!("{project_name}-client"));

    (server_package, client_package)
}

/// Parse the project name from `[project] name` in game.toml.
pub fn parse_project_name(game_toml_content: &str) -> Option<String> {
    let table: toml::Value = game_toml_content.parse().ok()?;
    table
        .get("project")
        .and_then(|p| p.get("name"))
        .and_then(|v| v.as_str())
        .map(String::from)
}

/// Parse the project version from `[project] version` in game.toml.
///
/// Returns `"0.0.0"` if the field is absent or the file cannot be parsed.
pub fn parse_project_version(game_toml_content: &str) -> String {
    let table: toml::Value = match game_toml_content.parse() {
        Ok(v) => v,
        Err(_) => return "0.0.0".into(),
    };

    table
        .get("project")
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .map(String::from)
        .unwrap_or_else(|| "0.0.0".into())
}

// ============================================================================
// Build orchestration
// ============================================================================

/// Run pre-flight checks for a platform (tool availability, required files/env).
fn preflight_checks(project_root: &Path, platform: &Platform) -> Result<()> {
    match platform.build_tool() {
        BuildTool::Trunk => {
            check_tool("trunk")?;
            if !project_root.join("client/index.html").exists() {
                bail!("WASM build requires client/index.html — not found");
            }
        }
        BuildTool::Cross => {
            check_tool("cross")?;
            check_docker()?;
            if platform.name().starts_with("macos-") {
                if std::env::var("MACOS_SDK_URL").is_err() {
                    bail!("macOS cross-build requires MACOS_SDK_URL");
                }
            }
        }
        BuildTool::Cargo => {
            // cargo is assumed available (we're running from cargo)
        }
    }
    Ok(())
}

/// Build a single platform, handling tool checks and dispatch.
#[allow(clippy::too_many_arguments)]
fn build_platform(
    runner: &dyn BuildRunner,
    project_root: &Path,
    env: &HashMap<String, String>,
    server_package: &str,
    client_package: &str,
    platform: &Platform,
    release: bool,
    skip_preflight: bool,
) -> Result<()> {
    info!(
        platform = %platform.name(),
        tool = ?platform.build_tool(),
        "Building platform"
    );

    if !skip_preflight {
        preflight_checks(project_root, platform)?;
    }

    match platform.build_tool() {
        BuildTool::Trunk => {
            wasm::build_wasm(runner, project_root, env, release)
        }
        tool => {
            let target = if tool == BuildTool::Cross || platform.target_triple() != host_target_triple() {
                Some(platform.target_triple())
            } else {
                None
            };

            native::build_native(
                runner,
                project_root,
                env,
                server_package,
                client_package,
                tool,
                target,
                platform.build_kind(),
                release,
            )
        }
    }
}

/// Build all specified platforms.
///
/// This is the main testable orchestration function. It:
/// 1. Parses project name, dev section, env vars from game.toml
/// 2. For each platform: resolves via [`platform_from_str`], runs pre-flight checks,
///    dispatches to [`native::build_native`] or [`wasm::build_wasm`]
/// 3. macOS (experimental) failures are non-fatal (warn + continue), other failures are fatal
///
/// Set `skip_preflight` to `true` in tests to avoid real tool-availability checks.
#[allow(clippy::too_many_arguments)]
pub fn build_all_platforms(
    runner: &dyn BuildRunner,
    project_root: &Path,
    game_toml_content: &str,
    platform_names: &[String],
    release: bool,
    env_file_path: Option<&Path>,
    skip_preflight: bool,
) -> Result<()> {
    let project_name = parse_project_name(game_toml_content)
        .ok_or_else(|| anyhow::anyhow!("game.toml is missing [project] name"))?;

    let (server_package, client_package) = parse_dev_section(game_toml_content, &project_name);

    // Build env from game.toml [build.env]
    let build_env = env::parse_build_env(game_toml_content);

    // Load .env file if present
    let dotenv_path = project_root.join(".env");
    let dotenv = if dotenv_path.is_file() {
        let content = std::fs::read_to_string(&dotenv_path).unwrap_or_default();
        env::parse_env_file(&content)
    } else {
        Vec::new()
    };

    // Load explicit --env-file if provided
    let env_file_entries = if let Some(path) = env_file_path {
        let content = std::fs::read_to_string(path)?;
        env::parse_env_file(&content)
    } else {
        Vec::new()
    };

    let merged_env = env::merge_env(&build_env, &dotenv, &env_file_entries);

    for name in platform_names {
        let platform = platform_from_str(name)?;

        let result = build_platform(
            runner,
            project_root,
            &merged_env,
            &server_package,
            &client_package,
            &platform,
            release,
            skip_preflight,
        );

        if let Err(e) = result {
            if platform.is_experimental() {
                warn!(
                    platform = %platform.name(),
                    error = %e,
                    "Experimental platform build failed (non-fatal)"
                );
            } else {
                return Err(e);
            }
        }
    }

    Ok(())
}

/// Entry point for `silm build` called from the CLI.
///
/// Reads game.toml, resolves platforms, and delegates to [`build_all_platforms`].
pub fn handle_build_command(cmd: BuildCommand, project_root: PathBuf) -> Result<()> {
    let game_toml_path = project_root.join("game.toml");
    let game_toml_content = std::fs::read_to_string(&game_toml_path)
        .map_err(|e| anyhow::anyhow!("Failed to read game.toml: {e}"))?;

    let platform_names: Vec<String> = if let Some(ref p) = cmd.platform {
        vec![p.clone()]
    } else {
        env::parse_build_section(&game_toml_content)
            .ok_or_else(|| anyhow::anyhow!(
                "no platforms specified — add [build] platforms = [...] to game.toml, or use --platform <name>"
            ))?
    };

    let env_file_path = cmd.env_file.as_ref().map(PathBuf::from);

    build_all_platforms(
        &RealRunner,
        &project_root,
        &game_toml_content,
        &platform_names,
        cmd.release,
        env_file_path.as_deref(),
        false,
    )
}
