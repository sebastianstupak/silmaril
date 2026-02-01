# Performance Targets

> **Industry-standard performance goals for all systems**
>
> ⚠️ **Must meet targets before merging performance-critical code**

---

## 🎯 **Overall Goals**

Match or exceed industry standards for AAA games:
- **Client:** 60 FPS minimum (1080p, medium settings)
- **Server:** 60 TPS (ticks per second) with 1000 concurrent players
- **Latency:** < 100ms total (including network RTT)
- **Memory:** Reasonable limits for target hardware

---

## 📊 **Client Performance Targets**

### **Frame Time Budget** (60 FPS = 16.67ms per frame)

| System | Budget | Critical | Notes |
|--------|--------|----------|-------|
| **ECS Update** | < 2ms | < 4ms | All game logic systems |
| **Physics** | < 4ms | < 8ms | Collision detection + simulation |
| **Rendering** | < 8ms | < 12ms | Vulkan draw calls + GPU wait |
| **Audio** | < 0.5ms | < 1ms | 3D audio processing |
| **Network** | < 1ms | < 2ms | Receive + apply updates |
| **Other** | < 1.17ms | < 2ms | Input, UI, misc |
| **TOTAL** | **< 16.67ms** | **< 33ms** | 60 FPS / 30 FPS |

---

### **Rendering**

| Metric | Target | Critical |
|--------|--------|----------|
| **Draw calls** | < 2000 | < 5000 |
| **Triangles** | < 5M | < 10M |
| **GPU memory** | < 2GB | < 4GB |
| **Texture streaming** | < 100ms | < 500ms |
| **Shader compilation** | < 50ms | < 200ms |
| **Frame capture overhead** | < 2ms | < 5ms |

**Test Scene:** 1920x1080, 10k entities, PBR materials, 3 lights, shadows

---

### **ECS**

| Operation | Target | Critical |
|-----------|--------|----------|
| **Spawn entity** | < 0.1μs | < 1μs |
| **Add component** | < 0.2μs | < 1μs |
| **Query (1 component, 10k entities)** | < 0.5ms | < 1ms |
| **Query (3 components, 10k entities)** | < 1ms | < 2ms |
| **Serialize world (1000 entities)** | < 5ms (bincode) | < 10ms |
| **Deserialize world** | < 10ms | < 20ms |

---

### **Memory**

| Resource | Target | Critical | Notes |
|----------|--------|----------|-------|
| **Client baseline** | < 500MB | < 1GB | Empty world |
| **Client (gameplay)** | < 2GB | < 4GB | Typical game session |
| **Asset cache** | < 1GB | < 2GB | Textures, meshes, audio |
| **ECS (100k entities)** | < 200MB | < 500MB | Transform + basic components |

---

## 🖥️ **Server Performance Targets**

### **Tick Budget** (60 TPS = 16.67ms per tick)

| System | Budget | Critical | Notes |
|--------|--------|----------|-------|
| **Receive inputs** | < 1ms | < 2ms | From all clients |
| **Game logic** | < 4ms | < 8ms | ECS systems |
| **Physics** | < 5ms | < 10ms | Server-authoritative |
| **Interest management** | < 2ms | < 4ms | Per-client visibility |
| **State serialization** | < 3ms | < 6ms | Delta generation |
| **Send updates** | < 1.67ms | < 4ms | To all clients |
| **TOTAL** | **< 16.67ms** | **< 33ms** | 60 TPS / 30 TPS |

---

### **Scalability**

| Players | Tick Time | Memory | CPU | Notes |
|---------|-----------|--------|-----|-------|
| **10** | < 2ms | < 100MB | < 10% (1 core) | Dev testing |
| **100** | < 8ms | < 500MB | < 40% | Small server |
| **1000** | < 16ms | < 8GB | < 80% | Production |
| **10000** | N/A | N/A | Multi-server | Sharding required |

**Test Configuration:** 4-core, 16GB RAM server

---

### **Network**

| Metric | Target | Critical | Notes |
|--------|--------|----------|-------|
| **Bandwidth per client (full state)** | < 100 KB/s | < 200 KB/s | Uncompressed |
| **Bandwidth per client (delta)** | < 20 KB/s | < 50 KB/s | 80% reduction |
| **State diff computation** | < 2ms | < 5ms | Per client |
| **Interest culling overhead** | < 2% | < 5% | VALORANT-level |
| **Packet loss tolerance** | Up to 5% | Up to 10% | UDP only |

---

## 🌐 **Cross-Platform Targets**

Performance must meet targets on **all** Tier 1 platforms:

| Platform | Client FPS | Notes |
|----------|------------|-------|
| **Windows (x64)** | 60+ | Primary development platform |
| **Linux (x64)** | 60+ | Mesa/NVIDIA drivers |
| **macOS (x64)** | 55+ | MoltenVK overhead acceptable |
| **macOS (ARM64)** | 60+ | M1+ chips very fast |

**Acceptable degradation on macOS x64:** 5-10% due to MoltenVK translation layer.

---

## 🧪 **Profiling Tools**

### **Tracy Profiler**

Enable Tracy in development builds:

```toml
[features]
profiling = ["tracing-tracy"]
```

```bash
cargo build --features profiling
./target/debug/client
# Open Tracy, connect to localhost
```

**Use Tracy zones:**
```rust
use tracing::instrument;

#[instrument]
fn expensive_system(world: &World) {
    // Automatically shows in Tracy
}
```

---

### **Criterion Benchmarks**

Run benchmarks regularly:

```bash
cargo bench
```

**Example output:**
```
spawn 10k entities      time:   [95.234 µs 96.891 µs 98.702 µs]
query 10k transforms    time:   [423.14 µs 431.92 µs 441.88 µs]
```

**Regression detection:** CI fails if benchmarks regress > 10%.

---

### **Flamegraph**

Generate CPU flamegraph:

```bash
cargo install flamegraph
cargo flamegraph --bin client
```

---

## 📈 **Performance Testing in CI**

```yaml
# .github/workflows/performance.yml
name: Performance Tests

on: [push, pull_request]

jobs:
  benchmarks:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo bench --bench ecs_benchmark
      - run: cargo bench --bench render_benchmark

      # Compare with baseline
      - uses: benchmark-action/github-action-benchmark@v1
        with:
          tool: 'cargo'
          output-file-path: target/criterion/output.json
          fail-on-alert: true
          alert-threshold: '110%'  # Fail if > 10% slower
```

---

## 🎯 **Optimization Priorities**

When optimizing, focus on:

1. **Hot paths** (Tracy shows where time is spent)
2. **Memory allocations** (batch operations, object pools)
3. **Cache locality** (SoA data layout in ECS)
4. **Parallelization** (rayon for data-parallel systems)
5. **GPU utilization** (indirect rendering, compute shaders)

---

## ✅ **Performance Checklist**

Before merging performance-critical code:

- [ ] Benchmarks added (criterion)
- [ ] Profiled with Tracy
- [ ] Meets target performance (see tables above)
- [ ] No regressions (< 10% slower than baseline)
- [ ] Tested on all platforms
- [ ] Flamegraph reviewed (no obvious bottlenecks)
- [ ] Memory usage acceptable

---

## 📚 **Related Documentation**

- [docs/architecture.md](docs/architecture.md) - System design
- [docs/ecs.md](docs/ecs.md) - ECS performance
- [docs/networking.md](docs/networking.md) - Network optimization
- [docs/rendering.md](docs/rendering.md) - GPU performance

---

**Last Updated:** 2026-01-31
