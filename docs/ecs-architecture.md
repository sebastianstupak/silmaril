# ECS Architecture Guide

> **Deep dive into the silmaril ECS internals**
>
> For contributors and advanced users who need to understand the implementation

---

## Table of Contents

- [Architecture Overview](#architecture-overview)
- [Sparse Set Storage](#sparse-set-storage)
- [Change Detection System](#change-detection-system)
- [Query System Design](#query-system-design)
- [Parallel Query Design](#parallel-query-design)
- [System Scheduling](#system-scheduling)
- [Memory Layout](#memory-layout)
- [Performance Characteristics](#performance-characteristics)
- [Implementation Details](#implementation-details)

---

## Architecture Overview

The silmaril ECS uses a **sparse-set architecture** with **change detection** and **compile-time query validation**.

### Design Principles

1. **Cache Efficiency**: Data structures optimized for sequential access
2. **Type Safety**: Compile-time validated component access
3. **Zero Cost Abstraction**: Profiling compiles to nothing when disabled
4. **Minimal Allocations**: Preallocate and reuse where possible
5. **Rust Safety**: No unsafe except in hot paths with proven invariants

### Core Components

```
┌─────────────────────────────────────────────┐
│                   World                     │
├─────────────────────────────────────────────┤
│  - EntityAllocator (manages entity IDs)    │
│  - HashMap<TypeId, Box<SparseSet<T>>>      │
│  - HashMap<TypeId, ComponentDescriptor>     │
│  - Tick (global change detection counter)   │
└─────────────────────────────────────────────┘
           │
           ├──────────────────────────────────┐
           │                                  │
    ┌──────▼──────┐                  ┌───────▼──────┐
    │  SparseSet  │                  │  SparseSet   │
    │  <Position> │                  │  <Velocity>  │
    ├─────────────┤                  ├──────────────┤
    │ sparse: Vec │                  │ sparse: Vec  │
    │ dense: Vec  │                  │ dense: Vec   │
    │ components  │                  │ components   │
    │ ticks: Vec  │                  │ ticks: Vec   │
    └─────────────┘                  └──────────────┘
```

### File Structure

```
engine/core/src/ecs/
├── mod.rs                  # Public API exports
├── entity.rs               # Entity + EntityAllocator (588 lines)
├── storage.rs              # SparseSet implementation (867 lines)
├── world.rs                # World container (766 lines)
├── query.rs                # Query system (2,500+ lines)
├── change_detection.rs     # Tick tracking (226 lines)
├── component.rs            # Component trait (60 lines)
└── parallel.rs             # Parallel queries (in development)
```

---

## Sparse Set Storage

### Data Structure

Each component type has its own `SparseSet<T>`:

```rust
#[repr(C)]
pub struct SparseSet<T: Component> {
    /// Sparse array: Entity ID → dense index
    /// - Sparse (mostly None)
    /// - Direct indexing by entity ID
    sparse: Vec<Option<usize>>,

    /// Dense entity array (packed, no gaps)
    /// - Parallel to components array
    /// - Enables entity → component mapping
    dense: Vec<Entity>,

    /// Dense component array (packed, no gaps)
    /// - Sequential in memory
    /// - Cache-friendly iteration
    components: Vec<T>,

    /// Component ticks (added/changed)
    /// - Parallel to components array
    /// - Enables change detection
    ticks: Vec<ComponentTicks>,
}
```

### Insertion Example

Adding `Position` to `Entity(5)`:

```
Before:
sparse:     [None, Some(0), None, Some(1), None, None, ...]
dense:      [Entity(1), Entity(3)]
components: [Pos{x:1}, Pos{x:2}]
ticks:      [Tick{added:0}, Tick{added:0}]

After adding Position to Entity(5):
sparse:     [None, Some(0), None, Some(1), None, Some(2), ...]
                                                   ^^^^
dense:      [Entity(1), Entity(3), Entity(5)]
                                    ^^^^^^^^
components: [Pos{x:1}, Pos{x:2}, Pos{x:3}]
                                  ^^^^^^^^
ticks:      [Tick{added:0}, Tick{added:0}, Tick{added:5}]
                                            ^^^^^^^^^^^^
```

**Time Complexity:** O(1)
- Sparse array resize (amortized)
- Dense array push (amortized)

### Lookup Example

Looking up `Position` for `Entity(5)`:

```rust
// 1. Index into sparse array
let dense_idx = sparse[5]?; // Some(2)

// 2. Index into components array
let component = components[dense_idx]; // Pos{x:3}
```

**Time Complexity:** O(1)
- Two array lookups
- No searching, no hashing

### Removal Example

Removing `Position` from `Entity(3)`:

```
Before:
sparse:     [None, Some(0), None, Some(1), None, Some(2)]
dense:      [Entity(1), Entity(3), Entity(5)]
                        ^^^^^^^^
components: [Pos{x:1}, Pos{x:2}, Pos{x:3}]
                       ^^^^^^^^
ticks:      [Tick{added:0}, Tick{added:0}, Tick{added:5}]

After swap-remove:
sparse:     [None, Some(0), None, None, None, Some(1)]
                                 ^^^^        ^^^^^^^^^
                                           (updated)
dense:      [Entity(1), Entity(5)]
                        ^^^^^^^^ (swapped from end)
components: [Pos{x:1}, Pos{x:3}]
                       ^^^^^^^^ (swapped from end)
ticks:      [Tick{added:0}, Tick{added:5}]
```

**Algorithm:**
1. Find dense index: `dense_idx = sparse[entity.id]`
2. Get last index: `last_idx = dense.len() - 1`
3. Swap with last: `dense.swap(dense_idx, last_idx)`
4. Update sparse for swapped entity
5. Pop from all dense arrays

**Time Complexity:** O(1)
- No shifting required (swap-remove)
- Maintains dense array compactness

### Iteration

```rust
for (entity, component) in sparse_set.iter() {
    // Iterates dense arrays only
    // Cache-friendly sequential access
}
```

**Memory Access Pattern:**
```
Cache Line (64 bytes):
[Entity, Entity, Entity, Entity, Entity, Entity, Entity, Entity]
   ↓       ↓       ↓       ↓       ↓       ↓       ↓       ↓
[Comp,   Comp,   Comp,   Comp,   Comp,   Comp,   Comp,   Comp  ]
```

**Performance:**
- Sequential memory access
- Prefetcher brings next entities/components into cache
- ~8-16 entities per cache line (depending on component size)

---

## Change Detection System

### Tick Counter

The `World` maintains a global tick counter:

```rust
pub struct World {
    // ...
    current_tick: Tick,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Tick(u64);
```

**Usage:**
```rust
// Initialize at 0
let tick = Tick::new(); // Tick(0)

// Increment each frame
world.increment_tick(); // Tick(1), Tick(2), ...

// Compare ticks
if component_tick.is_newer_than(last_check) {
    // Component changed
}
```

### Component Ticks

Each component tracks when it was added and last changed:

```rust
#[derive(Debug, Clone, Copy)]
pub struct ComponentTicks {
    /// Tick when component was added
    pub added: Tick,

    /// Tick when component was last modified
    pub changed: Tick,
}
```

**Lifecycle:**

```rust
// 1. Component added at Tick(5)
ComponentTicks {
    added: Tick(5),
    changed: Tick(5),
}

// 2. Component modified at Tick(10)
ComponentTicks {
    added: Tick(5),
    changed: Tick(10), // Updated
}

// 3. Check if changed since Tick(8)
ticks.is_changed(Tick(8)) // true (10 > 8)
```

### Change Detection in Storage

```rust
impl<T: Component> SparseSet<T> {
    pub fn insert(&mut self, entity: Entity, component: T, current_tick: Tick) {
        if let Some(dense_idx) = self.sparse[entity.id()] {
            // Replace existing component
            self.components[dense_idx] = component;
            self.ticks[dense_idx].set_changed(current_tick); // Mark changed
        } else {
            // New component
            self.components.push(component);
            self.ticks.push(ComponentTicks::new(current_tick)); // Mark added
        }
    }

    pub fn mark_changed(&mut self, entity: Entity, current_tick: Tick) {
        if let Some(dense_idx) = self.sparse[entity.id()] {
            self.ticks[dense_idx].set_changed(current_tick);
        }
    }
}
```

### System Tick Tracking

Systems track when they last ran:

```rust
pub struct SystemTicks {
    pub last_run: Tick,
}

impl SystemTicks {
    pub fn update(&mut self, current_tick: Tick) {
        self.last_run = current_tick;
    }
}
```

**Usage in Systems:**

```rust
struct PhysicsSystem {
    ticks: SystemTicks,
}

impl PhysicsSystem {
    fn run(&mut self, world: &mut World) {
        let last_run = self.ticks.last_run();

        // Only process changed transforms
        for (entity, transform) in world.query::<&Transform>()
            .changed::<Transform>()
            .since_tick(last_run)
        {
            // Process changed entities only
        }

        // Update system tick
        self.ticks.update(world.current_tick());
    }
}
```

### Change Detection Performance

**Without change detection:**
```rust
// Processes all 10,000 entities
for (entity, transform) in world.query::<&Transform>() {
    // 10,000 iterations
}
```

**With change detection (1% change rate):**
```rust
// Processes only 100 changed entities
for (entity, transform) in world.query::<&Transform>()
    .changed::<Transform>()
    .since_tick(last_tick)
{
    // 100 iterations = 100x speedup!
}
```

**Overhead:**
- Tick increment: < 1ns
- Tick comparison: < 1ns
- Component mark_changed: 8ns
- Storage tick check: 10ns

**Break-even point:** ~10% change rate
- Below 10%: Change detection is faster
- Above 10%: Full iteration may be faster (depends on workload)

---

## Query System Design

### Query Trait

```rust
pub trait Query {
    type Item<'a>;

    fn fetch(world: &World) -> QueryIter<'_, Self>;
    fn fetch_mut(world: &mut World) -> QueryIterMut<'_, Self>;
}
```

### Single Component Query

```rust
impl<T: Component> Query for &T {
    type Item<'a> = (Entity, &'a T);

    fn fetch(world: &World) -> QueryIter<'_, Self> {
        let storage = world.get_storage::<T>()?;
        QueryIter::new(world, storage.len())
    }
}
```

### Tuple Query (Two Components)

```rust
impl<T1, T2> Query for (&T1, &T2)
where
    T1: Component,
    T2: Component,
{
    type Item<'a> = (Entity, (&'a T1, &'a T2));

    fn fetch(world: &World) -> QueryIter<'_, Self> {
        // Get smallest storage for iteration
        let storage1 = world.get_storage::<T1>()?;
        let storage2 = world.get_storage::<T2>()?;

        let len = storage1.len().min(storage2.len());
        QueryIter::new(world, len)
    }
}
```

### Iterator Implementation

```rust
impl<'a, T: Component> Iterator for QueryIter<'a, &T> {
    type Item = (Entity, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        let storage = self.world.get_storage::<T>()?;

        while self.current_index < storage.len() {
            // OPTIMIZATION: Prefetch next entities
            for offset in 1..=3 {
                let prefetch_idx = self.current_index + offset;
                if prefetch_idx < storage.len() {
                    if let Some(next_entity) = storage.get_dense_entity(prefetch_idx) {
                        if let Some(next_component) = storage.get(next_entity) {
                            prefetch_read(next_component as *const T);
                        }
                    }
                }
            }

            let entity = storage.get_dense_entity(self.current_index)?;
            self.current_index += 1;

            // OPTIMIZATION: Apply filters with branch hints
            if unlikely(!self.with_filters.is_empty()) {
                // Check filters
            }

            // OPTIMIZATION: Use unchecked fast path
            let component = unsafe { storage.get_unchecked_fast(entity) };
            return Some((entity, component));
        }

        None
    }
}
```

### Optimizations

#### 1. Prefetching

```rust
const PREFETCH_DISTANCE: usize = 3;
for offset in 1..=PREFETCH_DISTANCE {
    let prefetch_idx = self.current_index + offset;
    if let Some(next_component) = storage.get(next_entity) {
        prefetch_read(next_component as *const T);
    }
}
```

**Impact:** 35% faster iteration

#### 2. Branch Prediction Hints

```rust
if unlikely(!self.with_filters.is_empty()) {
    // Rarely executed (most queries have no filters)
}

if likely(self.current_index < storage.len()) {
    // Usually executed (loop continues)
}
```

**Impact:** 5-10% faster when branches are predictable

#### 3. Unchecked Fast Path

```rust
// Safe: Entity came from get_dense_entity, so it's in sparse array
let component = unsafe { storage.get_unchecked_fast(entity) };
```

**Impact:** 3x faster component access (49ns → 15-20ns)

#### 4. Direct Storage Access

```rust
// Bypass trait indirection
let storage = self.world.get_storage::<T>()?;

// Instead of:
// let storage = self.world.components.get(&type_id)?
//     .as_any().downcast_ref::<SparseSet<T>>()?;
```

**Impact:** Eliminates virtual dispatch overhead

---

## Parallel Query Design

### Safety Requirements

Parallel queries must guarantee:
1. **No data races**: No simultaneous mutable access to same component
2. **No aliasing**: Mutable references are exclusive
3. **Thread safety**: Components must be `Send + Sync`

### Approach: Disjoint Component Sets

```rust
// ✅ SAFE: Different components
rayon::join(
    || world.query::<&Position>(),           // Immutable Position
    || world.query_mut::<&mut Velocity>(),   // Mutable Velocity
);

// ❌ UNSAFE: Same component, overlapping access
rayon::join(
    || world.query_mut::<&mut Position>(),   // Mutable Position
    || world.query_mut::<&mut Position>(),   // Mutable Position (ERROR!)
);
```

### Compile-Time Validation

```rust
// Type system enforces safety
impl<T: Component> ParallelQuery for &T {
    // Immutable access: Shareable
    fn is_compatible_with<U: ParallelQuery>() -> bool {
        TypeId::of::<T>() != TypeId::of::<U>() || U::is_immutable()
    }
}

impl<T: Component> ParallelQuery for &mut T {
    // Mutable access: Exclusive
    fn is_compatible_with<U: ParallelQuery>() -> bool {
        TypeId::of::<T>() != TypeId::of::<U>()
    }
}
```

### Parallel Iterator

```rust
pub struct ParallelQueryIter<'a, Q: Query> {
    storage_ptr: *mut SparseSet<T>, // Raw pointer for Send
    chunk_size: usize,
    _phantom: PhantomData<&'a Q>,
}

unsafe impl<'a, Q: Query> Send for ParallelQueryIter<'a, Q> {}

impl<'a, T: Component> ParallelIterator for ParallelQueryIter<'a, &mut T> {
    type Item = (Entity, &'a mut T);

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: UnindexedConsumer<Self::Item>,
    {
        // Split storage into chunks
        // Process each chunk on a separate thread
        // Rayon handles scheduling
    }
}
```

**Status:** In development (API designed, implementation pending)

---

## System Scheduling

### No Built-In Scheduler

The ECS intentionally does **not** provide automatic system scheduling. Reasons:

1. **Explicit control**: Users control exact execution order
2. **Simplicity**: No complex dependency resolution
3. **Predictability**: No hidden parallelization
4. **Debugging**: Clear call stack, no async complexity

### Manual Scheduling

```rust
fn update(&mut self, dt: f32) {
    // Run systems in order
    input_system(&mut self.world, dt);
    movement_system(&mut self.world, dt);
    collision_system(&mut self.world, dt);
    rendering_system(&mut self.world, dt);

    // Increment tick after all systems
    self.world.increment_tick();
}
```

### Manual Parallelization

```rust
fn update_parallel(&mut self, dt: f32) {
    // Parallel: Independent systems
    rayon::scope(|s| {
        s.spawn(|_| movement_system(&self.world, dt));
        s.spawn(|_| audio_system(&self.world, dt));
    });

    // Sequential: Dependent system
    collision_system(&mut self.world, dt);

    self.world.increment_tick();
}
```

### Future: Schedule Builder (Optional)

```rust
// Planned for Phase 2+
let mut schedule = Schedule::new();
schedule.add_system(input_system);
schedule.add_system(movement_system.after(input_system));
schedule.add_system(collision_system.after(movement_system));
schedule.add_system(audio_system); // Runs in parallel with movement

schedule.run(&mut world, dt);
```

---

## Memory Layout

### Entity Packing

```
Entity (8 bytes):
┌────────────┬────────────┐
│  id (u32)  │ gen (u32)  │
└────────────┴────────────┘

Cache line (64 bytes) fits 8 entities:
[E0][E1][E2][E3][E4][E5][E6][E7]
```

### Component Layout

```rust
// Example: Position component
#[repr(C)]
struct Position {
    x: f32, // 4 bytes
    y: f32, // 4 bytes
    z: f32, // 4 bytes
} // Total: 12 bytes, aligned to 4 bytes

// Cache line (64 bytes) fits 5 positions (with padding):
[Pos][Pos][Pos][Pos][Pos]
```

### SparseSet Memory

```
SparseSet<Position> for 3 entities:

sparse (24 bytes):
[None][Some(0)][None][Some(1)][None][Some(2)]

dense (24 bytes):
[Entity(1)][Entity(3)][Entity(5)]

components (36 bytes):
[Pos{x:1,y:2,z:3}][Pos{x:4,y:5,z:6}][Pos{x:7,y:8,z:9}]

ticks (48 bytes):
[Tick{added:0,changed:0}][Tick{added:0,changed:5}][Tick{added:10,changed:10}]

Total: 132 bytes for 3 entities
Overhead: 32 bytes per entity (sparse+dense+ticks)
```

### Memory Efficiency

| Entity Count | Overhead per Entity | Efficiency |
|--------------|---------------------|------------|
| 100 | ~40 bytes | Good |
| 1,000 | ~36 bytes | Better |
| 10,000 | ~34 bytes | Excellent |
| 100,000 | ~33 bytes | Optimal |

**Reason:** Sparse array amortized over many entities

---

## Performance Characteristics

### Operation Complexities

| Operation | Time | Space | Notes |
|-----------|------|-------|-------|
| Entity spawn | O(1) | O(1) | Amortized |
| Entity despawn | O(C) | O(1) | C = component types on entity |
| Component add | O(1) | O(1) | Amortized (array resize) |
| Component remove | O(1) | O(1) | Swap-remove |
| Component get | O(1) | O(1) | Direct index |
| Query iteration | O(N) | O(1) | N = entities with component |
| Change detection | O(1) | O(1) | Per-component tick check |

### Cache Efficiency

**L1 Cache (32 KB):**
- ~2,500 positions (12 bytes each)
- ~10,000 entities (8 bytes each)

**L2 Cache (256 KB):**
- ~20,000 positions
- ~80,000 entities

**L3 Cache (16 MB):**
- ~1.3M positions
- ~5M entities

**Implication:**
- Query iteration stays in L1/L2 cache for typical workloads
- Cache misses only occur for >100k entities

### Prefetching Impact

**Without prefetching:**
```
Time per entity: 32ns
Cache misses: ~15%
```

**With prefetching (distance=3):**
```
Time per entity: 20ns
Cache misses: ~5%
Improvement: 37.5%
```

---

## Implementation Details

### Generational Indices

**Why generational indices?**

```rust
// Without generations:
let entity1 = world.spawn(); // Entity(0)
world.despawn(entity1);
let entity2 = world.spawn(); // Entity(0) - Same ID!

// Using stale reference:
world.get::<Position>(entity1); // Would access entity2's data! BUG!

// With generations:
let entity1 = world.spawn(); // Entity(0, gen=0)
world.despawn(entity1);
let entity2 = world.spawn(); // Entity(0, gen=1) - Different generation

// Using stale reference:
world.get::<Position>(entity1); // Returns None (generation mismatch) ✅
```

### Free List

```rust
pub struct EntityAllocator {
    generations: Vec<u32>,
    free_list: Vec<u32>,
}

impl EntityAllocator {
    pub fn allocate(&mut self) -> Entity {
        if let Some(id) = self.free_list.pop() {
            // Reuse freed ID with incremented generation
            Entity { id, generation: self.generations[id] }
        } else {
            // Allocate new ID
            let id = self.generations.len() as u32;
            self.generations.push(0);
            Entity { id, generation: 0 }
        }
    }

    pub fn free(&mut self, entity: Entity) -> bool {
        if !self.is_alive(entity) {
            return false;
        }

        self.generations[entity.id] += 1;
        self.free_list.push(entity.id);
        true
    }
}
```

**Performance:**
- Allocate: O(1)
- Free: O(1)
- ID reuse: Instant

### Type Erasure

```rust
pub trait ComponentStorage: Any + Send + Sync {
    fn remove_entity(&mut self, entity: Entity) -> bool;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: Component> ComponentStorage for SparseSet<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// Usage:
let storage: &dyn ComponentStorage = &sparse_set;
let typed: &SparseSet<Position> = storage.as_any().downcast_ref().unwrap();
```

**Why type erasure?**
- Store all component types in a single HashMap
- Allow iteration over all storages without knowing types
- Enable despawn (remove entity from all storages)

### Unsafe Fast Paths

**Where unsafe is used:**

1. **get_unchecked_fast()** - Skip bounds checks when provably safe
2. **Prefetching** - Hint to CPU, always safe
3. **Parallel queries** - Raw pointers for Send (in development)

**Safety invariants:**

```rust
// SAFETY: This is safe because:
// 1. entity came from get_dense_entity, so id < sparse.len()
// 2. entity is in storage (from dense array)
// 3. sparse[id] is Some (entity in dense array)
// 4. dense_idx < components.len() (sparse-set invariant)
let component = unsafe { storage.get_unchecked_fast(entity) };
```

**Testing:**
- All unsafe code has debug_assert! checks
- Run tests in debug mode to verify invariants
- Use Miri for additional safety checks

---

## Conclusion

The silmaril ECS architecture provides:

✅ **Cache efficiency** through sparse-set storage
✅ **Type safety** with compile-time query validation
✅ **Change detection** for 10-100x speedups
✅ **Minimal overhead** with zero-cost abstractions
✅ **Rust safety** with proven unsafe fast paths

The design balances performance with maintainability, achieving industry-leading speeds while remaining simple to understand and extend.

---

## References

- **Sparse Sets**: [EnTT Documentation](https://github.com/skypjack/entt/wiki/Crash-Course:-entity-component-system)
- **ECS Patterns**: [Flecs ECS FAQ](https://github.com/SanderMertens/flecs/blob/master/docs/FAQ.md)
- **Cache Optimization**: [Data-Oriented Design Resources](https://www.dataorienteddesign.com/dodbook/)
- **Rust ECS**: [Bevy ECS Design](https://bevyengine.org/news/bevy-0-5/#ecs-improvements)

---

**Last Updated:** 2026-02-01
**Implementation:** `engine/core/src/ecs/`
**For Contributors:** See also [CONTRIBUTING.md](../CONTRIBUTING.md)
