//! Native (cargo/cross) build implementation.

use anyhow::{bail, Result};
use std::collections::HashMap;
use std::path::Path;

use super::{BuildKind, BuildRunner, BuildTool};

/// Build native server and/or client binaries using cargo or cross.
///
/// # Errors
///
/// Returns an error if:
/// - `tool` is [`BuildTool::Trunk`] (use [`super::wasm::build_wasm`] instead)
/// - The underlying build command fails
#[allow(clippy::too_many_arguments)]
pub fn build_native(
    runner: &dyn BuildRunner,
    project_root: &Path,
    env: &HashMap<String, String>,
    server_package: &str,
    client_package: &str,
    tool: BuildTool,
    target_triple: Option<&str>,
    kind: BuildKind,
    release: bool,
) -> Result<()> {
    let program = match tool {
        BuildTool::Cargo => "cargo",
        BuildTool::Cross => "cross",
        BuildTool::Trunk => bail!("Trunk is not a native build tool — use build_wasm instead"),
    };

    let binaries: Vec<(&str, &str)> = match kind {
        BuildKind::ServerAndClient => vec![(server_package, "server"), (client_package, "client")],
        BuildKind::ServerOnly => vec![(server_package, "server")],
        BuildKind::ClientOnly => vec![(client_package, "client")],
    };

    for (package, bin_name) in binaries {
        let mut args: Vec<String> = vec!["build".into()];

        if let Some(triple) = target_triple {
            args.push("--target".into());
            args.push(triple.into());
        }

        args.push("--package".into());
        args.push(package.into());
        args.push("--bin".into());
        args.push(bin_name.into());

        if release {
            args.push("--release".into());
        }

        runner.run_command(program, &args, env, project_root)?;
    }

    Ok(())
}
