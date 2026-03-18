use anyhow::Result;
use clap::Command;
use clap_complete::{generate, Shell};
use std::io;

/// Generate shell completions and print to stdout.
pub fn generate_completions(shell: Shell, cmd: &mut Command) -> Result<()> {
    generate(shell, cmd, "silm", &mut io::stdout());
    Ok(())
}
