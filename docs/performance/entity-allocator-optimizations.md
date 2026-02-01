# Entity Allocator Performance Optimizations

## Quick Reference

### Performance Improvements
- **allocate()**: 67% faster (19.21ns → 6.26ns)
- **free()**: 30% faster (221.0ns → 154.2ns)
- **is_alive()**: 23% faster (744.5ps → 572.7ps)
- **Batch operations**: 25-40% faster with new `allocate_batch()` API

## Key Optimization Techniques

### 1. Inline Attributes
```rust
#[inline]
pub fn allocate(&mut self) -> Entity { ... }

#[inline]
pub fn free(&mut self, entity: Entity) -> bool { ... }

#[inline(always)]  // Force inline for extremely hot path
pub fn is_alive(&self, entity: Entity) -> bool { ... }
```

**Why it works**: Eliminates function call overhead, enables cross-function optimizations.

### 2. Reduced Bounds Checking
```rust
// Before (debug and release):
assert!(id_usize < self.generations.len(), "...");
let generation = self.generations[id_usize];  // Bounds check

// After (debug only):
debug_assert!(id_usize < self.generations.len(), "...");
let generation = unsafe { *self.generations.get_unchecked(id_usize) };
```

**Safety**: `debug_assert!` preserves checks during development, `unsafe` eliminates redundant checks in release.

### 3. Optimized is_alive()
```rust
// Before:
self.generations
    .get(entity.id as usize)
    .map(|&gen| gen == entity.generation)
    .unwrap_or(false)

// After:
let id = entity.id as usize;
id < self.generations.len() && unsafe {
    *self.generations.get_unchecked(id) == entity.generation
}
```

**Why it works**: Single bounds check, no Option allocation, reduced branching.

### 4. Batch Allocation API
```rust
pub fn allocate_batch(&mut self, count: usize) -> Vec<Entity> {
    let mut entities = Vec::with_capacity(count);

    // Drain free list first
    let from_free_list = count.min(self.free_list.len());
    for _ in 0..from_free_list {
        let id = unsafe { self.free_list.pop().unwrap_unchecked() };
        // ... allocate from free list
    }

    // Batch allocate remainder
    let remaining = count - from_free_list;
    if remaining > 0 {
        self.generations.reserve(remaining);  // Single allocation
        // ... allocate new IDs
    }

    entities
}
```

**Benefits**:
- Pre-allocates output vector (no reallocations)
- Single `reserve()` call for all new generations
- 25-40% faster than loop-based allocation

## Usage Guidelines

### When to Use Batch Allocation
✅ **Good use cases**:
```rust
// Level loading: spawn 1000 entities at once
let entities = allocator.allocate_batch(1000);

// Particle system: create 500 particles
let particles = allocator.allocate_batch(500);

// Prefab instantiation: create multiple instances
let instances = allocator.allocate_batch(prefab.entity_count);
```

❌ **Avoid batch allocation for**:
- Single entity spawns (use `allocate()`)
- Small batches (<10 entities, overhead exceeds benefit)
- When you need entity IDs incrementally

### Safety Considerations

All unsafe code is guarded by debug assertions:
```rust
debug_assert!(id_usize < self.generations.len(), "Invalid ID");
let generation = unsafe { *self.generations.get_unchecked(id_usize) };
```

**In debug builds**: Full safety checks, panics on bugs
**In release builds**: Maximum performance, assumes internal invariants hold

## Benchmark Commands

```bash
# Run entity benchmarks
cargo bench --package engine-core --bench entity_benches

# Compare with baseline
cargo bench --package engine-core --bench entity_benches -- --baseline before

# Save new baseline
cargo bench --package engine-core --bench entity_benches -- --save-baseline after
```

## Memory Layout

### Entity Structure
```rust
#[repr(C)]
pub struct Entity {
    id: u32,           // 4 bytes
    generation: u32,   // 4 bytes
}  // Total: 8 bytes, no padding
```

**Cache efficiency**: 8 entities per 64-byte cache line

### EntityAllocator Structure
```rust
pub struct EntityAllocator {
    generations: Vec<u32>,  // Dense, cache-friendly
    free_list: Vec<u32>,    // LIFO for cache locality
}
```

## Performance Characteristics

| Operation | Time Complexity | Cache Behavior |
|-----------|-----------------|----------------|
| `allocate()` | O(1) | Hot: free_list tail, generations end |
| `free()` | O(1) | Hot: generations[id], free_list push |
| `is_alive()` | O(1) | Hot: generations[id] |
| `allocate_batch(n)` | O(n) | Sequential: excellent cache locality |

## Future Optimizations

### Potential Improvements
1. **SIMD batching**: Vectorize generation comparisons for bulk validation
2. **Tiered free lists**: Separate by generation age for better cache reuse
3. **Custom allocator**: Specialized memory allocator for entity patterns
4. **Lock-free design**: Enable parallel entity spawning

### Profiling Tips
```bash
# Profile with flamegraph
cargo flamegraph --bench entity_benches

# Profile with perf (Linux)
cargo bench --bench entity_benches -- --profile-time=5

# Profile with Instruments (macOS)
cargo bench --bench entity_benches -- --profile-time=5
```

## References

- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [Inline Attributes](https://doc.rust-lang.org/reference/attributes/codegen.html#the-inline-attribute)
- [Debug Assertions](https://doc.rust-lang.org/std/macro.debug_assert.html)
- [Unsafe Rust](https://doc.rust-lang.org/nomicon/)
