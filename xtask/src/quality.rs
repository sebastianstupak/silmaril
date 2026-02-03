use anyhow::Result;
use clap::Subcommand;

use crate::utils::*;

#[derive(Subcommand)]
pub enum QualityCommand {
    /// Format all code
    Fmt,
    /// Check formatting
    FmtCheck,
    /// Run clippy lints
    Clippy,
    /// Fix clippy issues automatically
    ClippyFix,
    /// Run all checks (format + clippy + test)
    Check,
}

pub fn execute(cmd: QualityCommand) -> Result<()> {
    match cmd {
        QualityCommand::Fmt => {
            print_section("Formatting Code");
            run_cargo_streaming(&["fmt", "--all"])?;
            print_success("Code formatted");
        }
        QualityCommand::FmtCheck => {
            print_section("Checking Formatting");
            run_cargo_streaming(&["fmt", "--all", "--check"])?;
            print_success("Formatting check passed");
        }
        QualityCommand::Clippy => {
            print_section("Running Clippy");
            run_cargo_streaming(&[
                "clippy",
                "--all-targets",
                "--all-features",
                "--",
                "-D",
                "warnings",
            ])?;
            print_success("Clippy check passed");
        }
        QualityCommand::ClippyFix => {
            print_section("Fixing Clippy Issues");
            run_cargo_streaming(&["clippy", "--all-targets", "--all-features", "--fix"])?;
            print_success("Clippy fixes applied");
        }
        QualityCommand::Check => {
            print_section("Running All Checks");

            print_info("Step 1/3: Checking formatting...");
            run_cargo_streaming(&["fmt", "--all", "--check"])?;
            print_success("Formatting check passed");

            println!();
            print_info("Step 2/3: Running clippy...");
            run_cargo_streaming(&[
                "clippy",
                "--all-targets",
                "--all-features",
                "--",
                "-D",
                "warnings",
            ])?;
            print_success("Clippy check passed");

            println!();
            print_info("Step 3/3: Running tests...");
            run_cargo_streaming(&["test", "--all-features"])?;
            print_success("All tests passed");

            println!();
            print_success("All checks passed!");
        }
    }
    Ok(())
}
