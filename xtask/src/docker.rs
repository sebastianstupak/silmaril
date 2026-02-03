use anyhow::Result;
use clap::Subcommand;

use crate::utils::*;

#[derive(Subcommand)]
pub enum DockerCommand {
    /// Start development environment with Docker Compose
    Dev,
    /// Start development environment (detached)
    DevDetached,
    /// Stop development Docker environment
    DevStop,
    /// View development Docker logs
    DevLogs,
    /// Rebuild development Docker images
    DevRebuild,
    /// Start production environment
    Prod,
    /// Stop production environment
    ProdStop,
    /// View server logs (production)
    ProdLogs,
    /// Rebuild Docker images
    Rebuild,
    /// Show Docker image sizes
    Sizes,
    /// Clean Docker artifacts
    Clean,
}

pub fn execute(cmd: DockerCommand) -> Result<()> {
    match cmd {
        DockerCommand::Dev => {
            print_section("Starting Development Docker Environment");
            run_command_streaming("docker-compose", &["-f", "docker-compose.dev.yml", "up"])?;
        }
        DockerCommand::DevDetached => {
            print_section("Starting Development Docker Environment (detached)");
            run_command_streaming("docker-compose", &["-f", "docker-compose.dev.yml", "up", "-d"])?;
            print_success("Development environment started");
        }
        DockerCommand::DevStop => {
            print_section("Stopping Development Docker Environment");
            run_command_streaming("docker-compose", &["-f", "docker-compose.dev.yml", "down"])?;
            print_success("Development environment stopped");
        }
        DockerCommand::DevLogs => {
            print_section("Development Docker Logs");
            run_command_streaming(
                "docker-compose",
                &["-f", "docker-compose.dev.yml", "logs", "-f"],
            )?;
        }
        DockerCommand::DevRebuild => {
            print_section("Rebuilding Development Docker Images");
            run_command_streaming(
                "docker-compose",
                &["-f", "docker-compose.dev.yml", "build", "--no-cache"],
            )?;
            run_command_streaming("docker-compose", &["-f", "docker-compose.dev.yml", "up"])?;
        }
        DockerCommand::Prod => {
            print_section("Starting Production Environment");
            run_command_streaming("docker-compose", &["up", "-d"])?;
            print_success("Production environment started");
        }
        DockerCommand::ProdStop => {
            print_section("Stopping Production Environment");
            run_command_streaming("docker-compose", &["down"])?;
            print_success("Production environment stopped");
        }
        DockerCommand::ProdLogs => {
            print_section("Production Server Logs");
            run_command_streaming("docker-compose", &["logs", "-f", "server"])?;
        }
        DockerCommand::Rebuild => {
            print_section("Rebuilding Docker Images");
            run_command_streaming("docker-compose", &["build", "--no-cache"])?;
            print_success("Docker images rebuilt");
        }
        DockerCommand::Sizes => {
            print_section("Docker Image Sizes");
            println!("Development images:");
            let _ = run_command_streaming("docker", &["images", "|", "grep", "agent-game.*dev"]);
            println!("\nProduction images:");
            let _ = run_command_streaming("docker", &["images", "|", "grep", "agent-game-engine"]);
        }
        DockerCommand::Clean => {
            print_section("Cleaning Docker Artifacts");
            run_command_streaming("docker-compose", &["down", "-v"])?;
            run_command_streaming(
                "docker-compose",
                &["-f", "docker-compose.dev.yml", "down", "-v"],
            )?;
            print_success("Docker artifacts cleaned");
        }
    }
    Ok(())
}
