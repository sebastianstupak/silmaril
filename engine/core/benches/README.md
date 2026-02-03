# ECS Core Benchmarks

This directory contains comprehensive benchmarks for the silmaril ECS system.

## Available Benchmarks

### Performance Comparison Benchmarks

#### `ecs_performance.rs` (NEW)
**Status:** ✅ Ready (pending core library compilation fixes)

Comprehensive ECS performance benchmarks comparing against industry standards:
- **Entity Creation**: Spawn 1M-10M entities, measure allocation performance
- **Component Iteration**: Iterate 1M+ entities with 1-3 components
- **Component Operations**: Add/remove/get operations with timing
- **Query Performance**: Simple, sparse, and complex query benchmarks
- **Archetype Changes**: Measure archetype migration overhead
- **Game Scenarios**: Realistic MMORPG simulation (10k entities)

**Industry Comparisons:**
- Bevy ECS (Rust, archetype-based)
- hecs (Rust, minimalist)
- EnTT (C++, reference: 0.8ns/entity iteration, 4.9ns/entity creation)
- Flecs (C/C++, query-focused)

**Documentation:** See [docs/ecs-performance-benchmarking.md](../../../docs/ecs-performance-benchmarking.md)

**Running:**
```bash
cargo bench --bench ecs_performance
cargo bench --bench ecs_performance -- entity_creation
cargo bench --bench ecs_performance -- --save-baseline main
```

---

### Existing Benchmarks

#### `ecs_simple.rs`
Simple, focused ECS benchmarks for quick performance checks.

#### `ecs_comprehensive.rs`
Comprehensive ECS benchmarks with statistical analysis.

#### `entity_benches.rs`
Entity allocation and deallocation benchmarks.

#### `query_benches.rs`
Query system performance benchmarks.

#### `sparse_set_benches.rs`
Sparse set storage benchmarks.

#### `world_benches.rs`
World operations benchmarks.

#### `change_detection.rs`
Change detection system benchmarks.

---

## Running Benchmarks

### Quick Start

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark file
cargo bench --bench ecs_performance

# Run specific benchmark group
cargo bench --bench ecs_performance -- entity_creation

# Run with baseline comparison
cargo bench --bench ecs_performance -- --save-baseline main
cargo bench --bench ecs_performance -- --baseline main
```

### Advanced Options

```bash
# Increase sample size
cargo bench --bench ecs_performance -- --sample-size 200

# Increase measurement time
cargo bench --bench ecs_performance -- --measurement-time 20

# Verbose output
cargo bench --bench ecs_performance -- --verbose

# Generate comparison report
cargo bench --bench ecs_performance -- --save-baseline main
# ... make changes ...
cargo bench --bench ecs_performance -- --baseline main
```

---

## Interpreting Results

### Criterion Output

```
entity_creation/spawn_entities/1000000
                        time:   [876.23 ms 884.56 ms 893.41 ms]
                        thrpt:  [1.1194 Melem/s 1.1304 Melem/s 1.1412 Melem/s]
```

- **time**: [lower bound, median, upper bound] in milliseconds
- **thrpt**: Throughput (elements/sec, higher is better)

### Per-Entity Metrics

Convert total time to per-entity time:
```
Time per entity = Total time / Entity count
Example: 884.56 ms / 1,000,000 = 0.88456 µs/entity = 884.56 ns/entity
```

### Performance Targets

| Operation | Target | Industry Baseline |
|-----------|--------|-------------------|
| Entity creation | <1µs/entity | EnTT: 4.9ns, Bevy: ~50-100ns |
| 1 component iteration | <100ns/entity | EnTT: 0.8ns, Bevy: ~5-10ns |
| 2 component iteration | <200ns/entity | EnTT: 4.2ns, Bevy: ~10-20ns |
| Component add | <1µs | Similar across frameworks |
| Component remove | <1µs | Similar across frameworks |
| Component get | <20ns | ~10-20ns typical |

**Color Coding:**
- ✅ Green: Within target
- ⚠️ Yellow: 1-2× target (acceptable)
- ❌ Red: >2× target (needs optimization)

---

## Benchmark Results Location

Results are saved in `target/criterion/`:

```
target/criterion/
├── ecs_performance/
│   ├── entity_creation/
│   │   ├── spawn_entities/
│   │   │   ├── 1000/
│   │   │   ├── 10000/
│   │   │   ├── 100000/
│   │   │   └── 1000000/
│   │   └── report/
│   ├── component_iteration/
│   ├── component_operations/
│   ├── query_performance/
│   ├── archetype_changes/
│   └── game_scenarios/
└── report/
    └── index.html  ← Open this for visual reports
```

---

## Profiling Integration

For detailed performance analysis:

```bash
# Run with profiling enabled
cargo bench --bench ecs_performance --features profiling

# Generate flame graph
cargo flamegraph --bench ecs_performance -- --bench

# Use perf (Linux only)
perf record -g cargo bench --bench ecs_performance -- --bench
perf report
```

---

## CI/CD Integration

Benchmarks are automatically run on CI for performance regression detection.

See `.github/workflows/benchmark-regression.yml` for configuration.

---

## Troubleshooting

### Compilation Errors

**Issue:** `cargo bench --bench ecs_performance` fails to compile

**Solution:** The core ECS library may have ongoing development work. Check:
1. `git status` - Are there uncommitted changes?
2. `cargo build -p engine-core` - Does the core library compile?
3. Check recent commits - Was a breaking change introduced?

### Inconsistent Results

**Issue:** Benchmark results vary significantly between runs

**Solutions:**
1. Close unnecessary applications
2. Disable CPU frequency scaling (Linux: `cpupower frequency-set -g performance`)
3. Increase sample size: `--sample-size 200`
4. Increase measurement time: `--measurement-time 20`

### Out of Memory

**Issue:** Benchmark crashes with OOM error

**Solutions:**
1. Reduce entity count in large-scale benchmarks
2. Run benchmarks individually rather than all at once
3. Monitor memory usage with `htop` or Task Manager

---

## Adding New Benchmarks

### Checklist

1. Create benchmark file in `engine/core/benches/`
2. Add `[[bench]]` entry to `engine/core/Cargo.toml`
3. Follow naming convention: `{category}_{description}.rs`
4. Use Criterion for statistical analysis
5. Document targets and industry comparisons
6. Add to this README

### Template

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use engine_core::ecs::{Component, World};

fn bench_my_feature(c: &mut Criterion) {
    let mut group = c.benchmark_group("my_feature");

    group.bench_function("operation", |b| {
        let mut world = World::new();
        world.register::<MyComponent>();

        b.iter(|| {
            // Benchmark code here
            black_box(&world);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_my_feature);
criterion_main!(benches);
```

---

## Resources

### Documentation

- [ECS Performance Benchmarking Guide](../../../docs/ecs-performance-benchmarking.md)
- [Platform Benchmark Comparison](../../../PLATFORM_BENCHMARK_COMPARISON.md)
- [ECS Architecture](../../../docs/ecs.md)
- [Performance Targets](../../../docs/performance-targets.md)

### External References

- [Criterion.rs Documentation](https://bheisler.github.io/criterion.rs/book/)
- [Bevy ECS Benchmarks](https://metrics.bevy.org/)
- [EnTT ECS Benchmark](https://github.com/abeimler/ecs_benchmark)
- [Rust ECS Benchmark Suite](https://github.com/rust-gamedev/ecs_bench_suite) (archived)

---

**Last Updated:** 2026-02-01
**Maintainer:** Engine Core Team
