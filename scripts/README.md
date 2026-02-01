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

## Benchmark Regression Checking

### check_benchmark_regression.py

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

**Local Testing:**
```bash
# Run Criterion benchmarks with baseline
cargo bench --features profiling-puffin -- --save-baseline main

# Make changes to code...

# Run benchmarks again
cargo bench --features profiling-puffin -- --save-baseline pr

# Compare manually
cargo bench --features profiling-puffin -- --baseline main

# Or use the script
python scripts/check_benchmark_regression.py \
  --baseline target/criterion/main \
  --current target/criterion/pr \
  --threshold 20 \
  --format criterion
```

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
