# Delta Compression Usage Guide

## Overview

Delta compression reduces network bandwidth by transmitting only changed entities/components instead of full world snapshots. Achieves 70-95% bandwidth reduction for typical game updates.

## Quick Start

### Basic Usage

```rust
use engine_networking::delta::{NetworkDelta, AdaptiveDeltaStrategy};
use engine_core::serialization::WorldState;

// Server-side: Create adaptive strategy (one per game session)
let mut strategy = AdaptiveDeltaStrategy::default();

// Capture snapshots before and after game tick
let old_state = WorldState::snapshot(&world);
// ... game tick happens ...
let new_state = WorldState::snapshot(&world);

// Compute delta
let net_delta = NetworkDelta::from_states(&old_state, &new_state);

// Decide whether to use delta or full snapshot
if net_delta.should_use_delta() && strategy.should_use_delta(net_delta.compression_ratio) {
    // Send compressed delta (70-95% smaller)
    let bytes = net_delta.to_bytes();
    network.send_to_clients(&bytes);

    // Record for adaptive strategy
    strategy.record_delta(net_delta.compression_ratio);
} else {
    // Send full snapshot (when delta isn't beneficial)
    let bytes = new_state.serialize(Format::Bincode)?;
    network.send_to_clients(&bytes);
}
```

### Client-Side Application

```rust
use engine_networking::delta::NetworkDelta;
use engine_core::serialization::WorldState;

// Client maintains current state
let mut current_state = WorldState::new();

// Receive delta from server
let bytes = network.receive_from_server();
let delta = NetworkDelta::from_bytes(&bytes)?;

// Apply delta to current state
delta.apply(&mut current_state);

// Restore world from updated state
current_state.restore(&mut world);
```

## Configuration

### AdaptiveDeltaStrategy Parameters

```rust
// Default configuration (recommended for most games)
let strategy = AdaptiveDeltaStrategy::default();
// - History: 10 recent deltas
// - Threshold: 0.8 (use delta if <80% of full size)

// Custom configuration
let strategy = AdaptiveDeltaStrategy::new(
    20,   // Track last 20 deltas
    0.7   // Use delta if <70% of full size
);
```

### When to Use Custom Configuration

- **Larger history (20-50)**: Smoother decisions, better for variable workloads
- **Smaller history (5-10)**: Faster adaptation, better for predictable patterns
- **Lower threshold (0.6-0.7)**: More aggressive delta usage, saves more bandwidth
- **Higher threshold (0.8-0.9)**: Conservative, only use delta when clearly beneficial

## Performance Characteristics

### Computation Time

| Entities | Change % | Diff Time | Serialization |
|----------|----------|-----------|---------------|
| 100 | 5% | ~250 µs | ~400 ns |
| 1,000 | 5% | ~1.1 ms | ~3 µs |
| 10,000 | 5% | ~14 ms | ~25 µs |

**Note**: Diff can be computed asynchronously while previous frame is being sent.

### Compression Ratios

| Change % | Delta/Full | Bandwidth Saved |
|----------|------------|-----------------|
| 1% | 5-10% | 90-95% |
| 5% | 15-30% | 70-85% |
| 10% | 30-50% | 50-70% |
| 50% | 60-80% | 20-40% |

**Break-even**: ~40-50% entity changes

## Common Patterns

### Pattern 1: Player Movement (Most Common)

```rust
// Only positions change (5% of entities)
// Expected compression: 80-90%

let net_delta = NetworkDelta::from_states(&old_state, &new_state);
assert!(net_delta.compression_ratio < 0.2); // 80%+ reduction
```

### Pattern 2: Combat (Mixed Changes)

```rust
// Positions, health, effects change (10-20% of entities)
// Expected compression: 50-70%

if net_delta.compression_ratio < 0.5 {
    // Still worthwhile to use delta
    send_delta(&net_delta);
}
```

### Pattern 3: Mass Spawning

```rust
// Many entities added/removed (>40% changes)
// Expected compression: 20-40% (might be better to send full)

if net_delta.should_use_delta() {
    send_delta(&net_delta);
} else {
    send_full_snapshot(&new_state);
}
```

## Integration with Network Stack

### TCP (Reliable Channel)

```rust
// Full snapshots and important deltas via TCP
if is_important_update || !net_delta.should_use_delta() {
    tcp.send(&serialize_message(ServerMessage::FullSnapshot(new_state)));
} else {
    tcp.send(&serialize_message(ServerMessage::DeltaUpdate(net_delta)));
}
```

### UDP (Unreliable Channel)

```rust
// Position-only deltas via UDP (can tolerate loss)
if is_position_only_update() {
    // Simplified delta with only position changes
    udp.send(&position_delta);
}
```

### Interest Management Integration

```rust
// Apply delta only to entities in client's AOI
for client in clients {
    let visible_entities = interest_manager.get_visible(client);
    let filtered_delta = net_delta.filter_entities(&visible_entities);
    send_to_client(client, &filtered_delta);
}
```

## Best Practices

### DO ✅

1. **Use adaptive strategy**: Automatically handles varying workloads
2. **Compute delta asynchronously**: Don't block game tick
3. **Monitor compression ratios**: Log and analyze for optimization
4. **Combine with interest management**: Further reduce bandwidth
5. **Test with realistic workloads**: Verify compression under game conditions

### DON'T ❌

1. **Don't send delta every frame**: Can accumulate on slow clients
2. **Don't skip full snapshots forever**: Send periodic full snapshots for sync
3. **Don't ignore compression ratio**: Always check before sending delta
4. **Don't forget to update strategy**: Record deltas for adaptive learning
5. **Don't compute delta on client**: Only server should compute deltas

## Monitoring & Debugging

### Logging Compression Stats

```rust
use tracing::info;

info!(
    delta_size = net_delta.delta_size,
    full_size = net_delta.full_size,
    compression_ratio = net_delta.compression_ratio,
    decision = net_delta.should_use_delta(),
    "Delta compression stats"
);
```

### Metrics to Track

```rust
struct DeltaMetrics {
    total_deltas_sent: u64,
    total_full_snapshots_sent: u64,
    average_compression_ratio: f32,
    bandwidth_saved_bytes: u64,
}
```

### Performance Profiling

```rust
#[cfg(feature = "profiling")]
use silmaril_profiling::profile_scope;

#[cfg(feature = "profiling")]
profile_scope!("delta_computation");

let delta = WorldStateDelta::compute(&old_state, &new_state);
```

## Troubleshooting

### Problem: Poor Compression (<50% reduction)

**Cause**: Too many entities changing
**Solution**:
- Check if world is stabilizing after initial spawn
- Consider increasing update interval
- Use full snapshots when >40% changes

### Problem: High Computation Time (>20ms)

**Cause**: Large world or inefficient comparison
**Solution**:
- Compute delta asynchronously
- Consider spatial partitioning
- Upgrade to parallel computation (future)

### Problem: Client Desyncs

**Cause**: Delta accumulation errors
**Solution**:
- Send periodic full snapshots (every 10-60 seconds)
- Validate delta application
- Add delta sequence numbers

### Problem: Adaptive Strategy Too Conservative

**Cause**: Threshold too high or bad history
**Solution**:
```rust
// Lower threshold for more aggressive delta usage
let mut strategy = AdaptiveDeltaStrategy::new(10, 0.7);

// Or reset history after major world change
strategy.reset();
```

## Advanced Usage

### Custom Delta Filtering

```rust
// Create delta with only specific component types
let delta = WorldStateDelta::compute(&old_state, &new_state);
let filtered = delta.filter_components(&["Transform", "Velocity"]);
```

### Delta Validation

```rust
// Verify delta application produces expected result
let mut test_state = old_state.clone();
delta.apply(&mut test_state);

assert_eq!(test_state.entities.len(), new_state.entities.len());
assert_eq!(test_state.components.len(), new_state.components.len());
```

### Bandwidth Estimation

```rust
fn estimate_bandwidth(update_rate: f32, avg_delta_size: usize) -> f32 {
    // Bytes per second
    update_rate * avg_delta_size as f32
}

// Example: 60 Hz updates, 15 KB average delta
let bandwidth = estimate_bandwidth(60.0, 15_000); // ~900 KB/s per client
```

## Examples

See `engine/networking/tests/delta_integration_test.rs` for comprehensive examples including:
- Basic delta computation
- Position-only updates
- Entity spawning/despawning
- Full pipeline simulation
- Adaptive strategy usage

## References

- [DELTA_COMPRESSION_REPORT.md](../DELTA_COMPRESSION_REPORT.md) - Implementation details
- [DELTA_COMPRESSION_COMPLETE.md](../DELTA_COMPRESSION_COMPLETE.md) - Benchmark results
- [docs/networking.md](networking.md) - Network architecture
- [engine/networking/benches/delta_benches.rs](../engine/networking/benches/delta_benches.rs) - Performance benchmarks
