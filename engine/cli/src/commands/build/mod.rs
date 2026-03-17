//! Build command types, platform mapping, and runner abstraction.
#![allow(dead_code)]

pub mod env;
pub mod package;

use anyhow::{bail, Result};
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

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

    pub fn tool(&self) -> BuildTool {
        self.tool
    }

    pub fn kind(&self) -> BuildKind {
        self.kind
    }

    pub fn experimental(&self) -> bool {
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
        _ => bail!("Docker is not running"),
    }
}
