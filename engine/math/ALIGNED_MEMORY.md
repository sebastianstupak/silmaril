# Cache-Aligned Memory Allocations

## Overview

This document describes the cache-aligned memory allocation infrastructure added to the `engine-math` crate for improved SIMD performance.

## What Was Added

### 1. AlignedVec<T, ALIGN> Type

**File:** `src/aligned.rs`

A custom vector type that guarantees alignment to `ALIGN` bytes. This is critical for SIMD performance.

```rust
use engine_math::aligned::AlignedVec;
use engine_math::simd::Vec3x4;
use engine_math::Vec3;

// Create a 64-byte aligned vector (cache line size)
let mut positions: AlignedVec<Vec3x4, 64> = AlignedVec::new();
positions.push(Vec3x4::splat(Vec3::new(1.0, 2.0, 3.0)));

// Guaranteed cache-line alignment
assert_eq!(positions.as_ptr() as usize % 64, 0);
```

**Key Features:**
- Generic over element type `T` and alignment `ALIGN` (must be power of 2)
- Uses custom allocator with `std::alloc::alloc` for aligned memory
- Implements `Deref` and `DerefMut` to `[T]` for seamless slice operations
- Maintains alignment through reallocations
- Safe Drop implementation with proper cleanup

**API:**
- `new()` - Create empty vector
- `with_capacity(capacity)` - Pre-allocate aligned memory
- `push(value)` - Add element
- `pop()` - Remove last element
- `resize(len, value)` - Resize with fill value
- `clear()` - Remove all elements (keeps capacity)
- Standard slice operations via `Deref`

### 2. Updated Vec3x4 with Alignment

**File:** `src/simd/vec3x4.rs`

Updated `Vec3x4` to include:

1. **Struct alignment annotation:**
   ```rust
   #[repr(C, align(16))]  // 16-byte alignment for 128-bit SIMD
   pub struct Vec3x4 {
       pub x: f32x4,
       pub y: f32x4,
       pub z: f32x4,
   }
   ```

2. **Aligned load/store methods:**
   ```rust
   unsafe fn load_aligned(ptr: *const f32) -> Self
   unsafe fn store_aligned(self, ptr: *mut f32)
   ```

**Usage:**
```rust
let mut buffer: AlignedVec<f32, 16> = AlignedVec::with_capacity(12);
buffer.resize(12, 0.0);

let vec = Vec3x4::splat(Vec3::new(1.0, 2.0, 3.0));

unsafe {
    vec.store_aligned(buffer.as_mut_ptr());
    let loaded = Vec3x4::load_aligned(buffer.as_ptr());
}
```

### 3. Updated Vec3x8 with Alignment

**File:** `src/simd/vec3x8.rs`

Similar updates for AVX2 (256-bit) SIMD:

1. **32-byte alignment:**
   ```rust
   #[repr(C, align(32))]  // 32-byte alignment for 256-bit SIMD
   pub struct Vec3x8 { ... }
   ```

2. **Aligned load/store methods** for 32-byte boundaries

### 4. Comprehensive Benchmarks

**File:** `benches/aligned_benches.rs`

Benchmarks to measure performance improvements:

1. **`aligned_vs_unaligned_vec3x4`**
   - Compares standard `Vec<Vec3x4>` vs `AlignedVec<Vec3x4, 64>`
   - Tests physics integration workload
   - Measures at 100, 1000, and 10000 element scales

2. **`aligned_loads_stores`**
   - Measures aligned load/store performance
   - Compares to AoS→SoA conversion overhead

3. **`cache_line_false_sharing`**
   - Demonstrates false sharing prevention
   - Tightly packed vs cache-line separated access patterns

4. **`bulk_physics_integration`**
   - Real-world physics simulation scenario
   - Tests at 1K, 10K, and 100K entity scales
   - Includes position, velocity, and acceleration updates

**Run benchmarks:**
```bash
cargo bench --features simd --bench aligned_benches
```

### 5. Integration Tests

**File:** `tests/aligned_integration_test.rs`

Comprehensive test suite covering:
- Basic AlignedVec operations with Vec3x4
- Alignment verification after multiple resizes
- Aligned load/store correctness
- Bulk SIMD operations (10,000 element physics simulation)
- Cache line separation validation

**Run tests:**
```bash
cargo test --features simd --test aligned_integration_test
```

### 6. Example Demonstration

**File:** `examples/aligned_demo.rs`

Interactive example showing:
- Creating cache-aligned vectors
- Verifying memory alignment
- SIMD physics integration
- Aligned load/store operations
- Comparison with standard Vec

**Run example:**
```bash
cargo run --example aligned_demo --features simd
```

## Performance Benefits

### 1. Prevents Cache Line Splits

When data straddles two cache lines, the CPU must fetch both lines, doubling memory traffic.

**Problem:**
```
Cache Line 0: [.......xyz]
Cache Line 1: [yzabc......]  ← Split access!
```

**Solution with 64-byte alignment:**
```
Cache Line 0: [xyzabc......]  ← Single cache line access
Cache Line 1: [............]
```

### 2. Enables Aligned SIMD Instructions

Modern CPUs have separate instructions for aligned vs unaligned loads/stores:

- **Aligned:** `movaps` (SSE), `vmovapd` (AVX) - **Faster**
- **Unaligned:** `movups` (SSE), `vmovupd` (AVX) - Slower

On some older CPUs, unaligned loads can cause significant penalties or even faults.

### 3. Prevents False Sharing

In multi-threaded scenarios, when two threads access different variables on the same cache line, cache coherency traffic can cause severe slowdowns.

**Problem:**
```
Thread 1 writes to positions[0]  ─┐
Thread 2 writes to positions[1]  ─┼─ Same cache line! False sharing!
```

**Solution:**
With 64-byte alignment, each critical element gets its own cache line (or you can ensure spacing).

### 4. Better Prefetching

CPU prefetchers work best with predictable, aligned access patterns. Cache-aligned data enables more effective hardware prefetching.

## Expected Performance Gains

Based on typical scenarios:

| Scenario | Expected Speedup | Reason |
|----------|-----------------|--------|
| Sequential SIMD ops | 5-15% | Aligned loads, better cache utilization |
| Strided access patterns | 10-30% | Reduced cache line splits |
| Multi-threaded SIMD | 20-50% | False sharing prevention |
| Large dataset iteration | 10-20% | Better prefetching, cache efficiency |

**Note:** Actual gains depend on:
- CPU architecture (newer CPUs handle unaligned better)
- Memory access patterns
- Dataset size (cache fit)
- Thread count and contention

## Use Cases

### When to Use AlignedVec

✅ **Use when:**
- Storing large arrays of SIMD types (`Vec3x4`, `Vec3x8`)
- Bulk physics integration (positions, velocities)
- Particle systems (thousands of particles)
- Spatial data structures (BVH nodes, octree data)
- Multi-threaded workloads with independent data access

❌ **Don't use when:**
- Small collections (< 100 elements) - overhead not worth it
- Single-use temporary data
- Frequently reallocated/resized vectors
- Already using SoA layout in a different way

### Recommended Alignments

- **16 bytes:** Minimum for SSE/NEON SIMD types
- **32 bytes:** For AVX/AVX2 types
- **64 bytes:** Cache line size on most modern CPUs (x86, ARM)
  - Best for preventing false sharing
  - Recommended for bulk storage

## Integration with Existing Code

### Before (Standard Vec):
```rust
let mut positions: Vec<Vec3x4> = Vec::new();
for i in 0..1000 {
    positions.push(Vec3x4::splat(Vec3::new(i as f32, 0.0, 0.0)));
}

// SIMD operations
for i in 0..positions.len() {
    positions[i] = positions[i] * 2.0;
}
```

### After (AlignedVec):
```rust
let mut positions: AlignedVec<Vec3x4, 64> = AlignedVec::new();
for i in 0..1000 {
    positions.push(Vec3x4::splat(Vec3::new(i as f32, 0.0, 0.0)));
}

// Same SIMD operations - but faster!
for i in 0..positions.len() {
    positions[i] = positions[i] * 2.0;
}
```

**Changes required:** Just the type annotation - the API is compatible!

## Technical Details

### Memory Layout

`AlignedVec<Vec3x4, 64>` layout:
```
Address 0x0000: [Vec3x4] [Vec3x4] [Vec3x4] ...
                ↑
                64-byte aligned start
```

### Size and Alignment

```rust
std::mem::size_of::<Vec3x4>() = 48 bytes  // 3 * f32x4 = 3 * 16 = 48
std::mem::align_of::<Vec3x4>() = 16 bytes // #[repr(C, align(16))]
```

With `AlignedVec<Vec3x4, 64>`:
- Buffer starts at 64-byte boundary
- Elements are tightly packed (48 bytes each)
- No per-element padding (unless you want it)

### Safety Considerations

The aligned load/store methods are `unsafe` because:
1. They require valid pointers
2. They require proper alignment (debug asserts check this)
3. They assume initialized memory

**Safe usage pattern:**
```rust
let mut buffer: AlignedVec<f32, 16> = AlignedVec::with_capacity(12);
buffer.resize(12, 0.0);  // Initialize!

unsafe {
    vec.store_aligned(buffer.as_mut_ptr());  // Safe: buffer is aligned & initialized
}
```

## Future Improvements

1. **SIMD-width aligned padding:**
   - Option to pad elements to cache line boundaries
   - Trade memory for guaranteed no false sharing

2. **Custom iterators:**
   - Prefetching iterators
   - Chunked SIMD iterators (process 4/8 at a time)

3. **NUMA awareness:**
   - Allocate on specific NUMA nodes
   - Thread-local aligned pools

4. **Compile-time alignment verification:**
   - Static assertions for alignment requirements
   - Type-level alignment constraints

## References

- [Intel® 64 and IA-32 Architectures Optimization Reference Manual](https://www.intel.com/content/www/us/en/developer/articles/technical/intel-sdm.html)
- [What Every Programmer Should Know About Memory](https://people.freebsd.org/~lstewart/articles/cpumemory.pdf)
- [False Sharing and Cache Line Size](https://mechanical-sympathy.blogspot.com/2011/07/false-sharing.html)

---

**Status:** ✅ Implemented and Tested
**Version:** 0.1.0
**Date:** 2026-02-01
