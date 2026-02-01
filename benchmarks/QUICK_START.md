# Benchmark Automation - Quick Start Guide

Get started with automated benchmarking in under 5 minutes.

---

## 1. Run Your First Benchmark

**Windows:**
```powershell
.\scripts\benchmark_all_platforms.ps1
```

**Linux/macOS:**
```bash
./scripts/benchmark_all_platforms.sh
```

This will:
- Run all benchmark suites
- Save results to `benchmarks/results/<platform>_<timestamp>/`
- Generate HTML report at `target/criterion/report/index.html`

**View results:**
```bash
# Windows
Start-Process target/criterion/report/index.html

# Linux
xdg-open target/criterion/report/index.html

# macOS
open target/criterion/report/index.html
```

---

## 2. Save a Baseline

Before making changes, save the current performance:

**Windows:**
```powershell
.\scripts\benchmark_all_platforms.ps1 -Baseline main
```

**Linux/macOS:**
```bash
./scripts/benchmark_all_platforms.sh --baseline main
```

This saves a baseline to `benchmarks/baselines/<platform>_main/`

---

## 3. Make Changes and Compare

After making optimizations:

**Windows:**
```powershell
.\scripts\benchmark_all_platforms.ps1 -Compare main
```

**Linux/macOS:**
```bash
./scripts/benchmark_all_platforms.sh --compare main
```

This shows performance delta vs baseline.

---

## 4. Check Industry Standards

Compare your results against Unity, Unreal, Godot, and Bevy:

```bash
python scripts/compare_with_industry.py \
  --results benchmarks/results/<platform>_<timestamp>
```

**Output:** `benchmarks/results/<platform>_<timestamp>/industry_comparison.md`

---

## 5. Detect Regressions

Check if performance has regressed:

```bash
python scripts/benchmark_regression_check.py \
  --baseline benchmarks/baselines/<platform>_main/criterion \
  --current target/criterion \
  --threshold 20 \
  --fail-on-regression
```

Exit code:
- `0` = No regressions
- `1` = Regressions detected

---

## Common Workflows

### Development Iteration

```bash
# Quick benchmarks during development
./scripts/benchmark_all_platforms.sh --quick

# Full run when done
./scripts/benchmark_all_platforms.sh
```

### Pre-Commit Check

```bash
# Compare with main
./scripts/benchmark_all_platforms.sh --compare main

# Check for regressions
python scripts/benchmark_regression_check.py \
  --baseline benchmarks/baselines/<platform>_main/criterion \
  --current target/criterion \
  --threshold 10 \
  --fail-on-regression
```

### Before/After Optimization

```bash
# 1. Save "before" baseline
./scripts/benchmark_all_platforms.sh --baseline before-simd

# 2. Make SIMD optimizations...

# 3. Save "after" and compare
./scripts/benchmark_all_platforms.sh --baseline after-simd --compare before-simd

# 4. Industry comparison
python scripts/compare_with_industry.py \
  --results benchmarks/results/<platform>_<timestamp>
```

---

## Quick Reference

### Bash Script Options

```bash
./scripts/benchmark_all_platforms.sh [OPTIONS]

--baseline NAME    # Save as baseline
--compare NAME     # Compare with baseline
--quick            # Quick mode (development)
--verbose          # Verbose output
--help             # Show help
```

### PowerShell Script Options

```powershell
.\scripts\benchmark_all_platforms.ps1 [OPTIONS]

-Baseline NAME     # Save as baseline
-Compare NAME      # Compare with baseline
-Quick             # Quick mode (development)
-Verbose           # Verbose output
-Help              # Show help
```

### Python Scripts

```bash
# Industry comparison
python scripts/compare_with_industry.py \
  --results <results_dir>

# Regression check
python scripts/benchmark_regression_check.py \
  --baseline <baseline_dir> \
  --current <current_dir> \
  --threshold <percent> \
  --fail-on-regression
```

---

## Output Locations

- **Results:** `benchmarks/results/<platform>_<timestamp>/`
- **Baselines:** `benchmarks/baselines/<platform>_<name>/`
- **Reports:** `benchmarks/reports/`
- **HTML:** `target/criterion/report/index.html`

---

## Troubleshooting

**"No benchmarks found"**
```bash
cargo bench  # Run benchmarks first
```

**"Baseline not found"**
```bash
ls benchmarks/baselines/  # List available
./scripts/benchmark_all_platforms.sh --baseline main  # Create one
```

**"Python not found"**
```bash
python3 --version  # Check Python installed
# Use python3 instead of python if needed
```

---

## Next Steps

- Read `benchmarks/AUTOMATION.md` for complete guide
- Check `PLATFORM_BENCHMARK_COMPARISON.md` for industry data
- Review `docs/benchmarking.md` for methodology

---

**Status:** Ready to use ✅
**Last Updated:** 2026-02-01
