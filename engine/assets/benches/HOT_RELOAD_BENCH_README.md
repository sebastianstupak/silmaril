# Hot-Reload System Benchmarks

Performance benchmarks for the hot-reload system.

## Benchmark Suite

### Creation & Setup
- `hot_reloader_creation` - Time to create a HotReloader instance
- `watch_registration` - Time to register/unregister a directory watch

### Asset Tracking
- `asset_registration` - Time to register assets for hot-reload (1, 10, 100, 1000 assets)
- `path_mapping_lookup` - Time to look up AssetId by path (with 1000 registered assets)

### Event Processing
- `event_processing` - Time to process events (batching on/off)
- `reload_queue_operations` - Time to queue reloads (batch sizes: 1, 5, 10, 50)

### Configuration
- `debouncing_overhead` - Impact of debounce duration (0ms, 100ms, 300ms, 500ms)
- `stats_collection` - Time to collect statistics

### Memory
- `memory_overhead` - Memory footprint of tracking (10, 100, 1000, 10000 assets)

## Running Benchmarks

### All benchmarks
```bash
cargo bench --package engine-assets --features hot-reload --bench hot_reload_benches
```

### Specific benchmark
```bash
cargo bench --package engine-assets --features hot-reload --bench hot_reload_benches -- hot_reloader_creation
```

### With baseline comparison
```bash
# Save baseline
cargo bench --package engine-assets --features hot-reload --bench hot_reload_benches -- --save-baseline main

# Compare after changes
cargo bench --package engine-assets --features hot-reload --bench hot_reload_benches -- --baseline main
```

## Performance Targets

| Metric | Target | Critical | Notes |
|--------|--------|----------|-------|
| HotReloader creation | < 1ms | < 10ms | Initialization overhead |
| Watch registration | < 10ms | < 100ms | Directory setup |
| Asset registration | < 10μs/asset | < 100μs/asset | Tracking overhead |
| Path lookup | < 1μs | < 10μs | HashMap lookup |
| Event processing | < 1ms | < 10ms | Per-frame overhead |
| Debounce overhead | < 100μs | < 1ms | Config impact |
| Memory overhead | < 1KB/asset | < 10KB/asset | Tracking structures |

## Expected Results

Based on the implementation:

### Fast Operations (< 10μs)
- Asset registration (HashMap insert)
- Path lookup (HashMap get)
- Stats collection (simple struct copy)

### Medium Operations (< 1ms)
- HotReloader creation (channel setup)
- Event processing (channel recv)
- Reload queue operations (VecDeque ops)

### Slow Operations (< 10ms)
- Watch registration (notify setup)
- Batch reload flushing (actual file I/O)

## Profiling Tips

### For detailed profiling
```bash
# With flamegraph
cargo flamegraph --bench hot_reload_benches --features hot-reload

# With perf
cargo bench --package engine-assets --features hot-reload --bench hot_reload_benches -- --profile-time=5
```

### Memory profiling
```bash
# With valgrind
valgrind --tool=massif cargo bench --package engine-assets --features hot-reload --bench hot_reload_benches -- memory_overhead --profile-time=1
```

## Optimization Notes

### Current Optimizations
- HashMap for O(1) path/ID lookups
- VecDeque for efficient queue operations
- Atomic refcounting in AssetHandle
- Zero-allocation debouncing (reuse HashMap entries)

### Potential Optimizations
- Use DashMap for concurrent access (if needed)
- Pool allocation for event objects
- Custom allocator for tracking structures
- Lazy initialization of watchers

## Regression Testing

Run benchmarks before/after changes:

```bash
# Before
git checkout main
cargo bench --package engine-assets --features hot-reload --bench hot_reload_benches -- --save-baseline main

# After changes
git checkout feature-branch
cargo bench --package engine-assets --features hot-reload --bench hot_reload_benches -- --baseline main
```

Criterion will report performance regressions automatically.
