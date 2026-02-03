# Benchmarking Guide

This document describes the benchmarking infrastructure and how to use it for performance validation.

---

## Overview

The engine uses a two-tier benchmarking system:

1. **Criterion** - Wall-clock time measurement (practical, but noisy)
2. **Iai-callgrind** - Instruction count measurement (deterministic, platform-independent)

Both are integrated into CI for automated regression detection.

---

## Running Benchmarks Locally

### Criterion Benchmarks

**Run all benchmarks:**
```bash
cargo xtask bench all
```

**Run specific benchmark:**
```bash
cargo xtask bench profiling
```

**Run with baseline comparison:**
```bash
# Save current state as baseline
cargo xtask bench save-baseline main

# Make code changes...

# Compare against baseline
cargo xtask bench baseline
```

**View results:**
```bash
# HTML report is generated at:
open target/criterion/report/index.html
```

### Iai-callgrind Benchmarks

**Prerequisites:**
- Linux only (requires Valgrind)
- Install Valgrind: `sudo apt-get install valgrind`

**Run benchmarks:**
```bash
cargo xtask bench iai
```

**Results location:**
```
target/iai/<benchmark_name>/
```

---

## CI Benchmark Regression

The `.github/workflows/benchmark-regression.yml` workflow runs automatically on:
- Pull requests to `main` or `develop`
- Pushes to `main`
- Manual workflow dispatch

### What CI Checks

1. **Criterion benchmarks** (3 platforms: Linux, Windows, macOS)
   - Compares current PR against `main` branch baseline
   - 20% threshold for warnings (due to CI VM noise)
   - Results archived as artifacts

2. **Iai-callgrind benchmarks** (Linux only)
   - Deterministic instruction count measurement
   - 10% threshold for regression (strict, no noise)
   - Fails CI if exceeded

### Viewing CI Results

1. Go to PR checks
2. Click "Benchmark Regression" workflow
3. View "Summary" tab for artifacts
4. Download `criterion-results-<platform>` or `iai-results`
5. Extract and open `target/criterion/report/index.html`

---

## Regression Thresholds

| Benchmark Type | Threshold | Why |
|----------------|-----------|-----|
| Iai (instructions) | 10% | Deterministic, no noise |
| Criterion (time) | 20% | Tolerates CI VM variance |

**Rationale:**
- Iai instruction counts are deterministic - 10% increase is a real regression
- Criterion wall-clock times vary on CI VMs - 20% threshold avoids false positives
- Both thresholds are stricter than industry standard (25-30%)

---

## Adding New Benchmarks

### Adding Criterion Benchmark

1. **Create or edit benchmark file:**
   ```rust
   // engine/<crate>/benches/my_bench.rs
   use criterion::{black_box, criterion_group, criterion_main, Criterion};

   fn my_benchmark(c: &mut Criterion) {
       c.bench_function("my_function", |b| {
           b.iter(|| {
               // Code to benchmark
               black_box(my_function());
           });
       });
   }

   criterion_group!(benches, my_benchmark);
   criterion_main!(benches);
   ```

2. **Register in Cargo.toml:**
   ```toml
   [[bench]]
   name = "my_bench"
   harness = false
   ```

3. **Run locally:**
   ```bash
   cargo xtask bench my_bench
   ```

### Adding Iai-callgrind Benchmark

1. **Create benchmark file:**
   ```rust
   // engine/<crate>/benches/iai_my_bench.rs
   use iai_callgrind::{library_benchmark, library_benchmark_group, main};

   fn iai_my_function() {
       my_function();
   }

   library_benchmark_group!(
       name = my_group;
       benchmarks = iai_my_function
   );

   main!(library_benchmark_groups = my_group);
   ```

2. **Add iai-callgrind to dev-dependencies:**
   ```toml
   [dev-dependencies]
   iai-callgrind = "0.13"
   ```

3. **Register in Cargo.toml:**
   ```toml
   [[bench]]
   name = "iai_my_bench"
   harness = false
   ```

4. **Run locally (Linux only):**
   ```bash
   cargo xtask bench iai_my_bench
   ```

---

## Benchmark Best Practices

### DO:
- ✅ Use `black_box()` to prevent compiler optimizations
- ✅ Warm up before timing (Criterion does this automatically)
- ✅ Benchmark realistic workloads (not micro-benchmarks unless necessary)
- ✅ Save baselines before major changes
- ✅ Use Iai for deterministic comparisons

### DON'T:
- ❌ Benchmark I/O operations (too noisy)
- ❌ Benchmark with debug builds (always use `cargo bench`)
- ❌ Compare benchmarks across different machines
- ❌ Ignore regression warnings ("it's probably noise" - validate!)

---

## Performance Budgets

Target performance for critical paths:

| Operation | Target | Critical |
|-----------|--------|----------|
| Scope creation (profiling ON) | < 200ns | < 500ns |
| Scope creation (profiling OFF) | < 1ns | < 10ns |
| Frame begin/end | < 100ns | < 500ns |
| Entity spawn | < 500ns | < 1µs |
| Component add | < 300ns | < 800ns |
| Query iteration (per entity) | < 50ns | < 200ns |

**See:** `docs/performance-targets.md` for complete performance targets.

---

## Troubleshooting

### "Benchmark not found"

**Cause:** Benchmark binary not built or wrong name.

**Fix:**
```bash
# List all benchmark targets
cargo xtask bench list

# Check Cargo.toml [[bench]] sections
```

### "Valgrind not found" (Iai)

**Cause:** Valgrind not installed (required for Iai).

**Fix:**
```bash
# Linux
sudo apt-get install valgrind

# macOS (limited support)
brew install valgrind
```

### "Baseline not found"

**Cause:** No baseline saved yet.

**Fix:**
```bash
# Save a baseline first
cargo xtask bench save-baseline main
```

### CI shows regressions but local doesn't

**Cause:** Different platform or noise in Criterion.

**Fix:**
1. Check Iai benchmarks (deterministic)
2. Run Criterion multiple times locally
3. Compare against same platform in CI artifacts

---

## Bencher.dev Integration (Optional)

For long-term benchmark tracking and historical analysis, integrate with [bencher.dev](https://bencher.dev):

### Setup

1. **Create Bencher account:** https://bencher.dev
2. **Create project:** "silmaril"
3. **Get API token:** Settings → API Tokens
4. **Add to GitHub Secrets:** `BENCHER_API_TOKEN`

### Enable Workflow

Uncomment the `bencher-tracking` job in `.github/workflows/benchmark-regression.yml`:

```yaml
bencher-tracking:
  name: Track Benchmarks with Bencher
  # ... (already configured, just uncomment)
```

### View Results

- **Dashboard:** https://bencher.dev/silmaril
- **PR Comments:** Bencher automatically comments on PRs with benchmark comparisons
- **Historical Charts:** Track performance over time

---

## Advanced: Custom Regression Script

The `scripts/check_benchmark_regression.py` script can be customized:

**Change thresholds:**
```bash
python scripts/check_benchmark_regression.py \
  --baseline target/criterion-baseline \
  --current target/criterion \
  --threshold 15 \  # Custom threshold
  --format criterion
```

**Don't fail CI on regression:**
```bash
python scripts/check_benchmark_regression.py \
  --baseline ... \
  --current ... \
  --threshold 10 \
  --format iai \
  --fail-on-regression false  # Just warn
```

**Script features:**
- Parses Criterion JSON output
- Parses Iai JSON output
- Calculates percentage changes
- Detects new/removed benchmarks
- Formats results as table
- Exits with error code on regression

---

## References

- [Criterion.rs](https://bheisler.github.io/criterion.rs/book/)
- [Iai-callgrind](https://docs.rs/iai-callgrind/)
- [Performance Testing Best Practices](https://easyperf.net/blog/2018/08/26/Basics-of-profiling)
- [Unity Profiling Guide](https://unity.com/how-to/best-practices-for-profiling-game-performance)

---

**Last Updated:** 2026-02-01
**Status:** Complete (Task 0.5.8)
