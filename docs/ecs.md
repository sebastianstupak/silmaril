# ECS Architecture

> **Entity Component System design for agent-game-engine**
>
> High-performance, cache-friendly ECS optimized for AI agent workflows

---

## Overview

The agent-game-engine uses a custom ECS implementation optimized for:
- **Cache efficiency** - Sparse-set storage with dense iteration
- **Query performance** - Macro-generated queries with zero overhead
- **Type safety** - Compile-time component registration
- **Profiling** - Built-in instrumentation for performance validation

## Core Concepts

### Entities

Entities are unique identifiers represented by generational indices:

```rust
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Entity {
    index: u32,    // Array index for reuse
    generation: u32 // Prevents use-after-free
}
```

**Key Features:**
- 8-byte compact layout for cache efficiency
- Free-list reuse with generation counter
- O(1) liveness checking
- Safe across network serialization

**Implementation:** `engine/core/src/ecs/entity.rs` (519 lines)

### Components

Components are pure data attached to entities:

```rust
pub trait Component: 'static + Send + Sync {
    fn type_id() -> TypeId {
        TypeId::of::<Self>()
    }
}

// Auto-implement for all valid types
impl<T: 'static + Send + Sync> Component for T {}
```

**Common Components:**
- `Transform` - Position, rotation, scale
- `Velocity` - Linear velocity (physics)
- `Health` - Current/max health
- `MeshRenderer` - Rendering data
- `RigidBody` - Physics body
- `AudioSource` - Audio emitter

**Requirements:**
- Must be `'static + Send + Sync`
- Recommended: `Clone + Debug` for serialization
- Use `#[derive(Component)]` for automatic registration

### Storage

Sparse-set storage provides O(1) operations with cache-friendly iteration:

```
Sparse Array (entity index → dense index):
[None, Some(0), None, Some(1), Some(2), None, ...]

Dense Array (component data):
[Component_B, Component_A, Component_D]

Entity Array (entity IDs):
[Entity(1), Entity(3), Entity(4)]
```

**Performance Characteristics:**
- Insert: O(1)
- Remove: O(1)
- Lookup: O(1)
- Iteration: O(n) over dense array only (cache-friendly)

**Implementation:** `engine/core/src/ecs/storage.rs` (620 lines)

### World

The World is the central ECS container:

```rust
pub struct World {
    entities: EntityAllocator,
    components: HashMap<TypeId, Box<dyn ComponentStorage>>,
}

impl World {
    pub fn spawn(&mut self) -> Entity { ... }
    pub fn despawn(&mut self, entity: Entity) { ... }
    pub fn add<T: Component>(&mut self, entity: Entity, component: T) { ... }
    pub fn get<T: Component>(&self, entity: Entity) -> Option<&T> { ... }
    pub fn get_mut<T: Component>(&mut self, entity: Entity) -> Option<&mut T> { ... }
    pub fn remove<T: Component>(&mut self, entity: Entity) -> Option<T> { ... }
}
```

**Implementation:** `engine/core/src/ecs/world.rs` (559 lines)

---

## Query System

### Basic Queries

Query single components:

```rust
// Immutable single component
for transform in world.query::<&Transform>() {
    println!("Position: {:?}", transform.position);
}

// Mutable single component
for health in world.query::<&mut Health>() {
    health.current = health.current.min(health.max);
}
```

### Tuple Queries

Query multiple components (macro-generated for 3-12 components):

```rust
// Two components
for (transform, velocity) in world.query::<(&Transform, &Velocity)>() {
    println!("Entity at {:?} moving {:?}", transform.position, velocity.linear);
}

// Three components with mixed mutability
for (transform, velocity, health) in world.query::<(&Transform, &mut Velocity, &Health)>() {
    if health.current <= 0.0 {
        velocity.linear = Vec3::ZERO; // Stop dead entities
    }
}
```

### Optional Components

Query entities that may or may not have a component:

```rust
for (transform, health) in world.query::<(&Transform, Option<&Health>)>() {
    match health {
        Some(h) => println!("Entity has {} health", h.current),
        None => println!("Entity is invulnerable"),
    }
}
```

### Filter Queries

Filter by component presence/absence:

```rust
// Entities WITH Transform AND Velocity, WITHOUT RigidBody
for transform in world.query::<&Transform>()
    .with::<Velocity>()
    .without::<RigidBody>()
{
    // Kinematic entities only
}
```

### SIMD Batch Queries

Process components in batches for SIMD optimization:

```rust
use engine_core::ecs::BatchQueryIter8;

// Process 8 entities at a time (AVX2/AVX-512)
for batch in world.query::<&mut Transform>().batch_iter_8() {
    // batch is [Transform; 8]
    // SIMD operations on positions
}
```

**Performance:** 35% faster than sequential iteration for large datasets

**Implementation:** `engine/core/src/ecs/query.rs` (2,522 lines)

---

## Systems

Systems are functions that operate on queries:

```rust
use engine_core::ecs::{World, Query};

pub fn movement_system(world: &mut World, dt: f32) {
    for (transform, velocity) in world.query::<(&mut Transform, &Velocity)>() {
        transform.position += velocity.linear * dt;
    }
}

pub fn health_regen_system(world: &mut World, dt: f32) {
    for (health, regen) in world.query::<(&mut Health, &RegenerationRate)>() {
        if health.current < health.max {
            health.current = (health.current + regen.0 * dt).min(health.max);
        }
    }
}
```

### System Scheduling

Systems run in a defined order:

```rust
// Client update loop
fn update(&mut self, dt: f32) {
    // Input processing
    input_system(&mut self.world, &self.input);

    // Game logic
    movement_system(&mut self.world, dt);
    health_regen_system(&mut self.world, dt);

    // Rendering (client only)
    #[cfg(feature = "client")]
    render_system(&mut self.world, &mut self.renderer);
}
```

### Parallel Systems

Systems with non-overlapping queries can run in parallel:

```rust
use rayon::prelude::*;

// These systems access different components, safe to parallelize
rayon::join(
    || movement_system(&world, dt),
    || audio_system(&world, &audio_context),
);
```

**Note:** Requires careful analysis to avoid data races. Use profiling to verify benefits.

---

## Parallel Query Iteration

Leverage multi-core CPUs with automatic parallel query iteration using Rayon.

### Basic Parallel Iteration

Use `par_iter()` and `par_iter_mut()` for automatic parallelization:

```rust
use rayon::prelude::*;

// Parallel read-only iteration
pub fn parallel_read_system(world: &World) {
    world.query::<&Transform>()
        .par_iter()
        .for_each(|transform| {
            // Read operations only
            println!("Position: {:?}", transform.position);
        });
}

// Parallel mutable iteration
pub fn parallel_movement_system(world: &mut World, dt: f32) {
    world.query::<(&mut Position, &Velocity)>()
        .par_iter_mut()
        .for_each(|(pos, vel)| {
            pos.x += vel.x * dt;
            pos.y += vel.y * dt;
            pos.z += vel.z * dt;
        });
}
```

### Chunk-Based Parallelism

Process entities in chunks for better work distribution:

```rust
// Process 1024 entities per thread
world.query::<&mut Health>()
    .par_chunks_mut(1024)
    .for_each(|chunk| {
        for health in chunk {
            health.current = health.current.min(health.max);
        }
    });
```

**Benefits:**
- Reduces thread spawn overhead
- Better cache locality within chunks
- More predictable performance

### Parallel with Rayon Combinators

Use Rayon's powerful combinators:

```rust
use rayon::prelude::*;

// Parallel map
let speeds: Vec<f32> = world.query::<&Velocity>()
    .par_iter()
    .map(|vel| vel.linear.length())
    .collect();

// Parallel filter
let low_health_entities: Vec<Entity> = world.query::<(Entity, &Health)>()
    .par_iter()
    .filter(|(_, health)| health.current < 20.0)
    .map(|(entity, _)| entity)
    .collect();

// Parallel reduce
let total_health: f32 = world.query::<&Health>()
    .par_iter()
    .map(|health| health.current)
    .sum();
```

### Performance Thresholds

Parallel iteration has overhead. Use these guidelines:

```rust
pub fn smart_iteration_system(world: &mut World, dt: f32) {
    let entity_count = world.entity_count();

    if entity_count > 10_000 {
        // Parallel: 3-4x faster for large datasets
        world.query::<(&mut Position, &Velocity)>()
            .par_iter_mut()
            .for_each(|(pos, vel)| {
                update_position(pos, vel, dt);
            });
    } else {
        // Sequential: Less overhead for small datasets
        for (pos, vel) in world.query::<(&mut Position, &Velocity)>().iter_mut() {
            update_position(pos, vel, dt);
        }
    }
}
```

**Benchmark-Derived Thresholds:**
- **< 1,000 entities**: Always sequential (parallel overhead > benefit)
- **1,000-10,000 entities**: Test both, usually sequential wins
- **> 10,000 entities**: Parallel usually 3-4x faster
- **> 100,000 entities**: Parallel strongly recommended (4-5x faster)

### Thread Safety

Parallel iteration is safe when queries don't overlap:

```rust
// ✅ SAFE: Different components
rayon::join(
    || world.query::<&mut Position>().par_iter_mut().for_each(|pos| { ... }),
    || world.query::<&mut Velocity>().par_iter_mut().for_each(|vel| { ... }),
);

// ❌ UNSAFE: Same component (compile error)
rayon::join(
    || world.query::<&mut Position>().par_iter_mut().for_each(|pos| { ... }),
    || world.query::<&mut Position>().par_iter_mut().for_each(|pos| { ... }),
    // ERROR: Cannot borrow world mutably more than once
);
```

### Nested Parallelism

Combine system-level and query-level parallelism:

```rust
pub fn parallel_game_update(world: &mut World, dt: f32) {
    // Run independent systems in parallel
    rayon::scope(|s| {
        // System 1: Update positions (mutable)
        s.spawn(|_| {
            world.query::<(&mut Position, &Velocity)>()
                .par_iter_mut()
                .for_each(|(pos, vel)| {
                    *pos += *vel * dt;
                });
        });

        // System 2: Update health (mutable, different component)
        s.spawn(|_| {
            world.query::<&mut Health>()
                .par_iter_mut()
                .for_each(|health| {
                    health.current = (health.current + 0.1).min(health.max);
                });
        });
    });
}
```

**Warning:** This requires `rayon::scope` to ensure systems don't access the same components.

### Profiling Parallel Queries

Always profile to verify benefit:

```rust
use engine_profiling::profile_scope;

pub fn movement_system(world: &mut World, dt: f32) {
    profile_scope!("movement_system");

    {
        profile_scope!("parallel_query");
        world.query::<(&mut Position, &Velocity)>()
            .par_iter_mut()
            .for_each(|(pos, vel)| {
                *pos += *vel * dt;
            });
    }
}
```

Check profiling results:
- If `parallel_query` takes longer than sequential, reduce parallelism
- Look for thread contention or work imbalance
- Adjust chunk size or threshold

**Implementation:** `engine/core/src/ecs/parallel.rs`

---

## Change Detection

Track which components have been modified for incremental processing and optimization.

### Tick System

The World maintains a monotonically increasing tick counter:

```rust
pub struct World {
    // ... other fields ...
    current_tick: Tick,
}

impl World {
    pub fn current_tick(&self) -> Tick { ... }
    pub fn increment_tick(&mut self) { ... }
}
```

### Component Ticks

Each component stores when it was added and last modified:

```rust
pub struct ComponentTicks {
    added: Tick,
    changed: Tick,
}
```

### Querying Changed Components

Use change detection to process only modified components:

```rust
use engine_core::ecs::{Changed, Tick};

pub fn sync_transforms_system(world: &World) {
    let current_tick = world.current_tick();

    for (entity, transform) in world.query::<(Entity, &Transform)>() {
        if let Some((_, ticks)) = world.get_with_tick::<Transform>(entity) {
            if ticks.is_changed_since(last_sync_tick) {
                // Only sync transforms that changed since last sync
                sync_to_physics(entity, transform);
            }
        }
    }
}
```

### Manual Change Marking

Mark components as changed explicitly:

```rust
// Mark specific component as changed
world.mark_changed::<Transform>(entity);

// Component automatically marked as changed when mutably accessed
if let Some(health) = world.get_mut::<Health>(entity) {
    health.current -= 10.0; // Automatically marked as changed
}
```

### Use Cases

- **Network synchronization**: Only send changed components to clients
- **Rendering**: Only update GPU buffers for modified transforms
- **Physics**: Only rebuild acceleration structures for moved entities
- **Audio**: Only update audio sources that changed position

**Performance Impact:** ~10-15% overhead for tick tracking, but enables 50-90% reduction in downstream processing.

---

## Events

Events enable decoupled inter-system communication without tight coupling.

### Defining Events

Events must implement the `Event` trait:

```rust
use engine_core::ecs::Event;

#[derive(Debug, Clone)]
pub struct CollisionEvent {
    pub entity_a: Entity,
    pub entity_b: Entity,
    pub impulse: f32,
    pub contact_point: Vec3,
}

impl Event for CollisionEvent {}

#[derive(Debug, Clone)]
pub struct PlayerDeathEvent {
    pub player: Entity,
    pub killer: Option<Entity>,
    pub time: f32,
}

impl Event for PlayerDeathEvent {}
```

**Requirements:**
- Must be `Send + Sync + 'static`
- Recommended: `Clone + Debug` for debugging
- Keep events lightweight (data only, no complex logic)

### Event Architecture

Events are stored in type-specific ring buffers:

```
Events
  ├─ CollisionEvent: RingBuffer<CollisionEvent> (capacity: 1024)
  ├─ PlayerDeathEvent: RingBuffer<PlayerDeathEvent> (capacity: 1024)
  └─ DamageEvent: RingBuffer<DamageEvent> (capacity: 1024)
```

**Key Features:**
- Fixed capacity (1024 events per type)
- Oldest events dropped when full
- Multiple readers can independently consume events
- Zero-copy event reading (returns references)

### Sending Events

Systems send events to the World:

```rust
pub fn physics_system(world: &mut World, dt: f32) {
    // ... physics simulation ...

    // Detect collisions
    for (entity_a, entity_b, impulse) in detect_collisions(world) {
        world.send_event(CollisionEvent {
            entity_a,
            entity_b,
            impulse,
            contact_point: Vec3::ZERO,
        });
    }
}

pub fn health_system(world: &mut World) {
    for (entity, health) in world.query::<(Entity, &Health)>() {
        if health.current <= 0.0 {
            world.send_event(PlayerDeathEvent {
                player: entity,
                killer: None,
                time: world.current_time(),
            });
        }
    }
}
```

### Reading Events

Systems use `EventReader` to consume events:

```rust
pub fn audio_system(world: &World, audio: &mut AudioContext) {
    // Create reader (tracks read position)
    let mut collision_reader = world.get_event_reader::<CollisionEvent>();

    // Read all unread events
    for event in world.read_events(&mut collision_reader) {
        let volume = (event.impulse / 100.0).clamp(0.0, 1.0);
        audio.play_sound("collision.wav", volume, event.contact_point);
    }
}

pub fn ui_system(world: &World, ui: &mut UI) {
    let mut death_reader = world.get_event_reader::<PlayerDeathEvent>();

    for event in world.read_events(&mut death_reader) {
        ui.show_death_screen(event.player);

        if let Some(killer) = event.killer {
            ui.show_kill_feed(killer, event.player);
        }
    }
}
```

### Multiple Readers

Multiple systems can read the same events independently:

```rust
// System 1: Audio
pub fn audio_system(world: &World, audio: &mut AudioContext) {
    let mut reader = world.get_event_reader::<CollisionEvent>();
    for event in world.read_events(&mut reader) {
        audio.play_sound("collision.wav", 1.0, event.contact_point);
    }
}

// System 2: Particles
pub fn particle_system(world: &World, particles: &mut ParticleSystem) {
    let mut reader = world.get_event_reader::<CollisionEvent>();
    for event in world.read_events(&mut reader) {
        particles.spawn_at(event.contact_point, "sparks");
    }
}

// Both systems receive all events independently
```

### Event Cleanup

Clear events when no longer needed:

```rust
// Clear specific event type
world.clear_events::<CollisionEvent>();

// Clear all events (typically done each frame)
world.clear_all_events();

// Typical frame loop
fn update(&mut self, dt: f32) {
    // Run systems
    physics_system(&mut self.world, dt);
    audio_system(&self.world, &mut self.audio);
    ui_system(&self.world, &mut self.ui);

    // Clear events at end of frame
    self.world.clear_all_events();
}
```

### Event Best Practices

**DO:**
- ✅ Use events for one-to-many communication
- ✅ Keep event data minimal and focused
- ✅ Clear events each frame to prevent memory growth
- ✅ Use different event types for different concerns
- ✅ Document when events are sent and who reads them

**DON'T:**
- ❌ Store large data in events (use entity references instead)
- ❌ Rely on event ordering (readers may process at different times)
- ❌ Send events in tight loops (batch if possible)
- ❌ Use events for direct entity-to-entity communication (use components)

### Event-Driven Example

Complete combat system using events:

```rust
// Define events
#[derive(Debug, Clone)]
struct AttackEvent { attacker: Entity, target: Entity, damage: f32 }
impl Event for AttackEvent {}

#[derive(Debug, Clone)]
struct DamageEvent { target: Entity, amount: f32, source: DamageSource }
impl Event for DamageEvent {}

#[derive(Debug, Clone)]
struct DeathEvent { entity: Entity, killer: Option<Entity> }
impl Event for DeathEvent {}

// Combat system sends attack events
pub fn combat_system(world: &mut World) {
    for (attacker, weapon, target_pos) in world.query::<(&Entity, &Weapon, &Transform)>() {
        if weapon.cooldown <= 0.0 {
            if let Some(target) = find_target_in_range(world, target_pos, weapon.range) {
                world.send_event(AttackEvent {
                    attacker: *attacker,
                    target,
                    damage: weapon.damage,
                });
            }
        }
    }
}

// Health system processes damage events
pub fn health_system(world: &mut World) {
    let mut attack_reader = world.get_event_reader::<AttackEvent>();

    for event in world.read_events(&mut attack_reader) {
        // Convert attack to damage (could apply armor, resistances, etc.)
        world.send_event(DamageEvent {
            target: event.target,
            amount: event.damage,
            source: DamageSource::Entity(event.attacker),
        });
    }

    let mut damage_reader = world.get_event_reader::<DamageEvent>();

    for event in world.read_events(&mut damage_reader) {
        if let Some(health) = world.get_mut::<Health>(event.target) {
            health.current -= event.amount;

            if health.current <= 0.0 {
                world.send_event(DeathEvent {
                    entity: event.target,
                    killer: match event.source {
                        DamageSource::Entity(e) => Some(e),
                        _ => None,
                    },
                });
            }
        }
    }
}

// Death system cleans up dead entities
pub fn death_system(world: &mut World) {
    let mut death_reader = world.get_event_reader::<DeathEvent>();

    for event in world.read_events(&mut death_reader) {
        world.despawn(event.entity);
    }
}

// Audio/VFX systems respond to events
pub fn audio_combat_system(world: &World, audio: &mut AudioContext) {
    let mut attack_reader = world.get_event_reader::<AttackEvent>();
    for _ in world.read_events(&mut attack_reader) {
        audio.play_sound("attack.wav", 1.0);
    }

    let mut death_reader = world.get_event_reader::<DeathEvent>();
    for _ in world.read_events(&mut death_reader) {
        audio.play_sound("death.wav", 1.0);
    }
}
```

**Implementation:** `engine/core/src/ecs/events.rs`

---

## Serialization

### WorldState

Serialize entire world state for networking/saving:

```rust
use engine_core::serialization::{WorldState, Format};

// Serialize
let world_state = WorldState::from_world(&world);
let bytes = world_state.to_bytes(Format::Bincode)?;

// Deserialize
let world_state = WorldState::from_bytes(&bytes, Format::Bincode)?;
let world = world_state.to_world();
```

### Formats

- **Bincode** - Fast, compact (production)
- **YAML** - Human-readable (debugging)
- **FlatBuffers** - Zero-copy (networking)

**Implementation:** `engine/core/src/serialization/` (partial)

---

## Performance Targets

### Core ECS Operations

| Operation | Target | Achieved | Status |
|-----------|--------|----------|--------|
| Spawn 10k entities | < 1ms | 0.4ms | ✅ 2.5x better |
| Query 10k entities (1 component) | < 0.5ms | 0.2ms | ✅ 2.5x better |
| Query 10k entities (2 components) | < 1ms | 0.6ms | ✅ 1.67x better |
| Add component | < 100ns | ~50ns | ✅ 2x better |
| Remove component | < 100ns | ~60ns | ✅ 1.67x better |
| Component get (read) | < 10ns | ~2ns | ✅ 5x better |
| Component get_mut (write) | < 20ns | ~8ns | ✅ 2.5x better |

### Recent Optimizations (February 2026)

Major performance improvements from optimization work:

| Category | Improvement | Details |
|----------|-------------|---------|
| **Grid Builds** | 35-47% faster | Spatial data structure construction |
| **BVH Builds** | 14-19% faster | Bounding volume hierarchy |
| **Arena Allocator** | 63% faster | 231% throughput increase |
| **Raycasting** | 32% faster | Linear raycast operations |
| **Query Iteration** | 10-15% faster | Through cache optimization |

### Parallel Query Performance

Parallel iteration scales well on multi-core systems:

| Entity Count | Sequential | Parallel (8 cores) | Speedup |
|--------------|------------|--------------------| --------|
| 100 | 2µs | 4µs | 0.5x (overhead) |
| 1,000 | 20µs | 22µs | 0.9x (minimal benefit) |
| 10,000 | 200µs | 60µs | 3.3x ⚡ |
| 100,000 | 2ms | 500µs | 4.0x ⚡⚡ |
| 1,000,000 | 20ms | 5ms | 4.0x ⚡⚡ |

**Recommendation:** Use parallel queries for 10,000+ entities

### Memory Performance

| Metric | Value | Notes |
|--------|-------|-------|
| Entity overhead | 8 bytes | Just the Entity ID |
| SparseSet overhead per entity | 16-24 bytes | Sparse + dense indices |
| Component storage | Exact size | No padding/fragmentation |
| Cache efficiency | >90% | Dense iteration, minimal misses |

### Profiling Results

Real-world ECS usage from profiling (100K entities, 60 FPS target):

```
Frame Budget: 16.67ms
├─ ECS Query (Position + Velocity): 0.2ms (1.2%)
├─ ECS Query (Transform + Mesh): 0.4ms (2.4%)
├─ ECS Component Updates: 0.5ms (3.0%)
├─ Physics (external): 4.0ms (24%)
├─ Rendering (external): 8.0ms (48%)
└─ Other: 3.57ms (21.4%)

Total ECS Overhead: 1.1ms (6.6% of frame)
```

**Result:** ECS is highly efficient, not a bottleneck

**Benchmarks:** `engine/core/benches/`
**Profiling:** See [docs/profiling.md](profiling.md)
**Latest Results:** See [BENCHMARK_RESULTS_2026-02-01.md](../BENCHMARK_RESULTS_2026-02-01.md)

---

## Best Practices

### DO

- ✅ Keep components small and focused (< 64 bytes ideal)
- ✅ Use `#[derive(Component)]` for automatic registration
- ✅ Profile queries with `#[profile(category = "ECS")]`
- ✅ Use batch iterators for SIMD-friendly data
- ✅ Prefer composition over inheritance

### DON'T

- ❌ Store `Vec<T>` in components (prefer indices)
- ❌ Use RefCell/Mutex in components (breaks parallelism)
- ❌ Query in tight loops (cache results)
- ❌ Over-engineer with too many small components

---

## Examples

### Spawning Entities

```rust
let mut world = World::new();

// Spawn player
let player = world.spawn();
world.add(player, Transform::from_translation(Vec3::ZERO));
world.add(player, Health { current: 100.0, max: 100.0 });
world.add(player, Velocity::ZERO);

// Spawn enemy
let enemy = world.spawn();
world.add(enemy, Transform::from_translation(Vec3::new(10.0, 0.0, 0.0)));
world.add(enemy, Health { current: 50.0, max: 50.0 });
```

### Combat System

```rust
pub fn combat_system(world: &mut World) {
    let mut damage_events = Vec::new();

    // Find all attackers and targets
    for (attacker, attacker_transform, weapon) in world.query::<(&Entity, &Transform, &Weapon)>() {
        for (target, target_transform, health) in world.query::<(&Entity, &Transform, &Health)>() {
            if attacker != target {
                let distance = (attacker_transform.position - target_transform.position).length();
                if distance < weapon.range {
                    damage_events.push((*target, weapon.damage));
                }
            }
        }
    }

    // Apply damage
    for (target, damage) in damage_events {
        if let Some(health) = world.get_mut::<Health>(target) {
            health.current -= damage;
            if health.current <= 0.0 {
                world.despawn(target);
            }
        }
    }
}
```

---

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawn_despawn() {
        let mut world = World::new();
        let entity = world.spawn();
        assert!(world.is_alive(entity));

        world.despawn(entity);
        assert!(!world.is_alive(entity));
    }

    #[test]
    fn test_component_add_remove() {
        let mut world = World::new();
        let entity = world.spawn();

        world.add(entity, Health { current: 100.0, max: 100.0 });
        assert!(world.get::<Health>(entity).is_some());

        world.remove::<Health>(entity);
        assert!(world.get::<Health>(entity).is_none());
    }
}
```

### Property Tests

```rust
#[cfg(test)]
mod proptests {
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_query_consistency(entity_count in 0..1000usize) {
            let mut world = World::new();
            let entities: Vec<_> = (0..entity_count).map(|_| world.spawn()).collect();

            for entity in &entities {
                world.add(*entity, Transform::default());
            }

            let queried: Vec<_> = world.query::<&Transform>().collect();
            assert_eq!(queried.len(), entity_count);
        }
    }
}
```

---

## Advanced Topics

### Component Metadata

Track component metadata for debugging:

```rust
pub struct ComponentMetadata {
    pub name: &'static str,
    pub size: usize,
    pub alignment: usize,
}
```

### Entity Relationships

Model parent-child relationships:

```rust
#[derive(Component)]
pub struct Parent(pub Entity);

#[derive(Component)]
pub struct Children(pub Vec<Entity>);

pub fn hierarchy_system(world: &mut World) {
    for (entity, parent) in world.query::<(&Entity, &Parent)>() {
        // Sync child transform to parent
    }
}
```

### Archetype Optimization (Future)

Group entities by component signature for even faster iteration:

```
Archetype 1: [Transform, Velocity, RigidBody]
Archetype 2: [Transform, Health, MeshRenderer]
Archetype 3: [Transform, AudioSource]
```

**Status:** Not implemented (current sparse-set is fast enough)

---

## References

- **Implementation:** `engine/core/src/ecs/`
- **Tests:** `engine/core/tests/`
- **Benchmarks:** `engine/core/benches/`
- **Examples:** `engine/core/examples/profiled_ecs.rs`

**Related Documentation:**
- [Performance Targets](performance-targets.md)
- [Profiling](profiling.md)
- [Testing Strategy](testing-strategy.md)
