# ECS Performance Benchmarking Guide

**Last Updated:** 2026-02-01
**Status:** Active
**Purpose:** Guide for running and interpreting ECS performance benchmarks

---

## Executive Summary

This document provides comprehensive guidance for benchmarking the agent-game-engine ECS against industry standards. We compare against:

- **Bevy ECS** (Rust, archetype-based, production-ready)
- **hecs** (Rust, minimalist, high-performance)
- **EnTT** (C++, reference implementation: 0.8ns/entity iteration, 4.9ns/entity creation)
- **Flecs** (C/C++, query-focused with caching)

Our targets are based on research compiled in `PLATFORM_BENCHMARK_COMPARISON.md`.

---

## Quick Start

```bash
# Run all ECS performance benchmarks
cargo bench --bench ecs_performance

# Run specific benchmark group
cargo bench --bench ecs_performance -- entity_creation
cargo bench --bench ecs_performance -- component_iteration
cargo bench --bench ecs_performance -- component_operations
cargo bench --bench ecs_performance -- query_performance
cargo bench --bench ecs_performance -- archetype_changes
cargo bench --bench ecs_performance -- game_scenarios

# Save baseline for future comparisons
cargo bench --bench ecs_performance -- --save-baseline main

# Compare against baseline
cargo bench --bench ecs_performance -- --baseline main
```

---

## Performance Targets

### 1. Entity Creation

| Metric | Target | Industry Baseline | Notes |
|--------|--------|-------------------|-------|
| **Spawn entity (bare)** | **<1µs per entity** | EnTT: 4.9ns, Bevy/hecs similar | Allocation only |
| **Spawn 1M entities** | **<1 second** | EnTT: 49ms for 10M | Batch allocation |
| **Spawn 10M entities** | **<10 seconds** | EnTT: 49ms for 10M (old data) | Stress test |
| **Spawn with 1 component** | **<1.5µs per entity** | Similar to bare + component add | Includes archetype setup |
| **Spawn with 2 components** | **<2µs per entity** | Similar to bare + 2× component add | Multi-component archetype |
| **Spawn with 3 components** | **<2.5µs per entity** | Similar to bare + 3× component add | Complex entity setup |

**Assessment Criteria:**
- ✅ Green: Within target
- ⚠️ Yellow: 1-2× target (acceptable)
- ❌ Red: >2× target (needs optimization)

---

### 2. Component Iteration

| Metric | Target | Industry Baseline | Notes |
|--------|--------|-------------------|-------|
| **1 component (1M entities)** | **<100ms** | EnTT: 8ms for 10M (0.8ns/entity) | Sequential memory access |
| **2 components (1M entities)** | **<200ms** | EnTT: 42ms for 10M (4.2ns/entity) | Dual storage access |
| **3 components (1M entities)** | **<300ms** | Estimated ~6-8ns/entity | Triple storage access |
| **Throughput** | **10M+ entities/sec** | EnTT: 1.25B entities/sec (1 comp) | Cache-friendly iteration |

**Per-Entity Targets:**
- 1 component: <100ns/entity
- 2 components: <200ns/entity
- 3 components: <300ns/entity

**Assessment Criteria:**
- ✅ Green: ≥10M entities/sec
- ⚠️ Yellow: 5-10M entities/sec
- ❌ Red: <5M entities/sec

---

### 3. Component Operations

| Operation | Target | Industry Baseline | Notes |
|-----------|--------|-------------------|-------|
| **Add component** | **<1µs** | EnTT: <1µs typical | May trigger archetype migration |
| **Remove component** | **<1µs** | EnTT: <1µs typical | May trigger archetype migration |
| **Get component (immutable)** | **<20ns** | ~10-20ns (pointer deref + bounds check) | Read-only access |
| **Get component (mutable)** | **<50ns** | ~20-50ns (includes change tracking) | Write access |
| **Batch add (3 components)** | **<3µs** | 3× single add overhead | Multiple components at once |

**Assessment Criteria:**
- ✅ Green: Within target
- ⚠️ Yellow: 1-2× target
- ❌ Red: >2× target

---

### 4. Query Performance

| Query Type | Target | Industry Baseline | Notes |
|------------|--------|-------------------|-------|
| **Simple query (100% match)** | **10M+ entities/sec** | EnTT: similar to iteration | All entities have components |
| **Sparse query (10% match)** | **100M+ checks/sec** | Varies by implementation | Fast rejection |
| **Complex query (4 components)** | **5M+ entities/sec** | ~50% of 2-component speed | Multiple storage access |

**Assessment Criteria:**
- ✅ Green: Meets throughput targets
- ⚠️ Yellow: 50-100% of target
- ❌ Red: <50% of target

---

### 5. Archetype Changes

| Operation | Target | Industry Baseline | Notes |
|-----------|--------|-------------------|-------|
| **Add component (migration)** | **<5µs** | Bevy: varies by archetype size | Moves entity to new archetype |
| **Remove component (migration)** | **<5µs** | Bevy: varies by archetype size | Moves entity back |
| **Add/remove cycle** | **<10µs** | 2× single migration cost | Full round-trip |

**Note:** Archetype migration is more expensive than simple component add/remove because it involves moving the entity between storage tables. This is a fundamental characteristic of archetype-based ECS designs.

**Assessment Criteria:**
- ✅ Green: Within target
- ⚠️ Yellow: 1-3× target (archetype migrations are expensive)
- ❌ Red: >3× target

---

### 6. Game Scenarios

| Scenario | Entity Count | Target Frame Time | Notes |
|----------|--------------|-------------------|-------|
| **MMORPG simulation** | 10,000 | **<5ms** | 6k NPCs, 3k projectiles, 1k players |
| **Large-scale RTS** | 100,000 | **<16ms** | Many static entities, few updates |

**Assessment Criteria:**
- ✅ Green: <5ms (200+ FPS headroom)
- ⚠️ Yellow: 5-10ms (100-200 FPS headroom)
- ❌ Red: >10ms (<100 FPS headroom)

---

## Running Benchmarks

### Prerequisites

```bash
# Ensure you're in release mode (benchmarks automatically use --release)
# Ensure no other heavy processes are running
# Close unnecessary applications for accurate results
```

### Basic Benchmark Run

```bash
# Run all ECS performance benchmarks
cargo bench --bench ecs_performance

# Results are saved in:
# target/criterion/ecs_performance/
```

### Advanced Options

```bash
# Set sample size (default: varies by benchmark)
cargo bench --bench ecs_performance -- --sample-size 100

# Set measurement time (default: 10 seconds)
cargo bench --bench ecs_performance -- --measurement-time 10

# Run only entity creation benchmarks
cargo bench --bench ecs_performance -- "1_entity_creation"

# Run specific benchmark within a group
cargo bench --bench ecs_performance -- "spawn_entities/1000000"

# Verbose output
cargo bench --bench ecs_performance -- --verbose

# Generate plots (requires gnuplot)
cargo bench --bench ecs_performance -- --plotting-backend gnuplot
```

### Baseline Comparisons

```bash
# Save current results as baseline
cargo bench --bench ecs_performance -- --save-baseline main

# Make changes to ECS implementation...

# Compare against baseline
cargo bench --bench ecs_performance -- --baseline main

# Save new baseline
cargo bench --bench ecs_performance -- --save-baseline optimized

# Compare two baselines
cargo bench --bench ecs_performance -- --baseline main --baseline optimized
```

---

## Interpreting Results

### Criterion Output Format

```
entity_creation/spawn_entities/1000000
                        time:   [876.23 ms 884.56 ms 893.41 ms]
                        thrpt:  [1.1194 Melem/s 1.1304 Melem/s 1.1412 Melem/s]
Found 3 outliers among 100 measurements (3.00%)
  2 (2.00%) high mild
  1 (1.00%) high severe
```

**Reading the output:**
- **time**: [lower bound, median, upper bound] - Time taken for benchmark
- **thrpt**: Throughput (elements/sec, higher is better)
- **Outliers**: Measurements significantly different from the median (ignore these)

### Converting to Per-Entity Metrics

```
Time per entity = Total time / Entity count

Example:
  Total time: 884.56 ms for 1,000,000 entities
  Per-entity: 884.56 ms / 1,000,000 = 0.88456 µs/entity = 884.56 ns/entity
```

### Comparing Against Targets

```
Target: <1µs per entity
Actual: 884.56 ns/entity

Result: ✅ Within target (88% of target)
```

### Statistical Significance

Criterion reports statistical significance when comparing baselines:

```
change: [-5.2134% -2.1345% +1.0234%] (p = 0.42 > 0.05)
        No change in performance detected.
```

- **p < 0.05**: Statistically significant change
- **p > 0.05**: Change within noise range

---

## Industry Comparison Table

### Entity Creation (per entity)

| Framework | Language | Bare Entity | With 1 Component | Notes |
|-----------|----------|-------------|------------------|-------|
| **Our ECS** | **Rust** | **TBD** | **TBD** | **Archetype-based** |
| EnTT | C++ | 4.9ns | ~10ns (estimated) | Historical benchmark |
| Bevy ECS | Rust | ~50-100ns | ~100-200ns | Active development |
| hecs | Rust | ~30-60ns | ~80-150ns | Minimalist |
| Flecs | C/C++ | Fast | Fast | Query-focused |

### Component Iteration (per entity)

| Framework | Language | 1 Component | 2 Components | Notes |
|-----------|----------|-------------|--------------|-------|
| **Our ECS** | **Rust** | **TBD** | **TBD** | **Archetype-based** |
| EnTT | C++ | 0.8ns | 4.2ns | Historical benchmark (10M entities) |
| Bevy ECS | Rust | ~5-10ns | ~10-20ns | Recent improvements (3.5× speedup) |
| hecs | Rust | ~3-8ns | ~8-15ns | Often fastest in benchmarks |
| Flecs | C/C++ | Very fast (cached) | Very fast (cached) | Query caching advantage |

**Note:** These are approximate values based on various sources. Actual performance varies by:
- Hardware (CPU, cache size, memory speed)
- Compiler version and optimization flags
- Entity count and component distribution
- Query patterns and cache locality

---

## Optimization Tips

### If Entity Creation is Slow

1. **Pre-allocate entities**: Use `World::with_capacity()`
2. **Batch spawning**: Spawn entities in bulk rather than one-by-one
3. **Reduce archetype fragmentation**: Group entities with similar components

### If Component Iteration is Slow

1. **Check cache locality**: Ensure components are stored contiguously
2. **Reduce component size**: Smaller components = better cache utilization
3. **Use parallel iteration**: Enable rayon for large entity counts
4. **Minimize component count**: Only query what you need

### If Component Operations are Slow

1. **Batch operations**: Add/remove multiple components at once
2. **Avoid frequent archetype migrations**: Group component changes
3. **Use system ordering**: Change detection can add overhead

### If Query Performance is Slow

1. **Filter early**: Reject entities that don't match as early as possible
2. **Cache query results**: If querying same entities repeatedly
3. **Use change detection**: Only process entities that changed
4. **Optimize component order**: Put smallest components first

### If Archetype Changes are Slow

1. **Minimize migrations**: Design entities to rarely change archetypes
2. **Batch migrations**: Change multiple components at once
3. **Use flags instead**: Consider bitflags instead of adding/removing components
4. **Profile hotspots**: Identify which archetype migrations are most expensive

---

## Profiling Integration

For deeper analysis, use profiling tools:

```bash
# Run with profiling enabled
cargo bench --bench ecs_performance --features profiling

# Generate flame graph
cargo flamegraph --bench ecs_performance -- --bench

# Use perf (Linux)
perf record -g cargo bench --bench ecs_performance -- --bench
perf report
```

See [docs/profiling.md](./profiling.md) for complete profiling guide.

---

## CI/CD Integration

### GitHub Actions

```yaml
name: ECS Performance Regression

on: [push, pull_request]

jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      # Run benchmarks and save baseline
      - name: Run ECS benchmarks
        run: cargo bench --bench ecs_performance -- --save-baseline main

      # Upload results
      - uses: actions/upload-artifact@v3
        with:
          name: benchmark-results
          path: target/criterion/
```

See `.github/workflows/benchmark-regression.yml` for complete workflow.

---

## Troubleshooting

### Benchmarks are Inconsistent

**Symptoms**: Large variance in results, many outliers

**Solutions:**
1. Close all unnecessary applications
2. Disable CPU frequency scaling (Linux: `cpupower frequency-set -g performance`)
3. Pin process to specific CPU cores
4. Increase sample size: `--sample-size 200`
5. Increase measurement time: `--measurement-time 20`

### Benchmarks Crash or Panic

**Symptoms**: Benchmark aborts with error

**Solutions:**
1. Check entity limits (memory exhaustion)
2. Verify component registration (panic if not registered)
3. Run in debug mode first: `cargo test --bench ecs_performance`
4. Enable backtrace: `RUST_BACKTRACE=1 cargo bench --bench ecs_performance`

### Results Don't Match Targets

**Symptoms**: Performance is worse than expected

**Solutions:**
1. Verify release mode: `cargo bench` automatically uses `--release`
2. Check CPU governor: Should be "performance" not "powersave"
3. Profile hotspots: Use `cargo flamegraph`
4. Compare against similar hardware benchmarks
5. Check for debug assertions: Ensure `debug-assertions = false` in Cargo.toml

---

## Future Work

### Planned Improvements

1. **Parallel iteration benchmarks**: Add rayon-based parallel query benchmarks
2. **Cross-platform baselines**: Establish separate targets for Windows/Linux/macOS
3. **Memory profiling**: Add heap allocation and memory usage benchmarks
4. **Change detection benchmarks**: Benchmark change detection query performance
5. **Serialization benchmarks**: Benchmark world state serialization/deserialization

### Research Areas

1. **Archetype optimization**: Investigate archetype table layouts for better cache locality
2. **Query optimization**: Explore query caching and pre-compiled queries
3. **SIMD opportunities**: Identify component operations suitable for SIMD
4. **Allocation strategies**: Compare different allocator strategies for entities

---

## References

### Research Documents

- [PLATFORM_BENCHMARK_COMPARISON.md](../PLATFORM_BENCHMARK_COMPARISON.md) - Comprehensive industry benchmark research
- [docs/ecs.md](./ecs.md) - ECS architecture documentation
- [docs/performance-targets.md](./performance-targets.md) - Overall engine performance targets

### External Benchmarks

- [EnTT ECS Benchmark](https://github.com/abeimler/ecs_benchmark) - C++ ECS comparison
- [Rust ECS Benchmark Suite](https://github.com/rust-gamedev/ecs_bench_suite) - Archived but useful
- [Bevy Metrics](https://metrics.bevy.org/) - Bevy engine performance tracking
- [Flecs Benchmarks](https://github.com/SanderMertens/ecs_benchmark) - Flecs official benchmarks

### Tools

- [Criterion.rs](https://github.com/bheisler/criterion.rs) - Benchmarking framework
- [cargo-flamegraph](https://github.com/flamegraph-rs/flamegraph) - Flame graph generation
- [perf](https://perf.wiki.kernel.org/index.php/Main_Page) - Linux profiling tool (Linux only)

---

**Document Status:** Active
**Maintainer:** Engine Core Team
**Last Reviewed:** 2026-02-01
