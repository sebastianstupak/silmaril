// TODO: These operations are duplicated in engine_ops::module.
// The CLI should eventually delegate to engine_ops instead of
// maintaining its own implementations. See: engine/ops/src/module/

pub mod list;
pub mod remove;

use anyhow::Result;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum ModuleCommand {
    /// List installed modules and their resolved versions
    List,
    /// Remove a module and its wiring
    Remove {
        /// Module name (e.g. combat)
        name: String,
    },
}

pub fn handle_module_command(command: ModuleCommand, project_root: std::path::PathBuf) -> Result<()> {
    match command {
        ModuleCommand::List => list::list_modules(&project_root),
        ModuleCommand::Remove { name } => remove::remove_module(&name, &project_root),
    }
}
