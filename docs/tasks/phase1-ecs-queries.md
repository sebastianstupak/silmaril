# Phase 1.2: Advanced ECS Query System

**Status:** ⚪ Not Started
**Estimated Time:** 4-6 days
**Priority:** High (blocks game logic systems)

---

## 🎯 **Objective**

Implement ergonomic, type-safe query system for accessing multiple components simultaneously. This is what makes ECS usable for game logic.

**Target API:**
```rust
// Single component
for (entity, transform) in world.query::<&Transform>() {
    // ...
}

// Multiple components (tuples)
for (entity, (transform, velocity)) in world.query::<(&mut Transform, &Velocity)>() {
    transform.position += velocity.0 * dt;
}

// Optional components
for (entity, (transform, mesh)) in world.query::<(&Transform, Option<&MeshRenderer>)>() {
    // ...
}
```

---

## 📋 **Detailed Tasks**

### **1. Query Trait** (Day 1)

**File:** `engine/core/src/ecs/query.rs`

```rust
/// Trait for types that can be queried from the world
pub trait Query {
    /// The item type returned by iteration
    type Item<'a>;

    /// Fetch data for iteration
    fn fetch<'a>(world: &'a World) -> QueryIter<'a, Self>
    where
        Self: Sized;
}

/// Iterator over query results
pub struct QueryIter<'a, Q: Query> {
    // Internal state
    world: &'a World,
    current_index: usize,
    // ...
}

impl<'a, Q: Query> Iterator for QueryIter<'a, Q> {
    type Item = Q::Item<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        // Implementation
    }
}
```

---

### **2. Single Component Queries** (Day 1-2)

**Immutable reference:**

```rust
impl<T: Component> Query for &T {
    type Item<'a> = (Entity, &'a T);

    fn fetch<'a>(world: &'a World) -> QueryIter<'a, Self> {
        let type_id = TypeId::of::<T>();
        let storage = world.components.get(&type_id)
            .expect("Component not registered")
            .downcast_ref::<SparseSet<T>>()
            .unwrap();

        QueryIter {
            world,
            current_index: 0,
            // Store reference to storage
        }
    }
}

impl<'a, T: Component> Iterator for QueryIter<'a, &T> {
    type Item = (Entity, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        // Iterate over sparse set
    }
}
```

**Mutable reference:**

```rust
impl<T: Component> Query for &mut T {
    type Item<'a> = (Entity, &'a mut T);

    fn fetch<'a>(world: &'a mut World) -> QueryIterMut<'a, Self> {
        // Similar to immutable, but mut
    }
}
```

**Tests:**

```rust
#[test]
fn test_query_single_component() {
    let mut world = World::new();
    world.register::<Transform>();

    for i in 0..100 {
        let e = world.spawn();
        world.add(e, Transform::from_translation(Vec3::new(i as f32, 0.0, 0.0)));
    }

    let mut count = 0;
    for (entity, transform) in world.query::<&Transform>() {
        assert!(transform.position.x >= 0.0);
        count += 1;
    }

    assert_eq!(count, 100);
}

#[test]
fn test_query_mut() {
    let mut world = World::new();
    world.register::<Transform>();

    let e = world.spawn();
    world.add(e, Transform::default());

    for (_, transform) in world.query::<&mut Transform>() {
        transform.position.x = 5.0;
    }

    assert_eq!(world.get::<Transform>(e).unwrap().position.x, 5.0);
}
```

---

### **3. Tuple Queries (2 Components)** (Day 2-3)

**The hard part:** Need to iterate over entities that have BOTH components.

```rust
impl<A: Component, B: Component> Query for (&A, &B) {
    type Item<'a> = (Entity, (&'a A, &'a B));

    fn fetch<'a>(world: &'a World) -> QueryIter<'a, Self> {
        // Get both storages
        let storage_a = world.components.get(&TypeId::of::<A>())
            .unwrap()
            .downcast_ref::<SparseSet<A>>()
            .unwrap();

        let storage_b = world.components.get(&TypeId::of::<B>())
            .unwrap()
            .downcast_ref::<SparseSet<B>>()
            .unwrap();

        // Iterate over smaller storage, check if entity exists in other
        let (primary, secondary, is_a_primary) = if storage_a.len() < storage_b.len() {
            (storage_a, storage_b, true)
        } else {
            (storage_b, storage_a, false)
        };

        QueryIter {
            world,
            // ... store both storages
        }
    }
}

impl<'a, A: Component, B: Component> Iterator for QueryIter<'a, (&A, &B)> {
    type Item = (Entity, (&'a A, &'a B));

    fn next(&mut self) -> Option<Self::Item> {
        // Iterate primary storage
        // For each entity, check if it exists in secondary
        // Return tuple of both components
    }
}
```

**Tests:**

```rust
#[test]
fn test_query_two_components() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Velocity>();

    // Entity with both components
    let e1 = world.spawn();
    world.add(e1, Transform::default());
    world.add(e1, Velocity(Vec3::X));

    // Entity with only Transform
    let e2 = world.spawn();
    world.add(e2, Transform::default());

    // Query should only return e1
    let mut count = 0;
    for (entity, (transform, velocity)) in world.query::<(&Transform, &Velocity)>() {
        assert_eq!(entity, e1);
        count += 1;
    }

    assert_eq!(count, 1);
}
```

---

### **4. Macro-Based Tuple Generation** (Day 3-4)

**Problem:** Need to implement Query for 3, 4, 5+ tuples. Lots of boilerplate.

**Solution:** Macro to generate implementations.

```rust
// engine/macros/src/lib.rs

macro_rules! impl_query_tuple {
    ($($T:ident),*) => {
        impl<$($T: Component),*> Query for ($(&$T),*) {
            type Item<'a> = (Entity, ($(&'a $T),*));

            fn fetch<'a>(world: &'a World) -> QueryIter<'a, Self> {
                // Get all storages
                $(
                    let $T = world.components.get(&TypeId::of::<$T>())
                        .unwrap()
                        .downcast_ref::<SparseSet<$T>>()
                        .unwrap();
                )*

                // Find smallest storage to iterate
                let storages = vec![$($T.len()),*];
                let min_idx = storages.iter().enumerate()
                    .min_by_key(|(_, &len)| len)
                    .unwrap().0;

                // ...
            }
        }

        // Similar for mutable variants
        impl<$($T: Component),*> Query for ($(&mut $T),*) {
            // ...
        }

        // Mixed immutable/mutable
        // This gets complex...
    };
}

// Generate for tuples up to size 12
impl_query_tuple!(A, B);
impl_query_tuple!(A, B, C);
impl_query_tuple!(A, B, C, D);
impl_query_tuple!(A, B, C, D, E);
impl_query_tuple!(A, B, C, D, E, F);
// ... up to 12
```

**Alternative:** Use `seq-macro` crate for cleaner generation.

---

### **5. Optional Components** (Day 4-5)

**API:**
```rust
for (entity, (transform, mesh)) in world.query::<(&Transform, Option<&MeshRenderer>)>() {
    if let Some(mesh) = mesh {
        // Has mesh
    }
}
```

**Implementation:**

```rust
impl<T: Component> Query for Option<&T> {
    type Item<'a> = Option<&'a T>;

    // Returns None if component doesn't exist, doesn't filter entity
}

impl<A: Component, B: Component> Query for (&A, Option<&B>) {
    type Item<'a> = (Entity, (&'a A, Option<&'a B>));

    fn fetch<'a>(world: &'a World) -> QueryIter<'a, Self> {
        // Iterate over A (required)
        // Lookup B (optional)
    }
}
```

---

### **6. Query Filters** (Day 5-6)

**API:**
```rust
// Only entities WITH Health
for (entity, transform) in world.query::<&Transform>().with::<Health>() { }

// Only entities WITHOUT Health
for (entity, transform) in world.query::<&Transform>().without::<Health>() { }
```

**Implementation:**

```rust
pub struct QueryIter<'a, Q: Query> {
    // ...
    with_filters: Vec<TypeId>,
    without_filters: Vec<TypeId>,
}

impl<'a, Q: Query> QueryIter<'a, Q> {
    pub fn with<T: Component>(mut self) -> Self {
        self.with_filters.push(TypeId::of::<T>());
        self
    }

    pub fn without<T: Component>(mut self) -> Self {
        self.without_filters.push(TypeId::of::<T>());
        self
    }
}

impl<'a, Q: Query> Iterator for QueryIter<'a, Q> {
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let item = self.next_unfiltered()?;
            let entity = item.0;

            // Check with filters
            if !self.with_filters.iter().all(|&type_id| {
                // Entity must have this component
                self.world.has_component(entity, type_id)
            }) {
                continue;
            }

            // Check without filters
            if self.without_filters.iter().any(|&type_id| {
                // Entity must NOT have this component
                self.world.has_component(entity, type_id)
            }) {
                continue;
            }

            return Some(item);
        }
    }
}
```

---

## ✅ **Acceptance Criteria**

- [ ] Single component queries work (&T, &mut T)
- [ ] Tuple queries work for 2, 3, 4+ components
- [ ] Optional components work (Option<&T>)
- [ ] Query filters work (.with(), .without())
- [ ] All combinations tested:
  - (&A, &B)
  - (&mut A, &B)
  - (&A, &mut B)
  - (&mut A, &mut B)
  - (&A, Option<&B>)
  - etc.
- [ ] Benchmarks meet targets
- [ ] No unsafe code (or minimal, well-documented)
- [ ] Compile-time checks prevent invalid queries:
  - Can't have (&mut T, &mut T) - double borrow
  - Can't have (&T, &mut T) - aliasing

---

## 🧪 **Complex Test Cases**

```rust
#[test]
fn test_query_complex() {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Velocity>();
    world.register::<Health>();
    world.register::<Dead>();

    // Various entity configurations
    let e1 = world.spawn();
    world.add(e1, Transform::default());
    world.add(e1, Velocity(Vec3::X));
    world.add(e1, Health { current: 100.0, max: 100.0 });

    let e2 = world.spawn();
    world.add(e2, Transform::default());
    world.add(e2, Velocity(Vec3::Y));
    world.add(e2, Dead);

    let e3 = world.spawn();
    world.add(e3, Transform::default());

    // Query: entities with Transform + Velocity, without Dead
    let results: Vec<_> = world
        .query::<(&Transform, &Velocity)>()
        .without::<Dead>()
        .collect();

    assert_eq!(results.len(), 1);  // Only e1
}
```

---

## 🎯 **Performance Targets**

| Query Type | Target (10k entities) | Critical |
|------------|----------------------|----------|
| Single component (&T) | < 0.5ms | < 1ms |
| Two components (&A, &B) | < 1ms | < 2ms |
| Three components (&A, &B, &C) | < 1.5ms | < 3ms |
| With filters | < 2ms | < 4ms |

---

## 💡 **Implementation Tips**

1. **Start simple:** Get single-component queries working first
2. **Test heavily:** Queries are complex, lots of edge cases
3. **Benchmark early:** Query performance is critical
4. **Study existing implementations:**
   - [hecs](https://github.com/Ralith/hecs/blob/master/src/query.rs)
   - [bevy_ecs](https://github.com/bevyengine/bevy/tree/main/crates/bevy_ecs)
   - [shipyard](https://github.com/leudz/shipyard)

5. **Use macros wisely:** Tuple generation is boilerplate, automate it

---

**Dependencies:** [phase1-ecs-core.md](phase1-ecs-core.md)
**Next:** [phase1-serialization.md](phase1-serialization.md)
