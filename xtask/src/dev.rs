use anyhow::Result;
use clap::Subcommand;

use crate::utils::*;

#[derive(Subcommand)]
pub enum DevCommand {
    /// Start full development environment (client + server with auto-reload)
    Full,
    /// Run client only (development mode with auto-reload)
    Client,
    /// Run server only (development mode with auto-reload)
    Server,
    /// Development with live log streaming (pretty formatted)
    Logs,
    /// Development with profiler attached (Puffin)
    Profiler,
    /// Development with debugger ready (extra debug symbols)
    Debug,
    /// Development with hot reload (assets only, no code reload)
    HotReload,
    /// Development in release mode (optimized but debuggable)
    Release,
    /// Clean and restart dev environment
    Clean,
    /// Check dev environment status
    Status,
    /// Stop all dev instances
    StopAll,
    /// Quick dev benchmarks (fast iteration)
    Benchmark,
    /// Development with full tracing (Chrome trace format)
    Trace,
    /// Development with Vulkan validation layers
    Validation,
    /// Development with metrics dashboard
    Metrics,
    /// Run multiple clients for multiplayer testing
    Multi { count: Option<u32> },
    /// Headless development (no rendering)
    Headless,
}

pub fn execute(cmd: DevCommand) -> Result<()> {
    match cmd {
        DevCommand::Full => {
            print_section("Starting Development Environment");
            print_warning("Full dev environment not yet implemented");
            print_info("Starting server instead...");
            run_cargo_streaming(&["run", "--bin", "server"])?;
        }
        DevCommand::Client => {
            print_section("Starting Client (development mode)");
            run_cargo_streaming(&["run", "--bin", "client"])?;
        }
        DevCommand::Server => {
            print_section("Starting Server (development mode)");
            run_cargo_streaming(&["run", "--bin", "server"])?;
        }
        DevCommand::Logs => {
            print_section("Starting Development with Live Logs");
            std::env::set_var("RUST_LOG", "debug");
            run_cargo_streaming(&["run", "--bin", "server"])?;
        }
        DevCommand::Profiler => {
            print_section("Starting Development with Profiler");
            print_info("Building with profiling support...");
            run_cargo_streaming(&["build", "--features", "profiling-puffin"])?;
            println!();
            print_success("Profiling enabled");
            print_info("Connect puffin_viewer to localhost:8585");
            println!();
            run_cargo_streaming(&["run", "--bin", "server", "--features", "profiling-puffin"])?;
        }
        DevCommand::Debug => {
            print_section("Starting Development in Debug Mode");
            std::env::set_var("RUST_LOG", "debug");
            std::env::set_var("RUST_BACKTRACE", "full");
            std::env::set_var("RUSTFLAGS", "-C debuginfo=2");
            print_info("Building with full debug symbols...");
            run_cargo_streaming(&["build", "--bin", "client"])?;
            println!();
            print_success("Debug build complete");
            print_info("Binary: target/debug/client");
            println!();
            run_cargo_streaming(&["run", "--bin", "client"])?;
        }
        DevCommand::HotReload => {
            print_section("Starting Development with Hot Reload");
            print_warning("Asset hot-reload not yet implemented (Phase 3)");
            print_info("Starting normal development mode instead...");
            run_cargo_streaming(&["run", "--bin", "server"])?;
        }
        DevCommand::Release => {
            print_section("Starting Development in Release Mode");
            std::env::set_var("RUST_LOG", "info");
            print_info("Building optimized binaries...");
            run_cargo_streaming(&["build", "--bin", "server", "--release"])?;
            run_cargo_streaming(&["build", "--bin", "client", "--release"])?;
            println!();
            print_success("Release build complete");
            print_info("Starting optimized server...");
            run_cargo_streaming(&["run", "--bin", "server", "--release"])?;
        }
        DevCommand::Clean => {
            print_section("Cleaning Development Environment");
            print_info("1. Cleaning build artifacts...");
            run_cargo_streaming(&["clean"])?;
            println!();
            print_success("Development environment cleaned");
        }
        DevCommand::Status => {
            print_section("Development Environment Status");
            print_info("Checking project compilation...");
            match run_cargo(&["check", "--all-targets"]) {
                Ok(_) => print_success("Project compiles successfully"),
                Err(_) => print_error("Project has compilation errors"),
            }
        }
        DevCommand::StopAll => {
            print_section("Stopping All Development Processes");
            print_warning("Process management not yet implemented");
        }
        DevCommand::Benchmark => {
            print_section("Running Quick Benchmarks");
            let benchmarks = [
                ("ECS World", "engine-core", "world_benches"),
                ("ECS Query", "engine-core", "query_benches"),
                ("Math SIMD", "engine-math", "simd_benches"),
            ];

            for (name, package, bench) in benchmarks {
                print_info(&format!("Running: {}", name));
                run_cargo_streaming(&[
                    "bench",
                    "--package",
                    package,
                    "--bench",
                    bench,
                    "--",
                    "--sample-size",
                    "10",
                    "--warm-up-time",
                    "1",
                    "--measurement-time",
                    "3",
                ])?;
                println!();
            }

            print_success("Quick benchmarks complete");
            print_info("Full report: target/criterion/report/index.html");
        }
        DevCommand::Trace => {
            print_section("Starting Development with Tracing");
            print_warning("Chrome trace export not yet implemented");
            print_info("Starting with TRACE level logging instead...");
            std::env::set_var("RUST_LOG", "trace");
            run_cargo_streaming(&["run", "--bin", "server"])?;
        }
        DevCommand::Validation => {
            print_section("Starting Development with Validation Layers");
            std::env::set_var("RUST_LOG", "debug");

            #[cfg(target_os = "windows")]
            {
                std::env::set_var("VK_LAYER_PATH", "C:\\VulkanSDK\\Bin");
            }
            #[cfg(target_os = "linux")]
            {
                std::env::set_var("VK_LAYER_PATH", "/usr/share/vulkan/explicit_layer.d");
            }
            #[cfg(target_os = "macos")]
            {
                std::env::set_var("VK_LAYER_PATH", "/usr/local/share/vulkan/explicit_layer.d");
            }

            std::env::set_var("VK_INSTANCE_LAYERS", "VK_LAYER_KHRONOS_validation");

            print_success("Vulkan validation layers enabled");
            print_warning("Performance will be slower");
            println!();

            run_cargo_streaming(&["run", "--bin", "client"])?;
        }
        DevCommand::Metrics => {
            print_section("Starting Development with Metrics");
            print_warning("Metrics dashboard not yet implemented (Phase 3)");
            print_info("Starting normal development mode...");
            run_cargo_streaming(&["run", "--bin", "server"])?;
        }
        DevCommand::Multi { count } => {
            let count = count.unwrap_or(2);
            print_section(&format!("Starting {} Clients + 1 Server", count));
            print_warning("Multi-client orchestration not yet implemented");
            print_info("Starting single server instead...");
            run_cargo_streaming(&["run", "--bin", "server"])?;
        }
        DevCommand::Headless => {
            print_section("Starting Headless Development");
            std::env::set_var("RUST_LOG", "info");
            print_info("Building with headless support...");
            run_cargo_streaming(&["build", "--bin", "client", "--features", "headless"])?;
            println!();
            print_success("Starting headless client");
            run_cargo_streaming(&["run", "--bin", "client", "--features", "headless"])?;
        }
    }
    Ok(())
}
