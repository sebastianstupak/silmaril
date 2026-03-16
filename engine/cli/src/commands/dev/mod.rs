#![allow(dead_code)]

pub mod orchestrator;
pub mod output;
pub mod process;
pub mod reload_client;
pub mod watcher;

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
    orchestrator::run(subcmd).await
}
