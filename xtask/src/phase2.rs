use anyhow::Result;
use clap::Subcommand;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use crate::utils::*;

#[derive(Subcommand)]
pub enum Phase2Command {
    /// Run complete Phase 2 demo (server + client + metrics check)
    Demo,
    /// Run server and client together
    RunBoth,
    /// Check Prometheus metrics endpoint
    CheckMetrics,
    /// Run Phase 2 validation suite
    Validate,
    /// Run all E2E tests
    E2eTests,
    /// Test protocol version check
    TestVersionCheck,
    /// Test connection timeout
    TestTimeout,
}

pub fn execute(cmd: Phase2Command) -> Result<()> {
    match cmd {
        Phase2Command::Demo => {
            print_section("Phase 2 Demo - Full Client/Server Stack");

            print_info("Starting server...");
            let mut server = Command::new("cargo")
                .args(&["run", "--bin", "server"])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?;

            print_info("Waiting for server to initialize (5 seconds)...");
            thread::sleep(Duration::from_secs(5));

            print_info("Starting client...");
            let mut client = Command::new("cargo")
                .args(&["run", "--bin", "client"])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?;

            print_info("Waiting for client to run (10 seconds)...");
            thread::sleep(Duration::from_secs(10));

            print_info("Checking Prometheus metrics...");
            match check_metrics_endpoint() {
                Ok(_) => print_success("✓ Metrics endpoint responding"),
                Err(e) => print_warning(&format!("Metrics check failed: {}", e)),
            }

            print_info("Stopping client...");
            client.kill()?;

            print_info("Stopping server...");
            server.kill()?;

            print_success("Phase 2 demo complete!");
        }

        Phase2Command::RunBoth => {
            print_section("Running Server and Client");

            print_info("Starting server in background...");
            let _server = Command::new("cargo")
                .args(&["run", "--bin", "server"])
                .spawn()?;

            thread::sleep(Duration::from_secs(3));

            print_info("Starting client...");
            run_cargo_streaming(&["run", "--bin", "client"])?;
        }

        Phase2Command::CheckMetrics => {
            print_section("Checking Prometheus Metrics Endpoint");
            check_metrics_endpoint()?;
            print_success("Metrics endpoint is working correctly");
        }

        Phase2Command::Validate => {
            print_section("Phase 2 Validation Suite");

            print_info("1. Running unit tests...");
            run_cargo_streaming(&["test", "--package", "engine-networking"])?;

            print_info("2. Running E2E tests...");
            run_cargo_streaming(&["test", "--package", "engine-shared", "--test", "e2e_tests"])?;

            print_info("3. Testing protocol version check...");
            run_cargo_streaming(&[
                "test",
                "--package",
                "engine-networking",
                "--test",
                "protocol_version_test",
            ])?;

            print_info("4. Checking code quality...");
            run_cargo_streaming(&["clippy", "--package", "engine-networking", "--", "-D", "warnings"])?;

            print_success("✓ Phase 2 validation complete - all checks passed!");
        }

        Phase2Command::E2eTests => {
            print_section("Running Phase 2 E2E Tests");
            run_cargo_streaming(&[
                "test",
                "--package",
                "engine-shared",
                "--test",
                "e2e_tests",
                "--",
                "--nocapture",
            ])?;
            print_success("E2E tests passed");
        }

        Phase2Command::TestVersionCheck => {
            print_section("Testing Protocol Version Check");
            run_cargo_streaming(&[
                "test",
                "--package",
                "engine-networking",
                "--test",
                "protocol_version_test",
                "--",
                "--nocapture",
            ])?;
            print_success("Protocol version tests passed");
        }

        Phase2Command::TestTimeout => {
            print_section("Testing Connection Timeout");
            run_cargo_streaming(&[
                "test",
                "--package",
                "engine-networking",
                "test_client_timeout",
                "--",
                "--nocapture",
            ])?;
            print_success("Connection timeout tests passed");
        }
    }
    Ok(())
}

fn check_metrics_endpoint() -> Result<()> {
    print_info("Attempting to connect to http://localhost:9090/metrics...");

    // Try to fetch metrics using curl or equivalent
    let output = Command::new("curl")
        .args(&["-s", "http://localhost:9090/metrics"])
        .output();

    match output {
        Ok(output) if output.status.success() => {
            let body = String::from_utf8_lossy(&output.stdout);
            if body.contains("engine_") {
                print_success("✓ Found engine metrics");
                print_info(&format!("Sample metrics:\n{}",
                    body.lines().take(10).collect::<Vec<_>>().join("\n")));
                Ok(())
            } else {
                anyhow::bail!("Metrics endpoint responded but no engine metrics found")
            }
        }
        Ok(_) => anyhow::bail!("Metrics endpoint returned error"),
        Err(e) => {
            print_warning(&format!("curl not available or failed: {}", e));
            print_info("Trying alternative method...");

            // Fallback: just check if port is open
            use std::net::TcpStream;
            match TcpStream::connect("127.0.0.1:9090") {
                Ok(_) => {
                    print_success("✓ Metrics port 9090 is open and accepting connections");
                    Ok(())
                }
                Err(e) => anyhow::bail!("Cannot connect to metrics endpoint: {}", e),
            }
        }
    }
}
