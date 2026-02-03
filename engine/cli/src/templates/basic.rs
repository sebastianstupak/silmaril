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
            self.shared_components_rs(),
            self.shared_systems_rs(),
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
        let content = r#"//! Shared game logic that runs on both client and server.
//!
//! This crate contains:
//! - Components (data): Health, Transform, Velocity, etc.
//! - Systems (logic): movement, combat, regeneration, etc.
//!
//! IMPORTANT: Code here must be deterministic and work the same on client & server.

pub mod components;
pub mod systems;

// Re-export for convenience
pub use components::*;
pub use systems::*;
"#;
        TemplateFile::new("shared/src/lib.rs", content)
    }

    fn shared_components_rs(&self) -> TemplateFile {
        let content = r#"//! Game components (data structures attached to entities)
//!
//! Add your components here using `silm add component`

use serde::{Deserialize, Serialize};

/// Example: Transform component (position, rotation, scale)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Transform {
    pub position: [f32; 3],
    pub rotation: [f32; 4], // Quaternion
    pub scale: [f32; 3],
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0], // Identity quaternion
            scale: [1.0, 1.0, 1.0],
        }
    }
}

/// Example: Velocity component
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Velocity {
    pub linear: [f32; 3],
    pub angular: [f32; 3],
}

impl Default for Velocity {
    fn default() -> Self {
        Self {
            linear: [0.0, 0.0, 0.0],
            angular: [0.0, 0.0, 0.0],
        }
    }
}
"#;
        TemplateFile::new("shared/src/components.rs", content)
    }

    fn shared_systems_rs(&self) -> TemplateFile {
        let content = r#"//! Game systems (logic that operates on components)
//!
//! Add your systems here using `silm add system`

use tracing::debug;
use crate::components::*;

/// Example: Simple movement system
pub fn movement_system(transform: &mut Transform, velocity: &Velocity, dt: f32) {
    transform.position[0] += velocity.linear[0] * dt;
    transform.position[1] += velocity.linear[1] * dt;
    transform.position[2] += velocity.linear[2] * dt;

    debug!(
        position = ?transform.position,
        velocity = ?velocity.linear,
        "Entity moved"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_movement_system() {
        let mut transform = Transform::default();
        let velocity = Velocity {
            linear: [1.0, 0.0, 0.0],
            angular: [0.0, 0.0, 0.0],
        };

        movement_system(&mut transform, &velocity, 1.0);

        assert_eq!(transform.position[0], 1.0);
        assert_eq!(transform.position[1], 0.0);
        assert_eq!(transform.position[2], 0.0);
    }
}
"#;
        TemplateFile::new("shared/src/systems.rs", content)
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

[dependencies]
{name}-shared = {{ path = "../shared" }}
silmaril-core = {{ workspace = true }}
silmaril-networking = {{ workspace = true }}
tokio = {{ workspace = true }}
tracing = {{ workspace = true }}
tracing-subscriber = {{ workspace = true }}
anyhow = {{ workspace = true }}
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

fn main() -> anyhow::Result<()> {{
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("{name} server starting...");

    // TODO: Initialize game server
    // - Load server config
    // - Initialize ECS world
    // - Start network server
    // - Run game loop (60 TPS)

    info!("{name} server running on 0.0.0.0:7777");

    // Keep server running
    std::thread::park();

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

[dependencies]
{name}-shared = {{ path = "../shared" }}
silmaril-core = {{ workspace = true }}
silmaril-renderer = {{ workspace = true }}
silmaril-networking = {{ workspace = true }}
tokio = {{ workspace = true }}
tracing = {{ workspace = true }}
tracing-subscriber = {{ workspace = true }}
anyhow = {{ workspace = true }}
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

fn main() -> anyhow::Result<()> {{
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("{name} client starting...");

    // TODO: Initialize game client
    // - Load client config
    // - Initialize Vulkan renderer
    // - Connect to server
    // - Run game loop (60 FPS target)

    info!("{name} client running");

    // Keep client running
    std::thread::park();

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
