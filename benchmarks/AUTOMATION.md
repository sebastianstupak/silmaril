## Benchmark Automation Guide

This document describes the automated benchmark infrastructure for cross-platform performance testing, regression detection, and industry comparison.

---

## Directory Structure

```
benchmarks/
├── AUTOMATION.md       ← You are here
├── README.md           ← Game scenario benchmarks
├── baselines/          ← Saved baseline results for comparison
│   ├── windows_main/
│   │   ├── criterion/
│   │   ├── results/
│   │   └── metadata.json
│   ├── linux_main/
│   └── macos_main/
├── results/            ← Timestamped benchmark results
│   ├── windows_20260201_120000/
│   │   ├── *.log
│   │   ├── SUMMARY.md
│   │   └── industry_comparison.md
│   └── linux_20260201_130000/
└── reports/            ← Generated comparison reports
    └── *.md
```

---

## Quick Start

### Run All Benchmarks

**Linux/macOS:**
```bash
./scripts/benchmark_all_platforms.sh
```

**Windows:**
```powershell
.\scripts\benchmark_all_platforms.ps1
```

**View results:**
```bash
# Open HTML report
open target/criterion/report/index.html  # macOS
xdg-open target/criterion/report/index.html  # Linux
Start-Process target/criterion/report/index.html  # Windows PowerShell
```

### Save a Baseline

Save current results as a baseline for future comparisons:

**Linux/macOS:**
```bash
./scripts/benchmark_all_platforms.sh --baseline main
```

**Windows:**
```powershell
.\scripts\benchmark_all_platforms.ps1 -Baseline main
```

This creates: `benchmarks/baselines/<platform>_main/`

### Compare with Baseline

Compare current performance against a saved baseline:

**Linux/macOS:**
```bash
./scripts/benchmark_all_platforms.sh --compare main
```

**Windows:**
```powershell
.\scripts\benchmark_all_platforms.ps1 -Compare main
```

### Industry Comparison

Compare results against industry standards:

```bash
# After running benchmarks
python scripts/compare_with_industry.py --results benchmarks/results/windows_20260201_120000

# View report
cat benchmarks/results/windows_20260201_120000/industry_comparison.md
```

---

## Benchmark Suites

### Platform Abstraction
- **Time Backend**: `monotonic_nanos()`, `now()`, `sleep()`
- **Threading**: Thread priority, CPU affinity, core count
- **Filesystem**: Path normalization, file operations

**Target Performance:**
- Time queries: <50ns (goal: 30ns)
- Threading operations: <5-15μs
- Path normalization: <500ns simple, <2μs complex

### ECS
- **Entity Operations**: Spawn, despawn, add/remove components
- **Query System**: Single/multi-component queries, iteration
- **Storage**: Sparse set operations, archetype management

**Target Performance:**
- Entity spawn: <500ns
- Query iteration: <50ns per entity (1 component)
- Component add: <300ns

### Physics
- **Integration**: Verlet integration, SIMD variants
- **Collision**: Broad-phase, narrow-phase

**Target Performance:**
- Integration: <100ns per entity (SIMD)
- Broad-phase: <1μs per 100 entities

### Math/SIMD
- **Vector Operations**: Add, subtract, multiply, dot, cross
- **Transform Operations**: Matrix multiplication, quaternion math

**Target Performance:**
- Vector ops: <5ns (SIMD)
- Transform: <50ns

### Serialization
- **Component Serialization**: Binary, JSON, MessagePack
- **World State**: Save/load entire world

**Target Performance:**
- Binary serialization: <100ns per component
- JSON: <500ns per component

### Profiling
- **Overhead**: Scope creation, frame markers

**Target Performance:**
- Enabled: <200ns per scope (goal: 100ns)
- Disabled: <1ns (zero-cost abstraction)

---

## Scripts

### benchmark_all_platforms.sh / .ps1

**Purpose:** Run complete benchmark suite on current platform

**Options:**
```bash
--baseline NAME      # Save results as named baseline
--compare NAME       # Compare with named baseline
--output DIR         # Custom output directory
--quick              # Run subset of benchmarks (faster)
--no-platform        # Skip platform benchmarks
--no-ecs             # Skip ECS benchmarks
--verbose            # Enable verbose output
--help               # Show help
```

**Examples:**
```bash
# Full run
./scripts/benchmark_all_platforms.sh

# Quick run (development)
./scripts/benchmark_all_platforms.sh --quick

# Save as baseline
./scripts/benchmark_all_platforms.sh --baseline before-optimization

# Compare with baseline
./scripts/benchmark_all_platforms.sh --compare before-optimization

# Save and compare in one run
./scripts/benchmark_all_platforms.sh --baseline after --compare before
```

### compare_with_industry.py

**Purpose:** Compare results against industry standards

**Usage:**
```bash
python scripts/compare_with_industry.py --results <results_dir>
```

**Output:**
- Markdown report with performance assessment
- ✅ Excellent, ✓ Good, ⚠️ Acceptable, ❌ Poor ratings
- Recommendations for optimization priorities

**Example:**
```bash
python scripts/compare_with_industry.py \
  --results benchmarks/results/windows_20260201_120000 \
  --output custom_report.md
```

### benchmark_regression_check.py

**Purpose:** Detect performance regressions between baseline and current

**Usage:**
```bash
python scripts/benchmark_regression_check.py \
  --baseline <baseline_dir> \
  --current <current_dir> \
  --threshold <percent> \
  --format <criterion|iai>
```

**Options:**
- `--fail-on-regression`: Exit with error code if regressions found (for CI)
- `--show-all`: Show all benchmarks, not just changes
- `--output FILE`: Generate markdown report

**Examples:**
```bash
# Compare Criterion benchmarks (20% threshold for CI noise)
python scripts/benchmark_regression_check.py \
  --baseline benchmarks/baselines/windows_main/criterion \
  --current target/criterion \
  --threshold 20 \
  --format criterion

# Compare Iai benchmarks (10% threshold, deterministic)
python scripts/benchmark_regression_check.py \
  --baseline benchmarks/baselines/linux_main/iai \
  --current target/iai \
  --threshold 10 \
  --format iai \
  --fail-on-regression

# Generate detailed report
python scripts/benchmark_regression_check.py \
  --baseline benchmarks/baselines/windows_main/criterion \
  --current target/criterion \
  --threshold 20 \
  --format criterion \
  --output benchmarks/reports/regression_report.md
```

---

## Baseline Management

### Creating Baselines

**When to create baselines:**
- Before major optimization work
- Before merging to main branch
- After completing a development phase
- For release candidates

**Naming conventions:**
```bash
main              # Current main branch baseline
develop           # Current develop branch baseline
v1.0.0            # Release version baseline
before-simd-opt   # Before specific optimization
after-ecs-refactor # After major refactor
```

**Example workflow:**
```bash
# 1. Checkout main branch
git checkout main

# 2. Build release mode
cargo build --release

# 3. Run benchmarks and save baseline
./scripts/benchmark_all_platforms.sh --baseline main

# 4. Switch to feature branch
git checkout feature/simd-optimization

# 5. Make changes...

# 6. Compare with baseline
./scripts/benchmark_all_platforms.sh --compare main
```

### Updating Baselines

Baselines should be updated when:
- Performance improvements are intentional and verified
- Main branch significantly changes
- New benchmarks are added

**Update process:**
```bash
# Run benchmarks with new baseline name
./scripts/benchmark_all_platforms.sh --baseline main-new

# Verify results
python scripts/compare_with_industry.py \
  --results benchmarks/results/<platform>_<timestamp>

# If satisfied, replace old baseline
mv benchmarks/baselines/<platform>_main benchmarks/baselines/<platform>_main-old
mv benchmarks/baselines/<platform>_main-new benchmarks/baselines/<platform>_main
```

### Listing Baselines

```bash
# List all baselines
ls -la benchmarks/baselines/

# View baseline metadata
cat benchmarks/baselines/windows_main/metadata.json
```

---

## CI Integration

### GitHub Actions

The benchmark system integrates with CI via `.github/workflows/benchmark-regression.yml`.

**Automated checks:**
1. **Criterion benchmarks** (Linux, Windows, macOS)
   - 20% threshold (tolerates CI VM noise)
   - Results archived as artifacts

2. **Iai-callgrind benchmarks** (Linux only)
   - 10% threshold (deterministic, strict)
   - Fails CI if regressions detected

**Viewing CI results:**
1. Go to PR → Checks → Benchmark Regression
2. Download artifacts: `criterion-results-<platform>` or `iai-results`
3. Extract and open `target/criterion/report/index.html`

### Local CI Simulation

Test what CI will do:

```bash
# Run same checks as CI
./scripts/benchmark_all_platforms.sh --baseline ci-baseline
./scripts/benchmark_all_platforms.sh --compare ci-baseline

# Check for regressions (like CI does)
python scripts/benchmark_regression_check.py \
  --baseline benchmarks/baselines/<platform>_ci-baseline/criterion \
  --current target/criterion \
  --threshold 20 \
  --fail-on-regression
```

---

## Performance Targets

All benchmarks are compared against industry standards and our engine's performance goals.

### Industry Comparison Sources

Performance targets are based on:
- **Unity, Unreal, Godot**: Commercial game engine benchmarks
- **Bevy, Hecs, EnTT, Flecs**: ECS framework comparisons
- **Windows QPC, Linux clock_gettime**: Platform timing APIs
- **Published research**: Academic papers and blog posts

See `PLATFORM_BENCHMARK_COMPARISON.md` for detailed industry data.

### Target Summary

| Category | Our Target | Industry Range | Assessment |
|----------|-----------|----------------|------------|
| Time Query | <50ns | 26-300ns | Competitive |
| Entity Spawn | <500ns | 5-1000ns | Good |
| Query Iteration | <50ns | 1-200ns | Competitive |
| Profiling (on) | <200ns | 50-500ns | Good |
| Profiling (off) | <1ns | 0-10ns | Excellent |

---

## Troubleshooting

### "No benchmark results found"

**Cause:** Benchmarks haven't been run or Criterion output missing.

**Fix:**
```bash
# Run benchmarks first
cargo bench

# Verify output exists
ls -la target/criterion/
```

### "Baseline not found"

**Cause:** No baseline saved with that name for current platform.

**Fix:**
```bash
# List available baselines
ls -la benchmarks/baselines/

# Create baseline if missing
./scripts/benchmark_all_platforms.sh --baseline main
```

### High variance in results

**Cause:** Background processes, thermal throttling, or VM noise.

**Fix:**
```bash
# Close unnecessary applications
# Disable background services (antivirus, indexing, etc.)

# Use quick mode for development
./scripts/benchmark_all_platforms.sh --quick

# Use Iai for deterministic results (Linux only)
cargo bench --bench iai_benchmarks
```

### Different results on CI vs local

**Cause:** Different hardware, OS settings, or workload.

**Fix:**
- Use 20% threshold for Criterion (CI tolerance)
- Use Iai benchmarks for deterministic comparison (Linux)
- Compare only against same platform baselines
- Download CI artifacts and compare locally

### Python script errors

**Cause:** Missing Python or dependencies.

**Fix:**
```bash
# Check Python version (3.7+ required)
python3 --version

# No external dependencies needed (uses stdlib only)
```

---

## Best Practices

### Before Benchmarking

1. **Close unnecessary applications**
2. **Disable power management** (performance mode)
3. **Plug in laptop** (avoid battery throttling)
4. **Wait for system idle** (CPU temperature stable)
5. **Build in release mode** (`cargo bench` does this automatically)

### During Development

1. **Use `--quick` mode** for rapid iteration
2. **Focus on specific benchmarks** using Criterion's filtering
3. **Save baselines before optimizations**
4. **Compare frequently** to catch regressions early

### Before Committing

1. **Run full benchmark suite** (no `--quick`)
2. **Compare with main baseline**
3. **Check for regressions** (>10% = investigate)
4. **Update baselines if improvements are intentional**

### For Releases

1. **Run on all platforms** (Windows, Linux, macOS)
2. **Compare with previous release**
3. **Document performance changes**
4. **Save release baseline** (e.g., `v1.0.0`)

---

## Further Reading

- [Benchmarking Guide](../docs/benchmarking.md) - Complete benchmarking documentation
- [Performance Targets](../docs/performance-targets.md) - Detailed performance goals
- [Platform Comparison](../PLATFORM_BENCHMARK_COMPARISON.md) - Industry baseline data
- [Criterion.rs Book](https://bheisler.github.io/criterion.rs/book/) - Criterion documentation
- [Iai-callgrind Docs](https://docs.rs/iai-callgrind/) - Deterministic benchmarking

---

**Last Updated:** 2026-02-01
**Status:** Complete
