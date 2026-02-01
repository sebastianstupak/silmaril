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

| Operation | Target | Achieved |
|-----------|--------|----------|
| Spawn 10k entities | < 1ms | 0.4ms ✅ |
| Query 10k entities (1 component) | < 0.5ms | 0.2ms ✅ |
| Query 10k entities (2 components) | < 1ms | 0.6ms ✅ |
| Add component | < 100ns | ~50ns ✅ |
| Remove component | < 100ns | ~60ns ✅ |

**Benchmarks:** `engine/core/benches/`

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
