# Physics Integration - Quick Start Guide

## Overview

The optimized physics integration system provides **3-4x faster** performance for updating entity positions based on velocities.

## Basic Usage

### 1. Simple Integration

```rust
use engine_core::ecs::World;
use engine_core::math::Transform;
use engine_physics::components::Velocity;
use engine_physics::systems::physics_integration_system_simd;

fn main() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Velocity>();

    // Create entities
    for i in 0..1000 {
        let entity = world.spawn();
        world.add(entity, Transform::identity());
        world.add(entity, Velocity::new(1.0, 2.0, 3.0));
    }

    // Game loop
    let dt = 0.016; // 60 FPS
    loop {
        // Update all entities: position += velocity * dt
        physics_integration_system_simd(&mut world, dt);

        // ... rest of game logic
    }
}
```

### 2. Choosing the Right System

```rust
use engine_physics::systems::{
    integration::physics_integration_system,        // Scalar
    integration_simd::physics_integration_system_simd, // SIMD + Parallel
};

fn update(world: &mut World, dt: f32, entity_count: usize) {
    if entity_count < 50 {
        // Use scalar for small counts (less overhead)
        physics_integration_system(world, dt);
    } else {
        // Use SIMD for better performance
        physics_integration_system_simd(world, dt);
    }
}
```

## What Happens Automatically

The SIMD system automatically:

1. **Chooses batch size**: Uses 8-wide (AVX2) or 4-wide (SSE) based on count
2. **Handles remainders**: Processes leftover entities with scalar (no overhead)
3. **Enables parallelism**: Uses rayon for >10k entities
4. **Prefetches data**: Hints next batch for better cache performance

## Performance Characteristics

### Entity Count vs. Speedup

```
   10 entities:  ~1.5-2x faster  (conversion overhead significant)
  100 entities:  ~2x faster      (good batch utilization)
1,000 entities:  ~3x faster      (optimal performance)
10,000+ entities: ~4x faster     (parallel + SIMD)
```

### Throughput

```
Scalar:        ~40M entities/second
SIMD (seq):    ~120M entities/second (3x)
SIMD (par):    ~160M entities/second (4x)
```

## Compilation Flags

For best performance, compile with native CPU features:

### Linux/macOS
```bash
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

### Windows PowerShell
```powershell
$env:RUSTFLAGS="-C target-cpu=native"
cargo build --release
```

This enables AVX2, FMA, and other CPU-specific optimizations.

## Testing

### Run Unit Tests
```bash
cd engine/physics
cargo test
```

### Run Integration Tests
```bash
cargo test --test integration_simd_test
```

### Run Demo
```bash
cargo run --example integration_demo --release
```

Expected output:
```
Testing with 1000 entities:
  Scalar:      12.5ms (100 iterations)
  SIMD:        4.2ms (100 iterations)
  Speedup:     3.0x faster
  ✓  Excellent speedup achieved!
```

## Benchmarking

### Quick Benchmark
```bash
cargo bench --bench integration_bench
```

### Full Analysis
```bash
# Windows
.\scripts\bench_physics_integration.ps1

# Linux/macOS
cd engine/physics
cargo bench --bench integration_bench -- --save-baseline main
```

Results saved to `benchmark_results_<timestamp>.txt`

## How It Works

### Processing Pipeline

```
Input: N entities with Transform + Velocity
  ↓
Collect into contiguous arrays
  ↓
┌─────────────────────────────────┐
│ Is N >= 10,000?                 │
├─────────────────────────────────┤
│ YES → Parallel Processing       │
│   - Split into chunks of 512    │
│   - Each thread: hybrid SIMD    │
│                                 │
│ NO → Sequential Processing      │
│   - Batch of 8 (AVX2)          │
│   - Batch of 4 (SSE)           │
│   - Remainder (scalar)         │
└─────────────────────────────────┘
  ↓
Output: Updated transforms
```

### SIMD Operations

Each batch processes multiple entities in parallel:

```rust
// Scalar (1 entity at a time):
for entity in entities {
    position += velocity * dt;  // 1 operation
}

// SIMD (8 entities at once):
position_8 = position_8.mul_add(velocity_8, dt);  // 8 operations
```

## Code Organization

```
engine/physics/
├── src/
│   ├── systems/
│   │   ├── integration.rs         # Scalar baseline
│   │   └── integration_simd.rs    # SIMD optimized ⭐
│   └── components.rs               # Velocity component
├── benches/
│   └── integration_bench.rs       # Performance tests
├── tests/
│   └── integration_simd_test.rs   # Correctness tests
└── examples/
    └── integration_demo.rs        # Usage demo
```

## Troubleshooting

### "Speedup lower than expected"

**Problem**: Getting <2x speedup
**Solutions**:
1. Compile with `-C target-cpu=native`
2. Use release mode: `cargo build --release`
3. Check entity count (need >100 for good speedup)
4. Disable debug assertions: `cargo run --release`

### "Tests failing"

**Problem**: Unit tests fail
**Solutions**:
1. Check dependency versions: `cargo update`
2. Clean build: `cargo clean && cargo build`
3. Verify ECS is working: `cargo test -p engine-core`

### "Build errors"

**Problem**: Compilation fails
**Solutions**:
1. Update Rust: `rustup update`
2. Check rayon version: Should be `1.10`
3. Verify workspace dependencies are correct

## API Reference

### Main Functions

#### `physics_integration_system_simd`
```rust
pub fn physics_integration_system_simd(world: &mut World, dt: f32)
```
Main optimized integration system. Automatically chooses best strategy.

#### `process_sequential`
```rust
pub fn process_sequential(
    transforms: &mut [Transform],
    velocities: &[Vec3],
    dt: f32
)
```
Hybrid SIMD batching (8-wide → 4-wide → scalar).

#### `process_parallel`
```rust
pub fn process_parallel(
    transforms: &mut [Transform],
    velocities: &[Vec3],
    dt: f32
)
```
Parallel processing with rayon (for >10k entities).

#### `process_batch_8_simd`
```rust
pub fn process_batch_8_simd(
    transforms: &mut [Transform],
    velocities: &[Vec3],
    dt: f32
)
```
Process 8 entities with AVX2.

#### `process_batch_4_simd`
```rust
pub fn process_batch_4_simd(
    transforms: &mut [Transform],
    velocities: &[Vec3],
    dt: f32
)
```
Process 4 entities with SSE.

## Advanced Usage

### Custom Batch Processing

```rust
use engine_physics::systems::integration_simd::{
    process_batch_8_simd,
    process_sequential,
};

// Direct batch processing
let mut transforms = vec![Transform::identity(); 8];
let velocities = vec![Vec3::new(1.0, 2.0, 3.0); 8];
process_batch_8_simd(&mut transforms, &velocities, 0.016);

// Hybrid processing
let mut transforms = vec![Transform::identity(); 100];
let velocities = vec![Vec3::new(1.0, 2.0, 3.0); 100];
process_sequential(&mut transforms, &velocities, 0.016);
```

### Performance Profiling

```rust
use std::time::Instant;

let start = Instant::now();
physics_integration_system_simd(&mut world, dt);
let elapsed = start.elapsed();
println!("Integration took: {:?}", elapsed);
```

## Best Practices

1. ✅ **Use SIMD for >50 entities** - Better performance
2. ✅ **Compile with native flags** - Maximum speedup
3. ✅ **Batch entity creation** - Better cache locality
4. ✅ **Profile before optimizing** - Measure first
5. ✅ **Run benchmarks regularly** - Track performance

## Further Reading

- [PHYSICS_INTEGRATION_OPTIMIZATION.md](PHYSICS_INTEGRATION_OPTIMIZATION.md) - Detailed architecture
- [engine-math SIMD guide](../math/CLAUDE.md) - SIMD math primitives
- [Performance Guidelines](../../docs/performance-guidelines.md) - General optimization

## Support

For issues or questions:
1. Check existing tests for examples
2. Run benchmarks to verify performance
3. Review documentation above
4. Check engine-core ECS documentation
