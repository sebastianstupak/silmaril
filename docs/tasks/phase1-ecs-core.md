# Phase 1.1: Core ECS Foundation

**Status:** ⚪ Not Started
**Estimated Time:** 5-7 days
**Priority:** Critical (blocks all other work)

---

## 🎯 **Objective**

Implement the foundational ECS (Entity Component System) with:
- Generational entity handles (prevents use-after-free)
- Sparse-set component storage (fast add/remove)
- World container (owns all data)
- Component registration system
- Basic single-component queries

This is the **most critical** piece of the engine. Everything else builds on ECS.

---

## 📋 **Detailed Tasks**

### **1. Entity Allocator** (Day 1-2)

**File:** `engine/core/src/ecs/entity.rs`

**Requirements:**
- Generational indices (ID + generation)
- Free list for recycling IDs
- Handle stale entity references safely

**Implementation:**

```rust
/// Entity handle - opaque, copyable, hashable
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Entity {
    id: u32,
    generation: u32,
}

impl Entity {
    pub fn id(&self) -> u32 { self.id }
    pub fn generation(&self) -> u32 { self.generation }
}

/// Allocates and tracks entities
pub struct EntityAllocator {
    generations: Vec<u32>,
    free_list: Vec<u32>,
}

impl EntityAllocator {
    pub fn new() -> Self {
        Self {
            generations: Vec::new(),
            free_list: Vec::new(),
        }
    }

    /// Allocate a new entity or reuse a freed one
    pub fn allocate(&mut self) -> Entity {
        if let Some(id) = self.free_list.pop() {
            // Reuse freed ID with incremented generation
            Entity {
                id,
                generation: self.generations[id as usize],
            }
        } else {
            // Allocate new ID
            let id = self.generations.len() as u32;
            self.generations.push(0);
            Entity { id, generation: 0 }
        }
    }

    /// Free an entity (doesn't delete immediately, increments generation)
    pub fn free(&mut self, entity: Entity) -> bool {
        if !self.is_alive(entity) {
            return false;
        }

        // Increment generation to invalidate old handles
        self.generations[entity.id as usize] += 1;
        self.free_list.push(entity.id);
        true
    }

    /// Check if entity handle is still valid
    pub fn is_alive(&self, entity: Entity) -> bool {
        self.generations
            .get(entity.id as usize)
            .map(|&gen| gen == entity.generation)
            .unwrap_or(false)
    }

    /// Clear all entities
    pub fn clear(&mut self) {
        self.generations.clear();
        self.free_list.clear();
    }
}
```

**Tests:**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_allocate() {
        let mut alloc = EntityAllocator::new();
        let e1 = alloc.allocate();
        let e2 = alloc.allocate();

        assert_ne!(e1.id, e2.id);
        assert!(alloc.is_alive(e1));
        assert!(alloc.is_alive(e2));
    }

    #[test]
    fn test_entity_free_and_reuse() {
        let mut alloc = EntityAllocator::new();
        let e1 = alloc.allocate();

        alloc.free(e1);
        assert!(!alloc.is_alive(e1));

        // Reuse same ID, different generation
        let e2 = alloc.allocate();
        assert_eq!(e1.id, e2.id);
        assert_ne!(e1.generation, e2.generation);
        assert!(alloc.is_alive(e2));
        assert!(!alloc.is_alive(e1));  // Old handle invalid
    }

    #[test]
    fn test_many_entities() {
        let mut alloc = EntityAllocator::new();
        let entities: Vec<_> = (0..10_000).map(|_| alloc.allocate()).collect();

        // All unique
        let mut ids: Vec<_> = entities.iter().map(|e| e.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 10_000);
    }
}
```

**Performance Test:**

```rust
#[bench]
fn bench_entity_allocate(b: &mut Bencher) {
    let mut alloc = EntityAllocator::new();
    b.iter(|| {
        black_box(alloc.allocate());
    });
}
// Target: < 0.1μs per allocation
```

---

### **2. Component Trait** (Day 2)

**File:** `engine/core/src/ecs/component.rs`

```rust
use std::any::TypeId;

/// Marker trait for components
/// Components are pure data, no methods
pub trait Component: 'static + Send + Sync {}

/// Component metadata
pub struct ComponentDescriptor {
    pub type_id: TypeId,
    pub type_name: &'static str,
    pub size: usize,
    pub align: usize,
}

impl ComponentDescriptor {
    pub fn new<T: Component>() -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            type_name: std::any::type_name::<T>(),
            size: std::mem::size_of::<T>(),
            align: std::mem::align_of::<T>(),
        }
    }
}
```

**Example Components:**

```rust
#[derive(Debug, Clone, Copy, Component)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

#[derive(Debug, Clone, Component)]
pub struct Name(pub String);

#[derive(Component)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}
```

---

### **3. Sparse-Set Storage** (Day 2-3)

**File:** `engine/core/src/ecs/storage.rs`

**Why sparse-set:** Fast O(1) insert/remove, good cache locality for iteration

```rust
/// Sparse-set storage for a single component type
pub struct SparseSet<T: Component> {
    sparse: Vec<Option<usize>>,  // Entity ID → dense index
    dense: Vec<Entity>,           // Dense entity array
    components: Vec<T>,           // Dense component array
}

impl<T: Component> SparseSet<T> {
    pub fn new() -> Self {
        Self {
            sparse: Vec::new(),
            dense: Vec::new(),
            components: Vec::new(),
        }
    }

    pub fn insert(&mut self, entity: Entity, component: T) {
        // Ensure sparse array is large enough
        let idx = entity.id as usize;
        if idx >= self.sparse.len() {
            self.sparse.resize(idx + 1, None);
        }

        if let Some(dense_idx) = self.sparse[idx] {
            // Component exists, replace
            self.components[dense_idx] = component;
        } else {
            // New component, add to dense arrays
            let dense_idx = self.dense.len();
            self.sparse[idx] = Some(dense_idx);
            self.dense.push(entity);
            self.components.push(component);
        }
    }

    pub fn remove(&mut self, entity: Entity) -> Option<T> {
        let idx = entity.id as usize;
        let dense_idx = self.sparse.get(idx)?.take()?;

        // Swap-remove from dense arrays
        let last_idx = self.dense.len() - 1;

        if dense_idx != last_idx {
            // Swap with last element
            self.dense.swap(dense_idx, last_idx);
            self.components.swap(dense_idx, last_idx);

            // Update sparse index for swapped entity
            let swapped_entity = self.dense[dense_idx];
            self.sparse[swapped_entity.id as usize] = Some(dense_idx);
        }

        self.dense.pop();
        Some(self.components.pop().unwrap())
    }

    pub fn get(&self, entity: Entity) -> Option<&T> {
        let idx = entity.id as usize;
        let dense_idx = *self.sparse.get(idx)?.as_ref()?;
        Some(&self.components[dense_idx])
    }

    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut T> {
        let idx = entity.id as usize;
        let dense_idx = *self.sparse.get(idx)?.as_ref()?;
        Some(&mut self.components[dense_idx])
    }

    pub fn iter(&self) -> impl Iterator<Item = (Entity, &T)> {
        self.dense.iter().copied().zip(self.components.iter())
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Entity, &mut T)> {
        self.dense.iter().copied().zip(self.components.iter_mut())
    }

    pub fn len(&self) -> usize {
        self.dense.len()
    }

    pub fn clear(&mut self) {
        self.sparse.clear();
        self.dense.clear();
        self.components.clear();
    }
}
```

**Tests:**

```rust
#[test]
fn test_sparse_set_insert_get() {
    let mut storage = SparseSet::<Health>::new();
    let entity = Entity { id: 0, generation: 0 };

    storage.insert(entity, Health { current: 100.0, max: 100.0 });

    let health = storage.get(entity).unwrap();
    assert_eq!(health.current, 100.0);
}

#[test]
fn test_sparse_set_remove() {
    let mut storage = SparseSet::<Health>::new();
    let e1 = Entity { id: 0, generation: 0 };
    let e2 = Entity { id: 1, generation: 0 };

    storage.insert(e1, Health { current: 100.0, max: 100.0 });
    storage.insert(e2, Health { current: 50.0, max: 100.0 });

    storage.remove(e1);

    assert!(storage.get(e1).is_none());
    assert!(storage.get(e2).is_some());
}

#[test]
fn test_sparse_set_iteration() {
    let mut storage = SparseSet::<i32>::new();

    for i in 0..100 {
        storage.insert(Entity { id: i, generation: 0 }, i as i32);
    }

    let count: usize = storage.iter().count();
    assert_eq!(count, 100);
}
```

**Performance:**

```rust
#[bench]
fn bench_sparse_set_insert(b: &mut Bencher) {
    let mut storage = SparseSet::<Transform>::new();
    let mut id = 0;
    b.iter(|| {
        storage.insert(
            Entity { id, generation: 0 },
            Transform::default()
        );
        id += 1;
    });
}
// Target: < 0.2μs per insert
```

---

### **4. World Container** (Day 3-4)

**File:** `engine/core/src/ecs/world.rs`

```rust
use std::any::TypeId;
use std::collections::HashMap;

pub struct World {
    entities: EntityAllocator,
    components: HashMap<TypeId, Box<dyn Any>>,  // Type-erased storage
}

impl World {
    pub fn new() -> Self {
        Self {
            entities: EntityAllocator::new(),
            components: HashMap::new(),
        }
    }

    /// Register a component type
    pub fn register<T: Component>(&mut self) {
        let type_id = TypeId::of::<T>();
        if !self.components.contains_key(&type_id) {
            self.components.insert(type_id, Box::new(SparseSet::<T>::new()));
        }
    }

    /// Spawn a new entity
    pub fn spawn(&mut self) -> Entity {
        self.entities.allocate()
    }

    /// Despawn an entity (removes all components)
    pub fn despawn(&mut self, entity: Entity) -> bool {
        if !self.entities.is_alive(entity) {
            return false;
        }

        // Remove from all component storages
        for storage in self.components.values_mut() {
            // This is type-erased, need to handle each type
            // (Implementation detail: use a trait for this)
        }

        self.entities.free(entity)
    }

    /// Add a component to an entity
    pub fn add<T: Component>(&mut self, entity: Entity, component: T) {
        if !self.entities.is_alive(entity) {
            panic!("Entity {:?} is not alive", entity);
        }

        let type_id = TypeId::of::<T>();
        let storage = self.components
            .get_mut(&type_id)
            .expect("Component type not registered")
            .downcast_mut::<SparseSet<T>>()
            .unwrap();

        storage.insert(entity, component);
    }

    /// Get a component reference
    pub fn get<T: Component>(&self, entity: Entity) -> Option<&T> {
        let type_id = TypeId::of::<T>();
        let storage = self.components.get(&type_id)?
            .downcast_ref::<SparseSet<T>>()?;
        storage.get(entity)
    }

    /// Get a mutable component reference
    pub fn get_mut<T: Component>(&mut self, entity: Entity) -> Option<&mut T> {
        let type_id = TypeId::of::<T>();
        let storage = self.components.get_mut(&type_id)?
            .downcast_mut::<SparseSet<T>>()?;
        storage.get_mut(entity)
    }

    /// Remove a component
    pub fn remove<T: Component>(&mut self, entity: Entity) -> Option<T> {
        let type_id = TypeId::of::<T>();
        let storage = self.components.get_mut(&type_id)?
            .downcast_mut::<SparseSet<T>>()?;
        storage.remove(entity)
    }

    /// Check if entity has a component
    pub fn has<T: Component>(&self, entity: Entity) -> bool {
        self.get::<T>(entity).is_some()
    }

    /// Check if entity is alive
    pub fn is_alive(&self, entity: Entity) -> bool {
        self.entities.is_alive(entity)
    }
}
```

**Tests:**

```rust
#[test]
fn test_world_spawn_despawn() {
    let mut world = World::new();
    let entity = world.spawn();

    assert!(world.is_alive(entity));

    world.despawn(entity);

    assert!(!world.is_alive(entity));
}

#[test]
fn test_world_add_get_component() {
    let mut world = World::new();
    world.register::<Transform>();

    let entity = world.spawn();
    world.add(entity, Transform::default());

    assert!(world.get::<Transform>(entity).is_some());
}

#[test]
#[should_panic]
fn test_world_add_to_dead_entity_panics() {
    let mut world = World::new();
    world.register::<Transform>();

    let entity = world.spawn();
    world.despawn(entity);

    world.add(entity, Transform::default());  // Should panic
}
```

---

## ✅ **Acceptance Criteria**

- [ ] Entity allocator implemented and tested
- [ ] Sparse-set storage implemented and tested
- [ ] World container implemented and tested
- [ ] All unit tests pass (>20 tests)
- [ ] Benchmarks meet targets:
  - Spawn 10k entities: < 1ms
  - Add 10k components: < 2ms
  - Query 10k components: < 0.5ms
- [ ] Zero unsafe code (except where necessary)
- [ ] 100% rustdoc coverage for public APIs
- [ ] Code formatted (rustfmt)
- [ ] No clippy warnings

---

## 🎯 **Performance Targets**

| Operation | Target | Critical |
|-----------|--------|----------|
| Allocate entity | < 0.1μs | < 1μs |
| Insert component | < 0.2μs | < 1μs |
| Get component | < 0.05μs | < 0.2μs |
| Iterate 10k | < 0.5ms | < 1ms |

---

**Dependencies:** None (first task!)
**Next:** [phase1-ecs-queries.md](phase1-ecs-queries.md)
