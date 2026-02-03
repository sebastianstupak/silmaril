use anyhow::Result;
use clap::Subcommand;

use crate::utils::*;

#[derive(Subcommand)]
pub enum BenchmarkCommand {
    /// Run all benchmarks
    All,
    /// Run all benchmarks and save baseline
    AllSave,
    /// Run platform-specific benchmarks only
    Platform,
    /// Run ECS benchmarks only
    Ecs,
    /// Run physics benchmarks
    Physics,
    /// Run renderer benchmarks
    Renderer,
    /// Run math benchmarks
    Math,
    /// Run profiling overhead benchmarks
    Profiling,
    /// Run industry comparison benchmarks
    Compare,
    /// Compare current benchmarks with saved baseline
    Baseline,
    /// Save current benchmarks as main baseline
    SaveBaseline,
    /// Run quick benchmark smoke test (fast, for CI)
    Smoke,
    /// Run benchmarks with profiling enabled
    Profile,
    /// Open benchmark report in browser
    View,
    /// Network benchmarks (when implemented)
    Network,
    /// Run serialization benchmarks
    Serialization,
    /// Run asset loading benchmarks
    Assets,
    /// Run asset system industry comparison benchmarks
    AssetsCompare,
    /// Run spatial data structure benchmarks
    Spatial,
    /// Run allocator benchmarks
    Allocators,
    /// Run audio system benchmarks
    Audio,
}

pub fn execute(cmd: BenchmarkCommand) -> Result<()> {
    match cmd {
        BenchmarkCommand::All => {
            print_section("Running All Benchmarks");
            run_cargo_streaming(&["bench", "--all-features"])?;
            print_success("All benchmarks complete");
        }
        BenchmarkCommand::AllSave => {
            print_section("Running All Benchmarks (saving baseline)");
            run_cargo_streaming(&["bench", "--all-features", "--", "--save-baseline", "current"])?;
            print_success("Benchmarks complete, baseline saved");
        }
        BenchmarkCommand::Platform => {
            print_section("Running Platform Benchmarks");
            run_cargo_streaming(&[
                "bench",
                "--package",
                "engine-core",
                "--bench",
                "platform_benches",
            ])?;
            run_cargo_streaming(&[
                "bench",
                "--package",
                "engine-renderer",
                "--bench",
                "vulkan_context_bench",
            ])?;
            print_success("Platform benchmarks complete");
        }
        BenchmarkCommand::Ecs => {
            print_section("Running ECS Benchmarks");
            run_cargo_streaming(&["bench", "--package", "engine-core", "--bench", "ecs_simple"])?;
            run_cargo_streaming(&[
                "bench",
                "--package",
                "engine-core",
                "--bench",
                "ecs_comprehensive",
            ])?;
            run_cargo_streaming(&[
                "bench",
                "--package",
                "engine-core",
                "--bench",
                "query_benches",
            ])?;
            run_cargo_streaming(&[
                "bench",
                "--package",
                "engine-core",
                "--bench",
                "world_benches",
            ])?;
            print_success("ECS benchmarks complete");
        }
        BenchmarkCommand::Physics => {
            print_section("Running Physics Benchmarks");
            run_cargo_streaming(&["bench", "--package", "engine-physics"])?;
            print_success("Physics benchmarks complete");
        }
        BenchmarkCommand::Renderer => {
            print_section("Running Renderer Benchmarks");
            run_cargo_streaming(&["bench", "--package", "engine-renderer"])?;
            print_success("Renderer benchmarks complete");
        }
        BenchmarkCommand::Math => {
            print_section("Running Math Benchmarks");
            run_cargo_streaming(&["bench", "--package", "engine-math"])?;
            print_success("Math benchmarks complete");
        }
        BenchmarkCommand::Profiling => {
            print_section("Running Profiling Overhead Benchmarks");
            run_cargo_streaming(&["bench", "--package", "engine-profiling"])?;
            print_success("Profiling benchmarks complete");
        }
        BenchmarkCommand::Compare => {
            print_section("Running Industry Comparison Benchmarks");
            run_cargo_streaming(&[
                "bench",
                "--package",
                "engine-core",
                "--bench",
                "game_engine_comparison",
            ])?;
            print_success("Comparison benchmarks complete");
        }
        BenchmarkCommand::Baseline => {
            print_section("Comparing with Baseline");
            run_cargo_streaming(&["bench", "--all-features", "--", "--baseline", "current"])?;
            print_success("Baseline comparison complete");
        }
        BenchmarkCommand::SaveBaseline => {
            print_section("Saving Benchmark Baseline");
            run_cargo_streaming(&["bench", "--all-features", "--", "--save-baseline", "main"])?;
            print_success("Baseline saved");
        }
        BenchmarkCommand::Smoke => {
            print_section("Running Smoke Tests");
            run_cargo_streaming(&[
                "bench",
                "--package",
                "engine-core",
                "--bench",
                "ecs_simple",
                "--",
                "--sample-size",
                "10",
            ])?;
            print_success("Smoke tests complete");
        }
        BenchmarkCommand::Profile => {
            print_section("Running Benchmarks with Profiling");
            run_cargo_streaming(&["bench", "--all-features", "--features", "profiling-puffin"])?;
            print_success("Profiled benchmarks complete");
        }
        BenchmarkCommand::View => {
            print_section("Opening Benchmark Report");
            let report_path = "target/criterion/report/index.html";

            #[cfg(target_os = "windows")]
            {
                run_command_streaming("cmd", &["/C", "start", report_path])?;
            }
            #[cfg(target_os = "macos")]
            {
                run_command_streaming("open", &[report_path])?;
            }
            #[cfg(target_os = "linux")]
            {
                run_command_streaming("xdg-open", &[report_path])?;
            }

            print_success("Opening benchmark report");
        }
        BenchmarkCommand::Network => {
            print_section("Running Network Benchmarks");
            run_cargo_streaming(&["bench", "--package", "engine-networking"])?;
            print_success("Network benchmarks complete");
        }
        BenchmarkCommand::Serialization => {
            print_section("Running Serialization Benchmarks");
            run_cargo_streaming(&[
                "bench",
                "--package",
                "engine-core",
                "--bench",
                "serialization_benches",
            ])?;
            run_cargo_streaming(&[
                "bench",
                "--package",
                "engine-core",
                "--bench",
                "serialization_comprehensive",
            ])?;
            print_success("Serialization benchmarks complete");
        }
        BenchmarkCommand::Assets => {
            print_section("Running Asset Benchmarks");
            print_info("Running all asset benchmarks (15 benchmark suites)");
            println!();
            println!("📦 Benchmarks included:");
            println!("  • Asset Handle System (creation, cloning, ref counting)");
            println!("  • Asset Loading (sync, async, streaming)");
            println!("  • Memory Management (LRU cache, budgets, eviction)");
            println!("  • Hot-Reload (file watching, debouncing, batching)");
            println!("  • Network Transfer (compression, checksums, chunking)");
            println!("  • Manifest & Bundles (YAML, Bincode, packing)");
            println!("  • Validation (format, data integrity, checksums)");
            println!("  • Procedural Generation (meshes, textures, audio)");
            println!();

            run_cargo_streaming(&["bench", "--package", "engine-assets", "--all-features"])?;

            println!();
            print_info("💡 Tip: Run 'cargo xtask bench assets-compare' for industry comparisons");
            print_success("Asset benchmarks complete");
        }
        BenchmarkCommand::AssetsCompare => {
            print_section("Running Asset System Industry Comparison");
            print_info("Comparing Silmaril's asset system against:");
            println!("  • Unity (AssetDatabase, AssetBundles)");
            println!("  • Unreal Engine (AssetRegistry, Pak files)");
            println!("  • Godot (ResourceLoader, PCK files)");
            println!("  • Bevy (AssetServer, hot-reload)");
            println!();

            print_info("Performance targets:");
            println!("  ✓ Asset loading: < 5ms (Unity parity)");
            println!("  ✓ Hot-reload: < 100ms (2-5x faster)");
            println!("  ✓ Memory overhead: < 100 bytes/asset");
            println!("  ✓ Network transfer: > 50 MB/s");
            println!("  ✓ Bundle packing: > 100 MB/s");
            println!();

            run_cargo_streaming(&[
                "bench",
                "--package",
                "engine-assets",
                "--bench",
                "industry_comparison",
                "--all-features",
            ])?;

            println!();
            print_success("Industry comparison complete");
            print_info(
                "📊 View detailed report: target/criterion/industry_comparison/report/index.html",
            );
        }
        BenchmarkCommand::Spatial => {
            print_section("Running Spatial Benchmarks");
            run_cargo_streaming(&[
                "bench",
                "--package",
                "engine-core",
                "--bench",
                "spatial_benches",
            ])?;
            print_success("Spatial benchmarks complete");
        }
        BenchmarkCommand::Allocators => {
            print_section("Running Allocator Benchmarks");
            run_cargo_streaming(&[
                "bench",
                "--package",
                "engine-core",
                "--bench",
                "allocator_benches",
            ])?;
            print_success("Allocator benchmarks complete");
        }
        BenchmarkCommand::Audio => {
            print_section("Running Audio Benchmarks");
            println!("Performance targets: audio_update < 0.5ms, play_sound < 0.1ms, 3d_position_update < 0.05ms");
            run_cargo_streaming(&[
                "bench",
                "--package",
                "engine-audio",
                "--bench",
                "audio_benches",
            ])?;
            print_success("Audio benchmarks complete");
        }
    }
    Ok(())
}
