# Workflow: Add New ECS System

> Step-by-step guide for adding a new system to the ECS

---

## Prerequisites

- [ ] Read [docs/tasks/phase1-ecs-core.md](../../docs/tasks/phase1-ecs-core.md)
- [ ] Read [docs/architecture.md](../../docs/architecture.md)
- [ ] Understand system execution order and scheduling

---

## Step 1: Determine System Category

**System types:**

1. **Gameplay Systems** (`engine/core/src/systems/gameplay/`)
   - Health, combat, inventory
   - Runs on: Server (authoritative)

2. **Physics Systems** (`engine/physics/src/systems/`)
   - Movement, collision, forces
   - Runs on: Server + Client (prediction)

3. **Rendering Systems** (`engine/renderer/src/systems/`)
   - Culling, LOD, transforms
   - Runs on: Client only

4. **Network Systems** (`engine/networking/src/systems/`)
   - Replication, interpolation
   - Runs on: Client + Server

Choose the appropriate directory based on your system's purpose.

---

## Step 2: Define the System Function

**File:** `engine/{crate}/src/systems/{category}/{name}.rs`

**Template:**
```rust
use crate::ecs::{Query, World};
use tracing::{debug, instrument};

/// Brief description of what this system does.
///
/// This system runs during [which phase] and modifies [which components].
/// It is responsible for [specific behavior].
///
/// # System Execution
///
/// - **Frequency:** Every frame / Every tick / On event
/// - **Phase:** Update / FixedUpdate / Render
/// - **Dependencies:** Runs after [`other_system`]
///
/// # Examples
///
/// ```
/// use agent_game_engine::ecs::*;
/// use agent_game_engine::systems::*;
///
/// let mut world = World::new();
/// // Setup world...
/// my_system(&mut world, 0.016);
/// ```
#[instrument(skip(world))]
pub fn my_system(world: &mut World, delta_time: f32) {
    // Query for entities with required components
    let query = world.query::<(&MyComponent, &mut Transform)>();

    for (entity, (my_comp, transform)) in query.iter() {
        // System logic here
        transform.position.x += my_comp.velocity * delta_time;

        debug!(?entity, ?transform.position, "Updated entity position");
    }
}
```

**For client-only systems:**
```rust
#[cfg(feature = "client")]
#[instrument(skip(world))]
pub fn client_only_system(world: &mut World, delta_time: f32) {
    // Client-specific logic
}
```

**For server-only systems:**
```rust
#[cfg(feature = "server")]
#[instrument(skip(world))]
pub fn server_only_system(world: &mut World, delta_time: f32) {
    // Server-specific logic
}
```

**Validation:**
```bash
cargo build --package agent-game-engine-{crate}
```

---

## Step 3: Write Unit Tests

**File:** Same file as system, bottom section

**Test template:**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::*;

    #[test]
    fn test_my_system_basic() {
        let mut world = World::new();

        // Setup test entities
        let entity = world.spawn();
        world.add(entity, MyComponent {
            velocity: 10.0,
        });
        world.add(entity, Transform::default());

        // Run system
        my_system(&mut world, 1.0);

        // Verify results
        let transform = world.get::<Transform>(entity).unwrap();
        assert_eq!(transform.position.x, 10.0);
    }

    #[test]
    fn test_my_system_multiple_entities() {
        let mut world = World::new();

        // Create multiple entities
        for i in 0..10 {
            let entity = world.spawn();
            world.add(entity, MyComponent {
                velocity: i as f32,
            });
            world.add(entity, Transform::default());
        }

        // Run system
        my_system(&mut world, 1.0);

        // Verify all entities updated
        let query = world.query::<&Transform>();
        assert_eq!(query.iter().count(), 10);
    }

    #[test]
    fn test_my_system_no_matching_entities() {
        let mut world = World::new();

        // Spawn entity without required components
        let entity = world.spawn();
        world.add(entity, Transform::default());
        // Missing MyComponent

        // System should not crash
        my_system(&mut world, 1.0);

        // Transform should not change
        let transform = world.get::<Transform>(entity).unwrap();
        assert_eq!(transform.position.x, 0.0);
    }

    #[test]
    fn test_my_system_delta_time() {
        let mut world = World::new();
        let entity = world.spawn();

        world.add(entity, MyComponent { velocity: 10.0 });
        world.add(entity, Transform::default());

        // Run with different delta times
        my_system(&mut world, 0.5);
        let transform = world.get::<Transform>(entity).unwrap();
        assert_eq!(transform.position.x, 5.0);

        my_system(&mut world, 0.5);
        let transform = world.get::<Transform>(entity).unwrap();
        assert_eq!(transform.position.x, 10.0);
    }

    #[test]
    fn test_my_system_boundary_conditions() {
        let mut world = World::new();
        let entity = world.spawn();

        world.add(entity, MyComponent { velocity: 0.0 });
        world.add(entity, Transform::default());

        my_system(&mut world, 1.0);

        let transform = world.get::<Transform>(entity).unwrap();
        assert_eq!(transform.position.x, 0.0);
    }
}
```

**Run tests:**
```bash
cargo test --package agent-game-engine-{crate} my_system
```

---

## Step 4: Register System in App

**File:** `engine/core/src/app.rs` or game-specific app file

**Add to system registry:**
```rust
impl App {
    pub fn new() -> Self {
        let mut app = Self {
            world: World::new(),
            systems: Vec::new(),
        };

        // Register systems in execution order
        app.add_system(SystemDescriptor {
            name: "my_system",
            function: my_system,
            phase: SystemPhase::Update,
            run_criteria: RunCriteria::Always,
        });

        app
    }

    /// Add a system to the app
    pub fn add_system(&mut self, descriptor: SystemDescriptor) {
        self.systems.push(descriptor);
    }
}
```

**System phases:**
```rust
pub enum SystemPhase {
    PreUpdate,    // Input handling
    Update,       // Main game logic
    FixedUpdate,  // Physics (60Hz)
    PostUpdate,   // Cleanup
    Render,       // Rendering
}
```

**Run criteria:**
```rust
pub enum RunCriteria {
    Always,                    // Every frame
    FixedTimeStep(f32),       // Every N seconds
    OnEvent(EventType),       // When event fires
    Conditional(Box<dyn Fn(&World) -> bool>),  // Custom condition
}
```

**Validation:**
```bash
cargo build --package agent-game-engine-core
```

---

## Step 5: Handle System Execution Order

**Define dependencies:**
```rust
app.add_system(SystemDescriptor {
    name: "my_system",
    function: my_system,
    phase: SystemPhase::Update,
    run_criteria: RunCriteria::Always,
    dependencies: vec!["physics_system"],  // Run after physics
});
```

**Common execution order:**
```
PreUpdate:
  - input_system

Update:
  - movement_system
  - combat_system
  - my_system

FixedUpdate:
  - physics_system

PostUpdate:
  - cleanup_system
  - network_replication_system

Render:
  - culling_system
  - render_system
```

---

## Step 6: Add Integration Tests

**File:** `engine/{crate}/tests/systems_integration.rs`

**Template:**
```rust
use agent_game_engine_core::*;
use agent_game_engine_{crate}::*;

#[test]
fn test_my_system_with_other_systems() {
    let mut app = App::new();

    // Spawn test entity
    let entity = app.world.spawn();
    app.world.add(entity, MyComponent { velocity: 10.0 });
    app.world.add(entity, Transform::default());

    // Run update cycle
    app.update(0.016);  // 60 FPS

    // Verify system ran
    let transform = app.world.get::<Transform>(entity).unwrap();
    assert!(transform.position.x > 0.0);
}

#[test]
fn test_my_system_order() {
    let mut app = App::new();

    // Setup entities
    let entity = app.world.spawn();
    app.world.add(entity, MyComponent { velocity: 10.0 });
    app.world.add(entity, Transform::default());

    // Run multiple frames
    for _ in 0..60 {
        app.update(0.016);
    }

    // Verify cumulative effect
    let transform = app.world.get::<Transform>(entity).unwrap();
    let expected = 10.0 * 0.016 * 60.0;
    assert!((transform.position.x - expected).abs() < 0.01);
}
```

**Run integration tests:**
```bash
cargo test --package agent-game-engine-{crate} --tests
```

---

## Step 7: Add Benchmarks (if performance-critical)

**File:** `engine/{crate}/benches/systems_bench.rs`

**Template:**
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use agent_game_engine_core::*;
use agent_game_engine_{crate}::*;

fn bench_my_system(c: &mut Criterion) {
    let mut world = World::new();

    // Setup benchmark scenario
    for i in 0..10_000 {
        let entity = world.spawn();
        world.add(entity, MyComponent { velocity: i as f32 });
        world.add(entity, Transform::default());
    }

    c.bench_function("my_system_10k_entities", |b| {
        b.iter(|| {
            my_system(black_box(&mut world), black_box(0.016));
        });
    });
}

criterion_group!(benches, bench_my_system);
criterion_main!(benches);
```

**Run benchmarks:**
```bash
cargo bench --package agent-game-engine-{crate}
```

**Performance targets:**
- 10k entities: < 1ms per system
- 100k entities: < 10ms per system

---

## Step 8: Add Documentation

**Document system behavior:**
```rust
/// Updates entity positions based on velocity.
///
/// This system reads [`MyComponent::velocity`] and updates [`Transform::position`]
/// each frame. It is typically used for [specific use case].
///
/// # System Details
///
/// - **Phase:** Update
/// - **Frequency:** Every frame
/// - **Dependencies:** None
/// - **Side Effects:** Modifies `Transform::position`
///
/// # Query
///
/// Queries for entities with:
/// - `&MyComponent` (read-only)
/// - `&mut Transform` (mutable)
///
/// # Parameters
///
/// - `world` - The ECS world
/// - `delta_time` - Time since last frame in seconds
///
/// # Performance
///
/// - 10k entities: ~0.5ms
/// - 100k entities: ~5ms
///
/// # Examples
///
/// Basic usage:
/// ```
/// use agent_game_engine::*;
///
/// let mut world = World::new();
/// let entity = world.spawn();
/// world.add(entity, MyComponent { velocity: 10.0 });
/// world.add(entity, Transform::default());
///
/// my_system(&mut world, 0.016);
/// ```
///
/// With multiple entities:
/// ```
/// # use agent_game_engine::*;
/// let mut app = App::new();
///
/// for i in 0..100 {
///     let entity = app.world.spawn();
///     app.world.add(entity, MyComponent { velocity: i as f32 });
///     app.world.add(entity, Transform::default());
/// }
///
/// app.update(0.016);
/// ```
///
/// # See Also
///
/// - [`other_system`] - Related system
/// - [`MyComponent`] - Component this system operates on
#[instrument(skip(world))]
pub fn my_system(world: &mut World, delta_time: f32) {
    // ...
}
```

**Build docs:**
```bash
cargo doc --package agent-game-engine-{crate} --no-deps --open
```

---

## Step 9: Handle Client/Server Split

### Server-Only System

```rust
#[cfg(feature = "server")]
pub fn server_authority_system(world: &mut World, delta_time: f32) {
    // Authoritative logic (combat, loot, etc.)
}
```

### Client-Only System

```rust
#[cfg(feature = "client")]
pub fn client_prediction_system(world: &mut World, delta_time: f32) {
    // Client prediction logic
}
```

### Shared System

```rust
pub fn shared_system(world: &mut World, delta_time: f32) {
    // Runs on both client and server
    // Example: physics, movement
}
```

**Register conditionally:**
```rust
impl App {
    pub fn new() -> Self {
        let mut app = Self::default();

        // Always register
        app.add_system(shared_system);

        #[cfg(feature = "server")]
        app.add_system(server_authority_system);

        #[cfg(feature = "client")]
        app.add_system(client_prediction_system);

        app
    }
}
```

---

## Step 10: Add Profiling

**Instrument system:**
```rust
use tracing::{instrument, span, Level};

#[instrument(skip(world), level = "trace")]
pub fn my_system(world: &mut World, delta_time: f32) {
    let _span = span!(Level::TRACE, "my_system_query").entered();

    let query = world.query::<(&MyComponent, &mut Transform)>();

    for (entity, (my_comp, transform)) in query.iter() {
        // System logic
    }
}
```

**Enable profiling:**
```bash
RUST_LOG=trace cargo run --features profiling
```

**View in Tracy:**
```bash
cargo build --features profiling
./target/debug/client
# Open Tracy profiler
```

---

## Step 11: Run Full Test Suite

**Pre-commit checklist:**
```bash
# Format
cargo fmt

# Check format
cargo fmt --check

# Clippy
cargo clippy --workspace -- -D warnings

# Unit tests
cargo test --package agent-game-engine-{crate} --lib

# Integration tests
cargo test --package agent-game-engine-{crate} --tests

# Doc tests
cargo test --package agent-game-engine-{crate} --doc

# Benchmarks
cargo bench --package agent-game-engine-{crate}

# Build docs
cargo doc --no-deps
```

---

## Common Errors and Solutions

### Error: Borrow checker issues
```
error[E0502]: cannot borrow `world` as mutable because it is also borrowed as immutable
```

**Solution:** Split queries or use interior mutability:
```rust
// Instead of:
let query1 = world.query::<&ComponentA>();
let query2 = world.query::<&mut ComponentB>();  // Error!

// Do:
let mut query = world.query::<(&ComponentA, &mut ComponentB)>();
```

---

### Error: System not running
```
// System registered but never executes
```

**Solution:** Check run criteria and phase:
```rust
app.add_system(SystemDescriptor {
    run_criteria: RunCriteria::Always,  // Not Conditional
    phase: SystemPhase::Update,         // Correct phase
});
```

---

### Error: Wrong execution order
```
// System runs before dependencies are ready
```

**Solution:** Specify dependencies:
```rust
app.add_system(SystemDescriptor {
    dependencies: vec!["dependency_system"],
});
```

---

## Validation Checklist

- [ ] System function defined
- [ ] Unit tests written (at least 5 tests)
- [ ] Integration tests written
- [ ] Registered in App
- [ ] Execution order defined
- [ ] Client/server split handled
- [ ] Documentation complete
- [ ] Profiling instrumentation added
- [ ] All tests pass
- [ ] No clippy warnings
- [ ] Benchmarks meet targets
- [ ] Code formatted

---

## Next Steps

After adding system:
1. Test with real game scenarios
2. Profile performance with Tracy
3. Add to examples
4. Update architecture docs

---

## References

- [docs/tasks/phase1-ecs-core.md](../../docs/tasks/phase1-ecs-core.md) - ECS core
- [docs/architecture.md](../../docs/architecture.md) - System architecture
- [docs/development-workflow.md](../../docs/development-workflow.md) - Dev workflow

---

**Last Updated:** 2026-02-01
