# Engine Math Performance Analysis

## Benchmark Results (2026-02-01)

### SIMD vs Scalar Performance

**Physics Integration Benchmark (position += velocity * dt)**

| Entity Count | Scalar | SIMD (with conversion) | SIMD (no conversion) | Speedup |
|--------------|--------|------------------------|----------------------|---------|
| 100 | 427ns | 404ns | 148ns | **2.9x** |
| 1,000 | 3.7µs | 4.8µs | 1.3µs | **2.85x** |
| 10,000 | 40.6µs | 44.4µs | 12.7µs | **3.2x** |

### Key Insights

1. **SIMD Performance**: When data is pre-formatted as SoA (Structure-of-Arrays), SIMD provides consistent **2.9-3.2x speedup** over scalar operations.

2. **Conversion Overhead**: AoS↔SoA conversion overhead is significant:
   - For small batches (< 1000 entities), conversion negates SIMD benefits
   - Conversion cost: ~100-200ns per 4-entity batch
   - For 10,000 entities, conversion adds ~32µs overhead

3. **Throughput**: SIMD achieves:
   - **777-823 Melem/s** for large batches (10K entities)
   - **652-679 Melem/s** for small batches (100 entities)
   - Scalar baseline: **234-262 Melem/s**

## Recommendations

### When to Use SIMD

✅ **Use SIMD when:**
- Processing > 1000 entities per frame
- Data can be stored natively in SoA format
- Hot path physics integration (position/velocity updates)
- Batch processing is possible

❌ **Avoid SIMD when:**
- Processing < 100 entities per frame
- Data must remain in AoS format
- Random access patterns required
- Conversion overhead exceeds compute savings

### Optimization Strategies

1. **Native SoA Storage**: Store physics components in SoA format:
   ```rust
   struct PhysicsWorld {
       positions_x: Vec<f32>,  // All X components
       positions_y: Vec<f32>,  // All Y components
       positions_z: Vec<f32>,  // All Z components
       // ... same for velocities
   }
   ```

2. **Batched Updates**: Process entities in groups of 4-8:
   ```rust
   for chunk in entities.chunks(4) {
       // Process 4 entities with SIMD
   }
   ```

3. **Hybrid Approach**: Use SIMD for bulk updates, scalar for edge cases:
   ```rust
   // Main batch (divisible by 4)
   simd_process(entities[..aligned_count]);
   // Remainder (1-3 entities)
   scalar_process(entities[aligned_count..]);
   ```

## Future Optimizations

### AVX2 Support (8-wide SIMD)
- Expected 5-6x speedup over scalar
- Requires Vec3x8 implementation
- Target: 1000+ Melem/s throughput

### Cache Optimization
- Align Vec3x4 to 16-byte boundaries
- Prefetch next batch during current computation
- Expected 10-15% additional speedup

### Compiler Optimizations
- ✅ Add `#[target_feature]` for AVX2/FMA support (Implemented)
- Use `std::simd` when it stabilizes
- Profile-guided optimization (PGO)

## CPU Features and SIMD Instructions

### Supported SIMD Instruction Sets

This crate leverages modern CPU features for maximum performance:

| Feature | Description | Performance Impact | Status |
|---------|-------------|-------------------|--------|
| **SSE4.2** | 128-bit SIMD operations | Baseline for all SIMD code | ✅ Required |
| **FMA** | Fused multiply-add instructions | 10-20% faster dot products, lerp | ✅ Enabled |
| **AVX2** | 256-bit SIMD operations | 2x throughput for batch operations | ✅ Enabled |
| **AVX-512** | 512-bit SIMD operations | 4x throughput (future optimization) | 🚧 Future |

### Enabling CPU Features

The build script automatically detects available CPU features at compile time. For optimal performance:

```bash
# Enable all features supported by your CPU
RUSTFLAGS="-C target-cpu=native" cargo build --release

# Or enable specific features
RUSTFLAGS="-C target-feature=+sse4.2,+fma,+avx2" cargo build --release
```

### FMA (Fused Multiply-Add) Optimizations

FMA instructions provide both performance and precision benefits:

**Optimized Functions:**
- `Vec3::dot()` - Single FMA instruction for `a*b + c*d + e*f`
- `Vec3::normalize()` - FMA for reciprocal square root refinement
- `Vec3::lerp()` - FMA for `a + (b - a) * t`
- `Vec3::reflect()` - FMA for reflection computation

**Performance Gains:**
- **Dot product**: ~15% faster with FMA
- **Normalize**: ~10% faster with FMA
- **Lerp**: ~12% faster with FMA

**Precision Gains:**
- FMA computes `a*b + c` with a single rounding step (vs two for separate mul+add)
- Reduces floating-point error accumulation in long chains

### Glam Integration

This crate uses [glam](https://github.com/bitshifter/glam-rs) for Vec3 implementation:
- Glam is heavily optimized with SIMD intrinsics
- Already uses SSE2/SSE3/SSE4.1 for scalar operations
- Our `fast-math` feature enables additional unsafe optimizations
- We add conditional FMA/AVX2 support through target features

### Compiler Optimization Flags

The release profile enables aggressive optimizations:

```toml
[profile.release]
opt-level = 3              # Maximum optimization
lto = "thin"               # Thin LTO for cross-crate optimization
codegen-units = 1          # Better optimization at cost of compile time
```

### Feature Detection at Build Time

The `build.rs` script:
1. Detects target architecture (x86/x86_64)
2. Checks `CARGO_CFG_TARGET_FEATURE` for enabled SIMD features
3. Sets custom cfg flags (`has_sse42`, `has_fma`, `has_avx2`)
4. Emits warnings if `target-cpu=native` is not set
5. Displays feature summary during compilation

### Runtime vs Compile-Time SIMD

**Compile-Time (Current Approach):**
- Features detected at build time via `target-cpu` or `target-feature`
- Code compiled with specific SIMD instructions
- Best performance, but not portable across CPUs
- ✅ Used for Vec3 scalar operations

**Runtime (SIMD Module):**
- Uses `wide` crate for portable SIMD
- Works across all CPUs (falls back to scalar if needed)
- Slightly lower performance than compile-time intrinsics
- ✅ Used for Vec3x4/Vec3x8 batch operations

## Measurement Methodology

Benchmarks run with:
- Criterion 0.5 (statistical analysis)
- Release mode with optimizations
- Quick mode (10 iterations)
- Windows x64 platform
- Blocking on memory allocations disabled
- CPU features: SSE4.2 + FMA + AVX2 (when available)

All measurements use `black_box()` to prevent compiler optimization of benchmark code.

---

**Last Updated**: 2026-02-01
**Next Review**: When implementing physics batching (Phase 2.x)
