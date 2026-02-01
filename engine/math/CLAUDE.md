# Engine Math

## Purpose
Pure mathematics library for the game engine. Provides:
- **Vector types**: Vec2, Vec3, Vec4 with standard operations
- **Transforms**: Transform, Quaternion for 3D rotations
- **SIMD**: High-performance vectorized math operations (Vec3x4, Vec3x8)

This is a **domain-agnostic** module - it has no knowledge of physics, rendering, or ECS.
It exists solely to provide mathematical primitives and optimized operations.

## MUST READ Documentation
Before working on this crate, read these documents in order:

1. **[architecture.md](../../docs/architecture.md)** - Overall engine architecture
2. **[performance-guidelines.md](../../docs/performance-guidelines.md)** - SIMD best practices
3. **[CPU_FEATURES.md](CPU_FEATURES.md)** - CPU features and SIMD optimizations
4. **[PERFORMANCE.md](PERFORMANCE.md)** - Benchmark results and optimization strategies
5. This file - for module-specific details

## Module Structure

```
src/
├── lib.rs           - Module exports and re-exports
├── vec3.rs          - Vector3 type (scalar operations)
├── vec2.rs          - Vector2 type
├── vec4.rs          - Vector4 type
├── transform.rs     - Transform (position, rotation, scale)
├── quaternion.rs    - Quaternion rotations
└── simd/            - SIMD-optimized operations
    ├── mod.rs
    ├── vec3x4.rs    - Process 4 Vec3s at once (128-bit SIMD)
    ├── vec3x8.rs    - Process 8 Vec3s at once (256-bit SIMD/AVX2)
    └── util.rs      - SIMD conversion utilities (AoS ↔ SoA)
```

## Design Principles

### 1. Separation of Scalar and SIMD
- **Scalar types** (`Vec3`, `Transform`): Single entity operations
- **SIMD types** (`Vec3x4`, `Vec3x8`): Batch operations on 4-8 entities

### 2. Structure-of-Arrays (SoA) for SIMD
```rust
// Array-of-Structures (AoS) - Cache-unfriendly for SIMD
struct Vec3 { x: f32, y: f32, z: f32 }
let positions: Vec<Vec3> = vec![...];  // [xyz][xyz][xyz]...

// Structure-of-Arrays (SoA) - SIMD-friendly
struct Vec3x4 {
    x: f32x4,  // 4 x-components packed together
    y: f32x4,  // 4 y-components packed together
    z: f32x4,  // 4 z-components packed together
}
```

### 3. Conversion Utilities
Provide easy conversion between AoS (ECS storage) and SoA (SIMD processing):
```rust
// ECS stores AoS (cache-friendly iteration)
let positions: &[Vec3] = ...;

// Convert to SoA for SIMD processing
let pos_soa = Vec3x4::from_slice_aos(&positions[0..4]);

// Process with SIMD
let result = pos_soa + velocity_soa * dt;

// Convert back to AoS for storage
result.write_to_slice_aos(&mut positions[0..4]);
```

## Usage Examples

### Scalar Operations (Single Entity)
```rust
use engine_math::{Vec3, Transform};

let pos = Vec3::new(1.0, 2.0, 3.0);
let vel = Vec3::new(0.1, 0.0, -0.1);
let new_pos = pos + vel * dt;
```

### SIMD Operations (4-8 Entities)
```rust
use engine_math::simd::{Vec3x4, vec3_aos_to_soa};

// Get 4 positions from ECS (AoS)
let positions: &[Vec3; 4] = ...;
let velocities: &[Vec3; 4] = ...;

// Convert to SoA for SIMD
let pos = vec3_aos_to_soa(positions);
let vel = vec3_aos_to_soa(velocities);

// SIMD math: Process all 4 at once
let new_pos = pos + vel * dt;  // Single instruction, 4 operations

// Convert back to AoS
let result: [Vec3; 4] = new_pos.into_aos();
```

## Performance Targets

- **Scalar Vec3 add**: <1 ns
- **SIMD Vec3x4 add**: <2 ns (4 operations, 2x throughput)
- **SIMD Vec3x8 add**: <3 ns (8 operations, 2.67x throughput)
- **AoS→SoA conversion**: <5 ns (minimize overhead)

## Dependencies

**Runtime:**
- `wide` - Portable SIMD (works on x86, ARM, WASM)
- `serde` (optional) - Serialization support

**Why `wide` over `std::simd`?**
- More stable (std::simd is nightly-only as of 2026)
- Better cross-platform support
- Easier to use with less unsafe code

## Testing Strategy

### Unit Tests
- Test scalar operations for correctness
- Test SIMD operations match scalar results
- Test edge cases (zero vectors, NaN, infinity)

### Benchmarks
- Compare scalar vs SIMD performance
- Measure conversion overhead (AoS↔SoA)
- Profile cache behavior

## Integration Points

### Used By
- `engine-physics` - Transform/velocity integration
- `engine-renderer` - Matrix transformations
- `engine-core` - ECS component storage (scalar types)

### Does NOT Depend On
- Any other engine modules (pure math, zero dependencies)

## Future Optimizations

1. **AVX-512** support (16-wide operations)
2. **Custom allocators** for SIMD-aligned memory
3. **Compile-time SIMD selection** based on target CPU
4. **WASM SIMD** for web builds

## Key Files

- `vec3.rs` - Most commonly used type
- `simd/vec3x4.rs` - Core SIMD implementation (4-wide)
- `simd/util.rs` - Conversion functions (critical for performance)

---

**Status:** 🚧 In Development (Phase 1.4)
**Performance:** Target 2-4x speedup over scalar operations
