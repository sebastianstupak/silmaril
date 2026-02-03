# Network Simulation and Integration Benchmarks

This document describes the comprehensive network benchmarking infrastructure for the silmaril.

## Overview

The networking benchmarks test end-to-end performance under realistic network conditions:

1. **Network Simulator** (`src/simulator.rs`) - Simulates realistic network conditions
2. **Integration Benchmarks** (`benches/integration_benches.rs`) - Full client-server game loop tests

## Network Simulator

### Features

The `NetworkSimulator` provides realistic network simulation with:

- **Latency**: Base round-trip time (RTT) simulation
- **Jitter**: Latency variance for unstable connections
- **Packet Loss**: Probabilistic packet dropping
- **Bandwidth Throttling**: Limit data rate (KB/s)
- **Packet Reordering**: Out-of-order delivery simulation

### Network Profiles

Pre-configured profiles for common network conditions:

| Profile | Latency | Jitter | Packet Loss | Bandwidth | Use Case |
|---------|---------|--------|-------------|-----------|----------|
| **LAN** | 1ms | 0ms | 0% | 100 Mbps | Local network testing |
| **Cable** | 20ms | 2ms | 0.1% | 10 Mbps | Good home broadband |
| **DSL** | 50ms | 5ms | 0.5% | 2 Mbps | Average DSL connection |
| **4G** | 80ms | 20ms | 1% | 5 Mbps | Mobile 4G network |
| **3G** | 150ms | 50ms | 3% | 1 Mbps | Mobile 3G network |
| **Terrible** | 300ms | 100ms | 10% | 500 Kbps | Worst-case scenario |

### Usage Example

```rust
use engine_networking::{NetworkProfile, NetworkSimulator};

// Create simulator with a profile
let mut sim = NetworkSimulator::new(NetworkProfile::Cable);

// Send a packet (will be delayed by latency + jitter)
let data = vec![1, 2, 3, 4, 5];
sim.send(data);

// Receive packets that are ready (latency has passed)
let received = sim.recv();

// Check packets in flight
let count = sim.in_flight();
```

### Custom Conditions

```rust
use engine_networking::NetworkConditions;

let custom = NetworkConditions {
    latency_ms: 100,
    jitter_ms: 10,
    packet_loss_percent: 2.0,
    bandwidth_kbps: 1000,
    reorder_probability: 0.05,
};

let mut sim = NetworkSimulator::with_conditions(custom);
```

## Integration Benchmarks

### Overview

The integration benchmarks simulate full client-server game loops with realistic scenarios:

- **MMORPG**: 100 players, mostly static, sparse updates (30Hz)
- **FPS**: 16 players, high movement, frequent updates (60Hz)
- **Battle Royale**: 100 players, distributed, varying density (60Hz)

### Benchmark Suites

#### 1. Game Scenarios (`bench_game_scenarios`)

Tests complete game loop performance for different game types:

```bash
cargo bench --bench integration_benches -- game_scenarios
```

Measures:
- Overall throughput
- Tick rate stability
- Client update latency

#### 2. End-to-End Latency (`bench_end_to_end_latency`)

Measures latency from input → server → client under different network conditions:

```bash
cargo bench --bench integration_benches -- end_to_end_latency
```

Target: **< 50ms** end-to-end latency for Cable/DSL

#### 3. Bandwidth Usage (`bench_bandwidth_usage`)

Measures bandwidth per client at different update rates (30Hz, 60Hz, 120Hz):

```bash
cargo bench --bench integration_benches -- bandwidth_usage
```

Target: **< 10 KB/sec per client** at 60Hz

#### 4. Concurrent Clients (`bench_concurrent_clients`)

Tests scalability with 1, 10, 50, 100 concurrent clients:

```bash
cargo bench --bench integration_benches -- concurrent_clients
```

Measures:
- Server CPU usage
- Latency degradation
- Memory usage

#### 5. Packet Loss Resilience (`bench_packet_loss_resilience`)

Tests graceful degradation under 0%, 0.5%, 1%, 3% packet loss:

```bash
cargo bench --bench integration_benches -- packet_loss_resilience
```

Ensures game remains playable even with packet loss.

#### 6. Scalability (`bench_scalability`)

Measures latency vs player count (10, 25, 50, 100, 200):

```bash
cargo bench --bench integration_benches -- scalability
```

Identifies scalability limits and bottlenecks.

#### 7. Simulator Overhead (`bench_simulator_overhead`)

Measures the performance cost of network simulation itself:

```bash
cargo bench --bench integration_benches -- simulator_overhead
```

Ensures simulation doesn't skew benchmark results.

## Performance Targets

| Metric | Target | Critical Threshold |
|--------|--------|-------------------|
| End-to-end latency | < 50ms | < 100ms |
| Bandwidth per client | < 10 KB/s | < 50 KB/s |
| Server tick time | < 16ms (60 TPS) | < 33ms |
| Concurrent clients | 100+ | 50+ |
| Packet loss tolerance | 3% playable | 5% critical |

## Running Benchmarks

### Quick Test

Run a specific benchmark suite:

```bash
cargo bench --bench integration_benches -- end_to_end_latency
```

### Full Suite

Run all integration benchmarks:

```bash
cargo bench --bench integration_benches
```

### Baseline Comparison

Create a baseline for comparison:

```bash
cargo bench --bench integration_benches -- --save-baseline main
```

Compare against baseline:

```bash
cargo bench --bench integration_benches -- --baseline main
```

### Output Location

Benchmark results are saved to:
```
target/criterion/
├── end_to_end_latency/
├── bandwidth_usage/
├── concurrent_clients/
├── ...
└── report/
    └── index.html  # ← Open in browser for visualizations
```

## Interpreting Results

### Example Output

```
end_to_end_latency/LAN  time:   [1.234 ms 1.250 ms 1.267 ms]
                        change: [-2.3% -1.8% -1.2%] (p = 0.00 < 0.05)
                        Performance has improved.

end_to_end_latency/Cable time:  [21.45 ms 21.68 ms 21.94 ms]
                         change: [+0.5% +1.2% +1.9%] (p = 0.02 < 0.05)
                         Performance has regressed.
```

### Key Metrics

- **time**: Median, mean, and upper bound of execution time
- **change**: Performance change vs baseline
- **p-value**: Statistical significance (< 0.05 = significant)

### HTML Report

Open `target/criterion/report/index.html` for:
- Performance graphs
- Distribution plots
- Historical trends
- Outlier detection

## Customization

### Custom Scenarios

Add new game scenarios in `integration_benches.rs`:

```rust
impl GameScenario {
    fn my_game() -> Self {
        Self {
            name: "MyGame",
            player_count: 64,
            entity_count: 500,
            update_radius: 150.0,
            movement_speed: 15.0,
            update_frequency_hz: 60,
        }
    }
}
```

### Custom Network Profiles

Test specific network conditions:

```rust
let profile = NetworkProfile::Custom(NetworkConditions {
    latency_ms: 75,
    jitter_ms: 15,
    packet_loss_percent: 1.5,
    bandwidth_kbps: 3000,
    reorder_probability: 0.02,
});

simulate_game_loop(&scenario, profile, Duration::from_secs(1));
```

## CI/CD Integration

### Regression Detection

Add to CI pipeline to detect performance regressions:

```yaml
- name: Run network benchmarks
  run: |
    cargo bench --bench integration_benches -- --save-baseline ci

- name: Check for regressions
  run: |
    cargo bench --bench integration_benches -- --baseline ci --noplot
```

### Performance Tracking

Track performance over time:

```bash
# Save results with git commit hash
cargo bench --bench integration_benches -- --save-baseline $(git rev-parse --short HEAD)
```

## Troubleshooting

### Benchmarks Take Too Long

Reduce sample size for expensive benchmarks:

```rust
group.sample_size(10); // Default is 100
```

### Inconsistent Results

Ensure stable environment:
- Close other applications
- Disable CPU power management
- Run on dedicated benchmark machine

### Out of Memory

Reduce scenario size:

```rust
scenario.player_count = 10; // Instead of 100
scenario.entity_count = 100; // Instead of 1000
```

## Future Enhancements

- [ ] Network congestion simulation
- [ ] Geographic latency profiles (US-EU, EU-Asia, etc.)
- [ ] NAT traversal overhead
- [ ] WebRTC data channel simulation
- [ ] Automated regression alerts
- [ ] Performance dashboard integration

## References

- [Network Simulator Implementation](src/simulator.rs)
- [Integration Benchmarks](benches/integration_benches.rs)
- [Criterion.rs Documentation](https://bheisler.github.io/criterion.rs/)
- [Networking Architecture](../../docs/networking.md)
