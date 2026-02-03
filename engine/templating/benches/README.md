# Template System Benchmarks

This directory contains comprehensive benchmarks for the template system.

## Available Benchmarks

### 1. `template_loading.rs`
Measures template loading performance across different scenarios:
- Small templates (1 entity)
- Medium templates (100 entities)
- Large templates (1000 entities)
- Templates with references
- Cache hit performance
- Scaling with entity counts
- Despawn performance
- Nested children loading

**Performance Targets:**
- Small (1 entity): < 1ms
- Medium (100 entities): < 10ms
- Large (1000 entities): < 100ms
- Cache hit: < 0.1ms

### 2. `template_validation.rs`
Benchmarks validation performance for templates.

### 3. `template_spawning.rs`
Measures entity spawning performance from templates into the World.

### 4. `hot_reload.rs`
Benchmarks hot-reload performance for template changes.

### 5. `memory_usage.rs` (NEW)
Measures memory consumption of template system operations:

**Benchmarks:**
1. `bench_yaml_memory` - Memory used by Template loaded from YAML
2. `bench_cache_memory` - Memory overhead of template cache (multiple templates)
3. `bench_spawned_entities_memory` - Memory of spawned entities in World
4. `bench_template_in_memory_size` - Compare YAML-parsed vs direct construction
5. `bench_cache_vs_reload_memory` - Cache efficiency for memory usage

**Performance Targets:**
- Bincode < 50% of YAML memory (when compiler available)
- Cache overhead < 1KB per template
- Spawned entities < 500 bytes per entity

**Methodology:**
Uses a custom global allocator wrapper (`TrackingAllocator`) to measure peak memory usage during template operations. The allocator tracks:
- Heap allocations during parsing
- In-memory size of Template structures
- World entity storage overhead

**Note:** The `bench_bincode_memory` benchmark is currently disabled (commented out) until the compiler module (Task #10) is implemented. Re-enable it when bincode compilation is available.

### 6. `yaml_vs_bincode.rs`
Compares YAML vs Bincode template loading performance (requires compiler module - Task #10).

**Note:** This benchmark is currently non-functional as it depends on the `TemplateCompiler` which is not yet implemented.

### 7. `component_parsing.rs`
Benchmarks component parsing performance.

## Running Benchmarks

### Run all benchmarks
```bash
cargo xtask bench templating
```

### Run specific benchmark
```bash
cargo bench --package engine-templating --bench memory_usage
```

### Run with specific filter
```bash
cargo bench --package engine-templating --bench memory_usage yaml_memory
```

### Generate detailed reports
```bash
cargo bench --package engine-templating --bench memory_usage -- --verbose
```

## Benchmark Output

Criterion generates reports in `target/criterion/`. View the HTML reports:
```bash
# Open the index
open target/criterion/report/index.html

# Or specific benchmark
open target/criterion/yaml_memory/report/index.html
```

## Adding New Benchmarks

1. Create new file in `engine/templating/benches/`
2. Add benchmark entry to `engine/templating/Cargo.toml`:
   ```toml
   [[bench]]
   name = "your_benchmark_name"
   harness = false
   ```
3. Use criterion framework:
   ```rust
   use criterion::{criterion_group, criterion_main, Criterion};

   fn bench_your_feature(c: &mut Criterion) {
       c.bench_function("your_feature", |b| {
           b.iter(|| {
               // Your benchmark code
           });
       });
   }

   criterion_group!(benches, bench_your_feature);
   criterion_main!(benches);
   ```

## Performance Tracking

Results are tracked in:
- `docs/benchmarks/` - Historical benchmark results
- CI/CD pipeline - Regression detection

## Tips

- Use `black_box()` to prevent compiler optimizations
- Use `iter_with_setup()` for benchmarks requiring setup/teardown
- Set appropriate `sample_size` for expensive operations
- Use `throughput()` to report elements/bytes processed
- Group related benchmarks with `benchmark_group()`
