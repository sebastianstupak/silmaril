# Benchmark Quick Reference

Quick reference for running and managing benchmarks.

---

## 🚀 Quick Start

```bash
# Run all benchmarks
cargo xtask bench all-all

# Compare with baseline
cargo xtask bench all-baseline

# View results
cargo xtask bench all-report
```

---

## 📊 Common Commands

### Running Benchmarks

| Command | Description | Use When |
|---------|-------------|----------|
| `cargo xtask bench all` | Run all benchmarks (standard) | Regular benchmark run |
| `cargo xtask bench all-all` | Run all + save baseline | Before creating PR |
| `cargo xtask bench all-smoke` | Quick smoke test (fast) | Verifying benchmarks compile |
| `cargo xtask bench all-profile` | Run with profiling | Investigating performance |

### Specific Suites

| Command | Description | Crate |
|---------|-------------|-------|
| `cargo xtask bench all-ecs` | ECS benchmarks | engine-core |
| `cargo xtask bench all-physics` | Physics benchmarks | engine-physics |
| `cargo xtask bench all-renderer` | Renderer benchmarks | engine-renderer |
| `cargo xtask bench all-math` | Math benchmarks | engine-math |
| `cargo xtask bench all-profiling` | Profiling overhead | engine-profiling |
| `cargo xtask bench all-platform` | Platform-specific | engine-core |
| `cargo xtask bench all-compare` | Industry comparison | engine-core |

### Baseline Management

| Command | Description | Use When |
|---------|-------------|----------|
| `cargo xtask bench all-baseline` | Compare with saved baseline | After changes to check regression |
| `cargo xtask bench all-save-baseline` | Save current as main baseline | After performance improvement |
| `cargo xtask bench all-report` | Open HTML report | Viewing detailed results |

---

## 🛠️ Helper Scripts

### Create/Update Baseline

```bash
# Update main baseline
./scripts/update_benchmark_baseline.sh main

# Update with specific platform
./scripts/update_benchmark_baseline.sh main linux-x64
```

### Compare with Baseline

```bash
# Compare against main
./scripts/compare_with_baseline.sh main

# Compare with custom threshold
./scripts/compare_with_baseline.sh main 15  # 15% threshold
```

---

## 📈 Cargo Commands

### Direct Cargo Usage

```bash
# Run all benchmarks
cargo bench --all-features

# Run specific crate
cargo bench --package engine-core

# Run specific benchmark
cargo bench --package engine-core --bench ecs_simple

# Save baseline
cargo bench --all-features -- --save-baseline main

# Compare with baseline
cargo bench --all-features -- --baseline main

# List all benchmarks
cargo bench --all-features -- --list
```

---

## 🎯 Performance Targets

Quick reference for performance targets:

| Benchmark | Target | Status |
|-----------|--------|--------|
| ECS entity spawn | < 50ns | ✅ 47ns |
| Component query (1K) | < 1ms | ✅ 0.8ms |
| Physics tick (10K) | < 8ms | ✅ 7.2ms |
| Vulkan fence reset | < 10µs | ✅ 1.0µs |
| Transform SIMD | < 100ns | ✅ 85ns |

See [docs/performance-targets.md](performance-targets.md) for complete list.

---

## 🔍 Viewing Results

### HTML Report

```bash
# Generate and open report
cargo xtask bench all-all
cargo xtask bench all-report
```

Or manually:
```bash
# Linux
xdg-open target/criterion/report/index.html

# macOS
open target/criterion/report/index.html

# Windows
start target/criterion/report/index.html
```

### Command Line

Criterion shows results in terminal after running:

```
ecs_spawn_entities/1000
                        time:   [47.123 ns 47.456 ns 47.789 ns]
                        thrpt:  [21.2 Melem/s 21.4 Melem/s 21.5 Melem/s]

Change vs baseline:
                        time:   [-5.234% -4.123% -3.012%] (improvement)
                        thrpt:  [+3.012% +4.123% +5.234%]
```

---

## 🚨 Regression Detection

### Local Check

```bash
# Run benchmarks with comparison
cargo xtask bench all-all
cargo xtask bench all-baseline

# Or use script for detailed report
./scripts/compare_with_baseline.sh main
```

### CI/CD Automatic

On every PR:
1. CI fetches `main` baseline
2. Runs all benchmarks
3. Compares results
4. Posts PR comment
5. Fails if regression > 20%

---

## 📝 Best Practices

### Before Committing

```bash
# 1. Run benchmarks
cargo xtask bench all-all

# 2. Compare with baseline
cargo xtask bench all-baseline

# 3. If no regressions, commit
git add .
git commit -m "feat: Add new feature"

# 4. If regressions, profile and optimize
cargo xtask bench all-profile
# ... optimize code ...
cargo xtask bench all-baseline
```

### After Performance Improvement

```bash
# 1. Verify improvement
cargo xtask bench all-baseline

# 2. Update baseline
./scripts/update_benchmark_baseline.sh main

# 3. Review changes
git diff benchmarks/baselines/

# 4. Commit with justification
git add benchmarks/baselines/
git commit -m "chore: Update benchmark baseline

Performance improvements:
- Entity spawn: 47ns → 38ns (-19%)
- Component query: 0.8ms → 0.6ms (-25%)"
```

---

## 🐛 Troubleshooting

### Benchmark Fails to Compile

```bash
# Check which benchmark is failing
cargo bench --all-features -- --list

# Run specific benchmark
cargo bench --package engine-core --bench ecs_simple
```

### Inconsistent Results

```bash
# Close other applications
# Disable CPU frequency scaling
# Run multiple times

# Run with more samples
cargo bench -- --sample-size 100

# Or increase warm-up time
# (Edit benchmark code to add):
group.warm_up_time(Duration::from_secs(5));
```

### Baseline Not Found

```bash
# Check available baselines
ls -la benchmarks/baselines/

# Create baseline
./scripts/update_benchmark_baseline.sh main
```

---

## 🔗 More Information

- **Full Documentation**: [benchmarks/README.md](../benchmarks/README.md)
- **Automation Guide**: [benchmarks/AUTOMATION.md](../benchmarks/AUTOMATION.md)
- **Quick Start**: [benchmarks/QUICK_START.md](../benchmarks/QUICK_START.md)
- **Contributing**: [docs/CONTRIBUTING.md](CONTRIBUTING.md#benchmark-requirements)
- **Performance Targets**: [docs/performance-targets.md](performance-targets.md)

---

**Tip**: Run `cargo xtask --help` to see all available commands!
