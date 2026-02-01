# Serialization Module

> **Phase 1.3 Complete** - Multi-format ECS state serialization with delta compression

## Overview

This module provides comprehensive serialization support for the ECS WorldState in three formats:
- **YAML**: Human-readable, editable by AI agents
- **Bincode**: Fast binary serialization for local storage
- **FlatBuffers**: Zero-copy network serialization (deferred to Phase 2)

## Key Features

### ✅ Multi-Format Serialization
- `Format` enum for format selection
- `Serializable` trait for unified API
- Reader/writer support for streaming I/O

### ✅ Type-Erased Components
- `ComponentData` enum wraps all component types
- Enables serialization without runtime type info
- Easy to extend (add to enum)

### ✅ WorldState Snapshots
- Complete ECS state capture
- Version tracking
- Timestamp metadata
- Entity count tracking

### ✅ Delta Compression
- Minimal state diffs
- Tracks added/removed entities
- Tracks modified/removed components
- Adaptive full/delta switching

## Usage

### Serialize WorldState

```rust
use engine_core::serialization::{WorldState, Format, Serializable};

let state = WorldState::new();

// YAML (human-readable)
let yaml = Serializable::serialize(&state, Format::Yaml)?;
let yaml_str = String::from_utf8(yaml)?;

// Bincode (fast binary)
let bytes = Serializable::serialize(&state, Format::Bincode)?;

// Restore
let restored = <WorldState as Serializable>::deserialize(&bytes, Format::Bincode)?;
```

### Delta Compression

```rust
use engine_core::serialization::{WorldState, WorldStateDelta};

let old_state = WorldState::new();
let new_state = WorldState::new();

// Compute minimal diff
let delta = WorldStateDelta::compute(&old_state, &new_state);

// Adaptive switching
if delta.is_smaller_than(&new_state) {
    // Send delta over network
    let delta_bytes = bincode::serialize(&delta)?;
} else {
    // Send full state
    let full_bytes = bincode::serialize(&new_state)?;
}

// Apply delta
let mut base = old_state.clone();
delta.apply(&mut base);
```

### Components

```rust
use engine_core::{Health, Transform, Velocity, MeshRenderer};

// Health component
let mut health = Health::new(100.0, 100.0);
health.damage(30.0);
assert_eq!(health.current, 70.0);
assert!(health.is_alive());

// Transform component
let transform = Transform::default();
assert_eq!(transform.position, Vec3::ZERO);

// Velocity component
let velocity = Velocity::new(1.0, 0.0, 0.0);

// MeshRenderer component
let renderer = MeshRenderer::new(mesh_id, material_id);
```

## Module Structure

```
serialization/
├── mod.rs              # Module exports
├── error.rs            # SerializationError types
├── format.rs           # Format enum + Serializable trait
├── component_data.rs   # ComponentData enum
├── world_state.rs      # WorldState snapshot/restore
└── delta.rs            # Delta compression
```

## Performance

### Targets (1000 entities):
- YAML snapshot: < 50ms
- Bincode snapshot: < 5ms
- FlatBuffers snapshot: < 3ms
- Delta compute: < 5ms
- Delta apply: < 3ms
- Delta size: 60-80% smaller than full state

### Current (empty state):
- ✅ YAML roundtrip: < 1ms
- ✅ Bincode roundtrip: < 1ms
- ✅ Delta compute: < 1ms

## Future Work (Phase 2)

### FlatBuffers Implementation
- [ ] Schema definition (`.fbs` files)
- [ ] Build script integration
- [ ] Zero-copy deserialization
- [ ] Network packet framing

### World Integration
- [ ] `World::entities()` iterator
- [ ] `World::get_all_components(entity)`
- [ ] `World::spawn_with_id(entity)`
- [ ] `World::add_component_data(entity, ComponentData)`

### Component Registration
- [ ] Proc macro for automatic ComponentData generation
- [ ] Runtime component registry
- [ ] Dynamic component loading

## Testing

Run all serialization tests:
```bash
cargo test --lib serialization
```

Run demo example:
```bash
cargo run --example serialization_demo
```

## See Also

- [Phase 1.3 Task Breakdown](../../../../docs/tasks/phase1-serialization.md)
- [PHASE_1_3_COMPLETE.md](../../../../PHASE_1_3_COMPLETE.md)
- [Error Handling Guide](../../../../docs/error-handling.md)
