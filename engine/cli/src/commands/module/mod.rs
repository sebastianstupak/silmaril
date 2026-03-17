#[allow(dead_code)] // functions will be wired in Task 7 (CLI registration)
pub mod list;
#[allow(dead_code)] // stub — implemented in Task 6
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

#[allow(dead_code)] // will be wired in Task 7 (CLI registration)
pub fn handle_module_command(command: ModuleCommand, project_root: std::path::PathBuf) -> Result<()> {
    match command {
        ModuleCommand::List => list::list_modules(&project_root),
        ModuleCommand::Remove { name } => remove::remove_module(&name, &project_root),
    }
}
