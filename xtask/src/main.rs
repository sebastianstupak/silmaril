mod benchmark;
mod build;
mod codegen;
mod dev;
mod docker;
mod lint;
mod phase2;
mod pgo;
mod quality;
mod test;
mod utils;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "xtask")]
#[command(version, about = "Silmaril build automation tasks", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build commands
    Build {
        #[command(subcommand)]
        command: build::BuildCommand,
    },
    /// Test commands
    Test {
        #[command(subcommand)]
        command: test::TestCommand,
    },
    /// Benchmark commands
    Bench {
        #[command(subcommand)]
        command: benchmark::BenchmarkCommand,
    },
    /// Development commands
    Dev {
        #[command(subcommand)]
        command: dev::DevCommand,
    },
    /// Docker commands
    Docker {
        #[command(subcommand)]
        command: docker::DockerCommand,
    },
    /// Code quality commands (fmt, clippy, check)
    Quality {
        #[command(subcommand)]
        command: quality::QualityCommand,
    },
    /// Profile-Guided Optimization commands
    Pgo {
        #[command(subcommand)]
        command: pgo::PgoCommand,
    },
    /// Phase 2 networking commands and validation
    Phase2 {
        #[command(subcommand)]
        command: phase2::Phase2Command,
    },
    /// Generate TypeScript bindings from Tauri command types
    Codegen,
    /// Verify generated TypeScript bindings are up to date (for CI)
    CheckBindings,
    /// Run all checks (format + clippy + test) - shorthand
    Check,
    /// Format all code - shorthand
    Fmt,
    /// Run clippy lints - shorthand
    Clippy,
    /// Check formatting - shorthand
    FmtCheck,
    /// Build documentation
    Doc {
        /// Open documentation in browser
        #[arg(long)]
        open: bool,
    },
    /// Watch for changes and rebuild
    Watch {
        /// Watch tests instead of build
        #[arg(long)]
        test: bool,
    },
    /// Check project compiles (fast, no codegen)
    CheckCompile,
    /// Show binary sizes
    Sizes,
    /// Update dependencies
    Update,
    /// Show outdated dependencies
    Outdated,
    /// Setup git hooks and development environment
    SetupHooks,
    /// Run client
    RunClient,
    /// Run server
    RunServer,
    /// Lint: enforce undo handler coverage invariant
    Lint,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Build { command } => build::execute(command),
        Commands::Test { command } => test::execute(command),
        Commands::Bench { command } => benchmark::execute(command),
        Commands::Dev { command } => dev::execute(command),
        Commands::Docker { command } => docker::execute(command),
        Commands::Quality { command } => quality::execute(command),
        Commands::Pgo { command } => pgo::execute(command),
        Commands::Phase2 { command } => phase2::execute(command),
        Commands::Codegen => codegen::run_codegen(),
        Commands::CheckBindings => codegen::run_check_bindings(),
        Commands::Check => quality::execute(quality::QualityCommand::Check),
        Commands::Fmt => quality::execute(quality::QualityCommand::Fmt),
        Commands::Clippy => quality::execute(quality::QualityCommand::Clippy),
        Commands::FmtCheck => quality::execute(quality::QualityCommand::FmtCheck),
        Commands::Doc { open } => {
            utils::print_section("Building Documentation");
            if open {
                utils::run_cargo_streaming(&["doc", "--no-deps", "--all-features", "--open"])?;
            } else {
                utils::run_cargo_streaming(&["doc", "--no-deps", "--all-features"])?;
            }
            utils::print_success("Documentation built");
            Ok(())
        }
        Commands::Watch { test } => {
            utils::print_section("Starting Watch Mode");
            if test {
                utils::run_cargo_streaming(&["watch", "-x", "test --all-features"])?;
            } else {
                utils::run_cargo_streaming(&["watch", "-x", "build --bin server"])?;
            }
            Ok(())
        }
        Commands::CheckCompile => {
            utils::print_section("Checking Compilation");
            utils::run_cargo_streaming(&["check", "--all-targets", "--all-features"])?;
            utils::print_success("Compilation check passed");
            Ok(())
        }
        Commands::Sizes => {
            utils::print_section("Binary Sizes");
            println!("Client (dev):");
            let _ = utils::run_command_streaming("ls", &["-lh", "target/debug/client*"]);
            println!("\nServer (dev):");
            let _ = utils::run_command_streaming("ls", &["-lh", "target/debug/server*"]);
            println!("\nClient (release):");
            let _ = utils::run_command_streaming("ls", &["-lh", "target/release/client*"]);
            println!("\nServer (release-server):");
            let _ = utils::run_command_streaming("ls", &["-lh", "target/release-server/server*"]);
            Ok(())
        }
        Commands::Update => {
            utils::print_section("Updating Dependencies");
            utils::run_cargo_streaming(&["update"])?;
            utils::print_success("Dependencies updated");
            Ok(())
        }
        Commands::Outdated => {
            utils::print_section("Checking Outdated Dependencies");
            utils::run_cargo_streaming(&["outdated"])?;
            Ok(())
        }
        Commands::SetupHooks => {
            utils::print_section("Setting up Development Environment");
            utils::print_info("Installing git hooks...");

            let project_root = utils::project_root()?;
            let hooks_dir = project_root.join(".git").join("hooks");
            let source_hook = project_root.join("scripts").join("hooks").join("pre-commit");
            let dest_hook = hooks_dir.join("pre-commit");

            if !source_hook.exists() {
                utils::print_error(&format!(
                    "Pre-commit hook not found at {}",
                    source_hook.display()
                ));
                anyhow::bail!("Pre-commit hook not found");
            }

            std::fs::create_dir_all(&hooks_dir)?;
            std::fs::copy(&source_hook, &dest_hook)?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = std::fs::metadata(&dest_hook)?.permissions();
                perms.set_mode(0o755);
                std::fs::set_permissions(&dest_hook, perms)?;
            }

            utils::print_success("Pre-commit hook installed");
            println!();
            utils::print_info("Pre-commit hooks will now run automatically.");
            Ok(())
        }
        Commands::RunClient => {
            utils::print_section("Running Client");
            utils::run_cargo_streaming(&["run", "--bin", "client"])?;
            Ok(())
        }
        Commands::RunServer => {
            utils::print_section("Running Server");
            utils::run_cargo_streaming(&["run", "--bin", "server"])?;
            Ok(())
        }
        Commands::Lint => lint::run_lint(),
    }
}
