//! WASM build implementation using Trunk.

use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;

use super::BuildRunner;

/// Build the client for WASM using Trunk.
///
/// Runs `trunk build client/index.html --dist dist/wasm [--release]`.
///
/// # Errors
///
/// Returns an error if the trunk build command fails.
pub fn build_wasm(
    runner: &dyn BuildRunner,
    project_root: &Path,
    env: &HashMap<String, String>,
    release: bool,
) -> Result<()> {
    let mut args: Vec<String> = vec![
        "build".into(),
        "client/index.html".into(),
        "--dist".into(),
        "dist/wasm".into(),
    ];

    if release {
        args.push("--release".into());
    }

    runner.run_command("trunk", &args, env, project_root)
}
