//! Installer generation via cargo-packager (AppImage/DMG/NSIS).

use anyhow::{bail, Result};
use std::path::Path;
use std::process::Command;

/// Check if cargo-packager is installed.
pub fn check_packager() -> Result<()> {
    match Command::new("cargo-packager").arg("--version").output() {
        Ok(output) if output.status.success() => Ok(()),
        _ => bail!(
            "'cargo-packager' not found — install: cargo install cargo-packager\n\
             See: https://github.com/nicosalm/packager"
        ),
    }
}

/// Generate a packager.toml config from game project metadata.
pub fn generate_packager_config(
    project_name: &str,
    version: &str,
    description: &str,
    binary_name: &str,
) -> String {
    format!(
        r#"[package]
product-name = "{name}"
version = "{version}"
description = "{description}"
identifier = "com.silmaril.{name}"

[[package.binaries]]
name = "{binary}"
path = "{binary}"
"#,
        name = project_name,
        version = version,
        description = description,
        binary = binary_name,
    )
}

/// Run cargo-packager with the generated config.
pub fn run_packager(project_root: &Path) -> Result<()> {
    let config_path = project_root.join("packager.toml");
    if !config_path.exists() {
        bail!("packager.toml not found — run silm package --installer to generate it");
    }

    tracing::info!("[silm] running cargo-packager...");
    let status = Command::new("cargo-packager")
        .args(["--config", "packager.toml"])
        .current_dir(project_root)
        .status()
        .map_err(|e| anyhow::anyhow!("failed to run cargo-packager: {}", e))?;

    if !status.success() {
        bail!("cargo-packager failed with status: {}", status);
    }

    tracing::info!("[silm] installers created successfully");
    Ok(())
}
