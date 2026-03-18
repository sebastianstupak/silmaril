//! CLI wrapper for `silm add component` — delegates to engine_ops::codegen.

use anyhow::Result;
use super::wiring::Target;

pub fn add_component(
    name: &str,
    fields_str: &str,
    target: Target,
    domain: &str,
) -> Result<()> {
    let ops_target = match target {
        Target::Shared => engine_ops::project::Target::Shared,
        Target::Server => engine_ops::project::Target::Server,
        Target::Client => engine_ops::project::Target::Client,
    };
    engine_ops::codegen::add_component(name, fields_str, ops_target, domain)?;
    tracing::info!("[silm] component '{}' added to {}/{}", name, target.crate_subdir(), domain);
    Ok(())
}
