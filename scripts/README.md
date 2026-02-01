# Development Scripts

This directory contains scripts for development workflow automation.

## Platform Testing

### Linux Platform Optimizations

Test Linux-specific platform optimizations (time backend, filesystem, threading):

```bash
./scripts/test_linux_optimizations.sh
```

**Options:**
- `--quick`: Run tests only (skip benchmarks)
- `--bench-only`: Run benchmarks only (skip tests)
- `--baseline`: Save benchmark baseline
- `--compare`: Compare with saved baseline
- `--verbose`: Enable verbose output
- `--help`: Show help message

**Requirements:**
- Linux kernel 2.6.32+ (for vDSO support)
- Rust toolchain
- Optional: cargo-criterion for detailed reports

**What it tests:**
- vDSO-accelerated clock_gettime (<30ns target)
- Fast path normalization (<200ns for simple paths)
- CPU affinity and thread priority
- SCHED_BATCH scheduling policy

**Results:** See `LINUX_OPTIMIZATION_RESULTS.md` for detailed documentation.

## Multi-Tier Build System

The engine supports building multiple optimized binaries for different CPU capabilities (Task #59).

### Build All Tiers

Build separate binaries optimized for baseline, modern, and high-end CPUs:

**Linux/macOS:**
```bash
./scripts/build_all_tiers.sh --release --both
```

**Windows (PowerShell):**
```powershell
.\scripts\build_all_tiers.ps1 -Release -Both
```

**Options:**
- `--release` / `-Release`: Build in release mode (default: debug)
- `--client` / `-Client`: Build client binary only
- `--server` / `-Server`: Build server binary only
- `--both` / `-Both`: Build both client and server

**Output:**
```
target/baseline/release/  - SSE2 only (100% compatible)
target/modern/release/    - AVX2 + FMA (95% compatible, 15-30% faster)
target/highend/release/   - AVX512 (70% compatible, 20-50% faster)
```

### Benchmark Tiers

Compare performance across all tiers:

```bash
./scripts/benchmark_tiers.sh --verbose
```

This runs all benchmarks for each tier and reports performance gains.

### CPU Tier Detection

Test runtime CPU feature detection:

```bash
cargo run --example cpu_tier_detection --package engine-build-utils
```

Shows:
- Detected CPU tier for your machine
- Individual feature support (SSE4.2, AVX2, FMA, AVX512)
- Which binaries will run on your CPU
- Recommended binary to use

**Example output:**
```
Detected Tier: modern (x86-64-v3 with AVX2+FMA)
Performance: 125% of native

SIMD Feature Support:
  SSE2:    ✓ (required for x86-64)
  AVX2:    ✓ (required for Modern tier)
  FMA:     ✓ (required for Modern tier)
  AVX512F: ✗ (required for High-end tier)

Recommended binary: modern
Expected performance: 125% of baseline
```

### Distribution

For end-user distribution:

1. Build all three tiers
2. Package all binaries in `bin/baseline/`, `bin/modern/`, `bin/highend/`
3. Create launcher that detects CPU and runs appropriate binary
4. Fallback to baseline if detection fails

See [docs/build-tiers.md](../docs/build-tiers.md) and [engine/math/CPU_FEATURES.md](../engine/math/CPU_FEATURES.md) for details.

## Setup Scripts

### setup-hooks.sh

Installs git pre-commit hooks and configures the development environment.

**Usage:**
```bash
./scripts/setup-hooks.sh
```

**What it does:**
- Installs pre-commit hook to `.git/hooks/pre-commit`
- Makes the hook executable
- Checks for optional development tools
- Displays installation confirmation

**Run this once after cloning the repository.**

## Git Hooks

### hooks/pre-commit

Pre-commit hook that runs automatically before each commit.

**Checks performed:**
1. Code formatting (`cargo fmt --check`)
2. Linting (`cargo clippy --all-targets -- -D warnings`)
3. Unit tests (`cargo test --lib`)
4. Dependency checks (`cargo deny check bans`, if installed)
5. Common issue detection:
   - `println!`/`eprintln!`/`dbg!` usage (should use `tracing` instead)
   - `anyhow::Result` usage (should use custom error types)
   - `Box<dyn Error>` usage (should use custom error types)

**Manual execution:**
```bash
.git/hooks/pre-commit
```

**Bypass (not recommended):**
```bash
git commit --no-verify
```

## Optional Development Tools

The scripts check for these optional tools:

- **cargo-deny**: Dependency auditing and policy enforcement
  ```bash
  cargo install cargo-deny
  ```

- **cargo-watch**: Auto-rebuild on file changes
  ```bash
  cargo install cargo-watch
  ```

- **cargo-flamegraph**: CPU profiling with flamegraphs
  ```bash
  cargo install flamegraph
  ```

## Troubleshooting

### Pre-commit hook fails

If the pre-commit hook fails, read the error messages carefully. They include:
- What check failed
- How to fix it (suggested commands)

Common fixes:
```bash
# Fix formatting
cargo fmt

# Fix clippy issues automatically
cargo clippy --fix --all-targets

# Run tests to see failures
cargo test --lib

# Check dependencies
cargo deny check bans
```

### Hook not running

Verify the hook is installed and executable:
```bash
ls -l .git/hooks/pre-commit
```

If missing, run setup again:
```bash
./scripts/setup-hooks.sh
```

### Permission denied

Make the hook executable:
```bash
chmod +x .git/hooks/pre-commit
```

## Cross-Platform Benchmark Automation

### benchmark_all_platforms.sh / .ps1

**Purpose:** Automated benchmark suite that runs all engine benchmarks and generates comprehensive reports.

**Features:**
- Runs all benchmark suites (Platform, ECS, Physics, Math, Serialization, Profiling)
- Saves results with timestamps for historical tracking
- Baseline management (save/compare)
- Integration with regression detection and industry comparison
- Cross-platform (Linux, macOS, Windows)

**Usage:**

**Linux/macOS:**
```bash
./scripts/benchmark_all_platforms.sh [OPTIONS]
```

**Windows (PowerShell):**
```powershell
.\scripts\benchmark_all_platforms.ps1 [OPTIONS]
```

**Options:**
- `--baseline NAME` / `-Baseline NAME`: Save results as named baseline
- `--compare NAME` / `-Compare NAME`: Compare with named baseline
- `--output DIR` / `-OutputDir DIR`: Custom output directory
- `--quick` / `-Quick`: Run subset of benchmarks (faster, for development)
- `--no-platform` / `-NoPlatform`: Skip platform-specific benchmarks
- `--no-ecs` / `-NoEcs`: Skip ECS benchmarks
- `--verbose` / `-Verbose`: Enable verbose output
- `--help` / `-Help`: Show help message

**Examples:**
```bash
# Run all benchmarks
./scripts/benchmark_all_platforms.sh

# Quick mode (development)
./scripts/benchmark_all_platforms.sh --quick

# Save as baseline
./scripts/benchmark_all_platforms.sh --baseline main

# Compare with baseline
./scripts/benchmark_all_platforms.sh --compare main

# Full workflow: optimize, save, and compare
./scripts/benchmark_all_platforms.sh --baseline after --compare before
```

**Output:**
- `benchmarks/results/<platform>_<timestamp>/` - Timestamped results directory
- `benchmarks/results/<platform>_<timestamp>/SUMMARY.md` - Summary report
- `benchmarks/results/<platform>_<timestamp>/*.log` - Individual benchmark logs
- `target/criterion/report/index.html` - Criterion HTML report

**Baseline Management:**

Baselines are saved in `benchmarks/baselines/<platform>_<name>/`:
```bash
# List available baselines
ls benchmarks/baselines/

# View baseline metadata
cat benchmarks/baselines/windows_main/metadata.json
```

**See:** `benchmarks/AUTOMATION.md` for complete documentation.

---

### compare_with_industry.py

**Purpose:** Compare benchmark results against industry standards and performance targets.

**Features:**
- Parses Criterion benchmark output
- Compares against industry baselines (Unity, Unreal, Godot, Bevy)
- Generates performance assessment (✅ Excellent, ✓ Good, ⚠️ Acceptable, ❌ Poor)
- Provides optimization recommendations
- No external dependencies (uses Python stdlib only)

**Usage:**
```bash
python scripts/compare_with_industry.py --results <results_dir> [OPTIONS]
```

**Options:**
- `--results DIR`: Path to results directory from benchmark_all_platforms.sh (required)
- `--output FILE`: Custom output markdown file (optional)
- `--criterion-dir DIR`: Custom Criterion output directory (default: target/criterion)

**Examples:**
```bash
# Compare after running benchmarks
python scripts/compare_with_industry.py \
  --results benchmarks/results/windows_20260201_120000

# Custom output file
python scripts/compare_with_industry.py \
  --results benchmarks/results/linux_20260201_130000 \
  --output custom_comparison.md

# View report
cat benchmarks/results/windows_20260201_120000/industry_comparison.md
```

**Output Report Includes:**
- Performance assessment for each benchmark
- Summary statistics (% meeting goals)
- High/medium priority optimization recommendations
- Comparison with industry baseline ranges

**Industry Targets:**

Targets are based on research from:
- Windows QPC, Linux clock_gettime (time queries)
- Unity, Unreal, Godot, Bevy (ECS performance)
- EnTT, Flecs, Hecs (ECS frameworks)

See `PLATFORM_BENCHMARK_COMPARISON.md` for detailed industry data.

---

### benchmark_regression_check.py

**Purpose:** Detect performance regressions by comparing baseline and current benchmark results.

**Features:**
- Supports Criterion (wall-clock time) and Iai-callgrind (instruction counts)
- Configurable regression threshold
- Detailed comparison tables and markdown reports
- CI integration (exit codes for automated checks)
- Shows regressions, improvements, and unchanged benchmarks

**Usage:**
```bash
python scripts/benchmark_regression_check.py \
  --baseline <baseline_dir> \
  --current <current_dir> \
  --threshold <percent> \
  --format <criterion|iai> \
  [OPTIONS]
```

**Options:**
- `--baseline DIR`: Path to baseline benchmark directory (required)
- `--current DIR`: Path to current benchmark directory (required)
- `--threshold PERCENT`: Regression threshold percentage (required)
- `--format {criterion,iai}`: Benchmark format (default: criterion)
- `--output FILE`: Generate markdown report
- `--fail-on-regression`: Exit with error code if regressions detected (for CI)
- `--show-all`: Show all benchmarks, not just changes

**Thresholds:**
- **Iai benchmarks**: 10% (deterministic instruction counts, strict)
- **Criterion benchmarks**: 20% (wall-clock time, tolerates CI VM noise)

**Examples:**
```bash
# Compare Criterion benchmarks
python scripts/benchmark_regression_check.py \
  --baseline benchmarks/baselines/windows_main/criterion \
  --current target/criterion \
  --threshold 20 \
  --format criterion

# Compare Iai benchmarks (Linux only, deterministic)
python scripts/benchmark_regression_check.py \
  --baseline benchmarks/baselines/linux_main/iai \
  --current target/iai \
  --threshold 10 \
  --format iai \
  --fail-on-regression

# Generate detailed markdown report
python scripts/benchmark_regression_check.py \
  --baseline benchmarks/baselines/windows_main/criterion \
  --current target/criterion \
  --threshold 20 \
  --output benchmarks/reports/regression.md \
  --show-all
```

**CI Integration:**

This script runs automatically in `.github/workflows/benchmark-regression.yml` on pull requests:
- Compares PR against main branch baseline
- Warns on 20% Criterion regression
- Fails on 10% Iai regression
- Archives results as artifacts

**Local Testing:**
```bash
# Simulate CI workflow
./scripts/benchmark_all_platforms.sh --baseline main
# ... make changes ...
./scripts/benchmark_all_platforms.sh --compare main

# Or use regression check directly
python scripts/benchmark_regression_check.py \
  --baseline benchmarks/baselines/<platform>_main/criterion \
  --current target/criterion \
  --threshold 20 \
  --fail-on-regression
```

---

## Benchmark Regression Checking (Legacy)

### check_benchmark_regression.py

**Note:** This is the original regression checker. Use `benchmark_regression_check.py` for enhanced functionality.

Python script that detects performance regressions in benchmark results.

**Supports:**
- Criterion benchmarks (wall-clock time)
- Iai-callgrind benchmarks (instruction counts)

**Usage:**
```bash
# Check Criterion benchmarks with 20% threshold
python scripts/check_benchmark_regression.py \
  --baseline target/criterion-baseline \
  --current target/criterion \
  --threshold 20 \
  --format criterion

# Check Iai benchmarks with 10% threshold
python scripts/check_benchmark_regression.py \
  --baseline /tmp/iai-baseline/iai \
  --current target/iai \
  --threshold 10 \
  --format iai
```

**Thresholds:**
- Iai benchmarks: 10% instruction count increase fails CI (deterministic)
- Criterion benchmarks: 20% time increase warns (CI noise tolerance)

**CI Integration:**

This script runs automatically in the `benchmark-regression.yml` workflow on pull requests.

## Profile-Guided Optimization (PGO)

### Overview

PGO is a compilation technique that optimizes code based on actual runtime behavior. It works in three steps:

1. **Build instrumented binary** - adds profiling instrumentation
2. **Run representative workload** - collects profile data
3. **Build optimized binary** - uses profile data for optimization

**Expected Gains:** 5-15% performance improvement on typical workloads.

### build_pgo_instrumented.sh

Builds a release binary instrumented to collect profiling data.

**Usage:**
```bash
./scripts/build_pgo_instrumented.sh [profile_dir]

# Example with custom profile directory
./scripts/build_pgo_instrumented.sh /tmp/my-pgo-data
```

**What it does:**
- Cleans old profile data
- Creates profile directory
- Builds with `-C profile-generate` flag
- Produces instrumented binaries in `target/release/`

**Important:** Instrumented binaries are slower than normal release builds. Only use them for profiling.

### run_pgo_workload.sh

Runs the instrumented binary through a representative workload to collect profile data.

**Usage:**
```bash
./scripts/run_pgo_workload.sh [profile_dir]
```

**Workloads included:**
- ECS world operations (spawn, despawn, add/remove components)
- ECS query system (single/multi-component, mutable queries)
- Physics integration (1K, 10K, 100K entities)
- SIMD math operations (vector ops, transforms)
- Typical game loop patterns

**Duration:** Several minutes (runs multiple benchmark suites)

**Output:** `.profraw` files in the profile directory

### build_pgo_optimized.sh

Builds the final optimized binary using collected profile data.

**Usage:**
```bash
./scripts/build_pgo_optimized.sh [profile_dir]
```

**What it does:**
- Merges `.profraw` files into `.profdata` using `llvm-profdata`
- Builds with `-C profile-use` flag
- Produces optimized binaries in `target/release/`

**Requirements:**
- `llvm-profdata` tool (install: `rustup component add llvm-tools-preview`)
- Profile data from previous steps

### compare_pgo_performance.sh

Automated script that runs the entire PGO workflow and compares performance.

**Usage:**
```bash
./scripts/compare_pgo_performance.sh
```

**What it does:**
1. Builds baseline (no PGO) and runs benchmarks
2. Runs full PGO workflow (instrument → profile → optimize)
3. Runs benchmarks on PGO-optimized build
4. Compares results using Criterion

**Output:**
- Console output showing performance differences
- HTML reports in `target/criterion/report/`
- Baseline saved for future comparisons

**Duration:** 15-30 minutes (full workflow + 2 benchmark runs)

### Complete PGO Workflow Example

```bash
# Option 1: Manual workflow (step-by-step)
./scripts/build_pgo_instrumented.sh
./scripts/run_pgo_workload.sh
./scripts/build_pgo_optimized.sh

# Option 2: Automated comparison
./scripts/compare_pgo_performance.sh

# View results
open target/criterion/report/index.html
```

### CI Integration

For release builds, PGO can be integrated into CI:

```yaml
# Example GitHub Actions workflow
- name: Build PGO-optimized release
  run: |
    ./scripts/build_pgo_instrumented.sh
    ./scripts/run_pgo_workload.sh
    ./scripts/build_pgo_optimized.sh
  if: github.ref == 'refs/tags/v*'
```

### Customizing the Workload

To add custom workloads to PGO profiling:

1. Add benchmarks to `engine/*/benches/`
2. Edit `run_pgo_workload.sh` to include new benchmarks
3. Ensure workloads are representative of production usage

**Example:**
```bash
# In run_pgo_workload.sh
run_workload "My Custom Workload" \
    "cargo bench --package my-crate --bench my_bench -- --sample-size 20"
```

### Troubleshooting

**No profile data generated:**
- Ensure instrumented binary was built correctly
- Check `LLVM_PROFILE_FILE` environment variable is set
- Verify benchmarks ran successfully

**llvm-profdata not found:**
```bash
# Install LLVM tools
rustup component add llvm-tools-preview

# Or use system llvm-profdata
# Ubuntu/Debian:
sudo apt install llvm

# macOS:
brew install llvm
```

**Profile data merge fails:**
- Check all `.profraw` files are valid
- Ensure profile directory path is correct
- Try deleting profile directory and starting over

**No performance improvement:**
- Verify workload is representative of production
- Check if hot paths are actually covered by profiling
- Some workloads may not benefit from PGO

## Adding New Scripts

When adding new development scripts:

1. Place them in this directory
2. Make them executable: `chmod +x script-name.sh`
3. Add documentation to this README
4. Update `docs/development-workflow.md` if user-facing

## See Also

- [Development Workflow Documentation](../docs/development-workflow.md)
- [Coding Standards](../docs/rules/coding-standards.md)
- [Error Handling Guide](../docs/error-handling.md)
- [Rust PGO Guide](https://doc.rust-lang.org/rustc/profile-guided-optimization.html)
