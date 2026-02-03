---
name: bench
description: Run performance benchmarks for silmaril
trigger: /bench
---

# Performance Benchmarks

Runs Criterion benchmarks to measure and compare performance of critical code paths.

## Instructions

1. **Run All Benchmarks**
   ```bash
   # Run all workspace benchmarks
   cargo bench --workspace

   # Save baseline for comparison
   cargo bench --workspace -- --save-baseline current
   ```

2. **Run Specific Benchmark**
   If user specifies a particular area:
   ```bash
   # ECS benchmarks only
   cargo bench -p silmaril-core -- ecs

   # Networking benchmarks
   cargo bench -p silmaril-networking

   # Rendering benchmarks
   cargo bench -p silmaril-renderer
   ```

3. **Compare Against Baseline**
   If baseline exists:
   ```bash
   # Compare current performance against saved baseline
   cargo bench --workspace -- --baseline previous
   ```

4. **Generate Performance Report**
   After benchmarks complete:
   - Show performance for each critical operation
   - Compare against performance targets from docs/performance-targets.md
   - Highlight any regressions (>5% slower)
   - Highlight any improvements (>5% faster)
   - Show percentile data (p50, p95, p99)

5. **Check Performance Targets**
   Compare results against targets:

   | Operation | Target | Status |
   |-----------|--------|--------|
   | Spawn 10k entities | < 1ms | PASS/FAIL |
   | Query 10k entities | < 0.5ms | PASS/FAIL |
   | Serialize 1000 entities | < 5ms | PASS/FAIL |
   | Network packet encode | < 100μs | PASS/FAIL |
   | Physics step (1000 bodies) | < 16ms | PASS/FAIL |

6. **Analyze Results**
   - Identify bottlenecks
   - Compare with industry benchmarks if available
   - Suggest optimizations for poor performers
   - Reference docs/performance-targets.md for context

7. **Generate Flamegraphs** (if requested)
   ```bash
   # Install flamegraph if needed
   cargo install flamegraph

   # Generate flamegraph for specific benchmark
   cargo flamegraph --bench ecs_benchmarks
   ```

## Output Format

Provide detailed benchmark summary:

```
Benchmark Results
=================

ECS Performance:
  spawn_entity              543 ns/iter   ✓ (target: < 1μs)
  spawn_10k_entities        812 μs/iter   ✓ (target: < 1ms)
  query_single_component    156 ns/iter   ✓ (target: < 500ns)
  query_10k_entities        423 μs/iter   ✓ (target: < 500μs)
  query_triple_component    234 ns/iter   ✓ (target: < 1μs)

Serialization Performance:
  serialize_entity          1.2 μs/iter   ✓ (target: < 5μs)
  serialize_1000_entities   3.4 ms/iter   ✓ (target: < 5ms)
  deserialize_entity        987 ns/iter   ✓ (target: < 5μs)
  deserialize_1000_entities 2.8 ms/iter   ✓ (target: < 5ms)

Network Performance:
  encode_packet             78 ns/iter    ✓ (target: < 100μs)
  decode_packet             92 ns/iter    ✓ (target: < 100μs)
  tcp_send_1kb             145 μs/iter    ✓ (target: < 1ms)
  udp_send_1kb              34 μs/iter    ✓ (target: < 100μs)

Physics Performance:
  step_100_bodies          1.2 ms/iter    ✓ (target: < 5ms)
  step_1000_bodies         12.4 ms/iter   ✓ (target: < 16ms)
  raycast                  234 ns/iter    ✓ (target: < 1μs)

All benchmarks PASSED
```

With comparisons if baseline exists:

```
Benchmark Comparison (vs. baseline)
===================================

ECS Performance:
  spawn_entity              543 ns/iter   (+2.3%)  regression
  spawn_10k_entities        812 μs/iter   (-5.1%)  improvement ✓
  query_single_component    156 ns/iter   (+0.5%)  no change
  query_10k_entities        423 μs/iter   (-12.3%) improvement ✓✓

Regressions: 1
Improvements: 2
No change: 1

Overall: 2 significant improvements detected
```

## Performance Targets Reference

From docs/performance-targets.md:

### Critical Path Targets
- Entity spawn: < 1μs per entity
- Component query: < 500ns per entity
- Serialization: < 5ms per 1000 entities
- Network packet: < 100μs encode/decode
- Frame time: < 16.67ms (client)
- Server tick: < 16ms (60 TPS)

### Memory Targets
- Client: < 2GB
- Server (1000 players): < 8GB

## Notes

- Benchmarks should run on release builds for accurate results
- Run multiple iterations to reduce noise
- Avoid running benchmarks while system is under load
- Save baselines before making major changes
- Use flamegraphs to identify hotspots in slow benchmarks
- Reference docs/performance-targets.md for all targets
- Compare against industry standards when applicable
