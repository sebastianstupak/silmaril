# Audio System Scalability, Memory, and Cache Benchmarks

**Date:** 2026-02-03
**Status:** Complete
**Location:** `engine/audio/benches/`

---

## Overview

Comprehensive benchmark suite for the audio system covering:
1. **Scalability** - Performance at 1 to 100k simultaneous sounds
2. **Memory** - Allocation tracking, leak detection, fragmentation
3. **Cache Efficiency** - Data locality, access patterns, cache utilization

These benchmarks validate the audio system meets AAA game performance targets and identify optimization opportunities.

---

## Performance Targets

| Metric | Target | Critical |
|--------|--------|----------|
| 10k simultaneous sounds | < 16ms frame time | < 33ms |
| 1k simultaneous sounds | < 1ms frame time | < 5ms |
| Hot path allocations | < 1KB per frame | < 10KB |
| Cache miss rate | < 5% | < 10% |
| Memory per sound | ~1KB metadata | < 10KB |

---

## Benchmark Suites

### 1. Scalability Benchmarks (`scalability_benches.rs`)

**10 benchmark suites** testing audio system performance at scale.

#### Benchmarks

1. **`simultaneous_sounds_frame_time`**
   - Tests: 1, 10, 100, 1k, 10k simultaneous 3D sounds
   - Validates: Full frame update performance
   - Target: < 1ms for 1k sounds, < 16ms for 10k sounds

2. **`listener_update_scaling`**
   - Tests: Listener updates with 10, 100, 1k, 10k emitters
   - Validates: Camera movement performance
   - Target: < 100μs even with 10k emitters

3. **`bulk_emitter_updates`**
   - Tests: Updating 10, 100, 1k, 10k emitter positions
   - Validates: Physics/animation integration performance
   - Target: < 10μs per emitter update

4. **`effect_processing_scale`**
   - Tests: Effect processing with 10, 100, 1k sounds
   - Validates: Reverb/echo overhead at scale
   - Target: < 5ms overhead for 1k sounds

5. **`doppler_calculation_scale`**
   - Tests: Pitch adjustments for 10, 100, 1k, 10k sounds
   - Validates: Doppler effect performance
   - Target: < 5μs per sound

6. **`cleanup_at_scale`**
   - Tests: Cleanup of 10, 100, 1k, 10k finished sounds
   - Validates: Garbage collection performance
   - Target: < 1ms for 1k finished sounds

7. **`emitter_lifecycle_scale`**
   - Tests: Rapid creation/removal of 10, 100, 1k emitters
   - Validates: Memory management efficiency
   - Target: < 1μs per create/remove cycle

8. **`active_sound_tracking_scale`**
   - Tests: Querying active sounds with 10, 100, 1k, 10k emitters
   - Validates: State tracking overhead
   - Target: < 1μs

9. **`aaa_mixed_workload`**
   - Tests: Realistic AAA frame with 1k, 5k, 10k sounds
   - Simulates: Camera movement, emitter updates, queries, cleanup
   - Target: < 5ms total frame time for 5k sounds

10. **`worst_case_max_distance`**
    - Tests: All sounds at maximum distance (100, 1k, 10k)
    - Validates: Distance attenuation performance
    - Target: < 20ms for 10k sounds

#### Usage

```bash
# Run all scalability benchmarks
cargo bench --package engine-audio --bench scalability_benches

# Run specific benchmark
cargo bench --package engine-audio --bench scalability_benches -- simultaneous_sounds

# Quick test
cargo bench --package engine-audio --bench scalability_benches -- --test
```

---

### 2. Memory Benchmarks (`memory_benches.rs`)

**8 benchmark suites** tracking memory allocations and efficiency.

#### Features

- **Global allocation tracking** - Measures exact bytes allocated
- **Allocation counting** - Tracks number of allocations
- **Leak detection** - Verifies memory is freed
- **Fragmentation testing** - Detects memory bloat over time

#### Benchmarks

1. **`play_3d_allocations`**
   - Measures: Memory allocated per `play_3d()` call
   - Target: < 512 bytes per call

2. **`emitter_update_allocations`**
   - Measures: Memory allocated per emitter position update
   - Target: Zero allocations (hot path optimization)

3. **`listener_update_allocations`**
   - Measures: Memory allocated per listener update
   - Target: Zero allocations (hot path optimization)

4. **`frame_allocation_rate`**
   - Tests: Allocation rate per frame with 100, 1k, 5k sounds
   - Logs: Bytes and allocation count per frame
   - Target: < 1KB per frame with 1k active sounds

5. **`peak_memory_usage`**
   - Tests: Total memory footprint with 10, 100, 1k, 10k sounds
   - Logs: Bytes per sound, peak memory
   - Validates: Linear memory scaling

6. **`memory_fragmentation`**
   - Tests: Memory growth over 1000 create/destroy cycles
   - Logs: Memory growth per cycle
   - Target: Stable memory usage (< 1% growth)

7. **`cleanup_memory_reclamation`**
   - Tests: Memory freed by `cleanup_finished()`
   - Validates: Full memory reclamation
   - Target: 100% reclamation within 1 frame

8. **`effect_memory_overhead`**
   - Measures: Memory overhead per audio effect
   - Target: < 128 bytes per effect

#### Usage

```bash
# Run all memory benchmarks
cargo bench --package engine-audio --bench memory_benches

# View allocation logs (includes tracing output)
RUST_LOG=engine_audio=debug cargo bench --package engine-audio --bench memory_benches

# Quick test
cargo bench --package engine-audio --bench memory_benches -- --test
```

#### Implementation Notes

Uses a custom `TrackingAllocator` that wraps the system allocator:

```rust
static ALLOCATED: AtomicUsize = AtomicUsize::new(0);
static ALLOCATION_COUNT: AtomicUsize = AtomicUsize::new(0);

#[global_allocator]
static GLOBAL: TrackingAllocator = TrackingAllocator;
```

**Limitations:**
- Tracks all allocations in the process (includes criterion overhead)
- Use `iter_custom()` to isolate audio system allocations
- Reset tracking before each measurement

---

### 3. Cache Efficiency Benchmarks (`cache_benches.rs`)

**9 benchmark suites** measuring cache performance and data locality.

#### Benchmarks

1. **`sequential_emitter_updates`**
   - Tests: Sequential access with 100, 1k, 10k emitters
   - Validates: Best-case cache performance
   - Expects: Excellent performance due to prefetching

2. **`random_emitter_updates`**
   - Tests: Random access with 100, 1k, 10k emitters
   - Validates: Worst-case cache performance
   - Expects: Significant cache misses

3. **`strided_emitter_access`**
   - Tests: Accessing every 1, 2, 4, 8, 16, 32, 64th emitter
   - Validates: Optimal stride for cache line utilization
   - Identifies: Best batch size

4. **`spatial_query_locality`**
   - Compares: Nearby emitters vs scattered emitters
   - Tests: 10k emitter grid, query 400 nearby vs random
   - Validates: Spatial coherence impact on cache

5. **`bulk_vs_individual`**
   - Compares: Individual updates vs batched updates
   - Tests: 1k emitters
   - Validates: Batching benefits

6. **`cache_line_utilization`**
   - Tests: Batch sizes of 8, 16, 32, 64, 128 emitters
   - Validates: Cache line size impact
   - Identifies: Optimal batch size (typically 16-32)

7. **`prefetch_patterns`**
   - Compares: Forward linear, backward linear, alternating access
   - Tests: 10k emitters
   - Validates: Hardware prefetcher effectiveness

8. **`cold_vs_warm_cache`**
   - Compares: First access (cold) vs repeated access (warm)
   - Tests: 1k emitters
   - Quantifies: Cache warmup benefit

9. **`cache_thrashing`**
   - Tests: Working sets of 1k, 10k, 100k emitters
   - Validates: Performance when working set exceeds cache size
   - Identifies: L3 cache size threshold

#### Usage

```bash
# Run all cache benchmarks
cargo bench --package engine-audio --bench cache_benches

# Compare access patterns
cargo bench --package engine-audio --bench cache_benches -- sequential
cargo bench --package engine-audio --bench cache_benches -- random

# Quick test
cargo bench --package engine-audio --bench cache_benches -- --test
```

#### Interpreting Results

**Good cache efficiency:**
- Sequential faster than random (2-3x)
- Warm cache faster than cold (1.5-2x)
- Small strides (1-4) faster than large strides (32-64)

**Cache thrashing indicators:**
- Performance cliff at specific working set size (L3 cache size)
- Non-linear slowdown as emitter count increases

**Optimization opportunities:**
- If random access is slow: Improve data locality (spatial sorting)
- If large strides are slow: Reduce data structure size
- If cache thrashing occurs: Implement spatial partitioning (grid, octree)

---

## Running All Benchmarks

```bash
# Run all three new benchmark suites
cargo bench --package engine-audio \
  --bench scalability_benches \
  --bench memory_benches \
  --bench cache_benches

# Quick validation test
cargo bench --package engine-audio \
  --bench scalability_benches \
  --bench memory_benches \
  --bench cache_benches \
  -- --test

# Generate flamegraphs (requires cargo-flamegraph)
cargo flamegraph --bench scalability_benches -- --bench
```

---

## Performance Validation

### Scalability Targets

| Sound Count | Frame Time Target | Status |
|-------------|-------------------|--------|
| 1 | < 10μs | ✅ Expected |
| 10 | < 100μs | ✅ Expected |
| 100 | < 500μs | ✅ Expected |
| 1,000 | < 1ms | ⚠️ Validate |
| 10,000 | < 16ms | ⚠️ Validate |
| 100,000 | N/A | 🔍 Experimental |

### Memory Targets

| Operation | Allocation Target | Status |
|-----------|-------------------|--------|
| `play_3d()` | < 512 bytes | ⚠️ Validate |
| `update_emitter_position()` | 0 bytes | ⚠️ Validate |
| `set_listener_transform()` | 0 bytes | ⚠️ Validate |
| Frame (1k sounds) | < 1KB | ⚠️ Validate |
| Memory per sound | ~1KB | ⚠️ Validate |

### Cache Targets

| Pattern | Performance Target | Status |
|---------|-------------------|--------|
| Sequential vs Random | 2-3x faster | ⚠️ Validate |
| Warm vs Cold Cache | 1.5-2x faster | ⚠️ Validate |
| Cache miss rate | < 5% | 🔍 Needs profiling tools |

---

## Baseline Results

**Note:** Run these benchmarks on your development machine to establish baseline performance.

```bash
# Save baseline results
cargo bench --package engine-audio --bench scalability_benches > baseline-scalability.txt
cargo bench --package engine-audio --bench memory_benches > baseline-memory.txt
cargo bench --package engine-audio --bench cache_benches > baseline-cache.txt

# Compare against baseline (requires critcmp)
cargo install critcmp
critcmp baseline-scalability.txt new-scalability.txt
```

---

## Integration with CI/CD

Add to `.github/workflows/benchmarks.yml`:

```yaml
- name: Audio Scalability Benchmarks
  run: |
    cargo bench --package engine-audio --bench scalability_benches -- --test

- name: Audio Memory Benchmarks
  run: |
    cargo bench --package engine-audio --bench memory_benches -- --test

- name: Audio Cache Benchmarks
  run: |
    cargo bench --package engine-audio --bench cache_benches -- --test
```

---

## Related Documentation

- [Audio System Architecture](../audio.md) - Overall audio design
- [Audio Benchmark Comparison](audio-vs-competition.md) - Industry comparison
- [Performance Targets](../performance-targets.md) - Engine-wide targets
- [Profiling Guide](../profiling.md) - Profiling infrastructure

---

## Future Improvements

1. **Platform-specific benchmarks**
   - WASM benchmarks (Web Audio API)
   - Android benchmarks (OpenSL ES)
   - iOS benchmarks (Core Audio)

2. **GPU profiling integration**
   - Measure GPU time for audio DSP
   - Track buffer uploads

3. **Real-world scenarios**
   - Battle scenario (1000 gunshots, explosions)
   - Crowd scenario (500 footsteps, voices)
   - Racing scenario (10 engines with Doppler)

4. **Automated regression detection**
   - Fail CI if benchmarks regress > 10%
   - Track performance over time

---

## Summary

### Deliverables

✅ **3 new benchmark files** with comprehensive coverage:
- `engine/audio/benches/scalability_benches.rs` (10 suites)
- `engine/audio/benches/memory_benches.rs` (8 suites)
- `engine/audio/benches/cache_benches.rs` (9 suites)

✅ **27 total benchmark suites** covering:
- Scalability: 1 to 100k simultaneous sounds
- Memory: Allocation tracking, leak detection, fragmentation
- Cache: Data locality, access patterns, utilization

✅ **Performance targets defined** and ready for validation

✅ **Documentation updated**:
- `engine/audio/README.md` - Benchmark usage guide
- `docs/benchmarks/audio-scalability-memory-cache-benchmarks.md` - This file

### Next Steps

1. Run benchmarks on reference hardware to establish baselines
2. Validate performance targets (especially 10k sounds < 16ms)
3. Add platform-specific benchmarks (WASM, Android, iOS)
4. Integrate with CI/CD for regression detection
5. Profile cache miss rate using hardware counters (perf, VTune)

---

**Maintained by:** Audio Team
**Last Updated:** 2026-02-03
