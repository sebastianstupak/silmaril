# Engine Core

## Purpose
The core crate provides the foundational systems for the entire game engine:
- **ECS (Entity Component System)**: High-performance archetype-based ECS with parallel query support
- **Serialization**: Binary serialization for game state, networking, and save files
- **Platform Abstraction**: Cross-platform utilities for Windows, Linux, and macOS

This is the foundation upon which all other engine crates are built.

## MUST READ Documentation
Before working on this crate, read these documents in order:

1. **[architecture.md](../../docs/architecture.md)** - Overall engine architecture and design principles
2. **[phase1-ecs-core.md](../../docs/phase1-ecs-core.md)** - ECS design, archetypes, and component storage
3. **[phase1-ecs-queries.md](../../docs/phase1-ecs-queries.md)** - Query system, filters, and parallel iteration
4. **[phase1-serialization.md](../../docs/phase1-serialization.md)** - Binary serialization format and versioning
5. **[phase1-platform.md](../../docs/phase1-platform.md)** - Platform abstraction layer design

## Related Crates
- **engine-macros**: Provides `#[derive(Component)]` and other proc macros
- **engine-networking**: Uses ECS and serialization for state sync
- **engine-renderer**: Queries ECS for renderable entities
- **engine-physics**: Integrates with ECS for transform updates

## Quick Example
```rust
use engine_core::{World, Entity, Component};

#[derive(Component)]
struct Position { x: f32, y: f32, z: f32 }

#[derive(Component)]
struct Velocity { x: f32, y: f32, z: f32 }

fn physics_system(world: &mut World) {
    // Parallel query over all entities with Position + Velocity
    world.query::<(&mut Position, &Velocity)>()
        .par_iter_mut()
        .for_each(|(pos, vel)| {
            pos.x += vel.x;
            pos.y += vel.y;
            pos.z += vel.z;
        });
}
```

## Key Dependencies
- `rayon` - Parallel iteration
- `serde` - Serialization traits (optional)
- `bitflags` - Component flags and masks

## Performance Targets
- ECS query iteration: 10M+ entities/sec
- Component addition/removal: <1us per operation
- Serialization: 100MB/sec for binary format
