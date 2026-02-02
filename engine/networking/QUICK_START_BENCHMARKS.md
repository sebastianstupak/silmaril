# Network Benchmarks - Quick Start Guide

## TL;DR

Run network benchmarks to validate performance:

### Windows
```powershell
# Quick test (2 min)
.\scripts\run_network_benchmarks.ps1 quick

# Full suite (10-20 min)
.\scripts\run_network_benchmarks.ps1 full

# View results
.\scripts\run_network_benchmarks.ps1 report
```

### Linux/macOS
```bash
# Quick test (2 min)
./scripts/run_network_benchmarks.sh quick

# Full suite (10-20 min)
./scripts/run_network_benchmarks.sh full

# View results
./scripts/run_network_benchmarks.sh report
```

## What Gets Tested

### Network Conditions
- **LAN**: 1ms latency, 0% loss (perfect)
- **Cable**: 20ms latency, 0.1% loss (good)
- **DSL**: 50ms latency, 0.5% loss (average)
- **4G**: 80ms latency, 1% loss (mobile)
- **3G**: 150ms latency, 3% loss (slow mobile)
- **Terrible**: 300ms latency, 10% loss (worst case)

### Game Scenarios
- **MMORPG**: 100 players, sparse updates, 30Hz
- **FPS**: 16 players, frequent updates, 60Hz
- **Battle Royale**: 100 players, varying density, 60Hz

### Performance Metrics
- End-to-end latency (input → server → client)
- Bandwidth usage per client (bytes/sec)
- Server tick time (milliseconds)
- Concurrent client scalability (1-200 players)
- Packet loss resilience (0-10% loss)

## Performance Targets

| Metric | Target | Critical |
|--------|--------|----------|
| Latency | < 50ms | < 100ms |
| Bandwidth (60Hz) | < 10 KB/s | < 50 KB/s |
| Server tick | < 16ms | < 33ms |
| Clients | 100+ | 50+ |
| Packet loss | 3% OK | 5% critical |

## Common Commands

### Quick Tests
```bash
# Just latency (30 sec)
./scripts/run_network_benchmarks.sh latency

# Just scalability (2 min)
./scripts/run_network_benchmarks.sh scalability

# Just packet loss (1 min)
./scripts/run_network_benchmarks.sh resilience
```

### Baseline Comparison
```bash
# Save current performance
./scripts/run_network_benchmarks.sh baseline main

# Make changes...

# Compare performance
./scripts/run_network_benchmarks.sh compare main
```

### Manual Cargo Commands
```bash
# Run specific benchmark
cargo bench --bench integration_benches -- end_to_end_latency

# Run all integration benchmarks
cargo bench --bench integration_benches

# Quick mode (fewer iterations)
cargo bench --bench integration_benches -- --quick
```

## Reading Results

### Console Output
```
end_to_end_latency/Cable
    time:   [21.45 ms 21.68 ms 21.94 ms]
    change: [-2.3% -1.8% -1.2%] (p = 0.00 < 0.05)
    Performance has improved.
```

- **time**: [lower bound, median, upper bound]
- **change**: Performance vs previous run
- **p-value**: < 0.05 means statistically significant

### HTML Report
Open `target/criterion/report/index.html` for:
- Performance graphs
- Distribution plots
- Historical trends
- Comparison charts

## Troubleshooting

### "Benchmarks take too long"
Use quick mode or specific tests:
```bash
./scripts/run_network_benchmarks.sh quick  # 2 min instead of 20
./scripts/run_network_benchmarks.sh latency  # Single test
```

### "Results are inconsistent"
- Close other applications
- Disable CPU frequency scaling
- Run on dedicated hardware
- Use baseline comparison

### "Out of memory"
Reduce scenario size in source:
```rust
// benches/integration_benches.rs
scenario.player_count = 10;  // Instead of 100
```

## What's Being Benchmarked

1. **Network Simulator Overhead**: Cost of simulation itself
2. **End-to-End Latency**: Full input → server → client loop
3. **Bandwidth Usage**: Bytes sent/received at different update rates
4. **Concurrent Clients**: Performance with 1-100 simultaneous clients
5. **Packet Loss Resilience**: Gameplay quality under packet loss
6. **Scalability**: Latency increase vs player count

## Files

- `src/simulator.rs` - Network condition simulator
- `benches/integration_benches.rs` - Benchmark implementations
- `tests/simulator_test.rs` - Unit tests
- `NETWORK_BENCHMARKS.md` - Full documentation
- `INTEGRATION_BENCHMARK_SUMMARY.md` - Implementation details

## Next Steps

After running benchmarks:

1. Check results meet targets (see table above)
2. Investigate any regressions
3. Create baseline before making changes
4. Re-run after optimizations
5. Track trends over time

## Help

For full documentation:
```bash
cat engine/networking/NETWORK_BENCHMARKS.md
```

For script options:
```bash
./scripts/run_network_benchmarks.sh help
```
