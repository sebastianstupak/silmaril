# Game Engine Comparison Benchmarks

Practical benchmarks measuring real-world game scenarios that can be compared against Unity, Unreal, Godot, and Bevy.

## Overview

This benchmark suite provides objective performance comparisons between Silmaril and industry-leading game engines. All scenarios are designed to reflect real-world game development workloads.

## Benchmark Scenarios

### Scenario 1: Simple Game Loop (60 FPS Target)

**Description**: Basic game loop with physics and rendering queries.

**Setup**:
- 1000 entities with Position + Velocity components
- Physics update system (applies velocity to position)
- Rendering query system (gathers all visible positions)

**Target**: <16.67ms per frame (60 FPS)

**Comparable To**:
- Unity: GameObject Update loop or DOTS system
- Unreal: Tick function or Mass Entity system
- Godot: Node `_process()` function
- Bevy: Basic system execution

**Run**:
```bash
cargo bench --bench game_engine_comparison -- scenario_1_simple_game_loop
```

---

### Scenario 2: MMO Simulation

**Description**: Server-authoritative multiplayer simulation.

**Setup**:
- 10,000 entities (1,000 players + 9,000 NPCs)
- Components: Position, Health, Inventory, NetworkId, Velocity, Team
- Systems: Movement, Combat, Network Replication
- Complete server tick simulation

**Target**: <16ms (60 TPS)

**Comparable To**:
- Unity: Netcode for GameObjects server tick
- Unreal: Dedicated server tick with replication
- Godot: Multiplayer server with RPCs
- Bevy: Server system with bevy_renet

**Run**:
```bash
cargo bench --bench game_engine_comparison -- scenario_2_mmo_simulation
```

---

### Scenario 3: Asset Loading

**Description**: Bulk asset loading and path normalization.

**Setup**:
- Load 1000 simulated asset files
- Path normalization for each asset
- Simulates I/O and parsing overhead

**Target**: <1000ms total

**Comparable To**:
- Unity: AssetDatabase or AssetBundle loading
- Unreal: Asset Manager async loading
- Godot: ResourceLoader
- Bevy: Asset Server

**Run**:
```bash
cargo bench --bench game_engine_comparison -- scenario_3_asset_loading
```

---

### Scenario 4: State Serialization

**Description**: Complete world state serialization/deserialization.

**Setup**:
- 10,000 entities with Transform, Health, Velocity
- Serialize complete world state to binary (bincode)
- Deserialize and restore world state
- Measures both operations separately and roundtrip

**Target**: <50ms total

**Comparable To**:
- Unity: JsonUtility or custom binary serialization
- Unreal: FArchive serialization
- Godot: JSON or binary resource serialization
- Bevy: serde with bincode/MessagePack

**Run**:
```bash
cargo bench --bench game_engine_comparison -- scenario_4_state_serialization
```

---

### Scenario 5: Spatial Queries

**Description**: Spatial partitioning and radius/AABB queries.

**Setup**:
- 10,000 entities distributed in 3D space
- Spatial grid construction
- Radius queries (10m radius)
- AABB queries (20m x 20m x 20m)

**Target**: <2ms per query

**Comparable To**:
- Unity: Physics.OverlapSphere / Physics.OverlapBox
- Unreal: World->OverlapMulti with PhysX/Chaos
- Godot: PhysicsServer3D spatial queries
- Bevy: bevy_rapier spatial queries

**Run**:
```bash
cargo bench --bench game_engine_comparison -- scenario_5_spatial_queries
```

---

## Running Benchmarks

### All Scenarios

```bash
cargo bench --bench game_engine_comparison
```

### Specific Scenario

```bash
# Scenario 1: Simple Game Loop
cargo bench --bench game_engine_comparison -- scenario_1

# Scenario 2: MMO Simulation
cargo bench --bench game_engine_comparison -- scenario_2

# Scenario 3: Asset Loading
cargo bench --bench game_engine_comparison -- scenario_3

# Scenario 4: State Serialization
cargo bench --bench game_engine_comparison -- scenario_4

# Scenario 5: Spatial Queries
cargo bench --bench game_engine_comparison -- scenario_5
```

### Comprehensive Comparison

```bash
cargo bench --bench game_engine_comparison -- comprehensive_comparison
```

---

## Performance Targets

All targets assume release builds with optimizations:

| Scenario | Target | Acceptable | Critical | Notes |
|----------|--------|------------|----------|-------|
| Frame Time (1K entities) | <1.5ms | <3ms | <5ms | 60 FPS = 16.67ms budget |
| Server Tick (10K entities) | <8ms | <12ms | <16ms | 60 TPS target |
| Asset Loading (1K assets) | <200ms | <500ms | <1000ms | Includes I/O simulation |
| Serialization (10K entities) | <15ms | <30ms | <50ms | Bincode format |
| Spatial Query (10K entities) | <0.5ms | <1ms | <2ms | Radius/AABB queries |

---

## Industry Comparison Data

See [`industry_comparison.yaml`](./industry_comparison.yaml) for detailed performance data from:
- Unity Engine (DOTS and Classic)
- Unreal Engine (Mass Entity and Blueprint)
- Godot Engine
- Bevy Engine

### Expected Performance Multipliers

Approximate comparison vs Silmaril (lower is better):

| Engine | Frame Time | Server Tick | Serialization | Spatial Query |
|--------|-----------|-------------|---------------|---------------|
| **Unity DOTS** | 1.5-3x | 1.5-2x | 2-4x | 1-3x |
| Unity Classic | 10-50x | N/A | 2-4x | 1-3x |
| **Unreal Mass** | 1-2x | 1-2x | 2-3x | 0.8-2x |
| Unreal Blueprint | 5-20x | N/A | 2-3x | 0.8-2x |
| **Godot** | 2-5x | 2-3x | 4-6x | 2-4x |
| **Bevy** | 0.5-1.5x | 0.5-1.5x | 1-2x | 0.5-1.5x |

**Note**: Bevy is our closest competitor in terms of performance. We aim for similar or better performance with additional AI automation features.

---

## Interpreting Results

### Criterion Output

Criterion will generate:
- **Time**: Mean execution time with confidence intervals
- **Throughput**: Operations per second (entities/sec)
- **Change**: Performance delta vs previous run
- **Charts**: HTML visualizations in `target/criterion/`

### Example Output

```
scenario_1_simple_game_loop/1000
                        time:   [1.2456 ms 1.2578 ms 1.2701 ms]
                        thrpt:  [787.38 Kelem/s 795.10 Kelem/s 802.87 Kelem/s]
```

This means:
- **Mean time**: 1.26ms per frame
- **Throughput**: ~795K entities processed per second
- **Status**: ✅ Meets target (<3ms)

### Performance Categories

- **🚀 Excellent**: Beats target by >20%
- **✅ Good**: Meets target within 10%
- **⚠️ Acceptable**: Within acceptable range
- **❌ Critical**: Exceeds critical threshold

---

## Generating Comparison Reports

### Install Report Generator

```bash
pip install pyyaml tabulate matplotlib
```

### Generate Report

```bash
python scripts/generate_comparison_report.py
```

This will:
1. Parse criterion benchmark results
2. Load industry comparison data
3. Calculate performance multipliers
4. Generate markdown report with charts
5. Output to `benchmarks/COMPARISON_REPORT.md`

---

## Hardware Requirements

### Minimum (for valid results)

- CPU: 4 cores, 3.0+ GHz
- RAM: 8GB
- Storage: SSD

### Recommended (matches industry baseline)

- CPU: Intel i7-12700K or AMD Ryzen 7 5800X (8+ cores)
- RAM: 32GB DDR4-3200
- Storage: NVMe PCIe 3.0+
- OS: Windows 11 or Linux (Ubuntu 22.04)

---

## Best Practices

### 1. Consistent Environment

- Close unnecessary applications
- Disable CPU frequency scaling
- Run on AC power (laptops)
- Same OS and hardware for all comparisons

### 2. Multiple Runs

```bash
# Run 3 times and average
for i in {1..3}; do
  cargo bench --bench game_engine_comparison
done
```

### 3. Baseline Comparison

```bash
# Save baseline
cargo bench --bench game_engine_comparison -- --save-baseline main

# After changes
cargo bench --bench game_engine_comparison -- --baseline main
```

### 4. Profiling Integration

```bash
# Run with profiling enabled
cargo bench --bench game_engine_comparison --features profiling

# Open results in puffin viewer
# Or export to Chrome tracing format
```

---

## Troubleshooting

### Benchmark Times Out

Increase measurement time:
```rust
group.measurement_time(Duration::from_secs(30));
```

### Inconsistent Results

- Check CPU throttling
- Ensure no background processes
- Disable hyperthreading for consistency
- Lock CPU frequency

### Memory Issues

Reduce entity counts in benchmarks:
```rust
for entity_count in [100, 500, 1000] { // instead of 10000
```

---

## Contributing New Scenarios

### Template

```rust
fn bench_new_scenario(c: &mut Criterion) {
    let mut group = c.benchmark_group("scenario_X_name");
    group.measurement_time(Duration::from_secs(10));

    for param in [100, 1000, 10000] {
        group.throughput(Throughput::Elements(param as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(param),
            &param,
            |b, &count| {
                // Setup
                let world = setup_world(count);

                b.iter(|| {
                    // Benchmark code
                    let result = do_work(&world);
                    black_box(result);
                });
            },
        );
    }

    group.finish();
}
```

### Requirements

1. **Realistic**: Must reflect real game development workload
2. **Comparable**: Should have equivalent in Unity/Unreal/Godot/Bevy
3. **Documented**: Add to this README with comparison data
4. **Validated**: Include performance targets and rationale

---

## References

### Industry Benchmarks

- [Unity DOTS Performance](https://docs.unity3d.com/Packages/com.unity.entities@latest)
- [Unreal Mass Entity](https://docs.unrealengine.com/5.0/en-US/overview-of-mass-entity-in-unreal-engine/)
- [Godot Performance Tips](https://docs.godotengine.org/en/stable/tutorials/performance/index.html)
- [Bevy ECS Benchmarks](https://github.com/bevyengine/bevy/tree/main/benches)

### Methodology

- [Criterion.rs User Guide](https://bheisler.github.io/criterion.rs/book/)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [Game Engine Benchmarks (Community)](https://github.com/topics/game-engine-benchmarks)

---

## License

Same as main project (Apache-2.0).
