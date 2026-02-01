# Build Tiers - Quick Reference

> **Fast reference for building and using platform-specific optimized binaries**

---

## Quick Commands

### Build All Tiers

**Linux/macOS:**
```bash
./scripts/build_all_tiers.sh --release --both
```

**Windows:**
```powershell
.\scripts\build_all_tiers.ps1 -Release -Both
```

### Detect Your CPU Tier

```bash
cargo run --example cpu_tier_detection --package engine-build-utils
```

### Benchmark Performance

```bash
./scripts/benchmark_tiers.sh --verbose
```

---

## Tier Comparison Table

| Tier | CPU Features | Compatibility | Speed vs Baseline | Example CPUs |
|------|-------------|---------------|-------------------|--------------|
| **Baseline** | SSE2 | 100% | 1.0x | All x86-64 |
| **Modern** | AVX2 + FMA + SSE4.2 | ~95% | 1.15-1.30x | Intel Haswell (2013+), AMD Ryzen |
| **High-end** | AVX512 + AVX2 | ~70% | 1.20-1.50x | Intel Skylake-X (2017+), AMD Zen 4 (2022+) |

---

## Binary Locations

After building:
```
target/baseline/release/client   - Universal compatibility
target/modern/release/client     - Recommended for distribution
target/highend/release/client    - Maximum performance
```

---

## Runtime Selection (Launcher)

```rust
use engine_build_utils::cpu_features::detect_tier;

let tier = detect_tier();
let binary = match tier {
    CpuTier::HighEnd => "bin/highend/client.exe",
    CpuTier::Modern => "bin/modern/client.exe",
    CpuTier::Baseline => "bin/baseline/client.exe",
};

launch_binary(binary);
```

---

## When to Use Each Tier

### Baseline
- ✅ Fallback for old CPUs
- ✅ Initial compatibility testing
- ✅ Very old hardware (pre-2013)

### Modern (Recommended)
- ✅ Steam/Epic releases
- ✅ Default distribution
- ✅ Best compatibility/performance balance

### High-end
- ✅ Enthusiast builds
- ✅ Server deployments (known hardware)
- ✅ Maximum performance requirement

---

## Build Options

**Client only:**
```bash
./scripts/build_all_tiers.sh --release --client
```

**Server only:**
```bash
./scripts/build_all_tiers.sh --release --server
```

**Debug builds:**
```bash
./scripts/build_all_tiers.sh --both  # No --release flag
```

---

## Performance Expectations

### Scalar Math Operations
- Modern: 15-30% faster than baseline
- High-end: 20-35% faster than baseline

### Batch Operations (SIMD)
- Modern: 2-3x faster than baseline
- High-end: 3-5x faster than baseline

### Game Workload (Overall)
- Modern: 15-25% faster than baseline
- High-end: 20-35% faster than baseline

---

## Troubleshooting

### "Illegal instruction" crash
→ CPU doesn't support the tier. Use a lower tier.

### No performance improvement
→ Ensure you built with `--release` flag.

### Build fails
→ Update Rust: `rustup update stable`

---

## See Also

- **[docs/build-tiers.md](build-tiers.md)** - Complete guide
- **[scripts/README.md](../scripts/README.md)** - Script documentation
- **[docs/pgo.md](pgo.md)** - Profile-Guided Optimization (can combine with tiers)

---

**Status:** ✅ Production Ready (Task #59 Complete)
**Last Updated:** 2025-02-01
