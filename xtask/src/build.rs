use anyhow::Result;
use clap::Subcommand;

use crate::utils::*;

#[derive(Subcommand)]
pub enum BuildCommand {
    /// Build client (dev)
    Client,
    /// Build server (dev)
    Server,
    /// Build both binaries (dev)
    Both,
    /// Build client (release - optimized for performance)
    ClientRelease,
    /// Build server (release - size-optimized)
    ServerRelease,
    /// Build both binaries (release)
    Release,
    /// Clean build artifacts
    Clean,
}

pub fn execute(cmd: BuildCommand) -> Result<()> {
    match cmd {
        BuildCommand::Client => {
            print_section("Building Client (dev)");
            run_cargo_streaming(&["build", "--bin", "client"])?;
            print_success("Client built successfully");
        }
        BuildCommand::Server => {
            print_section("Building Server (dev)");
            run_cargo_streaming(&["build", "--bin", "server"])?;
            print_success("Server built successfully");
        }
        BuildCommand::Both => {
            print_section("Building Client and Server (dev)");
            run_cargo_streaming(&["build", "--bin", "client"])?;
            run_cargo_streaming(&["build", "--bin", "server"])?;
            print_success("Both binaries built successfully");
        }
        BuildCommand::ClientRelease => {
            print_section("Building Client (release)");
            run_cargo_streaming(&["build", "--bin", "client", "--release"])?;
            print_success("Client release built successfully");
        }
        BuildCommand::ServerRelease => {
            print_section("Building Server (release)");
            run_cargo_streaming(&["build", "--bin", "server", "--profile", "release-server"])?;
            print_success("Server release built successfully");
        }
        BuildCommand::Release => {
            print_section("Building Release Binaries");
            run_cargo_streaming(&["build", "--bin", "client", "--release"])?;
            run_cargo_streaming(&["build", "--bin", "server", "--profile", "release-server"])?;
            print_success("Both release binaries built successfully");
        }
        BuildCommand::Clean => {
            print_section("Cleaning Build Artifacts");
            run_cargo_streaming(&["clean"])?;
            print_success("Clean complete");
        }
    }
    Ok(())
}
