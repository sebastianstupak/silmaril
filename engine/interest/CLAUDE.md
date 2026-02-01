# Engine Interest Management

## Purpose
The interest management crate optimizes multiplayer bandwidth:
- **Spatial Partitioning**: Grid-based or octree spatial indexing
- **Interest Areas**: Define what each client can see/hear
- **Priority System**: Prioritize important entities for network updates
- **Dynamic Updates**: Efficiently update interest as players move

This crate is critical for supporting 1000+ players by only sending relevant data.

## MUST READ Documentation
Before working on this crate, read these documents in order:

1. **[phase2-interest-basic.md](../../docs/phase2-interest-basic.md)** - Basic grid-based interest management
2. **[phase3-interest-advanced.md](../../docs/phase3-interest-advanced.md)** - Advanced priority and LOD integration

## Related Crates
- **engine-networking**: Uses interest management to filter network updates
- **engine-lod**: Integrates LOD with interest distance
- **engine-core**: Queries spatial data from ECS

## Quick Example
```rust
use engine_interest::{InterestManager, Grid};

fn update_interest(interest: &mut InterestManager, world: &World) {
    // Update grid with all entity positions
    for (entity, pos) in world.query::<(Entity, &Position)>() {
        interest.update_position(entity, pos);
    }

    // Get entities visible to a client
    let visible = interest.get_interested_entities(client_position, view_distance);

    // Send only visible entities to client
    for entity in visible {
        send_entity_update(entity);
    }
}
```

## Key Dependencies
- `engine-core` - ECS integration
- `engine-networking` - Network filtering

## Performance Targets
- 10K+ entities in interest grid
- <1ms to query nearby entities
- <100 entities per client update (bandwidth limit)
