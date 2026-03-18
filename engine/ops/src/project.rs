//! Project creation, discovery, and configuration.
//!
//! Extracted from `engine/cli/src/commands/new.rs`, `templates/basic.rs`, and `add/wiring.rs`.

use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Target enum
// ---------------------------------------------------------------------------

/// Which crate to target within a Silmaril game project.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Shared,
    Server,
    Client,
}

impl Target {
    /// Subdirectory name relative to project root.
    pub fn crate_subdir(&self) -> &'static str {
        match self {
            Target::Shared => "shared",
            Target::Server => "server",
            Target::Client => "client",
        }
    }

    /// Entry point file within `src/` (`lib.rs` for shared, `main.rs` for server/client).
    pub fn entry_file(&self) -> &'static str {
        match self {
            Target::Shared => "lib.rs",
            Target::Server | Target::Client => "main.rs",
        }
    }
}

// ---------------------------------------------------------------------------
// Project discovery
// ---------------------------------------------------------------------------

/// Walk up from `start` to find `game.toml`. Returns the directory containing it.
pub fn find_project_root(start: &Path) -> Result<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        if current.join("game.toml").exists() {
            return Ok(current);
        }
        if !current.pop() {
            bail!("no game.toml found — run this command from inside a silmaril project");
        }
    }
}

/// Load the raw `game.toml` content from the project root.
pub fn load_game_toml(project_root: &Path) -> Result<String> {
    let path = project_root.join("game.toml");
    fs::read_to_string(&path)
        .with_context(|| format!("Failed to read game.toml at {:?}", path))
}

/// Resolve the crate directory, error if it doesn't exist.
pub fn crate_dir(project_root: &Path, target: Target) -> Result<PathBuf> {
    let dir = project_root.join(target.crate_subdir());
    if !dir.is_dir() {
        bail!(
            "target crate '{}/' not found — is this project set up correctly?",
            target.crate_subdir()
        );
    }
    Ok(dir)
}

/// Resolve domain module file path: `<crate>/src/<domain>/mod.rs`
pub fn domain_file(crate_root: &Path, domain: &str) -> PathBuf {
    crate_root.join("src").join(domain).join("mod.rs")
}

/// Resolve wiring target: `<crate>/src/lib.rs` or `<crate>/src/main.rs`
pub fn wiring_target(crate_root: &Path, target: Target) -> PathBuf {
    crate_root.join("src").join(target.entry_file())
}

// ---------------------------------------------------------------------------
// File utilities
// ---------------------------------------------------------------------------

/// Write `content` to `path` atomically (temp file → rename).
/// Creates parent directories if needed.
pub fn atomic_write(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, content)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

/// Append `content` to domain file atomically.
/// If file doesn't exist, creates it. Reads original into memory for rollback.
/// Returns the original content (`None` if file was new) for rollback on wiring failure.
pub fn append_to_domain_file(file: &Path, content: &str) -> Result<Option<String>> {
    let original = if file.exists() { Some(fs::read_to_string(file)?) } else { None };

    let new_content = match &original {
        Some(existing) => format!("{}\n{}", existing, content),
        None => content.to_string(),
    };

    atomic_write(file, &new_content)?;
    Ok(original)
}

/// Add `pub mod <domain>;` to the wiring target file if not already present.
/// Uses atomic write. Returns the original content for rollback.
pub fn wire_module_declaration(target_file: &Path, domain: &str) -> Result<String> {
    let original = if target_file.exists() {
        fs::read_to_string(target_file)?
    } else {
        String::new()
    };

    let declaration = format!("pub mod {};", domain);
    if original.contains(&declaration) {
        return Ok(original); // already wired, nothing to do
    }

    let new_content = format!("{}\n{}\n", original.trim_end(), declaration);
    atomic_write(target_file, &new_content)?;
    Ok(original)
}

// ---------------------------------------------------------------------------
// Duplicate detection
// ---------------------------------------------------------------------------

/// Check if `pub struct <Name>` (followed by `{` with optional whitespace) exists in file.
pub fn has_duplicate_component(file: &Path, name: &str) -> Result<bool> {
    if !file.exists() {
        return Ok(false);
    }
    let content = fs::read_to_string(file)?;
    let pattern = format!("pub struct {}", name);
    Ok(content.lines().any(|line| {
        if let Some(rest) = line.trim_start().strip_prefix(&pattern) {
            rest.trim_start().starts_with('{')
        } else {
            false
        }
    }))
}

/// Check if `pub fn <name>_system(` exists in file.
pub fn has_duplicate_system(file: &Path, name: &str) -> Result<bool> {
    if !file.exists() {
        return Ok(false);
    }
    let content = fs::read_to_string(file)?;
    let pattern = format!("pub fn {}_system(", name);
    Ok(content.contains(&pattern))
}

// ---------------------------------------------------------------------------
// Rollback helpers
// ---------------------------------------------------------------------------

/// Rollback domain file: restore original content, or delete if it was newly created.
pub fn rollback_domain_file(file: &Path, original: Option<String>) -> Result<()> {
    match original {
        Some(content) => atomic_write(file, &content),
        None => {
            if file.exists() {
                fs::remove_file(file)?;
            }
            Ok(())
        }
    }
}

/// Rollback wiring target to original content.
#[allow(dead_code)]
pub fn rollback_wiring_target(file: &Path, original: &str) -> Result<()> {
    atomic_write(file, original)
}

// ---------------------------------------------------------------------------
// Project creation
// ---------------------------------------------------------------------------

/// Validate a project name for use as a Rust crate name.
pub fn validate_project_name(name: &str) -> Result<()> {
    if name.is_empty() {
        bail!("Project name cannot be empty");
    }

    if !name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        bail!(
            "Project name can only contain alphanumeric characters, dashes, and underscores.\nGot: '{}'",
            name
        );
    }

    if name.chars().next().unwrap().is_numeric() {
        bail!("Project name cannot start with a number");
    }

    let rust_keywords = [
        "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn",
        "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
        "return", "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe",
        "use", "where", "while", "async", "await", "dyn",
    ];

    if rust_keywords.contains(&name) {
        bail!("Project name '{}' is a reserved Rust keyword", name);
    }

    Ok(())
}

/// Template file produced by project creation.
pub struct TemplateFile {
    pub path: String,
    pub content: String,
}

impl TemplateFile {
    pub fn new(path: impl Into<String>, content: impl Into<String>) -> Self {
        Self { path: path.into(), content: content.into() }
    }
}

/// Trait for project templates.
pub trait Template {
    fn name(&self) -> &str;
    fn files(&self) -> Vec<TemplateFile>;
}

/// The basic starter template.
pub struct BasicTemplate {
    pub project_name: String,
    pub use_local: bool,
}

impl BasicTemplate {
    pub fn new(project_name: String, use_local: bool) -> Self {
        Self { project_name, use_local }
    }
}

impl Template for BasicTemplate {
    fn name(&self) -> &str {
        "basic"
    }

    fn files(&self) -> Vec<TemplateFile> {
        vec![
            self.game_toml(),
            self.cargo_toml(),
            self.gitignore(),
            self.readme(),
            self.cargo_config(),
            // Shared crate
            self.shared_cargo_toml(),
            self.shared_lib_rs(),
            // Server crate
            self.server_cargo_toml(),
            self.server_main_rs(),
            // Client crate
            self.client_cargo_toml(),
            self.client_main_rs(),
            self.client_index_html(),
            // Config files
            self.server_config_ron(),
            self.client_config_ron(),
            // xtask
            self.xtask_cargo_toml(),
            self.xtask_main_rs(),
            self.xtask_utils_rs(),
        ]
    }
}

impl BasicTemplate {
    fn game_toml(&self) -> TemplateFile {
        let content = format!(
            r#"[project]
name = "{name}"
version = "0.1.0"
description = "A game built with Silmaril"

[dependencies]
# Engine dependencies are specified in each crate's Cargo.toml

[modules]
# Game modules added via `silm add module` appear here
# Example:
# combat = {{ source = "registry", version = "^1.0.0", target = "shared" }}

[features]
client = []
server = []
networking = []

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
            name = self.project_name
        );
        TemplateFile::new("game.toml", content)
    }

    fn cargo_toml(&self) -> TemplateFile {
        let content = format!(
            r#"[workspace]
resolver = "2"
members = [
    "xtask",
    "shared",
    "server",
    "client",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
authors = ["{name} Team"]
license = "Apache-2.0"

[workspace.dependencies]
# Silmaril engine dependencies
{dependencies}

# Common dependencies
serde = {{ version = "1.0", features = ["derive"] }}
tokio = {{ version = "1.35", features = ["full"] }}
tracing = "0.1"
tracing-subscriber = {{ version = "0.3", features = ["json", "env-filter"] }}
anyhow = "1.0"

[profile.dev]
opt-level = 1  # Faster debug builds

[profile.dev.package."*"]
opt-level = 3  # Optimize dependencies even in debug

[profile.release]
lto = "thin"
codegen-units = 16
opt-level = 3

[profile.release-server]
inherits = "release"
lto = "fat"
codegen-units = 1
opt-level = "z"  # Optimize for size
strip = true
"#,
            name = self.project_name,
            dependencies = if self.use_local {
                r#"engine-core = { path = "../silmaril/engine/core" }
engine-networking = { path = "../silmaril/engine/networking" }
serde_json = "1.0""#
            } else {
                r#"engine-core = "0.1"
engine-networking = "0.1"
serde_json = "1.0""#
            }
        );
        TemplateFile::new("Cargo.toml", content)
    }

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

    fn readme(&self) -> TemplateFile {
        let content = format!(
            r#"# {name}

A game built with Silmaril game engine.

## Quick Start

### Prerequisites

- Rust 1.75+ ([rustup.rs](https://rustup.rs/))
- Vulkan SDK ([vulkan.lunarg.com](https://vulkan.lunarg.com/))

### Build & Run

```bash
# Build game
cargo xtask build both

# Run server
cargo xtask dev server

# Run client (in another terminal)
cargo xtask dev client

# Or run both together
cargo xtask dev full
```

### Development Commands

```bash
# Format code
cargo xtask fmt

# Run lints
cargo xtask clippy

# Run tests
cargo xtask test all

# Run checks (fmt + clippy + test)
cargo xtask check

# Build for release
cargo xtask build release

# Package for distribution
cargo xtask package
```

## Project Structure

```
{name}/
├── game.toml              # Game metadata
├── Cargo.toml             # Workspace definition
├── shared/                # Shared game logic (runs on both client & server)
│   ├── src/
│   │   ├── components.rs  # Game components (Health, Transform, etc.)
│   │   └── systems.rs     # Game systems (movement, combat, etc.)
├── server/                # Server-only logic
│   └── src/main.rs
├── client/                # Client-only logic (rendering, input)
│   └── src/main.rs
├── assets/                # Game assets (models, textures, audio)
├── config/                # Configuration files
└── xtask/                 # Build automation tasks
```

## Adding Components & Systems

```bash
# Add a component
silm add component Health --shared --fields "current:f32,max:f32"

# Add a system
silm add system health_regen --shared --query "Health,RegenerationRate"

# Add a module (reusable game logic)
silm add module combat
```

## Documentation

- [Silmaril Documentation](https://github.com/your-org/silmaril)
- [game.toml](game.toml) - Game configuration
- [Cargo.toml](Cargo.toml) - Dependencies

## License

Licensed under Apache-2.0
"#,
            name = self.project_name
        );
        TemplateFile::new("README.md", content)
    }

    fn cargo_config(&self) -> TemplateFile {
        let content = r#"[alias]
xtask = "run --package xtask --"

[build]
# Uncomment for native CPU optimizations (10-30% faster, but less portable)
# rustflags = ["-C", "target-cpu=native"]
"#;
        TemplateFile::new(".cargo/config.toml", content)
    }

    fn shared_cargo_toml(&self) -> TemplateFile {
        let content = format!(
            r#"[package]
name = "{name}-shared"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true

[dependencies]
engine-core = {{ workspace = true }}
serde = {{ workspace = true }}
tracing = {{ workspace = true }}

[dev-dependencies]
tokio-test = "0.4"
serde_json = "1.0"
"#,
            name = self.project_name
        );
        TemplateFile::new("shared/Cargo.toml", content)
    }

    fn shared_lib_rs(&self) -> TemplateFile {
        let content = String::from(
            r#"//! Shared game logic — components, systems, and types used by both server and client.
//!
//! Add new domains with: silm add component <Name> --shared --domain <domain>
"#,
        );
        TemplateFile::new("shared/src/lib.rs", content)
    }

    fn server_cargo_toml(&self) -> TemplateFile {
        let content = format!(
            r#"[package]
name = "{name}-server"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true

[[bin]]
name = "server"
path = "src/main.rs"

[features]
dev = ["engine-dev-tools-hot-reload/dev"]

[dependencies]
{name}-shared = {{ path = "../shared" }}
engine-core = {{ workspace = true }}
engine-networking = {{ workspace = true }}
tokio = {{ workspace = true }}
tracing = {{ workspace = true }}
tracing-subscriber = {{ workspace = true }}
anyhow = {{ workspace = true }}
engine-dev-tools-hot-reload = {{ path = "../../silmaril/engine/dev-tools/hot-reload", optional = true }}
"#,
            name = self.project_name
        );
        TemplateFile::new("server/Cargo.toml", content)
    }

    fn server_main_rs(&self) -> TemplateFile {
        let content = format!(
            r#"//! {name} - Server Binary
//!
//! Server-authoritative game logic

use tracing::{{info, Level}};
use tracing_subscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {{
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("{name} server starting...");

    // Start dev reload server (only when dev feature is enabled)
    #[cfg(feature = "dev")]
    engine_dev_tools_hot_reload::server::DevReloadServer::start(None).await;

    // TODO: Initialize game server
    // - Load server config
    // - Initialize ECS world
    // - Start network server
    // - Run game loop (60 TPS)

    info!("{name} server running on 0.0.0.0:7777");

    // Keep server running
    std::future::pending::<()>().await;

    Ok(())
}}
"#,
            name = self.project_name
        );
        TemplateFile::new("server/src/main.rs", content)
    }

    fn client_cargo_toml(&self) -> TemplateFile {
        let content = format!(
            r#"[package]
name = "{name}-client"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true

[[bin]]
name = "client"
path = "src/main.rs"

[features]
dev = ["engine-dev-tools-hot-reload/dev"]

[dependencies]
{name}-shared = {{ path = "../shared" }}
engine-core = {{ workspace = true }}
engine-networking = {{ workspace = true }}
tokio = {{ workspace = true }}
tracing = {{ workspace = true }}
tracing-subscriber = {{ workspace = true }}
anyhow = {{ workspace = true }}
engine-dev-tools-hot-reload = {{ path = "../../silmaril/engine/dev-tools/hot-reload", optional = true }}
"#,
            name = self.project_name
        );
        TemplateFile::new("client/Cargo.toml", content)
    }

    fn client_main_rs(&self) -> TemplateFile {
        let content = format!(
            r#"//! {name} - Client Binary
//!
//! Client-side rendering, input handling, and prediction

use tracing::{{info, Level}};
use tracing_subscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {{
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("{name} client starting...");

    // Start dev reload server (only when dev feature is enabled)
    #[cfg(feature = "dev")]
    engine_dev_tools_hot_reload::server::DevReloadServer::start(None).await;

    // TODO: Initialize game client
    // - Load client config
    // - Initialize Vulkan renderer
    // - Connect to server
    // - Run game loop (60 FPS target)

    info!("{name} client running");

    // Keep client running
    std::future::pending::<()>().await;

    Ok(())
}}
"#,
            name = self.project_name
        );
        TemplateFile::new("client/src/main.rs", content)
    }

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

    fn server_config_ron(&self) -> TemplateFile {
        let content = r#"// Server configuration
(
    network: (
        bind_address: "0.0.0.0:7777",
        max_players: 100,
        tick_rate: 60,
    ),
    gameplay: (
        gravity: -9.81,
        max_velocity: 50.0,
    ),
)
"#;
        TemplateFile::new("config/server.ron", content)
    }

    fn client_config_ron(&self) -> TemplateFile {
        let content = r#"// Client configuration
(
    network: (
        server_address: "127.0.0.1:7777",
        connect_timeout_ms: 5000,
    ),
    graphics: (
        resolution: (1920, 1080),
        vsync: true,
        fullscreen: false,
    ),
)
"#;
        TemplateFile::new("config/client.ron", content)
    }

    fn xtask_cargo_toml(&self) -> TemplateFile {
        let content = r#"[package]
name = "xtask"
version = "0.1.0"
edition = "2021"

[dependencies]
clap = { version = "4.4", features = ["derive"] }
anyhow = "1.0"
colored = "2.0"
"#;
        TemplateFile::new("xtask/Cargo.toml", content)
    }

    fn xtask_main_rs(&self) -> TemplateFile {
        let content = r#"//! Game-specific build automation tasks

use clap::{Parser, Subcommand};
use colored::Colorize;

mod utils;
use utils::run_cargo;

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Game build automation tasks")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build game binaries
    Build {
        /// Target to build: client, server, or both
        #[arg(default_value = "both")]
        target: String,
    },
    /// Run development environment
    Dev {
        /// What to run: client, server, or full
        #[arg(default_value = "full")]
        target: String,
    },
    /// Run tests
    Test {
        /// Test suite: all, shared, client, server
        #[arg(default_value = "all")]
        suite: String,
    },
    /// Format code
    Fmt,
    /// Run clippy lints
    Clippy,
    /// Run all checks (fmt + clippy + test)
    Check,
    /// Package game for distribution
    Package,
    /// Clean build artifacts
    Clean,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Build { target } => build(&target)?,
        Commands::Dev { target } => dev(&target)?,
        Commands::Test { suite } => test(&suite)?,
        Commands::Fmt => fmt()?,
        Commands::Clippy => clippy()?,
        Commands::Check => check()?,
        Commands::Package => package()?,
        Commands::Clean => clean()?,
    }

    Ok(())
}

fn build(target: &str) -> anyhow::Result<()> {
    println!("{}", "🔨 Building...".bright_blue().bold());

    match target {
        "client" => run_cargo(&["build", "--bin", "client"])?,
        "server" => run_cargo(&["build", "--bin", "server"])?,
        "both" => {
            run_cargo(&["build", "--bin", "client"])?;
            run_cargo(&["build", "--bin", "server"])?;
        }
        _ => anyhow::bail!("Unknown target: {}", target),
    }

    println!("{}", "✅ Build complete!".bright_green().bold());
    Ok(())
}

fn dev(target: &str) -> anyhow::Result<()> {
    println!("{}", "🚀 Starting dev environment...".bright_blue().bold());

    match target {
        "client" => run_cargo(&["run", "--bin", "client"])?,
        "server" => run_cargo(&["run", "--bin", "server"])?,
        "full" => {
            println!("{}", "Note: Run server and client in separate terminals".yellow());
            println!("  Terminal 1: cargo xtask dev server");
            println!("  Terminal 2: cargo xtask dev client");
            anyhow::bail!("Cannot run both in same terminal");
        }
        _ => anyhow::bail!("Unknown target: {}", target),
    }

    Ok(())
}

fn test(suite: &str) -> anyhow::Result<()> {
    println!("{}", "🧪 Running tests...".bright_blue().bold());

    match suite {
        "all" => run_cargo(&["test", "--all"])?,
        "shared" => run_cargo(&["test", "--package", "*-shared"])?,
        "client" => run_cargo(&["test", "--package", "*-client"])?,
        "server" => run_cargo(&["test", "--package", "*-server"])?,
        _ => anyhow::bail!("Unknown test suite: {}", suite),
    }

    println!("{}", "✅ Tests passed!".bright_green().bold());
    Ok(())
}

fn fmt() -> anyhow::Result<()> {
    println!("{}", "📝 Formatting code...".bright_blue().bold());
    run_cargo(&["fmt", "--all"])?;
    println!("{}", "✅ Code formatted!".bright_green().bold());
    Ok(())
}

fn clippy() -> anyhow::Result<()> {
    println!("{}", "🔍 Running clippy...".bright_blue().bold());
    run_cargo(&["clippy", "--all-targets", "--all-features", "--", "-D", "warnings"])?;
    println!("{}", "✅ Clippy passed!".bright_green().bold());
    Ok(())
}

fn check() -> anyhow::Result<()> {
    println!("{}", "✔️  Running all checks...".bright_blue().bold());
    fmt()?;
    clippy()?;
    test("all")?;
    println!("{}", "✅ All checks passed!".bright_green().bold());
    Ok(())
}

fn package() -> anyhow::Result<()> {
    println!("{}", "📦 Packaging game...".bright_blue().bold());

    // Build release binaries
    run_cargo(&["build", "--release", "--bin", "client"])?;
    run_cargo(&["build", "--release", "--bin", "server"])?;

    // TODO: Package assets
    // TODO: Create distribution archive

    println!("{}", "✅ Package complete! Check target/release/".bright_green().bold());
    Ok(())
}

fn clean() -> anyhow::Result<()> {
    println!("{}", "🧹 Cleaning...".bright_blue().bold());
    run_cargo(&["clean"])?;
    println!("{}", "✅ Clean complete!".bright_green().bold());
    Ok(())
}
"#;
        TemplateFile::new("xtask/src/main.rs", content)
    }

    fn xtask_utils_rs(&self) -> TemplateFile {
        let content = r#"use anyhow::Result;
use std::process::Command;

pub fn run_cargo(args: &[&str]) -> Result<()> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

    let status = Command::new(&cargo)
        .args(args)
        .status()?;

    if !status.success() {
        anyhow::bail!("cargo command failed");
    }

    Ok(())
}
"#;
        TemplateFile::new("xtask/src/utils.rs", content)
    }
}

/// Create a new Silmaril game project on disk.
///
/// This writes all template files and creates empty asset directories.
/// It does **not** produce any terminal output — the caller (CLI or editor)
/// is responsible for progress reporting.
pub fn create_project(name: &str, template: &str, use_local: bool) -> Result<()> {
    validate_project_name(name)?;

    let project_path = PathBuf::from(name);
    if project_path.exists() {
        bail!(
            "Directory '{}' already exists! Please choose a different name or remove the existing directory.",
            name
        );
    }

    let template_impl: Box<dyn Template> = match template {
        "basic" => Box::new(BasicTemplate::new(name.to_string(), use_local)),
        "mmo" => bail!("MMO template not yet implemented. Use 'basic' template for now."),
        "moba" => bail!("MOBA template not yet implemented. Use 'basic' template for now."),
        _ => bail!("Unknown template: '{}'. Available templates: basic, mmo, moba", template),
    };

    fs::create_dir(&project_path)
        .with_context(|| format!("Failed to create project directory: {}", name))?;

    for file in template_impl.files().iter() {
        let file_path = project_path.join(&file.path);

        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {:?}", parent))?;
        }

        fs::write(&file_path, &file.content)
            .with_context(|| format!("Failed to write file: {:?}", file_path))?;
    }

    let empty_dirs = ["assets", "assets/models", "assets/textures", "assets/audio"];
    for dir in &empty_dirs {
        let dir_path = project_path.join(dir);
        fs::create_dir_all(&dir_path)
            .with_context(|| format!("Failed to create directory: {}", dir))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_project(tmp: &TempDir) -> PathBuf {
        let root = tmp.path().to_path_buf();
        fs::write(root.join("game.toml"), "[game]\nname = \"test\"").unwrap();
        fs::create_dir_all(root.join("shared/src")).unwrap();
        fs::write(root.join("shared/src/lib.rs"), "").unwrap();
        root
    }

    #[test]
    fn test_find_project_root_from_same_dir() {
        let tmp = TempDir::new().unwrap();
        let root = make_project(&tmp);
        let found = find_project_root(&root).unwrap();
        assert_eq!(found, root);
    }

    #[test]
    fn test_find_project_root_from_subdir() {
        let tmp = TempDir::new().unwrap();
        let root = make_project(&tmp);
        let subdir = root.join("shared/src/health");
        fs::create_dir_all(&subdir).unwrap();
        let found = find_project_root(&subdir).unwrap();
        assert_eq!(found, root);
    }

    #[test]
    fn test_find_project_root_not_found() {
        let tmp = TempDir::new().unwrap();
        let result = find_project_root(tmp.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no game.toml found"));
    }

    #[test]
    fn test_crate_dir_ok() {
        let tmp = TempDir::new().unwrap();
        let root = make_project(&tmp);
        let dir = crate_dir(&root, Target::Shared).unwrap();
        assert_eq!(dir, root.join("shared"));
    }

    #[test]
    fn test_crate_dir_missing() {
        let tmp = TempDir::new().unwrap();
        let root = make_project(&tmp);
        let result = crate_dir(&root, Target::Server);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("server/"));
    }

    #[test]
    fn test_validate_project_name_valid() {
        assert!(validate_project_name("my-game").is_ok());
        assert!(validate_project_name("my_game").is_ok());
        assert!(validate_project_name("game123").is_ok());
    }

    #[test]
    fn test_validate_project_name_invalid() {
        assert!(validate_project_name("").is_err());
        assert!(validate_project_name("123game").is_err());
        assert!(validate_project_name("my game").is_err());
        assert!(validate_project_name("fn").is_err());
    }
}
