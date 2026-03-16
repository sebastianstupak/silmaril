//! `DevOrchestrator` ties together FileWatcher, ProcessManager, ReloadClient,
//! and OutputMux into the complete `silm dev` workflow.

use crate::commands::dev::{
    output::{OutputMux, OutputSender, Source},
    process::ProcessManager,
    reload_client::DevReloadClient,
    watcher::{ChangeKind, FileChange, FileWatcher},
    DevSubcommand,
};
use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

// ────────────────────────────────────────────────────────────────────────
// game.toml parsing
// ────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct GameToml {
    #[allow(dead_code)]
    project: ProjectSection,
    dev: DevSection,
}

#[derive(Debug, Deserialize)]
struct ProjectSection {
    #[allow(dead_code)]
    name: String,
}

#[derive(Debug, Deserialize)]
struct DevSection {
    server_package: String,
    client_package: String,
    #[serde(default = "default_dev_server_port")]
    dev_server_port: u16,
    #[serde(default = "default_dev_client_port")]
    dev_client_port: u16,
}

fn default_dev_server_port() -> u16 {
    9999
}
fn default_dev_client_port() -> u16 {
    9998
}

fn find_project_root() -> Result<PathBuf> {
    let mut dir = std::env::current_dir()?;
    loop {
        if dir.join("game.toml").exists() {
            return Ok(dir);
        }
        if !dir.pop() {
            anyhow::bail!("could not find game.toml — are you inside a Silmaril project?");
        }
    }
}

fn read_game_toml(root: &Path) -> Result<GameToml> {
    let content = std::fs::read_to_string(root.join("game.toml"))
        .context("could not read game.toml")?;
    toml::from_str(&content).context("could not parse game.toml")
}

// ────────────────────────────────────────────────────────────────────────
// Public entry point
// ────────────────────────────────────────────────────────────────────────

/// Run `silm dev [server|client]`.
pub async fn run(subcmd: Option<DevSubcommand>) -> Result<()> {
    let project_root = find_project_root()?;
    let config = read_game_toml(&project_root)?;

    info!(root = %project_root.display(), "starting silm dev");

    // Create .silmaril/ dir
    std::fs::create_dir_all(project_root.join(".silmaril"))
        .context("could not create .silmaril/")?;

    let mux = OutputMux::new();
    let mux_sender = mux.sender();

    // Start output mux in background
    tokio::spawn(async move { mux.run().await });

    let reload_client = DevReloadClient::new(
        config.dev.dev_server_port,
        config.dev.dev_client_port,
    );

    // Determine which processes to run
    let (run_server, run_client) = match subcmd {
        None => (true, true),
        Some(DevSubcommand::Server) => (true, false),
        Some(DevSubcommand::Client) => (false, true),
    };

    let mut server_mgr = if run_server {
        let mut mgr = ProcessManager::new(
            config.dev.server_package.clone(),
            config.dev.dev_server_port,
        );
        mgr.start(&project_root, mux_sender.clone(), Source::Server)
            .await
            .context("failed to start server")?;
        Some(mgr)
    } else {
        None
    };

    let mut client_mgr = if run_client {
        let mut mgr = ProcessManager::new(
            config.dev.client_package.clone(),
            config.dev.dev_client_port,
        );
        mgr.start(&project_root, mux_sender.clone(), Source::Client)
            .await
            .context("failed to start client")?;
        Some(mgr)
    } else {
        None
    };

    // File watcher: new() returns (watcher, rx), start() takes a tx
    let (tx, mut change_rx) = tokio::sync::mpsc::channel(256);
    let (watcher, _unused_rx) = FileWatcher::new(project_root.clone());
    watcher.start(tx).context("failed to start file watcher")?;

    mux_sender
        .send(Source::Dev, "silm dev running — watching for changes")
        .await;

    // Event loop
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                mux_sender.send(Source::Dev, "[dev] shutting down").await;
                if let Some(ref mut mgr) = server_mgr {
                    mgr.stop().await.ok();
                }
                if let Some(ref mut mgr) = client_mgr {
                    mgr.stop().await.ok();
                }
                break;
            }
            Some(change) = change_rx.recv() => {
                handle_change(
                    change,
                    &mut server_mgr,
                    &mut client_mgr,
                    &reload_client,
                    &project_root,
                    &mux_sender,
                ).await;
            }
        }
    }

    Ok(())
}

async fn handle_change(
    change: FileChange,
    server_mgr: &mut Option<ProcessManager>,
    client_mgr: &mut Option<ProcessManager>,
    reload_client: &DevReloadClient,
    project_root: &Path,
    mux_sender: &OutputSender,
) {
    let path_str = change.path.to_string_lossy().to_string();

    match change.kind {
        ChangeKind::Asset { .. } => {
            info!(path = %path_str, "asset change detected");
            reload_client.reload_asset_to_both(&path_str).await;
        }
        ChangeKind::Config { .. } => {
            info!(path = %path_str, "config change detected");
            reload_client.reload_config_smart(&path_str).await;
        }
        ChangeKind::Code { crate_name } => {
            info!(crate_name = %crate_name, "code change detected — state-preserving restart");
            mux_sender
                .send(
                    Source::Dev,
                    format!("[dev] code change in {} — restarting", crate_name),
                )
                .await;

            let restart_server = crate_name == "server" || crate_name == "shared";
            let restart_client = crate_name == "client" || crate_name == "shared";

            if restart_server {
                if let Some(ref mut mgr) = server_mgr {
                    reload_client.serialize_state_server().await;
                    if let Err(e) = mgr
                        .rebuild_and_restart(project_root, mux_sender.clone(), Source::Server)
                        .await
                    {
                        warn!(error = ?e, "server rebuild_and_restart failed");
                    }
                }
            }
            if restart_client {
                if let Some(ref mut mgr) = client_mgr {
                    reload_client.serialize_state_client().await;
                    if let Err(e) = mgr
                        .rebuild_and_restart(project_root, mux_sender.clone(), Source::Client)
                        .await
                    {
                        warn!(error = ?e, "client rebuild_and_restart failed");
                    }
                }
            }
        }
    }
}
