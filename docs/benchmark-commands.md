# Benchmark Commands Reference

> Quick reference for running all benchmark suites
>
> **Note:** just doesn't support colons in recipe names, so we use hyphens instead

---

## Quick Start

```bash
# Run all networking benchmarks (recommended)
just bench-network

# Run all ECS benchmarks
just bench-ecs

# Run all benchmarks
just bench
```

---

## Available Benchmark Commands

### Networking Benchmarks
```bash
just bench-network              # All networking benchmarks (~10-15 min)
```

This runs:
- Protocol benchmarks (serialization, framing, throughput)
- Socket benchmarks (TCP/UDP latency and throughput)
- Snapshot benchmarks (state generation, sizes)
- Delta benchmarks (compression, adaptive switching)
- Integration benchmarks (MMORPG, FPS, Battle Royale scenarios)

### ECS Benchmarks
```bash
just bench-ecs                  # All ECS benchmarks (~5-10 min)
```

Includes:
- Entity spawning
- Component operations (get/add/remove)
- Query iteration (simple, complex, filtered)
- Change detection
- Parallel queries
- System scheduling

### Physics Benchmarks
```bash
just bench-physics              # Physics integration (~2-3 min)
```

### Rendering Benchmarks
```bash
just bench-renderer             # Vulkan rendering (~2-3 min)
```

### Math/SIMD Benchmarks
```bash
just bench-math                 # Math and SIMD operations (~2-3 min)
```

### Profiling Overhead Benchmarks
```bash
just bench-profiling            # Profiling overhead (~1-2 min)
```

---

## Utility Commands

### Quick Smoke Test
```bash
just bench-smoke                # Fast validation (<1 min)
```

Runs a subset of benchmarks with reduced sample size for quick validation.

### Benchmark Report
```bash
just bench-report               # Open HTML report in browser
```

Opens the Criterion HTML report showing:
- Performance graphs
- Regression detection
- Statistical analysis
- Comparison with previous runs

### Baseline Management
```bash
# Save current performance as baseline
just bench-save-baseline

# Compare against baseline
just bench-baseline

# Run all benchmarks and save baseline
just bench-all
```

---

## Running Individual Benchmark Files

If you need to run a specific benchmark file:

```bash
# Protocol benchmarks only
cargo bench --package engine-networking --bench protocol_benches

# Socket benchmarks only
cargo bench --package engine-networking --bench socket_benches

# Snapshot benchmarks only
cargo bench --package engine-networking --bench snapshot_benches

# Delta benchmarks only
cargo bench --package engine-networking --bench delta_benches

# Integration scenarios
cargo bench --package engine-networking --bench integration_benches
```

---

## Benchmark Output

Benchmarks generate reports in:
```
target/criterion/
├── report/
│   └── index.html              # Main report (open with browser)
├── <benchmark-name>/
│   ├── base/                   # Baseline measurements
│   ├── new/                    # Latest measurements
│   └── change/                 # Regression analysis
```

---

## Performance Targets

See [AAA_NETWORKING_VALIDATION.md](../AAA_NETWORKING_VALIDATION.md) for detailed performance targets.

**Key AAA Standards:**
- **Serialization:** >200 MB/sec (Bincode)
- **Protocol:** >10K msg/sec throughput
- **TCP latency:** <50ms p95
- **UDP latency:** <20ms p95
- **Delta compression:** >70% bandwidth savings

---

## Continuous Integration

For CI/CD pipelines:

```bash
# Quick smoke test (fast, for every commit)
just bench-smoke

# Full benchmark suite (nightly)
just bench-all
```

---

## Troubleshooting

### Benchmarks take too long
Use `just bench-smoke` for quick validation (< 1 min)

### Want to run specific scenarios
Use cargo bench directly:
```bash
cargo bench --package engine-networking --bench integration_benches -- mmorpg
```

### Need to compare performance
```bash
# Run and save baseline
just bench-save-baseline

# Make changes...

# Compare against baseline
just bench-baseline
```

---

## See Also

- [benchmarking.md](benchmarking.md) - Complete benchmarking guide
- [AAA_NETWORKING_VALIDATION.md](../AAA_NETWORKING_VALIDATION.md) - AAA performance targets
- [NETWORKING_BENCHMARKS_COMPLETE.md](../NETWORKING_BENCHMARKS_COMPLETE.md) - Benchmark implementation summary
