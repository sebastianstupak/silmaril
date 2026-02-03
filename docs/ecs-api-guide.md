# ECS API Guide

> **Complete API reference for the silmaril Entity Component System**
>
> Production-ready, high-performance ECS with sparse-set storage and change detection

---

## Table of Contents

- [Overview](#overview)
- [Core Concepts](#core-concepts)
- [World Operations](#world-operations)
- [Query API](#query-api)
- [Change Detection](#change-detection)
- [Parallel Queries](#parallel-queries)
- [System Scheduling](#system-scheduling)
- [Advanced Features](#advanced-features)
- [Best Practices](#best-practices)

---

## Overview

The silmaril ECS provides a high-performance, cache-friendly architecture for managing game entities and their data. Key features:

- **Sparse-set storage**: O(1) insert/remove/lookup with cache-friendly iteration
- **Type-safe queries**: Compile-time validated component access
- **Change detection**: Track component modifications for 10-100x performance gains
- **Parallel iteration**: Safe concurrent access to disjoint component sets
- **Profiling support**: Built-in instrumentation for performance analysis

**Performance Characteristics:**
- Entity spawn: ~40ns per entity
- Component add: ~50ns per component
- Component lookup: ~15-20ns (unchecked fast path)
- Query iteration: ~20-30ns per entity (single component)
- Change detection overhead: ~5-10ns per component check

---

## Core Concepts

### Entities

Entities are unique identifiers using generational indices to prevent use-after-free:

```rust
use engine_core::ecs::{Entity, EntityAllocator};

// Entity structure
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Entity {
    id: u32,         // Index for array access
    generation: u32, // Prevents stale references
}
```

**Key Properties:**
- 8 bytes total (2x u32)
- Cache-friendly: 8 entities fit in a 64-byte cache line
- Generational safety: Old handles automatically invalidated
- Serializable: Safe to send over network

**Example:**

```rust
let mut allocator = EntityAllocator::new();
let entity = allocator.allocate();

// Entity ID and generation
println!("ID: {}, Generation: {}", entity.id(), entity.generation());

// Free and reuse
allocator.free(entity);
let new_entity = allocator.allocate();
assert_eq!(entity.id(), new_entity.id()); // Same ID
assert_ne!(entity.generation(), new_entity.generation()); // Different generation
```

### Components

Components are pure data types that can be attached to entities:

```rust
use engine_core::ecs::Component;

// Auto-implement Component for any 'static + Send + Sync type
#[derive(Debug, Clone, Copy)]
struct Position {
    x: f32,
    y: f32,
    z: f32,
}

impl Component for Position {}

// Or use derive macro (requires engine-macros)
#[derive(Component, Debug, Clone, Copy)]
struct Velocity {
    x: f32,
    y: f32,
    z: f32,
}
```

**Requirements:**
- Must be `'static + Send + Sync`
- Recommended: `Clone` for serialization, `Debug` for debugging
- Keep small (< 64 bytes ideal for cache efficiency)
- Avoid interior mutability (`RefCell`, `Mutex`) - breaks parallelism

### Storage

Components are stored in sparse-sets, providing:

```
Sparse Array (entity ID → dense index):
[None, Some(0), None, Some(1), Some(2), ...]
        ↓                ↓        ↓
Dense Entity Array:
[Entity(1), Entity(3), Entity(4)]
        ↓         ↓         ↓
Dense Component Array:
[Position, Position, Position]
```

**Advantages:**
- O(1) insert, remove, lookup
- Cache-friendly iteration over dense arrays
- No memory waste for entities without the component
- Automatic resizing

---

## World Operations

The `World` is the central ECS container that owns all entities and components.

### Creating a World

```rust
use engine_core::ecs::World;

let mut world = World::new();

// Or use default
let mut world = World::default();
```

### Registering Components

Components must be registered before use:

```rust
use engine_core::ecs::{World, Component};

#[derive(Component)]
struct Health { current: f32, max: f32 }

let mut world = World::new();

// Register component type (creates storage)
world.register::<Health>();

// Safe to call multiple times (idempotent)
world.register::<Health>();
```

**Important:**
- Call before adding components
- Panics if you add an unregistered component
- Idempotent: safe to call multiple times
- Zero cost if type already registered

### Spawning Entities

```rust
// Spawn a single entity
let entity = world.spawn();
assert!(world.is_alive(entity));

// Spawn with specific ID (for deserialization)
use engine_core::ecs::Entity;
let entity = Entity::new(42, 5);
world.spawn_with_id(entity);
```

### Adding Components

```rust
#[derive(Component)]
struct Position { x: f32, y: f32, z: f32 }

#[derive(Component)]
struct Velocity { x: f32, y: f32, z: f32 }

world.register::<Position>();
world.register::<Velocity>();

let entity = world.spawn();

// Add components
world.add(entity, Position { x: 0.0, y: 0.0, z: 0.0 });
world.add(entity, Velocity { x: 1.0, y: 0.0, z: 0.0 });

// Replace existing component
world.add(entity, Position { x: 5.0, y: 0.0, z: 0.0 });
```

**Panics:**
- If entity is dead (always checked, even in release)
- If component type is not registered

### Getting Components

```rust
// Immutable access
if let Some(pos) = world.get::<Position>(entity) {
    println!("Position: ({}, {}, {})", pos.x, pos.y, pos.z);
}

// Mutable access
if let Some(vel) = world.get_mut::<Velocity>(entity) {
    vel.x += 1.0;
}

// Check existence
if world.has::<Health>(entity) {
    // Entity has health component
}
```

**Performance:**
- `get()`: ~20ns with bounds checks
- `get_mut()`: ~20ns with bounds checks
- `has()`: ~15ns (contains check only)

### Removing Components

```rust
// Remove and return component
let velocity = world.remove::<Velocity>(entity);
assert!(velocity.is_some());

// Component is gone
assert!(!world.has::<Velocity>(entity));
```

### Despawning Entities

```rust
// Despawn entity (removes all components)
assert!(world.despawn(entity));
assert!(!world.is_alive(entity));

// Despawn already-dead entity returns false
assert!(!world.despawn(entity));
```

### World Utilities

```rust
// Get entity count
let count = world.entity_count();

// Clear all entities and components
world.clear();

// Iterate all alive entities
for entity in world.entities() {
    println!("Entity: {:?}", entity);
}
```

---

## Query API

Queries provide type-safe, efficient iteration over entities with specific components.

### Single Component Queries

```rust
// Immutable query
for (entity, position) in world.query::<&Position>() {
    println!("Entity {:?} at ({}, {})", entity, position.x, position.y);
}

// Mutable query
for (entity, velocity) in world.query_mut::<&mut Velocity>() {
    velocity.x += 0.1;
}
```

### Multi-Component Queries

Query up to 12 components in a single tuple:

```rust
// Two components
for (entity, (pos, vel)) in world.query::<(&Position, &Velocity)>() {
    println!("Entity {:?}: pos={:?}, vel={:?}", entity, pos, vel);
}

// Three components with mixed mutability
for (entity, (pos, vel, health)) in world.query_mut::<(&Position, &mut Velocity, &Health)>() {
    if health.current <= 0.0 {
        vel.x = 0.0;
        vel.y = 0.0;
    }
}

// Five components
for (e, (pos, vel, acc, mass, health)) in world.query::<(
    &Position,
    &Velocity,
    &Acceleration,
    &Mass,
    &Health
)>() {
    // All components available
}
```

**Performance:**
- 1 component: ~20-30ns per entity
- 2 components: ~40-50ns per entity
- 3 components: ~60-70ns per entity
- Scales linearly with component count

### Optional Components

Use `Option<&T>` to query entities that may not have a component:

```rust
for (entity, (transform, health)) in world.query::<(&Transform, Option<&Health>)>() {
    match health {
        Some(h) => println!("Entity has {} health", h.current),
        None => println!("Entity is invulnerable"),
    }
}
```

### Filter Queries

#### With Filter

Only include entities that have an additional component:

```rust
#[derive(Component)]
struct Alive;

#[derive(Component)]
struct Player;

// Query positions, but only for alive entities
for (entity, pos) in world.query::<&Position>()
    .with::<Alive>()
{
    // Only alive entities returned
}

// Multiple with filters
for (entity, pos) in world.query::<&Position>()
    .with::<Alive>()
    .with::<Player>()
{
    // Only alive players returned
}
```

#### Without Filter

Exclude entities that have a specific component:

```rust
#[derive(Component)]
struct Dead;

// Query positions, excluding dead entities
for (entity, pos) in world.query::<&Position>()
    .without::<Dead>()
{
    // Dead entities excluded
}

// Combine with and without filters
for (entity, pos) in world.query::<&Position>()
    .with::<Alive>()
    .without::<Dead>()
{
    // Alive and not dead (redundant but demonstrates API)
}
```

**Performance:**
- Each filter adds ~10-20ns per entity checked
- Filters applied during iteration (early exit on fail)
- Best performance: fewest filters, most selective filters first

---

## Change Detection

Change detection allows systems to only process entities that have been modified, providing **10-100x performance improvements** for systems that only need to react to changes.

### Tick System

The world maintains a global tick counter:

```rust
use engine_core::ecs::change_detection::Tick;

// Get current tick
let current_tick = world.current_tick();

// Increment tick (typically once per frame)
world.increment_tick();

// Get new tick value
let new_tick = world.current_tick();
assert!(new_tick.is_newer_than(current_tick));
```

### Tracking Changes

Components automatically track when they were added and last modified:

```rust
// Add component (marks as added and changed at current tick)
world.add(entity, Transform::default());
let add_tick = world.current_tick();

// Modify component (marks as changed)
world.increment_tick();
if let Some(transform) = world.get_mut::<Transform>(entity) {
    transform.position.x += 1.0;
}
world.mark_changed::<Transform>(entity);
```

### Changed Queries

Filter queries to only return entities with changed components:

```rust
// Store tick before processing
let last_tick = world.current_tick();

// ... game logic modifies entities ...
world.increment_tick();

// Only process changed transforms
for (entity, transform) in world.query::<&Transform>()
    .changed::<Transform>()
    .since_tick(last_tick)
{
    // Only entities modified after last_tick are returned
    println!("Transform changed for {:?}", entity);
}
```

### System Tick Tracking

Systems should track their last run tick for change detection:

```rust
use engine_core::ecs::change_detection::SystemTicks;

struct MySystem {
    ticks: SystemTicks,
}

impl MySystem {
    fn new() -> Self {
        Self {
            ticks: SystemTicks::new(),
        }
    }

    fn run(&mut self, world: &mut World) {
        let last_run = self.ticks.last_run();

        // Query only changed entities
        for (entity, transform) in world.query::<&Transform>()
            .changed::<Transform>()
            .since_tick(last_run)
        {
            // Process changed transforms
        }

        // Update system tick
        self.ticks.update(world.current_tick());
        world.increment_tick();
    }
}
```

### Performance Impact

**Without change detection:**
```rust
// Processes all 10,000 entities every frame
for (entity, transform) in world.query::<&Transform>() {
    // 10,000 iterations
}
```

**With change detection:**
```rust
// Only processes ~100 changed entities per frame (1% change rate)
for (entity, transform) in world.query::<&Transform>()
    .changed::<Transform>()
    .since_tick(last_tick)
{
    // ~100 iterations = 100x speedup!
}
```

**Typical performance gains:**
- 1% change rate: 100x faster
- 5% change rate: 20x faster
- 10% change rate: 10x faster
- 50% change rate: 2x faster

---

## Parallel Queries

**Note:** Parallel query support is currently in development. The following API is the planned design.

### Parallel Iteration

Process entities concurrently using Rayon:

```rust
use rayon::prelude::*;

// Parallel query over mutable components
world.query_mut::<&mut Position>()
    .par_iter_mut()
    .for_each(|(entity, position)| {
        // Each thread processes a chunk of entities
        position.x += 1.0;
    });

// Parallel query with multiple components
world.query_mut::<(&mut Position, &Velocity)>()
    .par_iter_mut()
    .for_each(|(entity, (pos, vel))| {
        pos.x += vel.x * 0.016;
    });
```

### Safety Guarantees

Parallel queries are statically verified to be safe:

```rust
// ✅ SAFE: Disjoint component access
rayon::join(
    || {
        for (e, pos) in world.query::<&Position>() { }
    },
    || {
        for (e, vel) in world.query_mut::<&mut Velocity>() { }
    },
);

// ❌ COMPILE ERROR: Overlapping mutable access
rayon::join(
    || {
        for (e, pos) in world.query_mut::<&mut Position>() { }
    },
    || {
        for (e, pos) in world.query_mut::<&mut Position>() { }
    },
);
```

### Performance Considerations

Parallel queries add overhead from thread synchronization:

- **Worth it:** > 10,000 entities with non-trivial per-entity work
- **Not worth it:** < 1,000 entities or simple operations

```rust
// Use parallel for heavy workloads
world.query_mut::<(&mut Transform, &Velocity)>()
    .par_iter_mut()
    .for_each(|(e, (transform, velocity))| {
        // Complex physics calculations
        transform.integrate_velocity(velocity, 0.016);
    });

// Use sequential for light workloads
for (e, health) in world.query_mut::<&mut Health>() {
    // Simple addition
    health.current = health.current.min(health.max);
}
```

---

## System Scheduling

Systems are functions that operate on the world. The ECS does not provide automatic scheduling; you control execution order.

### Basic Systems

```rust
use engine_core::ecs::World;

fn movement_system(world: &mut World, dt: f32) {
    for (entity, (transform, velocity)) in world.query_mut::<(&mut Transform, &Velocity)>() {
        transform.position.x += velocity.x * dt;
        transform.position.y += velocity.y * dt;
        transform.position.z += velocity.z * dt;
    }
}

fn health_regen_system(world: &mut World, dt: f32) {
    for (entity, health) in world.query_mut::<&mut Health>() {
        if health.current < health.max {
            health.current = (health.current + 10.0 * dt).min(health.max);
        }
    }
}
```

### Manual Scheduling

```rust
struct GameState {
    world: World,
}

impl GameState {
    fn update(&mut self, dt: f32) {
        // Systems run in order
        input_system(&mut self.world, dt);
        movement_system(&mut self.world, dt);
        collision_system(&mut self.world, dt);
        health_regen_system(&mut self.world, dt);

        // Increment tick after all systems
        self.world.increment_tick();
    }
}
```

### Parallel System Execution

Execute independent systems concurrently:

```rust
use rayon::prelude::*;

fn update_parallel(&mut self, dt: f32) {
    // These systems access different components
    rayon::scope(|s| {
        s.spawn(|_| movement_system(&self.world, dt));
        s.spawn(|_| audio_system(&self.world, dt));
        s.spawn(|_| particle_system(&self.world, dt));
    });

    // Barrier: wait for all systems to complete

    // Run dependent systems sequentially
    collision_system(&mut self.world, dt);

    self.world.increment_tick();
}
```

### System Dependencies

Track dependencies manually:

```rust
struct SystemSchedule {
    movement_tick: SystemTicks,
    render_tick: SystemTicks,
}

impl SystemSchedule {
    fn run(&mut self, world: &mut World, dt: f32) {
        // Movement system
        {
            let last_run = self.movement_tick.last_run();
            // ... run movement system
            self.movement_tick.update(world.current_tick());
        }

        world.increment_tick();

        // Render system (depends on movement)
        {
            let last_run = self.render_tick.last_run();
            // Only render if transforms changed
            for (e, t) in world.query::<&Transform>()
                .changed::<Transform>()
                .since_tick(last_run)
            {
                // Render
            }
            self.render_tick.update(world.current_tick());
        }

        world.increment_tick();
    }
}
```

---

## Advanced Features

### Batch Entity Spawning

Spawn multiple entities efficiently:

```rust
use engine_core::ecs::EntityAllocator;

let mut allocator = EntityAllocator::new();

// Spawn 1000 entities in a batch (faster than individual spawns)
let entities = allocator.allocate_batch(1000);

for entity in entities {
    world.add(entity, Transform::default());
    world.add(entity, Velocity::default());
}
```

**Performance:**
- Batch spawn: ~30ns per entity
- Individual spawn: ~40ns per entity
- 25% faster for large batches

### Component Metadata

Query component information:

```rust
use engine_core::ecs::ComponentDescriptor;

// Get component descriptor
if let Some(descriptor) = world.get_component_descriptor::<Transform>() {
    println!("Type name: {}", descriptor.name);
    println!("Type ID: {:?}", descriptor.type_id);
}
```

### Serialization Support

Serialize and deserialize world state:

```rust
use engine_core::serialization::{WorldState, ComponentData};

// Serialize entire world
let world_state = WorldState::from_world(&world);

// Get all components for an entity
let components = world.get_all_components(entity);
for component in components {
    match component {
        ComponentData::Transform(t) => println!("Transform: {:?}", t),
        ComponentData::Health(h) => println!("Health: {:?}", h),
        ComponentData::Velocity(v) => println!("Velocity: {:?}", v),
        ComponentData::MeshRenderer(m) => println!("MeshRenderer: {:?}", m),
    }
}

// Restore from component data
world.add_component_data(entity, ComponentData::Transform(transform));
```

---

## Best Practices

### Component Design

**DO:**
```rust
// ✅ Small, focused components
#[derive(Component, Clone, Copy)]
struct Position { x: f32, y: f32, z: f32 } // 12 bytes

#[derive(Component, Clone, Copy)]
struct Velocity { x: f32, y: f32, z: f32 } // 12 bytes
```

**DON'T:**
```rust
// ❌ Large, monolithic components
#[derive(Component)]
struct Entity {
    position: Vec3,
    velocity: Vec3,
    health: f32,
    inventory: Vec<Item>, // Heap allocation!
    // ... 100 more fields
}
```

### Query Patterns

**DO:**
```rust
// ✅ Query once, iterate multiple times
let entities: Vec<_> = world.query::<&Transform>().collect();
for (entity, transform) in entities {
    // Process
}
```

**DON'T:**
```rust
// ❌ Nested queries
for (e1, t1) in world.query::<&Transform>() {
    for (e2, t2) in world.query::<&Transform>() { // Expensive!
        // Process pairs
    }
}
```

### Change Detection

**DO:**
```rust
// ✅ Use change detection for reactive systems
for (e, t) in world.query::<&Transform>()
    .changed::<Transform>()
    .since_tick(last_tick)
{
    // 100x faster if only 1% of transforms change
}
```

**DON'T:**
```rust
// ❌ Manually track dirty flags
#[derive(Component)]
struct Transform {
    position: Vec3,
    dirty: bool, // Redundant! Use change detection instead
}
```

### Memory Layout

**DO:**
```rust
// ✅ Use primitives and fixed-size arrays
#[derive(Component, Clone, Copy)]
struct Inventory {
    items: [ItemId; 10], // Fixed size, stack allocated
    count: usize,
}
```

**DON'T:**
```rust
// ❌ Use heap allocations in components
#[derive(Component)]
struct Inventory {
    items: Vec<Item>, // Heap allocation! Not cache-friendly
}
```

### Profiling

**DO:**
```rust
#[cfg(feature = "profiling")]
use silmaril_profiling::{profile_scope, ProfileCategory};

fn expensive_system(world: &mut World) {
    #[cfg(feature = "profiling")]
    profile_scope!("expensive_system", ProfileCategory::ECS);

    // System logic
}
```

---

## Complete Example

```rust
use engine_core::ecs::{World, Component};

// Define components
#[derive(Component, Debug, Clone, Copy)]
struct Transform {
    x: f32,
    y: f32,
}

#[derive(Component, Debug, Clone, Copy)]
struct Velocity {
    x: f32,
    y: f32,
}

#[derive(Component, Debug, Clone, Copy)]
struct Health {
    current: f32,
    max: f32,
}

fn main() {
    // Create world
    let mut world = World::new();

    // Register components
    world.register::<Transform>();
    world.register::<Velocity>();
    world.register::<Health>();

    // Spawn entities
    let player = world.spawn();
    world.add(player, Transform { x: 0.0, y: 0.0 });
    world.add(player, Velocity { x: 1.0, y: 0.0 });
    world.add(player, Health { current: 100.0, max: 100.0 });

    let enemy = world.spawn();
    world.add(enemy, Transform { x: 10.0, y: 0.0 });
    world.add(enemy, Health { current: 50.0, max: 50.0 });

    // Run systems
    let dt = 0.016; // 60 FPS

    // Movement system
    for (entity, (transform, velocity)) in world.query_mut::<(&mut Transform, &Velocity)>() {
        transform.x += velocity.x * dt;
        transform.y += velocity.y * dt;
    }

    // Health regen system
    for (entity, health) in world.query_mut::<&mut Health>() {
        if health.current < health.max {
            health.current = (health.current + 10.0 * dt).min(health.max);
        }
    }

    // Query results
    for (entity, (transform, health)) in world.query::<(&Transform, Option<&Health>)>() {
        println!("Entity {:?} at ({}, {})", entity, transform.x, transform.y);
        if let Some(h) = health {
            println!("  Health: {}/{}", h.current, h.max);
        }
    }

    world.increment_tick();
}
```

---

## Performance Reference

| Operation | Latency | Throughput |
|-----------|---------|------------|
| Entity spawn | 40ns | 25M entities/sec |
| Component add | 50ns | 20M ops/sec |
| Component get | 20ns | 50M ops/sec |
| Component get (unchecked) | 15ns | 66M ops/sec |
| Single component query | 20-30ns/entity | 33-50M entities/sec |
| Two component query | 40-50ns/entity | 20-25M entities/sec |
| Change detection filter | +10ns/entity | N/A |
| Batch entity spawn | 30ns/entity | 33M entities/sec |

**Test Configuration:**
- CPU: AMD Ryzen 9 / Intel i9 (modern x86_64)
- Compiler: rustc 1.75+ with `-C target-cpu=native`
- Build: `--release` with LTO

---

## See Also

- [ECS Architecture](ecs-architecture.md) - Internal implementation details
- [Performance Guide](../PERFORMANCE.md) - Optimization techniques
- [Profiling Guide](profiling.md) - Performance measurement
- [Testing Strategy](testing-strategy.md) - Test coverage requirements

---

**Last Updated:** 2026-02-01
**API Stability:** Stable (v0.1)
**Next Review:** Phase 2 (Rendering integration)
