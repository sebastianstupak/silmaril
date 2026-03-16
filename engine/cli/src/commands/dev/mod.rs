#![allow(dead_code)]

pub mod output;
pub mod process;
pub mod watcher;
// other modules will be added in later tasks:
// pub mod orchestrator;
// pub mod reload_client;

use anyhow::Result;
use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum DevSubcommand {
    /// Start server only
    Server,
    /// Start client only
    Client,
}

pub async fn handle_dev_command(subcmd: Option<DevSubcommand>) -> Result<()> {
    // Placeholder — DevOrchestrator wired in Task 12
    match subcmd {
        None => tracing::info!("silm dev: starting server + client (not yet implemented)"),
        Some(DevSubcommand::Server) => tracing::info!("silm dev server (not yet implemented)"),
        Some(DevSubcommand::Client) => tracing::info!("silm dev client (not yet implemented)"),
    }
    Ok(())
}
