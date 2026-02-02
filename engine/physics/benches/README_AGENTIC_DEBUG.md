# Agentic Debugging Benchmarks

This document describes the comprehensive benchmarks for the agentic debugging system.

## Overview

The agentic debugging system adds performance instrumentation to capture complete physics state for AI agent analysis. These benchmarks measure the overhead and validate that performance targets are met.

## Benchmarks

### 1. Snapshot Creation Overhead

**Command:**
```bash
cargo bench --bench agentic_debug_benches -- snapshot_creation_overhead
```

**Tests:** Snapshot creation time for 1, 100, 1000, and 10000 entities

**Target:** < 1ms for 1000 entities

**Validates:**
- `PhysicsWorld::create_debug_snapshot()` performance
- Scales linearly with entity count
- No memory allocation bottlenecks

### 2. JSONL Export Throughput

**Command:**
```bash
cargo bench --bench agentic_debug_benches -- jsonl_export_throughput
```

**Tests:** JSONL export throughput for 100, 1000, and 10000 snapshots (100 entities each)

**Target:** > 10 MB/sec throughput

**Validates:**
- `JsonlExporter::write_snapshot()` performance
- Streaming write performance
- JSON serialization overhead

### 3. SQLite Export Throughput

**Command:**
```bash
cargo bench --bench agentic_debug_benches -- sqlite_export_throughput
```

**Tests:** SQLite export throughput for 100, 1000, and 10000 snapshots (100 entities each)

**Target:** > 100 frames/sec throughput

**Validates:**
- `SqliteExporter::write_snapshot()` performance
- Database transaction batching
- Index creation overhead

### 4. CSV Export Throughput

**Command:**
```bash
cargo bench --bench agentic_debug_benches -- csv_export_throughput
```

**Tests:** CSV export throughput for 100, 1000, and 10000 snapshots (100 entities each)

**Target:** > 5 MB/sec throughput

**Validates:**
- `CsvExporter::write_snapshot()` performance
- CSV serialization overhead
- File write performance

### 5. entity_history() Query Latency

**Command:**
```bash
cargo bench --bench agentic_debug_benches -- entity_history_query_latency
```

**Tests:** Query latency for databases with 1K, 10K, and 100K frames

**Target:** < 10ms query latency

**Validates:**
- `PhysicsQueryAPI::entity_history()` performance
- SQLite index effectiveness
- Query scaling with database size

### 6. find_high_velocity() Query Latency

**Command:**
```bash
cargo bench --bench agentic_debug_benches -- find_high_velocity_query_latency
```

**Tests:** Query latency for databases with 1K, 10K, and 100K frames

**Target:** < 10ms query latency

**Validates:**
- `PhysicsQueryAPI::find_high_velocity()` performance
- Computed column performance (velocity magnitude)
- WHERE clause optimization

### 7. events_by_type() Query Latency

**Command:**
```bash
cargo bench --bench agentic_debug_benches -- events_by_type_query_latency
```

**Tests:** Query latency for databases with 1K, 10K, and 100K frames

**Target:** < 10ms query latency

**Validates:**
- `PhysicsQueryAPI::events_by_type()` performance
- Event type index effectiveness
- JSON deserialization overhead

### 8. Total Overhead - Physics Step with Debugging

**Command:**
```bash
cargo bench --bench agentic_debug_benches -- total_overhead
```

**Tests:**
- Normal physics step (baseline)
- Physics step + snapshot creation (with debugging)
- Isolated snapshot overhead measurement

**Target:** < 5% overhead compared to normal physics step

**Validates:**
- End-to-end debugging overhead
- Production viability of always-on debugging
- Overhead percentage calculation

### 9. Realistic Physics Scenarios

**Command:**
```bash
cargo bench --bench agentic_debug_benches -- realistic_scenarios
```

**Tests:**
- Falling objects (1000 active entities)
- Mostly sleeping entities (10000 entities, 99% sleeping)

**Target:** Maintains < 5% overhead in realistic scenarios

**Validates:**
- Performance in production-like scenarios
- Sleeping entity optimization
- Large-scale simulation overhead

## Running All Benchmarks

```bash
cargo bench --bench agentic_debug_benches --package engine-physics
```

This will run all benchmarks and generate a report in `target/criterion/`.

## Interpreting Results

Criterion outputs results in the following format:

```
snapshot_creation_overhead/1000
                        time:   [XXX.XX µs XXX.XX µs XXX.XX µs]
                        thrpt:  [XXXX.X elem/s XXXX.X elem/s XXXX.X elem/s]
```

### Performance Targets Summary

| Benchmark | Target | Critical | Validates |
|-----------|--------|----------|-----------|
| Snapshot creation (1000 entities) | < 1ms | < 2ms | Overhead acceptable |
| JSONL export | > 10 MB/s | > 5 MB/s | Streaming performance |
| SQLite export | > 100 frames/s | > 50 frames/s | Batch write performance |
| CSV export | > 5 MB/s | > 2 MB/s | Simple format overhead |
| entity_history query | < 10ms | < 50ms | Query optimization |
| find_high_velocity query | < 10ms | < 50ms | Computed queries |
| events_by_type query | < 10ms | < 50ms | Event filtering |
| **Total overhead** | **< 5%** | **< 10%** | **Production viability** |

## Continuous Integration

These benchmarks should be run in CI to detect performance regressions:

```bash
# Run benchmarks and save baseline
cargo bench --bench agentic_debug_benches -- --save-baseline main

# Compare against baseline (on PR)
cargo bench --bench agentic_debug_benches -- --baseline main
```

## Profiling

To profile snapshot creation or export, use the profiling feature:

```bash
cargo bench --bench agentic_debug_benches --features profiling -- snapshot_creation_overhead/1000 --profile-time=5
```

Then open the profile in Chrome Tracing or Tracy.

## Troubleshooting

### Benchmarks fail to compile

Ensure all dependencies are installed:
- `rusqlite` (bundled SQLite)
- `csv` (CSV writer)
- `serde_json` (JSON serialization)
- `tempfile` (for test files)

### Benchmarks timeout

Reduce the number of samples or iterations:

```bash
cargo bench --bench agentic_debug_benches -- --sample-size 10
```

### High variance in results

Ensure system is idle during benchmarking:
- Close other applications
- Disable power-saving features
- Run multiple times and compare

## Performance Regression Detection

If a benchmark shows > 10% regression:

1. Check git history for changes to:
   - `engine/physics/src/agentic_debug/snapshot.rs`
   - `engine/physics/src/agentic_debug/exporters.rs`
   - `engine/physics/src/agentic_debug/query.rs`

2. Profile the slow path using `cargo flamegraph`:
   ```bash
   cargo flamegraph --bench agentic_debug_benches -- snapshot_creation_overhead/1000 --bench
   ```

3. Check for:
   - New allocations in hot path
   - Missing indices in SQLite queries
   - Unnecessary JSON serialization
   - Missing `#[inline]` on small functions

## Future Improvements

Potential optimizations to benchmark:

- [ ] Parallel snapshot creation (multiple threads)
- [ ] Compression for JSONL export (gzip)
- [ ] Custom binary format (faster than JSON)
- [ ] Memory-mapped SQLite database
- [ ] Incremental snapshots (delta encoding)
- [ ] GPU-accelerated state extraction

## Related Documentation

- [Agentic Debugging Guide](../../../docs/AGENTIC_DEBUGGING_SUMMARY.md)
- [Physics Performance Targets](../../../docs/PHYSICS_AAA_PERFORMANCE_TARGETS.md)
- [Benchmarking Guide](../../../docs/benchmarking.md)
