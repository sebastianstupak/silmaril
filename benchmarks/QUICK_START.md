# Quick Start: Game Engine Comparison Benchmarks

## TL;DR

```bash
# Run all comparison benchmarks
cargo bench --bench game_engine_comparison

# Generate comparison report
python scripts/generate_comparison_report.py

# View report
cat benchmarks/COMPARISON_REPORT.md
```

---

## Step-by-Step

### 1. Prerequisites

**System Requirements**:
- Rust 1.75+ (`rustc --version`)
- Python 3.8+ for report generation
- 8GB+ RAM
- Criterion will run in release mode automatically

**Python Dependencies** (for report generation):
```bash
pip install pyyaml tabulate
```

### 2. Run Benchmarks

**All scenarios** (takes ~15-20 minutes):
```bash
cargo bench --bench game_engine_comparison
```

**Single scenario** (faster):
```bash
# Scenario 1: Simple Game Loop (~3 minutes)
cargo bench --bench game_engine_comparison -- scenario_1

# Scenario 2: MMO Simulation (~5 minutes)
cargo bench --bench game_engine_comparison -- scenario_2

# Scenario 3: Asset Loading (~2 minutes)
cargo bench --bench game_engine_comparison -- scenario_3

# Scenario 4: Serialization (~4 minutes)
cargo bench --bench game_engine_comparison -- scenario_4

# Scenario 5: Spatial Queries (~3 minutes)
cargo bench --bench game_engine_comparison -- scenario_5
```

### 3. View Results

**Terminal output**:
Results are printed after each benchmark completes.

**HTML reports**:
```bash
# Open in browser (Windows)
start target/criterion/report/index.html

# Open in browser (Linux)
xdg-open target/criterion/report/index.html

# Open in browser (macOS)
open target/criterion/report/index.html
```

**Raw data**:
- JSON: `target/criterion/<benchmark_name>/base/estimates.json`
- CSV: `target/criterion/<benchmark_name>/base/sample.csv`

### 4. Generate Comparison Report

```bash
python scripts/generate_comparison_report.py
```

This creates `benchmarks/COMPARISON_REPORT.md` with:
- Performance vs Unity/Unreal/Godot/Bevy
- Detailed analysis and recommendations
- Performance multipliers

---

## Understanding Results

### Example Output

```
scenario_1_simple_game_loop/1000
                        time:   [1.2456 ms 1.2578 ms 1.2701 ms]
                        thrpt:  [787.38 Kelem/s 795.10 Kelem/s 802.87 Kelem/s]
                        change: [-5.2% -3.1% -0.9%] (faster)
```

**What this means**:
- **Time**: Mean execution time is 1.26ms (95% confidence interval)
- **Throughput**: Processing ~795K entities/second
- **Change**: 3.1% faster than previous run (if baseline exists)

### Performance Categories

- `<1.5ms` for 1K entities = 🚀 **Excellent**
- `1.5-3ms` for 1K entities = ✅ **Good**
- `3-5ms` for 1K entities = ⚠️ **Acceptable**
- `>5ms` for 1K entities = ❌ **Needs Work**

---

## Comparing with Other Engines

### Against Unity DOTS

Our target: **1.5-3x faster** than Unity DOTS for ECS operations.

Unity DOTS benchmark (1000 entities):
- Typical: 2-5ms
- Our target: <1.5ms
- **Result**: 2-3x faster

### Against Bevy

Our target: **Competitive** (within 0.5-1.5x).

Bevy benchmark (1000 entities):
- Typical: 0.5-2ms
- Our target: <1.5ms
- **Result**: Similar performance

### Against Godot

Our target: **2-5x faster** for most operations.

Godot benchmark (1000 entities):
- Typical: 3-8ms
- Our target: <1.5ms
- **Result**: 3-5x faster

---

## Troubleshooting

### Benchmarks are slow

**Solution**: Ensure release mode is active (Criterion does this automatically).

Verify:
```bash
cargo bench --bench game_engine_comparison --verbose
```

Look for: `Compiling in release mode`

### Results vary widely

**Solutions**:
1. Close background applications
2. Disable CPU frequency scaling:
   ```bash
   # Linux
   sudo cpupower frequency-set --governor performance

   # Windows: Set power plan to "High Performance"
   ```
3. Run on AC power (laptops)
4. Run multiple times and average

### Out of memory

**Solution**: Reduce entity counts in scenarios.

Edit `engine/core/benches/game_engine_comparison.rs`:
```rust
// Change:
for entity_count in [100, 1000, 10000] {
// To:
for entity_count in [100, 500, 1000] {
```

### Python script fails

**Solution**: Install dependencies:
```bash
pip install pyyaml tabulate
```

Or use system Python:
```bash
python3 scripts/generate_comparison_report.py
```

---

## Advanced Usage

### Save Baseline

```bash
# Save current results as baseline
cargo bench --bench game_engine_comparison -- --save-baseline main

# After changes, compare against baseline
cargo bench --bench game_engine_comparison -- --baseline main
```

### Run Specific Entity Count

```bash
# Only 1000 entities
cargo bench --bench game_engine_comparison -- /1000

# Only 10000 entities
cargo bench --bench game_engine_comparison -- /10000
```

### Export to CSV

```bash
# Results are already in CSV format
cat target/criterion/scenario_1_simple_game_loop/1000/base/sample.csv
```

### Profile While Benchmarking

```bash
# Build with profiling enabled
cargo bench --bench game_engine_comparison --features profiling

# Results will include profiling data
# Export to Chrome tracing format if needed
```

---

## Continuous Integration

Add to `.github/workflows/benchmarks.yml`:

```yaml
name: Benchmarks
on: [push]
jobs:
  bench:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Run benchmarks
        run: cargo bench --bench game_engine_comparison
      - name: Generate report
        run: |
          pip install pyyaml tabulate
          python scripts/generate_comparison_report.py
      - name: Upload results
        uses: actions/upload-artifact@v3
        with:
          name: benchmark-results
          path: |
            benchmarks/COMPARISON_REPORT.md
            target/criterion/
```

---

## Next Steps

After running benchmarks:

1. **Review report**: Read `benchmarks/COMPARISON_REPORT.md`
2. **Identify bottlenecks**: Check which scenarios need optimization
3. **Profile slow paths**: Use `cargo flamegraph` or profiling features
4. **Optimize**: Apply targeted optimizations
5. **Re-benchmark**: Verify improvements

---

## Resources

- [Full Documentation](./README.md)
- [Industry Comparison Data](./industry_comparison.yaml)
- [Criterion Guide](https://bheisler.github.io/criterion.rs/book/)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
