use anyhow::Result;
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
