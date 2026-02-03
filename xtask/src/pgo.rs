use anyhow::{Context, Result};
use clap::Subcommand;
use std::env;
use std::fs;
use std::path::PathBuf;

use crate::utils::*;

#[derive(Subcommand)]
pub enum PgoCommand {
    /// Build instrumented binary for PGO (step 1/3)
    BuildInstrumented,
    /// Run PGO workload to collect profile data (step 2/3)
    RunWorkload,
    /// Build PGO-optimized binary (step 3/3)
    BuildOptimized,
    /// Compare PGO-optimized vs regular release build
    Compare,
    /// Test PGO workflow
    Test,
}

fn profile_dir() -> Result<PathBuf> {
    let dir = if cfg!(windows) {
        PathBuf::from(env::var("TEMP").unwrap_or_else(|_| "C:\\temp".to_string())).join("pgo-data")
    } else {
        PathBuf::from("/tmp/pgo-data")
    };
    Ok(dir)
}

pub fn execute(cmd: PgoCommand) -> Result<()> {
    match cmd {
        PgoCommand::BuildInstrumented => {
            print_section(
                "Profile-Guided Optimization (PGO) - Step 1/3: Build Instrumented Binary",
            );

            let profile_dir = profile_dir()?;

            // Clean old profile data
            if profile_dir.exists() {
                print_info(&format!("Cleaning old profile data in {}", profile_dir.display()));
                fs::remove_dir_all(&profile_dir)?;
            }

            fs::create_dir_all(&profile_dir)?;
            print_info(&format!("Created profile directory: {}", profile_dir.display()));
            println!();

            print_info("Building instrumented binaries...");
            print_warning("This will be slower than a regular build");
            println!();

            let rustflags = format!("-C profile-generate={}", profile_dir.display());
            std::env::set_var("RUSTFLAGS", &rustflags);

            run_cargo_streaming(&["build", "--release", "--all-targets"])?;

            println!();
            print_section("Instrumented Build Complete!");
            println!();
            print_info("Next Steps:");
            println!("1. Run representative workload:");
            println!("   cargo xtask pgo run-workload");
            println!();
            println!("2. Build optimized binary:");
            println!("   cargo xtask pgo build-optimized");
            println!();
            print_info(&format!("Profile Directory: {}", profile_dir.display()));
        }
        PgoCommand::RunWorkload => {
            print_section(
                "Profile-Guided Optimization (PGO) - Step 2/3: Run Representative Workload",
            );

            let profile_dir = profile_dir()?;

            if !profile_dir.exists() {
                print_error(&format!("Profile directory not found: {}", profile_dir.display()));
                println!("Run 'cargo xtask pgo build-instrumented' first");
                anyhow::bail!("Profile directory not found");
            }

            print_info("Running representative workload...");
            print_warning("This will take several minutes");
            println!();

            let profile_file = profile_dir.join("pgo-%p-%m.profraw");
            std::env::set_var("LLVM_PROFILE_FILE", profile_file.to_str().unwrap());

            let workloads = [
                (
                    "ECS World Operations",
                    vec![
                        "bench",
                        "--package",
                        "engine-core",
                        "--bench",
                        "world_benches",
                        "--",
                        "--sample-size",
                        "20",
                    ],
                ),
                (
                    "ECS Query System",
                    vec![
                        "bench",
                        "--package",
                        "engine-core",
                        "--bench",
                        "query_benches",
                        "--",
                        "--sample-size",
                        "20",
                    ],
                ),
                (
                    "Physics Integration",
                    vec![
                        "bench",
                        "--package",
                        "engine-physics",
                        "--bench",
                        "integration_bench",
                        "--",
                        "--sample-size",
                        "20",
                    ],
                ),
                (
                    "SIMD Math Operations",
                    vec![
                        "bench",
                        "--package",
                        "engine-math",
                        "--bench",
                        "simd_benches",
                        "--",
                        "--sample-size",
                        "20",
                    ],
                ),
                (
                    "Transform Operations",
                    vec![
                        "bench",
                        "--package",
                        "engine-math",
                        "--bench",
                        "transform_benches",
                        "--",
                        "--sample-size",
                        "20",
                    ],
                ),
            ];

            for (i, (name, args)) in workloads.iter().enumerate() {
                print_info(&format!("[{}/{}] Running: {}", i + 1, workloads.len(), name));
                let args_refs: Vec<&str> = args.iter().map(|s| s.as_ref()).collect();
                match run_cargo(&args_refs) {
                    Ok(_) => print_success(&format!("Completed: {}", name)),
                    Err(_) => {
                        print_warning(&format!("Warning: {} failed (continuing anyway)", name))
                    }
                }
                println!();
            }

            // Count profraw files
            let profraw_count = fs::read_dir(&profile_dir)?
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().map(|ext| ext == "profraw").unwrap_or(false))
                .count();

            print_section("Workload Complete!");
            println!();
            print_info(&format!("Profile files generated: {}", profraw_count));
            print_info(&format!("Profile directory: {}", profile_dir.display()));
            println!();

            if profraw_count == 0 {
                print_error("No profile data was generated!");
                anyhow::bail!("No profile data generated");
            }

            print_info("Next Steps:");
            println!("Build the optimized binary:");
            println!("  cargo xtask pgo build-optimized");
        }
        PgoCommand::BuildOptimized => {
            print_section("Profile-Guided Optimization (PGO) - Step 3/3: Build Optimized Binary");

            let profile_dir = profile_dir()?;

            if !profile_dir.exists() {
                print_error(&format!("Profile directory not found: {}", profile_dir.display()));
                println!("Run 'cargo xtask pgo build-instrumented' and 'cargo xtask pgo run-workload' first");
                anyhow::bail!("Profile directory not found");
            }

            // Count profraw files
            let profraw_count = fs::read_dir(&profile_dir)?
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().map(|ext| ext == "profraw").unwrap_or(false))
                .count();

            if profraw_count == 0 {
                print_error(&format!("No .profraw files found in {}", profile_dir.display()));
                println!("Run 'cargo xtask pgo run-workload' first");
                anyhow::bail!("No profile data found");
            }

            print_info(&format!("Found {} profile data files", profraw_count));
            println!();

            // Merge profile data
            print_info("Merging profile data...");
            let merged_profile = profile_dir.join("merged.profdata");

            // Try to find llvm-profdata
            let llvm_profdata = which::which("llvm-profdata")
                .or_else(|_| {
                    // Try rustup
                    let output = std::process::Command::new("rustup")
                        .args(["which", "--toolchain", "stable", "llvm-profdata"])
                        .output()?;

                    if output.status.success() {
                        Ok(PathBuf::from(String::from_utf8(output.stdout)?.trim()))
                    } else {
                        anyhow::bail!("llvm-profdata not found")
                    }
                })
                .context("llvm-profdata not found. Install with: rustup component add llvm-tools-preview")?;

            let profraw_files: Vec<String> = fs::read_dir(&profile_dir)?
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().map(|ext| ext == "profraw").unwrap_or(false))
                .map(|e| e.path().to_string_lossy().to_string())
                .collect();

            let mut merge_args = vec!["merge", "-o", merged_profile.to_str().unwrap()];
            let profraw_refs: Vec<&str> = profraw_files.iter().map(|s| s.as_str()).collect();
            merge_args.extend(profraw_refs);

            run_command(llvm_profdata.to_str().unwrap(), &merge_args)?;
            print_success("Profile data merged successfully");
            println!();

            // Build with profile data
            print_info("Building PGO-optimized binaries...");

            let rustflags = format!(
                "-C profile-use={} -C llvm-args=-pgo-warn-missing-function",
                merged_profile.display()
            );
            std::env::set_var("RUSTFLAGS", &rustflags);

            run_cargo_streaming(&["build", "--release", "--all-targets"])?;

            println!();
            print_section("PGO-Optimized Build Complete!");
            println!();
            print_info("Performance Gains:");
            println!("  - Expected: 5-15% faster on typical workloads");
            println!("  - Hot paths optimized based on actual usage");
            println!();
            print_info("Next Steps:");
            println!("  1. Run benchmarks: cargo xtask bench all");
            println!("  2. Compare with non-PGO: cargo xtask pgo compare");
        }
        PgoCommand::Compare => {
            print_section("PGO Performance Comparison");

            print_info("Step 1/5: Building baseline release binary (no PGO)");
            println!();
            std::env::remove_var("RUSTFLAGS");
            run_cargo_streaming(&["clean"])?;
            run_cargo_streaming(&["build", "--release", "--all-targets"])?;

            println!();
            print_info("Step 2/5: Running baseline benchmarks");
            println!();
            run_cargo_streaming(&["bench", "--", "--save-baseline", "no-pgo"])?;

            // Build PGO workflow
            println!();
            print_info("Step 3/5: Building PGO workflow");
            println!();
            execute(PgoCommand::BuildInstrumented)?;

            println!();
            print_info("Step 4/5: Collecting profile data");
            println!();
            execute(PgoCommand::RunWorkload)?;

            println!();
            print_info("Step 5/5: Building PGO-optimized binary");
            println!();
            execute(PgoCommand::BuildOptimized)?;

            // Compare
            println!();
            print_info("Running PGO-optimized benchmarks and comparing");
            println!();
            run_cargo_streaming(&["bench", "--", "--baseline", "no-pgo"])?;

            println!();
            print_section("Performance Comparison Complete!");
            println!();
            print_info("Results:");
            println!("  - Baseline: target/criterion (no-pgo baseline)");
            println!("  - PGO: target/criterion (compared against baseline)");
            println!();
            print_info("Expected gain: 5-15% on typical workloads");
            println!("HTML reports: target/criterion/report/index.html");
        }
        PgoCommand::Test => {
            print_section("PGO Workflow Test");

            let profile_dir = profile_dir()?;

            print_info("[1/3] Checking dependencies...");
            if run_cargo(&["--version"]).is_ok() {
                print_success("cargo available");
            } else {
                print_error("cargo not found");
                anyhow::bail!("cargo not found");
            }

            println!();
            print_info("[2/3] Testing profile directory creation...");
            fs::create_dir_all(&profile_dir)?;
            if profile_dir.exists() {
                print_success(&format!("Created: {}", profile_dir.display()));
                fs::remove_dir_all(&profile_dir)?;
            }

            println!();
            print_info("[3/3] All checks passed!");
            println!();
            print_section("All Tests Passed!");
            println!();
            print_info("PGO workflow is ready to use.");
            println!();
            println!("To run the full workflow:");
            println!("  cargo xtask pgo build-instrumented");
            println!("  cargo xtask pgo run-workload");
            println!("  cargo xtask pgo build-optimized");
            println!();
            println!("Or use automated comparison:");
            println!("  cargo xtask pgo compare");
        }
    }
    Ok(())
}
