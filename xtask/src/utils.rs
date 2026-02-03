use anyhow::{Context, Result};
use colored::Colorize;
use std::env;
use std::path::PathBuf;
use std::process::{Command, Output};

/// Get the cargo command to use (respects CARGO env var)
pub fn cargo() -> String {
    env::var("CARGO").unwrap_or_else(|_| "cargo".to_string())
}

/// Get the project root directory
pub fn project_root() -> Result<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .context("Failed to find git root")?;

    if !output.status.success() {
        anyhow::bail!("Not in a git repository");
    }

    let path = String::from_utf8(output.stdout)?;
    Ok(PathBuf::from(path.trim()))
}

/// Run a cargo command
pub fn run_cargo(args: &[&str]) -> Result<Output> {
    let cargo = cargo();
    println!("{} {}", "Running:".cyan(), format!("{} {}", cargo, args.join(" ")).bold());

    let output = Command::new(&cargo).args(args).output().context(format!(
        "Failed to run: {} {}",
        cargo,
        args.join(" ")
    ))?;

    if !output.status.success() {
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        anyhow::bail!("Command failed: {} {}", cargo, args.join(" "));
    }

    Ok(output)
}

/// Run a cargo command and stream output
pub fn run_cargo_streaming(args: &[&str]) -> Result<()> {
    let cargo = cargo();
    println!("{} {}", "Running:".cyan(), format!("{} {}", cargo, args.join(" ")).bold());

    let status = Command::new(&cargo).args(args).status().context(format!(
        "Failed to run: {} {}",
        cargo,
        args.join(" ")
    ))?;

    if !status.success() {
        anyhow::bail!("Command failed: {} {}", cargo, args.join(" "));
    }

    Ok(())
}

/// Run a command (non-cargo)
pub fn run_command(program: &str, args: &[&str]) -> Result<Output> {
    println!("{} {}", "Running:".cyan(), format!("{} {}", program, args.join(" ")).bold());

    let output = Command::new(program).args(args).output().context(format!(
        "Failed to run: {} {}",
        program,
        args.join(" ")
    ))?;

    if !output.status.success() {
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        anyhow::bail!("Command failed: {} {}", program, args.join(" "));
    }

    Ok(output)
}

/// Run a command and stream output
pub fn run_command_streaming(program: &str, args: &[&str]) -> Result<()> {
    println!("{} {}", "Running:".cyan(), format!("{} {}", program, args.join(" ")).bold());

    let status = Command::new(program).args(args).status().context(format!(
        "Failed to run: {} {}",
        program,
        args.join(" ")
    ))?;

    if !status.success() {
        anyhow::bail!("Command failed: {} {}", program, args.join(" "));
    }

    Ok(())
}

/// Print success message
pub fn print_success(message: &str) {
    println!("{} {}", "✓".green().bold(), message);
}

/// Print error message
pub fn print_error(message: &str) {
    eprintln!("{} {}", "✗".red().bold(), message);
}

/// Print info message
pub fn print_info(message: &str) {
    println!("{} {}", "ℹ".blue().bold(), message);
}

/// Print warning message
pub fn print_warning(message: &str) {
    println!("{} {}", "⚠".yellow().bold(), message);
}

/// Print section header
pub fn print_section(title: &str) {
    println!();
    println!("{}", "=".repeat(60).cyan());
    println!("{}", title.bold());
    println!("{}", "=".repeat(60).cyan());
    println!();
}
