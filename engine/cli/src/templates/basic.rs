use super::{Template, TemplateFile};

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
        let files = vec![
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
            // Config files
            self.server_config_ron(),
            self.client_config_ron(),
            // xtask
            self.xtask_cargo_toml(),
            self.xtask_main_rs(),
            self.xtask_utils_rs(),
        ];

        files
    }
}

impl BasicTemplate {
    fn game_toml(&self) -> TemplateFile {
        let content = format!(
            r#"[game]
name = "{name}"
version = "0.1.0"
description = "A game built with Silmaril"

[dependencies]
# Engine dependencies are specified in each crate's Cargo.toml

[modules]
# Game modules added via `silm add module` appear here
# Example:
# combat = {{ source = "git", version = "0.1.0" }}

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
                r#"silmaril-core = { path = "../silmaril/engine/core" }
silmaril-ecs = { path = "../silmaril/engine/core" }
silmaril-math = { path = "../silmaril/engine/math" }
silmaril-renderer = { path = "../silmaril/engine/renderer" }
silmaril-networking = { path = "../silmaril/engine/networking" }
silmaril-physics = { path = "../silmaril/engine/physics" }"#
            } else {
                r#"silmaril-core = "0.1"
silmaril-ecs = "0.1"
silmaril-math = "0.1"
silmaril-renderer = "0.1"
silmaril-networking = "0.1"
silmaril-physics = "0.1""#
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
silmaril-core = {{ workspace = true }}
serde = {{ workspace = true }}
tracing = {{ workspace = true }}

[dev-dependencies]
tokio-test = "0.4"
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
silmaril-core = {{ workspace = true }}
silmaril-networking = {{ workspace = true }}
tokio = {{ workspace = true }}
tracing = {{ workspace = true }}
tracing-subscriber = {{ workspace = true }}
anyhow = {{ workspace = true }}
engine-dev-tools-hot-reload = {{ path = "../../engine/dev-tools/hot-reload", optional = true }}
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

    // Start dev reload server — no-op in release builds (dev feature off)
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
silmaril-core = {{ workspace = true }}
silmaril-renderer = {{ workspace = true }}
silmaril-networking = {{ workspace = true }}
tokio = {{ workspace = true }}
tracing = {{ workspace = true }}
tracing-subscriber = {{ workspace = true }}
anyhow = {{ workspace = true }}
engine-dev-tools-hot-reload = {{ path = "../../engine/dev-tools/hot-reload", optional = true }}
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

    // Start dev reload server — no-op in release builds (dev feature off)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_template(name: &str) -> BasicTemplate {
        BasicTemplate::new(name.to_string(), false)
    }

    fn find_file<'a>(files: &'a [TemplateFile], path: &str) -> &'a TemplateFile {
        files.iter().find(|f| f.path == path).unwrap_or_else(|| panic!("file not found: {path}"))
    }

    #[test]
    fn test_game_toml_has_dev_section() {
        let t = make_template("mygame");
        let files = t.files();
        let game_toml = find_file(&files, "game.toml");

        assert!(
            game_toml.content.contains("[dev]"),
            "game.toml should contain [dev] section"
        );
        assert!(
            game_toml.content.contains("dev_server_port = 9999"),
            "game.toml should contain dev_server_port = 9999"
        );
        assert!(
            game_toml.content.contains("dev_client_port = 9998"),
            "game.toml should contain dev_client_port = 9998"
        );
        assert!(
            game_toml.content.contains("server_port = 7777"),
            "game.toml should contain server_port = 7777"
        );
        assert!(
            game_toml.content.contains("server_package = \"mygame-server\""),
            "game.toml should contain server_package with project name"
        );
        assert!(
            game_toml.content.contains("client_package = \"mygame-client\""),
            "game.toml should contain client_package with project name"
        );
    }

    #[test]
    fn test_server_cargo_toml_has_dev_feature() {
        let t = make_template("mygame");
        let files = t.files();
        let server_toml = find_file(&files, "server/Cargo.toml");

        assert!(
            server_toml.content.contains("dev = [\"engine-dev-tools-hot-reload/dev\"]"),
            "server/Cargo.toml should contain dev feature enabling engine-dev-tools-hot-reload/dev"
        );
        assert!(
            server_toml.content.contains("engine-dev-tools-hot-reload"),
            "server/Cargo.toml should depend on engine-dev-tools-hot-reload"
        );
        assert!(
            server_toml.content.contains("optional = true"),
            "server/Cargo.toml engine-dev-tools-hot-reload dependency should be optional"
        );
    }

    #[test]
    fn test_client_cargo_toml_has_dev_feature() {
        let t = make_template("mygame");
        let files = t.files();
        let client_toml = find_file(&files, "client/Cargo.toml");

        assert!(
            client_toml.content.contains("dev = [\"engine-dev-tools-hot-reload/dev\"]"),
            "client/Cargo.toml should contain dev feature enabling engine-dev-tools-hot-reload/dev"
        );
        assert!(
            client_toml.content.contains("engine-dev-tools-hot-reload"),
            "client/Cargo.toml should depend on engine-dev-tools-hot-reload"
        );
        assert!(
            client_toml.content.contains("optional = true"),
            "client/Cargo.toml engine-dev-tools-hot-reload dependency should be optional"
        );
    }

    #[test]
    fn test_server_main_rs_calls_dev_reload_server() {
        let t = make_template("mygame");
        let files = t.files();
        let server_main = find_file(&files, "server/src/main.rs");

        assert!(
            server_main.content.contains("engine_dev_tools_hot_reload::server::DevReloadServer::start(None).await"),
            "server/src/main.rs should call DevReloadServer::start(None).await"
        );
        assert!(
            server_main.content.contains("async fn main"),
            "server/src/main.rs should use async fn main"
        );
        assert!(
            server_main.content.contains("#[tokio::main]"),
            "server/src/main.rs should have #[tokio::main]"
        );
    }

    #[test]
    fn test_client_main_rs_calls_dev_reload_server() {
        let t = make_template("mygame");
        let files = t.files();
        let client_main = find_file(&files, "client/src/main.rs");

        assert!(
            client_main.content.contains("engine_dev_tools_hot_reload::server::DevReloadServer::start(None).await"),
            "client/src/main.rs should call DevReloadServer::start(None).await"
        );
        assert!(
            client_main.content.contains("async fn main"),
            "client/src/main.rs should use async fn main"
        );
        assert!(
            client_main.content.contains("#[tokio::main]"),
            "client/src/main.rs should have #[tokio::main]"
        );
    }

    #[test]
    fn test_project_name_substitution_in_dev_section() {
        let t = make_template("my-awesome-game");
        let files = t.files();
        let game_toml = find_file(&files, "game.toml");

        assert!(
            game_toml.content.contains("server_package = \"my-awesome-game-server\""),
            "server_package should use the actual project name"
        );
        assert!(
            game_toml.content.contains("client_package = \"my-awesome-game-client\""),
            "client_package should use the actual project name"
        );
    }
}
