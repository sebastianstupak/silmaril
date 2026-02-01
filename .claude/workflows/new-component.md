# Workflow: Add New ECS Component

> Step-by-step guide for adding a new component to the ECS system

---

## Prerequisites

- [ ] Read [docs/tasks/phase1-ecs-core.md](../../docs/tasks/phase1-ecs-core.md)
- [ ] Read [docs/architecture.md](../../docs/architecture.md)
- [ ] Understand ECS architecture and component design

---

## Step 1: Define the Component Type

**File:** `engine/core/src/ecs/components/{category}.rs`

**Categories:**
- `transform.rs` - Position, rotation, scale
- `physics.rs` - RigidBody, Collider, Velocity
- `rendering.rs` - Mesh, Material, Light
- `gameplay.rs` - Health, Inventory, Stats
- `network.rs` - Replicated, ClientOnly, ServerOnly

**Template:**
```rust
use serde::{Deserialize, Serialize};
use crate::ecs::Component;

/// Brief description of what this component represents.
///
/// # Examples
///
/// ```
/// use agent_game_engine::ecs::*;
///
/// let component = MyComponent {
///     field1: 100.0,
///     field2: true,
/// };
/// ```
#[derive(Debug, Clone, Component, Serialize, Deserialize)]
pub struct MyComponent {
    /// Description of field1
    pub field1: f32,

    /// Description of field2
    pub field2: bool,
}

impl Default for MyComponent {
    fn default() -> Self {
        Self {
            field1: 0.0,
            field2: false,
        }
    }
}
```

**Validation:**
```bash
cargo build --package agent-game-engine-core
```

---

## Step 2: Register Component in ComponentData Enum

**File:** `engine/core/src/ecs/serialization.rs`

**Add variant:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComponentData {
    Transform(Transform),
    Health(Health),
    // ... existing components
    MyComponent(MyComponent),  // Add this line
}
```

**Update conversion methods:**
```rust
impl ComponentData {
    /// Convert ComponentData to concrete component
    pub fn add_to_world(&self, world: &mut World, entity: Entity) {
        match self {
            ComponentData::Transform(c) => world.add(entity, c.clone()),
            ComponentData::Health(c) => world.add(entity, c.clone()),
            // ... existing components
            ComponentData::MyComponent(c) => world.add(entity, c.clone()),
        }
    }

    /// Extract component from world as ComponentData
    pub fn from_world<T: Component>(world: &World, entity: Entity) -> Option<Self>
    where
        Self: From<T>,
    {
        world.get::<T>(entity).map(|c| Self::from(c.clone()))
    }
}

// Add From implementation
impl From<MyComponent> for ComponentData {
    fn from(c: MyComponent) -> Self {
        ComponentData::MyComponent(c)
    }
}
```

**Validation:**
```bash
cargo build --package agent-game-engine-core
```

---

## Step 3: Add Component to World Registration

**File:** `engine/core/src/ecs/world.rs`

**Update World::new():**
```rust
impl World {
    pub fn new() -> Self {
        let mut world = Self {
            entities: EntityAllocator::new(),
            components: HashMap::new(),
        };

        // Register built-in components
        world.register::<Transform>();
        world.register::<Health>();
        // ... existing components
        world.register::<MyComponent>();  // Add this line

        world
    }
}
```

**Validation:**
```bash
cargo test --package agent-game-engine-core --lib world
```

---

## Step 4: Write Unit Tests

**File:** `engine/core/src/ecs/components/{category}.rs` (bottom of file)

**Test template:**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::World;

    #[test]
    fn test_my_component_default() {
        let component = MyComponent::default();
        assert_eq!(component.field1, 0.0);
        assert_eq!(component.field2, false);
    }

    #[test]
    fn test_my_component_add_get() {
        let mut world = World::new();
        let entity = world.spawn();

        let component = MyComponent {
            field1: 42.0,
            field2: true,
        };

        world.add(entity, component.clone());

        let retrieved = world.get::<MyComponent>(entity).unwrap();
        assert_eq!(retrieved.field1, 42.0);
        assert_eq!(retrieved.field2, true);
    }

    #[test]
    fn test_my_component_remove() {
        let mut world = World::new();
        let entity = world.spawn();

        world.add(entity, MyComponent::default());
        assert!(world.has::<MyComponent>(entity));

        world.remove::<MyComponent>(entity);
        assert!(!world.has::<MyComponent>(entity));
    }

    #[test]
    fn test_my_component_serialization() {
        let component = MyComponent {
            field1: 100.0,
            field2: true,
        };

        // Test YAML serialization
        let yaml = serde_yaml::to_string(&component).unwrap();
        let deserialized: MyComponent = serde_yaml::from_str(&yaml).unwrap();

        assert_eq!(component.field1, deserialized.field1);
        assert_eq!(component.field2, deserialized.field2);
    }

    #[test]
    fn test_my_component_query() {
        let mut world = World::new();

        // Spawn multiple entities with component
        for i in 0..10 {
            let entity = world.spawn();
            world.add(entity, MyComponent {
                field1: i as f32,
                field2: i % 2 == 0,
            });
        }

        // Query and verify
        let storage = world.components::<MyComponent>().unwrap();
        assert_eq!(storage.len(), 10);
    }
}
```

**Run tests:**
```bash
cargo test --package agent-game-engine-core my_component
```

---

## Step 5: Write Integration Tests

**File:** `engine/core/tests/components_integration.rs`

**Template:**
```rust
use agent_game_engine_core::ecs::*;

#[test]
fn test_my_component_with_other_components() {
    let mut world = World::new();
    let entity = world.spawn();

    // Add multiple components
    world.add(entity, Transform::default());
    world.add(entity, MyComponent {
        field1: 50.0,
        field2: true,
    });

    // Verify both exist
    assert!(world.has::<Transform>(entity));
    assert!(world.has::<MyComponent>(entity));

    // Query both
    let transform = world.get::<Transform>(entity).unwrap();
    let my_comp = world.get::<MyComponent>(entity).unwrap();

    assert_eq!(my_comp.field1, 50.0);
}

#[test]
fn test_my_component_despawn() {
    let mut world = World::new();
    let entity = world.spawn();

    world.add(entity, MyComponent::default());
    assert!(world.is_alive(entity));

    world.despawn(entity);
    assert!(!world.is_alive(entity));
    assert!(!world.has::<MyComponent>(entity));
}
```

**Run integration tests:**
```bash
cargo test --package agent-game-engine-core --tests
```

---

## Step 6: Add Documentation

**Update component documentation:**
```rust
/// Represents [specific gameplay/rendering/physics concept].
///
/// This component is used for [specific purpose]. It is typically
/// paired with [`OtherComponent`] for [reason].
///
/// # Network Replication
///
/// This component is [replicated/client-only/server-only].
///
/// # Examples
///
/// Basic usage:
/// ```
/// use agent_game_engine::ecs::*;
///
/// let mut world = World::new();
/// let entity = world.spawn();
///
/// world.add(entity, MyComponent {
///     field1: 100.0,
///     field2: true,
/// });
/// ```
///
/// With other components:
/// ```
/// # use agent_game_engine::ecs::*;
/// let mut world = World::new();
/// let entity = world.spawn();
///
/// world.add(entity, Transform::default());
/// world.add(entity, MyComponent::default());
/// ```
///
/// # Performance
///
/// - Add operation: O(1)
/// - Get operation: O(1)
/// - Remove operation: O(1)
///
/// # See Also
///
/// - [`RelatedComponent`] - Related functionality
/// - [`OtherComponent`] - Often used together
#[derive(Debug, Clone, Component, Serialize, Deserialize)]
pub struct MyComponent {
    // ...
}
```

**Build docs:**
```bash
cargo doc --package agent-game-engine-core --no-deps --open
```

---

## Step 7: Update Example Code (if applicable)

**File:** `examples/*/src/main.rs`

If this component is useful for examples, add it:

```rust
// In setup function
fn setup_world(world: &mut World) {
    let entity = world.spawn();
    world.add(entity, Transform::default());
    world.add(entity, MyComponent {
        field1: 100.0,
        field2: true,
    });
}
```

---

## Step 8: Run Full Test Suite

**Pre-commit checklist:**
```bash
# Format
cargo fmt

# Check format
cargo fmt --check

# Clippy
cargo clippy --workspace -- -D warnings

# All tests
cargo test --workspace --all-features

# Build docs
cargo doc --no-deps

# Benchmarks (if performance-critical)
cargo bench --package agent-game-engine-core
```

---

## Step 9: Update CHANGELOG (if public-facing)

**File:** `CHANGELOG.md`

```markdown
## [Unreleased]

### Added
- New `MyComponent` for [purpose] (#issue-number)
```

---

## Common Errors and Solutions

### Error: Component not registered
```
Error: Component type not registered
```

**Solution:** Add `world.register::<MyComponent>()` in `World::new()`

---

### Error: Serialization failed
```
Error: missing field `field1`
```

**Solution:** Ensure all fields implement `Serialize` and `Deserialize`. Add `#[serde(default)]` for optional fields.

---

### Error: Tests fail on other platforms
```
Error: test failed on Windows but passed on Linux
```

**Solution:** Check for platform-specific types (paths, line endings). Use `std::path::PathBuf` and normalize data.

---

### Error: Query compilation errors
```
Error: the trait `Component` is not implemented for `MyComponent`
```

**Solution:** Ensure `#[derive(Component)]` is present. Check that component module is exported.

---

## Validation Checklist

- [ ] Component struct defined with proper derives
- [ ] Added to ComponentData enum
- [ ] Registered in World::new()
- [ ] Unit tests written (at least 4 tests)
- [ ] Integration tests written
- [ ] Documentation complete (struct + fields)
- [ ] All tests pass
- [ ] No clippy warnings
- [ ] Code formatted
- [ ] Examples updated (if applicable)
- [ ] CHANGELOG updated (if applicable)

---

## Performance Considerations

**Target benchmarks for new component:**
```bash
cargo bench my_component
```

**Expected results:**
- Insert: < 0.2μs
- Get: < 0.05μs
- Remove: < 0.2μs
- Query 10k: < 0.5ms

If benchmarks don't meet targets, consider:
- Reducing component size (use smaller types)
- Avoiding heap allocations (use arrays instead of Vec)
- Using `Copy` instead of `Clone` where possible

---

## Next Steps

After adding component, you may want to:
1. Create systems that operate on this component (see [new-system.md](new-system.md))
2. Add network replication support (see networking docs)
3. Add to save/load system
4. Create editor tools for component

---

## References

- [docs/tasks/phase1-ecs-core.md](../../docs/tasks/phase1-ecs-core.md) - ECS implementation details
- [docs/architecture.md](../../docs/architecture.md) - System architecture
- [docs/rules/coding-standards.md](../../docs/rules/coding-standards.md) - Code style guide

---

**Last Updated:** 2026-02-01
