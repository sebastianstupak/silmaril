//! Game-specific build automation tasks

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
