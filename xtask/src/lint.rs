use anyhow::Result;
use std::process::Command;

use crate::utils;

pub fn run_lint() -> Result<()> {
    utils::print_section("Running Undo Coverage Lint");

    let cargo = utils::cargo();
    let status = Command::new(&cargo)
        .args([
            "test",
            "-p",
            "silmaril-editor",
            "--",
            "lint_undo_coverage",
            "--nocapture",
        ])
        .status()?;

    if !status.success() {
        utils::print_error("Undo coverage lint failed");
        anyhow::bail!("Undo coverage lint failed");
    }

    utils::print_success("Undo coverage lint passed");
    Ok(())
}
