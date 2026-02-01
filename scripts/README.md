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

All `.sh` and `.ps1` scripts have been converted to `just` recipes:

| Old Script | New Just Recipe |
|------------|----------------|
| `benchmark_all_platforms.sh` | `just bench-all-platforms` |
| `update_benchmark_baseline.sh` | `just bench-update-baseline` |
| `compare_with_baseline.sh` | `just bench-compare-baseline` |
| `benchmark_tiers.sh` | `just benchmark-tiers` |
| `build_all_tiers.sh` | `just build-all-tiers` |
| `build_pgo_instrumented.sh` | `just pgo-build-instrumented` |
| `build_pgo_optimized.sh` | `just pgo-build-optimized` |
| `run_pgo_workload.sh` | `just pgo-run-workload` |
| `compare_pgo_performance.sh` | `just pgo-compare` |
| `test_pgo_workflow.sh` | `just pgo-test` |
| `setup-hooks.sh` | `just setup-hooks` |
| `verify_build_tiers.sh` | `just verify-build-tiers` |
| `validate_component_get_optimization.sh` | `just validate-component-optimization` |
| `verify_physics_optimization.sh` | `just verify-physics-optimization` |
| `test_linux_optimizations.sh` | (Linux-specific, kept as-is) |

**Benefits of `just`:**
- ✅ True cross-platform support (Windows, Linux, macOS)
- ✅ Python scripts for complex logic (no shell-specific syntax)
- ✅ Consistent command interface
- ✅ Parameterizable recipes
- ✅ Built-in help system (`just --list`)
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
- Profile with: `just bench-profile`
- Optimize hot paths
- Re-run: `just bench-compare-baseline`

---

## 📖 See Also

- [Development Workflow Documentation](../docs/development-workflow.md)
- [Coding Standards](../docs/rules/coding-standards.md)
- [Profiling Guide](../docs/profiling.md)
- [Performance Targets](../docs/performance-targets.md)
- [Just Command Runner](https://github.com/casey/just)
