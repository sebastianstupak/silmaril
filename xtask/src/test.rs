use anyhow::Result;
use clap::Subcommand;

use crate::utils::*;

#[derive(Subcommand)]
pub enum TestCommand {
    /// Run all tests
    All,
    /// Run tests for client code only
    Client,
    /// Run tests for server code only
    Server,
    /// Run macro tests
    Macros,
    /// Run with verbose output
    Verbose,
    /// Test ECS (Entity Component System)
    Ecs,
    /// Test serialization
    Serialization,
    /// Test physics
    Physics,
    /// Test renderer
    Renderer,
    /// Test math
    Math,
    /// Test networking
    Networking,
    /// Test profiling
    Profiling,
}

pub fn execute(cmd: TestCommand) -> Result<()> {
    match cmd {
        TestCommand::All => {
            print_section("Running All Tests");
            run_cargo_streaming(&["test", "--all-features"])?;
            print_success("All tests passed");
        }
        TestCommand::Client => {
            print_section("Running Client Tests");
            run_cargo_streaming(&["test", "--features", "client"])?;
            print_success("Client tests passed");
        }
        TestCommand::Server => {
            print_section("Running Server Tests");
            run_cargo_streaming(&["test", "--features", "server"])?;
            print_success("Server tests passed");
        }
        TestCommand::Macros => {
            print_section("Running Macro Tests");
            run_cargo_streaming(&["test", "--package", "engine-macros"])?;
            print_success("Macro tests passed");
        }
        TestCommand::Verbose => {
            print_section("Running Tests (verbose)");
            run_cargo_streaming(&["test", "--all-features", "--", "--nocapture"])?;
            print_success("Tests passed");
        }
        TestCommand::Ecs => {
            print_section("Testing ECS");
            run_cargo_streaming(&["test", "--package", "engine-core", "--lib", "ecs"])?;
            run_cargo_streaming(&[
                "test",
                "--package",
                "engine-core",
                "--test",
                "ecs_integration",
            ])?;
            print_success("ECS tests passed");
        }
        TestCommand::Serialization => {
            print_section("Testing Serialization");
            run_cargo_streaming(&["test", "--package", "engine-core", "--lib", "serialization"])?;
            run_cargo_streaming(&[
                "test",
                "--package",
                "engine-core",
                "--test",
                "serialization_integration",
            ])?;
            print_success("Serialization tests passed");
        }
        TestCommand::Physics => {
            print_section("Testing Physics");
            run_cargo_streaming(&["test", "--package", "engine-physics"])?;
            print_success("Physics tests passed");
        }
        TestCommand::Renderer => {
            print_section("Testing Renderer");
            run_cargo_streaming(&["test", "--package", "engine-renderer"])?;
            print_success("Renderer tests passed");
        }
        TestCommand::Math => {
            print_section("Testing Math");
            run_cargo_streaming(&["test", "--package", "engine-math"])?;
            print_success("Math tests passed");
        }
        TestCommand::Networking => {
            print_section("Testing Networking");
            run_cargo_streaming(&["test", "--package", "engine-networking"])?;
            print_success("Networking tests passed");
        }
        TestCommand::Profiling => {
            print_section("Testing Profiling");
            run_cargo_streaming(&["test", "--package", "engine-profiling"])?;
            print_success("Profiling tests passed");
        }
    }
    Ok(())
}
