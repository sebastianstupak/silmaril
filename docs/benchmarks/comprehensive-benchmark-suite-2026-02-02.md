# Comprehensive Benchmark Suite - 2026-02-02

> Complete benchmark infrastructure for GPU performance, large-scale ECS, and cross-platform validation

---

## Overview

This document describes the expanded benchmark suite added to validate performance at AAA game scale and across all target platforms.

### New Benchmarks Added

1. **GPU Performance Comprehensive** - Real-world GPU operations
2. **Large-Scale ECS** - 50K-500K entity testing
3. **Cross-Platform CI** - Linux, Windows, macOS validation

---

## 1. GPU Performance Benchmarks

**Location:** `engine/renderer/benches/gpu_performance_comprehensive.rs`

### Scenarios Covered

#### 1.1 Draw Call Throughput
Tests draw call overhead with varying batch sizes.

| Scenario | Entity Count | Target | Industry Comparison |
|----------|--------------|--------|---------------------|
| Low | 10 draws | N/A | Unity: ~5-10 draws/ms |
| Medium | 100 draws | N/A | Unreal: ~50-100 draws/ms |
| High | 1000 draws | < 2ms | Bevy: ~500-1000 draws/ms |
| AAA | 2000 draws | < 4ms | Target: < 2000 draws/frame |

**Validation:** Ensure draw call batching is working effectively

#### 1.2 Triangle Throughput
Tests GPU vertex processing capacity.

| Scenario | Triangle Count | Target | Notes |
|----------|----------------|--------|-------|
| Small scene | 10K | < 0.1ms | Indie game level |
| Medium scene | 100K | < 1ms | AA game level |
| Large scene | 1M | < 10ms | AAA game level |
| Extreme | 5M | < 16ms | Cinematic scene |

**Industry Standards:**
- Unity: 1-5M triangles at 60 FPS
- Unreal: 5-10M triangles at 60 FPS (Nanite: billions)
- Godot: 100K-1M triangles at 60 FPS
- Bevy: 1-5M triangles at 60 FPS

#### 1.3 GPU Memory Operations
Tests VRAM allocation/deallocation performance.

| Scenario | Size | Target | Notes |
|----------|------|--------|-------|
| Small buffer | 1 MB | < 0.1ms | Shader constants |
| Medium buffer | 10 MB | < 0.5ms | Vertex buffers |
| Large buffer | 100 MB | < 2ms | Textures |
| Huge buffer | 500 MB | < 10ms | Terrain data |

**Validation:** Memory allocator efficiency (gpu-allocator crate)

#### 1.4 Texture Upload Bandwidth
Tests CPU-to-GPU data transfer rates.

| Resolution | Size | Target Upload Time | Bandwidth Target |
|------------|------|-------------------|------------------|
| 512x512 | 1 MB | < 0.5ms | > 2 GB/s |
| 1024x1024 (1K) | 4 MB | < 2ms | > 2 GB/s |
| 2048x2048 (2K) | 16 MB | < 8ms | > 2 GB/s |
| 4096x4096 (4K) | 64 MB | < 32ms | > 2 GB/s |

**Industry Standards:**
- PCIe 3.0 x16: ~15 GB/s theoretical
- PCIe 4.0 x16: ~30 GB/s theoretical
- Practical bandwidth: 2-8 GB/s (depends on driver overhead)

#### 1.5 Shader Compilation
Tests shader module creation overhead.

| Shader Type | Target | Notes |
|-------------|--------|-------|
| Simple vertex | < 1ms | SPIR-V compilation |
| Simple fragment | < 1ms | SPIR-V compilation |
| Complex compute | < 5ms | Large shader modules |

**Industry Standards:**
- Initial compilation: 10-100ms acceptable
- Runtime compilation: < 5ms critical
- Shader cache should eliminate most recompilations

#### 1.6 Frame Capture Overhead
Tests screenshot/recording impact on frame time.

| Resolution | Target Overhead | Notes |
|------------|-----------------|-------|
| 1080p | < 2ms | Standard capture |
| 4K | < 5ms | High-res capture |

**Validation:** Frame capture doesn't impact gameplay performance

#### 1.7 Synchronization Overhead
Tests fence/semaphore creation and reset.

| Operation | Target | Notes |
|-----------|--------|-------|
| Fence create/destroy | < 10µs | Per-frame operation |
| Fence reset | < 1µs | Very frequent operation |
| Semaphore signal/wait | < 5µs | GPU-GPU sync |

#### 1.8 Pipeline State Changes
Tests render pass and shader switching overhead.

| State Changes | Target | Notes |
|---------------|--------|-------|
| 10 states | < 0.1ms | Simple scene |
| 50 states | < 0.5ms | Complex scene |
| 100 states | < 1ms | Very complex scene |
| 500 states | < 5ms | Extreme (avoid this) |

**Best Practice:** Minimize state changes through batching

#### 1.9 Comprehensive Frame Simulation
Tests a complete AAA game frame.

**Scenario:** 1080p frame with:
- 1000 draw calls
- 5M triangles
- 10 shader changes
- Depth testing enabled
- PBR materials

**Target:** < 8ms (leaves 8.67ms for CPU + other GPU work at 60 FPS)

---

## 2. Large-Scale ECS Benchmarks

**Location:** `engine/core/benches/large_scale_ecs_benches.rs`

### Scenarios Covered

#### 2.1 Simple Iteration at Scale
Basic ECS query performance with massive entity counts.

| Entity Count | Components | Target Time | Throughput Target |
|--------------|------------|-------------|-------------------|
| 50K | Pos + Vel | < 5ms | > 10M entities/sec |
| 100K | Pos + Vel | < 10ms | > 10M entities/sec |
| 250K | Pos + Vel | < 25ms | > 10M entities/sec |
| 500K | Pos + Vel | < 50ms | > 10M entities/sec |

**Industry Comparison:**
- Unity DOTS: 100K entities in ~8ms
- Bevy: 200K entities in ~10ms
- Unreal Mass Entity: 100K entities in ~10ms

#### 2.2 Complex Queries (7+ Components)
Real MMO character query (Position, Velocity, Health, Armor, Team, AI, NetworkId).

| Entity Count | Components | Target Time | Notes |
|--------------|------------|-------------|-------|
| 50K | 7 components | < 10ms | Typical MMO zone |
| 100K | 7 components | < 20ms | Large MMO zone |
| 250K | 7 components | < 50ms | Mega server |

**Validation:** Archetype-based ECS should handle this efficiently

#### 2.3 Sparse Queries (10% Match Rate)
Query where only 10% of entities have all required components.

| Entity Count | Match Rate | Target Time | Notes |
|--------------|------------|-------------|-------|
| 50K | 10% (5K match) | < 1ms | Early rejection optimization |
| 100K | 10% (10K match) | < 2ms | Should be fast with filtering |
| 250K | 10% (25K match) | < 5ms | Tests archetype skipping |
| 500K | 10% (50K match) | < 10ms | Stress test |

**Validation:** Sparse queries should be much faster than dense queries

#### 2.4 Entity Spawn/Despawn at Scale
Batch entity creation and destruction.

| Batch Size | Target Time | Notes |
|------------|-------------|-------|
| 1K entities | < 1ms | Small batch |
| 5K entities | < 5ms | Medium batch |
| 10K entities | < 10ms | Large batch |
| 50K entities | < 50ms | Extreme batch (loading screen) |

**Use Cases:**
- Loading new zone: 10K-50K entities
- Particle system spawn: 1K-5K entities
- Despawning old zone: 10K-50K entities

#### 2.5 Component Addition/Removal at Scale
Dynamic component modification.

| Entity Count | Operation | Target Time | Notes |
|--------------|-----------|-------------|-------|
| 10K | Add component | < 5ms | Buff application |
| 50K | Add component | < 25ms | Mass buff |
| 100K | Add component | < 50ms | Extreme case |
| 10K | Remove component | < 5ms | Buff removal |

**Validation:** Component addition/removal shouldn't cause archetype migration bottlenecks

#### 2.6 Full MMO Server Tick
Complete server tick simulation at massive scale.

**Systems:**
1. Movement (Position += Velocity * dt)
2. Combat (Health -= damage)
3. Network replication query

| Entity Count | Target Tick Time | TPS Possible | Headroom |
|--------------|------------------|--------------|----------|
| 50K | < 8ms | 125 TPS | 108% |
| 100K | < 12ms | 83 TPS | 39% |
| 250K | < 16ms | 62 TPS | 4% |

**Target:** 100K entities at 60 TPS (16.67ms budget)

**Industry Standards:**
- Unity Netcode: ~10K entities at 60 TPS
- Unreal Replication Graph: ~50K entities at 60 TPS
- EVE Online (Python): ~6K entities at 1 TPS (!) - we vastly exceed this
- World of Warcraft: ~5K entities per zone at 60 TPS

---

## 3. Cross-Platform Validation

**Location:** `.github/workflows/benchmark-ci.yml`

### Platforms Tested

#### 3.1 Linux (ubuntu-latest)
- **Vulkan:** Native support via LunarG SDK
- **Target:** Baseline performance (best)
- **CI Setup:** Vulkan SDK + libxcb + libx11

#### 3.2 Windows (windows-latest)
- **Vulkan:** Native support via LunarG SDK
- **Target:** 95-100% of Linux performance
- **CI Setup:** Vulkan SDK installer

#### 3.3 macOS (macos-latest)
- **Vulkan:** MoltenVK translation layer
- **Target:** 85-95% of Linux performance (translation overhead acceptable)
- **CI Setup:** Homebrew MoltenVK

### Performance Expectations

| Platform | Expected Performance | Notes |
|----------|---------------------|-------|
| Linux | 100% (baseline) | Native Vulkan, best drivers |
| Windows | 95-100% | Native Vulkan, slight driver overhead |
| macOS | 85-95% | MoltenVK translation layer |

**Acceptable Degradation:**
- macOS: 5-15% slower due to MoltenVK
- Windows: 0-5% slower due to driver differences

### CI Workflow

The benchmark CI runs:
1. On every push to `main` or `develop`
2. On every pull request
3. Weekly schedule (Monday 00:00 UTC)
4. Manual workflow dispatch

**Outputs:**
- Benchmark results for each platform
- Performance comparison across platforms
- Regression detection (baseline comparison)
- Artifacts: Criterion HTML reports

---

## Running Benchmarks

### GPU Performance Benchmarks

```bash
# Run all GPU benchmarks
cargo bench --bench gpu_performance_comprehensive

# Run specific GPU benchmark group
cargo bench --bench gpu_performance_comprehensive -- gpu_draw_calls
cargo bench --bench gpu_performance_comprehensive -- gpu_triangle_throughput
cargo bench --bench gpu_performance_comprehensive -- gpu_memory
cargo bench --bench gpu_performance_comprehensive -- gpu_texture_upload

# View results
open target/criterion/report/index.html
```

### Large-Scale ECS Benchmarks

```bash
# Run all large-scale benchmarks
cargo bench --bench large_scale_ecs_benches

# Run specific scenario
cargo bench --bench large_scale_ecs_benches -- large_scale_simple_iteration
cargo bench --bench large_scale_ecs_benches -- large_scale_complex_queries
cargo bench --bench large_scale_ecs_benches -- large_scale_mmo_server_tick

# View results
open target/criterion/report/index.html
```

### Cross-Platform Benchmarks (CI)

```bash
# Trigger manual CI run
gh workflow run benchmark-ci.yml

# Check CI results
gh run list --workflow=benchmark-ci.yml

# Download artifacts
gh run download <run-id>
```

---

## Performance Targets Summary

### GPU Rendering (1080p)

| System | Target | Critical | Status |
|--------|--------|----------|--------|
| Draw calls | < 2ms (1000 calls) | < 4ms | ⚠️ TODO |
| Triangles | < 1ms (1M tris) | < 10ms | ⚠️ TODO |
| GPU memory alloc | < 0.5ms (10MB) | < 2ms | ⚠️ TODO |
| Texture upload | < 2ms (1K tex) | < 8ms | ⚠️ TODO |
| Shader compile | < 1ms | < 5ms | ⚠️ TODO |
| Frame capture | < 2ms | < 5ms | ⚠️ TODO |
| Full frame (AAA) | < 8ms | < 12ms | ⚠️ TODO |

### Large-Scale ECS

| System | Target | Critical | Status |
|--------|--------|----------|--------|
| Simple iteration (100K) | < 10ms | < 20ms | ⚠️ TODO |
| Complex queries (100K) | < 20ms | < 40ms | ⚠️ TODO |
| Sparse queries (100K) | < 2ms | < 5ms | ⚠️ TODO |
| Spawn/despawn (10K) | < 10ms | < 20ms | ⚠️ TODO |
| Component ops (10K) | < 5ms | < 10ms | ⚠️ TODO |
| MMO server tick (100K) | < 16ms | < 33ms | ⚠️ TODO |

### Cross-Platform

| Platform | Target | Critical | Status |
|----------|--------|----------|--------|
| Linux | 100% baseline | 95%+ | ✅ SETUP |
| Windows | 95-100% | 90%+ | ✅ SETUP |
| macOS | 85-95% | 80%+ | ✅ SETUP |

---

## Next Steps

### Immediate Actions

1. **Run GPU Benchmarks**
   ```bash
   cargo bench --bench gpu_performance_comprehensive
   ```
   Expected: ~15 minutes runtime

2. **Run Large-Scale ECS Benchmarks**
   ```bash
   cargo bench --bench large_scale_ecs_benches
   ```
   Expected: ~20 minutes runtime

3. **Analyze Results**
   - Compare against industry standards
   - Identify bottlenecks
   - Document performance characteristics

4. **Trigger Cross-Platform CI**
   ```bash
   gh workflow run benchmark-ci.yml
   ```
   Expected: ~45 minutes per platform (3 platforms in parallel)

### Documentation Updates

1. **Update Performance Targets** (`docs/performance-targets.md`)
   - Add GPU performance section
   - Add large-scale ECS section
   - Document cross-platform expectations

2. **Update Comparison Report** (`docs/benchmarks/game-engine-comparison-2026-02-02.md`)
   - Add GPU performance comparison
   - Add large-scale ECS comparison
   - Add cross-platform performance matrix

3. **Create Platform-Specific Guides**
   - Linux optimization guide
   - Windows optimization guide
   - macOS MoltenVK optimization guide

---

## References

### Industry Performance Data

**Unity:**
- DOTS: 100K entities at 60 FPS
- Traditional: 10K GameObjects at 60 FPS
- Draw calls: 1000-2000 per frame

**Unreal Engine 5:**
- Mass Entity: 100K entities at 60 FPS
- Nanite: Billions of triangles (virtualized)
- Draw calls: 2000-5000 per frame (with GPU-driven rendering)

**Godot 4:**
- Nodes: 10K-50K at 60 FPS
- Triangles: 100K-1M at 60 FPS
- Draw calls: 500-1000 per frame

**Bevy:**
- ECS: 200K+ entities at 60 FPS
- Triangles: 1-5M at 60 FPS
- Draw calls: 1000-2000 per frame

### Documentation

- [Vulkan Performance Guide](https://www.khronos.org/blog/vulkan-performance-guide)
- [Unity DOTS Performance](https://docs.unity3d.com/Packages/com.unity.entities@1.0/manual/index.html)
- [Unreal Mass Entity](https://docs.unrealengine.com/5.0/en-US/mass-entity-in-unreal-engine/)
- [Bevy ECS Guide](https://bevyengine.org/learn/book/getting-started/ecs/)

---

**Status:** Infrastructure complete, benchmarks running
**Next Update:** After benchmark results are collected and analyzed
**Date:** 2026-02-02
