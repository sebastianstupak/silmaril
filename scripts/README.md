# Development Scripts

This directory contains scripts for development workflow automation.

## ⚡ Quick Start - Using `just`

Most scripts have been converted to `just` recipes for true cross-platform support.

**Install just:**
```bash
cargo install just
```

**View all available commands:**
```bash
just --list
```

**Run a command:**
```bash
just bench
just build-all-tiers mode=release
just pgo-compare
```

See the main [justfile](../justfile) for all available recipes.

---

## 📋 Command Reference

### Test Coverage and Validation

```bash
# Generate coverage report
./scripts/coverage.sh              # Linux/macOS
.\scripts\coverage.ps1             # Windows

# Run all benchmarks
./scripts/benchmark_all.sh         # Linux/macOS
.\scripts\benchmark_all.ps1        # Windows

# Quick mode for CI
./scripts/benchmark_all.sh --quick

# Validate performance targets
./scripts/check_performance_targets.sh    # Linux/macOS
.\scripts\check_performance_targets.ps1   # Windows
```

### Benchmark Management

```bash
# Run comprehensive benchmark suite
just bench-all-platforms

# Quick mode for development
just bench-all-platforms quick=true

# Update baseline for regression testing
just bench-update-baseline main

# Compare with baseline
just bench-compare-baseline main threshold=20
```

### Build Tiers (Multi-CPU Optimization)

```bash
# Build all tiers (baseline, modern, highend)
just build-all-tiers mode=release client=true server=true

# Benchmark all tiers
just benchmark-tiers

# Verify implementation
just verify-build-tiers

# Test CPU detection
cargo run --example cpu_tier_detection --package engine-build-utils
```

**Output:**
```
target/baseline/release/  - SSE2 only (100% compatible)
target/modern/release/    - AVX2 + FMA (95% compatible, 15-30% faster)
target/highend/release/   - AVX512 (70% compatible, 20-50% faster)
```

### Profile-Guided Optimization (PGO)

```bash
# Step-by-step workflow
just pgo-build-instrumented  # Step 1: Build instrumented binary
just pgo-run-workload        # Step 2: Collect profile data
just pgo-build-optimized     # Step 3: Build optimized binary

# Or run automated comparison
just pgo-compare

# Test PGO workflow
just pgo-test
```

**Expected Gains:** 5-15% performance improvement

### Optimization Validation

```bash
# Validate component get() optimization
just validate-component-optimization

# Verify physics optimization
just verify-physics-optimization
```

### Development Setup

```bash
# Setup git hooks and dev environment
just setup-hooks
```

**What it does:**
- Installs pre-commit hook
- Checks for optional development tools
- Displays setup confirmation

---

## 🧪 Test Coverage Scripts

### coverage.sh / coverage.ps1

Generate comprehensive test coverage reports using cargo-llvm-cov.

**Usage (Linux/macOS):**
```bash
./scripts/coverage.sh
```

**Usage (Windows):**
```powershell
.\scripts\coverage.ps1
```

**Features:**
- Automatic cargo-llvm-cov installation
- HTML coverage reports
- LCOV format for CI integration
- Per-module coverage breakdown
- Coverage target validation (80% overall)

**Output:**
- `coverage.lcov` - LCOV format for CI
- `coverage-html/index.html` - Interactive HTML report

### benchmark_all.sh / benchmark_all.ps1

Run comprehensive benchmark suite across all engine modules.

**Usage (Linux/macOS):**
```bash
./scripts/benchmark_all.sh [--save-baseline] [--compare-baseline] [--quick]
```

**Usage (Windows):**
```powershell
.\scripts\benchmark_all.ps1 [-SaveBaseline] [-CompareBaseline] [-Quick]
```

**Options:**
- `--save-baseline` / `-SaveBaseline`: Save results as new baseline
- `--compare-baseline` / `-CompareBaseline`: Compare with existing baseline
- `--baseline-name NAME` / `-BaselineName NAME`: Custom baseline name (default: "baseline")
- `--quick` / `-Quick`: Reduced sample size for faster runs

**Output:**
- `benchmark-results/benchmark-TIMESTAMP.txt` - Results file
- `benchmark-results/baseline-NAME.txt` - Baseline snapshots

### check_performance_targets.sh / check_performance_targets.ps1

Validate that benchmarks meet performance targets defined in `benchmark_thresholds.yaml`.

**Usage (Linux/macOS):**
```bash
./scripts/check_performance_targets.sh
```

**Usage (Windows):**
```powershell
.\scripts\check_performance_targets.ps1 [-Verbose]
```

**Features:**
- Automated performance regression detection
- Configurable thresholds per module
- CI-friendly exit codes (fail if targets not met)
- Clear pass/fail reporting

**Thresholds file:** `benchmark_thresholds.yaml`

```yaml
ecs:
  entity_spawn: 1000          # ns per entity
  world_update: 16000         # µs (60 FPS target)

serialization:
  bincode_roundtrip: 10000    # µs

physics:
  step_100_bodies: 2000       # µs per step
```

---

## 🐍 Python Scripts

These Python scripts are used by the justfile recipes and can be run directly:

### benchmark_regression_check.py

Detect performance regressions by comparing baseline and current benchmarks.

**Usage:**
```bash
python scripts/benchmark_regression_check.py \
  --baseline benchmarks/baselines/windows_main/criterion \
  --current target/criterion \
  --threshold 20 \
  --format criterion \
  --fail-on-regression
```

**Options:**
- `--baseline DIR`: Path to baseline benchmark directory (required)
- `--current DIR`: Path to current benchmark directory (required)
- `--threshold PERCENT`: Regression threshold percentage (required)
- `--format {criterion,iai}`: Benchmark format (default: criterion)
- `--output FILE`: Generate markdown report
- `--fail-on-regression`: Exit with error code if regressions detected
- `--show-all`: Show all benchmarks, not just changes

**Thresholds:**
- **Iai benchmarks**: 10% (deterministic instruction counts)
- **Criterion benchmarks**: 20% (wall-clock time with CI noise tolerance)

### compare_with_industry.py

Compare benchmark results against industry standards and performance targets.

**Usage:**
```bash
python scripts/compare_with_industry.py --results <results_dir>
```

**Features:**
- Compares against Unity, Unreal, Godot, Bevy baselines
- Generates performance assessment (✅ Excellent, ✓ Good, ⚠️ Acceptable, ❌ Poor)
- Provides optimization recommendations
- No external dependencies (Python stdlib only)

### check_benchmark_regression.py

Legacy regression checker (use `benchmark_regression_check.py` instead).

**Usage:**
```bash
python scripts/check_benchmark_regression.py \
  --baseline target/criterion-baseline \
  --current target/criterion \
  --threshold 20 \
  --format criterion
```

---

## 🔄 Migration from Shell Scripts

All `.sh` and `.ps1` scripts have been converted to `cargo xtask` commands:

| Old Script | New XTask Command |
|------------|-------------------|
| `benchmark_all_platforms.sh` | `cargo xtask bench all-platforms` |
| `update_benchmark_baseline.sh` | `cargo xtask bench update-baseline` |
| `compare_with_baseline.sh` | `cargo xtask bench baseline` |
| `benchmark_tiers.sh` | `cargo xtask bench tiers` |
| `build_all_tiers.sh` | `cargo xtask build tiers` |
| `build_pgo_instrumented.sh` | `cargo xtask pgo build-instrumented` |
| `build_pgo_optimized.sh` | `cargo xtask pgo build-optimized` |
| `run_pgo_workload.sh` | `cargo xtask pgo run-workload` |
| `compare_pgo_performance.sh` | `cargo xtask pgo compare` |
| `test_pgo_workflow.sh` | `cargo xtask pgo test` |
| `setup-hooks.sh` | `cargo xtask setup hooks` |
| `verify_build_tiers.sh` | `cargo xtask verify build-tiers` |
| `validate_component_get_optimization.sh` | `cargo xtask verify component-optimization` |
| `verify_physics_optimization.sh` | `cargo xtask verify physics-optimization` |
| `test_linux_optimizations.sh` | (Linux-specific, kept as-is) |

**Benefits of `cargo xtask`:**
- ✅ True cross-platform support (Windows, Linux, macOS)
- ✅ No external dependencies (just cargo)
- ✅ Python scripts for complex logic (no shell-specific syntax)
- ✅ Consistent command interface
- ✅ Built-in help system (`cargo xtask --help`)
- ✅ No need for separate `.sh` and `.ps1` versions

---

## 🔧 Optional Development Tools

Install these for enhanced development experience:

```bash
# Dependency auditing and policy enforcement
cargo install cargo-deny

# Auto-rebuild on file changes
cargo install cargo-watch

# CPU profiling with flamegraphs
cargo install flamegraph

# Outdated dependency checker
cargo install cargo-outdated
```

---

## 🐧 Linux-Specific Testing

### test_linux_optimizations.sh

Test Linux-specific platform optimizations (vDSO, filesystem, threading).

**Usage:**
```bash
./scripts/test_linux_optimizations.sh [--quick] [--bench-only] [--baseline] [--compare]
```

**Options:**
- `--quick`: Run tests only (skip benchmarks)
- `--bench-only`: Run benchmarks only (skip tests)
- `--baseline`: Save benchmark baseline
- `--compare`: Compare with saved baseline
- `--verbose`: Enable verbose output

**Requirements:**
- Linux kernel 2.6.32+ (for vDSO support)
- Rust toolchain
- Optional: cargo-criterion for detailed reports

**What it tests:**
- vDSO-accelerated clock_gettime (<30ns target)
- Fast path normalization (<200ns for simple paths)
- CPU affinity and thread priority
- SCHED_BATCH scheduling policy

---

## 📚 Cross-Platform Examples

### Run all benchmarks (all platforms)
```bash
just bench
```

### Build release binaries (all platforms)
```bash
just build-release
```

### Run tests (all platforms)
```bash
just test
```

### Format and lint (all platforms)
```bash
just fmt
just clippy
```

### Complete check before commit (all platforms)
```bash
just check  # Runs fmt-check, clippy, and tests
```

---

## 🎯 Common Workflows

### Before committing
```bash
just check  # Format, lint, test
```

### After optimizations
```bash
# Save baseline before changes
just bench-update-baseline before

# Make optimizations...

# Compare performance
just bench-compare-baseline before
```

### For release builds
```bash
# Option 1: Standard release
just build-release

# Option 2: PGO-optimized release (5-15% faster)
just pgo-compare

# Option 3: Multi-tier release (target-specific optimization)
just build-all-tiers mode=release
```

---

## 🆘 Troubleshooting

### `just` command not found
```bash
cargo install just
```

### Python3 not found
- **Windows**: Install from [python.org](https://www.python.org)
- **Linux**: `sudo apt install python3` or `sudo yum install python3`
- **macOS**: `brew install python3`

### Recipe fails with "Permission denied"
On Unix-like systems, some operations may require permissions:
```bash
chmod +x justfile
```

### Benchmark regression check fails
This is expected! It means performance regressed.
- Review the regressions
- Profile with: `cargo xtask bench profile`
- Optimize hot paths
- Re-run: `cargo xtask bench baseline`

---

## 📖 See Also

- [Development Workflow Documentation](../docs/development-workflow.md)
- [Coding Standards](../docs/rules/coding-standards.md)
- [Profiling Guide](../docs/profiling.md)
- [Performance Targets](../docs/performance-targets.md)
- [Just Command Runner](https://github.com/casey/just)
