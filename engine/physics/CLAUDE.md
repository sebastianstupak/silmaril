# Engine Physics

## Purpose
The physics crate integrates a physics engine for realistic simulation:
- **Rigid Body Dynamics**: Integration with Rapier for 3D physics
- **Collision Detection**: Broad-phase and narrow-phase collision detection
- **Constraints**: Joints, motors, and other physics constraints
- **Raycasting**: Fast raycasting for line-of-sight and projectile physics
- **Character Controller**: Specialized controller for player movement

## MUST READ Documentation
Before working on this crate, read these documents in order:

1. **[phase3-physics.md](../../docs/phase3-physics.md)** - Physics integration and ECS sync

## Related Crates
- **engine-core**: Syncs physics state with ECS components
- **engine-networking**: Network physics state for multiplayer

## Quick Example
```rust
use engine_physics::{PhysicsWorld, RigidBody, Collider};

fn create_physics_object(world: &mut World) {
    let entity = world.spawn();
    world.add_component(entity, RigidBody::dynamic());
    world.add_component(entity, Collider::cuboid(1.0, 1.0, 1.0));
}

fn physics_step(physics: &mut PhysicsWorld, world: &World) {
    // Step physics simulation
    physics.step(1.0 / 60.0);

    // Sync physics state back to ECS
    physics.sync_to_world(world);
}
```

## Key Dependencies
- `rapier3d` - Physics engine
- `engine-core` - ECS integration

## Performance Targets
- 1000+ physics objects at 60 FPS
- <1ms per physics step for typical scenes
- Deterministic simulation for network replay
